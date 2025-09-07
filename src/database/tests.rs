#[cfg(test)]
mod tests {
    use crate::database::{Database, DbError};

    #[test]
    fn test_database_creation() {
        let db = Database::new_in_memory().expect("Failed to create in-memory database");
        
        // Verify that the database was created successfully
        let count = db.get_transaction_count().expect("Failed to get transaction count");
        assert_eq!(count, 0);
        
        // Verify that net_flows table is initialized
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow data");
        assert_eq!(net_flow.total_inflow, "0");
        assert_eq!(net_flow.total_outflow, "0");
        assert_eq!(net_flow.net_flow, "0");
        assert_eq!(net_flow.last_processed_block, 0);
    }

    #[test]
    fn test_store_and_retrieve_transaction() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Store a test transaction
        db.store_transaction(
            12345,
            "0xabcdef1234567890",
            0,
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            1640995200,
            "inflow",
        ).expect("Failed to store transaction");
        
        // Retrieve the transaction
        let transaction = db.get_transaction("0xabcdef1234567890", 0)
            .expect("Failed to retrieve transaction");
        
        assert_eq!(transaction.block_number, 12345);
        assert_eq!(transaction.transaction_hash, "0xabcdef1234567890");
        assert_eq!(transaction.log_index, 0);
        assert_eq!(transaction.from_address, "0x1111111111111111111111111111111111111111");
        assert_eq!(transaction.to_address, "0x2222222222222222222222222222222222222222");
        assert_eq!(transaction.amount, "1000000000000000000");
        assert_eq!(transaction.timestamp, 1640995200);
        assert_eq!(transaction.direction, "inflow");
    }

    #[test]
    fn test_get_nonexistent_transaction() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        let result = db.get_transaction("0xnonexistent", 0);
        assert!(matches!(result, Err(DbError::NotFound)));
    }

    #[test]
    fn test_get_transactions_by_block() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Store multiple transactions in the same block
        db.store_transaction(
            12345,
            "0xhash1",
            0,
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            1640995200,
            "inflow",
        ).expect("Failed to store transaction 1");
        
        db.store_transaction(
            12345,
            "0xhash2",
            1,
            "0x3333333333333333333333333333333333333333",
            "0x4444444444444444444444444444444444444444",
            "2000000000000000000",
            1640995201,
            "outflow",
        ).expect("Failed to store transaction 2");
        
        // Store a transaction in a different block
        db.store_transaction(
            12346,
            "0xhash3",
            0,
            "0x5555555555555555555555555555555555555555",
            "0x6666666666666666666666666666666666666666",
            "3000000000000000000",
            1640995202,
            "inflow",
        ).expect("Failed to store transaction 3");
        
        // Retrieve transactions from block 12345
        let transactions = db.get_transactions_by_block(12345)
            .expect("Failed to get transactions by block");
        
        assert_eq!(transactions.len(), 2);
        assert_eq!(transactions[0].transaction_hash, "0xhash1");
        assert_eq!(transactions[1].transaction_hash, "0xhash2");
        
        // Retrieve transactions from block 12346
        let transactions = db.get_transactions_by_block(12346)
            .expect("Failed to get transactions by block");
        
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].transaction_hash, "0xhash3");
        
        // Retrieve transactions from non-existent block
        let transactions = db.get_transactions_by_block(99999)
            .expect("Failed to get transactions by block");
        
        assert_eq!(transactions.len(), 0);
    }

    #[test]
    fn test_update_transaction() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Store a transaction
        db.store_transaction(
            12345,
            "0xabcdef1234567890",
            0,
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            1640995200,
            "inflow",
        ).expect("Failed to store transaction");
        
        // Update the transaction amount
        db.update_transaction_amount("0xabcdef1234567890", 0, "2000000000000000000")
            .expect("Failed to update transaction");
        
        // Verify the update
        let transaction = db.get_transaction("0xabcdef1234567890", 0)
            .expect("Failed to retrieve updated transaction");
        
        assert_eq!(transaction.amount, "2000000000000000000");
        
        // Try to update non-existent transaction
        let result = db.update_transaction_amount("0xnonexistent", 0, "1000000000000000000");
        assert!(matches!(result, Err(DbError::NotFound)));
    }

    #[test]
    fn test_delete_transaction() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Store a transaction
        db.store_transaction(
            12345,
            "0xabcdef1234567890",
            0,
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            1640995200,
            "inflow",
        ).expect("Failed to store transaction");
        
        // Verify it exists
        let _transaction = db.get_transaction("0xabcdef1234567890", 0)
            .expect("Transaction should exist");
        
        // Delete the transaction
        db.delete_transaction("0xabcdef1234567890", 0)
            .expect("Failed to delete transaction");
        
        // Verify it's gone
        let result = db.get_transaction("0xabcdef1234567890", 0);
        assert!(matches!(result, Err(DbError::NotFound)));
        
        // Try to delete non-existent transaction
        let result = db.delete_transaction("0xnonexistent", 0);
        assert!(matches!(result, Err(DbError::NotFound)));
    }

    #[test]
    fn test_last_processed_block() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Initial value should be 0
        let block_number = db.get_last_processed_block()
            .expect("Failed to get last processed block");
        assert_eq!(block_number, 0);
        
        // Update the block number
        db.set_last_processed_block(12345)
            .expect("Failed to set last processed block");
        
        // Verify the update
        let block_number = db.get_last_processed_block()
            .expect("Failed to get last processed block");
        assert_eq!(block_number, 12345);
    }

    #[test]
    fn test_transaction_count() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Initial count should be 0
        let count = db.get_transaction_count()
            .expect("Failed to get transaction count");
        assert_eq!(count, 0);
        
        // Add some transactions
        db.store_transaction(
            12345,
            "0xhash1",
            0,
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            1640995200,
            "inflow",
        ).expect("Failed to store transaction 1");
        
        db.store_transaction(
            12346,
            "0xhash2",
            0,
            "0x3333333333333333333333333333333333333333",
            "0x4444444444444444444444444444444444444444",
            "2000000000000000000",
            1640995201,
            "outflow",
        ).expect("Failed to store transaction 2");
        
        // Count should be 2
        let count = db.get_transaction_count()
            .expect("Failed to get transaction count");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_duplicate_transaction_hash_different_log_index() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Store two transactions with same hash but different log index
        db.store_transaction(
            12345,
            "0xsamehash",
            0,
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            1640995200,
            "inflow",
        ).expect("Failed to store transaction 1");
        
        db.store_transaction(
            12345,
            "0xsamehash",
            1,
            "0x3333333333333333333333333333333333333333",
            "0x4444444444444444444444444444444444444444",
            "2000000000000000000",
            1640995201,
            "outflow",
        ).expect("Failed to store transaction 2");
        
        // Both should be retrievable
        let tx1 = db.get_transaction("0xsamehash", 0)
            .expect("Failed to get transaction 1");
        let tx2 = db.get_transaction("0xsamehash", 1)
            .expect("Failed to get transaction 2");
        
        assert_eq!(tx1.log_index, 0);
        assert_eq!(tx2.log_index, 1);
        assert_eq!(tx1.amount, "1000000000000000000");
        assert_eq!(tx2.amount, "2000000000000000000");
    }

    #[test]
    fn test_duplicate_transaction_hash_same_log_index_fails() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Store a transaction
        db.store_transaction(
            12345,
            "0xsamehash",
            0,
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            1640995200,
            "inflow",
        ).expect("Failed to store transaction 1");
        
        // Try to store another transaction with same hash and log index
        let result = db.store_transaction(
            12345,
            "0xsamehash",
            0,
            "0x3333333333333333333333333333333333333333",
            "0x4444444444444444444444444444444444444444",
            "2000000000000000000",
            1640995201,
            "outflow",
        );
        
        // Should fail due to unique constraint
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_direction_constraint() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Try to store a transaction with invalid direction
        let result = db.store_transaction(
            12345,
            "0xhash",
            0,
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "1000000000000000000",
            1640995200,
            "invalid_direction",
        );
        
        // Should fail due to CHECK constraint
        assert!(result.is_err());
    }

    #[test]
    fn test_update_net_flow_inflow() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Initial net flow should be zero
        let net_flow = db.get_net_flow_data().expect("Failed to get initial net flow");
        assert_eq!(net_flow.total_inflow, "0");
        assert_eq!(net_flow.total_outflow, "0");
        assert_eq!(net_flow.net_flow, "0");
        
        // Add first inflow
        db.update_net_flow_inflow("1000").expect("Failed to update inflow");
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow after first inflow");
        assert_eq!(net_flow.total_inflow, "1000");
        assert_eq!(net_flow.total_outflow, "0");
        assert_eq!(net_flow.net_flow, "1000");
        
        // Add second inflow
        db.update_net_flow_inflow("500").expect("Failed to update inflow");
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow after second inflow");
        assert_eq!(net_flow.total_inflow, "1500");
        assert_eq!(net_flow.total_outflow, "0");
        assert_eq!(net_flow.net_flow, "1500");
    }

    #[test]
    fn test_update_net_flow_outflow() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Initial net flow should be zero
        let net_flow = db.get_net_flow_data().expect("Failed to get initial net flow");
        assert_eq!(net_flow.total_inflow, "0");
        assert_eq!(net_flow.total_outflow, "0");
        assert_eq!(net_flow.net_flow, "0");
        
        // Add first outflow
        db.update_net_flow_outflow("750").expect("Failed to update outflow");
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow after first outflow");
        assert_eq!(net_flow.total_inflow, "0");
        assert_eq!(net_flow.total_outflow, "750");
        assert_eq!(net_flow.net_flow, "-750");
        
        // Add second outflow
        db.update_net_flow_outflow("250").expect("Failed to update outflow");
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow after second outflow");
        assert_eq!(net_flow.total_inflow, "0");
        assert_eq!(net_flow.total_outflow, "1000");
        assert_eq!(net_flow.net_flow, "-1000");
    }

    #[test]
    fn test_update_net_flow_mixed_inflow_outflow() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Add inflow first
        db.update_net_flow_inflow("2000").expect("Failed to update inflow");
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow after inflow");
        assert_eq!(net_flow.total_inflow, "2000");
        assert_eq!(net_flow.total_outflow, "0");
        assert_eq!(net_flow.net_flow, "2000");
        
        // Add outflow
        db.update_net_flow_outflow("800").expect("Failed to update outflow");
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow after outflow");
        assert_eq!(net_flow.total_inflow, "2000");
        assert_eq!(net_flow.total_outflow, "800");
        assert_eq!(net_flow.net_flow, "1200");
        
        // Add more inflow
        db.update_net_flow_inflow("300").expect("Failed to update inflow");
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow after second inflow");
        assert_eq!(net_flow.total_inflow, "2300");
        assert_eq!(net_flow.total_outflow, "800");
        assert_eq!(net_flow.net_flow, "1500");
        
        // Add more outflow to make net negative
        db.update_net_flow_outflow("2000").expect("Failed to update outflow");
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow after large outflow");
        assert_eq!(net_flow.total_inflow, "2300");
        assert_eq!(net_flow.total_outflow, "2800");
        assert_eq!(net_flow.net_flow, "-500");
    }

    #[test]
    fn test_update_net_flow_with_transfer_direction() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Test ToBinance (inflow)
        db.update_net_flow_with_transfer("1000", &crate::models::TransferDirection::ToBinance)
            .expect("Failed to update with ToBinance transfer");
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow");
        assert_eq!(net_flow.total_inflow, "1000");
        assert_eq!(net_flow.total_outflow, "0");
        assert_eq!(net_flow.net_flow, "1000");
        
        // Test FromBinance (outflow)
        db.update_net_flow_with_transfer("600", &crate::models::TransferDirection::FromBinance)
            .expect("Failed to update with FromBinance transfer");
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow");
        assert_eq!(net_flow.total_inflow, "1000");
        assert_eq!(net_flow.total_outflow, "600");
        assert_eq!(net_flow.net_flow, "400");
        
        // Test NotRelevant (should not change anything)
        db.update_net_flow_with_transfer("999999", &crate::models::TransferDirection::NotRelevant)
            .expect("Failed to update with NotRelevant transfer");
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow");
        assert_eq!(net_flow.total_inflow, "1000");
        assert_eq!(net_flow.total_outflow, "600");
        assert_eq!(net_flow.net_flow, "400");
    }

    #[test]
    fn test_store_transfer_and_update_net_flow_inflow() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        let transfer = crate::models::ProcessedTransfer {
            block_number: 12345,
            transaction_hash: "0xabcdef1234567890".to_string(),
            log_index: 0,
            from_address: "0x1111111111111111111111111111111111111111".to_string(),
            to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(), // Binance address
            amount: "1500000000000000000000".to_string(), // 1500 POL in wei
            timestamp: 1640995200,
            direction: crate::models::TransferDirection::ToBinance,
        };
        
        // Store transfer and update net flow
        db.store_transfer_and_update_net_flow(&transfer)
            .expect("Failed to store transfer and update net flow");
        
        // Verify transaction was stored
        let stored_tx = db.get_transaction("0xabcdef1234567890", 0)
            .expect("Failed to retrieve stored transaction");
        assert_eq!(stored_tx.amount, "1500000000000000000000");
        assert_eq!(stored_tx.direction, "inflow");
        
        // Verify net flow was updated
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow");
        assert_eq!(net_flow.total_inflow, "1500000000000000000000");
        assert_eq!(net_flow.total_outflow, "0");
        assert_eq!(net_flow.net_flow, "1500000000000000000000");
        
        // Verify transaction count
        let count = db.get_transaction_count().expect("Failed to get transaction count");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_store_transfer_and_update_net_flow_outflow() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        let transfer = crate::models::ProcessedTransfer {
            block_number: 12346,
            transaction_hash: "0xfedcba0987654321".to_string(),
            log_index: 1,
            from_address: "0xe7804c37c13166ff0b37f5ae0bb07a3aebb6e245".to_string(), // Binance address
            to_address: "0x2222222222222222222222222222222222222222".to_string(),
            amount: "800000000000000000000".to_string(), // 800 POL in wei
            timestamp: 1640995300,
            direction: crate::models::TransferDirection::FromBinance,
        };
        
        // Store transfer and update net flow
        db.store_transfer_and_update_net_flow(&transfer)
            .expect("Failed to store transfer and update net flow");
        
        // Verify transaction was stored
        let stored_tx = db.get_transaction("0xfedcba0987654321", 1)
            .expect("Failed to retrieve stored transaction");
        assert_eq!(stored_tx.amount, "800000000000000000000");
        assert_eq!(stored_tx.direction, "outflow");
        
        // Verify net flow was updated
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow");
        assert_eq!(net_flow.total_inflow, "0");
        assert_eq!(net_flow.total_outflow, "800000000000000000000");
        assert_eq!(net_flow.net_flow, "-800000000000000000000");
    }

    #[test]
    fn test_store_transfer_and_update_net_flow_not_relevant() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        let transfer = crate::models::ProcessedTransfer {
            block_number: 12347,
            transaction_hash: "0x1234567890abcdef".to_string(),
            log_index: 0,
            from_address: "0x1111111111111111111111111111111111111111".to_string(),
            to_address: "0x2222222222222222222222222222222222222222".to_string(),
            amount: "500000000000000000000".to_string(),
            timestamp: 1640995400,
            direction: crate::models::TransferDirection::NotRelevant,
        };
        
        // Store transfer (should not store or update net flow)
        db.store_transfer_and_update_net_flow(&transfer)
            .expect("Failed to handle NotRelevant transfer");
        
        // Verify transaction was NOT stored
        let result = db.get_transaction("0x1234567890abcdef", 0);
        assert!(matches!(result, Err(crate::database::DbError::NotFound)));
        
        // Verify net flow was NOT updated
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow");
        assert_eq!(net_flow.total_inflow, "0");
        assert_eq!(net_flow.total_outflow, "0");
        assert_eq!(net_flow.net_flow, "0");
        
        // Verify transaction count is still 0
        let count = db.get_transaction_count().expect("Failed to get transaction count");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_store_multiple_transfers_and_update_net_flow() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Store multiple transfers
        let transfers = vec![
            crate::models::ProcessedTransfer {
                block_number: 12345,
                transaction_hash: "0xhash1".to_string(),
                log_index: 0,
                from_address: "0x1111111111111111111111111111111111111111".to_string(),
                to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
                amount: "1000000000000000000000".to_string(), // 1000 POL
                timestamp: 1640995200,
                direction: crate::models::TransferDirection::ToBinance,
            },
            crate::models::ProcessedTransfer {
                block_number: 12346,
                transaction_hash: "0xhash2".to_string(),
                log_index: 0,
                from_address: "0xe7804c37c13166ff0b37f5ae0bb07a3aebb6e245".to_string(),
                to_address: "0x2222222222222222222222222222222222222222".to_string(),
                amount: "600000000000000000000".to_string(), // 600 POL
                timestamp: 1640995300,
                direction: crate::models::TransferDirection::FromBinance,
            },
            crate::models::ProcessedTransfer {
                block_number: 12347,
                transaction_hash: "0xhash3".to_string(),
                log_index: 0,
                from_address: "0x3333333333333333333333333333333333333333".to_string(),
                to_address: "0x505e71695e9bc45943c58adec1650577bca68fd9".to_string(),
                amount: "2000000000000000000000".to_string(), // 2000 POL
                timestamp: 1640995400,
                direction: crate::models::TransferDirection::ToBinance,
            },
        ];
        
        // Store all transfers
        for transfer in &transfers {
            db.store_transfer_and_update_net_flow(transfer)
                .expect("Failed to store transfer");
        }
        
        // Verify final net flow calculations
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow");
        assert_eq!(net_flow.total_inflow, "3000000000000000000000"); // 1000 + 2000
        assert_eq!(net_flow.total_outflow, "600000000000000000000"); // 600
        assert_eq!(net_flow.net_flow, "2400000000000000000000"); // 3000 - 600
        
        // Verify transaction count
        let count = db.get_transaction_count().expect("Failed to get transaction count");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_net_flow_calculation_with_decimals() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Test with decimal amounts
        db.update_net_flow_inflow("1000.5").expect("Failed to update inflow");
        db.update_net_flow_outflow("500.25").expect("Failed to update outflow");
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow");
        assert_eq!(net_flow.total_inflow, "1000.5");
        assert_eq!(net_flow.total_outflow, "500.25");
        assert_eq!(net_flow.net_flow, "500.25");
    }

    #[test]
    fn test_net_flow_calculation_error_handling() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Test with invalid decimal format
        let result = db.update_net_flow_inflow("invalid_number");
        assert!(result.is_err());
        
        let result = db.update_net_flow_outflow("not_a_number");
        assert!(result.is_err());
        
        // Verify net flow remains unchanged after errors
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow");
        assert_eq!(net_flow.total_inflow, "0");
        assert_eq!(net_flow.total_outflow, "0");
        assert_eq!(net_flow.net_flow, "0");
    }

    #[test]
    fn test_atomic_transaction_rollback_on_error() {
        let db = Database::new_in_memory().expect("Failed to create database");
        
        // Create a transfer with invalid amount to trigger calculation error
        let transfer = crate::models::ProcessedTransfer {
            block_number: 12345,
            transaction_hash: "0xbadtransfer".to_string(),
            log_index: 0,
            from_address: "0x1111111111111111111111111111111111111111".to_string(),
            to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(),
            amount: "invalid_amount".to_string(),
            timestamp: 1640995200,
            direction: crate::models::TransferDirection::ToBinance,
        };
        
        // This should fail and rollback
        let result = db.store_transfer_and_update_net_flow(&transfer);
        assert!(result.is_err());
        
        // Verify that neither transaction was stored nor net flow updated
        let tx_result = db.get_transaction("0xbadtransfer", 0);
        assert!(matches!(tx_result, Err(crate::database::DbError::NotFound)));
        
        let net_flow = db.get_net_flow_data().expect("Failed to get net flow");
        assert_eq!(net_flow.total_inflow, "0");
        assert_eq!(net_flow.total_outflow, "0");
        assert_eq!(net_flow.net_flow, "0");
        
        let count = db.get_transaction_count().expect("Failed to get transaction count");
        assert_eq!(count, 0);
    }
}