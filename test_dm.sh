#!/bin/bash

# Test script for Direct Messaging functionality
# This script helps debug DM issues by providing controlled testing

set -e

echo "=== LAIR CHAT DM TEST SCRIPT ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up processes...${NC}"
    pkill -f lair-chat-server 2>/dev/null || true
    pkill -f lair-chat-client 2>/dev/null || true
    sleep 2
    echo -e "${GREEN}Cleanup complete${NC}"
}

# Set cleanup trap
trap cleanup EXIT

# Clean up any existing processes
cleanup

echo -e "${BLUE}Step 1: Building project...${NC}"
cargo build --quiet

echo -e "${BLUE}Step 2: Starting server with debug logging...${NC}"
RUST_LOG=info cargo run --bin lair-chat-server > server_test.log 2>&1 &
SERVER_PID=$!
echo "Server PID: $SERVER_PID"

# Wait for server to start
sleep 3

# Check if server is running
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo -e "${RED}ERROR: Server failed to start!${NC}"
    echo "Server log:"
    cat server_test.log
    exit 1
fi

echo -e "${GREEN}Server started successfully${NC}"

echo -e "${BLUE}Step 3: Starting first client (alice)...${NC}"
echo "Starting alice client in background..."
RUST_LOG=info cargo run --bin lair-chat-client -- --username alice > alice_test.log 2>&1 &
ALICE_PID=$!
echo "Alice PID: $ALICE_PID"

# Wait for alice to connect
sleep 5

echo -e "${BLUE}Step 4: Starting second client (bob)...${NC}"
echo "Starting bob client in background..."
RUST_LOG=info cargo run --bin lair-chat-client -- --username bob > bob_test.log 2>&1 &
BOB_PID=$!
echo "Bob PID: $BOB_PID"

# Wait for bob to connect
sleep 5

echo -e "${BLUE}Step 5: Checking process status...${NC}"
echo "Server running: $(kill -0 $SERVER_PID 2>/dev/null && echo 'YES' || echo 'NO')"
echo "Alice running: $(kill -0 $ALICE_PID 2>/dev/null && echo 'YES' || echo 'NO')"
echo "Bob running: $(kill -0 $BOB_PID 2>/dev/null && echo 'YES' || echo 'NO')"

echo -e "\n${BLUE}Step 6: Analyzing logs...${NC}"

echo -e "\n${YELLOW}=== SERVER LOG ANALYSIS ===${NC}"
if [ -f server_test.log ]; then
    echo "Server log size: $(wc -l < server_test.log) lines"
    echo -e "\n${YELLOW}Server startup messages:${NC}"
    head -10 server_test.log

    echo -e "\n${YELLOW}User connections in server log:${NC}"
    grep -i "user.*joined\|connected_user\|add_connected_user" server_test.log || echo "No user connection messages found"

    echo -e "\n${YELLOW}DM-related messages in server log:${NC}"
    grep -i "DM\|direct" server_test.log || echo "No DM messages found in server log"
else
    echo -e "${RED}No server log found!${NC}"
fi

echo -e "\n${YELLOW}=== ALICE LOG ANALYSIS ===${NC}"
if [ -f alice_test.log ]; then
    echo "Alice log size: $(wc -l < alice_test.log) lines"
    echo -e "\n${YELLOW}Alice authentication:${NC}"
    grep -i "auth\|welcome\|lobby" alice_test.log | head -5 || echo "No auth messages found"

    echo -e "\n${YELLOW}Alice DM-related messages:${NC}"
    grep -i "DM\|direct" alice_test.log || echo "No DM messages found in alice log"
else
    echo -e "${RED}No alice log found!${NC}"
fi

echo -e "\n${YELLOW}=== BOB LOG ANALYSIS ===${NC}"
if [ -f bob_test.log ]; then
    echo "Bob log size: $(wc -l < bob_test.log) lines"
    echo -e "\n${YELLOW}Bob authentication:${NC}"
    grep -i "auth\|welcome\|lobby" bob_test.log | head -5 || echo "No auth messages found"

    echo -e "\n${YELLOW}Bob DM-related messages:${NC}"
    grep -i "DM\|direct" bob_test.log || echo "No DM messages found in bob log"
else
    echo -e "${RED}No bob log found!${NC}"
fi

echo -e "\n${BLUE}Step 7: Manual test instructions${NC}"
echo -e "${YELLOW}The clients are now running. To test DM functionality:${NC}"
echo ""
echo "1. Switch to alice's terminal and:"
echo "   - Press Ctrl+L then 'n' to open user list"
echo "   - Select 'bob' from the list with arrow keys and Enter"
echo "   - Type a message and press Enter"
echo ""
echo "2. Switch to bob's terminal and check if the message appears"
echo ""
echo "3. Try sending a reply from bob to alice"
echo ""
echo -e "${YELLOW}Logs are being written to:${NC}"
echo "  - server_test.log (server)"
echo "  - alice_test.log (alice client)"
echo "  - bob_test.log (bob client)"
echo ""
echo -e "${YELLOW}To monitor logs in real-time:${NC}"
echo "  tail -f server_test.log"
echo "  tail -f alice_test.log"
echo "  tail -f bob_test.log"
echo ""
echo -e "${YELLOW}To kill all processes: ${RED}pkill -f lair-chat${NC}"
echo ""
echo -e "${GREEN}Test setup complete! Press Ctrl+C to cleanup and exit.${NC}"

# Keep script running until user interrupts
echo "Waiting for user interrupt (Ctrl+C)..."
while true; do
    sleep 5
    # Check if processes are still running
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo -e "${RED}Server process died unexpectedly!${NC}"
        break
    fi
    if ! kill -0 $ALICE_PID 2>/dev/null; then
        echo -e "${RED}Alice process died unexpectedly!${NC}"
        break
    fi
    if ! kill -0 $BOB_PID 2>/dev/null; then
        echo -e "${RED}Bob process died unexpectedly!${NC}"
        break
    fi
done
