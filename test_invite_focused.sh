#!/bin/bash

# Focused invitation test script
# This script will help us trace exactly what happens during an invitation

echo "🔬 FOCUSED INVITATION DEBUG TEST"
echo "================================="

# Clean up any existing processes
echo "🧹 Cleaning up existing processes..."
pkill -f lair-chat-server
pkill -f lair-chat-client
sleep 2

# Start server with debug output
echo "🚀 Starting server with debug logging..."
RUST_LOG=debug cargo run --bin lair-chat-server > server_debug.log 2>&1 &
SERVER_PID=$!
echo "Server PID: $SERVER_PID"
sleep 3

echo ""
echo "📋 MANUAL TEST INSTRUCTIONS:"
echo "1. Open two terminals"
echo "2. In terminal 1: cargo run --bin lair-chat-client"
echo "3. Register as 'alice' with password 'test123'"
echo "4. In terminal 2: cargo run --bin lair-chat-client"
echo "5. Register as 'bob' with password 'test123'"
echo "6. In Alice's terminal, type: /invite bob testroom"
echo "7. Watch both terminals for debug output"
echo "8. Check if Bob sees any invitation message"
echo ""

echo "🔍 What to look for:"
echo "- Alice should see: 📤 Invitation sent to bob for room 'testroom'"
echo "- Bob should see: 🔔 INVITATION: alice invited you to join room 'testroom'"
echo "- Server log should show: SERVER DEBUG messages about invitation"
echo "- Client logs should show: CLIENT DEBUG messages about parsing"
echo ""

echo "📊 DEBUGGING CHECKLIST:"
echo "□ Server receives INVITE_USER:bob:testroom from Alice"
echo "□ Server sends SYSTEM_MESSAGE:alice invited you to join room 'testroom' to Bob"
echo "□ Server sends SYSTEM_MESSAGE:You invited bob to join room 'testroom' to Alice"
echo "□ Alice's client parses confirmation message correctly"
echo "□ Bob's client receives and parses invitation message"
echo "□ Bob's client triggers Action::InvitationReceived"
echo "□ Bob's UI updates with invitation display"
echo ""

echo "⏳ Waiting for manual testing... Press Ctrl+C when done."
echo "Server log is being written to: server_debug.log"
echo ""

# Function to show server logs in real time
show_server_logs() {
    echo "🔍 REAL-TIME SERVER DEBUG (Press Ctrl+C to stop):"
    tail -f server_debug.log | grep --line-buffered -E "(SERVER DEBUG|invite|INVITE)"
}

# Wait for user input
read -p "Press Enter to start monitoring server logs, or Ctrl+C to exit: "
show_server_logs

# Cleanup function
cleanup() {
    echo ""
    echo "🧹 Cleaning up..."
    kill $SERVER_PID 2>/dev/null
    echo "✅ Test completed. Check server_debug.log for detailed server output."
}

trap cleanup EXIT
