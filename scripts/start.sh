#!/bin/bash

# Lair Chat - Production Start Script
# Starts the complete Lair Chat system with TCP and REST API servers

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration with sensible defaults
SERVER_HOST="${SERVER_HOST:-127.0.0.1}"
SERVER_PORT="${SERVER_PORT:-8082}"
TCP_PORT="${TCP_PORT:-8080}"
ADMIN_USERNAME="${ADMIN_USERNAME:-admin}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-AdminPassword123!}"

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

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Cleanup function for graceful shutdown
cleanup() {
    print_warning "Shutting down Lair Chat system..."
    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
    if [ ! -z "$TCP_SERVER_PID" ]; then
        kill $TCP_SERVER_PID 2>/dev/null || true
        wait $TCP_SERVER_PID 2>/dev/null || true
    fi
    print_success "Lair Chat stopped cleanly"
    exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

print_banner "🚀 Lair Chat - Production Server"

# Validate prerequisites
if [ ! -f "Cargo.toml" ]; then
    print_error "Please run this script from the lair-chat directory"
    exit 1
fi

# Check for Rust installation
if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo not found. Please install from https://rustup.rs/"
    exit 1
fi

print_info "Starting Lair Chat production environment..."

# Create necessary directories
mkdir -p data logs

# Setup environment configuration
if [ ! -f ".env" ]; then
    print_info "Creating production environment configuration..."
    cat > .env << EOF
# Database Configuration
DATABASE_URL=sqlite:data/lair_chat.db

# Server Configuration
SERVER_HOST=${SERVER_HOST}
SERVER_PORT=${SERVER_PORT}
TCP_PORT=${TCP_PORT}

# Security Configuration
JWT_SECRET=lair_chat_jwt_secret_$(openssl rand -hex 32 2>/dev/null || echo "fallback_secret_$(date +%s)")
ENABLE_ENCRYPTION=true

# Feature Flags
ENABLE_ADMIN_API=true
ENABLE_AUDIT_LOGGING=true
ENABLE_WEBSOCKETS=true

# Logging Configuration
RUST_LOG=info,lair_chat=debug

# Admin Configuration
ADMIN_USERNAME=${ADMIN_USERNAME}
ADMIN_PASSWORD=${ADMIN_PASSWORD}
EOF
    print_success "Environment configuration created"
fi

# Load environment variables
if [ -f ".env" ]; then
    set -a
    source .env
    set +a
fi

# Build production binaries if needed
if [ ! -f "target/release/lair-chat-server-new" ] || [ ! -f "target/release/lair-chat-server" ]; then
    print_info "Building Lair Chat (this may take a few minutes)..."
    cargo build --release --bin lair-chat-server-new --bin lair-chat-server --bin create_admin_user > logs/build.log 2>&1
    if [ $? -eq 0 ]; then
        print_success "Build completed successfully"
    else
        print_error "Build failed. Check logs/build.log for details"
        exit 1
    fi
fi

# Create admin user if needed
print_info "Ensuring admin user exists..."
cargo run --release --bin create_admin_user > logs/admin_setup.log 2>&1 || true

# Check and setup admin dashboard
if [ ! -f "admin-dashboard/index.html" ]; then
    print_warning "Admin dashboard not found. Creating basic version..."
    mkdir -p admin-dashboard
    cat > admin-dashboard/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Lair Chat - Admin Dashboard</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, sans-serif; margin: 40px; background: #f5f7fa; }
        .container { max-width: 600px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        h1 { color: #34495e; }
        .service { margin: 20px 0; padding: 15px; background: #ecf0f1; border-radius: 5px; }
        .service h3 { margin: 0 0 10px 0; color: #2c3e50; }
        a { color: #3498db; text-decoration: none; }
        a:hover { text-decoration: underline; }
        .status { display: inline-block; width: 12px; height: 12px; background: #27ae60; border-radius: 50%; margin-right: 8px; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🛡️ Lair Chat Admin Dashboard</h1>
        <p>Welcome to Lair Chat administration interface.</p>

        <div class="service">
            <h3><span class="status"></span>REST API</h3>
            <p>RESTful API for chat operations</p>
            <a href="/api/v1/health">→ API Health Check</a>
        </div>

        <div class="service">
            <h3><span class="status"></span>API Documentation</h3>
            <p>Interactive API documentation</p>
            <a href="/docs">→ View API Docs</a>
        </div>

        <hr style="margin: 30px 0; border: none; border-top: 1px solid #ecf0f1;">

        <p><strong>Default Admin Credentials:</strong></p>
        <ul>
            <li>Username: <code>admin</code></li>
            <li>Password: <code>AdminPassword123!</code></li>
        </ul>

        <p><em>For the full-featured dashboard, ensure the admin-dashboard directory contains the complete web interface.</em></p>
    </div>
</body>
</html>
EOF
    print_info "Basic admin dashboard created"
fi

print_banner "🎯 Starting Lair Chat Servers"

# Start the REST API server
print_info "Starting REST API server on ${SERVER_HOST}:${SERVER_PORT}..."
cargo run --release --bin lair-chat-server-new > logs/rest_server.log 2>&1 &
SERVER_PID=$!

# Start the TCP server
print_info "Starting TCP chat server on ${SERVER_HOST}:${TCP_PORT}..."
cargo run --release --bin lair-chat-server > logs/tcp_server.log 2>&1 &
TCP_SERVER_PID=$!

# Wait for servers to initialize
print_info "Waiting for servers to initialize..."
sleep 5

# Health checks
REST_READY=false
TCP_READY=false

# Check REST API server
for i in {1..15}; do
    if curl -s http://${SERVER_HOST}:${SERVER_PORT}/api/v1/health > /dev/null 2>&1; then
        REST_READY=true
        break
    fi
    sleep 1
    printf "."
done
echo ""

# Check TCP server
for i in {1..10}; do
    if nc -z ${SERVER_HOST} ${TCP_PORT} 2>/dev/null; then
        TCP_READY=true
        break
    fi
    sleep 1
    printf "."
done
echo ""

# Verify servers are running
if ! ps -p $SERVER_PID > /dev/null 2>&1; then
    print_error "REST API server failed to start. Check logs/rest_server.log"
    cleanup
    exit 1
fi

if ! ps -p $TCP_SERVER_PID > /dev/null 2>&1; then
    print_error "TCP server failed to start. Check logs/tcp_server.log"
    cleanup
    exit 1
fi

# Report server status
if [ "$REST_READY" = true ]; then
    print_success "REST API server is ready"
else
    print_warning "REST API server may not be fully ready"
fi

if [ "$TCP_READY" = true ]; then
    print_success "TCP chat server is ready"
else
    print_warning "TCP chat server may not be fully ready"
fi

print_banner "🎉 Lair Chat System Ready!"

echo -e "${CYAN}📊 Access Points:${NC}"
echo -e "   🌐 Admin Dashboard:      ${GREEN}http://${SERVER_HOST}:${SERVER_PORT}/admin/${NC}"
echo -e "   🔗 REST API:            ${GREEN}http://${SERVER_HOST}:${SERVER_PORT}/api/v1${NC}"
echo -e "   📚 API Documentation:   ${GREEN}http://${SERVER_HOST}:${SERVER_PORT}/docs${NC}"
echo -e "   🔌 TCP Chat:            ${GREEN}telnet ${SERVER_HOST} ${TCP_PORT}${NC}"
echo -e "   ❤️  Health Check:        ${GREEN}http://${SERVER_HOST}:${SERVER_PORT}/api/v1/health${NC}"

echo -e "\n${CYAN}🔐 Admin Credentials:${NC}"
echo -e "   Username: ${YELLOW}${ADMIN_USERNAME}${NC}"
echo -e "   Password: ${YELLOW}${ADMIN_PASSWORD}${NC}"

echo -e "\n${CYAN}🛠️  Quick Commands:${NC}"
echo -e "   Connect TUI Client:   ${YELLOW}cargo run --bin lair-chat-client${NC}"
echo -e "   Test API:            ${YELLOW}curl http://${SERVER_HOST}:${SERVER_PORT}/api/v1/health${NC}"
echo -e "   View REST Logs:      ${YELLOW}tail -f logs/rest_server.log${NC}"
echo -e "   View TCP Logs:       ${YELLOW}tail -f logs/tcp_server.log${NC}"

echo -e "\n${CYAN}✨ Features Available:${NC}"
echo -e "   ✅ REST API with JWT authentication"
echo -e "   ✅ TCP chat server for TUI clients"
echo -e "   ✅ Web-based admin dashboard"
echo -e "   ✅ Real-time messaging"
echo -e "   ✅ User & room management"
echo -e "   ✅ Role-based access control"
echo -e "   ✅ Database persistence"
echo -e "   ✅ Health monitoring"
echo -e "   ✅ Audit logging"

echo -e "\n${GREEN}🎯 System is ready for production use!${NC}"
echo -e "${PURPLE}Press Ctrl+C to shutdown all servers${NC}\n"

# Keep running and monitor servers
print_info "Monitoring servers... (Press Ctrl+C to stop)"
while true; do
    sleep 30

    # Check if servers are still running
    if ! ps -p $SERVER_PID > /dev/null 2>&1; then
        print_error "REST API server has stopped unexpectedly!"
        print_info "Check logs/rest_server.log for details"
        cleanup
        exit 1
    fi

    if ! ps -p $TCP_SERVER_PID > /dev/null 2>&1; then
        print_error "TCP server has stopped unexpectedly!"
        print_info "Check logs/tcp_server.log for details"
        cleanup
        exit 1
    fi

    # Show health status
    printf "${BLUE}[$(date '+%H:%M:%S')]${NC} "

    # REST API health check
    if curl -s http://${SERVER_HOST}:${SERVER_PORT}/api/v1/health > /dev/null 2>&1; then
        printf "${GREEN}REST:✓${NC} "
    else
        printf "${RED}REST:✗${NC} "
    fi

    # TCP server health check
    if nc -z ${SERVER_HOST} ${TCP_PORT} 2>/dev/null; then
        printf "${GREEN}TCP:✓${NC}"
    else
        printf "${RED}TCP:✗${NC}"
    fi

    echo ""
done
