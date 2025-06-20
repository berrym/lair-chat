#!/bin/bash

# Simple Invitation Debug Script
# This captures the exact message flow during an invitation test

echo "🔬 INVITATION DEBUG CAPTURE"
echo "=========================="

# Clean up
pkill -f lair-chat-server 2>/dev/null
pkill -f lair-chat-client 2>/dev/null
sleep 2

echo "🚀 Starting server with debug output..."
RUST_LOG=debug cargo run --bin lair-chat-server 2>&1 | tee server_debug_capture.log &
SERVER_PID=$!
sleep 4

echo "📱 Starting Alice (sender) client..."
# Alice will connect and send invitation
(
    sleep 2
    echo '{"Register":{"username":"alice","password":"test123","fingerprint":"alice_device"}}'
    sleep 3
    echo "/invite bob testroom"
    sleep 5
    echo "quit"
) | cargo run --bin lair-chat-client 2>&1 | tee alice_debug_capture.log &
ALICE_PID=$!

sleep 3

echo "📱 Starting Bob (recipient) client..."
# Bob will connect and wait for invitation
(
    sleep 2
    echo '{"Register":{"username":"bob","password":"test123","fingerprint":"bob_device"}}'
    sleep 8
    echo "quit"
) | cargo run --bin lair-chat-client 2>&1 | tee bob_debug_capture.log &
BOB_PID=$!

echo "⏳ Waiting for test to complete (15 seconds)..."
sleep 15

echo "🔍 ANALYSIS:"

echo ""
echo "=== SERVER DEBUG OUTPUT ==="
echo "Looking for invitation-related messages:"
grep -i "invite\|INVITE\|SERVER DEBUG" server_debug_capture.log | tail -10

echo ""
echo "=== ALICE (SENDER) DEBUG OUTPUT ==="
echo "Looking for invitation confirmations:"
grep -i "invite\|CLIENT DEBUG\|sent to bob" alice_debug_capture.log | tail -10

echo ""
echo "=== BOB (RECIPIENT) DEBUG OUTPUT ==="
echo "Looking for received invitations:"
grep -i "invite\|CLIENT DEBUG\|INVITATION" bob_debug_capture.log | tail -10

echo ""
echo "🔍 KEY QUESTIONS TO ANSWER:"
echo "1. Did Alice's client send 'INVITE_USER:bob:testroom' to server?"
echo "2. Did server receive and process the invitation?"
echo "3. Did server send invitation message to Bob?"
echo "4. Did Bob's client receive the invitation message?"
echo "5. Did Bob's client parse and handle the invitation?"

echo ""
echo "📊 FULL LOG FILES:"
echo "- server_debug_capture.log (server-side processing)"
echo "- alice_debug_capture.log (sender-side processing)"
echo "- bob_debug_capture.log (recipient-side processing)"

# Cleanup
kill $SERVER_PID $ALICE_PID $BOB_PID 2>/dev/null

echo ""
echo "✅ Debug capture completed!"
