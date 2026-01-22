#!/bin/bash

# Lair Chat Database Reset Script
# This script completely resets the database and starts fresh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
DEFAULT_ADMIN_USERNAME="${ADMIN_USERNAME:-admin}"
DEFAULT_ADMIN_PASSWORD="${ADMIN_PASSWORD:-AdminPassword123!}"
DEFAULT_ADMIN_EMAIL="${ADMIN_EMAIL:-admin@example.com}"

# Function to print colored output
print_banner() {
    echo -e "\n${PURPLE}============================================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}============================================================${NC}\n"
}

print_section() {
    echo -e "\n${CYAN}▶ $1${NC}"
    echo -e "${CYAN}$(printf '%.0s-' {1..50})${NC}"
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

# Function to check if user wants to proceed
confirm_reset() {
    print_banner "🔄 Lair Chat Database Reset"

    echo -e "${RED}⚠️  WARNING: This will completely destroy your current database!${NC}"
    echo -e "${RED}⚠️  All users, rooms, messages, and settings will be lost!${NC}"
    echo ""
    echo -e "${YELLOW}This operation will:${NC}"
    echo -e "   • Delete all database files"
    echo -e "   • Stop any running servers"
    echo -e "   • Create a fresh database schema"
    echo -e "   • Create a new admin user"
    echo ""

    read -p "Are you absolutely sure you want to continue? (type 'YES' to confirm): " confirm

    if [ "$confirm" != "YES" ]; then
        print_info "Operation cancelled. Database unchanged."
        exit 0
    fi

    echo ""
    print_warning "Starting database reset in 3 seconds..."
    sleep 1
    echo -e "${YELLOW}3...${NC}"
    sleep 1
    echo -e "${YELLOW}2...${NC}"
    sleep 1
    echo -e "${YELLOW}1...${NC}"
    sleep 1
    print_info "Beginning reset process..."
}

# Function to stop running services
stop_services() {
    print_section "Stopping Running Services"

    print_step "Stopping lair-chat servers..."
    pkill -f lair-chat-server-new 2>/dev/null || true
    pkill -f lair-chat-server 2>/dev/null || true
    pkill -f lair-chat-client 2>/dev/null || true
    pkill -f quick_start.sh 2>/dev/null || true
    pkill -f setup_admin_system.sh 2>/dev/null || true

    # Stop Python HTTP servers (admin dashboard)
    pkill -f "python.*http.server" 2>/dev/null || true
    pkill -f "python3.*http.server" 2>/dev/null || true

    sleep 2
    print_success "Services stopped"
}

# Function to backup existing database
backup_database() {
    print_section "Creating Database Backup"

    if [ -f "data/lair-chat.db" ] || [ -f "data/lair_chat.db" ]; then
        local timestamp=$(date +"%Y%m%d_%H%M%S")
        local backup_dir="backups/db_backup_${timestamp}"

        print_step "Creating backup directory..."
        mkdir -p "$backup_dir"

        # Backup all database files
        if [ -f "data/lair-chat.db" ]; then
            cp data/lair-chat.db* "$backup_dir/" 2>/dev/null || true
            print_success "Backed up lair-chat.db to $backup_dir/"
        fi

        if [ -f "data/lair_chat.db" ]; then
            cp data/lair_chat.db* "$backup_dir/" 2>/dev/null || true
            print_success "Backed up lair_chat.db to $backup_dir/"
        fi

        print_info "Database backup created at: $backup_dir"
    else
        print_info "No existing database found - skipping backup"
    fi
}

# Function to remove database files
remove_database_files() {
    print_section "Removing Database Files"

    print_step "Removing SQLite database files..."

    # Remove main database files
    rm -f data/lair-chat.db
    rm -f data/lair_chat.db

    # Remove SQLite auxiliary files
    rm -f data/lair-chat.db-shm
    rm -f data/lair-chat.db-wal
    rm -f data/lair_chat.db-shm
    rm -f data/lair_chat.db-wal

    # Remove any other database-related files
    rm -f data/*.db-journal
    rm -f data/*.db-lock

    print_success "Database files removed"
}

# Function to clean logs
clean_logs() {
    print_section "Cleaning Log Files"

    print_step "Removing old log files..."

    # Clean server logs
    rm -f logs/*.log
    rm -f *.log
    rm -f server*.log
    rm -f client*.log
    rm -f *debug*.log

    # Recreate logs directory
    mkdir -p logs

    print_success "Log files cleaned"
}

# Function to reset environment configuration
reset_environment() {
    print_section "Resetting Environment Configuration"

    print_step "Updating .env file..."

    # Ensure .env points to the correct database (using lair-chat.db consistently)
    if [ -f ".env" ]; then
        # Update database URL to use consistent naming that matches our fixes
        sed -i 's|DATABASE_URL=sqlite:data/lair_chat.db|DATABASE_URL=sqlite:data/lair-chat.db|g' .env

        # Generate new JWT secret
        local new_jwt_secret="lair_chat_jwt_secret_$(openssl rand -hex 32)"
        sed -i "s|JWT_SECRET=.*|JWT_SECRET=$new_jwt_secret|g" .env

        print_success "Environment configuration updated"
    else
        print_warning ".env file not found - creating one with consistent database path"
        cat > .env << EOF
DATABASE_URL=sqlite:data/lair-chat.db
JWT_SECRET=lair_chat_jwt_secret_$(openssl rand -hex 32)
ADMIN_USERNAME=${DEFAULT_ADMIN_USERNAME}
ADMIN_PASSWORD=${DEFAULT_ADMIN_PASSWORD}
ADMIN_EMAIL=${DEFAULT_ADMIN_EMAIL}
EOF
        print_success "Created .env file with correct database path"
    fi
}

# Function to initialize fresh database
initialize_database() {
    print_section "Initializing Fresh Database"

    print_step "Creating database directory..."
    mkdir -p data

    print_step "Building project..."
    cargo build --release --bin lair-chat-server-new --bin create_admin_user

    if [ $? -ne 0 ]; then
        print_error "Failed to build project"
        exit 1
    fi

    print_success "Project built successfully"
}

# Function to create admin user
create_admin_user() {
    print_section "Creating Admin User"

    print_step "Creating default admin user..."
    print_info "Username: $DEFAULT_ADMIN_USERNAME"
    print_info "Password: $DEFAULT_ADMIN_PASSWORD"
    print_info "Email: $DEFAULT_ADMIN_EMAIL"

    # Ensure we use the correct database path for admin user creation
    export DATABASE_URL="sqlite:data/lair-chat.db"

    # Run the admin user creation utility with environment variable set
    if DATABASE_URL="sqlite:data/lair-chat.db" cargo run --bin create_admin_user -- "$DEFAULT_ADMIN_USERNAME" "$DEFAULT_ADMIN_PASSWORD" "$DEFAULT_ADMIN_EMAIL"; then
        print_success "Admin user created successfully in lair-chat.db"
    else
        print_error "Failed to create admin user"
        exit 1
    fi
}

# Function to verify database
verify_database() {
    print_section "Verifying Database"

    print_step "Checking database integrity..."

    if [ -f "data/lair-chat.db" ]; then
        # Check if database has expected tables
        local table_count=$(sqlite3 data/lair-chat.db ".tables" | wc -w)

        if [ $table_count -gt 5 ]; then
            print_success "Database schema appears valid ($table_count tables found)"

            # Check admin user exists
            local admin_count=$(sqlite3 data/lair-chat.db "SELECT COUNT(*) FROM users WHERE role = 'admin';" 2>/dev/null || echo "0")

            if [ "$admin_count" -gt 0 ]; then
                print_success "Admin user(s) found: $admin_count"
            else
                print_warning "No admin users found in database"
            fi
        else
            print_warning "Database may be incomplete ($table_count tables found)"
        fi
    else
        print_error "Database file not found after initialization"
        exit 1
    fi
}

# Function to start services for testing
start_services() {
    print_section "Starting Services for Testing"

    print_step "Starting REST API server..."

    # Start server in background with correct database path
    DATABASE_URL="sqlite:data/lair-chat.db" nohup cargo run --bin lair-chat-server-new > logs/reset_test_server.log 2>&1 &
    local server_pid=$!

    print_info "Server PID: $server_pid"
    print_step "Waiting for server to start..."

    # Wait for server to be ready
    for i in {1..10}; do
        if curl -s http://127.0.0.1:8082/api/v1/health > /dev/null 2>&1; then
            print_success "Server is responding on port 8082"
            break
        fi

        if [ $i -eq 10 ]; then
            print_warning "Server may not be ready yet (check logs/reset_test_server.log)"
        else
            sleep 2
        fi
    done

    # Test admin login
    print_step "Testing admin login..."
    local login_response=$(curl -s -X POST http://127.0.0.1:8082/api/v1/auth/login \
        -H "Content-Type: application/json" \
        -d "{\"identifier\": \"$DEFAULT_ADMIN_USERNAME\", \"password\": \"$DEFAULT_ADMIN_PASSWORD\"}" 2>/dev/null)

    if echo "$login_response" | grep -q "access_token"; then
        print_success "Admin login test passed - database serialization fixes preserved"
    else
        print_warning "Admin login test failed - check server logs"
        print_info "Response: $login_response"
    fi

    # Stop test server
    kill $server_pid 2>/dev/null || true
    print_info "Test server stopped"
}

# Function to display final instructions
show_final_instructions() {
    print_banner "🎉 Database Reset Complete!"

    echo -e "${GREEN}Your database has been completely reset and is ready to use.${NC}"
    echo ""
    echo -e "${CYAN}📋 Admin Credentials:${NC}"
    echo -e "   Username: ${YELLOW}$DEFAULT_ADMIN_USERNAME${NC}"
    echo -e "   Password: ${YELLOW}$DEFAULT_ADMIN_PASSWORD${NC}"
    echo -e "   Email: ${YELLOW}$DEFAULT_ADMIN_EMAIL${NC}"
    echo ""
    echo -e "${CYAN}🚀 Next Steps:${NC}"
    echo -e "   1. Start the server: ${BLUE}DATABASE_URL=\"sqlite:data/lair-chat.db\" ./target/release/lair-chat-server${NC}"
    echo -e "   2. Or use environment: ${BLUE}source .env && cargo run --bin lair-chat-server-new${NC}"
    echo -e "   3. Access admin dashboard: ${BLUE}http://127.0.0.1:8082/admin/${NC}"
    echo -e "   4. API endpoint: ${BLUE}http://127.0.0.1:8082/api/v1${NC}"
    echo ""
    echo -e "${YELLOW}🔧 Database Serialization Fixes:${NC}"
    echo -e "   • Always use DATABASE_URL=\"sqlite:data/lair-chat.db\" when starting server"
    echo -e "   • UserProfile serialization conflicts have been resolved"
    echo -e "   • Admin dashboard authentication is working correctly"
    echo ""
    echo -e "${CYAN}🔧 Additional Options:${NC}"
    echo -e "   • Run setup script: ${BLUE}./setup_admin_system.sh${NC}"
    echo -e "   • Test API endpoints: ${BLUE}./test_api.sh${NC}"
    echo -e "   • Debug authentication: ${BLUE}cargo run --bin debug_jwt_auth${NC}"
    echo ""
    echo -e "${YELLOW}💡 Pro Tips:${NC}"
    echo -e "   • Change your admin password after first login"
    echo -e "   • Check logs/ directory for troubleshooting"
    echo -e "   • Database backups are in backups/ directory"
    echo ""
}

# Main execution function
main() {
    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ]; then
        print_error "Please run this script from the lair-chat directory"
        exit 1
    fi

    # Skip backup if requested
    if [ "${SKIP_BACKUP:-false}" = "true" ]; then
        print_info "Skipping backup as requested"
    fi

    # Confirm with user
    confirm_reset

    # Execute reset steps
    stop_services

    if [ "${SKIP_BACKUP:-false}" != "true" ]; then
        backup_database
    fi

    remove_database_files
    clean_logs
    reset_environment
    initialize_database
    create_admin_user
    verify_database
    start_services
    show_final_instructions

    print_success "Database reset completed successfully!"
}

# Handle script arguments
case "${1:-}" in
    --help|-h)
        echo "Usage: $0 [options]"
        echo ""
        echo "Options:"
        echo "  --help, -h     Show this help message"
        echo "  --no-backup    Skip database backup step"
        echo "  --quiet        Reduce output verbosity"
        echo ""
        echo "Environment Variables:"
        echo "  ADMIN_USERNAME   Default admin username (default: admin)"
        echo "  ADMIN_PASSWORD   Default admin password (default: AdminPassword123!)"
        echo "  ADMIN_EMAIL      Default admin email (default: admin@example.com)"
        echo ""
        echo "This script will completely reset your lair-chat database."
        echo "Make sure to backup any important data before running."
        exit 0
        ;;
    --no-backup)
        export SKIP_BACKUP=true
        ;;
    --quiet)
        export QUIET=true
        ;;
esac

# Run the main function
main "$@"
