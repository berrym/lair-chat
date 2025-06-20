#!/bin/bash

# Lair Chat Quick Start Script
# Simple script to quickly test Lair Chat with minimal setup

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo -e "${BLUE}🚀 Lair Chat Quick Start${NC}"
echo -e "${BLUE}========================${NC}\n"

# Check if we're in the right directory
if [[ ! -f "$PROJECT_DIR/Cargo.toml" ]]; then
    echo -e "${RED}Error: Not in Lair Chat project directory${NC}"
    exit 1
fi

cd "$PROJECT_DIR"

# Build the project
echo -e "${BLUE}Building project...${NC}"
if ! cargo build --release; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Build completed${NC}\n"

# Check if binaries exist
SERVER_BIN="./target/release/lair-chat-server"
CLIENT_BIN="./target/release/lair-chat-client"

if [[ ! -f "$SERVER_BIN" ]] || [[ ! -f "$CLIENT_BIN" ]]; then
    echo -e "${RED}Error: Binaries not found after build${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Binaries ready${NC}"
echo -e "${BLUE}Server: $SERVER_BIN${NC}"
echo -e "${BLUE}Client: $CLIENT_BIN${NC}\n"

# Start server in background
echo -e "${BLUE}Starting server...${NC}"
export LAIR_CHAT_DATABASE_URL=":memory:"
export LAIR_CHAT_LOGGING_LEVEL="info"
export LAIR_CHAT_SERVER_HOST="127.0.0.1"
export LAIR_CHAT_SERVER_PORT="8080"

"$SERVER_BIN" > server.log 2>&1 &
SERVER_PID=$!

# Wait for server to start
sleep 3

# Check if server is running
if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo -e "${RED}Server failed to start! Check server.log${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Server started (PID: $SERVER_PID)${NC}"
echo -e "${BLUE}Server running on 127.0.0.1:8080${NC}\n"

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Stopping server...${NC}"
    if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        sleep 2
        # Force kill if still running
        if kill -0 "$SERVER_PID" 2>/dev/null; then
            kill -9 "$SERVER_PID" 2>/dev/null || true
        fi
    fi
    echo -e "${GREEN}✓ Server stopped${NC}"
    exit 0
}

# Set up signal handler
trap cleanup SIGINT SIGTERM EXIT

# Instructions for manual testing
echo -e "${YELLOW}📋 Manual Testing Instructions:${NC}"
echo -e "${YELLOW}================================${NC}"
echo ""
echo "1. Open new terminal windows/tabs for each client"
echo ""
echo "2. In each terminal, run:"
echo -e "   ${GREEN}cd $(pwd)${NC}"
echo -e "   ${GREEN}$CLIENT_BIN${NC}"
echo ""
echo "3. For each client, use these test credentials:"
echo -e "   ${BLUE}Server: 127.0.0.1:8080${NC}"
echo -e "   ${BLUE}Username: alice, bob, charlie, etc.${NC}"
echo -e "   ${BLUE}Password: test123${NC}"
echo ""
echo "4. Test basic chat functionality:"
echo "   - Send messages between clients"
echo "   - Try commands: /help, /users, /rooms"
echo "   - Create rooms: /create-room test"
echo "   - Join rooms: /join test"
echo "   - Send direct messages: /dm username message"
echo ""
echo -e "${YELLOW}🛑 Press Ctrl+C to stop the server when done${NC}"
echo ""

# Alternative: automated test option
echo -e "${BLUE}Alternative: Automated Multi-Client Test${NC}"
echo -e "${BLUE}=========================================${NC}"
echo ""
echo "For automated testing with multiple clients, use:"
echo -e "${GREEN}./scripts/test_multiple_clients.sh${NC}"
echo ""
echo "Examples:"
echo -e "  ${GREEN}./scripts/test_multiple_clients.sh${NC}                    # 3 clients"
echo -e "  ${GREEN}./scripts/test_multiple_clients.sh -n 5${NC}               # 5 clients"
echo -e "  ${GREEN}./scripts/test_multiple_clients.sh -n 3 -a${NC}            # 3 clients with auto-messaging"
echo ""

# Keep server running
echo -e "${GREEN}Server is running and ready for connections...${NC}"
echo -e "${YELLOW}Waiting for clients to connect...${NC}"

# Monitor server status
while kill -0 "$SERVER_PID" 2>/dev/null; do
    sleep 5
done

echo -e "${RED}Server stopped unexpectedly!${NC}"
exit 1
