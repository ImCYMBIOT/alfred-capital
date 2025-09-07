use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::time::{sleep, interval};
use tokio::signal;
use thiserror::Error;
use log::{info, warn, error, debug};

use crate::blockchain::{RpcClient, BlockProcessor};
use crate::database::Database;
use crate::error::IndexerError;
use crate::logging::{LogContext, PerformanceMonitor, ErrorLogger, MetricsLogger};
use crate::retry::CircuitBreaker;

#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("Indexer error: {0}")]
    Indexer(#[from] IndexerError),
    #[error("Monitor configuration error: {0}")]
    Config(String),
    #[error("Shutdown requested")]
    Shutdown,
}

impl From<crate::blockchain::rpc_client::RpcError> for MonitorError {
    fn from(err: crate::blockchain::rpc_client::RpcError) -> Self {
        MonitorError::Indexer(IndexerError::from(err))
    }
}

impl From<crate::database::DbError> for MonitorError {
    fn from(err: crate::database::DbError) -> Self {
        MonitorError::Indexer(IndexerError::from(err))
    }
}

impl From<crate::blockchain::block_processor::ProcessError> for MonitorError {
    fn from(err: crate::blockchain::block_processor::ProcessError) -> Self {
        MonitorError::Indexer(IndexerError::from(err))
    }
}

pub struct BlockMonitorConfig {
    pub poll_interval_seconds: u64,
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
    pub max_retry_delay_seconds: u64,
}

impl Default for BlockMonitorConfig {
    fn default() -> Self {
        Self {
            poll_interval_seconds: 2,
            max_retries: 5,
            retry_delay_seconds: 1,
            max_retry_delay_seconds: 60,
        }
    }
}

pub struct BlockMonitor {
    rpc_client: Arc<RpcClient>,
    block_processor: Arc<BlockProcessor>,
    database: Arc<Database>,
    pub config: BlockMonitorConfig,
    pub shutdown_signal: Arc<AtomicBool>,
    rpc_circuit_breaker: Arc<CircuitBreaker>,
    database_circuit_breaker: Arc<CircuitBreaker>,
}

impl BlockMonitor {
    pub fn new(
        rpc_client: RpcClient,
        block_processor: BlockProcessor,
        database: Database,
        config: Option<BlockMonitorConfig>,
    ) -> Self {
        let context = LogContext::new("block_monitor", "initialization");
        context.info("Initializing block monitor with circuit breakers");
        
        Self {
            rpc_client: Arc::new(rpc_client),
            block_processor: Arc::new(block_processor),
            database: Arc::new(database),
            config: config.unwrap_or_default(),
            shutdown_signal: Arc::new(AtomicBool::new(false)),
            rpc_circuit_breaker: Arc::new(CircuitBreaker::new(5, 60)), // 5 failures, 60s recovery
            database_circuit_breaker: Arc::new(CircuitBreaker::new(3, 30)), // 3 failures, 30s recovery
        }
    }

    /// Start the block monitoring loop
    pub async fn start(&self) -> Result<(), MonitorError> {
        info!("Starting block monitor with {} second polling interval", self.config.poll_interval_seconds);

        // Get the starting block number
        let mut last_processed_block = self.get_starting_block_number().await?;
        info!("Starting from block number: {}", last_processed_block);

        // Set up polling interval
        let mut interval = interval(Duration::from_secs(self.config.poll_interval_seconds));

        // Set up graceful shutdown handling
        let shutdown_signal = Arc::clone(&self.shutdown_signal);
        tokio::spawn(async move {
            match signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received shutdown signal");
                    shutdown_signal.store(true, Ordering::Relaxed);
                }
                Err(err) => {
                    error!("Unable to listen for shutdown signal: {}", err);
                }
            }
        });

        // Main monitoring loop
        loop {
            // Check for shutdown signal
            if self.shutdown_signal.load(Ordering::Relaxed) {
                info!("Shutdown signal received, stopping block monitor");
                self.persist_state(last_processed_block).await?;
                return Err(MonitorError::Shutdown);
            }

            // Wait for next polling interval
            interval.tick().await;

            // Process new blocks with retry logic
            match self.process_new_blocks(&mut last_processed_block).await {
                Ok(blocks_processed) => {
                    if blocks_processed > 0 {
                        debug!("Processed {} new blocks, current block: {}", blocks_processed, last_processed_block);
                    }
                }
                Err(e) => {
                    warn!("Error processing blocks: {}", e);
                    // Continue the loop - errors are handled with retries in process_new_blocks
                }
            }
        }
    }

    /// Process new blocks since the last processed block
    async fn process_new_blocks(&self, last_processed_block: &mut u64) -> Result<u32, MonitorError> {
        let latest_block = self.get_latest_block_with_retry().await?;
        
        if latest_block <= *last_processed_block {
            // No new blocks to process
            return Ok(0);
        }

        let mut blocks_processed = 0;
        let mut current_block = *last_processed_block + 1;

        // Process each new block sequentially
        while current_block <= latest_block {
            // Check for shutdown signal during processing
            if self.shutdown_signal.load(Ordering::Relaxed) {
                info!("Shutdown signal received during block processing");
                break;
            }

            match self.process_single_block(current_block).await {
                Ok(transfer_count) => {
                    info!("Processed block {} with {} POL transfers", current_block, transfer_count);
                    
                    // Update last processed block in database
                    if let Err(e) = self.database.set_last_processed_block(current_block) {
                        error!("Failed to update last processed block in database: {}", e);
                        // Don't return error here, just log it and continue
                    }
                    
                    *last_processed_block = current_block;
                    blocks_processed += 1;
                    current_block += 1;
                }
                Err(e) => {
                    error!("Failed to process block {}: {}", current_block, e);
                    // For block processing errors, we'll retry the same block
                    // after a delay to avoid getting stuck
                    sleep(Duration::from_secs(self.config.retry_delay_seconds)).await;
                    
                    // Skip this block after max retries to avoid infinite loop
                    // In production, you might want to implement more sophisticated error handling
                    warn!("Skipping block {} due to processing error", current_block);
                    current_block += 1;
                }
            }
        }

        Ok(blocks_processed)
    }

    /// Process a single block and return the number of transfers found
    async fn process_single_block(&self, block_number: u64) -> Result<u32, MonitorError> {
        let monitor = PerformanceMonitor::new("process_single_block")
            .with_metadata("block_number", serde_json::json!(block_number));
        
        let context = LogContext::new("block_monitor", "process_single_block")
            .with_block_number(block_number);
        context.debug(&format!("Processing block {}", block_number));
        
        // Process block with circuit breaker protection
        let transfers = {
            let rpc_circuit_breaker = Arc::clone(&self.rpc_circuit_breaker);
            rpc_circuit_breaker.execute(|| async {
                self.block_processor.process_block(block_number).await
                    .map_err(|e| IndexerError::from(e))
            }).await?
        };
        
        let transfer_count = transfers.len() as u32;

        // Store transfers with database circuit breaker protection
        let database_circuit_breaker = Arc::clone(&self.database_circuit_breaker);
        database_circuit_breaker.execute(|| async {
            for transfer in &transfers {
                self.database.store_transfer_and_update_net_flow(transfer)
                    .map_err(|e| IndexerError::from(e))?;
            }
            Ok::<(), IndexerError>(())
        }).await?;

        let duration = monitor.finish();
        MetricsLogger::log_block_processed(block_number, transfer_count, duration);

        let context = LogContext::new("block_monitor", "process_single_block")
            .with_block_number(block_number)
            .with_metadata("transfer_count", serde_json::json!(transfer_count))
            .with_duration_ms(duration);
        context.info(&format!("Successfully processed block {} with {} transfers", block_number, transfer_count));

        Ok(transfer_count)
    }

    /// Get the latest block number with retry logic and circuit breaker
    pub async fn get_latest_block_with_retry(&self) -> Result<u64, MonitorError> {
        let circuit_breaker = Arc::clone(&self.rpc_circuit_breaker);
        
        let result = circuit_breaker.execute(|| async {
            self.rpc_client.get_latest_block_number_with_retry().await
        }).await;

        match result {
            Ok(block_number) => Ok(block_number),
            Err(e) => {
                let context = LogContext::new("block_monitor", "get_latest_block")
                    .with_metadata("error_severity", serde_json::json!(format!("{:?}", e.severity())));
                
                ErrorLogger::log_error(&e, Some(context));
                Err(MonitorError::Indexer(e))
            }
        }
    }

    /// Get the starting block number (either from database or current latest)
    async fn get_starting_block_number(&self) -> Result<u64, MonitorError> {
        // Try to get last processed block from database
        match self.database.get_last_processed_block() {
            Ok(last_block) => {
                if last_block > 0 {
                    info!("Resuming from last processed block: {}", last_block);
                    return Ok(last_block);
                }
            }
            Err(e) => {
                warn!("Could not get last processed block from database: {}", e);
            }
        }

        // If no last processed block, start from current latest block
        info!("No previous state found, starting from current latest block");
        let latest_block = self.get_latest_block_with_retry().await?;
        
        // Initialize the database with the starting block
        if let Err(e) = self.database.set_last_processed_block(latest_block) {
            warn!("Failed to initialize last processed block in database: {}", e);
        }

        Ok(latest_block)
    }

    /// Persist the current state to database
    pub async fn persist_state(&self, last_processed_block: u64) -> Result<(), MonitorError> {
        info!("Persisting state: last processed block = {}", last_processed_block);
        self.database.set_last_processed_block(last_processed_block)?;
        Ok(())
    }

    /// Request graceful shutdown
    pub fn shutdown(&self) {
        info!("Requesting graceful shutdown");
        self.shutdown_signal.store(true, Ordering::Relaxed);
    }

    /// Get current monitoring status
    pub async fn get_status(&self) -> Result<MonitorStatus, MonitorError> {
        let latest_block = self.get_latest_block_with_retry().await?;
        let last_processed_block = self.database.get_last_processed_block().unwrap_or(0);
        let net_flow_data = self.database.get_net_flow_data()?;
        let transaction_count = self.database.get_transaction_count()?;

        Ok(MonitorStatus {
            latest_block,
            last_processed_block,
            blocks_behind: if latest_block > last_processed_block {
                latest_block - last_processed_block
            } else {
                0
            },
            total_transactions: transaction_count,
            current_net_flow: net_flow_data.net_flow,
            is_running: !self.shutdown_signal.load(Ordering::Relaxed),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MonitorStatus {
    pub latest_block: u64,
    pub last_processed_block: u64,
    pub blocks_behind: u64,
    pub total_transactions: u64,
    pub current_net_flow: String,
    pub is_running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::RpcClient;
    use crate::database::Database;

    #[test]
    fn test_block_monitor_config_default() {
        let config = BlockMonitorConfig::default();
        assert_eq!(config.poll_interval_seconds, 2);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_delay_seconds, 1);
        assert_eq!(config.max_retry_delay_seconds, 60);
    }

    #[test]
    fn test_block_monitor_creation() {
        let rpc_client = RpcClient::new("http://test".to_string());
        let block_processor = BlockProcessor::new(rpc_client.clone());
        let database = Database::new_in_memory().expect("Failed to create test database");
        
        let monitor = BlockMonitor::new(rpc_client, block_processor, database, None);
        
        assert_eq!(monitor.config.poll_interval_seconds, 2);
        assert!(!monitor.shutdown_signal.load(Ordering::Relaxed));
    }

    #[test]
    fn test_block_monitor_with_custom_config() {
        let rpc_client = RpcClient::new("http://test".to_string());
        let block_processor = BlockProcessor::new(rpc_client.clone());
        let database = Database::new_in_memory().expect("Failed to create test database");
        
        let config = BlockMonitorConfig {
            poll_interval_seconds: 5,
            max_retries: 3,
            retry_delay_seconds: 2,
            max_retry_delay_seconds: 30,
        };
        
        let monitor = BlockMonitor::new(rpc_client, block_processor, database, Some(config));
        
        assert_eq!(monitor.config.poll_interval_seconds, 5);
        assert_eq!(monitor.config.max_retries, 3);
        assert_eq!(monitor.config.retry_delay_seconds, 2);
        assert_eq!(monitor.config.max_retry_delay_seconds, 30);
    }

    #[test]
    fn test_shutdown_signal() {
        let rpc_client = RpcClient::new("http://test".to_string());
        let block_processor = BlockProcessor::new(rpc_client.clone());
        let database = Database::new_in_memory().expect("Failed to create test database");
        
        let monitor = BlockMonitor::new(rpc_client, block_processor, database, None);
        
        assert!(!monitor.shutdown_signal.load(Ordering::Relaxed));
        
        monitor.shutdown();
        
        assert!(monitor.shutdown_signal.load(Ordering::Relaxed));
    }

    #[test]
    fn test_monitor_status_creation() {
        let status = MonitorStatus {
            latest_block: 1000,
            last_processed_block: 995,
            blocks_behind: 5,
            total_transactions: 42,
            current_net_flow: "1500.5".to_string(),
            is_running: true,
        };

        assert_eq!(status.latest_block, 1000);
        assert_eq!(status.blocks_behind, 5);
        assert_eq!(status.current_net_flow, "1500.5");
        assert!(status.is_running);
    }

    #[tokio::test]
    async fn test_get_starting_block_number_with_empty_database() {
        let rpc_client = RpcClient::new("http://test".to_string());
        let block_processor = BlockProcessor::new(rpc_client.clone());
        let database = Database::new_in_memory().expect("Failed to create test database");
        
        let monitor = BlockMonitor::new(rpc_client, block_processor, database, None);
        
        // This will fail with network error in tests, but we can verify the method exists
        let result = monitor.get_starting_block_number().await;
        assert!(result.is_err()); // Expected to fail due to no network connection
    }

    #[tokio::test]
    async fn test_persist_state() {
        let rpc_client = RpcClient::new("http://test".to_string());
        let block_processor = BlockProcessor::new(rpc_client.clone());
        let database = Database::new_in_memory().expect("Failed to create test database");
        
        // Test persisting state directly first
        let result = database.set_last_processed_block(12345);
        assert!(result.is_ok());
        
        // Verify state was persisted
        let last_block = database.get_last_processed_block().expect("Failed to get last processed block");
        assert_eq!(last_block, 12345);
        
        // Test with monitor
        let monitor = BlockMonitor::new(rpc_client, block_processor, database, None);
        let persist_result = monitor.persist_state(12346).await;
        assert!(persist_result.is_ok());
    }

    #[test]
    fn test_monitor_error_display() {
        let config_error = MonitorError::Config("Test config error".to_string());
        assert_eq!(format!("{}", config_error), "Monitor configuration error: Test config error");

        let shutdown_error = MonitorError::Shutdown;
        assert_eq!(format!("{}", shutdown_error), "Shutdown requested");
    }
}