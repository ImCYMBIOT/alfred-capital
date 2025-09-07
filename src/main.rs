mod blockchain;
mod database;
mod models;
mod api;

use log::{info, error};
use std::env;

use blockchain::{RpcClient, BlockProcessor, BlockMonitor, BlockMonitorConfig};
use database::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::init();
    
    info!("Starting Polygon POL Token Indexer");
    
    // Get configuration from environment variables
    let rpc_endpoint = env::var("POLYGON_RPC_URL")
        .unwrap_or_else(|_| "https://polygon-rpc.com/".to_string());
    let db_path = env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "./blockchain.db".to_string());
    let poll_interval = env::var("BLOCK_POLL_INTERVAL")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<u64>()
        .unwrap_or(2);
    
    info!("Configuration:");
    info!("  RPC Endpoint: {}", rpc_endpoint);
    info!("  Database Path: {}", db_path);
    info!("  Poll Interval: {} seconds", poll_interval);
    
    // Initialize components
    info!("Initializing components...");
    
    let rpc_client = RpcClient::new(rpc_endpoint);
    let block_processor = BlockProcessor::new(rpc_client.clone());
    let database = Database::new(&db_path)?;
    
    let monitor_config = BlockMonitorConfig {
        poll_interval_seconds: poll_interval,
        ..Default::default()
    };
    
    let block_monitor = BlockMonitor::new(
        rpc_client,
        block_processor,
        database,
        Some(monitor_config),
    );
    
    info!("Components initialized successfully");
    
    // Start block monitoring
    info!("Starting block monitoring...");
    match block_monitor.start().await {
        Ok(()) => {
            info!("Block monitor stopped normally");
        }
        Err(blockchain::MonitorError::Shutdown) => {
            info!("Block monitor stopped due to shutdown signal");
        }
        Err(e) => {
            error!("Block monitor stopped with error: {}", e);
            return Err(e.into());
        }
    }
    
    info!("Polygon POL Token Indexer stopped");
    Ok(())
}