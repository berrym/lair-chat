#!/bin/bash

# Focused DM Test Script
# Tests the direct message functionality between two clients

echo "🧪 Starting DM functionality test..."
echo "Make sure the server is running on localhost:8080"
echo ""

# Clean up any existing test files
rm -f dm_test_alice.log dm_test_fox.log

echo "📝 Building client..."
cargo build --bin lair-chat-client --quiet

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Build successful"
echo ""

# Function to extract meaningful messages from TUI output
extract_messages() {
    local logfile=$1
    local username=$2

    # Look for our debug messages and any actual content
    echo "=== $username's Messages ==="

    # Extract debug messages that we added
    grep -E "🔍 DEBUG \[$username\]:" "$logfile" | head -10

    # Look for any system messages or DM content that might appear
    grep -E "(💬|✅|❌|🔔|ℹ️)" "$logfile" | head -5

    echo ""
}

echo "🚀 Starting test clients..."
echo "⏰ Each client will run for 15 seconds"

# Start Alice (receiver) - will login and wait for messages
echo "👩 Starting Alice (receiver)..."
timeout 15s bash -c '
    echo "alice"
    sleep 1
    echo "password"
    sleep 10
    echo "/quit"
' | ./target/debug/lair-chat-client > dm_test_alice.log 2>&1 &

ALICE_PID=$!
sleep 3  # Give Alice time to connect

# Start Fox (sender) - will login and send DM
echo "🦊 Starting Fox (sender)..."
timeout 15s bash -c '
    echo "fox"
    sleep 1
    echo "password"
    sleep 2
    echo "/dm alice hello"
    sleep 5
    echo "/quit"
' | ./target/debug/lair-chat-client > dm_test_fox.log 2>&1 &

FOX_PID=$!

echo "⏳ Waiting for test to complete..."
sleep 18

# Clean up any remaining processes
kill $ALICE_PID 2>/dev/null
kill $FOX_PID 2>/dev/null

echo "📊 Test completed! Analyzing results..."
echo ""

# Extract and display results
extract_messages "dm_test_alice.log" "alice"
extract_messages "dm_test_fox.log" "fox"

echo "🔍 Looking for specific issues..."
echo ""

# Check Alice's log for duplicate messages
echo "🔎 Alice - Checking for duplicate message issue:"
if grep -q "🔍 DEBUG \[alice\]: Message filtered out by catch-all filter" dm_test_alice.log; then
    echo "✅ Good: Alice's client is filtering out processed messages"
else
    echo "⚠️  Warning: No evidence of message filtering in Alice's log"
fi

if grep -q "🔍 DEBUG \[alice\]: Adding unprocessed message" dm_test_alice.log; then
    echo "⚠️  Warning: Alice is still adding unprocessed messages"
else
    echo "✅ Good: No unprocessed message additions found for Alice"
fi

echo ""

# Check Fox's log for confirmation message
echo "🔎 Fox - Checking for confirmation message:"
if grep -q "🔍 DEBUG \[fox\]: Processing SYSTEM_MESSAGE" dm_test_fox.log; then
    echo "✅ Good: Fox received and processed a SYSTEM_MESSAGE"
    grep "🔍 DEBUG \[fox\]: Processing SYSTEM_MESSAGE" dm_test_fox.log | head -1
else
    echo "❌ Issue: Fox did not receive any SYSTEM_MESSAGE"
fi

if grep -q "🔍 DEBUG \[fox\]: Displaying system message" dm_test_fox.log; then
    echo "✅ Good: Fox displayed a system message"
    grep "🔍 DEBUG \[fox\]: Displaying system message" dm_test_fox.log | head -1
else
    echo "❌ Issue: Fox did not display any system message"
fi

echo ""
echo "📁 Full logs saved in:"
echo "   - dm_test_alice.log ($(wc -l < dm_test_alice.log) lines)"
echo "   - dm_test_fox.log ($(wc -l < dm_test_fox.log) lines)"
echo ""

echo "🔬 Quick analysis summary:"
echo "1. Check Alice's log for 'PRIVATE_MESSAGE:fox:hello' - should NOT appear"
echo "2. Check Fox's log for 'DM sent to alice' - should appear as formatted message"
echo "3. Both clients should show proper debug message filtering"

echo ""
echo "To examine full logs:"
echo "  cat dm_test_alice.log | grep -E '(DM|PRIVATE_MESSAGE|SYSTEM_MESSAGE)'"
echo "  cat dm_test_fox.log | grep -E '(DM|PRIVATE_MESSAGE|SYSTEM_MESSAGE)'"
