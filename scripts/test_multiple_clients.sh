#!/bin/bash

# Lair Chat Multiple Client Testing Script
# This script automates testing with multiple clients for easier development and testing

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
SERVER_BIN="$PROJECT_DIR/target/release/lair-chat-server"
CLIENT_BIN="$PROJECT_DIR/target/release/lair-chat-client"
SERVER_PID=""
CLIENT_PIDS=()

# Default settings
NUM_CLIENTS=3
SERVER_HOST="127.0.0.1"
SERVER_PORT="8080"
AUTO_MESSAGE=false
MESSAGE_INTERVAL=5
TEST_DURATION=60
VERBOSE=false

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"

    # Kill client processes
    for pid in "${CLIENT_PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            echo "Stopping client (PID: $pid)"
            kill "$pid" 2>/dev/null || true
        fi
    done

    # Kill server process
    if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "Stopping server (PID: $SERVER_PID)"
        kill "$SERVER_PID" 2>/dev/null || true
        sleep 2
        # Force kill if still running
        if kill -0 "$SERVER_PID" 2>/dev/null; then
            kill -9 "$SERVER_PID" 2>/dev/null || true
        fi
    fi

    echo -e "${GREEN}Cleanup complete${NC}"
    exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM EXIT

# Help function
show_help() {
    cat << EOF
Lair Chat Multiple Client Testing Script

Usage: $0 [OPTIONS]

Options:
    -n, --num-clients NUM       Number of clients to spawn (default: 3)
    -h, --host HOST            Server host (default: 127.0.0.1)
    -p, --port PORT            Server port (default: 8080)
    -a, --auto-message         Enable automatic message sending
    -i, --interval SECONDS     Message interval for auto-messaging (default: 5)
    -d, --duration SECONDS     Test duration in seconds (default: 60)
    -v, --verbose              Enable verbose logging
    -s, --server-only          Start only the server (no clients)
    -c, --clients-only         Start only clients (assume server is running)
    --build                    Build the project before testing
    --help                     Show this help message

Examples:
    $0                         # Start server + 3 clients
    $0 -n 5 -a                 # Start server + 5 clients with auto-messaging
    $0 --server-only           # Start only the server
    $0 --clients-only -n 2     # Start 2 clients (server must be running)
    $0 --build -n 4 -v         # Build project, then test with 4 clients, verbose

Interactive Commands (while running):
    Press 'q' + Enter to quit
    Press 'r' + Enter to restart clients
    Press 's' + Enter to show status
EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -n|--num-clients)
                NUM_CLIENTS="$2"
                shift 2
                ;;
            -h|--host)
                SERVER_HOST="$2"
                shift 2
                ;;
            -p|--port)
                SERVER_PORT="$2"
                shift 2
                ;;
            -a|--auto-message)
                AUTO_MESSAGE=true
                shift
                ;;
            -i|--interval)
                MESSAGE_INTERVAL="$2"
                shift 2
                ;;
            -d|--duration)
                TEST_DURATION="$2"
                shift 2
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -s|--server-only)
                SERVER_ONLY=true
                shift
                ;;
            -c|--clients-only)
                CLIENTS_ONLY=true
                shift
                ;;
            --build)
                BUILD_FIRST=true
                shift
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
                show_help
                exit 1
                ;;
        esac
    done
}

# Build the project
build_project() {
    echo -e "${BLUE}Building project...${NC}"
    cd "$PROJECT_DIR"

    if ! cargo build --release; then
        echo -e "${RED}Build failed!${NC}"
        exit 1
    fi

    echo -e "${GREEN}Build successful${NC}"
}

# Check if binaries exist
check_binaries() {
    if [[ ! -f "$SERVER_BIN" ]]; then
        echo -e "${RED}Server binary not found: $SERVER_BIN${NC}"
        echo "Run with --build flag or build manually with: cargo build --release"
        exit 1
    fi

    if [[ ! -f "$CLIENT_BIN" ]]; then
        echo -e "${RED}Client binary not found: $CLIENT_BIN${NC}"
        echo "Run with --build flag or build manually with: cargo build --release"
        exit 1
    fi
}

# Start the server
start_server() {
    echo -e "${BLUE}Starting Lair Chat server on $SERVER_HOST:$SERVER_PORT...${NC}"

    cd "$PROJECT_DIR"

    # Set environment variables
    export LAIR_CHAT_SERVER_HOST="$SERVER_HOST"
    export LAIR_CHAT_SERVER_PORT="$SERVER_PORT"
    export LAIR_CHAT_DATABASE_URL=":memory:"
    export LAIR_CHAT_LOGGING_LEVEL="info"

    if [[ "$VERBOSE" == "true" ]]; then
        export RUST_LOG="debug"
        export LAIR_CHAT_LOGGING_LEVEL="debug"
    fi

    # Start server in background
    if [[ "$VERBOSE" == "true" ]]; then
        "$SERVER_BIN" &
    else
        "$SERVER_BIN" > /dev/null 2>&1 &
    fi

    SERVER_PID=$!

    # Wait for server to start
    echo "Waiting for server to start..."
    sleep 3

    # Check if server is still running
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo -e "${RED}Server failed to start!${NC}"
        exit 1
    fi

    echo -e "${GREEN}Server started (PID: $SERVER_PID)${NC}"
}

# Generate test user credentials
generate_user_creds() {
    local user_num=$1
    echo "testuser$user_num:testpass$user_num"
}

# Start a single client
start_client() {
    local client_num=$1
    local user_creds
    user_creds=$(generate_user_creds "$client_num")
    local username="${user_creds%:*}"
    local password="${user_creds#*:}"

    echo -e "${BLUE}Starting client $client_num (user: $username)...${NC}"

    # Create expect script for automated client interaction
    local expect_script="/tmp/lair_chat_client_$client_num.exp"

    cat > "$expect_script" << EOF
#!/usr/bin/expect -f

set timeout 30
set client_num $client_num
set username "$username"
set password "$password"
set server "$SERVER_HOST:$SERVER_PORT"
set auto_message $AUTO_MESSAGE
set message_interval $MESSAGE_INTERVAL
set duration $TEST_DURATION

# Start the client
spawn $CLIENT_BIN

# Handle connection prompts
expect {
    "Server address" {
        send "\$server\r"
        exp_continue
    }
    "Username" {
        send "\$username\r"
        exp_continue
    }
    "Password" {
        send "\$password\r"
        exp_continue
    }
    "Register new account" {
        send "y\r"
        exp_continue
    }
    "Email" {
        send "\$username@example.com\r"
        exp_continue
    }
    "Connected" {
        puts "Client \$client_num connected successfully"
    }
    "Welcome" {
        puts "Client \$client_num welcomed to chat"
    }
    ">" {
        puts "Client \$client_num ready for input"
    }
    timeout {
        puts "Client \$client_num connection timeout"
        exit 1
    }
    eof {
        puts "Client \$client_num connection closed unexpectedly"
        exit 1
    }
}

# Send initial message
sleep 2
send "Hello from \$username (client \$client_num)!\r"

# Auto-messaging loop if enabled
if {\$auto_message} {
    set start_time [clock seconds]
    set message_count 0

    while {[expr [clock seconds] - \$start_time] < \$duration} {
        sleep \$message_interval
        incr message_count
        send "Auto message \$message_count from \$username\r"

        # Occasionally send commands
        if {\$message_count % 10 == 0} {
            send "/users\r"
            sleep 1
        }
    }
} else {
    # Keep client alive for manual testing
    interact
}

EOF

    # Make expect script executable
    chmod +x "$expect_script"

    # Start client with expect script
    if [[ "$VERBOSE" == "true" ]]; then
        "$expect_script" &
    else
        "$expect_script" > "/tmp/client_$client_num.log" 2>&1 &
    fi

    local client_pid=$!
    CLIENT_PIDS+=("$client_pid")

    echo -e "${GREEN}Client $client_num started (PID: $client_pid)${NC}"
    sleep 1
}

# Start multiple clients
start_clients() {
    echo -e "${BLUE}Starting $NUM_CLIENTS clients...${NC}"

    for ((i=1; i<=NUM_CLIENTS; i++)); do
        start_client "$i"
        sleep 2  # Stagger client starts
    done

    echo -e "${GREEN}All clients started${NC}"
}

# Show status
show_status() {
    echo -e "\n${BLUE}=== Lair Chat Test Status ===${NC}"

    if [[ -n "$SERVER_PID" ]]; then
        if kill -0 "$SERVER_PID" 2>/dev/null; then
            echo -e "Server: ${GREEN}Running${NC} (PID: $SERVER_PID)"
        else
            echo -e "Server: ${RED}Stopped${NC}"
        fi
    else
        echo -e "Server: ${YELLOW}Not started by this script${NC}"
    fi

    echo "Clients: ${#CLIENT_PIDS[@]} started"
    local running_clients=0

    for i in "${!CLIENT_PIDS[@]}"; do
        local pid="${CLIENT_PIDS[$i]}"
        if kill -0 "$pid" 2>/dev/null; then
            echo -e "  Client $((i+1)): ${GREEN}Running${NC} (PID: $pid)"
            ((running_clients++))
        else
            echo -e "  Client $((i+1)): ${RED}Stopped${NC}"
        fi
    done

    echo "Running clients: $running_clients/${#CLIENT_PIDS[@]}"
    echo -e "${BLUE}=========================${NC}\n"
}

# Interactive mode
interactive_mode() {
    echo -e "\n${YELLOW}Interactive mode: Press 'q' to quit, 'r' to restart clients, 's' for status${NC}"

    while true; do
        read -r -t 1 input || continue

        case "$input" in
            q|quit|exit)
                echo "Quitting..."
                break
                ;;
            r|restart)
                echo "Restarting clients..."
                # Kill existing clients
                for pid in "${CLIENT_PIDS[@]}"; do
                    kill "$pid" 2>/dev/null || true
                done
                CLIENT_PIDS=()
                sleep 2
                start_clients
                ;;
            s|status)
                show_status
                ;;
            *)
                if [[ -n "$input" ]]; then
                    echo "Unknown command: $input (use 'q', 'r', or 's')"
                fi
                ;;
        esac
    done
}

# Main function
main() {
    echo -e "${BLUE}Lair Chat Multiple Client Testing Script${NC}"
    echo -e "${BLUE}=======================================${NC}\n"

    parse_args "$@"

    # Build if requested
    if [[ "$BUILD_FIRST" == "true" ]]; then
        build_project
    fi

    # Check binaries
    check_binaries

    # Check if expect is available for automated clients
    if ! command -v expect &> /dev/null; then
        echo -e "${YELLOW}Warning: 'expect' not found. Installing for automated client interaction...${NC}"
        if command -v apt-get &> /dev/null; then
            sudo apt-get update && sudo apt-get install -y expect
        elif command -v brew &> /dev/null; then
            brew install expect
        else
            echo -e "${RED}Please install 'expect' manually for full functionality${NC}"
            exit 1
        fi
    fi

    # Start server unless clients-only mode
    if [[ "$CLIENTS_ONLY" != "true" ]]; then
        start_server
    fi

    # Start clients unless server-only mode
    if [[ "$SERVER_ONLY" != "true" ]]; then
        start_clients
        show_status

        if [[ "$AUTO_MESSAGE" == "true" ]]; then
            echo -e "${YELLOW}Auto-messaging enabled. Test will run for $TEST_DURATION seconds...${NC}"
            sleep "$TEST_DURATION"
            echo -e "${GREEN}Test duration completed${NC}"
        else
            interactive_mode
        fi
    else
        echo -e "${GREEN}Server running. Press Ctrl+C to stop.${NC}"
        # Keep script running
        while kill -0 "$SERVER_PID" 2>/dev/null; do
            sleep 1
        done
    fi
}

# Run main function with all arguments
main "$@"
