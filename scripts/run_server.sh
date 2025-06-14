#!/bin/bash

# run_server.sh - Start Lair Chat server with proper configuration
#
# This script starts the server with enhanced configuration and logging

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SERVER_LOG="$PROJECT_ROOT/server_debug.log"
CONFIG_FILE="$PROJECT_ROOT/server.toml"

# Environment variables with defaults
LAIR_CHAT_HOST="${LAIR_CHAT_HOST:-127.0.0.1}"
LAIR_CHAT_PORT="${LAIR_CHAT_PORT:-8080}"
LAIR_CHAT_LOG_LEVEL="${LAIR_CHAT_LOG_LEVEL:-info}"
LAIR_CHAT_MAX_CONNECTIONS="${LAIR_CHAT_MAX_CONNECTIONS:-1000}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1" | tee -a "$SERVER_LOG"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1" | tee -a "$SERVER_LOG"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$SERVER_LOG"
}

# Help function
show_help() {
    cat << EOF
Usage: $0 [OPTIONS] [CONFIG_FILE]

Start Lair Chat server with proper configuration and logging.

OPTIONS:
    -h, --help              Show this help message
    -H, --host HOST         Server host (default: $LAIR_CHAT_HOST)
    -p, --port PORT         Server port (default: $LAIR_CHAT_PORT)
    -l, --log-level LEVEL   Log level (default: $LAIR_CHAT_LOG_LEVEL)
    -m, --max-conn NUM      Max connections (default: $LAIR_CHAT_MAX_CONNECTIONS)
    -d, --daemon            Run as daemon (background)
    -v, --verbose           Verbose output
    --no-color              Disable colored output

ARGUMENTS:
    CONFIG_FILE             Configuration file path (optional)

ENVIRONMENT VARIABLES:
    LAIR_CHAT_HOST          Default server host
    LAIR_CHAT_PORT          Default server port
    LAIR_CHAT_LOG_LEVEL     Default log level
    LAIR_CHAT_MAX_CONNECTIONS  Default max connections

EXAMPLES:
    $0                              # Start with defaults
    $0 -H 0.0.0.0 -p 9090           # Custom host and port
    $0 --daemon                     # Run in background
    $0 custom_config.toml           # Use custom config file

LOGS:
    Server output: $SERVER_LOG
EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -H|--host)
                LAIR_CHAT_HOST="$2"
                shift 2
                ;;
            -p|--port)
                LAIR_CHAT_PORT="$2"
                shift 2
                ;;
            -l|--log-level)
                LAIR_CHAT_LOG_LEVEL="$2"
                shift 2
                ;;
            -m|--max-conn)
                LAIR_CHAT_MAX_CONNECTIONS="$2"
                shift 2
                ;;
            -d|--daemon)
                DAEMON=true
                shift
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            --no-color)
                RED=''
                GREEN=''
                YELLOW=''
                BLUE=''
                NC=''
                shift
                ;;
            -*)
                log_error "Unknown option: $1"
                echo "Use -h or --help for usage information."
                exit 1
                ;;
            *)
                CONFIG_FILE="$1"
                shift
                ;;
        esac
    done
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check if we're in the right directory
    if [[ ! -f "$PROJECT_ROOT/Cargo.toml" ]]; then
        log_error "Not in Lair Chat project directory"
        log_error "Please run this script from the project root or scripts directory"
        exit 1
    fi

    # Check if cargo is available
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo is not installed or not in PATH"
        log_error "Please install Rust and Cargo first"
        exit 1
    fi

    # Check if project builds
    log_info "Verifying project builds..."
    cd "$PROJECT_ROOT"
    if ! cargo check --bin lair-chat-server --quiet; then
        log_error "Project build check failed"
        log_error "Please fix compilation errors first"
        exit 1
    fi

    log_info "Prerequisites check passed"
}

# Setup logging
setup_logging() {
    log_info "Setting up server logging..."

    # Create log file
    touch "$SERVER_LOG"

    # Set up log rotation for large files
    if [[ -f "$SERVER_LOG" ]] && [[ $(stat -f%z "$SERVER_LOG" 2>/dev/null || stat -c%s "$SERVER_LOG" 2>/dev/null) -gt 10485760 ]]; then
        local timestamp=$(date +"%Y%m%d_%H%M%S")
        mv "$SERVER_LOG" "${SERVER_LOG}.${timestamp}"
        touch "$SERVER_LOG"
        log_info "Rotated large server log file"
    fi

    # Log session start
    {
        echo "==================== SERVER SESSION START ===================="
        echo "Timestamp: $(date)"
        echo "Host: $LAIR_CHAT_HOST"
        echo "Port: $LAIR_CHAT_PORT"
        echo "Log Level: $LAIR_CHAT_LOG_LEVEL"
        echo "Max Connections: $LAIR_CHAT_MAX_CONNECTIONS"
        echo "Config File: ${CONFIG_FILE:-default}"
        echo "Project Root: $PROJECT_ROOT"
        echo "=============================================================="
        echo
    } | tee -a "$SERVER_LOG"
}

# Check if port is available
check_port_available() {
    if command -v ss &> /dev/null; then
        if ss -tuln | grep -q ":${LAIR_CHAT_PORT} "; then
            log_error "Port $LAIR_CHAT_PORT is already in use"
            log_error "Use 'ss -tuln | grep :$LAIR_CHAT_PORT' to check what's using it"
            return 1
        fi
    elif command -v netstat &> /dev/null; then
        if netstat -tuln | grep -q ":${LAIR_CHAT_PORT} "; then
            log_error "Port $LAIR_CHAT_PORT is already in use"
            log_error "Use 'netstat -tuln | grep :$LAIR_CHAT_PORT' to check what's using it"
            return 1
        fi
    else
        log_warn "Cannot check if port is in use (ss/netstat not available)"
    fi
    return 0
}

# Create default config if needed
create_default_config() {
    if [[ ! -f "$CONFIG_FILE" ]] && [[ "$CONFIG_FILE" != "$PROJECT_ROOT/server.toml" ]]; then
        log_info "Config file not found, using defaults"
        return
    fi

    if [[ ! -f "$CONFIG_FILE" ]]; then
        log_info "Creating default configuration file: $CONFIG_FILE"
        cat > "$CONFIG_FILE" << EOF
[server]
host = "$LAIR_CHAT_HOST"
port = $LAIR_CHAT_PORT
max_connections = $LAIR_CHAT_MAX_CONNECTIONS
log_level = "$LAIR_CHAT_LOG_LEVEL"

[database]
database_path = "lair_chat.db"
pool_size = 10

[auth]
min_password_length = 8
max_password_length = 128
require_complex_passwords = true

[messaging]
max_message_length = 4096
rate_limit_messages_per_minute = 60
encrypt_messages = true
EOF
        log_info "Default configuration created"
    fi
}

# Run the server
run_server() {
    log_info "Starting Lair Chat Server..."
    log_info "Host: $LAIR_CHAT_HOST"
    log_info "Port: $LAIR_CHAT_PORT"
    log_info "Log Level: $LAIR_CHAT_LOG_LEVEL"
    log_info "Max Connections: $LAIR_CHAT_MAX_CONNECTIONS"

    cd "$PROJECT_ROOT"

    # Set environment variables
    export RUST_LOG="$LAIR_CHAT_LOG_LEVEL"
    export RUST_BACKTRACE=1
    export LAIR_CHAT_HOST
    export LAIR_CHAT_PORT
    export LAIR_CHAT_MAX_CONNECTIONS

    # Additional server environment
    export RUST_LOG_STYLE=always

    local cmd_args=(
        "--host" "$LAIR_CHAT_HOST"
        "--port" "$LAIR_CHAT_PORT"
        "--max-connections" "$LAIR_CHAT_MAX_CONNECTIONS"
        "--log-level" "$LAIR_CHAT_LOG_LEVEL"
    )

    if [[ -f "$CONFIG_FILE" ]]; then
        cmd_args+=("--config" "$CONFIG_FILE")
    fi

    log_info "Server command: cargo run --bin lair-chat-server -- ${cmd_args[*]}"
    echo
    log_info "Server starting... (Press Ctrl+C to stop)"
    echo -e "${YELLOW}========================================${NC}"

    if [[ "${DAEMON:-false}" == "true" ]]; then
        # Run as daemon
        log_info "Starting server as daemon..."
        nohup cargo run --bin lair-chat-server -- "${cmd_args[@]}" >> "$SERVER_LOG" 2>&1 &
        local server_pid=$!
        echo "$server_pid" > "$PROJECT_ROOT/server.pid"
        log_info "Server started as daemon with PID: $server_pid"
        log_info "Logs: $SERVER_LOG"
        log_info "Stop with: kill $server_pid"
    else
        # Run in foreground
        if [[ "${VERBOSE:-false}" == "true" ]]; then
            cargo run --bin lair-chat-server -- "${cmd_args[@]}" 2>&1 | tee -a "$SERVER_LOG"
        else
            cargo run --bin lair-chat-server -- "${cmd_args[@]}"
        fi
    fi
}

# Cleanup function
cleanup() {
    log_info "Server stopping..."

    # Remove PID file if it exists
    if [[ -f "$PROJECT_ROOT/server.pid" ]]; then
        rm -f "$PROJECT_ROOT/server.pid"
    fi

    log_info "Server cleanup completed"
}

# Main function
main() {
    # Set up trap for cleanup
    trap cleanup EXIT INT TERM

    # Parse arguments
    parse_args "$@"

    # Run setup steps
    check_prerequisites
    setup_logging

    # Check port availability
    if ! check_port_available; then
        exit 1
    fi

    # Create config if needed
    create_default_config

    # Run the server
    run_server
}

# Run main function with all arguments
main "$@"
