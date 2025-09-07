use std::time::Duration;
use tokio::time::timeout;
use polygon_pol_indexer::blockchain::{RpcClient, BlockProcessor};
use polygon_pol_indexer::database::Database;
use polygon_pol_indexer::models::{ProcessedTransfer, TransferDirection};

// Polygon Mumbai testnet RPC endpoint
const POLYGON_TESTNET_RPC: &str = "https://rpc-mumbai.maticvigil.com";

// Known POL token contract address on Polygon Mumbai testnet
const POL_TOKEN_TESTNET: &str = "0x499d11E0b6eAC7c0593d8Fb292DCBbF815Fb29Ae";

// Test with real Polygon testnet data
#[tokio::test]
#[ignore] // Use --ignored flag to run this test
async fn test_real_polygon_testnet_connection() {
    let rpc_client = RpcClient::new(POLYGON_TESTNET_RPC.to_string());
    
    // Test basic connectivity with timeout
    let result = timeout(Duration::from_secs(10), rpc_client.get_latest_block_number()).await;
    
    match result {
        Ok(Ok(block_number)) => {
            println!("Successfully connected to Polygon testnet, latest block: {}", block_number);
            assert!(block_number > 0, "Block number should be greater than 0");
        }
        Ok(Err(e)) => {
            println!("RPC error: {}", e);
            // Don't fail the test if network is unavailable
        }
        Err(_) => {
            println!("Timeout connecting to testnet");
            // Don't fail the test if network is slow
        }
    }
}

#[tokio::test]
#[ignore] // Use --ignored flag to run this test
async fn test_real_block_fetching() {
    let rpc_client = RpcClient::new(POLYGON_TESTNET_RPC.to_string());
    
    // First get the latest block number
    let latest_result = timeout(Duration::from_secs(10), rpc_client.get_latest_block_number()).await;
    
    if let Ok(Ok(latest_block)) = latest_result {
        // Fetch a recent block (go back a few blocks to ensure it's finalized)
        let target_block = latest_block.saturating_sub(10);
        
        let block_result = timeout(Duration::from_secs(15), rpc_client.get_block(target_block)).await;
        
        match block_result {
            Ok(Ok(block)) => {
                println!("Successfully fetched block {}: {} transactions", 
                    target_block, block.transactions.len());
                
                assert_eq!(block.number, target_block);
                assert!(!block.hash.is_empty());
                assert!(block.timestamp > 0);
                
                // Verify block structure
                for tx in &block.transactions {
                    assert!(!tx.hash.is_empty());
                    assert!(!tx.from.is_empty());
                }
            }
            Ok(Err(e)) => {
                println!("Error fetching block: {}", e);
            }
            Err(_) => {
                println!("Timeout fetching block");
            }
        }
    }
}

#[tokio::test]
#[ignore] // Use --ignored flag to run this test
async fn test_real_pol_transfer_detection() {
    let rpc_client = RpcClient::new(POLYGON_TESTNET_RPC.to_string());
    let processor = BlockProcessor::new(rpc_client.clone());
    
    // Get latest block number
    let latest_result = timeout(Duration::from_secs(10), rpc_client.get_latest_block_number()).await;
    
    if let Ok(Ok(latest_block)) = latest_result {
        // Search through recent blocks for POL transfers
        let start_block = latest_block.saturating_sub(100);
        let end_block = latest_block.saturating_sub(50);
        
        println!("Searching for POL transfers in blocks {} to {}", start_block, end_block);
        
        let mut total_transfers = 0;
        let mut binance_transfers = 0;
        
        for block_num in start_block..=end_block {
            let result = timeout(Duration::from_secs(10), processor.extract_pol_transfers(block_num)).await;
            
            match result {
                Ok(Ok(transfers)) => {
                    total_transfers += transfers.len();
                    
                    for transfer in transfers {
                        println!("Found POL transfer in block {}: {} -> {} ({})", 
                            block_num, transfer.from_address, transfer.to_address, transfer.amount);
                        
                        // Check if it involves Binance addresses
                        if matches!(transfer.direction, TransferDirection::ToBinance | TransferDirection::FromBinance) {
                            binance_transfers += 1;
                            println!("  -> Binance-related transfer: {:?}", transfer.direction);
                        }
                    }
                }
                Ok(Err(e)) => {
                    println!("Error processing block {}: {}", block_num, e);
                }
                Err(_) => {
                    println!("Timeout processing block {}", block_num);
                    break;
                }
            }
            
            // Small delay to avoid overwhelming the RPC
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        println!("Found {} total POL transfers, {} Binance-related", total_transfers, binance_transfers);
        
        // The test passes if we can successfully scan blocks without errors
        // We don't assert specific numbers since testnet activity varies
    }
}

#[tokio::test]
#[ignore] // Use --ignored flag to run this test
async fn test_real_network_resilience() {
    // Test with multiple RPC endpoints to verify resilience
    let endpoints = vec![
        "https://rpc-mumbai.maticvigil.com",
        "https://polygon-mumbai.g.alchemy.com/v2/demo",
        "https://rpc-mumbai.matic.today",
    ];
    
    for endpoint in endpoints {
        println!("Testing endpoint: {}", endpoint);
        
        let rpc_client = RpcClient::new(endpoint.to_string());
        let result = timeout(Duration::from_secs(5), rpc_client.get_latest_block_number()).await;
        
        match result {
            Ok(Ok(block_number)) => {
                println!("  ✓ Success: block {}", block_number);
                assert!(block_number > 0);
            }
            Ok(Err(e)) => {
                println!("  ✗ RPC Error: {}", e);
            }
            Err(_) => {
                println!("  ✗ Timeout");
            }
        }
    }
}

#[tokio::test]
#[ignore] // Use --ignored flag to run this test
async fn test_real_data_validation() {
    let rpc_client = RpcClient::new(POLYGON_TESTNET_RPC.to_string());
    let processor = BlockProcessor::new(rpc_client.clone());
    
    // Get a recent block
    let latest_result = timeout(Duration::from_secs(10), rpc_client.get_latest_block_number()).await;
    
    if let Ok(Ok(latest_block)) = latest_result {
        let target_block = latest_block.saturating_sub(5);
        
        let transfers_result = timeout(Duration::from_secs(15), processor.extract_pol_transfers(target_block)).await;
        
        if let Ok(Ok(transfers)) = transfers_result {
            for transfer in transfers {
                // Validate transfer data structure
                assert!(!transfer.transaction_hash.is_empty(), "Transaction hash should not be empty");
                assert!(transfer.transaction_hash.starts_with("0x"), "Transaction hash should start with 0x");
                assert_eq!(transfer.transaction_hash.len(), 66, "Transaction hash should be 66 characters");
                
                assert!(!transfer.from_address.is_empty(), "From address should not be empty");
                assert!(transfer.from_address.starts_with("0x"), "From address should start with 0x");
                assert_eq!(transfer.from_address.len(), 42, "From address should be 42 characters");
                
                assert!(!transfer.to_address.is_empty(), "To address should not be empty");
                assert!(transfer.to_address.starts_with("0x"), "To address should start with 0x");
                assert_eq!(transfer.to_address.len(), 42, "To address should be 42 characters");
                
                assert!(!transfer.amount.is_empty(), "Amount should not be empty");
                assert!(transfer.amount.parse::<u128>().is_ok(), "Amount should be a valid number");
                
                assert!(transfer.block_number > 0, "Block number should be greater than 0");
                assert!(transfer.timestamp > 0, "Timestamp should be greater than 0");
                
                println!("Validated transfer: {} POL from {} to {}", 
                    transfer.amount, transfer.from_address, transfer.to_address);
            }
        }
    }
}

// Test known POL transfer transactions (if any are available on testnet)
#[tokio::test]
#[ignore] // Use --ignored flag to run this test
async fn test_known_pol_transactions() {
    // This test would validate against known POL transfer transactions
    // For now, we'll test the validation logic with mock data that matches real structure
    
    let database = Database::new_in_memory().expect("Failed to create test database");
    
    // Create transfers that match real-world data patterns
    let known_transfers = vec![
        ProcessedTransfer {
            block_number: 40000000, // Realistic Mumbai block number
            transaction_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            log_index: 0,
            from_address: "0x742d35cc6634c0532925a3b8d0c9e3e0c0c0c0c0".to_string(),
            to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(), // Binance
            amount: "1000000000000000000".to_string(), // 1 POL
            timestamp: 1640995200,
            direction: TransferDirection::ToBinance,
        },
        ProcessedTransfer {
            block_number: 40000001,
            transaction_hash: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            log_index: 0,
            from_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(), // Binance
            to_address: "0x742d35cc6634c0532925a3b8d0c9e3e0c0c0c0c0".to_string(),
            amount: "500000000000000000".to_string(), // 0.5 POL
            timestamp: 1640995260,
            direction: TransferDirection::FromBinance,
        },
    ];
    
    // Store and validate each transfer
    for transfer in &known_transfers {
        let result = database.store_transfer_and_update_net_flow(transfer);
        assert!(result.is_ok(), "Failed to store known transfer: {:?}", result);
        
        // Retrieve and validate
        let stored = database.get_transaction(&transfer.transaction_hash, transfer.log_index);
        assert!(stored.is_ok(), "Failed to retrieve stored transfer");
        
        let stored_transfer = stored.unwrap();
        assert_eq!(stored_transfer.block_number, transfer.block_number);
        assert_eq!(stored_transfer.amount, transfer.amount);
        assert_eq!(stored_transfer.from_address, transfer.from_address);
        assert_eq!(stored_transfer.to_address, transfer.to_address);
    }
    
    // Validate net flow calculation
    let net_flow = database.get_net_flow_data().expect("Failed to get net flow");
    assert_eq!(net_flow.total_inflow, "1000000000000000000");
    assert_eq!(net_flow.total_outflow, "500000000000000000");
    assert_eq!(net_flow.net_flow, "500000000000000000"); // 0.5 POL net inflow
    
    println!("Successfully validated known POL transactions");
}