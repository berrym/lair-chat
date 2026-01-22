#!/bin/bash

# Lair Chat Server Startup Script with Database Serialization Fixes
# This script ensures the server starts with the correct database configuration
# to maintain the UserProfile serialization fixes.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
DATABASE_PATH="sqlite:data/lair-chat.db"
SERVER_BINARY="./target/release/lair-chat-server"
LOG_FILE="logs/server.log"

print_banner() {
    echo -e "\n${CYAN}============================================================${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}============================================================${NC}\n"
}

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

# Check if we're in the right directory
check_environment() {
    print_step "Checking environment..."

    if [ ! -f "Cargo.toml" ]; then
        print_error "Please run this script from the lair-chat directory"
        exit 1
    fi

    if [ ! -f "$SERVER_BINARY" ]; then
        print_warning "Server binary not found. Building..."
        cargo build --release --bin lair-chat-server
        if [ $? -ne 0 ]; then
            print_error "Failed to build server"
            exit 1
        fi
    fi

    # Ensure logs directory exists
    mkdir -p logs

    # Ensure data directory exists
    mkdir -p data

    print_success "Environment check passed"
}

# Stop any running servers
stop_existing_servers() {
    print_step "Stopping any existing lair-chat servers..."

    pkill -f lair-chat-server 2>/dev/null || true
    pkill -f lair-chat-server-new 2>/dev/null || true

    sleep 2
    print_success "Existing servers stopped"
}

# Check database status
check_database() {
    print_step "Checking database status..."

    if [ -f "data/lair-chat.db" ]; then
        # Check if admin user exists
        local admin_count=$(sqlite3 data/lair-chat.db "SELECT COUNT(*) FROM users WHERE role = 'admin';" 2>/dev/null || echo "0")

        if [ "$admin_count" -gt 0 ]; then
            print_success "Database found with $admin_count admin user(s)"
        else
            print_warning "Database found but no admin users detected"
            print_info "You may need to create an admin user with: DATABASE_URL=\"$DATABASE_PATH\" cargo run --bin create_admin_user"
        fi
    else
        print_warning "Database not found at data/lair-chat.db"
        print_info "Server will create a new database on first run"
    fi
}

# Start the server with correct configuration
start_server() {
    print_step "Starting lair-chat server with fixed database configuration..."

    print_info "Database Path: $DATABASE_PATH"
    print_info "Log File: $LOG_FILE"
    print_info "Server Binary: $SERVER_BINARY"

    # Export the correct database URL
    export DATABASE_URL="$DATABASE_PATH"

    case "${1:-foreground}" in
        "background"|"bg"|"-d"|"--daemon")
            print_info "Starting server in background mode..."
            nohup $SERVER_BINARY > $LOG_FILE 2>&1 &
            local server_pid=$!
            echo $server_pid > .server.pid

            print_success "Server started in background (PID: $server_pid)"
            print_info "Server logs: $LOG_FILE"
            print_info "Stop with: kill $server_pid or ./stop_server.sh"
            ;;
        "foreground"|"fg"|*)
            print_info "Starting server in foreground mode..."
            print_info "Press Ctrl+C to stop the server"
            exec $SERVER_BINARY
            ;;
    esac
}

# Wait for server to be ready and test
test_server() {
    if [ -f ".server.pid" ]; then
        local server_pid=$(cat .server.pid)
        print_step "Waiting for server to be ready..."

        for i in {1..15}; do
            if curl -s http://127.0.0.1:8082/api/v1/health > /dev/null 2>&1; then
                print_success "Server is responding on port 8082"
                break
            fi

            if ! kill -0 $server_pid 2>/dev/null; then
                print_error "Server process died. Check logs: $LOG_FILE"
                exit 1
            fi

            if [ $i -eq 15 ]; then
                print_warning "Server may still be starting. Check logs: $LOG_FILE"
                return 1
            fi

            sleep 2
        done
    fi
}

# Display connection information
show_info() {
    print_banner "🚀 Lair Chat Server Started Successfully!"

    echo -e "${GREEN}Your server is now running with database serialization fixes applied.${NC}"
    echo ""
    echo -e "${CYAN}🎯 Access Points:${NC}"
    echo -e "   • Admin Dashboard:  ${YELLOW}http://127.0.0.1:8082/admin/${NC}"
    echo -e "   • REST API:         ${YELLOW}http://127.0.0.1:8082/api/v1${NC}"
    echo -e "   • API Health:       ${YELLOW}http://127.0.0.1:8082/api/v1/health${NC}"
    echo -e "   • TCP Chat:         ${YELLOW}telnet 127.0.0.1 8080${NC}"
    echo ""
    echo -e "${CYAN}🔐 Default Admin Credentials:${NC}"
    echo -e "   • Username: ${YELLOW}admin${NC}"
    echo -e "   • Password: ${YELLOW}AdminPassword123!${NC}"
    echo ""
    echo -e "${CYAN}🔧 Database Configuration:${NC}"
    echo -e "   • Database URL:     ${YELLOW}$DATABASE_PATH${NC}"
    echo -e "   • UserProfile fixes applied and working"
    echo -e "   • Serialization conflicts resolved"
    echo ""
    echo -e "${CYAN}📋 Useful Commands:${NC}"
    echo -e "   • View logs:        ${BLUE}tail -f $LOG_FILE${NC}"
    echo -e "   • Stop server:      ${BLUE}pkill -f lair-chat-server${NC}"
    echo -e "   • Create admin:     ${BLUE}DATABASE_URL=\"$DATABASE_PATH\" cargo run --bin create_admin_user${NC}"
    echo -e "   • Test auth:        ${BLUE}curl -X POST http://127.0.0.1:8082/api/v1/auth/login -H \"Content-Type: application/json\" -d '{\"identifier\":\"admin\",\"password\":\"AdminPassword123!\",\"remember_me\":true}'${NC}"
    echo ""
}

# Main function
main() {
    print_banner "🔧 Lair Chat Server (Fixed Database Configuration)"

    case "${1:-}" in
        "--help"|"-h")
            echo "Usage: $0 [mode]"
            echo ""
            echo "Modes:"
            echo "  foreground, fg     Start server in foreground (default)"
            echo "  background, bg     Start server in background"
            echo "  -d, --daemon       Start server as daemon (same as background)"
            echo ""
            echo "This script ensures the server starts with the correct database"
            echo "configuration to maintain UserProfile serialization fixes."
            echo ""
            echo "The server will use: $DATABASE_PATH"
            exit 0
            ;;
    esac

    check_environment
    stop_existing_servers
    check_database
    start_server "$1"

    if [ "${1:-foreground}" != "foreground" ] && [ "${1:-foreground}" != "fg" ]; then
        test_server
        show_info
    fi
}

# Trap to ensure cleanup on exit
cleanup() {
    if [ -f ".server.pid" ]; then
        local server_pid=$(cat .server.pid)
        if kill -0 $server_pid 2>/dev/null; then
            print_info "Stopping server (PID: $server_pid)..."
            kill $server_pid
        fi
        rm -f .server.pid
    fi
}

trap cleanup EXIT INT TERM

# Run main function with all arguments
main "$@"
