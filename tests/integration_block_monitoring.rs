use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use polygon_pol_indexer::blockchain::{RpcClient, BlockProcessor, BlockMonitor, BlockMonitorConfig, MonitorError};
use polygon_pol_indexer::database::Database;
use polygon_pol_indexer::models::{ProcessedTransfer, TransferDirection};

/// Mock RPC client for testing that simulates network behavior
struct MockRpcClient {
    current_block: Arc<std::sync::atomic::AtomicU64>,
    should_fail: Arc<AtomicBool>,
    fail_count: Arc<std::sync::atomic::AtomicU32>,
}

impl MockRpcClient {
    fn new(starting_block: u64) -> Self {
        Self {
            current_block: Arc::new(std::sync::atomic::AtomicU64::new(starting_block)),
            should_fail: Arc::new(AtomicBool::new(false)),
            fail_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }

    fn advance_block(&self) {
        self.current_block.fetch_add(1, Ordering::Relaxed);
    }

    fn set_should_fail(&self, should_fail: bool) {
        self.should_fail.store(should_fail, Ordering::Relaxed);
    }

    fn get_fail_count(&self) -> u32 {
        self.fail_count.load(Ordering::Relaxed)
    }
}

#[tokio::test]
async fn test_block_monitor_initialization() {
    // Create test components
    let rpc_client = RpcClient::new("http://test".to_string());
    let block_processor = BlockProcessor::new(rpc_client.clone());
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    let config = BlockMonitorConfig {
        poll_interval_seconds: 1,
        max_retries: 3,
        retry_delay_seconds: 1,
        max_retry_delay_seconds: 10,
    };
    
    let monitor = BlockMonitor::new(rpc_client, block_processor, database, Some(config));
    
    // Test that monitor is created with correct configuration
    assert_eq!(monitor.config.poll_interval_seconds, 1);
    assert_eq!(monitor.config.max_retries, 3);
}

#[tokio::test]
async fn test_block_monitor_state_persistence() {
    // Create test database
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Create monitor components
    let rpc_client = RpcClient::new("http://test".to_string());
    let block_processor = BlockProcessor::new(rpc_client.clone());
    
    let config = BlockMonitorConfig {
        poll_interval_seconds: 1,
        max_retries: 1,
        retry_delay_seconds: 1,
        max_retry_delay_seconds: 5,
    };
    
    // Test state persistence directly on database first
    let test_block = 12345u64;
    let result = database.set_last_processed_block(test_block);
    assert!(result.is_ok(), "Failed to set last processed block: {:?}", result);
    
    // Verify state was persisted
    let retrieved_block = database.get_last_processed_block()
        .expect("Failed to retrieve last processed block");
    assert_eq!(retrieved_block, test_block);
    
    let monitor = BlockMonitor::new(rpc_client, block_processor, database, Some(config));
    
    // Test monitor state persistence
    let persist_result = monitor.persist_state(test_block + 1).await;
    assert!(persist_result.is_ok(), "Failed to persist state via monitor: {:?}", persist_result);
}

#[tokio::test]
async fn test_block_monitor_graceful_shutdown() {
    // Create test components
    let rpc_client = RpcClient::new("http://test".to_string());
    let block_processor = BlockProcessor::new(rpc_client.clone());
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    let config = BlockMonitorConfig {
        poll_interval_seconds: 1,
        max_retries: 1,
        retry_delay_seconds: 1,
        max_retry_delay_seconds: 5,
    };
    
    let monitor = BlockMonitor::new(rpc_client, block_processor, database, Some(config));
    
    // Test shutdown signal
    assert!(!monitor.shutdown_signal.load(Ordering::Relaxed));
    
    monitor.shutdown();
    
    assert!(monitor.shutdown_signal.load(Ordering::Relaxed));
}

#[tokio::test]
async fn test_block_monitor_status() {
    // Create test database with some initial data
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Set initial state
    database.set_last_processed_block(1000).expect("Failed to set last processed block");
    
    // Create monitor components
    let rpc_client = RpcClient::new("http://test".to_string());
    let block_processor = BlockProcessor::new(rpc_client.clone());
    
    let monitor = BlockMonitor::new(rpc_client, block_processor, database, None);
    
    // Test getting status (will fail due to network, but we can test the structure)
    let status_result = monitor.get_status().await;
    
    // Should fail with RPC error due to no network connection
    assert!(status_result.is_err());
    
    // Verify it's the expected error type
    match status_result {
        Err(MonitorError::Rpc(_)) => {
            // This is expected - we can't actually connect to RPC in tests
        }
        Err(other) => {
            panic!("Expected RPC error, got: {:?}", other);
        }
        Ok(_) => {
            panic!("Expected error due to no network connection");
        }
    }
}

#[tokio::test]
async fn test_block_monitor_error_handling() {
    // Create test components
    let rpc_client = RpcClient::new("http://invalid-endpoint".to_string());
    let block_processor = BlockProcessor::new(rpc_client.clone());
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    let config = BlockMonitorConfig {
        poll_interval_seconds: 1,
        max_retries: 2,
        retry_delay_seconds: 1,
        max_retry_delay_seconds: 5,
    };
    
    let monitor = BlockMonitor::new(rpc_client, block_processor, database, Some(config));
    
    // Test that get_latest_block_with_retry fails after max retries
    let start_time = std::time::Instant::now();
    let result = monitor.get_latest_block_with_retry().await;
    let elapsed = start_time.elapsed();
    
    // Should fail after retries
    assert!(result.is_err());
    
    // Should have taken some time due to retries (at least 1 second for first retry)
    assert!(elapsed >= Duration::from_secs(1));
    
    match result {
        Err(MonitorError::Rpc(_)) => {
            // Expected error type
        }
        Err(other) => {
            panic!("Expected RPC error, got: {:?}", other);
        }
        Ok(_) => {
            panic!("Expected error due to invalid endpoint");
        }
    }
}

#[tokio::test]
async fn test_block_monitor_database_integration() {
    // Create test database
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Create a test transfer
    let transfer = ProcessedTransfer {
        block_number: 1000,
        transaction_hash: "0xtest123".to_string(),
        log_index: 0,
        from_address: "0x1111111111111111111111111111111111111111".to_string(),
        to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(), // Binance address
        amount: "1000000000000000000".to_string(), // 1 POL
        timestamp: 1640995200,
        direction: TransferDirection::ToBinance,
    };
    
    // Store transfer and update net flow
    let result = database.store_transfer_and_update_net_flow(&transfer);
    assert!(result.is_ok(), "Failed to store transfer: {:?}", result);
    
    // Verify the transfer was stored
    let stored_transfer = database.get_transaction(&transfer.transaction_hash, transfer.log_index);
    assert!(stored_transfer.is_ok(), "Failed to retrieve stored transfer: {:?}", stored_transfer);
    
    let stored = stored_transfer.unwrap();
    assert_eq!(stored.block_number, transfer.block_number);
    assert_eq!(stored.amount, transfer.amount);
    assert_eq!(stored.direction, "inflow");
    
    // Verify net flow was updated
    let net_flow_data = database.get_net_flow_data();
    assert!(net_flow_data.is_ok(), "Failed to get net flow data: {:?}", net_flow_data);
    
    let net_flow = net_flow_data.unwrap();
    assert_eq!(net_flow.total_inflow, "1000000000000000000");
    assert_eq!(net_flow.total_outflow, "0");
    assert_eq!(net_flow.net_flow, "1000000000000000000");
}

#[tokio::test]
async fn test_block_monitor_configuration_validation() {
    // Test default configuration
    let default_config = BlockMonitorConfig::default();
    assert_eq!(default_config.poll_interval_seconds, 2);
    assert_eq!(default_config.max_retries, 5);
    assert_eq!(default_config.retry_delay_seconds, 1);
    assert_eq!(default_config.max_retry_delay_seconds, 60);
    
    // Test custom configuration
    let custom_config = BlockMonitorConfig {
        poll_interval_seconds: 5,
        max_retries: 3,
        retry_delay_seconds: 2,
        max_retry_delay_seconds: 30,
    };
    
    let rpc_client = RpcClient::new("http://test".to_string());
    let block_processor = BlockProcessor::new(rpc_client.clone());
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    let monitor = BlockMonitor::new(rpc_client, block_processor, database, Some(custom_config));
    
    assert_eq!(monitor.config.poll_interval_seconds, 5);
    assert_eq!(monitor.config.max_retries, 3);
    assert_eq!(monitor.config.retry_delay_seconds, 2);
    assert_eq!(monitor.config.max_retry_delay_seconds, 30);
}

#[tokio::test]
async fn test_block_monitor_resume_from_last_block() {
    // Create test database with existing state
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Set a last processed block
    let last_block = 5000u64;
    database.set_last_processed_block(last_block).expect("Failed to set last processed block");
    
    // Verify the block was set
    let retrieved_block = database.get_last_processed_block().expect("Failed to get last processed block");
    assert_eq!(retrieved_block, last_block);
    
    // Create monitor
    let rpc_client = RpcClient::new("http://test".to_string());
    let block_processor = BlockProcessor::new(rpc_client.clone());
    let _monitor = BlockMonitor::new(rpc_client, block_processor, database, None);
}

#[tokio::test]
async fn test_monitor_error_types() {
    // Test error type creation and display
    let config_error = MonitorError::Config("Invalid configuration".to_string());
    assert_eq!(format!("{}", config_error), "Monitor configuration error: Invalid configuration");
    
    let shutdown_error = MonitorError::Shutdown;
    assert_eq!(format!("{}", shutdown_error), "Shutdown requested");
    
    // Test error conversion
    let db_error = polygon_pol_indexer::database::DbError::Operation("Test error".to_string());
    let monitor_error: MonitorError = db_error.into();
    
    match monitor_error {
        MonitorError::Database(_) => {
            // Expected conversion
        }
        other => {
            panic!("Expected Database error, got: {:?}", other);
        }
    }
}

// Integration test that simulates a complete monitoring workflow
#[tokio::test]
async fn test_complete_monitoring_workflow() {
    // This test simulates the complete workflow without actual network calls
    
    // Create test database
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Verify initial state
    let initial_count = database.get_transaction_count().expect("Failed to get transaction count");
    assert_eq!(initial_count, 0);
    
    let initial_net_flow = database.get_net_flow_data().expect("Failed to get net flow data");
    assert_eq!(initial_net_flow.total_inflow, "0");
    assert_eq!(initial_net_flow.total_outflow, "0");
    assert_eq!(initial_net_flow.net_flow, "0");
    assert_eq!(initial_net_flow.last_processed_block, 0);
    
    // Simulate processing some transfers
    let transfers = vec![
        ProcessedTransfer {
            block_number: 1001,
            transaction_hash: "0xabc123".to_string(),
            log_index: 0,
            from_address: "0x1111111111111111111111111111111111111111".to_string(),
            to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
            amount: "2000000000000000000".to_string(), // 2 POL
            timestamp: 1640995200,
            direction: TransferDirection::ToBinance,
        },
        ProcessedTransfer {
            block_number: 1002,
            transaction_hash: "0xdef456".to_string(),
            log_index: 0,
            from_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
            to_address: "0x2222222222222222222222222222222222222222".to_string(),
            amount: "500000000000000000".to_string(), // 0.5 POL
            timestamp: 1640995260,
            direction: TransferDirection::FromBinance,
        },
    ];
    
    // Process each transfer
    for transfer in &transfers {
        let result = database.store_transfer_and_update_net_flow(transfer);
        assert!(result.is_ok(), "Failed to store transfer: {:?}", result);
    }
    
    // Update last processed block
    database.set_last_processed_block(1002).expect("Failed to set last processed block");
    
    // Verify final state
    let final_count = database.get_transaction_count().expect("Failed to get transaction count");
    assert_eq!(final_count, 2);
    
    let final_net_flow = database.get_net_flow_data().expect("Failed to get net flow data");
    assert_eq!(final_net_flow.total_inflow, "2000000000000000000");
    assert_eq!(final_net_flow.total_outflow, "500000000000000000");
    assert_eq!(final_net_flow.net_flow, "1500000000000000000"); // 1.5 POL net inflow
    assert_eq!(final_net_flow.last_processed_block, 1002);
    
    // Verify individual transactions
    let tx1 = database.get_transaction("0xabc123", 0).expect("Failed to get transaction 1");
    assert_eq!(tx1.direction, "inflow");
    assert_eq!(tx1.amount, "2000000000000000000");
    
    let tx2 = database.get_transaction("0xdef456", 0).expect("Failed to get transaction 2");
    assert_eq!(tx2.direction, "outflow");
    assert_eq!(tx2.amount, "500000000000000000");
}