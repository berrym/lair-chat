#!/bin/bash

# Simple debug test for invitation system
echo "🔬 INVITATION DEBUG TEST"
echo "========================"

# Clean up any existing processes
pkill -f lair-chat-server
pkill -f lair-chat-client
sleep 2

# Start server with debug logging
echo "🚀 Starting server..."
RUST_LOG=debug cargo run --bin lair-chat-server > server_debug.log 2>&1 &
SERVER_PID=$!
sleep 3

echo "📱 Starting Alice (inviter)..."
# Alice registers and sends invitation
(
    sleep 2
    echo '{"Register":{"username":"alice","password":"test123","fingerprint":"alice_device"}}'
    sleep 3
    echo "/create-room testroom"
    sleep 2
    echo "/invite bob testroom"
    sleep 5
    echo "quit"
) | RUST_LOG=debug cargo run --bin lair-chat-client > alice_debug.log 2>&1 &
ALICE_PID=$!

sleep 3

echo "📱 Starting Bob (recipient)..."
# Bob registers and waits for invitation
(
    sleep 2
    echo '{"Register":{"username":"bob","password":"test123","fingerprint":"bob_device"}}'
    sleep 8
    echo "quit"
) | RUST_LOG=debug cargo run --bin lair-chat-client > bob_debug.log 2>&1 &
BOB_PID=$!

echo "⏳ Waiting for test to complete..."
sleep 15

echo "🔍 Checking logs..."

echo ""
echo "=== SERVER DEBUG ==="
grep -E "(INVITE|invite|Invitation)" server_debug.log | tail -10

echo ""
echo "=== ALICE DEBUG ==="
grep -E "(INVITE|invite|Invitation)" alice_debug.log | tail -5

echo ""
echo "=== BOB DEBUG ==="
grep -E "(INVITE|invite|Invitation|SYSTEM_MESSAGE|DisplayMessage)" bob_debug.log | tail -10

echo ""
echo "🧹 Cleaning up..."
kill $ALICE_PID $BOB_PID $SERVER_PID 2>/dev/null

echo ""
echo "📋 ANALYSIS:"
echo "Check the logs above for:"
echo "1. Server receiving INVITE_USER:bob:testroom from Alice"
echo "2. Server sending SYSTEM_MESSAGE to Bob"
echo "3. Bob's client parsing the invitation message"
echo "4. Bob's client sending DisplayMessage actions"
echo ""
echo "Full logs saved to:"
echo "- server_debug.log"
echo "- alice_debug.log"
echo "- bob_debug.log"
