#!/bin/bash

# Lair Chat Universal Start Script
# Choose your server mode and start with one command!

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored output
print_banner() {
    echo -e "\n${PURPLE}============================================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}============================================================${NC}\n"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_option() {
    echo -e "${CYAN}$1${NC} $2"
}

# Show banner
print_banner "🎯 Lair Chat Universal Launcher"

echo -e "${CYAN}Choose your server mode:${NC}\n"

print_option "1)" "🔗 REST API Only (Modern Web + Admin Dashboard)"
print_option "2)" "🔌 TCP Chat Only (Classic Terminal Chat)"
print_option "3)" "🎯 INTEGRATED (Both TCP + REST API together) ⭐ RECOMMENDED"
print_option "4)" "🔧 Development Mode (Verbose logging)"
print_option "5)" "✅ Setup & Verify System"
print_option "6)" "❓ Help & Information"

echo ""

# Get user choice
read -p "$(echo -e ${YELLOW}Enter your choice [1-6]: ${NC})" choice

case $choice in
    1)
        print_banner "🔗 Starting REST API Server"
        print_info "Features: Admin Dashboard, REST API, User Management"
        print_info "Access: http://127.0.0.1:8082/admin/"
        echo ""
        if [ -x "./start.sh" ]; then
            ./start.sh
        else
            print_error "start.sh not found. Using fallback..."
            cargo run --bin lair-chat-server-new
        fi
        ;;

    2)
        print_banner "🔌 Starting TCP Chat Server"
        print_info "Features: Terminal-based chat, Classic protocol"
        print_info "Access: telnet 127.0.0.1 8080"
        echo ""
        export TCP_PORT=8080
        cargo run --bin lair-chat-server
        ;;

    3)
        print_banner "🎯 Starting INTEGRATED Server (RECOMMENDED)"
        print_info "Features: TCP Chat + REST API + Admin Dashboard"
        print_info "TCP Access: telnet 127.0.0.1 8080"
        print_info "Web Access: http://127.0.0.1:8082/admin/"
        echo ""
        if [ -x "./start_integrated.sh" ]; then
            ./start_integrated.sh
        else
            print_error "start_integrated.sh not found. Using fallback..."
            export TCP_PORT=8080
            export REST_PORT=8082
            cargo run --bin lair-chat-server
        fi
        ;;

    4)
        print_banner "🔧 Starting Development Mode"
        print_info "Features: Verbose logging, Debug output, Hot reload friendly"
        echo ""
        if [ -x "./dev_start.sh" ]; then
            ./dev_start.sh
        else
            print_error "dev_start.sh not found. Using fallback..."
            export RUST_LOG=debug,lair_chat=trace
            export RUST_BACKTRACE=full
            cargo run --bin lair-chat-server-new
        fi
        ;;

    5)
        print_banner "✅ Setup & Verify System"
        print_info "Running system setup and verification..."
        echo ""

        # Run setup if available
        if [ -x "./setup_admin_system.sh" ]; then
            print_info "Running complete system setup..."
            ./setup_admin_system.sh
        else
            print_info "Basic setup (creating directories and building)..."
            mkdir -p data logs admin-dashboard
            cargo build --release --bin lair-chat-server-new --bin lair-chat-server
        fi

        # Run verification if available
        if [ -x "./verify_system.sh" ]; then
            print_info "Running system verification..."
            ./verify_system.sh
        else
            print_info "Basic verification (checking builds)..."
            if [ -f "target/release/lair-chat-server-new" ]; then
                print_success "REST API server binary ready"
            else
                print_error "REST API server binary not found"
            fi

            if [ -f "target/release/lair-chat-server" ]; then
                print_success "TCP server binary ready"
            else
                print_error "TCP server binary not found"
            fi
        fi
        ;;

    6)
        print_banner "❓ Help & Information"
        echo -e "${CYAN}🎯 Lair Chat Server Modes:${NC}\n"

        echo -e "${YELLOW}1. REST API Only:${NC}"
        echo -e "   • Modern web-based admin dashboard"
        echo -e "   • RESTful API for integrations"
        echo -e "   • JWT authentication"
        echo -e "   • User and room management"
        echo -e "   • Best for: Web applications, mobile apps, integrations"
        echo ""

        echo -e "${YELLOW}2. TCP Chat Only:${NC}"
        echo -e "   • Classic terminal-based chat"
        echo -e "   • Direct TCP connections"
        echo -e "   • Real-time messaging"
        echo -e "   • End-to-end encryption"
        echo -e "   • Best for: Terminal users, legacy clients, maximum performance"
        echo ""

        echo -e "${YELLOW}3. INTEGRATED (Recommended):${NC}"
        echo -e "   • Combines both TCP and REST API servers"
        echo -e "   • Shared database and user accounts"
        echo -e "   • Manage TCP users via web dashboard"
        echo -e "   • Best for: Maximum flexibility and features"
        echo ""

        echo -e "${YELLOW}4. Development Mode:${NC}"
        echo -e "   • Verbose logging and debugging"
        echo -e "   • Full stack traces"
        echo -e "   • Live monitoring"
        echo -e "   • Best for: Development and troubleshooting"
        echo ""

        echo -e "${CYAN}🔧 Available Scripts:${NC}"
        echo -e "   ./run.sh              - This universal launcher"
        echo -e "   ./start.sh            - REST API server only"
        echo -e "   ./start_integrated.sh - Combined TCP + REST server"
        echo -e "   ./dev_start.sh        - Development mode"
        echo -e "   ./setup_admin_system.sh - Complete system setup"
        echo -e "   ./verify_system.sh    - System verification"
        echo -e "   ./test_api.sh         - API endpoint testing"
        echo ""

        echo -e "${CYAN}🔐 Default Credentials:${NC}"
        echo -e "   Username: admin"
        echo -e "   Password: AdminPassword123!"
        echo ""

        echo -e "${CYAN}📊 Access URLs:${NC}"
        echo -e "   TCP Chat:         telnet 127.0.0.1 8080"
        echo -e "   Admin Dashboard:  http://127.0.0.1:8082/admin/"
        echo -e "   REST API:         http://127.0.0.1:8082/api/v1"
        echo -e "   Health Check:     http://127.0.0.1:8082/api/v1/health"
        echo ""

        echo -e "${CYAN}💡 Quick Start:${NC}"
        echo -e "   For most users: Choose option 3 (INTEGRATED)"
        echo -e "   For web only:   Choose option 1 (REST API)"
        echo -e "   For chat only:  Choose option 2 (TCP)"
        echo ""

        # Ask if user wants to start a server
        read -p "$(echo -e ${YELLOW}Would you like to start a server now? [y/N]: ${NC})" -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo ""
            exec "$0"  # Restart this script
        fi
        ;;

    *)
        print_error "Invalid choice. Please select 1-6."
        echo ""
        exec "$0"  # Restart this script
        ;;
esac
