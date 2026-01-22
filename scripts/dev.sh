#!/bin/bash

# Lair Chat - Development Start Script
# Quick development environment setup with hot-reloading and debugging

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Development configuration
DEV_HOST="${DEV_HOST:-127.0.0.1}"
DEV_API_PORT="${DEV_API_PORT:-8082}"
DEV_TCP_PORT="${DEV_TCP_PORT:-8080}"
DEV_ADMIN_USERNAME="${DEV_ADMIN_USERNAME:-admin}"
DEV_ADMIN_PASSWORD="${DEV_ADMIN_PASSWORD:-DevPassword123!}"

# Function to print colored output
print_banner() {
    echo -e "\n${PURPLE}============================================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}============================================================${NC}\n"
}

print_info() {
    echo -e "${BLUE}[DEV]${NC} $1"
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

# Cleanup function
cleanup() {
    print_warning "Shutting down development environment..."
    jobs -p | xargs -r kill 2>/dev/null || true
    print_success "Development environment stopped"
    exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

print_banner "🔧 Lair Chat - Development Environment"

# Validate prerequisites
if [ ! -f "Cargo.toml" ]; then
    print_error "Please run this script from the lair-chat directory"
    exit 1
fi

# Check for required tools
if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo not found. Please install from https://rustup.rs/"
    exit 1
fi

if ! command -v cargo-watch &> /dev/null; then
    print_info "Installing cargo-watch for hot-reloading..."
    cargo install cargo-watch
fi

print_info "Setting up development environment..."

# Create development directories
mkdir -p data logs dev-logs

# Setup development environment
print_info "Creating development environment configuration..."
cat > .env.dev << EOF
# Development Database Configuration
DATABASE_URL=sqlite:data/lair_chat_dev.db

# Development Server Configuration
SERVER_HOST=${DEV_HOST}
SERVER_PORT=${DEV_API_PORT}
TCP_PORT=${DEV_TCP_PORT}

# Development Security Configuration
JWT_SECRET=dev_jwt_secret_$(date +%s)
ENABLE_ENCRYPTION=true

# Development Feature Flags
ENABLE_ADMIN_API=true
ENABLE_AUDIT_LOGGING=true
ENABLE_WEBSOCKETS=true
ENABLE_DEBUG_ENDPOINTS=true

# Development Logging Configuration
RUST_LOG=debug,lair_chat=trace,sqlx=debug
RUST_BACKTRACE=1

# Development Admin Configuration
ADMIN_USERNAME=${DEV_ADMIN_USERNAME}
ADMIN_PASSWORD=${DEV_ADMIN_PASSWORD}

# Development-specific settings
CORS_ALLOW_ALL=true
RATE_LIMIT_DISABLED=true
AUTH_TOKEN_EXPIRY=86400
EOF

# Load development environment
set -a
source .env.dev
set +a

print_success "Development environment configured"

# Quick build check
print_info "Checking build status..."
if ! cargo check --all-targets > dev-logs/check.log 2>&1; then
    print_error "Build check failed. Fix compilation errors first:"
    tail -20 dev-logs/check.log
    exit 1
fi

print_success "Build check passed"

# Create admin user for development
print_info "Setting up development admin user..."
cargo run --bin create_admin_user > dev-logs/admin_setup.log 2>&1 || true

print_banner "🚀 Starting Development Servers"

# Start REST API server with hot-reload
print_info "Starting REST API server with hot-reload..."
cargo watch -x "run --bin lair-chat-server-new" --ignore "*.log" --ignore "target/*" > dev-logs/rest_api.log 2>&1 &
REST_PID=$!

# Give REST API time to start
sleep 3

# Start TCP server with hot-reload
print_info "Starting TCP server with hot-reload..."
cargo watch -x "run --bin lair-chat-server" --ignore "*.log" --ignore "target/*" > dev-logs/tcp_server.log 2>&1 &
TCP_PID=$!

# Wait for servers to initialize
print_info "Waiting for servers to initialize..."
sleep 5

# Check server status
REST_READY=false
TCP_READY=false

print_info "Checking server health..."

# Check REST API
for i in {1..10}; do
    if curl -s http://${DEV_HOST}:${DEV_API_PORT}/api/v1/health > /dev/null 2>&1; then
        REST_READY=true
        break
    fi
    sleep 1
done

# Check TCP server
for i in {1..5}; do
    if nc -z ${DEV_HOST} ${DEV_TCP_PORT} 2>/dev/null; then
        TCP_READY=true
        break
    fi
    sleep 1
done

print_banner "🎉 Development Environment Ready!"

# Show status
if [ "$REST_READY" = true ]; then
    print_success "REST API server running (hot-reload enabled)"
else
    print_warning "REST API server may not be ready (check dev-logs/rest_api.log)"
fi

if [ "$TCP_READY" = true ]; then
    print_success "TCP server running (hot-reload enabled)"
else
    print_warning "TCP server may not be ready (check dev-logs/tcp_server.log)"
fi

echo -e "\n${CYAN}🔗 Development Access Points:${NC}"
echo -e "   🌐 Admin Dashboard:      ${GREEN}http://${DEV_HOST}:${DEV_API_PORT}/admin/${NC}"
echo -e "   🔗 REST API:            ${GREEN}http://${DEV_HOST}:${DEV_API_PORT}/api/v1${NC}"
echo -e "   📚 API Documentation:   ${GREEN}http://${DEV_HOST}:${DEV_API_PORT}/docs${NC}"
echo -e "   🔌 TCP Chat:            ${GREEN}telnet ${DEV_HOST} ${DEV_TCP_PORT}${NC}"
echo -e "   ❤️  Health Check:        ${GREEN}http://${DEV_HOST}:${DEV_API_PORT}/api/v1/health${NC}"

echo -e "\n${CYAN}🔐 Development Credentials:${NC}"
echo -e "   Username: ${YELLOW}${DEV_ADMIN_USERNAME}${NC}"
echo -e "   Password: ${YELLOW}${DEV_ADMIN_PASSWORD}${NC}"

echo -e "\n${CYAN}🛠️  Development Commands:${NC}"
echo -e "   Run TUI Client:      ${YELLOW}cargo run --bin lair-chat-client${NC}"
echo -e "   Run Tests:           ${YELLOW}cargo test${NC}"
echo -e "   Run Clippy:          ${YELLOW}cargo clippy${NC}"
echo -e "   Format Code:         ${YELLOW}cargo fmt${NC}"
echo -e "   Check Build:         ${YELLOW}cargo check${NC}"

echo -e "\n${CYAN}📋 Development Logs:${NC}"
echo -e "   REST API Logs:       ${YELLOW}tail -f dev-logs/rest_api.log${NC}"
echo -e "   TCP Server Logs:     ${YELLOW}tail -f dev-logs/tcp_server.log${NC}"
echo -e "   Combined Logs:       ${YELLOW}tail -f dev-logs/*.log${NC}"

echo -e "\n${CYAN}🔄 Hot-Reload Features:${NC}"
echo -e "   ✅ Automatic rebuild on source changes"
echo -e "   ✅ Server restart on successful compilation"
echo -e "   ✅ Debug logging enabled"
echo -e "   ✅ CORS disabled for development"
echo -e "   ✅ Extended token expiry"
echo -e "   ✅ Rate limiting disabled"

echo -e "\n${CYAN}🧪 Quick Development Tests:${NC}"
echo -e "   API Health:          ${YELLOW}curl http://${DEV_HOST}:${DEV_API_PORT}/api/v1/health${NC}"
echo -e "   Register User:       ${YELLOW}curl -X POST http://${DEV_HOST}:${DEV_API_PORT}/api/v1/auth/register -H 'Content-Type: application/json' -d '{\"username\":\"testuser\",\"email\":\"test@example.com\",\"password\":\"TestPass123!\"}'${NC}"
echo -e "   Login:               ${YELLOW}curl -X POST http://${DEV_HOST}:${DEV_API_PORT}/api/v1/auth/login -H 'Content-Type: application/json' -d '{\"identifier\":\"admin\",\"password\":\"${DEV_ADMIN_PASSWORD}\"}'${NC}"

echo -e "\n${GREEN}🎯 Development environment is ready!${NC}"
echo -e "${BLUE}💡 Code changes will automatically trigger server restarts${NC}"
echo -e "${PURPLE}Press Ctrl+C to stop all development servers${NC}\n"

# Monitor and keep alive
print_info "Monitoring development environment... (Press Ctrl+C to stop)"
while true; do
    sleep 10

    # Show periodic status
    printf "${BLUE}[$(date '+%H:%M:%S')] DEV STATUS:${NC} "

    # Check REST API
    if curl -s http://${DEV_HOST}:${DEV_API_PORT}/api/v1/health > /dev/null 2>&1; then
        printf "${GREEN}REST:✓${NC} "
    else
        printf "${RED}REST:✗${NC} "
    fi

    # Check TCP server
    if nc -z ${DEV_HOST} ${DEV_TCP_PORT} 2>/dev/null; then
        printf "${GREEN}TCP:✓${NC}"
    else
        printf "${RED}TCP:✗${NC}"
    fi

    echo ""
done
