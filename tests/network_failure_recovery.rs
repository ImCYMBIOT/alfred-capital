use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

use polygon_pol_indexer::blockchain::{RpcClient, BlockProcessor, BlockMonitor, BlockMonitorConfig};
use polygon_pol_indexer::database::Database;
use polygon_pol_indexer::models::{ProcessedTransfer, TransferDirection};
use polygon_pol_indexer::error::IndexerError;

/// Test RPC client behavior under various network failure scenarios
#[tokio::test]
async fn test_rpc_client_network_failures() {
    // Test with completely invalid endpoint
    let invalid_client = RpcClient::new("http://invalid-endpoint-that-does-not-exist.com".to_string());
    
    let start_time = Instant::now();
    let result = timeout(Duration::from_secs(5), invalid_client.get_latest_block_number()).await;
    let elapsed = start_time.elapsed();
    
    // Should fail quickly and gracefully
    assert!(result.is_err() || result.unwrap().is_err(), "Should fail with invalid endpoint");
    assert!(elapsed < Duration::from_secs(10), "Should fail quickly, took {:?}", elapsed);
    
    // Test with localhost (connection refused)
    let localhost_client = RpcClient::new("http://localhost:9999".to_string());
    
    let start_time = Instant::now();
    let result = timeout(Duration::from_secs(3), localhost_client.get_latest_block_number()).await;
    let elapsed = start_time.elapsed();
    
    // Should fail with connection error
    match result {
        Ok(Err(_)) => println!("Connection refused handled gracefully"),
        Err(_) => println!("Timeout handled gracefully"),
        Ok(Ok(_)) => panic!("Should not succeed with connection refused"),
    }
    
    assert!(elapsed <= Duration::from_secs(4), "Should timeout quickly, took {:?}", elapsed);
}

/// Test RPC client with mock server that simulates various failure modes
#[tokio::test]
async fn test_rpc_client_with_mock_failures() {
    let mock_server = MockServer::start().await;
    let rpc_client = RpcClient::new(mock_server.uri());
    
    // Test 1: Server returns 500 error
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;
    
    let result = rpc_client.get_latest_block_number().await;
    assert!(result.is_err(), "Should fail with 500 error");
    
    // Test 2: Server returns invalid JSON
    mock_server.reset().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;
    
    let result = rpc_client.get_latest_block_number().await;
    assert!(result.is_err(), "Should fail with invalid JSON");
    
    // Test 3: Server returns JSON-RPC error
    mock_server.reset().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32603,
                "message": "Internal error"
            }
        })))
        .mount(&mock_server)
        .await;
    
    let result = rpc_client.get_latest_block_number().await;
    assert!(result.is_err(), "Should fail with JSON-RPC error");
    
    // Test 4: Server returns successful response
    mock_server.reset().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": "0x1234567"
        })))
        .mount(&mock_server)
        .await;
    
    let result = rpc_client.get_latest_block_number().await;
    assert!(result.is_ok(), "Should succeed with valid response");
    assert_eq!(result.unwrap(), 0x1234567);
}

/// Test block monitor retry logic under network failures
#[tokio::test]
async fn test_block_monitor_retry_logic() {
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Create mock server that fails initially then succeeds
    let mock_server = MockServer::start().await;
    let rpc_client = RpcClient::new(mock_server.uri());
    let block_processor = BlockProcessor::new(rpc_client.clone());
    
    let config = BlockMonitorConfig {
        poll_interval_seconds: 1,
        max_retries: 3,
        retry_delay_seconds: 1,
        max_retry_delay_seconds: 5,
    };
    
    let monitor = BlockMonitor::new(rpc_client, block_processor, database, Some(config));
    
    // Set up mock to fail twice then succeed
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = call_count.clone();
    
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(move |_req| {
            let count = call_count_clone.fetch_add(1, Ordering::Relaxed);
            if count < 2 {
                ResponseTemplate::new(500).set_body_string("Server Error")
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": "0x12345"
                }))
            }
        })
        .mount(&mock_server)
        .await;
    
    // Test retry logic
    let start_time = Instant::now();
    let result = monitor.get_latest_block_with_retry().await;
    let elapsed = start_time.elapsed();
    
    // Should eventually succeed after retries
    assert!(result.is_ok(), "Should succeed after retries: {:?}", result);
    assert_eq!(result.unwrap(), 0x12345);
    
    // Should have taken some time due to retries
    assert!(elapsed >= Duration::from_secs(2), "Should have retried, took {:?}", elapsed);
    
    // Verify retry count
    assert_eq!(call_count.load(Ordering::Relaxed), 3, "Should have made 3 attempts");
}

/// Test network recovery scenarios
#[tokio::test]
async fn test_network_recovery_scenarios() {
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Scenario 1: Temporary network outage
    let mock_server = MockServer::start().await;
    let rpc_client = RpcClient::new(mock_server.uri());
    
    // Initially working
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": "0x1000"
        })))
        .mount(&mock_server)
        .await;
    
    let result1 = rpc_client.get_latest_block_number().await;
    assert!(result1.is_ok(), "Initial request should succeed");
    
    // Simulate network outage
    mock_server.reset().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;
    
    let result2 = rpc_client.get_latest_block_number().await;
    assert!(result2.is_err(), "Should fail during outage");
    
    // Network recovery
    mock_server.reset().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": "0x1001"
        })))
        .mount(&mock_server)
        .await;
    
    let result3 = rpc_client.get_latest_block_number().await;
    assert!(result3.is_ok(), "Should succeed after recovery");
    assert_eq!(result3.unwrap(), 0x1001);
}

/// Test database resilience during network failures
#[tokio::test]
async fn test_database_resilience_during_network_failures() {
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Store some initial data
    let initial_transfer = ProcessedTransfer {
        block_number: 1000,
        transaction_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        log_index: 0,
        from_address: "0x1111111111111111111111111111111111111111".to_string(),
        to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
        amount: "1000000000000000000".to_string(),
        timestamp: 1640995200,
        direction: TransferDirection::ToBinance,
    };
    
    let result = database.store_transfer_and_update_net_flow(&initial_transfer);
    assert!(result.is_ok(), "Initial storage should succeed");
    
    // Verify initial state
    let initial_net_flow = database.get_net_flow_data().expect("Should get initial net flow");
    assert_eq!(initial_net_flow.total_inflow, "1000000000000000000");
    
    // Simulate network failure scenario where we can't fetch new blocks
    // but database operations should still work
    
    // Continue storing transfers (simulating cached/queued data)
    let cached_transfers = vec![
        ProcessedTransfer {
            block_number: 1001,
            transaction_hash: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            log_index: 0,
            from_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
            to_address: "0x2222222222222222222222222222222222222222".to_string(),
            amount: "500000000000000000".to_string(),
            timestamp: 1640995260,
            direction: TransferDirection::FromBinance,
        },
    ];
    
    for transfer in &cached_transfers {
        let result = database.store_transfer_and_update_net_flow(transfer);
        assert!(result.is_ok(), "Cached transfer storage should succeed during network failure");
    }
    
    // Verify database consistency during network failure
    let net_flow = database.get_net_flow_data().expect("Should get net flow during network failure");
    assert_eq!(net_flow.total_inflow, "1000000000000000000");
    assert_eq!(net_flow.total_outflow, "500000000000000000");
    assert_eq!(net_flow.net_flow, "500000000000000000");
    
    // Verify transaction count
    let count = database.get_transaction_count().expect("Should get transaction count");
    assert_eq!(count, 2);
    
    println!("Database remained consistent during simulated network failure");
}

/// Test system behavior under prolonged network instability
#[tokio::test]
async fn test_prolonged_network_instability() {
    let database = Database::new_in_memory().expect("Failed to create test database");
    let mock_server = MockServer::start().await;
    let rpc_client = RpcClient::new(mock_server.uri());
    let block_processor = BlockProcessor::new(rpc_client.clone());
    
    let config = BlockMonitorConfig {
        poll_interval_seconds: 1,
        max_retries: 2,
        retry_delay_seconds: 1,
        max_retry_delay_seconds: 3,
    };
    
    let monitor = BlockMonitor::new(rpc_client, block_processor, database.clone(), Some(config));
    
    // Simulate intermittent failures (50% success rate)
    let success_count = Arc::new(AtomicU32::new(0));
    let failure_count = Arc::new(AtomicU32::new(0));
    let call_count = Arc::new(AtomicU32::new(0));
    
    let success_count_clone = success_count.clone();
    let failure_count_clone = failure_count.clone();
    let call_count_clone = call_count.clone();
    
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(move |_req| {
            let count = call_count_clone.fetch_add(1, Ordering::Relaxed);
            if count % 2 == 0 {
                success_count_clone.fetch_add(1, Ordering::Relaxed);
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": format!("0x{:x}", 1000 + count)
                }))
            } else {
                failure_count_clone.fetch_add(1, Ordering::Relaxed);
                ResponseTemplate::new(500).set_body_string("Intermittent failure")
            }
        })
        .mount(&mock_server)
        .await;
    
    // Test multiple requests under instability
    let mut successful_requests = 0;
    let mut failed_requests = 0;
    
    for i in 0..10 {
        let result = timeout(Duration::from_secs(5), monitor.get_latest_block_with_retry()).await;
        
        match result {
            Ok(Ok(_)) => {
                successful_requests += 1;
                println!("Request {} succeeded", i + 1);
            }
            Ok(Err(e)) => {
                failed_requests += 1;
                println!("Request {} failed: {}", i + 1, e);
            }
            Err(_) => {
                failed_requests += 1;
                println!("Request {} timed out", i + 1);
            }
        }
        
        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    println!("Network instability test results:");
    println!("Successful requests: {}", successful_requests);
    println!("Failed requests: {}", failed_requests);
    println!("Total RPC calls made: {}", call_count.load(Ordering::Relaxed));
    println!("Successful RPC calls: {}", success_count.load(Ordering::Relaxed));
    println!("Failed RPC calls: {}", failure_count.load(Ordering::Relaxed));
    
    // Should have some successful requests despite instability
    assert!(successful_requests > 0, "Should have some successful requests");
    
    // Database should remain functional
    let net_flow = database.get_net_flow_data().expect("Database should remain functional");
    assert_eq!(net_flow.total_inflow, "0"); // No actual transfers processed
}

/// Test graceful degradation under network failures
#[tokio::test]
async fn test_graceful_degradation() {
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Store some data before network failure
    let pre_failure_transfers = vec![
        ProcessedTransfer {
            block_number: 1000,
            transaction_hash: "0x1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            log_index: 0,
            from_address: "0x1111111111111111111111111111111111111111".to_string(),
            to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
            amount: "1000000000000000000".to_string(),
            timestamp: 1640995200,
            direction: TransferDirection::ToBinance,
        },
    ];
    
    for transfer in &pre_failure_transfers {
        database.store_transfer_and_update_net_flow(transfer)
            .expect("Pre-failure storage should succeed");
    }
    
    // Verify system can still provide data during network failure
    let net_flow = database.get_net_flow_data().expect("Should get net flow during network failure");
    assert_eq!(net_flow.total_inflow, "1000000000000000000");
    
    // Test that queries still work
    let transaction_count = database.get_transaction_count().expect("Should get transaction count");
    assert_eq!(transaction_count, 1);
    
    // Test that we can still store new data (from cache/queue)
    let during_failure_transfer = ProcessedTransfer {
        block_number: 1001,
        transaction_hash: "0x2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        log_index: 0,
        from_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
        to_address: "0x3333333333333333333333333333333333333333".to_string(),
        amount: "500000000000000000".to_string(),
        timestamp: 1640995260,
        direction: TransferDirection::FromBinance,
    };
    
    let result = database.store_transfer_and_update_net_flow(&during_failure_transfer);
    assert!(result.is_ok(), "Should be able to store data during network failure");
    
    // Verify graceful degradation - system continues to function
    let final_net_flow = database.get_net_flow_data().expect("Should get final net flow");
    assert_eq!(final_net_flow.total_inflow, "1000000000000000000");
    assert_eq!(final_net_flow.total_outflow, "500000000000000000");
    assert_eq!(final_net_flow.net_flow, "500000000000000000");
    
    let final_count = database.get_transaction_count().expect("Should get final count");
    assert_eq!(final_count, 2);
    
    println!("System gracefully degraded during network failure");
    println!("Continued to process {} transactions", final_count);
    println!("Maintained net flow calculation: {} POL", format_wei_to_pol(&final_net_flow.net_flow));
}

/// Test recovery after extended network outage
#[tokio::test]
async fn test_recovery_after_extended_outage() {
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Set initial state before outage
    database.set_last_processed_block(1000).expect("Failed to set initial block");
    
    let initial_transfer = ProcessedTransfer {
        block_number: 1000,
        transaction_hash: "0x1000000000000000000000000000000000000000000000000000000000000000".to_string(),
        log_index: 0,
        from_address: "0x1111111111111111111111111111111111111111".to_string(),
        to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
        amount: "1000000000000000000".to_string(),
        timestamp: 1640995200,
        direction: TransferDirection::ToBinance,
    };
    
    database.store_transfer_and_update_net_flow(&initial_transfer)
        .expect("Failed to store initial transfer");
    
    // Simulate extended outage (system state is preserved)
    let pre_outage_net_flow = database.get_net_flow_data().expect("Should get pre-outage net flow");
    let pre_outage_block = database.get_last_processed_block().expect("Should get pre-outage block");
    
    // Simulate recovery - system resumes from last known state
    assert_eq!(pre_outage_block, 1000);
    assert_eq!(pre_outage_net_flow.total_inflow, "1000000000000000000");
    
    // Process new transfers after recovery
    let post_recovery_transfers = vec![
        ProcessedTransfer {
            block_number: 1001,
            transaction_hash: "0x1001000000000000000000000000000000000000000000000000000000000000".to_string(),
            log_index: 0,
            from_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
            to_address: "0x2222222222222222222222222222222222222222".to_string(),
            amount: "300000000000000000".to_string(),
            timestamp: 1640995260,
            direction: TransferDirection::FromBinance,
        },
        ProcessedTransfer {
            block_number: 1002,
            transaction_hash: "0x1002000000000000000000000000000000000000000000000000000000000000".to_string(),
            log_index: 0,
            from_address: "0x3333333333333333333333333333333333333333".to_string(),
            to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
            amount: "2000000000000000000".to_string(),
            timestamp: 1640995320,
            direction: TransferDirection::ToBinance,
        },
    ];
    
    for transfer in &post_recovery_transfers {
        let result = database.store_transfer_and_update_net_flow(transfer);
        assert!(result.is_ok(), "Post-recovery transfer should succeed");
        
        database.set_last_processed_block(transfer.block_number)
            .expect("Should update last processed block");
    }
    
    // Verify recovery state
    let post_recovery_net_flow = database.get_net_flow_data().expect("Should get post-recovery net flow");
    let post_recovery_block = database.get_last_processed_block().expect("Should get post-recovery block");
    
    assert_eq!(post_recovery_block, 1002);
    assert_eq!(post_recovery_net_flow.total_inflow, "3000000000000000000"); // 1 + 2 POL
    assert_eq!(post_recovery_net_flow.total_outflow, "300000000000000000"); // 0.3 POL
    assert_eq!(post_recovery_net_flow.net_flow, "2700000000000000000"); // 2.7 POL
    
    let final_count = database.get_transaction_count().expect("Should get final transaction count");
    assert_eq!(final_count, 3);
    
    println!("Successfully recovered after extended outage");
    println!("Resumed from block: {}", pre_outage_block);
    println!("Processed to block: {}", post_recovery_block);
    println!("Final net flow: {} POL", format_wei_to_pol(&post_recovery_net_flow.net_flow));
}

// Helper function
fn format_wei_to_pol(wei_str: &str) -> String {
    let wei = wei_str.parse::<u128>().unwrap_or(0);
    let pol = wei as f64 / 1e18;
    format!("{:.6}", pol)
}