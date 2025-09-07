#!/bin/bash

# Polygon POL Indexer Deployment Script
# This script handles deployment to various environments

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SERVICE_NAME="polygon-pol-indexer"
SERVICE_USER="indexer"
INSTALL_DIR="/opt/polygon-pol-indexer"
CONFIG_DIR="/etc/polygon-pol-indexer"
LOG_DIR="/var/log/polygon-pol-indexer"
DATA_DIR="/var/lib/polygon-pol-indexer"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS] ENVIRONMENT

Deploy Polygon POL Indexer to specified environment.

ENVIRONMENTS:
    development     Deploy for local development
    staging         Deploy to staging environment
    production      Deploy to production environment
    testnet         Deploy for testnet testing

OPTIONS:
    -h, --help      Show this help message
    -u, --user      Service user (default: $SERVICE_USER)
    -d, --dir       Installation directory (default: $INSTALL_DIR)
    --skip-build    Skip building the application
    --skip-service  Skip systemd service setup
    --dry-run       Show what would be done without executing

EXAMPLES:
    $0 production
    $0 staging --user myuser
    $0 development --skip-service
EOF
}

# Parse command line arguments
ENVIRONMENT=""
SKIP_BUILD=false
SKIP_SERVICE=false
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_usage
            exit 0
            ;;
        -u|--user)
            SERVICE_USER="$2"
            shift 2
            ;;
        -d|--dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --skip-service)
            SKIP_SERVICE=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        development|staging|production|testnet)
            ENVIRONMENT="$1"
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Validate environment
if [[ -z "$ENVIRONMENT" ]]; then
    log_error "Environment must be specified"
    show_usage
    exit 1
fi

# Function to execute commands (respects dry-run)
execute() {
    if [[ "$DRY_RUN" == "true" ]]; then
        echo "[DRY-RUN] $*"
    else
        "$@"
    fi
}

# Function to check if running as root
check_root() {
    if [[ $EUID -ne 0 ]] && [[ "$DRY_RUN" != "true" ]]; then
        log_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

# Function to create system user
create_user() {
    log_info "Creating system user: $SERVICE_USER"
    
    if id "$SERVICE_USER" &>/dev/null; then
        log_warn "User $SERVICE_USER already exists"
    else
        execute useradd -r -s /bin/false -m -d "$DATA_DIR" "$SERVICE_USER"
        log_info "Created user: $SERVICE_USER"
    fi
}

# Function to create directories
create_directories() {
    log_info "Creating directories..."
    
    execute mkdir -p "$INSTALL_DIR"
    execute mkdir -p "$CONFIG_DIR"
    execute mkdir -p "$LOG_DIR"
    execute mkdir -p "$DATA_DIR"
    
    execute chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"
    execute chown -R "$SERVICE_USER:$SERVICE_USER" "$LOG_DIR"
    execute chmod 755 "$INSTALL_DIR"
    execute chmod 750 "$CONFIG_DIR"
    execute chmod 750 "$LOG_DIR"
    execute chmod 750 "$DATA_DIR"
}

# Function to build the application
build_application() {
    if [[ "$SKIP_BUILD" == "true" ]]; then
        log_info "Skipping build (--skip-build specified)"
        return
    fi
    
    log_info "Building application..."
    cd "$PROJECT_ROOT"
    
    execute cargo build --release
    log_info "Build completed successfully"
}

# Function to install binaries
install_binaries() {
    log_info "Installing binaries..."
    
    execute cp "$PROJECT_ROOT/target/release/indexer" "$INSTALL_DIR/"
    execute cp "$PROJECT_ROOT/target/release/cli" "$INSTALL_DIR/"
    execute cp "$PROJECT_ROOT/target/release/server" "$INSTALL_DIR/"
    
    execute chmod +x "$INSTALL_DIR/indexer"
    execute chmod +x "$INSTALL_DIR/cli"
    execute chmod +x "$INSTALL_DIR/server"
    
    execute chown root:root "$INSTALL_DIR"/*
}

# Function to install configuration
install_configuration() {
    log_info "Installing configuration for environment: $ENVIRONMENT"
    
    local config_file="$PROJECT_ROOT/config/${ENVIRONMENT}.toml"
    
    if [[ ! -f "$config_file" ]]; then
        log_warn "Environment config not found: $config_file"
        log_info "Using example configuration"
        config_file="$PROJECT_ROOT/config.example.toml"
    fi
    
    execute cp "$config_file" "$CONFIG_DIR/config.toml"
    execute chown root:root "$CONFIG_DIR/config.toml"
    execute chmod 644 "$CONFIG_DIR/config.toml"
    
    # Update paths in configuration for system installation
    if [[ "$DRY_RUN" != "true" ]]; then
        sed -i "s|path = \".*blockchain\.db\"|path = \"$DATA_DIR/blockchain.db\"|g" "$CONFIG_DIR/config.toml"
        sed -i "s|file_path = \".*indexer\.log\"|file_path = \"$LOG_DIR/indexer.log\"|g" "$CONFIG_DIR/config.toml"
    fi
}

# Function to install systemd service
install_systemd_service() {
    if [[ "$SKIP_SERVICE" == "true" ]]; then
        log_info "Skipping systemd service setup (--skip-service specified)"
        return
    fi
    
    log_info "Installing systemd service..."
    
    # Generate service file
    cat > /tmp/polygon-pol-indexer.service << EOF
[Unit]
Description=Polygon POL Token Indexer
Documentation=https://github.com/your-org/polygon-pol-indexer
After=network.target
Wants=network.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$DATA_DIR
ExecStart=$INSTALL_DIR/indexer --config $CONFIG_DIR/config.toml
ExecReload=/bin/kill -HUP \$MAINPID
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=polygon-pol-indexer

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$DATA_DIR $LOG_DIR
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Environment
Environment=RUST_LOG=info
Environment=CONFIG_PATH=$CONFIG_DIR/config.toml

[Install]
WantedBy=multi-user.target
EOF
    
    execute cp /tmp/polygon-pol-indexer.service /etc/systemd/system/
    execute rm /tmp/polygon-pol-indexer.service
    execute chmod 644 /etc/systemd/system/polygon-pol-indexer.service
    
    execute systemctl daemon-reload
    log_info "Systemd service installed"
}

# Function to start and enable service
start_service() {
    if [[ "$SKIP_SERVICE" == "true" ]]; then
        return
    fi
    
    log_info "Starting and enabling service..."
    
    execute systemctl enable polygon-pol-indexer
    execute systemctl start polygon-pol-indexer
    
    sleep 2
    
    if [[ "$DRY_RUN" != "true" ]]; then
        if systemctl is-active --quiet polygon-pol-indexer; then
            log_info "Service started successfully"
        else
            log_error "Service failed to start"
            systemctl status polygon-pol-indexer
            exit 1
        fi
    fi
}

# Function to show post-deployment information
show_post_deployment_info() {
    log_info "Deployment completed successfully!"
    
    cat << EOF

Post-deployment information:
============================

Service Status:
    sudo systemctl status $SERVICE_NAME
    sudo systemctl start|stop|restart $SERVICE_NAME

Logs:
    sudo journalctl -u $SERVICE_NAME -f
    sudo tail -f $LOG_DIR/indexer.log

Configuration:
    $CONFIG_DIR/config.toml

Data Directory:
    $DATA_DIR

CLI Usage:
    $INSTALL_DIR/cli net-flow
    $INSTALL_DIR/cli status

Health Check:
    curl http://localhost:8080/status

EOF
}

# Main deployment function
main() {
    log_info "Starting deployment for environment: $ENVIRONMENT"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_warn "DRY RUN MODE - No changes will be made"
    fi
    
    check_root
    create_user
    create_directories
    build_application
    install_binaries
    install_configuration
    install_systemd_service
    start_service
    show_post_deployment_info
    
    log_info "Deployment completed successfully!"
}

# Run main function
main