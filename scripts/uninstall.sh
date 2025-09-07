#!/bin/bash

# Uninstall script for Polygon POL Indexer

set -e

# Configuration
SERVICE_NAME="polygon-pol-indexer"
INSTALL_DIR="/opt/polygon-pol-indexer"
USER_INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/polygon-pol-indexer"
USER_CONFIG_DIR="$HOME/.config/polygon-pol-indexer"
LOG_DIR="/var/log/polygon-pol-indexer"
USER_LOG_DIR="$HOME/.local/share/polygon-pol-indexer/logs"
DATA_DIR="/var/lib/polygon-pol-indexer"
USER_DATA_DIR="$HOME/.local/share/polygon-pol-indexer"
SERVICE_USER="indexer"

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

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Uninstall Polygon POL Indexer from the system.

OPTIONS:
    -h, --help          Show this help message
    --system            Uninstall system-wide installation (requires sudo)
    --user              Uninstall user installation
    --keep-data         Keep data and configuration files
    --keep-config       Keep configuration files only
    --dry-run           Show what would be removed without doing it

EXAMPLES:
    $0 --system        # Remove system installation
    $0 --user          # Remove user installation
    $0 --system --keep-data  # Remove system installation but keep data
EOF
}

# Parse command line arguments
SYSTEM_INSTALL=false
USER_INSTALL=false
KEEP_DATA=false
KEEP_CONFIG=false
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_usage
            exit 0
            ;;
        --system)
            SYSTEM_INSTALL=true
            shift
            ;;
        --user)
            USER_INSTALL=true
            shift
            ;;
        --keep-data)
            KEEP_DATA=true
            shift
            ;;
        --keep-config)
            KEEP_CONFIG=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Auto-detect installation type if not specified
if [[ "$SYSTEM_INSTALL" == "false" && "$USER_INSTALL" == "false" ]]; then
    if [[ -d "$INSTALL_DIR" ]] || systemctl list-unit-files | grep -q "$SERVICE_NAME"; then
        SYSTEM_INSTALL=true
        log_info "Detected system installation"
    elif [[ -f "$USER_INSTALL_DIR/indexer" ]] || [[ -d "$USER_CONFIG_DIR" ]]; then
        USER_INSTALL=true
        log_info "Detected user installation"
    else
        log_error "No installation detected. Use --system or --user to force."
        exit 1
    fi
fi

# Function to execute commands (respects dry-run)
execute() {
    if [[ "$DRY_RUN" == "true" ]]; then
        echo "[DRY-RUN] $*"
    else
        "$@"
    fi
}

# Function to remove file/directory if exists
remove_if_exists() {
    local path="$1"
    if [[ -e "$path" ]]; then
        log_info "Removing: $path"
        execute rm -rf "$path"
    fi
}

# Function to stop and disable systemd service
stop_systemd_service() {
    log_info "Stopping and disabling systemd service..."
    
    if systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
        execute systemctl stop "$SERVICE_NAME"
        log_info "Service stopped"
    fi
    
    if systemctl is-enabled --quiet "$SERVICE_NAME" 2>/dev/null; then
        execute systemctl disable "$SERVICE_NAME"
        log_info "Service disabled"
    fi
    
    remove_if_exists "/etc/systemd/system/$SERVICE_NAME.service"
    execute systemctl daemon-reload
}

# Function to remove system user
remove_system_user() {
    if id "$SERVICE_USER" &>/dev/null; then
        log_info "Removing system user: $SERVICE_USER"
        execute userdel "$SERVICE_USER" 2>/dev/null || log_warn "Could not remove user $SERVICE_USER"
    fi
}

# Function to remove system installation
remove_system_installation() {
    log_info "Removing system installation..."
    
    # Check if running as root
    if [[ $EUID -ne 0 ]] && [[ "$DRY_RUN" != "true" ]]; then
        log_error "System uninstall requires root privileges (use sudo)"
        exit 1
    fi
    
    # Stop service
    stop_systemd_service
    
    # Remove binaries
    remove_if_exists "$INSTALL_DIR"
    
    # Remove configuration (unless keeping)
    if [[ "$KEEP_CONFIG" == "false" ]]; then
        remove_if_exists "$CONFIG_DIR"
    else
        log_info "Keeping configuration: $CONFIG_DIR"
    fi
    
    # Remove data (unless keeping)
    if [[ "$KEEP_DATA" == "false" ]]; then
        remove_if_exists "$DATA_DIR"
        remove_if_exists "$LOG_DIR"
    else
        log_info "Keeping data: $DATA_DIR"
        log_info "Keeping logs: $LOG_DIR"
    fi
    
    # Remove system user
    remove_system_user
}

# Function to remove user installation
remove_user_installation() {
    log_info "Removing user installation..."
    
    # Remove binaries from user install directory
    remove_if_exists "$USER_INSTALL_DIR/indexer"
    remove_if_exists "$USER_INSTALL_DIR/cli"
    remove_if_exists "$USER_INSTALL_DIR/server"
    
    # Remove configuration (unless keeping)
    if [[ "$KEEP_CONFIG" == "false" ]]; then
        remove_if_exists "$USER_CONFIG_DIR"
    else
        log_info "Keeping configuration: $USER_CONFIG_DIR"
    fi
    
    # Remove data (unless keeping)
    if [[ "$KEEP_DATA" == "false" ]]; then
        remove_if_exists "$USER_DATA_DIR"
    else
        log_info "Keeping data: $USER_DATA_DIR"
    fi
    
    # Remove aliases from shell rc files
    remove_aliases
}

# Function to remove shell aliases
remove_aliases() {
    log_info "Removing shell aliases..."
    
    local shell_files=("$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile")
    
    for shell_rc in "${shell_files[@]}"; do
        if [[ -f "$shell_rc" ]]; then
            if grep -q "polygon-pol-indexer aliases" "$shell_rc"; then
                if [[ "$DRY_RUN" == "true" ]]; then
                    echo "[DRY-RUN] Would remove aliases from $shell_rc"
                else
                    # Remove the aliases section
                    sed -i.bak '/# Polygon POL Indexer aliases/,/^$/d' "$shell_rc"
                    rm "$shell_rc.bak" 2>/dev/null || true
                    log_info "Removed aliases from $shell_rc"
                fi
            fi
        fi
    done
}

# Function to show what will be removed
show_removal_plan() {
    log_info "Uninstall plan:"
    echo "==============="
    
    if [[ "$SYSTEM_INSTALL" == "true" ]]; then
        echo "System installation will be removed:"
        echo "  - Binaries: $INSTALL_DIR"
        echo "  - Systemd service: $SERVICE_NAME"
        echo "  - System user: $SERVICE_USER"
        
        if [[ "$KEEP_CONFIG" == "false" ]]; then
            echo "  - Configuration: $CONFIG_DIR"
        else
            echo "  - Configuration: $CONFIG_DIR (KEEPING)"
        fi
        
        if [[ "$KEEP_DATA" == "false" ]]; then
            echo "  - Data: $DATA_DIR"
            echo "  - Logs: $LOG_DIR"
        else
            echo "  - Data: $DATA_DIR (KEEPING)"
            echo "  - Logs: $LOG_DIR (KEEPING)"
        fi
    fi
    
    if [[ "$USER_INSTALL" == "true" ]]; then
        echo "User installation will be removed:"
        echo "  - Binaries: $USER_INSTALL_DIR/{indexer,cli,server}"
        echo "  - Shell aliases"
        
        if [[ "$KEEP_CONFIG" == "false" ]]; then
            echo "  - Configuration: $USER_CONFIG_DIR"
        else
            echo "  - Configuration: $USER_CONFIG_DIR (KEEPING)"
        fi
        
        if [[ "$KEEP_DATA" == "false" ]]; then
            echo "  - Data: $USER_DATA_DIR"
        else
            echo "  - Data: $USER_DATA_DIR (KEEPING)"
        fi
    fi
    
    echo ""
}

# Function to confirm removal
confirm_removal() {
    if [[ "$DRY_RUN" == "true" ]]; then
        return
    fi
    
    echo -n "Are you sure you want to proceed? [y/N]: "
    read -r response
    
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        log_info "Uninstall cancelled"
        exit 0
    fi
}

# Main uninstall function
main() {
    log_info "Polygon POL Indexer Uninstaller"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_warn "DRY RUN MODE - No changes will be made"
    fi
    
    show_removal_plan
    confirm_removal
    
    if [[ "$SYSTEM_INSTALL" == "true" ]]; then
        remove_system_installation
    fi
    
    if [[ "$USER_INSTALL" == "true" ]]; then
        remove_user_installation
    fi
    
    log_info "Uninstall completed successfully!"
    
    if [[ "$KEEP_DATA" == "true" || "$KEEP_CONFIG" == "true" ]]; then
        log_info "Some files were preserved as requested"
    fi
}

# Run main function
main