# Polygon POL Token Indexer

A real-time blockchain data indexing system that monitors the Polygon network for POL token transfers involving Binance exchange addresses. The system calculates and maintains cumulative net-flows (inflows minus outflows) and provides query interfaces for accessing this data.

## Features

- **Real-time Processing**: Monitors new blocks as they are mined on Polygon
- **POL Token Focus**: Specifically tracks POL token transfers using ERC-20 event logs
- **Binance Integration**: Identifies transfers to/from known Binance addresses
- **Net-flow Calculation**: Maintains cumulative inflow/outflow statistics
- **Multiple Interfaces**: CLI tool and HTTP API for data access
- **Fault Tolerant**: Handles network failures and RPC errors gracefully
- **Scalable Design**: Modular architecture for easy extension to other exchanges

## Quick Start

### Using Docker (Recommended)

1. **Clone the repository**

   ```bash
   git clone <repository-url>
   cd polygon-pol-indexer
   ```

2. **Configure the application**

   ```bash
   cp config.example.toml config.toml
   # Edit config.toml with your settings
   ```

3. **Start with Docker Compose**

   ```bash
   docker-compose up -d
   ```

4. **Check status**
   ```bash
   curl http://localhost:8080/status
   ```

### Manual Installation

#### Prerequisites

- Rust 1.75+ ([Install Rust](https://rustup.rs/))
- SQLite 3.x
- Internet connection for Polygon RPC access

#### Build and Run

1. **Clone and build**

   ```bash
   git clone <repository-url>
   cd polygon-pol-indexer
   cargo build --release
   ```

2. **Configure**

   ```bash
   cp config.example.toml config.toml
   # Edit config.toml as needed
   ```

3. **Run the indexer**
   ```bash
   ./target/release/indexer
   ```

## Configuration

### Configuration File

The application uses a TOML configuration file. Copy `config.example.toml` to `config.toml` and modify:

```toml
[rpc]
endpoint = "https://polygon-rpc.com/"
timeout_seconds = 30
max_retries = 5

[database]
path = "./blockchain.db"
connection_pool_size = 10

[processing]
poll_interval_seconds = 2
pol_token_address = "0x455e53bd25bfb4ed405b8b8c2db7ab87cd0a7e9f"

[api]
enabled = true
port = 8080
host = "127.0.0.1"
```

### Environment Variables

You can override configuration with environment variables:

- `POLYGON_RPC_URL`: Polygon RPC endpoint
- `DATABASE_PATH`: SQLite database file path
- `API_PORT`: HTTP API port
- `API_HOST`: HTTP API bind address
- `RUST_LOG`: Logging level (error, warn, info, debug, trace)

## Usage

### Running the Indexer

The main indexer process monitors the blockchain and processes transactions:

```bash
# Using binary
./target/release/indexer

# Using Docker
docker-compose up indexer
```

### CLI Interface

Query data using the command-line interface:

```bash
# Get current net-flow
./target/release/cli net-flow

# Get system status
./target/release/cli status

# Get recent transactions
./target/release/cli transactions --limit 50

# Using Docker
docker-compose exec indexer cli net-flow
```

### HTTP API

The HTTP server provides REST endpoints:

```bash
# Start the server
./target/release/server

# Query endpoints
curl http://localhost:8080/net-flow
curl http://localhost:8080/status
curl http://localhost:8080/transactions?limit=100
```

#### API Endpoints

- `GET /net-flow` - Current cumulative net-flow data
- `GET /status` - System status and health information
- `GET /transactions` - Recent transactions (supports `?limit=N`)

## Architecture

### System Components

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   RPC Client    │────│  Block Processor │────│    Database     │
│                 │    │                  │    │                 │
│ - Polygon RPC   │    │ - Filter POL     │    │ - SQLite        │
│ - Block fetching│    │ - Detect Binance │    │ - Transactions  │
│ - Error handling│    │ - Calculate flow │    │ - Net-flows     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌──────────────────┐
                    │  Query Interface │
                    │                  │
                    │ - CLI Tool       │
                    │ - HTTP API       │
                    │ - Net-flow query │
                    └──────────────────┘
```

### Data Flow

1. **Block Monitoring**: RPC client polls for new blocks every 2 seconds
2. **Transaction Processing**: Extract and decode POL token transfer events
3. **Address Classification**: Identify transfers involving Binance addresses
4. **Flow Calculation**: Categorize as inflow (to Binance) or outflow (from Binance)
5. **Data Storage**: Store raw transaction data and update cumulative net-flows
6. **Query Serving**: Provide current net-flow data through CLI or API

## Database Schema

The system uses SQLite with the following schema:

```sql
-- Raw transaction storage
CREATE TABLE transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_number INTEGER NOT NULL,
    transaction_hash TEXT NOT NULL UNIQUE,
    log_index INTEGER NOT NULL,
    from_address TEXT NOT NULL,
    to_address TEXT NOT NULL,
    amount TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('inflow', 'outflow')),
    created_at INTEGER DEFAULT (strftime('%s', 'now'))
);

-- Cumulative net-flow tracking
CREATE TABLE net_flows (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    total_inflow TEXT NOT NULL DEFAULT '0',
    total_outflow TEXT NOT NULL DEFAULT '0',
    net_flow TEXT NOT NULL DEFAULT '0',
    last_processed_block INTEGER NOT NULL DEFAULT 0,
    last_updated INTEGER DEFAULT (strftime('%s', 'now'))
);
```

## Development

### Building from Source

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Testing

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# End-to-end tests
cargo test --test end_to_end_workflow

# Performance tests
cargo test --test performance_tests
```

### Code Structure

```
src/
├── main.rs              # Main indexer binary
├── lib.rs               # Library root
├── config.rs            # Configuration management
├── error.rs             # Error types
├── api/                 # Query interfaces
│   ├── cli.rs           # Command-line interface
│   └── http.rs          # HTTP API server
├── blockchain/          # Blockchain interaction
│   ├── rpc_client.rs    # RPC client
│   └── block_processor.rs # Block processing logic
├── database/            # Data persistence
│   ├── operations.rs    # Database operations
│   └── schema.rs        # Schema definitions
└── models/              # Data models
    ├── transaction.rs   # Transaction models
    └── net_flow.rs      # Net-flow models
```

## Deployment

### Docker Deployment

1. **Build the image**

   ```bash
   docker build -t polygon-pol-indexer .
   ```

2. **Run with Docker Compose**

   ```bash
   docker-compose up -d
   ```

3. **Monitor logs**
   ```bash
   docker-compose logs -f indexer
   ```

### Systemd Service

For Linux systems, use the provided systemd service file:

```bash
# Copy service file
sudo cp scripts/polygon-pol-indexer.service /etc/systemd/system/

# Enable and start
sudo systemctl enable polygon-pol-indexer
sudo systemctl start polygon-pol-indexer

# Check status
sudo systemctl status polygon-pol-indexer
```

### Production Considerations

- **RPC Endpoint**: Use a reliable Polygon RPC provider (Alchemy, Infura, etc.)
- **Database Backup**: Regular SQLite database backups
- **Monitoring**: Set up health checks and alerting
- **Logging**: Configure log rotation and aggregation
- **Security**: Run with minimal privileges, secure API endpoints

## Monitoring and Maintenance

### Health Checks

The system provides health check endpoints:

```bash
# HTTP health check
curl http://localhost:8080/status

# Docker health check (automatic)
docker-compose ps
```

### Logs

Monitor application logs for issues:

```bash
# Docker logs
docker-compose logs -f indexer

# Systemd logs
sudo journalctl -u polygon-pol-indexer -f

# File logs (if configured)
tail -f logs/indexer.log
```

### Database Maintenance

```bash
# Backup database
cp blockchain.db blockchain.db.backup

# Check database integrity
sqlite3 blockchain.db "PRAGMA integrity_check;"

# Vacuum database (reclaim space)
sqlite3 blockchain.db "VACUUM;"
```

## Troubleshooting

### Common Issues

1. **RPC Connection Errors**

   - Check network connectivity
   - Verify RPC endpoint URL
   - Check rate limits

2. **Database Lock Errors**

   - Ensure only one instance is running
   - Check file permissions
   - Consider WAL mode for concurrency

3. **High Memory Usage**

   - Adjust batch sizes in configuration
   - Monitor block processing rate
   - Check for memory leaks in logs

4. **Missing Transactions**
   - Verify POL token contract address
   - Check Binance address list
   - Review block processing logs

### Getting Help

- Check the logs for error messages
- Review configuration settings
- Verify network connectivity
- Check system resources (CPU, memory, disk)

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Run the test suite
6. Submit a pull request

## License

[Add your license information here]

## Changelog

### v0.1.0

- Initial release
- Real-time POL token indexing
- Binance address detection
- CLI and HTTP API interfaces
- Docker support
