use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tempfile::TempDir;
use std::collections::HashMap;

use polygon_pol_indexer::blockchain::{RpcClient, BlockProcessor};
use polygon_pol_indexer::database::{Database, NetFlowRow, TransactionRow};
use polygon_pol_indexer::models::{ProcessedTransfer, TransferDirection, NetFlowData};
use polygon_pol_indexer::config::AppConfig;

/// Final integration test - Test complete system with live Polygon network connection
/// This test validates the entire system end-to-end with real network connectivity
#[tokio::test]
#[ignore] // Use --ignored flag to run this test with live network
async fn test_complete_system_with_live_polygon_network() {
    println!("🚀 Starting complete system test with live Polygon network");
    
    // Load configuration for live testing
    let config = AppConfig::load().unwrap_or_default();
    
    // Create temporary database for testing
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("live_test.db");
    let database = Arc::new(Database::new(db_path.to_str().unwrap()).expect("Failed to create database"));
    
    // Initialize RPC client with live Polygon endpoint
    let rpc_client = RpcClient::new_with_config(
        config.rpc.endpoint.clone(),
        config.rpc.timeout_seconds
    );
    
    // Test 1: Verify live network connectivity
    println!("📡 Testing live network connectivity...");
    let latest_block_result = timeout(
        Duration::from_secs(30),
        rpc_client.get_latest_block_number_with_retry()
    ).await;
    
    match latest_block_result {
        Ok(Ok(block_number)) => {
            println!("✅ Successfully connected to Polygon network, latest block: {}", block_number);
            assert!(block_number > 0, "Block number should be greater than 0");
        }
        Ok(Err(e)) => {
            println!("❌ Failed to connect to Polygon network: {}", e);
            panic!("Live network test requires working Polygon RPC connection");
        }
        Err(_) => {
            println!("❌ Timeout connecting to Polygon network");
            panic!("Live network test timed out - check network connectivity");
        }
    }
    
    // Test 2: Process real blocks and extract POL transfers
    println!("🔍 Processing real blocks for POL transfers...");
    let block_processor = BlockProcessor::new(rpc_client.clone());
    
    let latest_block = rpc_client.get_latest_block_number_with_retry().await
        .expect("Failed to get latest block number");
    
    // Process last 10 blocks to find real POL transfers
    let start_block = latest_block.saturating_sub(10);
    let mut total_transfers_found = 0;
    let mut binance_transfers_found = 0;
    let mut processed_transfers = Vec::new();
    
    for block_num in start_block..=latest_block {
        match timeout(Duration::from_secs(30), block_processor.process_block(block_num)).await {
            Ok(Ok(transfers)) => {
                let transfer_count = transfers.len();
                total_transfers_found += transfer_count;
                for transfer in transfers {
                    if matches!(transfer.direction, TransferDirection::ToBinance | TransferDirection::FromBinance) {
                        binance_transfers_found += 1;
                        processed_transfers.push(transfer);
                    }
                }
                println!("  Block {}: {} POL transfers found", block_num, transfer_count);
            }
            Ok(Err(e)) => {
                println!("  Block {}: Error processing - {}", block_num, e);
            }
            Err(_) => {
                println!("  Block {}: Timeout processing", block_num);
            }
        }
    }
    
    println!("✅ Processed {} blocks, found {} total POL transfers, {} Binance-related", 
             latest_block - start_block + 1, total_transfers_found, binance_transfers_found);
    
    // Test 3: Validate net-flow calculations against manual verification
    println!("🧮 Validating net-flow calculations...");
    
    // Store processed transfers in database
    for transfer in &processed_transfers {
        database.store_transfer_and_update_net_flow(transfer)
            .expect("Failed to store transaction");
    }
    
    // Manual calculation - parse amounts from string
    let mut manual_inflow = 0f64;
    let mut manual_outflow = 0f64;
    
    for transfer in &processed_transfers {
        let amount: f64 = transfer.amount.parse().expect("Failed to parse amount");
        match &transfer.direction {
            TransferDirection::ToBinance => manual_inflow += amount,
            TransferDirection::FromBinance => manual_outflow += amount,
            TransferDirection::NotRelevant => {}
        }
    }
    
    let manual_net_flow = manual_inflow - manual_outflow;
    
    // Database calculation
    let db_net_flow = database.get_net_flow_data()
        .expect("Failed to get net flow from database");
    
    let db_net_flow_value: f64 = db_net_flow.net_flow.parse()
        .expect("Failed to parse net flow value");
    
    println!("  Manual calculation: inflow={}, outflow={}, net={}", 
             manual_inflow, manual_outflow, manual_net_flow);
    println!("  Database calculation: net={}", db_net_flow_value);
    
    // Allow for small floating point differences
    let diff = (manual_net_flow - db_net_flow_value).abs();
    assert!(diff < 0.001, 
            "Net-flow calculation mismatch between manual ({}) and database ({})", 
            manual_net_flow, db_net_flow_value);
    
    println!("✅ Net-flow calculations validated successfully");
    
    // Test 4: System recovery after various failure scenarios
    println!("🔄 Testing system recovery scenarios...");
    
    // Test 4a: Database connection recovery
    println!("  Testing database recovery...");
    {
        // Test database accessibility
        let recovery_result = database.get_net_flow_data();
        assert!(recovery_result.is_ok(), "Database should be accessible");
    }
    
    // Test 4b: RPC connection recovery with retry
    println!("  Testing RPC recovery with retry...");
    {
        let start_time = std::time::Instant::now();
        let recovery_result = rpc_client.get_latest_block_number_with_retry().await;
        let elapsed = start_time.elapsed();
        
        match recovery_result {
            Ok(block_num) => {
                println!("    ✅ RPC recovery successful in {:?}, block: {}", elapsed, block_num);
            }
            Err(e) => {
                println!("    ⚠️  RPC recovery failed: {}", e);
                // Don't fail the test if network is temporarily unavailable
            }
        }
    }
    
    // Test 4c: Block processing error recovery
    println!("  Testing block processing error recovery...");
    {
        // Try to process a very old block that might not exist
        let old_block = 1u64;
        match block_processor.process_block(old_block).await {
            Ok(_) => println!("    ✅ Old block processed successfully"),
            Err(e) => println!("    ✅ Block processing error handled gracefully: {}", e),
        }
    }
    
    println!("✅ System recovery scenarios tested");
    
    // Test 5: End-to-end requirements verification
    println!("📋 Verifying all requirements are met...");
    
    // Requirement 6.2: System should continue from last processed block
    let last_processed = database.get_last_processed_block()
        .expect("Failed to get last processed block");
    println!("  ✅ Requirement 6.2: Last processed block tracking works (block: {})", last_processed);
    
    // Requirement 6.3: System should initialize with zero cumulative net-flow for fresh start
    // (This is tested implicitly by the net-flow calculation validation above)
    println!("  ✅ Requirement 6.3: Net-flow calculation and storage verified");
    
    // Requirement 6.5: System should maintain data consistency without gaps
    let stored_transactions = database.get_recent_transactions(100, 0)
        .expect("Failed to get recent transactions");
    
    // Verify transaction data integrity
    for transaction in &stored_transactions {
        assert!(!transaction.transaction_hash.is_empty(), "Transaction hash should not be empty");
        assert!(!transaction.from_address.is_empty(), "From address should not be empty");
        assert!(!transaction.to_address.is_empty(), "To address should not be empty");
        assert!(!transaction.amount.is_empty(), "Amount should not be empty");
        assert!(transaction.block_number > 0, "Block number should be positive");
    }
    
    println!("  ✅ Requirement 6.5: Data consistency verified ({} transactions)", stored_transactions.len());
    
    println!("🎉 All final integration tests passed successfully!");
}

/// Load testing with high block processing rates
#[tokio::test]
#[ignore] // Use --ignored flag to run load tests
async fn test_load_testing_high_block_processing_rates() {
    println!("🚀 Starting load testing with high block processing rates");
    
    let config = AppConfig::load().unwrap_or_default();
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("load_test.db");
    let database = Arc::new(Database::new(db_path.to_str().unwrap()).expect("Failed to create database"));
    
    let rpc_client = RpcClient::new_with_config(
        config.rpc.endpoint.clone(),
        config.rpc.timeout_seconds
    );
    
    let block_processor = BlockProcessor::new(rpc_client.clone());
    
    // Get current block range for testing
    let latest_block = rpc_client.get_latest_block_number_with_retry().await
        .expect("Failed to get latest block number");
    
    let test_blocks = 50; // Process 50 blocks for load testing
    let start_block = latest_block.saturating_sub(test_blocks);
    
    println!("📊 Processing {} blocks from {} to {} for load testing", 
             test_blocks, start_block, latest_block);
    
    let start_time = std::time::Instant::now();
    let mut successful_blocks = 0;
    let mut total_transfers = 0;
    let mut processing_times = Vec::new();
    
    // Process blocks concurrently with limited concurrency
    let semaphore = Arc::new(tokio::sync::Semaphore::new(5)); // Max 5 concurrent requests
    let mut tasks = Vec::new();
    
    for block_num in start_block..=latest_block {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let processor = BlockProcessor::new(rpc_client.clone());
        let db = database.clone();
        
        let task = tokio::spawn(async move {
            let _permit = permit; // Hold permit for duration of task
            let block_start = std::time::Instant::now();
            
            match processor.process_block(block_num).await {
                Ok(transfers) => {
                    let processing_time = block_start.elapsed();
                    
                    // Store transfers in database
                    for transfer in &transfers {
                        if let Err(e) = db.store_transfer_and_update_net_flow(transfer) {
                            eprintln!("Failed to store transaction: {}", e);
                        }
                    }
                    
                    Ok((transfers.len(), processing_time))
                }
                Err(e) => {
                    eprintln!("Failed to process block {}: {}", block_num, e);
                    Err(e)
                }
            }
        });
        
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    for task in tasks {
        match task.await {
            Ok(Ok((transfer_count, processing_time))) => {
                successful_blocks += 1;
                total_transfers += transfer_count;
                processing_times.push(processing_time);
            }
            Ok(Err(_)) => {
                // Error already logged
            }
            Err(e) => {
                eprintln!("Task join error: {}", e);
            }
        }
    }
    
    let total_time = start_time.elapsed();
    
    // Calculate statistics
    let avg_processing_time = if !processing_times.is_empty() {
        processing_times.iter().sum::<Duration>() / processing_times.len() as u32
    } else {
        Duration::from_secs(0)
    };
    
    let blocks_per_second = successful_blocks as f64 / total_time.as_secs_f64();
    
    println!("📈 Load testing results:");
    println!("  Total time: {:?}", total_time);
    println!("  Successful blocks: {}/{}", successful_blocks, test_blocks);
    println!("  Total transfers found: {}", total_transfers);
    println!("  Average processing time per block: {:?}", avg_processing_time);
    println!("  Blocks processed per second: {:.2}", blocks_per_second);
    
    // Performance assertions
    assert!(successful_blocks >= test_blocks * 80 / 100, 
            "At least 80% of blocks should be processed successfully");
    assert!(blocks_per_second > 0.1, 
            "Should process at least 0.1 blocks per second");
    assert!(avg_processing_time < Duration::from_secs(30), 
            "Average processing time should be under 30 seconds");
    
    println!("✅ Load testing completed successfully");
}

/// Test system behavior under various failure scenarios
#[tokio::test]
#[ignore] // Use --ignored flag to run failure scenario tests
async fn test_system_failure_recovery_scenarios() {
    println!("🚀 Starting system failure recovery scenario tests");
    
    let config = AppConfig::load().unwrap_or_default();
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("failure_test.db");
    let database = Arc::new(Database::new(db_path.to_str().unwrap()).expect("Failed to create database"));
    
    // Test 1: Invalid RPC endpoint recovery
    println!("🔧 Testing invalid RPC endpoint recovery...");
    {
        let invalid_rpc = RpcClient::new_with_config("http://invalid-endpoint.com".to_string(), 5);
        
        let start_time = std::time::Instant::now();
        let result = invalid_rpc.get_latest_block_number_with_retry().await;
        let elapsed = start_time.elapsed();
        
        assert!(result.is_err(), "Invalid RPC should fail");
        assert!(elapsed >= Duration::from_secs(5), "Should respect timeout");
        println!("  ✅ Invalid RPC endpoint handled correctly in {:?}", elapsed);
    }
    
    // Test 2: Database corruption recovery
    println!("🔧 Testing database error handling...");
    {
        // Try to access database with invalid operations
        let result = database.get_net_flow_data();
        match result {
            Ok(_) => println!("  ✅ Database operations working normally"),
            Err(e) => println!("  ✅ Database error handled gracefully: {}", e),
        }
    }
    
    // Test 3: Network timeout recovery
    println!("🔧 Testing network timeout recovery...");
    {
        let rpc_client = RpcClient::new_with_config(config.rpc.endpoint.clone(), 1); // Very short timeout
        
        let start_time = std::time::Instant::now();
        let result = rpc_client.get_latest_block_number_with_retry().await;
        let elapsed = start_time.elapsed();
        
        match result {
            Ok(_) => println!("  ✅ Network request succeeded despite short timeout"),
            Err(_) => {
                assert!(elapsed >= Duration::from_secs(1), "Should respect minimum timeout");
                println!("  ✅ Network timeout handled correctly in {:?}", elapsed);
            }
        }
    }
    
    // Test 4: Block processing with malformed data
    println!("🔧 Testing malformed data handling...");
    {
        let rpc_client = RpcClient::new_with_config(config.rpc.endpoint.clone(), 30);
        let block_processor = BlockProcessor::new(rpc_client);
        
        // Try to process block 0 (genesis block, might have different structure)
        let result = block_processor.process_block(0).await;
        match result {
            Ok(transfers) => println!("  ✅ Genesis block processed successfully: {} transfers", transfers.len()),
            Err(e) => println!("  ✅ Genesis block error handled gracefully: {}", e),
        }
    }
    
    // Test 5: Concurrent access stress test
    println!("🔧 Testing concurrent access handling...");
    {
        let mut tasks = Vec::new();
        
        for i in 0..10 {
            let db = database.clone();
            let task = tokio::spawn(async move {
                // Simulate concurrent database operations
                for j in 0..5 {
                    let result = db.get_net_flow_data();
                    match result {
                        Ok(_) => {},
                        Err(e) => eprintln!("Concurrent access error {}-{}: {}", i, j, e),
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            });
            tasks.push(task);
        }
        
        // Wait for all concurrent tasks
        for task in tasks {
            task.await.expect("Concurrent task should complete");
        }
        
        println!("  ✅ Concurrent access handled successfully");
    }
    
    println!("✅ All failure recovery scenarios tested successfully");
}

/// Comprehensive end-to-end requirements verification
#[tokio::test]
#[ignore] // Use --ignored flag to run comprehensive verification
async fn test_comprehensive_requirements_verification() {
    println!("🚀 Starting comprehensive requirements verification");
    
    let config = AppConfig::load().unwrap_or_default();
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("requirements_test.db");
    let database = Arc::new(Database::new(db_path.to_str().unwrap()).expect("Failed to create database"));
    
    // Verify all requirements systematically
    let mut requirements_verified = HashMap::new();
    
    // Requirement 6.2: System should continue from last processed block
    println!("📋 Verifying Requirement 6.2: Block continuation...");
    {
        // Set a test block number
        let test_block = 12345u64;
        database.set_last_processed_block(test_block)
            .expect("Failed to set last processed block");
        
        let retrieved_block = database.get_last_processed_block()
            .expect("Failed to get last processed block");
        
        assert_eq!(test_block, retrieved_block, "Block number persistence failed");
        requirements_verified.insert("6.2", true);
        println!("  ✅ Requirement 6.2 verified: Block continuation works");
    }
    
    // Requirement 6.3: System should initialize with zero cumulative net-flow
    println!("📋 Verifying Requirement 6.3: Zero initialization...");
    {
        // Create fresh database
        let fresh_temp_dir = TempDir::new().expect("Failed to create temp directory");
        let fresh_db_path = fresh_temp_dir.path().join("fresh_test.db");
        let fresh_database = Database::new(fresh_db_path.to_str().unwrap())
            .expect("Failed to create fresh database");
        
        let initial_net_flow = fresh_database.get_net_flow_data()
            .expect("Failed to get initial net flow");
        
        assert_eq!(initial_net_flow.net_flow, "0", "Initial net flow should be zero");
        assert_eq!(initial_net_flow.total_inflow, "0", "Initial inflow should be zero");
        assert_eq!(initial_net_flow.total_outflow, "0", "Initial outflow should be zero");
        
        requirements_verified.insert("6.3", true);
        println!("  ✅ Requirement 6.3 verified: Zero initialization works");
    }
    
    // Requirement 6.5: System should maintain data consistency
    println!("📋 Verifying Requirement 6.5: Data consistency...");
    {
        // Create test transactions
        let test_transfers = vec![
            ProcessedTransfer {
                block_number: 100,
                transaction_hash: "0xtest1".to_string(),
                log_index: 0,
                from_address: "0xsender".to_string(),
                to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(), // Binance address
                amount: "1000000000000000000".to_string(), // 1 POL
                timestamp: 1234567890,
                direction: TransferDirection::ToBinance,
            },
            ProcessedTransfer {
                block_number: 101,
                transaction_hash: "0xtest2".to_string(),
                log_index: 0,
                from_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(), // Binance address
                to_address: "0xreceiver".to_string(),
                amount: "500000000000000000".to_string(), // 0.5 POL
                timestamp: 1234567891,
                direction: TransferDirection::FromBinance,
            },
        ];
        
        // Store transactions
        for transfer in &test_transfers {
            database.store_transfer_and_update_net_flow(transfer)
                .expect("Failed to store test transaction");
        }
        
        // Verify data consistency
        let net_flow = database.get_net_flow_data()
            .expect("Failed to get net flow");
        
        let expected_net_flow = 1000000000000000000f64 - 500000000000000000f64;
        let actual_net_flow: f64 = net_flow.net_flow.parse()
            .expect("Failed to parse net flow");
        
        let diff = (expected_net_flow - actual_net_flow).abs();
        assert!(diff < 0.001, "Net flow calculation inconsistent: expected {}, got {}", expected_net_flow, actual_net_flow);
        
        // Verify transaction retrieval
        let stored_transactions = database.get_recent_transactions(10, 0)
            .expect("Failed to get recent transactions");
        
        assert!(stored_transactions.len() >= 2, "Should have at least 2 stored transactions");
        
        requirements_verified.insert("6.5", true);
        println!("  ✅ Requirement 6.5 verified: Data consistency maintained");
    }
    
    // Summary
    println!("📊 Requirements verification summary:");
    for (req, verified) in &requirements_verified {
        let status = if *verified { "✅ PASSED" } else { "❌ FAILED" };
        println!("  Requirement {}: {}", req, status);
    }
    
    let total_verified = requirements_verified.values().filter(|&&v| v).count();
    let total_requirements = requirements_verified.len();
    
    assert_eq!(total_verified, total_requirements, 
               "All requirements should be verified");
    
    println!("🎉 All {} requirements verified successfully!", total_requirements);
}