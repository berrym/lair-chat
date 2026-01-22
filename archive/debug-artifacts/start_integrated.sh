#!/bin/bash

# Lair Chat - Integrated Server Startup Script
# Starts both TCP chat server and REST API server together

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
TCP_PORT="${TCP_PORT:-8080}"
REST_PORT="${REST_PORT:-8082}"
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

# Cleanup function
cleanup() {
    print_warning "Shutting down integrated server..."
    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
    print_success "Integrated server stopped cleanly"
    exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

print_banner "🎯 Lair Chat - Integrated TCP + REST Server"

print_info "Starting the ultimate Lair Chat experience..."
print_info "This server combines TCP chat + REST API + Admin Dashboard"

# Check prerequisites
if [ ! -f "Cargo.toml" ]; then
    print_error "Please run this script from the lair-chat directory"
    exit 1
fi

# Create necessary directories
mkdir -p data logs

# Setup environment if not exists
if [ ! -f ".env" ]; then
    print_info "Creating environment configuration..."
    cat > .env << EOF
# Database Configuration
DATABASE_URL=sqlite:data/lair_chat.db

# JWT Configuration
JWT_SECRET=lair_chat_jwt_secret_$(openssl rand -hex 32 2>/dev/null || echo "fallback_secret_$(date +%s)")

# Server Ports
TCP_PORT=${TCP_PORT}
REST_PORT=${REST_PORT}

# Logging
RUST_LOG=info,lair_chat=debug

# Admin Configuration
ADMIN_USERNAME=${ADMIN_USERNAME}
ADMIN_PASSWORD=${ADMIN_PASSWORD}

# Features
ENABLE_ADMIN_API=true
ENABLE_WEBSOCKETS=true
ENABLE_AUDIT_LOGGING=true
EOF
    print_success "Environment configured"
fi

# Load environment
if [ -f ".env" ]; then
    set -a
    source .env
    set +a
fi

# Build if needed
if [ ! -f "target/release/lair-chat-server" ]; then
    print_info "Building integrated server (this may take a moment)..."
    cargo build --release --bin lair-chat-server > logs/build.log 2>&1
    print_success "Build complete"
fi

# Check if admin dashboard exists, create basic version if needed
if [ ! -f "admin-dashboard/index.html" ]; then
    print_warning "Admin dashboard not found, creating basic version..."
    mkdir -p admin-dashboard
    cat > admin-dashboard/index.html << 'EOF'
<!DOCTYPE html>
<html><head><title>Lair Chat Admin</title></head><body>
<h1>🛡️ Lair Chat Admin Dashboard</h1>
<p>Admin dashboard loaded from integrated server!</p>
<p><a href="/api/v1/health">API Health Check</a></p>
<p><strong>Note:</strong> Run <code>./setup_admin_system.sh</code> for the full-featured dashboard.</p>
</body></html>
EOF
    print_info "Basic admin dashboard created"
fi

print_banner "🚀 Starting Integrated Server"

print_info "Server configuration:"
print_info "  TCP Chat Port:    ${TCP_PORT}"
print_info "  REST API Port:    ${REST_PORT}"
print_info "  Admin Dashboard:  Integrated"
print_info "  Database:         SQLite"

# Start the integrated server
print_info "Launching integrated Lair Chat server..."
cargo run --release --bin lair-chat-server > logs/integrated_server.log 2>&1 &
SERVER_PID=$!

# Wait for server to be ready
print_info "Waiting for servers to be ready..."
sleep 5

# Check TCP server
TCP_READY=false
for i in {1..10}; do
    if nc -z 127.0.0.1 ${TCP_PORT} 2>/dev/null; then
        TCP_READY=true
        break
    fi
    sleep 1
    printf "."
done
echo ""

# Check REST API server
REST_READY=false
for i in {1..10}; do
    if curl -s http://127.0.0.1:${REST_PORT}/api/v1/health > /dev/null 2>&1; then
        REST_READY=true
        break
    fi
    sleep 1
    printf "."
done
echo ""

# Verify server is running
if ! ps -p $SERVER_PID > /dev/null 2>&1; then
    print_error "Server failed to start. Check logs/integrated_server.log"
    tail -20 logs/integrated_server.log
    exit 1
fi

if [ "$TCP_READY" = true ]; then
    print_success "TCP Chat server is ready"
else
    print_warning "TCP Chat server may not be ready"
fi

if [ "$REST_READY" = true ]; then
    print_success "REST API server is ready"
else
    print_warning "REST API server may not be ready"
fi

print_banner "🎉 Lair Chat Integrated Server is Running!"

echo -e "${CYAN}📊 Access Your Services:${NC}"
echo -e "   🔌 TCP Chat (telnet):    ${GREEN}telnet 127.0.0.1 ${TCP_PORT}${NC}"
echo -e "   🌐 Admin Dashboard:      ${GREEN}http://127.0.0.1:${REST_PORT}/admin/${NC}"
echo -e "   🔗 REST API:            ${GREEN}http://127.0.0.1:${REST_PORT}/api/v1${NC}"
echo -e "   📚 API Documentation:   ${GREEN}http://127.0.0.1:${REST_PORT}/docs${NC}"
echo -e "   ❤️  Server Info:         ${GREEN}http://127.0.0.1:${REST_PORT}/${NC}"
echo -e "   ⚡ Health Check:        ${GREEN}http://127.0.0.1:${REST_PORT}/api/v1/health${NC}"

echo -e "\n${CYAN}🔐 Admin Credentials:${NC}"
echo -e "   Username: ${YELLOW}${ADMIN_USERNAME}${NC}"
echo -e "   Password: ${YELLOW}${ADMIN_PASSWORD}${NC}"

echo -e "\n${CYAN}🛠️  Usage Examples:${NC}"
echo -e "   TCP Chat:         ${YELLOW}telnet 127.0.0.1 ${TCP_PORT}${NC}"
echo -e "   API Test:         ${YELLOW}curl http://127.0.0.1:${REST_PORT}/api/v1/health${NC}"
echo -e "   View Logs:        ${YELLOW}tail -f logs/integrated_server.log${NC}"
echo -e "   Admin Login:      ${YELLOW}curl -X POST http://127.0.0.1:${REST_PORT}/api/v1/auth/login \\${NC}"
echo -e "                     ${YELLOW}  -H 'Content-Type: application/json' \\${NC}"
echo -e "                     ${YELLOW}  -d '{\"identifier\":\"admin\",\"password\":\"AdminPassword123!\"}'${NC}"

echo -e "\n${CYAN}✨ Integrated Features:${NC}"
echo -e "   ✅ TCP-based chat client support"
echo -e "   ✅ REST API with JWT authentication"
echo -e "   ✅ Admin dashboard (web interface)"
echo -e "   ✅ Real-time messaging"
echo -e "   ✅ User & room management"
echo -e "   ✅ Role-based access control"
echo -e "   ✅ SQLite database"
echo -e "   ✅ Health monitoring"
echo -e "   ✅ Audit logging"

echo -e "\n${CYAN}💡 Pro Tips:${NC}"
echo -e "   • Use a telnet client to connect to the TCP chat"
echo -e "   • Use the web dashboard for administration"
echo -e "   • Use the REST API for integrations"
echo -e "   • Both servers share the same database"
echo -e "   • Admin users can manage both TCP and web users"

echo -e "\n${GREEN}🎯 Ready! Both TCP and REST servers are running.${NC}"
echo -e "${PURPLE}Press Ctrl+C to stop both servers${NC}\n"

# Keep running and show basic monitoring
print_info "Monitoring integrated server... (Press Ctrl+C to stop)"
while true; do
    sleep 15

    # Check if server is still running
    if ! ps -p $SERVER_PID > /dev/null 2>&1; then
        print_error "Integrated server has stopped unexpectedly!"
        print_info "Check logs/integrated_server.log for details:"
        tail -10 logs/integrated_server.log 2>/dev/null || echo "No log file found"
        exit 1
    fi

    # Show heartbeat for both services
    printf "${BLUE}[$(date '+%H:%M:%S')]${NC} "

    # TCP check
    if nc -z 127.0.0.1 ${TCP_PORT} 2>/dev/null; then
        printf "${GREEN}TCP:✓${NC} "
    else
        printf "${RED}TCP:✗${NC} "
    fi

    # REST check
    if curl -s http://127.0.0.1:${REST_PORT}/api/v1/health > /dev/null 2>&1; then
        printf "${GREEN}REST:✓${NC} "
    else
        printf "${RED}REST:✗${NC} "
    fi

    echo ""
done
