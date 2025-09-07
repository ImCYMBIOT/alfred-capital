use std::collections::HashMap;
use polygon_pol_indexer::blockchain::{RpcClient, BlockProcessor, TransferDetector};
use polygon_pol_indexer::database::Database;
use polygon_pol_indexer::models::{ProcessedTransfer, TransferDirection, RawLog, NetFlowData};
use polygon_pol_indexer::blockchain::transfer_detector::{POL_TOKEN_ADDRESS, TRANSFER_EVENT_SIGNATURE, BINANCE_ADDRESSES};

/// Test validation against known POL transfer transaction patterns
#[tokio::test]
async fn test_known_pol_transfer_patterns() {
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Known POL transfer patterns based on real-world data
    let known_transfers = create_known_transfer_patterns();
    
    for (description, transfer) in &known_transfers {
        println!("Validating: {}", description);
        
        let result = database.store_transfer_and_update_net_flow(transfer);
        assert!(result.is_ok(), "Failed to store known transfer '{}': {:?}", description, result);
        
        // Validate stored data
        let stored = database.get_transaction(&transfer.transaction_hash, transfer.log_index);
        assert!(stored.is_ok(), "Failed to retrieve stored transfer '{}'", description);
        
        let stored_transfer = stored.unwrap();
        assert_eq!(stored_transfer.amount, transfer.amount, "Amount mismatch for '{}'", description);
        assert_eq!(stored_transfer.from_address, transfer.from_address, "From address mismatch for '{}'", description);
        assert_eq!(stored_transfer.to_address, transfer.to_address, "To address mismatch for '{}'", description);
        
        // Validate direction classification
        let expected_direction = match transfer.direction {
            TransferDirection::ToBinance => "inflow",
            TransferDirection::FromBinance => "outflow",
            TransferDirection::NotRelevant => panic!("Should not store NotRelevant transfers"),
        };
        assert_eq!(stored_transfer.direction, expected_direction, "Direction mismatch for '{}'", description);
    }
    
    // Validate final net flow calculation
    let net_flow = database.get_net_flow_data().expect("Failed to get net flow");
    
    // Calculate expected values
    let expected_inflow = "5500000000000000000"; // 5.5 POL
    let expected_outflow = "2300000000000000000"; // 2.3 POL  
    let expected_net_flow = "3200000000000000000"; // 3.2 POL
    
    assert_eq!(net_flow.total_inflow, expected_inflow, "Total inflow mismatch");
    assert_eq!(net_flow.total_outflow, expected_outflow, "Total outflow mismatch");
    assert_eq!(net_flow.net_flow, expected_net_flow, "Net flow mismatch");
    
    println!("All known transfer patterns validated successfully");
    println!("Final net flow: {} POL", format_wei_to_pol(&net_flow.net_flow));
}

/// Test validation of POL token transfer event parsing
#[test]
fn test_pol_transfer_event_parsing() {
    let transfer_detector = TransferDetector::new();
    
    // Create known POL transfer logs
    let test_logs = create_known_pol_transfer_logs();
    
    for (description, log, expected_amount, expected_from, expected_to) in test_logs {
        println!("Testing log parsing: {}", description);
        
        // Verify it's identified as a POL transfer
        assert!(transfer_detector.is_pol_transfer(&log), 
            "Should identify as POL transfer: {}", description);
        
        // Parse the transfer
        let result = transfer_detector.decode_transfer_log(&log);
        assert!(result.is_ok(), "Failed to decode transfer '{}': {:?}", description, result);
        
        let transfer = result.unwrap();
        
        // Validate parsed data
        assert_eq!(transfer.amount, expected_amount, "Amount mismatch for '{}'", description);
        assert_eq!(transfer.from_address.to_lowercase(), expected_from.to_lowercase(), 
            "From address mismatch for '{}'", description);
        assert_eq!(transfer.to_address.to_lowercase(), expected_to.to_lowercase(), 
            "To address mismatch for '{}'", description);
        assert_eq!(transfer.block_number, log.block_number, "Block number mismatch for '{}'", description);
        assert_eq!(transfer.transaction_hash, log.transaction_hash, "Transaction hash mismatch for '{}'", description);
    }
    
    println!("All POL transfer event parsing tests passed");
}

/// Test validation of Binance address classification
#[test]
fn test_binance_address_classification() {
    let transfer_detector = TransferDetector::new();
    
    // Test all known Binance addresses
    for binance_addr in BINANCE_ADDRESSES {
        println!("Testing Binance address: {}", binance_addr);
        
        // Test inflow (to Binance)
        let inflow_transfer = ProcessedTransfer {
            block_number: 1000,
            transaction_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            log_index: 0,
            from_address: "0x1111111111111111111111111111111111111111".to_string(),
            to_address: binance_addr.to_string(),
            amount: "1000000000000000000".to_string(),
            timestamp: 1640995200,
            direction: TransferDirection::NotRelevant, // Will be classified
        };
        
        let classified_inflow = transfer_detector.classify_transfer_direction(&inflow_transfer);
        assert_eq!(classified_inflow, TransferDirection::ToBinance, 
            "Should classify as inflow for address {}", binance_addr);
        
        // Test outflow (from Binance)
        let outflow_transfer = ProcessedTransfer {
            block_number: 1001,
            transaction_hash: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            log_index: 0,
            from_address: binance_addr.to_string(),
            to_address: "0x2222222222222222222222222222222222222222".to_string(),
            amount: "500000000000000000".to_string(),
            timestamp: 1640995260,
            direction: TransferDirection::NotRelevant, // Will be classified
        };
        
        let classified_outflow = transfer_detector.classify_transfer_direction(&outflow_transfer);
        assert_eq!(classified_outflow, TransferDirection::FromBinance, 
            "Should classify as outflow for address {}", binance_addr);
    }
    
    // Test non-Binance addresses
    let non_binance_addresses = vec![
        "0x1111111111111111111111111111111111111111",
        "0x2222222222222222222222222222222222222222",
        "0x3333333333333333333333333333333333333333",
        "0x0000000000000000000000000000000000000000",
    ];
    
    for addr in non_binance_addresses {
        let transfer = ProcessedTransfer {
            block_number: 1000,
            transaction_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            log_index: 0,
            from_address: addr.to_string(),
            to_address: "0x4444444444444444444444444444444444444444".to_string(),
            amount: "1000000000000000000".to_string(),
            timestamp: 1640995200,
            direction: TransferDirection::NotRelevant,
        };
        
        let classified = transfer_detector.classify_transfer_direction(&transfer);
        assert_eq!(classified, TransferDirection::NotRelevant, 
            "Should classify as not relevant for non-Binance address {}", addr);
    }
    
    println!("All Binance address classification tests passed");
}

/// Test validation of amount parsing and precision
#[test]
fn test_amount_parsing_validation() {
    let test_cases = vec![
        // (hex_data, expected_decimal_string, description)
        ("0x0000000000000000000000000000000000000000000000000de0b6b3a7640000", "1000000000000000000", "1 POL"),
        ("0x00000000000000000000000000000000000000000000000006f05b59d3b20000", "500000000000000000", "0.5 POL"),
        ("0x0000000000000000000000000000000000000000000000001bc16d674ec80000", "2000000000000000000", "2 POL"),
        ("0x0000000000000000000000000000000000000000000000000000000000000001", "1", "1 wei"),
        ("0x0000000000000000000000000000000000000000000000000000000000000000", "0", "0 POL"),
        ("0x00000000000000000000000000000000000000000000d3c21bcecceda1000000", "1000000000000000000000", "1000 POL"),
    ];
    
    let transfer_detector = TransferDetector::new();
    
    for (hex_data, expected_amount, description) in test_cases {
        println!("Testing amount parsing: {}", description);
        
        // Create a mock log with the test data
        let log = RawLog {
            address: POL_TOKEN_ADDRESS.to_string(),
            topics: vec![
                TRANSFER_EVENT_SIGNATURE.to_string(),
                "0x0000000000000000000000001111111111111111111111111111111111111111".to_string(),
                "0x000000000000000000000000f977814e90da44bfa03b6295a0616a897441acec".to_string(),
            ],
            data: hex_data.to_string(),
            block_number: 12345,
            transaction_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            log_index: 0,
        };
        
        let result = transfer_detector.decode_transfer_log(&log);
        assert!(result.is_ok(), "Failed to decode amount for '{}': {:?}", description, result);
        
        let transfer = result.unwrap();
        assert_eq!(transfer.amount, expected_amount, "Amount mismatch for '{}'", description);
        
        // Verify amount can be parsed as a number
        let parsed_amount = transfer.amount.parse::<u128>();
        assert!(parsed_amount.is_ok(), "Amount should be parseable as u128 for '{}'", description);
        
        println!("  ✓ Parsed {} as {}", hex_data, expected_amount);
    }
    
    println!("All amount parsing validation tests passed");
}

/// Test validation of edge cases and error conditions
#[test]
fn test_edge_case_validation() {
    let transfer_detector = TransferDetector::new();
    
    // Test invalid POL transfer logs
    let invalid_logs = vec![
        // Wrong contract address
        RawLog {
            address: "0x2791bca1f2de4661ed88a30c99a7a9449aa84174".to_string(), // USDC
            topics: vec![
                TRANSFER_EVENT_SIGNATURE.to_string(),
                "0x0000000000000000000000001111111111111111111111111111111111111111".to_string(),
                "0x000000000000000000000000f977814e90da44bfa03b6295a0616a897441acec".to_string(),
            ],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            block_number: 12345,
            transaction_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            log_index: 0,
        },
        // Wrong event signature
        RawLog {
            address: POL_TOKEN_ADDRESS.to_string(),
            topics: vec![
                "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925".to_string(), // Approval event
                "0x0000000000000000000000001111111111111111111111111111111111111111".to_string(),
                "0x000000000000000000000000f977814e90da44bfa03b6295a0616a897441acec".to_string(),
            ],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            block_number: 12345,
            transaction_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            log_index: 0,
        },
        // Insufficient topics
        RawLog {
            address: POL_TOKEN_ADDRESS.to_string(),
            topics: vec![
                TRANSFER_EVENT_SIGNATURE.to_string(),
                "0x0000000000000000000000001111111111111111111111111111111111111111".to_string(),
                // Missing to address topic
            ],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            block_number: 12345,
            transaction_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            log_index: 0,
        },
    ];
    
    for (i, log) in invalid_logs.iter().enumerate() {
        println!("Testing invalid log {}", i + 1);
        
        // Should not be identified as POL transfer
        assert!(!transfer_detector.is_pol_transfer(log), 
            "Should not identify invalid log {} as POL transfer", i + 1);
        
        // If somehow it gets to decoding, it should fail gracefully
        if transfer_detector.is_pol_transfer(log) {
            let result = transfer_detector.decode_transfer_log(log);
            assert!(result.is_err(), "Should fail to decode invalid log {}", i + 1);
        }
    }
    
    println!("All edge case validation tests passed");
}

/// Test validation of net flow calculations with complex scenarios
#[tokio::test]
async fn test_complex_net_flow_validation() {
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Complex scenario with multiple transfers
    let complex_transfers = vec![
        // Large inflow
        create_test_transfer(1001, "user1", "binance1", "10000000000000000000", TransferDirection::ToBinance), // 10 POL
        // Small outflow
        create_test_transfer(1002, "binance1", "user2", "1500000000000000000", TransferDirection::FromBinance), // 1.5 POL
        // Medium inflow
        create_test_transfer(1003, "user3", "binance2", "5000000000000000000", TransferDirection::ToBinance), // 5 POL
        // Large outflow
        create_test_transfer(1004, "binance2", "user4", "8000000000000000000", TransferDirection::FromBinance), // 8 POL
        // Small inflow
        create_test_transfer(1005, "user5", "binance3", "2500000000000000000", TransferDirection::ToBinance), // 2.5 POL
    ];
    
    // Process transfers and track expected values
    let mut expected_inflow = 0u128;
    let mut expected_outflow = 0u128;
    
    for transfer in &complex_transfers {
        let result = database.store_transfer_and_update_net_flow(transfer);
        assert!(result.is_ok(), "Failed to store complex transfer: {:?}", result);
        
        match transfer.direction {
            TransferDirection::ToBinance => {
                expected_inflow += transfer.amount.parse::<u128>().unwrap();
            }
            TransferDirection::FromBinance => {
                expected_outflow += transfer.amount.parse::<u128>().unwrap();
            }
            TransferDirection::NotRelevant => {
                panic!("Should not have NotRelevant transfers in test data");
            }
        }
        
        // Verify intermediate calculations
        let current_net_flow = database.get_net_flow_data().expect("Failed to get intermediate net flow");
        let current_expected_net = expected_inflow - expected_outflow;
        
        assert_eq!(current_net_flow.total_inflow, expected_inflow.to_string(), 
            "Intermediate inflow mismatch at block {}", transfer.block_number);
        assert_eq!(current_net_flow.total_outflow, expected_outflow.to_string(), 
            "Intermediate outflow mismatch at block {}", transfer.block_number);
        assert_eq!(current_net_flow.net_flow, current_expected_net.to_string(), 
            "Intermediate net flow mismatch at block {}", transfer.block_number);
    }
    
    // Verify final calculations
    let final_net_flow = database.get_net_flow_data().expect("Failed to get final net flow");
    let final_expected_net = expected_inflow - expected_outflow;
    
    assert_eq!(final_net_flow.total_inflow, expected_inflow.to_string());
    assert_eq!(final_net_flow.total_outflow, expected_outflow.to_string());
    assert_eq!(final_net_flow.net_flow, final_expected_net.to_string());
    
    // Expected values: 17.5 POL inflow, 9.5 POL outflow, 8 POL net inflow
    assert_eq!(expected_inflow, 17500000000000000000u128);
    assert_eq!(expected_outflow, 9500000000000000000u128);
    assert_eq!(final_expected_net, 8000000000000000000u128);
    
    println!("Complex net flow validation completed successfully");
    println!("Total inflow: {} POL", format_wei_to_pol(&expected_inflow.to_string()));
    println!("Total outflow: {} POL", format_wei_to_pol(&expected_outflow.to_string()));
    println!("Net flow: {} POL", format_wei_to_pol(&final_expected_net.to_string()));
}

/// Test validation of data consistency across system restart
#[tokio::test]
async fn test_data_consistency_validation() {
    use tempfile::TempDir;
    
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("consistency_test.db");
    
    // Phase 1: Store initial data
    {
        let database = Database::new(db_path.to_str().unwrap()).expect("Failed to create database");
        
        let initial_transfers = vec![
            create_test_transfer(1001, "user1", "binance1", "3000000000000000000", TransferDirection::ToBinance),
            create_test_transfer(1002, "binance1", "user2", "1000000000000000000", TransferDirection::FromBinance),
        ];
        
        for transfer in &initial_transfers {
            database.store_transfer_and_update_net_flow(transfer)
                .expect("Failed to store initial transfer");
            database.set_last_processed_block(transfer.block_number)
                .expect("Failed to set last processed block");
        }
        
        let phase1_net_flow = database.get_net_flow_data().expect("Failed to get phase 1 net flow");
        assert_eq!(phase1_net_flow.total_inflow, "3000000000000000000");
        assert_eq!(phase1_net_flow.total_outflow, "1000000000000000000");
        assert_eq!(phase1_net_flow.net_flow, "2000000000000000000");
        assert_eq!(phase1_net_flow.last_processed_block, 1002);
    }
    
    // Phase 2: Reopen database and continue (simulating restart)
    {
        let database = Database::new(db_path.to_str().unwrap()).expect("Failed to reopen database");
        
        // Verify data persisted correctly
        let restored_net_flow = database.get_net_flow_data().expect("Failed to get restored net flow");
        assert_eq!(restored_net_flow.total_inflow, "3000000000000000000");
        assert_eq!(restored_net_flow.total_outflow, "1000000000000000000");
        assert_eq!(restored_net_flow.net_flow, "2000000000000000000");
        assert_eq!(restored_net_flow.last_processed_block, 1002);
        
        let restored_count = database.get_transaction_count().expect("Failed to get restored count");
        assert_eq!(restored_count, 2);
        
        // Add more data
        let additional_transfers = vec![
            create_test_transfer(1003, "user3", "binance2", "1500000000000000000", TransferDirection::ToBinance),
            create_test_transfer(1004, "binance2", "user4", "500000000000000000", TransferDirection::FromBinance),
        ];
        
        for transfer in &additional_transfers {
            database.store_transfer_and_update_net_flow(transfer)
                .expect("Failed to store additional transfer");
            database.set_last_processed_block(transfer.block_number)
                .expect("Failed to set last processed block");
        }
        
        let final_net_flow = database.get_net_flow_data().expect("Failed to get final net flow");
        assert_eq!(final_net_flow.total_inflow, "4500000000000000000"); // 3 + 1.5 POL
        assert_eq!(final_net_flow.total_outflow, "1500000000000000000"); // 1 + 0.5 POL
        assert_eq!(final_net_flow.net_flow, "3000000000000000000"); // 3 POL net
        assert_eq!(final_net_flow.last_processed_block, 1004);
        
        let final_count = database.get_transaction_count().expect("Failed to get final count");
        assert_eq!(final_count, 4);
    }
    
    println!("Data consistency validation across restart completed successfully");
}

// Helper functions

fn create_known_transfer_patterns() -> Vec<(String, ProcessedTransfer)> {
    vec![
        ("Large inflow to Binance hot wallet".to_string(), ProcessedTransfer {
            block_number: 40000001,
            transaction_hash: "0xa1b2c3d4e5f6789012345678901234567890123456789012345678901234567890".to_string(),
            log_index: 0,
            from_address: "0x742d35cc6634c0532925a3b8d0c9e3e0c0c0c0c0".to_string(),
            to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(), // Binance 8
            amount: "2500000000000000000".to_string(), // 2.5 POL
            timestamp: 1640995200,
            direction: TransferDirection::ToBinance,
        }),
        ("Medium outflow from Binance cold wallet".to_string(), ProcessedTransfer {
            block_number: 40000002,
            transaction_hash: "0xb2c3d4e5f6789012345678901234567890123456789012345678901234567890a1".to_string(),
            log_index: 0,
            from_address: "0xe7804c37c13166ff0b37f5ae0bb07a3aebb6e245".to_string(), // Binance 14
            to_address: "0x123456789012345678901234567890123456789012".to_string(),
            amount: "1800000000000000000".to_string(), // 1.8 POL
            timestamp: 1640995260,
            direction: TransferDirection::FromBinance,
        }),
        ("Small inflow to Binance deposit wallet".to_string(), ProcessedTransfer {
            block_number: 40000003,
            transaction_hash: "0xc3d4e5f6789012345678901234567890123456789012345678901234567890a1b2".to_string(),
            log_index: 0,
            from_address: "0x987654321098765432109876543210987654321098".to_string(),
            to_address: "0x505e71695e9bc45943c58adec1650577bca68fd9".to_string(), // Binance 3
            amount: "750000000000000000".to_string(), // 0.75 POL
            timestamp: 1640995320,
            direction: TransferDirection::ToBinance,
        }),
        ("Large inflow to Binance main wallet".to_string(), ProcessedTransfer {
            block_number: 40000004,
            transaction_hash: "0xd4e5f6789012345678901234567890123456789012345678901234567890a1b2c3".to_string(),
            log_index: 0,
            from_address: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdef".to_string(),
            to_address: "0x290275e3db66394c52272398959845170e4dcb88".to_string(), // Binance 4
            amount: "2250000000000000000".to_string(), // 2.25 POL
            timestamp: 1640995380,
            direction: TransferDirection::ToBinance,
        }),
        ("Small outflow from Binance withdrawal wallet".to_string(), ProcessedTransfer {
            block_number: 40000005,
            transaction_hash: "0xe5f6789012345678901234567890123456789012345678901234567890a1b2c3d4".to_string(),
            log_index: 0,
            from_address: "0xd5c08681719445a5fdce2bda98b341a49050d821".to_string(), // Binance 5
            to_address: "0x1111111111111111111111111111111111111111".to_string(),
            amount: "500000000000000000".to_string(), // 0.5 POL
            timestamp: 1640995440,
            direction: TransferDirection::FromBinance,
        }),
    ]
}

fn create_known_pol_transfer_logs() -> Vec<(String, RawLog, String, String, String)> {
    vec![
        ("Standard POL transfer to Binance".to_string(), RawLog {
            address: POL_TOKEN_ADDRESS.to_string(),
            topics: vec![
                TRANSFER_EVENT_SIGNATURE.to_string(),
                "0x000000000000000000000000742d35cc6634c0532925a3b8d0c9e3e0c0c0c0c0".to_string(),
                "0x000000000000000000000000f977814e90da44bfa03b6295a0616a897441acec".to_string(),
            ],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            block_number: 40000001,
            transaction_hash: "0xa1b2c3d4e5f6789012345678901234567890123456789012345678901234567890".to_string(),
            log_index: 0,
        }, "1000000000000000000".to_string(), "0x742d35cc6634c0532925a3b8d0c9e3e0c0c0c0c0".to_string(), "0xf977814e90da44bfa03b6295a0616a897441acec".to_string()),
        
        ("Large POL transfer from Binance".to_string(), RawLog {
            address: POL_TOKEN_ADDRESS.to_string(),
            topics: vec![
                TRANSFER_EVENT_SIGNATURE.to_string(),
                "0x000000000000000000000000e7804c37c13166ff0b37f5ae0bb07a3aebb6e245".to_string(),
                "0x000000000000000000000000123456789012345678901234567890123456789012".to_string(),
            ],
            data: "0x0000000000000000000000000000000000000000000000001bc16d674ec80000".to_string(),
            block_number: 40000002,
            transaction_hash: "0xb2c3d4e5f6789012345678901234567890123456789012345678901234567890a1".to_string(),
            log_index: 0,
        }, "2000000000000000000".to_string(), "0xe7804c37c13166ff0b37f5ae0bb07a3aebb6e245".to_string(), "0x123456789012345678901234567890123456789012".to_string()),
        
        ("Small POL transfer to Binance".to_string(), RawLog {
            address: POL_TOKEN_ADDRESS.to_string(),
            topics: vec![
                TRANSFER_EVENT_SIGNATURE.to_string(),
                "0x000000000000000000000000987654321098765432109876543210987654321098".to_string(),
                "0x000000000000000000000000505e71695e9bc45943c58adec1650577bca68fd9".to_string(),
            ],
            data: "0x00000000000000000000000000000000000000000000000006f05b59d3b20000".to_string(),
            block_number: 40000003,
            transaction_hash: "0xc3d4e5f6789012345678901234567890123456789012345678901234567890a1b2".to_string(),
            log_index: 0,
        }, "500000000000000000".to_string(), "0x987654321098765432109876543210987654321098".to_string(), "0x505e71695e9bc45943c58adec1650577bca68fd9".to_string()),
    ]
}

fn create_test_transfer(
    block_number: u64,
    from: &str,
    to: &str,
    amount: &str,
    direction: TransferDirection,
) -> ProcessedTransfer {
    let binance_addresses = [
        "0xf977814e90da44bfa03b6295a0616a897441acec", // binance1
        "0xe7804c37c13166ff0b37f5ae0bb07a3aebb6e245", // binance2
        "0x505e71695e9bc45943c58adec1650577bca68fd9", // binance3
    ];
    
    let from_address = if from.starts_with("binance") {
        let index = from.chars().last().unwrap().to_digit(10).unwrap_or(1) as usize - 1;
        binance_addresses[index.min(binance_addresses.len() - 1)].to_string()
    } else {
        format!("0x{:040}", from)
    };
    
    let to_address = if to.starts_with("binance") {
        let index = to.chars().last().unwrap().to_digit(10).unwrap_or(1) as usize - 1;
        binance_addresses[index.min(binance_addresses.len() - 1)].to_string()
    } else {
        format!("0x{:040}", to)
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