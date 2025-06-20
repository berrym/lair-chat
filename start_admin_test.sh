#!/bin/bash

# Lair Chat Admin Dashboard Test Script
# This script sets up and starts the admin dashboard testing environment

set -e

echo "🚀 Lair Chat Admin Dashboard Test Setup"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
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

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    print_error "Cargo not found. Please install Rust first."
    exit 1
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Cargo.toml not found. Please run this script from the lair-chat directory."
    exit 1
fi

print_status "Building project..."
cargo build --release --bin create_admin_user --bin lair-chat-server-new

# Create admin user
print_status "Creating admin user..."
cargo run --bin create_admin_user

print_status "Setting up environment..."

# Create .env file if it doesn't exist
if [ ! -f ".env" ]; then
    print_status "Creating .env file..."
    cat > .env << EOF
# Database Configuration
DATABASE_URL=sqlite:data/lair_chat.db

# JWT Configuration
JWT_SECRET=your_super_secure_jwt_secret_key_here_make_it_long_and_random

# Server Configuration
SERVER_HOST=127.0.0.1
SERVER_PORT=8082

# Logging
RUST_LOG=info,lair_chat=debug

# Security
BCRYPT_COST=12
SESSION_TIMEOUT_HOURS=24
EOF
    print_success ".env file created with default configuration"
else
    print_status ".env file already exists"
fi

# Create data directory
mkdir -p data
mkdir -p logs

# Check if admin dashboard exists
if [ ! -f "admin-dashboard/index.html" ]; then
    print_error "Admin dashboard not found at admin-dashboard/index.html"
    print_error "Please ensure the admin dashboard files are present."
    exit 1
fi

print_success "Setup complete!"
echo ""
print_status "Starting services..."

# Function to handle cleanup
cleanup() {
    print_warning "Shutting down services..."
    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
    fi
    if [ ! -z "$HTTP_PID" ]; then
        kill $HTTP_PID 2>/dev/null || true
    fi
    exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

# Start the Lair Chat server in the background
print_status "Starting Lair Chat REST API server on port 8082..."
cargo run --release --bin lair-chat-server-new > logs/server.log 2>&1 &
SERVER_PID=$!

# Wait a moment for the server to start
sleep 3

# Check if server is running
if ! ps -p $SERVER_PID > /dev/null; then
    print_error "Failed to start Lair Chat server. Check logs/server.log for details."
    exit 1
fi

print_success "Lair Chat server started (PID: $SERVER_PID)"

# Start a simple HTTP server for the admin dashboard
print_status "Starting HTTP server for admin dashboard on port 8083..."
if command -v python3 &> /dev/null; then
    cd admin-dashboard
    python3 -m http.server 8083 > ../logs/http_server.log 2>&1 &
    HTTP_PID=$!
    cd ..
elif command -v python &> /dev/null; then
    cd admin-dashboard
    python -m SimpleHTTPServer 8083 > ../logs/http_server.log 2>&1 &
    HTTP_PID=$!
    cd ..
else
    print_warning "Python not found. You'll need to serve the admin dashboard manually."
    print_warning "You can use any HTTP server to serve the admin-dashboard/ directory on port 8083"
fi

if [ ! -z "$HTTP_PID" ]; then
    print_success "HTTP server started for admin dashboard (PID: $HTTP_PID)"
fi

echo ""
print_success "🎉 Admin Dashboard Test Environment Ready!"
echo ""
echo "📋 Access Information:"
echo "   • Admin Dashboard: http://127.0.0.1:8083"
echo "   • REST API: http://127.0.0.1:8082/api/v1"
echo "   • API Documentation: http://127.0.0.1:8082/swagger-ui"
echo ""
echo "🔐 Default Admin Credentials:"
echo "   • Username: admin"
echo "   • Password: AdminPassword123!"
echo ""
echo "📁 Log Files:"
echo "   • Server logs: logs/server.log"
echo "   • HTTP server logs: logs/http_server.log"
echo ""
echo "🛠️  API Testing:"
echo "   • Test connection: curl http://127.0.0.1:8082/api/v1/health"
echo "   • Login: curl -X POST http://127.0.0.1:8082/api/v1/auth/login \\"
echo "            -H 'Content-Type: application/json' \\"
echo "            -d '{\"identifier\":\"admin\",\"password\":\"AdminPassword123!\"}'"
echo ""
print_status "Services are running. Press Ctrl+C to stop all services."
echo ""

# Keep the script running
while true; do
    sleep 1

    # Check if server is still running
    if ! ps -p $SERVER_PID > /dev/null; then
        print_error "Lair Chat server has stopped unexpectedly!"
        print_error "Check logs/server.log for details."
        break
    fi

    # Check if HTTP server is still running (if it was started)
    if [ ! -z "$HTTP_PID" ] && ! ps -p $HTTP_PID > /dev/null; then
        print_warning "HTTP server has stopped unexpectedly!"
        break
    fi
done

# Cleanup
cleanup
