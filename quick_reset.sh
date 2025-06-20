#!/bin/bash

# Quick Database Reset Script for Lair Chat
# This is a fast, no-frills database reset

set -e

echo "🔄 Quick Database Reset"
echo "======================"

# Stop any running processes
echo "Stopping services..."
pkill -f lair-chat-server 2>/dev/null || true
pkill -f quick_start.sh 2>/dev/null || true
pkill -f "python.*http.server" 2>/dev/null || true
sleep 1

# Remove database files
echo "Removing database files..."
rm -f data/lair-chat.db*
rm -f data/lair_chat.db*

# Clean logs
echo "Cleaning logs..."
rm -f logs/*.log *.log 2>/dev/null || true
mkdir -p logs data

# Build project
echo "Building project..."
cargo build --release --bin lair-chat-server-new --bin create_admin_user

# Create admin user
echo "Creating admin user..."
cargo run --bin create_admin_user

echo ""
echo "✅ Database reset complete!"
echo ""
echo "🔐 Default Admin Credentials:"
echo "   Username: admin"
echo "   Password: AdminPassword123!"
echo ""
echo "🚀 Start the server:"
echo "   ./quick_start.sh"
echo ""
