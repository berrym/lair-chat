#!/bin/bash

# run_client.sh - Start Lair Chat client with enhanced debugging and user-friendly interface
#
# This script starts the client with proper configuration and enhanced error reporting

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CLIENT_LOG="$PROJECT_ROOT/client_debug.log"

# Environment variables with defaults
LAIR_CHAT_HOST="${LAIR_CHAT_HOST:-127.0.0.1}"
LAIR_CHAT_PORT="${LAIR_CHAT_PORT:-8080}"
LAIR_CHAT_LOG_LEVEL="${LAIR_CHAT_LOG_LEVEL:-info}"
LAIR_CHAT_DEBUG="${LAIR_CHAT_DEBUG:-false}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1" | tee -a "$CLIENT_LOG"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1" | tee -a "$CLIENT_LOG"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$CLIENT_LOG"
}

# Help function
show_help() {
    cat << EOF
Usage: $0 [OPTIONS] [SERVER_ADDRESS]

Start Lair Chat client with enhanced debugging and user-friendly interface.

OPTIONS:
    -h, --help              Show this help message
    -s, --server HOST       Server host (default: $LAIR_CHAT_HOST)
    -p, --port PORT         Server port (default: $LAIR_CHAT_PORT)
    -l, --log-level LEVEL   Log level (default: $LAIR_CHAT_LOG_LEVEL)
    -d, --debug             Enable debug mode
    -r, --reconnect         Enable automatic reconnection
    -v, --verbose           Verbose output
    --no-color              Disable colored output

ARGUMENTS:
    SERVER_ADDRESS          Server address in format host:port (optional)

ENVIRONMENT VARIABLES:
    LAIR_CHAT_HOST          Default server host
    LAIR_CHAT_PORT          Default server port
    LAIR_CHAT_LOG_LEVEL     Default log level
    LAIR_CHAT_DEBUG         Enable debug mode (true/false)

EXAMPLES:
    $0                              # Connect to local server
    $0 -s 192.168.1.100 -p 9090    # Connect to remote server
    $0 --debug --verbose           # Debug mode with verbose output
    $0 192.168.1.100:9090          # Connect using address format

LOGS:
    Client output: $CLIENT_LOG
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
            -s|--server)
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
            -d|--debug)
                LAIR_CHAT_DEBUG=true
                LAIR_CHAT_LOG_LEVEL=debug
                shift
                ;;
            -r|--reconnect)
                AUTO_RECONNECT=true
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
                # Parse server address in format host:port
                if [[ "$1" =~ ^([^:]+):([0-9]+)$ ]]; then
                    LAIR_CHAT_HOST="${BASH_REMATCH[1]}"
                    LAIR_CHAT_PORT="${BASH_REMATCH[2]}"
                else
                    LAIR_CHAT_HOST="$1"
                fi
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
    if ! cargo check --bin lair-chat-client --quiet; then
        log_error "Project build check failed"
        log_error "Please fix compilation errors first"
        exit 1
    fi

    log_info "Prerequisites check passed"
}

# Setup logging
setup_logging() {
    log_info "Setting up client logging..."

    # Create log file
    touch "$CLIENT_LOG"

    # Set up log rotation for large files
    if [[ -f "$CLIENT_LOG" ]] && [[ $(stat -f%z "$CLIENT_LOG" 2>/dev/null || stat -c%s "$CLIENT_LOG" 2>/dev/null) -gt 10485760 ]]; then
        local timestamp=$(date +"%Y%m%d_%H%M%S")
        mv "$CLIENT_LOG" "${CLIENT_LOG}.${timestamp}"
        touch "$CLIENT_LOG"
        log_info "Rotated large client log file"
    fi

    # Log session start
    {
        echo "==================== CLIENT SESSION START ===================="
        echo "Timestamp: $(date)"
        echo "Server: $LAIR_CHAT_HOST:$LAIR_CHAT_PORT"
        echo "Log Level: $LAIR_CHAT_LOG_LEVEL"
        echo "Debug Mode: $LAIR_CHAT_DEBUG"
        echo "Auto Reconnect: ${AUTO_RECONNECT:-false}"
        echo "Project Root: $PROJECT_ROOT"
        echo "=============================================================="
        echo
    } | tee -a "$CLIENT_LOG"
}

# Test server connectivity
test_server_connectivity() {
    log_info "Testing server connectivity..."

    # Test basic TCP connectivity
    if timeout 5 bash -c "</dev/tcp/$LAIR_CHAT_HOST/$LAIR_CHAT_PORT" 2>/dev/null; then
        log_info "Server is reachable at $LAIR_CHAT_HOST:$LAIR_CHAT_PORT"
        return 0
    else
        log_warn "Cannot connect to server at $LAIR_CHAT_HOST:$LAIR_CHAT_PORT"
        log_warn "Make sure the server is running"
        log_warn "You can start the server with: ./scripts/run_server.sh"

        if [[ "${AUTO_RECONNECT:-false}" != "true" ]]; then
            read -p "Continue anyway? [y/N]: " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                log_info "Exiting..."
                exit 1
            fi
        fi
        return 1
    fi
}

# Run the client with retry logic
run_client_with_retry() {
    local max_retries=3
    local retry_count=0

    while [[ $retry_count -lt $max_retries ]]; do
        if run_client; then
            break
        else
            retry_count=$((retry_count + 1))
            if [[ $retry_count -lt $max_retries ]] && [[ "${AUTO_RECONNECT:-false}" == "true" ]]; then
                log_warn "Client failed, retrying in 5 seconds... (attempt $retry_count/$max_retries)"
                sleep 5
            else
                log_error "Client failed after $retry_count attempts"
                break
            fi
        fi
    done
}

# Run the client
run_client() {
    log_info "Starting Lair Chat Client..."
    log_info "Server: $LAIR_CHAT_HOST:$LAIR_CHAT_PORT"
    log_info "Log Level: $LAIR_CHAT_LOG_LEVEL"
    log_info "Debug Mode: $LAIR_CHAT_DEBUG"

    cd "$PROJECT_ROOT"

    # Set environment variables for the client
    export RUST_LOG="$LAIR_CHAT_LOG_LEVEL"
    export RUST_BACKTRACE=1
    export LAIR_CHAT_HOST
    export LAIR_CHAT_PORT
    export LAIR_CHAT_DEBUG

    # Additional debug environment
    if [[ "$LAIR_CHAT_DEBUG" == "true" ]]; then
        export RUST_LOG_STYLE=always
        export COLORBT_SHOW_HIDDEN=1
    fi

    log_info "Environment configured:"
    log_info "RUST_LOG=$RUST_LOG"
    log_info "LAIR_CHAT_HOST=$LAIR_CHAT_HOST"
    log_info "LAIR_CHAT_PORT=$LAIR_CHAT_PORT"
    log_info "LAIR_CHAT_DEBUG=$LAIR_CHAT_DEBUG"

    echo
    log_info "Starting client... (Press Ctrl+C to stop)"
    echo -e "${YELLOW}========================================${NC}"
    echo -e "${BLUE}Welcome to Lair Chat!${NC}"
    echo -e "${BLUE}Connecting to $LAIR_CHAT_HOST:$LAIR_CHAT_PORT...${NC}"
    echo -e "${YELLOW}========================================${NC}"

    # Run the client with output to both console and log file
    if [[ "${VERBOSE:-false}" == "true" ]]; then
        cargo run --bin lair-chat-client 2>&1 | tee -a "$CLIENT_LOG"
    else
        cargo run --bin lair-chat-client
    fi
}

# Check for updates
check_for_updates() {
    if [[ "${VERBOSE:-false}" == "true" ]]; then
        log_info "Checking for updates..."
        cd "$PROJECT_ROOT"
        if git status --porcelain | grep -q .; then
            log_warn "Local changes detected, skipping update check"
        else
            local current_commit=$(git rev-parse HEAD)
            git fetch --quiet origin main 2>/dev/null || true
            local latest_commit=$(git rev-parse origin/main 2>/dev/null || echo "$current_commit")

            if [[ "$current_commit" != "$latest_commit" ]]; then
                log_info "Updates available! Run 'git pull' to update"
            fi
        fi
    fi
}

# Show helpful tips
show_tips() {
    echo
    echo -e "${BLUE}=== Helpful Tips ===${NC}"
    echo "• Use '/help' in the client for available commands"
    echo "• Use '/quit' or Ctrl+C to exit"
    echo "• Check logs in: $CLIENT_LOG"
    echo "• Server logs: $PROJECT_ROOT/server_debug.log"
    echo "• For debugging: ./scripts/debug_client.sh"
    echo
}

# Cleanup function
cleanup() {
    log_info "Client session ending..."

    # Archive logs if they exist and are not empty
    if [[ -s "$CLIENT_LOG" ]]; then
        local timestamp=$(date +"%Y%m%d_%H%M%S")
        local archive_log="$PROJECT_ROOT/client_${timestamp}.log"
        cp "$CLIENT_LOG" "$archive_log"
        log_info "Client log archived to: $archive_log"
    fi

    log_info "Client cleanup completed"
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
    check_for_updates

    # Test connectivity
    test_server_connectivity

    # Show helpful tips
    show_tips

    # Run the client
    if [[ "${AUTO_RECONNECT:-false}" == "true" ]]; then
        run_client_with_retry
    else
        run_client
    fi
}

# Run main function with all arguments
main "$@"
