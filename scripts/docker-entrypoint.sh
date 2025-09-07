#!/bin/bash
set -e

# Function to log messages
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $1"
}

# Function to check if RPC endpoint is reachable
check_rpc_endpoint() {
    local endpoint="${POLYGON_RPC_URL:-https://polygon-rpc.com/}"
    log "Checking RPC endpoint: $endpoint"
    
    if curl -s --max-time 10 "$endpoint" > /dev/null; then
        log "RPC endpoint is reachable"
        return 0
    else
        log "WARNING: RPC endpoint is not reachable"
        return 1
    fi
}

# Function to initialize database directory
init_database() {
    local db_path="${DATABASE_PATH:-/app/data/blockchain.db}"
    local db_dir=$(dirname "$db_path")
    
    log "Initializing database directory: $db_dir"
    mkdir -p "$db_dir"
    
    # Ensure proper permissions
    if [ "$(id -u)" = "0" ]; then
        chown -R indexer:indexer "$db_dir"
    fi
}

# Function to validate configuration
validate_config() {
    log "Validating configuration..."
    
    if [ ! -f "/app/config.toml" ]; then
        log "WARNING: config.toml not found, using default configuration"
        cp /app/config.example.toml /app/config.toml
    fi
    
    # Check required environment variables
    if [ -z "$POLYGON_RPC_URL" ]; then
        log "WARNING: POLYGON_RPC_URL not set, using default"
    fi
}

# Function to wait for dependencies
wait_for_dependencies() {
    log "Waiting for dependencies..."
    
    # Wait for RPC endpoint (with retries)
    local retries=5
    local count=0
    
    while [ $count -lt $retries ]; do
        if check_rpc_endpoint; then
            break
        fi
        
        count=$((count + 1))
        if [ $count -lt $retries ]; then
            log "Retrying in 10 seconds... ($count/$retries)"
            sleep 10
        else
            log "WARNING: Could not reach RPC endpoint after $retries attempts"
        fi
    done
}

# Main execution
main() {
    log "Starting Polygon POL Indexer..."
    log "Command: $*"
    
    # Initialize
    validate_config
    init_database
    wait_for_dependencies
    
    # Handle different commands
    case "$1" in
        "indexer")
            log "Starting indexer service..."
            exec /usr/local/bin/indexer
            ;;
        "cli")
            log "Starting CLI interface..."
            shift
            exec /usr/local/bin/cli "$@"
            ;;
        "server")
            log "Starting HTTP server..."
            exec /usr/local/bin/server
            ;;
        "bash"|"sh")
            log "Starting shell..."
            exec "$@"
            ;;
        *)
            log "Starting custom command: $*"
            exec "$@"
            ;;
    esac
}

# Trap signals for graceful shutdown
trap 'log "Received shutdown signal, exiting..."; exit 0' SIGTERM SIGINT

# Run main function
main "$@"