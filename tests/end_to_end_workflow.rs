use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tempfile::TempDir;

use polygon_pol_indexer::blockchain::{RpcClient, BlockProcessor, BlockMonitor, BlockMonitorConfig};
use polygon_pol_indexer::database::Database;
use polygon_pol_indexer::models::{ProcessedTransfer, TransferDirection, NetFlowData};
use polygon_pol_indexer::api::cli::CliHandler;

/// End-to-end test of the complete block processing workflow
/// This test simulates the entire system from RPC calls to database storage to CLI queries
#[tokio::test]
async fn test_complete_end_to_end_workflow() {
    // Create temporary database
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");
    let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
    
    // Verify initial state
    let initial_net_flow = database.get_net_flow_data().expect("Failed to get initial net flow");
    assert_eq!(initial_net_flow.total_inflow, "0");
    assert_eq!(initial_net_flow.total_outflow, "0");
    assert_eq!(initial_net_flow.net_flow, "0");
    assert_eq!(initial_net_flow.last_processed_block, 0);
    
    // Simulate processing multiple blocks with transfers
    let test_transfers = create_test_transfer_sequence();
    
    // Process each transfer (simulating block processing)
    for transfer in &test_transfers {
        let result = database.store_transfer_and_update_net_flow(transfer);
        assert!(result.is_ok(), "Failed to store transfer: {:?}", result);
        
        // Update last processed block
        database.set_last_processed_block(transfer.block_number)
            .expect("Failed to update last processed block");
    }
    
    // Verify final state
    let final_net_flow = database.get_net_flow_data().expect("Failed to get final net flow");
    
    // Calculate expected values
    let expected_inflow = "3500000000000000000"; // 3.5 POL
    let expected_outflow = "1200000000000000000"; // 1.2 POL
    let expected_net_flow = "2300000000000000000"; // 2.3 POL
    
    assert_eq!(final_net_flow.total_inflow, expected_inflow);
    assert_eq!(final_net_flow.total_outflow, expected_outflow);
    assert_eq!(final_net_flow.net_flow, expected_net_flow);
    assert_eq!(final_net_flow.last_processed_block, 1005);
    
    // Test CLI query functionality
    let cli_handler = CliHandler::new(database.clone());
    
    // Test net flow query
    let net_flow_result = cli_handler.get_net_flow_data().await;
    assert!(net_flow_result.is_ok(), "CLI net flow query failed");
    
    let cli_net_flow = net_flow_result.unwrap();
    assert_eq!(cli_net_flow.net_flow, expected_net_flow);
    
    // Test status query
    let status_result = cli_handler.get_system_status().await;
    assert!(status_result.is_ok(), "CLI status query failed");
    
    let status = status_result.unwrap();
    assert_eq!(status.last_processed_block, 1005);
    assert!(status.total_transactions > 0);
    
    // Test recent transactions query
    let transactions_result = cli_handler.get_recent_transactions(10).await;
    assert!(transactions_result.is_ok(), "CLI transactions query failed");
    
    let transactions = transactions_result.unwrap();
    assert_eq!(transactions.len(), test_transfers.len());
    
    println!("End-to-end workflow test completed successfully");
    println!("Final net flow: {} POL", format_wei_to_pol(&final_net_flow.net_flow));
    println!("Total transactions processed: {}", transactions.len());
}

/// Test the complete monitoring workflow with mock data
#[tokio::test]
async fn test_complete_monitoring_workflow() {
    // Create temporary database
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("monitor_test.db");
    let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
    
    // Create monitor components (will fail on actual RPC calls, but tests the structure)
    let rpc_client = RpcClient::new("http://localhost:8545".to_string());
    let block_processor = BlockProcessor::new(rpc_client.clone());
    
    let config = BlockMonitorConfig {
        poll_interval_seconds: 1,
        max_retries: 2,
        retry_delay_seconds: 1,
        max_retry_delay_seconds: 5,
    };
    
    let monitor = BlockMonitor::new(rpc_client, block_processor, database.clone(), Some(config));
    
    // Test monitor initialization
    assert_eq!(monitor.config.poll_interval_seconds, 1);
    assert_eq!(monitor.config.max_retries, 2);
    
    // Simulate manual processing of transfers (since we can't connect to real RPC)
    let test_transfers = create_test_transfer_sequence();
    
    for transfer in &test_transfers {
        let result = database.store_transfer_and_update_net_flow(transfer);
        assert!(result.is_ok(), "Failed to store transfer in monitoring test");
        
        // Simulate monitor state persistence
        let persist_result = monitor.persist_state(transfer.block_number).await;
        assert!(persist_result.is_ok(), "Failed to persist monitor state");
    }
    
    // Test monitor status (will fail with RPC error, but tests the method)
    let status_result = monitor.get_status().await;
    assert!(status_result.is_err(), "Expected RPC error in test environment");
    
    // Test graceful shutdown
    monitor.shutdown();
    assert!(monitor.is_shutdown_requested());
    
    println!("Complete monitoring workflow test completed");
}

/// Test error recovery and resilience in the workflow
#[tokio::test]
async fn test_workflow_error_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("error_test.db");
    let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
    
    // Test database recovery after errors
    let valid_transfer = ProcessedTransfer {
        block_number: 1000,
        transaction_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        log_index: 0,
        from_address: "0x1111111111111111111111111111111111111111".to_string(),
        to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
        amount: "1000000000000000000".to_string(),
        timestamp: 1640995200,
        direction: TransferDirection::ToBinance,
    };
    
    // Store valid transfer
    let result = database.store_transfer_and_update_net_flow(&valid_transfer);
    assert!(result.is_ok(), "Failed to store valid transfer");
    
    // Try to store duplicate (should handle gracefully)
    let duplicate_result = database.store_transfer_and_update_net_flow(&valid_transfer);
    // This might succeed or fail depending on implementation, but shouldn't crash
    match duplicate_result {
        Ok(_) => println!("Duplicate transfer handled gracefully"),
        Err(e) => println!("Duplicate transfer rejected as expected: {}", e),
    }
    
    // Verify database is still functional
    let net_flow = database.get_net_flow_data().expect("Database should still be functional");
    assert_eq!(net_flow.total_inflow, "1000000000000000000");
    
    // Test RPC client error handling
    let rpc_client = RpcClient::new("http://invalid-endpoint".to_string());
    let processor = BlockProcessor::new(rpc_client);
    
    // This should fail gracefully
    let result = timeout(Duration::from_secs(5), processor.process_block(12345)).await;
    
    match result {
        Ok(Err(_)) => println!("RPC error handled gracefully"),
        Err(_) => println!("RPC timeout handled gracefully"),
        Ok(Ok(_)) => panic!("Should not succeed with invalid endpoint"),
    }
    
    println!("Error recovery test completed successfully");
}

/// Test data consistency across the entire workflow
#[tokio::test]
async fn test_data_consistency_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("consistency_test.db");
    let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
    
    // Create a sequence of transfers that should result in specific net flow
    let transfers = vec![
        create_transfer(1001, "0xaaa", "binance", "1000000000000000000", TransferDirection::ToBinance),
        create_transfer(1002, "binance", "0xbbb", "300000000000000000", TransferDirection::FromBinance),
        create_transfer(1003, "0xccc", "binance", "2000000000000000000", TransferDirection::ToBinance),
        create_transfer(1004, "binance", "0xddd", "500000000000000000", TransferDirection::FromBinance),
        create_transfer(1005, "0xeee", "0xfff", "1000000000000000000", TransferDirection::NotRelevant),
    ];
    
    // Process transfers and track expected values
    let mut expected_inflow = 0u128;
    let mut expected_outflow = 0u128;
    let mut transaction_count = 0;
    
    for transfer in &transfers {
        let result = database.store_transfer_and_update_net_flow(transfer);
        assert!(result.is_ok(), "Failed to store transfer");
        
        match transfer.direction {
            TransferDirection::ToBinance => {
                expected_inflow += transfer.amount.parse::<u128>().unwrap();
                transaction_count += 1;
            }
            TransferDirection::FromBinance => {
                expected_outflow += transfer.amount.parse::<u128>().unwrap();
                transaction_count += 1;
            }
            TransferDirection::NotRelevant => {
                // Should not be stored
            }
        }
        
        database.set_last_processed_block(transfer.block_number)
            .expect("Failed to update last processed block");
    }
    
    // Verify final consistency
    let net_flow = database.get_net_flow_data().expect("Failed to get net flow");
    let expected_net = expected_inflow - expected_outflow;
    
    assert_eq!(net_flow.total_inflow, expected_inflow.to_string());
    assert_eq!(net_flow.total_outflow, expected_outflow.to_string());
    assert_eq!(net_flow.net_flow, expected_net.to_string());
    assert_eq!(net_flow.last_processed_block, 1005);
    
    // Verify transaction count
    let stored_count = database.get_transaction_count().expect("Failed to get transaction count");
    assert_eq!(stored_count, transaction_count);
    
    // Verify individual transactions
    for transfer in &transfers {
        if matches!(transfer.direction, TransferDirection::ToBinance | TransferDirection::FromBinance) {
            let stored = database.get_transaction(&transfer.transaction_hash, transfer.log_index);
            assert!(stored.is_ok(), "Failed to retrieve transaction");
            
            let stored_tx = stored.unwrap();
            assert_eq!(stored_tx.amount, transfer.amount);
            assert_eq!(stored_tx.block_number, transfer.block_number);
        }
    }
    
    println!("Data consistency test completed successfully");
    println!("Expected inflow: {} POL", format_wei_to_pol(&expected_inflow.to_string()));
    println!("Expected outflow: {} POL", format_wei_to_pol(&expected_outflow.to_string()));
    println!("Expected net flow: {} POL", format_wei_to_pol(&expected_net.to_string()));
}

/// Test concurrent access and thread safety
#[tokio::test]
async fn test_concurrent_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("concurrent_test.db");
    let database = Arc::new(Database::new(db_path.to_str().unwrap()).expect("Failed to create database"));
    
    // Create multiple tasks that process transfers concurrently
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let db = database.clone();
        let handle = tokio::spawn(async move {
            let transfer = create_transfer(
                1000 + i,
                "0x1111111111111111111111111111111111111111",
                "binance",
                "1000000000000000000",
                TransferDirection::ToBinance,
            );
            
            let result = db.store_transfer_and_update_net_flow(&transfer);
            assert!(result.is_ok(), "Concurrent transfer storage failed");
            
            db.set_last_processed_block(transfer.block_number)
                .expect("Failed to update block number concurrently");
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Concurrent task failed");
    }
    
    // Verify final state
    let net_flow = database.get_net_flow_data().expect("Failed to get net flow after concurrent access");
    assert_eq!(net_flow.total_inflow, "5000000000000000000"); // 5 POL
    assert_eq!(net_flow.total_outflow, "0");
    assert_eq!(net_flow.net_flow, "5000000000000000000");
    
    let transaction_count = database.get_transaction_count().expect("Failed to get transaction count");
    assert_eq!(transaction_count, 5);
    
    println!("Concurrent workflow test completed successfully");
}

// Helper functions

fn create_test_transfer_sequence() -> Vec<ProcessedTransfer> {
    vec![
        create_transfer(1001, "0x1111", "binance", "1000000000000000000", TransferDirection::ToBinance),
        create_transfer(1002, "0x2222", "binance", "2500000000000000000", TransferDirection::ToBinance),
        create_transfer(1003, "binance", "0x3333", "700000000000000000", TransferDirection::FromBinance),
        create_transfer(1004, "binance", "0x4444", "500000000000000000", TransferDirection::FromBinance),
        create_transfer(1005, "0x5555", "0x6666", "1000000000000000000", TransferDirection::NotRelevant),
    ]
}

fn create_transfer(
    block_number: u64,
    from: &str,
    to: &str,
    amount: &str,
    direction: TransferDirection,
) -> ProcessedTransfer {
    let from_address = if from == "binance" {
        "0xf977814e90da44bfa03b6295a0616a897441acec".to_string()
    } else {
        format!("{}1111111111111111111111111111111111111111", from)
    };
    
    let to_address = if to == "binance" {
        "0xf977814e90da44bfa03b6295a0616a897441acec".to_string()
    } else {
        format!("{}2222222222222222222222222222222222222222", to)
    };
    
    ProcessedTransfer {
        block_number,
        transaction_hash: format!("0x{:064x}", block_number),
        log_index: 0,
        from_address,
        to_address,
        amount: amount.to_string(),
        timestamp: 1640995200 + block_number,
        direction,
    }
}

fn format_wei_to_pol(wei_str: &str) -> String {
    let wei = wei_str.parse::<u128>().unwrap_or(0);
    let pol = wei as f64 / 1e18;
    format!("{:.6}", pol)
}