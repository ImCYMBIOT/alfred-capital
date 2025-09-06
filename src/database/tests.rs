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
}