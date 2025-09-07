#!/bin/bash

# Quick installation script for Polygon POL Indexer
# This script provides a simple way to install the indexer

set -e

# Configuration
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="$HOME/.config/polygon-pol-indexer"
DATA_DIR="$HOME/.local/share/polygon-pol-indexer"
LOG_DIR="$HOME/.local/share/polygon-pol-indexer/logs"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if Rust is installed
check_rust() {
    if ! command -v cargo &> /dev/null; then
        log_error "Rust is not installed. Please install Rust first:"
        echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    log_info "Rust found: $(rustc --version)"
}

# Create directories
create_directories() {
    log_info "Creating directories..."
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$DATA_DIR"
    mkdir -p "$LOG_DIR"
}

# Build the application
build_application() {
    log_info "Building application..."
    cargo build --release
}

# Install binaries
install_binaries() {
    log_info "Installing binaries to $INSTALL_DIR..."
    
    # Check if we can write to /usr/local/bin
    if [[ -w "$INSTALL_DIR" ]]; then
        cp target/release/indexer "$INSTALL_DIR/"
        cp target/release/cli "$INSTALL_DIR/"
        cp target/release/server "$INSTALL_DIR/"
    else
        log_warn "Cannot write to $INSTALL_DIR, using sudo..."
        sudo cp target/release/indexer "$INSTALL_DIR/"
        sudo cp target/release/cli "$INSTALL_DIR/"
        sudo cp target/release/server "$INSTALL_DIR/"
    fi
    
    log_info "Binaries installed successfully"
}

# Install configuration
install_configuration() {
    log_info "Installing configuration..."
    
    if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
        cp config.example.toml "$CONFIG_DIR/config.toml"
        
        # Update paths for user installation
        sed -i.bak "s|path = \".*blockchain\.db\"|path = \"$DATA_DIR/blockchain.db\"|g" "$CONFIG_DIR/config.toml"
        sed -i.bak "s|file_path = \".*indexer\.log\"|file_path = \"$LOG_DIR/indexer.log\"|g" "$CONFIG_DIR/config.toml"
        rm "$CONFIG_DIR/config.toml.bak" 2>/dev/null || true
        
        log_info "Configuration installed at $CONFIG_DIR/config.toml"
    else
        log_warn "Configuration already exists at $CONFIG_DIR/config.toml"
    fi
}

# Create shell aliases
create_aliases() {
    log_info "Creating shell aliases..."
    
    local shell_rc=""
    if [[ -n "$BASH_VERSION" ]]; then
        shell_rc="$HOME/.bashrc"
    elif [[ -n "$ZSH_VERSION" ]]; then
        shell_rc="$HOME/.zshrc"
    else
        shell_rc="$HOME/.profile"
    fi
    
    if [[ -f "$shell_rc" ]]; then
        if ! grep -q "polygon-pol-indexer aliases" "$shell_rc"; then
            cat >> "$shell_rc" << EOF

# Polygon POL Indexer aliases
alias pol-indexer='indexer --config $CONFIG_DIR/config.toml'
alias pol-cli='cli --config $CONFIG_DIR/config.toml'
alias pol-server='server --config $CONFIG_DIR/config.toml'
EOF
            log_info "Aliases added to $shell_rc"
        else
            log_warn "Aliases already exist in $shell_rc"
        fi
    fi
}

# Show post-installation information
show_post_install_info() {
    log_info "Installation completed successfully!"
    
    cat << EOF

Post-installation information:
==============================

Configuration:
    Edit: $CONFIG_DIR/config.toml
    
Data Directory:
    $DATA_DIR
    
Log Directory:
    $LOG_DIR

Usage:
    # Start the indexer
    indexer --config $CONFIG_DIR/config.toml
    
    # Query net-flow
    cli --config $CONFIG_DIR/config.toml net-flow
    
    # Start HTTP server
    server --config $CONFIG_DIR/config.toml

    # Or use aliases (after restarting shell):
    pol-indexer
    pol-cli net-flow
    pol-server

Next Steps:
1. Edit the configuration file: $CONFIG_DIR/config.toml
2. Set your Polygon RPC endpoint
3. Start the indexer: pol-indexer
4. Query data: pol-cli net-flow

EOF
}

# Main installation function
main() {
    log_info "Starting Polygon POL Indexer installation..."
    
    check_rust
    create_directories
    build_application
    install_binaries
    install_configuration
    create_aliases
    show_post_install_info
    
    log_info "Installation completed!"
}

# Run main function
main