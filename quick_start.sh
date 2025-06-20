#!/bin/bash
# Quick start script for Lair Chat Admin System

echo "🚀 Starting Lair Chat Admin System..."

# Ensure correct database path to preserve serialization fixes
export DATABASE_URL="sqlite:data/lair-chat.db"

# Start server
echo "Starting REST API server with fixed database configuration..."
DATABASE_URL="sqlite:data/lair-chat.db" cargo run --release --bin lair-chat-server-new > logs/server.log 2>&1 &
SERVER_PID=$!

# Wait for server to start
sleep 3

# Start dashboard server
echo "Starting admin dashboard..."
if command -v python3 &> /dev/null; then
    cd admin-dashboard && python3 -m http.server 8083 > ../logs/dashboard.log 2>&1 &
    DASHBOARD_PID=$!
    cd ..
fi

echo "✅ System started!"
echo "📊 Admin Dashboard: http://127.0.0.1:8083"
echo "🔗 API Endpoint: http://127.0.0.1:8082/api/v1"
echo "📚 API Docs: http://127.0.0.1:8082/swagger-ui"
echo ""
echo "🔐 Admin Credentials:"
echo "   Username: admin"
echo "   Password: AdminPassword123!"
echo ""
echo "🔧 Database: Using lair-chat.db (serialization fixes applied)"
echo ""
echo "Press Ctrl+C to stop all services"

# Cleanup function
cleanup() {
    echo "Stopping services..."
    kill $SERVER_PID 2>/dev/null || true
    kill $DASHBOARD_PID 2>/dev/null || true
    exit 0
}

trap cleanup SIGINT SIGTERM

# Keep running
while true; do
    sleep 1
done
