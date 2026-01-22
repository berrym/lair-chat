#!/bin/bash

# Test script for room creation functionality
echo "🏠 Testing Room Creation Functionality"
echo "======================================"

# Make sure we're in the right directory
cd "$(dirname "$0")"

# Check if server is running
if ! pgrep -f "lair-chat-server" > /dev/null; then
    echo "❌ Server is not running. Please start the server first:"
    echo "   ./target/release/lair-chat-server"
    exit 1
fi

echo "✅ Server is running"

# Function to test room creation with expect
test_room_creation() {
    local test_name="$1"
    local username="$2"
    local password="$3"
    local room_name="$4"

    echo ""
    echo "🧪 Test: $test_name"
    echo "   Username: $username"
    echo "   Room: $room_name"

    # Create expect script
    cat > temp_test_script.exp << EOF
#!/usr/bin/expect -f
set timeout 10

spawn ./target/release/lair-chat-client
expect "Welcome to Lair Chat!"
expect "Login with existing account? (y/n):"
send "n\r"
expect "Enter username:"
send "$username\r"
expect "Enter password:"
send "$password\r"
expect "Registration successful"
expect "Authentication successful"

# Wait for lobby
sleep 2

# Create room
send "/create-room $room_name\r"
sleep 1

# Check if room was created
send "/rooms\r"
sleep 1

# Try to join the room (should already be in it)
send "hello from $room_name!\r"
sleep 1

# Leave the room
send "/leave\r"
sleep 1

# Try to join the room again
send "/join $room_name\r"
sleep 1

send "back in $room_name!\r"
sleep 1

# Quit
send "/quit\r"
expect eof
EOF

    chmod +x temp_test_script.exp

    # Run the test
    if ./temp_test_script.exp > "test_output_$username.log" 2>&1; then
        echo "✅ Test completed"

        # Check for success indicators in the log
        if grep -q "Room '$room_name' created successfully" "test_output_$username.log"; then
            echo "✅ Room creation message found"
        else
            echo "❌ Room creation message not found"
        fi

        if grep -q "Current Room: $room_name" "test_output_$username.log"; then
            echo "✅ Status bar updated"
        else
            echo "❌ Status bar not updated"
        fi

    else
        echo "❌ Test failed"
    fi

    # Clean up
    rm -f temp_test_script.exp
}

# Install expect if not available
if ! command -v expect > /dev/null; then
    echo "⚠️  'expect' not found. Installing..."
    if command -v apt-get > /dev/null; then
        sudo apt-get install -y expect
    elif command -v yum > /dev/null; then
        sudo yum install -y expect
    elif command -v brew > /dev/null; then
        brew install expect
    else
        echo "❌ Please install 'expect' manually"
        exit 1
    fi
fi

# Run tests
test_room_creation "Basic Room Creation" "testuser1" "testpass1" "testroom1"
test_room_creation "Room with Spaces" "testuser2" "testpass2" "test room 2"
test_room_creation "Gaming Room" "testuser3" "testpass3" "gaming"

echo ""
echo "🔍 Checking server logs for room creation messages..."
if [ -f server_test.log ]; then
    echo "Room creation events in server log:"
    grep -i "room.*created\|user.*joined\|room.*joined" server_test.log | tail -5
else
    echo "❌ Server log not found"
fi

echo ""
echo "📋 Test Results Summary:"
echo "========================"

# Count successful tests
success_count=0
total_tests=3

for i in {1..3}; do
    if [ -f "test_output_testuser$i.log" ]; then
        if grep -q "Room.*created successfully" "test_output_testuser$i.log"; then
            echo "✅ Test $i: PASSED"
            ((success_count++))
        else
            echo "❌ Test $i: FAILED"
            echo "   Check test_output_testuser$i.log for details"
        fi
    else
        echo "❌ Test $i: NO OUTPUT"
    fi
done

echo ""
echo "Results: $success_count/$total_tests tests passed"

if [ $success_count -eq $total_tests ]; then
    echo "🎉 All tests passed! Room creation is working correctly."
    exit 0
else
    echo "⚠️  Some tests failed. Check the logs for details."
    exit 1
fi

# Clean up test logs
read -p "Clean up test logs? (y/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -f test_output_*.log
    echo "✅ Test logs cleaned up"
fi
