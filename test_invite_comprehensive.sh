#!/bin/bash

# Comprehensive Invitation System Test
# This script tests the complete invitation flow step by step

echo "🔬 COMPREHENSIVE INVITATION SYSTEM TEST"
echo "========================================"

# Clean up any existing processes
echo "🧹 Cleaning up existing processes..."
pkill -f lair-chat-server
pkill -f lair-chat-client
sleep 3

# Start server with detailed logging
echo "🚀 Starting server with detailed logging..."
RUST_LOG=debug cargo run --bin lair-chat-server > server_comprehensive.log 2>&1 &
SERVER_PID=$!
echo "Server PID: $SERVER_PID"
sleep 5

echo ""
echo "📝 TEST SCENARIO:"
echo "1. Alice registers and creates a room 'gameroom'"
echo "2. Alice invites Bob to 'gameroom'"
echo "3. Bob registers (after invitation is sent)"
echo "4. Check if Bob receives the invitation"
echo ""

# Function to send commands to client
send_commands_to_client() {
    local client_name=$1
    local commands=$2
    local log_file=$3

    echo "📱 Starting $client_name..."
    (
        echo "$commands"
    ) | RUST_LOG=debug cargo run --bin lair-chat-client > "$log_file" 2>&1 &

    return $!
}

# Alice's commands
alice_commands=$(cat << 'EOF'
{"Register":{"username":"alice","password":"test123","fingerprint":"alice_device"}}


/create-room gameroom


/invite bob gameroom


quit
EOF
)

# Bob's commands (starts after Alice sends invitation)
bob_commands=$(cat << 'EOF'
{"Register":{"username":"bob","password":"test123","fingerprint":"bob_device"}}



quit
EOF
)

# Start Alice
send_commands_to_client "Alice" "$alice_commands" "alice_comprehensive.log"
ALICE_PID=$!
echo "Alice PID: $ALICE_PID"

# Wait for Alice to complete her actions
sleep 8

# Start Bob
send_commands_to_client "Bob" "$bob_commands" "bob_comprehensive.log"
BOB_PID=$!
echo "Bob PID: $BOB_PID"

# Wait for test to complete
echo "⏳ Waiting for test to complete..."
sleep 10

# Stop all processes
echo "🛑 Stopping all processes..."
kill $ALICE_PID $BOB_PID $SERVER_PID 2>/dev/null
sleep 2

echo ""
echo "🔍 ANALYZING RESULTS..."
echo "========================"

echo ""
echo "=== SERVER LOGS (Invitation Processing) ==="
echo "Looking for invitation-related server activity:"
grep -E "(INVITE_USER|invite|Invitation|alice|bob|gameroom)" server_comprehensive.log | head -20

echo ""
echo "=== ALICE LOGS (Invitation Sender) ==="
echo "Looking for Alice's invitation activity:"
grep -E "(invite|gameroom|bob|DisplayMessage|Action)" alice_comprehensive.log | head -10

echo ""
echo "=== BOB LOGS (Invitation Recipient) ==="
echo "Looking for Bob's invitation reception:"
grep -E "(INVITATION|invite|alice|gameroom|DisplayMessage|SYSTEM_MESSAGE)" bob_comprehensive.log | head -15

echo ""
echo "📊 DETAILED ANALYSIS:"
echo "===================="

# Check if server received INVITE_USER command
invite_commands=$(grep -c "INVITE_USER" server_comprehensive.log)
echo "1. Server received INVITE_USER commands: $invite_commands"

# Check if server sent invitation to Bob
invitations_sent=$(grep -c "invited you to join room" server_comprehensive.log)
echo "2. Server sent invitation messages: $invitations_sent"

# Check if Bob's client received SYSTEM_MESSAGE
bob_system_msgs=$(grep -c "SYSTEM_MESSAGE" bob_comprehensive.log)
echo "3. Bob received SYSTEM_MESSAGE count: $bob_system_msgs"

# Check if Bob's client processed invitation
bob_invitations=$(grep -c "INVITATION" bob_comprehensive.log)
echo "4. Bob processed invitation messages: $bob_invitations"

# Check authentication status
alice_auth=$(grep -c "Authentication successful\|authenticated" alice_comprehensive.log)
bob_auth=$(grep -c "Authentication successful\|authenticated" bob_comprehensive.log)
echo "5. Alice authentication events: $alice_auth"
echo "6. Bob authentication events: $bob_auth"

echo ""
echo "🔍 KEY DIAGNOSTICS:"
echo "=================="

# Check server state
echo "Checking server user management:"
grep -E "(connected_users|peers|send_to_user)" server_comprehensive.log | tail -5

echo ""
echo "Checking invitation flow:"
grep -A 2 -B 2 "Processing invitation\|INVITE_USER\|invited you to join" server_comprehensive.log | tail -10

echo ""
echo "📋 ISSUE IDENTIFICATION:"
echo "======================="

if [ $invite_commands -eq 0 ]; then
    echo "❌ PROBLEM: Server never received INVITE_USER command from Alice"
    echo "   → Check if Alice's command processing is working"
elif [ $invitations_sent -eq 0 ]; then
    echo "❌ PROBLEM: Server received INVITE_USER but never sent invitation to Bob"
    echo "   → Check server validation logic (room exists, user permissions)"
elif [ $bob_system_msgs -eq 0 ]; then
    echo "❌ PROBLEM: Bob never received SYSTEM_MESSAGE from server"
    echo "   → Check if Bob is properly connected and authenticated"
elif [ $bob_invitations -eq 0 ]; then
    echo "❌ PROBLEM: Bob received SYSTEM_MESSAGE but didn't process invitation"
    echo "   → Check client-side invitation parsing logic"
else
    echo "✅ All steps completed - check detailed logs for subtle issues"
fi

echo ""
echo "📁 LOG FILES CREATED:"
echo "===================="
echo "- server_comprehensive.log (server debug output)"
echo "- alice_comprehensive.log (Alice client debug output)"
echo "- bob_comprehensive.log (Bob client debug output)"

echo ""
echo "🔧 DEBUGGING COMMANDS:"
echo "====================="
echo "View server logs: tail -f server_comprehensive.log"
echo "Search Alice logs: grep -i invite alice_comprehensive.log"
echo "Search Bob logs: grep -i invitation bob_comprehensive.log"
echo ""
echo "Full invitation flow: grep -E 'INVITE_USER|invited you|INVITATION' *.log"
