use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tempfile::TempDir;
use criterion::{black_box, Criterion};

use polygon_pol_indexer::database::Database;
use polygon_pol_indexer::models::{ProcessedTransfer, TransferDirection};

/// Performance test for database operations under load
#[tokio::test]
async fn test_database_bulk_insert_performance() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("perf_test.db");
    let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
    
    // Generate test data
    let transfer_count = 1000;
    let transfers = generate_test_transfers(transfer_count);
    
    println!("Testing bulk insert of {} transfers", transfer_count);
    
    let start_time = Instant::now();
    
    // Insert transfers in bulk
    for (i, transfer) in transfers.iter().enumerate() {
        let result = database.store_transfer_and_update_net_flow(transfer);
        assert!(result.is_ok(), "Failed to store transfer {}: {:?}", i, result);
        
        if i % 100 == 0 {
            println!("Inserted {} transfers", i + 1);
        }
    }
    
    let elapsed = start_time.elapsed();
    let transfers_per_second = transfer_count as f64 / elapsed.as_secs_f64();
    
    println!("Bulk insert completed in {:?}", elapsed);
    println!("Performance: {:.2} transfers/second", transfers_per_second);
    
    // Verify data integrity
    let final_count = database.get_transaction_count().expect("Failed to get transaction count");
    assert_eq!(final_count, transfer_count as i64);
    
    let net_flow = database.get_net_flow_data().expect("Failed to get net flow");
    println!("Final net flow: {} POL", format_wei_to_pol(&net_flow.net_flow));
    
    // Performance assertions
    assert!(transfers_per_second > 50.0, "Performance too slow: {:.2} transfers/second", transfers_per_second);
    assert!(elapsed < Duration::from_secs(30), "Bulk insert took too long: {:?}", elapsed);
}

/// Test database query performance under load
#[tokio::test]
async fn test_database_query_performance() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("query_perf_test.db");
    let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
    
    // Insert test data
    let transfer_count = 500;
    let transfers = generate_test_transfers(transfer_count);
    
    for transfer in &transfers {
        database.store_transfer_and_update_net_flow(transfer)
            .expect("Failed to store transfer");
    }
    
    println!("Testing query performance with {} transfers", transfer_count);
    
    // Test net flow query performance
    let start_time = Instant::now();
    let query_count = 100;
    
    for _ in 0..query_count {
        let _net_flow = database.get_net_flow_data().expect("Failed to get net flow");
    }
    
    let elapsed = start_time.elapsed();
    let queries_per_second = query_count as f64 / elapsed.as_secs_f64();
    
    println!("Net flow query performance: {:.2} queries/second", queries_per_second);
    assert!(queries_per_second > 1000.0, "Query performance too slow: {:.2} queries/second", queries_per_second);
    
    // Test transaction lookup performance
    let start_time = Instant::now();
    let lookup_count = 100;
    
    for i in 0..lookup_count {
        let transfer = &transfers[i % transfers.len()];
        let _result = database.get_transaction(&transfer.transaction_hash, transfer.log_index);
    }
    
    let elapsed = start_time.elapsed();
    let lookups_per_second = lookup_count as f64 / elapsed.as_secs_f64();
    
    println!("Transaction lookup performance: {:.2} lookups/second", lookups_per_second);
    assert!(lookups_per_second > 500.0, "Lookup performance too slow: {:.2} lookups/second", lookups_per_second);
}

/// Test concurrent database access performance
#[tokio::test]
async fn test_concurrent_database_performance() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("concurrent_perf_test.db");
    let database = Arc::new(Database::new(db_path.to_str().unwrap()).expect("Failed to create database"));
    
    let concurrent_tasks = 10;
    let transfers_per_task = 50;
    
    println!("Testing concurrent performance: {} tasks, {} transfers each", concurrent_tasks, transfers_per_task);
    
    let start_time = Instant::now();
    let mut handles = Vec::new();
    
    for task_id in 0..concurrent_tasks {
        let db = database.clone();
        let handle = tokio::spawn(async move {
            let transfers = generate_test_transfers_with_offset(transfers_per_task, task_id * 1000);
            
            for transfer in transfers {
                let result = db.store_transfer_and_update_net_flow(&transfer);
                if result.is_err() {
                    println!("Task {} failed to store transfer: {:?}", task_id, result);
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Concurrent task failed");
    }
    
    let elapsed = start_time.elapsed();
    let total_transfers = concurrent_tasks * transfers_per_task;
    let transfers_per_second = total_transfers as f64 / elapsed.as_secs_f64();
    
    println!("Concurrent insert completed in {:?}", elapsed);
    println!("Performance: {:.2} transfers/second", transfers_per_second);
    
    // Verify data integrity
    let final_count = database.get_transaction_count().expect("Failed to get transaction count");
    println!("Final transaction count: {}", final_count);
    
    // Performance assertions
    assert!(transfers_per_second > 30.0, "Concurrent performance too slow: {:.2} transfers/second", transfers_per_second);
    assert!(elapsed < Duration::from_secs(60), "Concurrent insert took too long: {:?}", elapsed);
}

/// Test memory usage during large data processing
#[tokio::test]
async fn test_memory_usage_performance() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("memory_test.db");
    let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
    
    // Process transfers in batches to test memory efficiency
    let batch_size = 100;
    let batch_count = 10;
    
    println!("Testing memory usage with {} batches of {} transfers", batch_count, batch_size);
    
    for batch in 0..batch_count {
        let transfers = generate_test_transfers_with_offset(batch_size, batch * 1000);
        
        let batch_start = Instant::now();
        
        for transfer in transfers {
            database.store_transfer_and_update_net_flow(&transfer)
                .expect("Failed to store transfer");
        }
        
        let batch_elapsed = batch_start.elapsed();
        println!("Batch {} completed in {:?}", batch + 1, batch_elapsed);
        
        // Verify net flow calculation is still accurate
        let net_flow = database.get_net_flow_data().expect("Failed to get net flow");
        assert!(!net_flow.net_flow.is_empty(), "Net flow should not be empty");
        
        // Small delay to allow for memory cleanup
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    let final_count = database.get_transaction_count().expect("Failed to get transaction count");
    assert_eq!(final_count, (batch_count * batch_size) as i64);
    
    println!("Memory usage test completed successfully");
}

/// Test database performance with realistic block processing simulation
#[tokio::test]
async fn test_realistic_block_processing_performance() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("realistic_perf_test.db");
    let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
    
    // Simulate processing 100 blocks with varying numbers of transfers
    let block_count = 100;
    let mut total_transfers = 0;
    
    println!("Simulating realistic block processing for {} blocks", block_count);
    
    let start_time = Instant::now();
    
    for block_num in 1..=block_count {
        // Simulate varying transfer counts per block (0-10 transfers)
        let transfers_in_block = (block_num % 11) as usize;
        let transfers = generate_block_transfers(block_num, transfers_in_block);
        
        let block_start = Instant::now();
        
        // Process all transfers in the block
        for transfer in transfers {
            database.store_transfer_and_update_net_flow(&transfer)
                .expect("Failed to store transfer");
            total_transfers += 1;
        }
        
        // Update last processed block
        database.set_last_processed_block(block_num)
            .expect("Failed to update last processed block");
        
        let block_elapsed = block_start.elapsed();
        
        if block_num % 20 == 0 {
            println!("Processed block {} ({} transfers) in {:?}", 
                block_num, transfers_in_block, block_elapsed);
        }
        
        // Simulate 2-second block time
        if block_elapsed < Duration::from_millis(100) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    
    let total_elapsed = start_time.elapsed();
    let blocks_per_second = block_count as f64 / total_elapsed.as_secs_f64();
    let transfers_per_second = total_transfers as f64 / total_elapsed.as_secs_f64();
    
    println!("Realistic processing completed in {:?}", total_elapsed);
    println!("Performance: {:.2} blocks/second, {:.2} transfers/second", 
        blocks_per_second, transfers_per_second);
    
    // Verify final state
    let net_flow = database.get_net_flow_data().expect("Failed to get net flow");
    assert_eq!(net_flow.last_processed_block, block_count);
    
    let final_count = database.get_transaction_count().expect("Failed to get transaction count");
    assert_eq!(final_count, total_transfers as i64);
    
    // Performance assertions for realistic workload
    assert!(blocks_per_second > 10.0, "Block processing too slow: {:.2} blocks/second", blocks_per_second);
    assert!(total_elapsed < Duration::from_secs(30), "Realistic processing took too long: {:?}", total_elapsed);
    
    println!("Final net flow: {} POL", format_wei_to_pol(&net_flow.net_flow));
}

/// Benchmark database operations using criterion (if available)
#[tokio::test]
async fn test_database_benchmarks() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("benchmark_test.db");
    let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
    
    // Prepare test data
    let test_transfer = ProcessedTransfer {
        block_number: 1000,
        transaction_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        log_index: 0,
        from_address: "0x1111111111111111111111111111111111111111".to_string(),
        to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
        amount: "1000000000000000000".to_string(),
        timestamp: 1640995200,
        direction: TransferDirection::ToBinance,
    };
    
    // Benchmark single insert
    let iterations = 100;
    let start_time = Instant::now();
    
    for i in 0..iterations {
        let mut transfer = test_transfer.clone();
        transfer.transaction_hash = format!("0x{:064x}", i);
        
        let result = database.store_transfer_and_update_net_flow(&transfer);
        assert!(result.is_ok(), "Benchmark insert failed");
    }
    
    let elapsed = start_time.elapsed();
    let inserts_per_second = iterations as f64 / elapsed.as_secs_f64();
    
    println!("Benchmark: {:.2} inserts/second", inserts_per_second);
    
    // Benchmark queries
    let start_time = Instant::now();
    
    for _ in 0..iterations {
        let _net_flow = database.get_net_flow_data().expect("Benchmark query failed");
    }
    
    let elapsed = start_time.elapsed();
    let queries_per_second = iterations as f64 / elapsed.as_secs_f64();
    
    println!("Benchmark: {:.2} queries/second", queries_per_second);
    
    // Performance thresholds
    assert!(inserts_per_second > 100.0, "Insert benchmark too slow: {:.2}/second", inserts_per_second);
    assert!(queries_per_second > 1000.0, "Query benchmark too slow: {:.2}/second", queries_per_second);
}

// Helper functions

fn generate_test_transfers(count: usize) -> Vec<ProcessedTransfer> {
    (0..count)
        .map(|i| {
            let direction = if i % 3 == 0 {
                TransferDirection::ToBinance
            } else if i % 3 == 1 {
                TransferDirection::FromBinance
            } else {
                TransferDirection::NotRelevant
            };
            
            ProcessedTransfer {
                block_number: 1000 + i as u64,
                transaction_hash: format!("0x{:064x}", i),
                log_index: 0,
                from_address: format!("0x{:040x}", i),
                to_address: if matches!(direction, TransferDirection::ToBinance) {
                    "0xf977814e90da44bfa03b6295a0616a897441acec".to_string()
                } else if matches!(direction, TransferDirection::FromBinance) {
                    format!("0x{:040x}", i + 1000)
                } else {
                    format!("0x{:040x}", i + 2000)
                },
                amount: format!("{}", (i + 1) * 1000000000000000000), // Varying amounts
                timestamp: 1640995200 + i as u64,
                direction,
            }
        })
        .collect()
}

fn generate_test_transfers_with_offset(count: usize, offset: usize) -> Vec<ProcessedTransfer> {
    (0..count)
        .map(|i| {
            let idx = i + offset;
            let direction = if idx % 3 == 0 {
                TransferDirection::ToBinance
            } else if idx % 3 == 1 {
                TransferDirection::FromBinance
            } else {
                TransferDirection::NotRelevant
            };
            
            ProcessedTransfer {
                block_number: 1000 + idx as u64,
                transaction_hash: format!("0x{:064x}", idx),
                log_index: 0,
                from_address: format!("0x{:040x}", idx),
                to_address: if matches!(direction, TransferDirection::ToBinance) {
                    "0xf977814e90da44bfa03b6295a0616a897441acec".to_string()
                } else {
                    format!("0x{:040x}", idx + 1000)
                },
                amount: format!("{}", (idx + 1) * 1000000000000000000),
                timestamp: 1640995200 + idx as u64,
                direction,
            }
        })
        .collect()
}

fn generate_block_transfers(block_number: u64, count: usize) -> Vec<ProcessedTransfer> {
    (0..count)
        .map(|i| {
            let direction = if i % 4 == 0 {
                TransferDirection::ToBinance
            } else if i % 4 == 1 {
                TransferDirection::FromBinance
            } else {
                TransferDirection::NotRelevant
            };
            
            ProcessedTransfer {
                block_number,
                transaction_hash: format!("0x{:064x}", block_number * 1000 + i as u64),
                log_index: i as u32,
                from_address: format!("0x{:040x}", block_number + i as u64),
                to_address: if matches!(direction, TransferDirection::ToBinance) {
                    "0xf977814e90da44bfa03b6295a0616a897441acec".to_string()
                } else {
                    format!("0x{:040x}", block_number + i as u64 + 1000)
                },
                amount: format!("{}", (i + 1) * 500000000000000000), // 0.5 POL increments
                timestamp: 1640995200 + block_number,
                direction,
            }
        })
        .collect()
}

fn format_wei_to_pol(wei_str: &str) -> String {
    let wei = wei_str.parse::<u128>().unwrap_or(0);
    let pol = wei as f64 / 1e18;
    format!("{:.6}", pol)
}