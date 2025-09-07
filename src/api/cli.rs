use clap::{Parser, Subcommand};
use thiserror::Error;
use crate::database::Database;
use std::sync::Arc;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("CLI operation failed: {0}")]
    Operation(String),
    #[error("Database error: {0}")]
    Database(#[from] crate::database::DbError),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

#[derive(Parser)]
#[command(name = "polygon-pol-indexer")]
#[command(about = "A CLI tool for querying POL token net-flow data\nCreated by Agnivesh Kumar for Alfred Capital assignment")]
#[command(version = "0.1.0")]
#[command(long_about = "Polygon POL Token Indexer - Real-time blockchain data analysis\n\nThis tool provides access to POL token transfer data and net-flow calculations\nfor Binance exchange addresses on the Polygon network.\n\nCreated by Agnivesh Kumar for Alfred Capital assignment")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    /// Database path
    #[arg(long, default_value = "./blockchain.db")]
    pub database: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Display current cumulative net-flow
    NetFlow,
    /// Show system status and last processed block
    Status,
    /// Display recent transactions with pagination
    Transactions {
        /// Number of transactions to display
        #[arg(short, long, default_value = "10")]
        limit: u32,
        /// Number of transactions to skip
        #[arg(short, long, default_value = "0")]
        offset: u32,
    },
}

pub struct CliHandler {
    database: Arc<Database>,
}

impl CliHandler {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Handle net-flow query command
    pub async fn handle_net_flow_query(&self) -> Result<(), CliError> {
        let net_flow_data = self.database.get_net_flow_data()?;
        
        println!("=== POL Token Net-Flow Data ===");
        println!("Total Inflow:  {} POL", net_flow_data.total_inflow);
        println!("Total Outflow: {} POL", net_flow_data.total_outflow);
        println!("Net Flow:      {} POL", net_flow_data.net_flow);
        println!("Last Updated:  {}", format_timestamp(net_flow_data.last_updated));
        
        Ok(())
    }

    /// Handle status query command
    pub async fn handle_status_query(&self) -> Result<(), CliError> {
        let net_flow_data = self.database.get_net_flow_data()?;
        let transaction_count = self.database.get_transaction_count()?;
        
        println!("=== System Status ===");
        println!("Last Processed Block: {}", net_flow_data.last_processed_block);
        println!("Total Transactions:   {}", transaction_count);
        println!("Last Updated:         {}", format_timestamp(net_flow_data.last_updated));
        println!("Database Status:      Connected");
        
        Ok(())
    }

    /// Handle recent transactions query with pagination
    pub async fn handle_recent_transactions(&self, limit: u32, offset: u32) -> Result<(), CliError> {
        // Validate input parameters
        if limit == 0 {
            return Err(CliError::InvalidArgument("Limit must be greater than 0".to_string()));
        }
        if limit > 1000 {
            return Err(CliError::InvalidArgument("Limit cannot exceed 1000".to_string()));
        }

        let transactions = self.database.get_recent_transactions(limit, offset)?;
        let total_count = self.database.get_transaction_count()?;
        
        if transactions.is_empty() {
            if offset == 0 {
                println!("No transactions found.");
            } else {
                println!("No more transactions found at offset {}.", offset);
            }
            return Ok(());
        }

        println!("=== Recent Transactions ===");
        println!("Showing {} transactions (offset: {}, total: {})", transactions.len(), offset, total_count);
        println!();
        
        for (i, tx) in transactions.iter().enumerate() {
            println!("Transaction #{}", offset + i as u32 + 1);
            println!("  Block:     {}", tx.block_number);
            println!("  Hash:      {}", tx.transaction_hash);
            println!("  Log Index: {}", tx.log_index);
            println!("  From:      {}", tx.from_address);
            println!("  To:        {}", tx.to_address);
            println!("  Amount:    {} POL", tx.amount);
            println!("  Direction: {}", tx.direction);
            println!("  Timestamp: {}", format_timestamp(tx.timestamp));
            println!("  Created:   {}", format_timestamp(tx.created_at));
            
            if i < transactions.len() - 1 {
                println!();
            }
        }
        
        // Show pagination info
        if offset + limit < total_count as u32 {
            println!();
            println!("Use --offset {} to see more transactions", offset + limit);
        }
        
        Ok(())
    }

    /// Execute CLI command based on parsed arguments
    pub async fn execute_command(&self, command: &Commands) -> Result<(), CliError> {
        match command {
            Commands::NetFlow => self.handle_net_flow_query().await,
            Commands::Status => self.handle_status_query().await,
            Commands::Transactions { limit, offset } => {
                self.handle_recent_transactions(*limit, *offset).await
            }
        }
    }
}

/// Format Unix timestamp to human-readable string
fn format_timestamp(timestamp: u64) -> String {
    use std::time::{UNIX_EPOCH, Duration};
    
    match UNIX_EPOCH.checked_add(Duration::from_secs(timestamp)) {
        Some(datetime) => {
            // Simple formatting - in a real application you might want to use chrono
            format!("{:?}", datetime)
        }
        None => format!("Invalid timestamp: {}", timestamp),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::models::{ProcessedTransfer, TransferDirection};
    use std::sync::Arc;

    async fn setup_test_database() -> Arc<Database> {
        let db = Database::new_in_memory().expect("Failed to create test database");
        Arc::new(db)
    }

    async fn populate_test_data(db: &Database) {
        // Add some test transactions
        let transfers = vec![
            ProcessedTransfer {
                block_number: 100,
                transaction_hash: "0x1234567890abcdef".to_string(),
                log_index: 0,
                from_address: "0xsender1".to_string(),
                to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(), // Binance
                amount: "1000.5".to_string(),
                timestamp: 1640995200, // 2022-01-01 00:00:00 UTC
                direction: TransferDirection::ToBinance,
            },
            ProcessedTransfer {
                block_number: 101,
                transaction_hash: "0xfedcba0987654321".to_string(),
                log_index: 1,
                from_address: "0xe7804c37c13166ff0b37f5ae0bb07a3aebb6e245".to_string(), // Binance
                to_address: "0xreceiver1".to_string(),
                amount: "500.25".to_string(),
                timestamp: 1640995260, // 2022-01-01 00:01:00 UTC
                direction: TransferDirection::FromBinance,
            },
            ProcessedTransfer {
                block_number: 102,
                transaction_hash: "0xabcdef1234567890".to_string(),
                log_index: 0,
                from_address: "0xsender2".to_string(),
                to_address: "0x505e71695e9bc45943c58adec1650577bca68fd9".to_string(), // Binance
                amount: "2500.0".to_string(),
                timestamp: 1640995320, // 2022-01-01 00:02:00 UTC
                direction: TransferDirection::ToBinance,
            },
        ];

        for transfer in transfers {
            db.store_transfer_and_update_net_flow(&transfer)
                .expect("Failed to store test transfer");
        }

        // Update last processed block
        db.set_last_processed_block(102)
            .expect("Failed to set last processed block");
    }

    #[tokio::test]
    async fn test_handle_net_flow_query() {
        let db = setup_test_database().await;
        populate_test_data(&db).await;
        
        let cli_handler = CliHandler::new(db);
        
        // This should not panic and should return Ok
        let result = cli_handler.handle_net_flow_query().await;
        assert!(result.is_ok(), "Net flow query should succeed");
    }

    #[tokio::test]
    async fn test_handle_status_query() {
        let db = setup_test_database().await;
        populate_test_data(&db).await;
        
        let cli_handler = CliHandler::new(db);
        
        // This should not panic and should return Ok
        let result = cli_handler.handle_status_query().await;
        assert!(result.is_ok(), "Status query should succeed");
    }

    #[tokio::test]
    async fn test_handle_recent_transactions_valid_params() {
        let db = setup_test_database().await;
        populate_test_data(&db).await;
        
        let cli_handler = CliHandler::new(db);
        
        // Test with valid parameters
        let result = cli_handler.handle_recent_transactions(10, 0).await;
        assert!(result.is_ok(), "Recent transactions query should succeed with valid params");
        
        // Test with limit and offset
        let result = cli_handler.handle_recent_transactions(2, 1).await;
        assert!(result.is_ok(), "Recent transactions query should succeed with limit and offset");
    }

    #[tokio::test]
    async fn test_handle_recent_transactions_invalid_limit() {
        let db = setup_test_database().await;
        let cli_handler = CliHandler::new(db);
        
        // Test with zero limit
        let result = cli_handler.handle_recent_transactions(0, 0).await;
        assert!(result.is_err(), "Should fail with zero limit");
        
        match result.unwrap_err() {
            CliError::InvalidArgument(msg) => {
                assert!(msg.contains("greater than 0"));
            }
            _ => panic!("Expected InvalidArgument error"),
        }
        
        // Test with limit too high
        let result = cli_handler.handle_recent_transactions(1001, 0).await;
        assert!(result.is_err(), "Should fail with limit > 1000");
        
        match result.unwrap_err() {
            CliError::InvalidArgument(msg) => {
                assert!(msg.contains("cannot exceed 1000"));
            }
            _ => panic!("Expected InvalidArgument error"),
        }
    }

    #[tokio::test]
    async fn test_handle_recent_transactions_empty_database() {
        let db = setup_test_database().await;
        let cli_handler = CliHandler::new(db);
        
        // Test with empty database
        let result = cli_handler.handle_recent_transactions(10, 0).await;
        assert!(result.is_ok(), "Should succeed even with empty database");
    }

    #[tokio::test]
    async fn test_handle_recent_transactions_high_offset() {
        let db = setup_test_database().await;
        populate_test_data(&db).await;
        
        let cli_handler = CliHandler::new(db);
        
        // Test with offset beyond available data
        let result = cli_handler.handle_recent_transactions(10, 100).await;
        assert!(result.is_ok(), "Should succeed even with high offset");
    }

    #[tokio::test]
    async fn test_execute_command_net_flow() {
        let db = setup_test_database().await;
        populate_test_data(&db).await;
        
        let cli_handler = CliHandler::new(db);
        let command = Commands::NetFlow;
        
        let result = cli_handler.execute_command(&command).await;
        assert!(result.is_ok(), "Execute net flow command should succeed");
    }

    #[tokio::test]
    async fn test_execute_command_status() {
        let db = setup_test_database().await;
        populate_test_data(&db).await;
        
        let cli_handler = CliHandler::new(db);
        let command = Commands::Status;
        
        let result = cli_handler.execute_command(&command).await;
        assert!(result.is_ok(), "Execute status command should succeed");
    }

    #[tokio::test]
    async fn test_execute_command_transactions() {
        let db = setup_test_database().await;
        populate_test_data(&db).await;
        
        let cli_handler = CliHandler::new(db);
        let command = Commands::Transactions { limit: 5, offset: 0 };
        
        let result = cli_handler.execute_command(&command).await;
        assert!(result.is_ok(), "Execute transactions command should succeed");
    }

    #[tokio::test]
    async fn test_cli_handler_with_database_error() {
        // Create a database and then close it to simulate connection issues
        let db = Database::new_in_memory().expect("Failed to create test database");
        let db_arc = Arc::new(db);
        
        // Drop the original reference to potentially cause issues
        drop(db_arc.clone());
        
        let cli_handler = CliHandler::new(db_arc);
        
        // These operations should still work since we're using Arc
        let result = cli_handler.handle_net_flow_query().await;
        assert!(result.is_ok(), "Should work with Arc even after dropping reference");
    }

    #[test]
    fn test_format_timestamp() {
        // Test with a known timestamp
        let timestamp = 1640995200; // 2022-01-01 00:00:00 UTC
        let formatted = format_timestamp(timestamp);
        
        // Just verify it doesn't panic and returns a string
        assert!(!formatted.is_empty());
        assert!(!formatted.contains("Invalid timestamp"));
    }

    #[test]
    fn test_format_invalid_timestamp() {
        // Test with an invalid timestamp (too large)
        let timestamp = u64::MAX;
        let formatted = format_timestamp(timestamp);
        
        // Should handle invalid timestamps gracefully
        assert!(formatted.contains("Invalid timestamp"));
    }
}