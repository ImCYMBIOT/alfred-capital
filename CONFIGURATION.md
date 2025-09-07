# Configuration Management

The Polygon POL Token Indexer uses a comprehensive configuration system that supports both file-based configuration and environment variable overrides.

## Configuration Sources

The configuration system loads settings in the following priority order:

1. **Environment Variables** (highest priority)
2. **Configuration File** (TOML format)
3. **Default Values** (lowest priority)

## Configuration File

The application looks for a configuration file in the following order:

1. Path specified in `CONFIG_FILE` environment variable
2. `config.toml` in the current directory
3. If no file is found, default values are used

### Example Configuration File

Copy `config.example.toml` to `config.toml` and modify as needed:

```toml
[rpc]
endpoint = "https://polygon-rpc.com/"
timeout_seconds = 30
max_retries = 5
retry_delay_seconds = 2
max_retry_delay_seconds = 60

[database]
path = "./blockchain.db"
connection_pool_size = 10
enable_wal_mode = true
busy_timeout_ms = 5000

[processing]
poll_interval_seconds = 2
batch_size = 100
pol_token_address = "0x455e53bd25bfb4ed405b8b8c2db7ab87cd0a7e9f"
max_blocks_per_batch = 10

[api]
enabled = true
port = 8080
host = "127.0.0.1"
request_timeout_seconds = 30
max_connections = 100

[logging]
level = "info"
format = "pretty"
file_enabled = false
max_file_size_mb = 100
max_files = 5
```

## Environment Variables

All configuration values can be overridden using environment variables:

### RPC Configuration

- `POLYGON_RPC_URL` - Polygon RPC endpoint URL
- `RPC_TIMEOUT_SECONDS` - Request timeout in seconds
- `RPC_MAX_RETRIES` - Maximum number of retry attempts

### Database Configuration

- `DATABASE_PATH` - SQLite database file path
- `DATABASE_POOL_SIZE` - Connection pool size
- `DATABASE_WAL_MODE` - Enable WAL mode (true/false)

### Processing Configuration

- `BLOCK_POLL_INTERVAL` - Block polling interval in seconds
- `PROCESSING_BATCH_SIZE` - Batch size for processing multiple blocks
- `POL_TOKEN_ADDRESS` - POL token contract address on Polygon

### API Configuration

- `API_ENABLED` - Enable HTTP API server (true/false)
- `API_PORT` - Server port
- `API_HOST` - Server host/bind address

### Logging Configuration

- `LOG_LEVEL` - Log level (error, warn, info, debug, trace)
- `LOG_FORMAT` - Log format (json, pretty)
- `LOG_FILE_ENABLED` - Enable file logging (true/false)
- `LOG_FILE_PATH` - Log file path (if file logging enabled)

## Configuration Validation

The configuration system validates all values to ensure they are within acceptable ranges:

- **RPC timeout**: 1-300 seconds
- **Poll interval**: 1-300 seconds
- **Batch size**: 1-1000 blocks
- **POL token address**: Must be a valid 42-character hex address
- **Log level**: Must be one of: error, warn, info, debug, trace
- **Log format**: Must be one of: json, pretty

## Usage Examples

### Using Environment Variables

```bash
export POLYGON_RPC_URL="https://polygon-mainnet.infura.io/v3/YOUR_KEY"
export DATABASE_PATH="/data/blockchain.db"
export BLOCK_POLL_INTERVAL=5
export LOG_LEVEL=debug

./target/release/indexer
```

### Using Configuration File

```bash
# Create config.toml with your settings
cp config.example.toml config.toml
# Edit config.toml as needed
./target/release/indexer
```

### Using Custom Configuration File

```bash
export CONFIG_FILE="/path/to/my-config.toml"
./target/release/indexer
```

## Configuration Structure

The configuration is organized into logical sections:

### RpcConfig

- Connection settings for Polygon RPC
- Retry and timeout configuration
- Error handling parameters

### DatabaseConfig

- SQLite database settings
- Connection pool configuration
- Performance tuning options

### ProcessingConfig

- Block processing parameters
- Token contract addresses
- Batch processing settings

### ApiConfig

- HTTP API server settings
- Connection limits
- Request handling configuration

### LoggingConfig

- Logging level and format
- File logging options
- Log rotation settings

## Default Values

If no configuration is provided, the system uses these defaults:

- **RPC Endpoint**: `https://polygon-rpc.com/`
- **Database Path**: `./blockchain.db`
- **Poll Interval**: 2 seconds
- **API Port**: 8080
- **Log Level**: info
- **Log Format**: pretty

## Error Handling

Configuration errors are reported with detailed messages:

- Invalid values show the expected range
- Missing files are handled gracefully with defaults
- Environment variable parsing errors include the variable name and invalid value

## Best Practices

1. **Use environment variables for secrets** (API keys, sensitive URLs)
2. **Use configuration files for deployment-specific settings**
3. **Keep default values for development environments**
4. **Validate configuration in staging before production deployment**
5. **Use appropriate log levels for different environments**

## Troubleshooting

### Common Issues

1. **Invalid URL format**: Ensure RPC URLs start with `http://` or `https://`
2. **Invalid token address**: Must be 42 characters starting with `0x`
3. **Port conflicts**: Ensure the API port is not already in use
4. **File permissions**: Ensure the application can read config files and write to database path

### Debug Configuration

To see the loaded configuration, run with debug logging:

```bash
LOG_LEVEL=debug ./target/release/indexer
```

This will show all configuration values loaded from files and environment variables.
