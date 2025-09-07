mod blockchain;
mod database;
mod models;
mod api;
mod error;
mod error_recovery;
mod logging;
mod retry;
mod config;

#[cfg(test)]
mod error_tests;

use log::info;

use blockchain::{RpcClient, BlockProcessor, BlockMonitor, BlockMonitorConfig};
use database::Database;
use error::IndexerError;
use logging::{LogContext, ErrorLogger};
use config::AppConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Display welcome banner
    print_startup_banner();
    
    // Initialize structured logging
    if let Err(e) = logging::init_logging() {
        eprintln!("Failed to initialize logging: {}", e);
        return Err(e);
    }
    
    let context = LogContext::new("main", "startup");
    context.info("Starting Polygon POL Token Indexer");
    
    // Load configuration with enhanced error handling
    let config = match AppConfig::load() {
        Ok(config) => config,
        Err(e) => {
            let indexer_error = IndexerError::Config(e);
            ErrorLogger::log_error(&indexer_error, Some(LogContext::new("main", "configuration")));
            return Err(indexer_error.into());
        }
    };
    
    // Log configuration
    let config_context = LogContext::new("main", "configuration")
        .with_metadata("rpc_endpoint", serde_json::json!(config.rpc.endpoint))
        .with_metadata("database_path", serde_json::json!(config.database.path))
        .with_metadata("poll_interval_seconds", serde_json::json!(config.processing.poll_interval_seconds));
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
                blockchain::MonitorError::Config(msg) => IndexerError::Config(error::ConfigError::InvalidValue {
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

fn print_startup_banner() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              🚀 Polygon POL Token Indexer 🚀                ║");
    println!("║                                                              ║");
    println!("║         Real-time blockchain monitoring system              ║");
    println!("║                                                              ║");
    println!("║        Created by Agnivesh Kumar for Alfred Capital         ║");
    println!("║                        Assignment                            ║");
    println!("║                                                              ║");
    println!("║              Starting blockchain monitoring...               ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
}

/// Components structure
struct AppComponents {
    block_monitor: BlockMonitor,
}

/// Initialize all application components
async fn initialize_components(config: AppConfig) -> Result<AppComponents, IndexerError> {
    let context = LogContext::new("components", "initialization");
    
    // Initialize RPC client with timeout configuration
    context.debug("Initializing RPC client");
    let rpc_client = RpcClient::new_with_config(config.rpc.endpoint, config.rpc.timeout_seconds);
    
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
    let database = Database::new(&config.database.path)
        .map_err(|e| IndexerError::from(e))?;
    
    // Initialize block processor
    context.debug("Initializing block processor");
    let block_processor = BlockProcessor::new(rpc_client.clone());
    
    // Initialize block monitor with configuration
    context.debug("Initializing block monitor");
    let monitor_config = BlockMonitorConfig {
        poll_interval_seconds: config.processing.poll_interval_seconds,
        max_retries: config.rpc.max_retries,
        retry_delay_seconds: config.rpc.retry_delay_seconds,
        max_retry_delay_seconds: config.rpc.max_retry_delay_seconds,
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