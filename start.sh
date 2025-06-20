#!/bin/bash

# Lair Chat - Ultimate One-Command Startup Script
# The one command to rule them all! 🎯

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
ADMIN_USERNAME="${ADMIN_USERNAME:-admin}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-AdminPassword123!}"
SERVER_PORT="${SERVER_PORT:-8082}"

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
    print_warning "Shutting down all services..."
    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
    print_success "Lair Chat stopped cleanly"
    exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

print_banner "🚀 Lair Chat - One Command To Rule Them All!"

print_info "Starting ultimate Lair Chat experience..."

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
DATABASE_URL=sqlite:data/lair_chat.db
JWT_SECRET=lair_chat_jwt_secret_$(openssl rand -hex 32 2>/dev/null || echo "fallback_secret_$(date +%s)")
SERVER_HOST=127.0.0.1
SERVER_PORT=${SERVER_PORT}
RUST_LOG=info,lair_chat=debug
ENABLE_ADMIN_API=true
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
if [ ! -f "target/release/lair-chat-server-new" ]; then
    print_info "Building Lair Chat server (this may take a moment)..."
    cargo build --release --bin lair-chat-server-new --bin create_admin_user > logs/build.log 2>&1
    print_success "Build complete"
fi

# Create admin user if needed
print_info "Ensuring admin user exists..."
cargo run --release --bin create_admin_user > logs/admin_setup.log 2>&1 || true

# Check if admin dashboard exists, create if needed
if [ ! -f "admin-dashboard/index.html" ]; then
    print_warning "Admin dashboard not found, creating basic version..."
    mkdir -p admin-dashboard
    cat > admin-dashboard/index.html << 'EOF'
<!DOCTYPE html>
<html><head><title>Lair Chat Admin</title></head><body>
<h1>🛡️ Lair Chat Admin</h1>
<p>Admin dashboard is being set up. Please run <code>./setup_admin_system.sh</code> for full features.</p>
<p><a href="/api/v1/health">API Health Check</a></p>
</body></html>
EOF
    print_info "Basic admin dashboard created"
fi

print_banner "🎯 Starting Lair Chat Server"

# Start the server
print_info "Launching Lair Chat REST API server..."
cargo run --release --bin lair-chat-server-new > logs/server.log 2>&1 &
SERVER_PID=$!

# Wait for server to be ready
print_info "Waiting for server to be ready..."
for i in {1..30}; do
    if curl -s http://127.0.0.1:${SERVER_PORT}/api/v1/health > /dev/null 2>&1; then
        break
    fi
    if [ $i -eq 30 ]; then
        print_error "Server failed to start. Check logs/server.log"
        cleanup
        exit 1
    fi
    sleep 1
    printf "."
done
echo ""

print_success "Server is ready!"

print_banner "🎉 Lair Chat is Running!"

echo -e "${CYAN}📊 Access Your Lair Chat System:${NC}"
echo -e "   🌐 Admin Dashboard:      ${GREEN}http://127.0.0.1:${SERVER_PORT}/admin/${NC}"
echo -e "   🔗 REST API:            ${GREEN}http://127.0.0.1:${SERVER_PORT}/api/v1${NC}"
echo -e "   📚 API Documentation:   ${GREEN}http://127.0.0.1:${SERVER_PORT}/docs${NC}"
echo -e "   ❤️  Server Info:         ${GREEN}http://127.0.0.1:${SERVER_PORT}/${NC}"

echo -e "\n${CYAN}🔐 Admin Credentials:${NC}"
echo -e "   Username: ${YELLOW}${ADMIN_USERNAME}${NC}"
echo -e "   Password: ${YELLOW}${ADMIN_PASSWORD}${NC}"

echo -e "\n${CYAN}🛠️  Quick Commands:${NC}"
echo -e "   Test API:     ${YELLOW}curl http://127.0.0.1:${SERVER_PORT}/api/v1/health${NC}"
echo -e "   View logs:    ${YELLOW}tail -f logs/server.log${NC}"
echo -e "   Setup full:   ${YELLOW}./setup_admin_system.sh${NC}"

echo -e "\n${CYAN}✨ Features Available:${NC}"
echo -e "   ✅ REST API with JWT authentication"
echo -e "   ✅ Admin dashboard (integrated)"
echo -e "   ✅ User & room management"
echo -e "   ✅ Real-time messaging"
echo -e "   ✅ Role-based access control"
echo -e "   ✅ SQLite database with 15+ tables"
echo -e "   ✅ Health monitoring & audit logs"

echo -e "\n${GREEN}🎯 Ready! Open your browser and go to: ${BLUE}http://127.0.0.1:${SERVER_PORT}/${NC}"
echo -e "${PURPLE}Press Ctrl+C to stop all services${NC}\n"

# Keep running and show basic monitoring
print_info "Monitoring server... (Press Ctrl+C to stop)"
while true; do
    sleep 10

    # Check if server is still running
    if ! ps -p $SERVER_PID > /dev/null 2>&1; then
        print_error "Server has stopped unexpectedly!"
        print_info "Check logs/server.log for details:"
        tail -5 logs/server.log 2>/dev/null || echo "No log file found"
        exit 1
    fi

    # Optional: Show a heartbeat
    if curl -s http://127.0.0.1:${SERVER_PORT}/api/v1/health > /dev/null 2>&1; then
        printf "${GREEN}.${NC}"
    else
        printf "${RED}!${NC}"
    fi
done
