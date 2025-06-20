#!/bin/bash

# Real-time Invitation Test Script
# This script provides step-by-step manual testing with real-time monitoring

echo "🔬 REAL-TIME INVITATION TEST"
echo "============================"

# Clean up any existing processes
echo "🧹 Cleaning up existing processes..."
pkill -f lair-chat-server
pkill -f lair-chat-client
sleep 2

# Start server with focused logging
echo "🚀 Starting server..."
RUST_LOG=info cargo run --bin lair-chat-server > server_realtime.log 2>&1 &
SERVER_PID=$!
echo "Server started with PID: $SERVER_PID"
sleep 3

echo ""
echo "📋 MANUAL TEST INSTRUCTIONS:"
echo "============================="
echo ""
echo "Step 1: Open TWO new terminal windows"
echo "Step 2: In Terminal 1 (Alice):"
echo "   cd $(pwd)"
echo "   cargo run --bin lair-chat-client"
echo "   Login: alice / password123"
echo "   Type: /create-room gameroom"
echo "   Type: /invite bob gameroom"
echo ""
echo "Step 3: In Terminal 2 (Bob):"
echo "   cd $(pwd)"
echo "   cargo run --bin lair-chat-client"
echo "   Login: bob / password123"
echo "   Watch for invitation message"
echo ""
echo "This script will monitor server logs in real-time..."
echo "Press Ctrl+C when done testing"
echo ""

# Function to monitor logs
monitor_logs() {
    echo "🔍 MONITORING SERVER LOGS:"
    echo "=========================="
    echo "Watching for invitation activity..."
    echo ""

    tail -f server_realtime.log | while read line; do
        # Highlight important events
        if echo "$line" | grep -q "INVITE_USER\|invite\|Invitation\|Processing invitation\|send_to_user"; then
            echo "🔥 INVITATION: $line"
        elif echo "$line" | grep -q "ERROR\|error\|failed\|Failed"; then
            echo "❌ ERROR: $line"
        elif echo "$line" | grep -q "CREATE_ROOM\|ROOM_CREATED\|created and joined"; then
            echo "🏠 ROOM: $line"
        elif echo "$line" | grep -q "authenticated\|connected\|Authentication"; then
            echo "🔐 AUTH: $line"
        fi
    done
}

# Start monitoring in background
monitor_logs &
MONITOR_PID=$!

# Wait for user to finish testing
wait

# Cleanup function
cleanup() {
    echo ""
    echo "🧹 Cleaning up..."
    kill $MONITOR_PID 2>/dev/null
    kill $SERVER_PID 2>/dev/null

    echo ""
    echo "📊 FINAL LOG ANALYSIS:"
    echo "====================="

    echo "INVITE_USER commands received:"
    grep -c "INVITE_USER" server_realtime.log

    echo "Invitations processed:"
    grep -c "Processing invitation" server_realtime.log

    echo "Messages sent to users:"
    grep -c "send_to_user" server_realtime.log

    echo "Authentication events:"
    grep -c "authenticated\|Authentication" server_realtime.log

    echo ""
    echo "Full server log saved to: server_realtime.log"
    echo "Search for issues with: grep -i 'error\|fail' server_realtime.log"
}

# Set trap for cleanup
trap cleanup EXIT INT TERM

# Keep script running
echo "Monitoring... Press Ctrl+C to stop"
while true; do
    sleep 1
done
