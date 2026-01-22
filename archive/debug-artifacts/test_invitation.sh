#!/bin/bash

# Test script for invitation system debugging
# This script tests the complete invitation flow between two users

echo "🔬 Starting invitation system test..."

# Kill any existing processes
pkill -f lair-chat-server
pkill -f lair-chat-client
sleep 2

# Start server with debug logging
echo "🚀 Starting server with debug logging..."
RUST_LOG=debug cargo run --bin lair-chat-server > server_invitation_test.log 2>&1 &
SERVER_PID=$!
sleep 3

echo "📱 Starting Alice (inviter) client..."
# Start Alice's client
(
    echo "Registering Alice..."
    sleep 2
    echo '{"Register":{"username":"alice","password":"password123","fingerprint":"alice_device"}}'
    sleep 2
    echo "Alice authenticated, ready to send invitation..."
    sleep 5
    echo "/invite bob testroom"
    echo "Invitation sent by Alice"
    sleep 10
    echo "quit"
) | RUST_LOG=debug cargo run --bin lair-chat-client > alice_invitation_test.log 2>&1 &
ALICE_PID=$!

sleep 5

echo "📱 Starting Bob (recipient) client..."
# Start Bob's client
(
    echo "Registering Bob..."
    sleep 2
    echo '{"Register":{"username":"bob","password":"password123","fingerprint":"bob_device"}}'
    sleep 2
    echo "Bob authenticated, waiting for invitation..."
    sleep 15
    echo "quit"
) | RUST_LOG=debug cargo run --bin lair-chat-client > bob_invitation_test.log 2>&1 &
BOB_PID=$!

echo "⏳ Waiting for test to complete..."
sleep 20

echo "🔍 Analyzing logs..."

echo "=== SERVER LOG ==="
if [ -f server_invitation_test.log ]; then
    echo "Server debug output:"
    grep -i "invite\|invitation" server_invitation_test.log | tail -20
else
    echo "No server log found"
fi

echo ""
echo "=== ALICE LOG (Inviter) ==="
if [ -f alice_invitation_test.log ]; then
    echo "Alice debug output:"
    grep -i "invite\|CLIENT DEBUG" alice_invitation_test.log | tail -20
else
    echo "No Alice log found"
fi

echo ""
echo "=== BOB LOG (Recipient) ==="
if [ -f bob_invitation_test.log ]; then
    echo "Bob debug output:"
    grep -i "invite\|CLIENT DEBUG" bob_invitation_test.log | tail -20
else
    echo "No Bob log found"
fi

echo ""
echo "🧹 Cleaning up processes..."
kill $ALICE_PID 2>/dev/null
kill $BOB_PID 2>/dev/null
kill $SERVER_PID 2>/dev/null

echo ""
echo "📊 Test Summary:"
echo "- Check server_invitation_test.log for server-side processing"
echo "- Check alice_invitation_test.log for sender-side processing"
echo "- Check bob_invitation_test.log for recipient-side processing"
echo "- Look for 'CLIENT DEBUG' messages to trace invitation flow"

echo ""
echo "🔍 Key things to look for:"
echo "1. Alice sends: /invite bob testroom"
echo "2. Server receives: INVITE_USER:bob:testroom"
echo "3. Server sends to Bob: SYSTEM_MESSAGE:alice invited you to join room 'testroom'"
echo "4. Server sends to Alice: SYSTEM_MESSAGE:You invited bob to join room 'testroom'"
echo "5. Bob's client shows: CLIENT DEBUG: INVITATION MESSAGE DETECTED"
echo "6. Bob's client shows: CLIENT DEBUG: INVITATION RECEIVED PATTERN MATCHED"
echo "7. Bob's UI updates with invitation message"

echo ""
echo "✅ Invitation test completed. Check the logs above for debugging information."
