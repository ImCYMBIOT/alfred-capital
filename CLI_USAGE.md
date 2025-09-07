# CLI Usage Guide

The Polygon POL Indexer includes a command-line interface for querying net-flow data and system status.

## Building the CLI

```bash
cargo build --bin cli
```

## Usage

The CLI provides three main commands:

### 1. Display Current Net-Flow

Shows the current cumulative net-flow of POL tokens to/from Binance:

```bash
cargo run --bin cli -- net-flow
```

Example output:

```
=== POL Token Net-Flow Data ===
Total Inflow:  3500.5 POL
Total Outflow: 500.25 POL
Net Flow:      3000.25 POL
Last Updated:  SystemTime { tv_sec: 1640995320, tv_nsec: 0 }
```

### 2. Show System Status

Displays system status and last processed block information:

```bash
cargo run --bin cli -- status
```

Example output:

```
=== System Status ===
Last Processed Block: 102
Total Transactions:   3
Last Updated:         SystemTime { tv_sec: 1640995320, tv_nsec: 0 }
Database Status:      Connected
```

### 3. Display Recent Transactions

Shows recent transactions with pagination support:

```bash
# Show 10 most recent transactions (default)
cargo run --bin cli -- transactions

# Show 5 most recent transactions
cargo run --bin cli -- transactions --limit 5

# Show transactions with offset (skip first 10)
cargo run --bin cli -- transactions --limit 5 --offset 10
```

Example output:

```
=== Recent Transactions ===
Showing 2 transactions (offset: 0, total: 3)

Transaction #1
  Block:     102
  Hash:      0xabcdef1234567890
  Log Index: 0
  From:      0xsender2
  To:        0x505e71695e9bc45943c58adec1650577bca68fd9
  Amount:    2500.0 POL
  Direction: inflow
  Timestamp: SystemTime { tv_sec: 1640995320, tv_nsec: 0 }
  Created:   SystemTime { tv_sec: 1640995320, tv_nsec: 0 }

Transaction #2
  Block:     101
  Hash:      0xfedcba0987654321
  Log Index: 1
  From:      0xe7804c37c13166ff0b37f5ae0bb07a3aebb6e245
  To:        0xreceiver1
  Amount:    500.25 POL
  Direction: outflow
  Timestamp: SystemTime { tv_sec: 1640995260, tv_nsec: 0 }
  Created:   SystemTime { tv_sec: 1640995260, tv_nsec: 0 }

Use --offset 2 to see more transactions
```

## Database Configuration

By default, the CLI looks for the database at `./blockchain.db`. You can specify a different path:

```bash
# Using command line option
cargo run --bin cli -- --database /path/to/custom.db net-flow

# Using environment variable
DATABASE_PATH=/path/to/custom.db cargo run --bin cli -- net-flow
```

## Error Handling

The CLI provides clear error messages for common issues:

- **Database not found**: Make sure the indexer has been run at least once to create the database
- **Invalid parameters**: Check that limit values are between 1 and 1000
- **Connection issues**: Verify the database path is correct and accessible

## Integration with Scripts

The CLI is designed to be script-friendly and returns appropriate exit codes:

- `0`: Success
- `1`: Error occurred

Example bash script:

```bash
#!/bin/bash
if cargo run --bin cli -- status > /dev/null 2>&1; then
    echo "Indexer is running normally"
    cargo run --bin cli -- net-flow
else
    echo "Error accessing indexer database"
    exit 1
fi
```
