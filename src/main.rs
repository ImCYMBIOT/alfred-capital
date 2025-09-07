mod blockchain;
mod database;
mod models;
mod api;
mod error;
mod logging;
mod retry;

#[cfg(test)]
mod error_tests;

use log::info;
use std::env;

use blockchain::{RpcClient, BlockProcessor, BlockMonitor, BlockMonitorConfig};
use database::Database;
use error::{IndexerError, ConfigError};
use logging::{LogContext, ErrorLogger};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    if let Err(e) = logging::init_logging() {
        eprintln!("Failed to initialize logging: {}", e);
        return Err(e);
    }
    
    let context = LogContext::new("main", "startup");
    context.info("Starting Polygon POL Token Indexer");
    
    // Load configuration with enhanced error handling
    let config = match load_configuration() {
        Ok(config) => config,
        Err(e) => {
            ErrorLogger::log_error(&e, Some(LogContext::new("main", "configuration")));
            return Err(e.into());
        }
    };
    
    // Log configuration
    let config_context = LogContext::new("main", "configuration")
        .with_metadata("rpc_endpoint", serde_json::json!(config.rpc_endpoint))
        .with_metadata("database_path", serde_json::json!(config.db_path))
        .with_metadata("poll_interval_seconds", serde_json::json!(config.poll_interval));
    config_context.info("Configuration loaded successfully");
    
    // Initialize components with enhanced error handling
    let context = LogContext::new("main", "initialization");
    context.info("Initializing components...");
    
    let components = match initialize_components(config).await {
        Ok(components) => components,
        Err(e) => {
            ErrorLogger::log_error(&e, Some(LogContext::new("main", "initialization")));
            return Err(e.into());
        }
    };
    
    context.info("Components initialized successfully");
    
    // Start block monitoring with enhanced error handling
    let context = LogContext::new("main", "monitoring");
    context.info("Starting block monitoring...");
    
    match components.block_monitor.start().await {
        Ok(()) => {
            context.info("Block monitor stopped normally");
        }
        Err(blockchain::MonitorError::Shutdown) => {
            context.info("Block monitor stopped due to shutdown signal");
        }
        Err(e) => {
            let error = match e {
                blockchain::MonitorError::Indexer(indexer_error) => indexer_error,
                blockchain::MonitorError::Config(msg) => IndexerError::Config(ConfigError::InvalidValue {
                    key: "monitor_config".to_string(),
                    value: msg,
                }),
                blockchain::MonitorError::Shutdown => {
                    context.info("Shutdown requested");
                    return Ok(());
                }
            };
            ErrorLogger::log_error(&error, Some(LogContext::new("main", "monitoring")));
            return Err(error.into());
        }
    }
    
    let context = LogContext::new("main", "shutdown");
    context.info("Polygon POL Token Indexer stopped");
    Ok(())
}

/// Configuration structure
struct AppConfig {
    rpc_endpoint: String,
    db_path: String,
    poll_interval: u64,
    rpc_timeout_seconds: u64,
}

/// Components structure
struct AppComponents {
    block_monitor: BlockMonitor,
}

/// Load and validate configuration from environment variables
fn load_configuration() -> Result<AppConfig, IndexerError> {
    let context = LogContext::new("config", "load");
    context.debug("Loading configuration from environment variables");
    
    let rpc_endpoint = env::var("POLYGON_RPC_URL")
        .unwrap_or_else(|_| "https://polygon-rpc.com/".to_string());
    
    // Validate RPC endpoint URL
    if !rpc_endpoint.starts_with("http://") && !rpc_endpoint.starts_with("https://") {
        return Err(IndexerError::Config(ConfigError::InvalidUrl(rpc_endpoint)));
    }
    
    let db_path = env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "./blockchain.db".to_string());
    
    let poll_interval = env::var("BLOCK_POLL_INTERVAL")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<u64>()
        .map_err(|_| IndexerError::Config(ConfigError::InvalidValue {
            key: "BLOCK_POLL_INTERVAL".to_string(),
            value: env::var("BLOCK_POLL_INTERVAL").unwrap_or_default(),
        }))?;
    
    // Validate poll interval
    if poll_interval == 0 || poll_interval > 300 {
        return Err(IndexerError::Config(ConfigError::InvalidValue {
            key: "BLOCK_POLL_INTERVAL".to_string(),
            value: poll_interval.to_string(),
        }));
    }
    
    let rpc_timeout_seconds = env::var("RPC_TIMEOUT_SECONDS")
        .unwrap_or_else(|_| "30".to_string())
        .parse::<u64>()
        .map_err(|_| IndexerError::Config(ConfigError::InvalidValue {
            key: "RPC_TIMEOUT_SECONDS".to_string(),
            value: env::var("RPC_TIMEOUT_SECONDS").unwrap_or_default(),
        }))?;
    
    Ok(AppConfig {
        rpc_endpoint,
        db_path,
        poll_interval,
        rpc_timeout_seconds,
    })
}

/// Initialize all application components
async fn initialize_components(config: AppConfig) -> Result<AppComponents, IndexerError> {
    let context = LogContext::new("components", "initialization");
    
    // Initialize RPC client with timeout configuration
    context.debug("Initializing RPC client");
    let rpc_client = RpcClient::new_with_config(config.rpc_endpoint, config.rpc_timeout_seconds);
    
    // Test RPC connection
    context.debug("Testing RPC connection");
    match rpc_client.get_latest_block_number_with_retry().await {
        Ok(block_number) => {
            let test_context = LogContext::new("components", "rpc_test")
                .with_block_number(block_number);
            test_context.info("RPC connection test successful");
        }
        Err(e) => {
            let test_context = LogContext::new("components", "rpc_test");
            ErrorLogger::log_error(&e, Some(test_context));
            return Err(e);
        }
    }
    
    // Initialize database
    context.debug("Initializing database");
    let database = Database::new(&config.db_path)
        .map_err(|e| IndexerError::from(e))?;
    
    // Initialize block processor
    context.debug("Initializing block processor");
    let block_processor = BlockProcessor::new(rpc_client.clone());
    
    // Initialize block monitor with configuration
    context.debug("Initializing block monitor");
    let monitor_config = BlockMonitorConfig {
        poll_interval_seconds: config.poll_interval,
        max_retries: 5,
        retry_delay_seconds: 2,
        max_retry_delay_seconds: 60,
    };
    
    let block_monitor = BlockMonitor::new(
        rpc_client,
        block_processor,
        database,
        Some(monitor_config),
    );
    
    Ok(AppComponents {
        block_monitor,
    })
}