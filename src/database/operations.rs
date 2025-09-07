use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use crate::database::schema::{initialize_schema, run_migrations};

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database connection failed: {0}")]
    Connection(#[from] rusqlite::Error),
    #[error("Database operation failed: {0}")]
    Operation(String),
    #[error("Transaction not found")]
    NotFound,
}

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Create a new database connection and initialize schema
    pub fn new(db_path: &str) -> Result<Self, DbError> {
        let conn = Connection::open(db_path)?;
        
        // Initialize schema
        initialize_schema(&conn)?;
        run_migrations(&conn)?;
        
        Ok(Database {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create an in-memory database for testing
    pub fn new_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        
        // Initialize schema
        initialize_schema(&conn)?;
        run_migrations(&conn)?;
        
        Ok(Database {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Store a transaction in the database
    pub fn store_transaction(
        &self,
        block_number: u64,
        transaction_hash: &str,
        log_index: u32,
        from_address: &str,
        to_address: &str,
        amount: &str,
        timestamp: u64,
        direction: &str,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Operation("Failed to acquire lock".to_string()))?;
        
        conn.execute(
            "INSERT INTO transactions (block_number, transaction_hash, log_index, from_address, to_address, amount, timestamp, direction)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![block_number, transaction_hash, log_index, from_address, to_address, amount, timestamp, direction],
        )?;
        
        Ok(())
    }

    /// Get a transaction by transaction hash and log index
    pub fn get_transaction(&self, transaction_hash: &str, log_index: u32) -> Result<TransactionRow, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Operation("Failed to acquire lock".to_string()))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, block_number, transaction_hash, log_index, from_address, to_address, amount, timestamp, direction, created_at
             FROM transactions WHERE transaction_hash = ?1 AND log_index = ?2"
        )?;
        
        let row = stmt.query_row(params![transaction_hash, log_index], |row| {
            Ok(TransactionRow {
                id: row.get(0)?,
                block_number: row.get(1)?,
                transaction_hash: row.get(2)?,
                log_index: row.get(3)?,
                from_address: row.get(4)?,
                to_address: row.get(5)?,
                amount: row.get(6)?,
                timestamp: row.get(7)?,
                direction: row.get(8)?,
                created_at: row.get(9)?,
            })
        }).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound,
            _ => DbError::Connection(e),
        })?;
        
        Ok(row)
    }

    /// Get transactions by block number
    pub fn get_transactions_by_block(&self, block_number: u64) -> Result<Vec<TransactionRow>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Operation("Failed to acquire lock".to_string()))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, block_number, transaction_hash, log_index, from_address, to_address, amount, timestamp, direction, created_at
             FROM transactions WHERE block_number = ?1 ORDER BY log_index"
        )?;
        
        let rows = stmt.query_map(params![block_number], |row| {
            Ok(TransactionRow {
                id: row.get(0)?,
                block_number: row.get(1)?,
                transaction_hash: row.get(2)?,
                log_index: row.get(3)?,
                from_address: row.get(4)?,
                to_address: row.get(5)?,
                amount: row.get(6)?,
                timestamp: row.get(7)?,
                direction: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        
        let mut transactions = Vec::new();
        for row in rows {
            transactions.push(row?);
        }
        
        Ok(transactions)
    }

    /// Update a transaction (for testing purposes)
    pub fn update_transaction_amount(&self, transaction_hash: &str, log_index: u32, new_amount: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Operation("Failed to acquire lock".to_string()))?;
        
        let rows_affected = conn.execute(
            "UPDATE transactions SET amount = ?1 WHERE transaction_hash = ?2 AND log_index = ?3",
            params![new_amount, transaction_hash, log_index],
        )?;
        
        if rows_affected == 0 {
            return Err(DbError::NotFound);
        }
        
        Ok(())
    }

    /// Delete a transaction
    pub fn delete_transaction(&self, transaction_hash: &str, log_index: u32) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Operation("Failed to acquire lock".to_string()))?;
        
        let rows_affected = conn.execute(
            "DELETE FROM transactions WHERE transaction_hash = ?1 AND log_index = ?2",
            params![transaction_hash, log_index],
        )?;
        
        if rows_affected == 0 {
            return Err(DbError::NotFound);
        }
        
        Ok(())
    }

    /// Get the last processed block number
    pub fn get_last_processed_block(&self) -> Result<u64, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Operation("Failed to acquire lock".to_string()))?;
        
        let block_number: u64 = conn.query_row(
            "SELECT last_processed_block FROM net_flows WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        
        Ok(block_number)
    }

    /// Set the last processed block number
    pub fn set_last_processed_block(&self, block_number: u64) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Operation("Failed to acquire lock".to_string()))?;
        
        conn.execute(
            "UPDATE net_flows SET last_processed_block = ?1, last_updated = strftime('%s', 'now') WHERE id = 1",
            params![block_number],
        )?;
        
        Ok(())
    }

    /// Get current net flow data
    pub fn get_net_flow_data(&self) -> Result<NetFlowRow, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Operation("Failed to acquire lock".to_string()))?;
        
        let row = conn.query_row(
            "SELECT id, total_inflow, total_outflow, net_flow, last_processed_block, last_updated
             FROM net_flows WHERE id = 1",
            [],
            |row| {
                Ok(NetFlowRow {
                    id: row.get(0)?,
                    total_inflow: row.get(1)?,
                    total_outflow: row.get(2)?,
                    net_flow: row.get(3)?,
                    last_processed_block: row.get(4)?,
                    last_updated: row.get(5)?,
                })
            },
        )?;
        
        Ok(row)
    }

    /// Get transaction count
    pub fn get_transaction_count(&self) -> Result<u64, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Operation("Failed to acquire lock".to_string()))?;
        
        let count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM transactions",
            [],
            |row| row.get(0),
        )?;
        
        Ok(count)
    }

    /// Update net-flow data atomically with a new inflow amount
    pub fn update_net_flow_inflow(&self, amount: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Operation("Failed to acquire lock".to_string()))?;
        
        let tx = conn.unchecked_transaction()?;
        
        // Get current values
        let current_inflow: String = tx.query_row(
            "SELECT total_inflow FROM net_flows WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        
        // Calculate new inflow using NetFlowCalculator
        let new_inflow = crate::models::NetFlowCalculator::add_inflow(&current_inflow, amount)
            .map_err(|e| DbError::Operation(format!("Failed to calculate new inflow: {}", e)))?;
        
        // Get current outflow to recalculate net flow
        let current_outflow: String = tx.query_row(
            "SELECT total_outflow FROM net_flows WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        
        // Calculate new net flow
        let new_net_flow = crate::models::NetFlowCalculator::calculate_net(&new_inflow, &current_outflow)
            .map_err(|e| DbError::Operation(format!("Failed to calculate net flow: {}", e)))?;
        
        // Update the net_flows table
        tx.execute(
            "UPDATE net_flows SET total_inflow = ?1, net_flow = ?2, last_updated = strftime('%s', 'now') WHERE id = 1",
            params![new_inflow, new_net_flow],
        )?;
        
        tx.commit()?;
        Ok(())
    }

    /// Update net-flow data atomically with a new outflow amount
    pub fn update_net_flow_outflow(&self, amount: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Operation("Failed to acquire lock".to_string()))?;
        
        let tx = conn.unchecked_transaction()?;
        
        // Get current values
        let current_outflow: String = tx.query_row(
            "SELECT total_outflow FROM net_flows WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        
        // Calculate new outflow using NetFlowCalculator
        let new_outflow = crate::models::NetFlowCalculator::add_outflow(&current_outflow, amount)
            .map_err(|e| DbError::Operation(format!("Failed to calculate new outflow: {}", e)))?;
        
        // Get current inflow to recalculate net flow
        let current_inflow: String = tx.query_row(
            "SELECT total_inflow FROM net_flows WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        
        // Calculate new net flow
        let new_net_flow = crate::models::NetFlowCalculator::calculate_net(&current_inflow, &new_outflow)
            .map_err(|e| DbError::Operation(format!("Failed to calculate net flow: {}", e)))?;
        
        // Update the net_flows table
        tx.execute(
            "UPDATE net_flows SET total_outflow = ?1, net_flow = ?2, last_updated = strftime('%s', 'now') WHERE id = 1",
            params![new_outflow, new_net_flow],
        )?;
        
        tx.commit()?;
        Ok(())
    }

    /// Update net-flow data atomically based on transfer direction
    pub fn update_net_flow_with_transfer(&self, amount: &str, direction: &crate::models::TransferDirection) -> Result<(), DbError> {
        match direction {
            crate::models::TransferDirection::ToBinance => self.update_net_flow_inflow(amount),
            crate::models::TransferDirection::FromBinance => self.update_net_flow_outflow(amount),
            crate::models::TransferDirection::NotRelevant => Ok(()), // No update needed for irrelevant transfers
        }
    }

    /// Store a processed transfer and update net-flow data atomically
    pub fn store_transfer_and_update_net_flow(&self, transfer: &crate::models::ProcessedTransfer) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Operation("Failed to acquire lock".to_string()))?;
        
        let tx = conn.unchecked_transaction()?;
        
        // Convert direction to string for database storage
        let direction_str = match transfer.direction {
            crate::models::TransferDirection::ToBinance => "inflow",
            crate::models::TransferDirection::FromBinance => "outflow",
            crate::models::TransferDirection::NotRelevant => return Ok(()), // Don't store irrelevant transfers
        };
        
        // Store the transaction
        tx.execute(
            "INSERT INTO transactions (block_number, transaction_hash, log_index, from_address, to_address, amount, timestamp, direction)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                transfer.block_number,
                transfer.transaction_hash,
                transfer.log_index,
                transfer.from_address,
                transfer.to_address,
                transfer.amount,
                transfer.timestamp,
                direction_str
            ],
        )?;
        
        // Update net-flow data based on direction
        match transfer.direction {
            crate::models::TransferDirection::ToBinance => {
                // Get current inflow
                let current_inflow: String = tx.query_row(
                    "SELECT total_inflow FROM net_flows WHERE id = 1",
                    [],
                    |row| row.get(0),
                )?;
                
                // Calculate new inflow
                let new_inflow = crate::models::NetFlowCalculator::add_inflow(&current_inflow, &transfer.amount)
                    .map_err(|e| DbError::Operation(format!("Failed to calculate new inflow: {}", e)))?;
                
                // Get current outflow to recalculate net flow
                let current_outflow: String = tx.query_row(
                    "SELECT total_outflow FROM net_flows WHERE id = 1",
                    [],
                    |row| row.get(0),
                )?;
                
                // Calculate new net flow
                let new_net_flow = crate::models::NetFlowCalculator::calculate_net(&new_inflow, &current_outflow)
                    .map_err(|e| DbError::Operation(format!("Failed to calculate net flow: {}", e)))?;
                
                // Update net flows
                tx.execute(
                    "UPDATE net_flows SET total_inflow = ?1, net_flow = ?2, last_updated = strftime('%s', 'now') WHERE id = 1",
                    params![new_inflow, new_net_flow],
                )?;
            },
            crate::models::TransferDirection::FromBinance => {
                // Get current outflow
                let current_outflow: String = tx.query_row(
                    "SELECT total_outflow FROM net_flows WHERE id = 1",
                    [],
                    |row| row.get(0),
                )?;
                
                // Calculate new outflow
                let new_outflow = crate::models::NetFlowCalculator::add_outflow(&current_outflow, &transfer.amount)
                    .map_err(|e| DbError::Operation(format!("Failed to calculate new outflow: {}", e)))?;
                
                // Get current inflow to recalculate net flow
                let current_inflow: String = tx.query_row(
                    "SELECT total_inflow FROM net_flows WHERE id = 1",
                    [],
                    |row| row.get(0),
                )?;
                
                // Calculate new net flow
                let new_net_flow = crate::models::NetFlowCalculator::calculate_net(&current_inflow, &new_outflow)
                    .map_err(|e| DbError::Operation(format!("Failed to calculate net flow: {}", e)))?;
                
                // Update net flows
                tx.execute(
                    "UPDATE net_flows SET total_outflow = ?1, net_flow = ?2, last_updated = strftime('%s', 'now') WHERE id = 1",
                    params![new_outflow, new_net_flow],
                )?;
            },
            crate::models::TransferDirection::NotRelevant => {
                // This case is already handled above, but included for completeness
            }
        }
        
        tx.commit()?;
        Ok(())
    }
}

/// Represents a row from the transactions table
#[derive(Debug, Clone)]
pub struct TransactionRow {
    pub id: i64,
    pub block_number: u64,
    pub transaction_hash: String,
    pub log_index: u32,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub timestamp: u64,
    pub direction: String,
    pub created_at: u64,
}

/// Represents a row from the net_flows table
#[derive(Debug, Clone)]
pub struct NetFlowRow {
    pub id: i64,
    pub total_inflow: String,
    pub total_outflow: String,
    pub net_flow: String,
    pub last_processed_block: u64,
    pub last_updated: u64,
}