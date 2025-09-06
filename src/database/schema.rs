use rusqlite::{Connection, Result};

/// Initialize the database schema with required tables
pub fn initialize_schema(conn: &Connection) -> Result<()> {
    // Create transactions table for raw transaction storage
    conn.execute(
        "CREATE TABLE IF NOT EXISTS transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            block_number INTEGER NOT NULL,
            transaction_hash TEXT NOT NULL,
            log_index INTEGER NOT NULL,
            from_address TEXT NOT NULL,
            to_address TEXT NOT NULL,
            amount TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            direction TEXT NOT NULL CHECK (direction IN ('inflow', 'outflow')),
            created_at INTEGER DEFAULT (strftime('%s', 'now')),
            UNIQUE(transaction_hash, log_index)
        )",
        [],
    )?;

    // Create net_flows table for cumulative net-flow tracking
    conn.execute(
        "CREATE TABLE IF NOT EXISTS net_flows (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            total_inflow TEXT NOT NULL DEFAULT '0',
            total_outflow TEXT NOT NULL DEFAULT '0',
            net_flow TEXT NOT NULL DEFAULT '0',
            last_processed_block INTEGER NOT NULL DEFAULT 0,
            last_updated INTEGER DEFAULT (strftime('%s', 'now'))
        )",
        [],
    )?;

    // Create indexes for performance
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_transactions_block ON transactions(block_number)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_transactions_direction ON transactions(direction)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_transactions_timestamp ON transactions(timestamp)",
        [],
    )?;

    // Initialize net_flows table with default values if empty
    conn.execute(
        "INSERT OR IGNORE INTO net_flows (id, total_inflow, total_outflow, net_flow, last_processed_block)
         VALUES (1, '0', '0', '0', 0)",
        [],
    )?;

    Ok(())
}

/// Run database migrations (for future schema updates)
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Check current schema version and apply migrations as needed
    // For now, just ensure the schema is initialized
    initialize_schema(conn)
}