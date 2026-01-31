#!/bin/bash
# WebSocket manual testing helper script
# Usage: ./scripts/test_websocket.sh

SERVER="${1:-localhost:8082}"

echo "=== WebSocket Testing Helper ==="
echo "Server: $SERVER"
echo ""

# Check if websocat is installed
if ! command -v websocat &> /dev/null; then
    echo "websocat not found. Install with:"
    echo "  cargo install websocat"
    echo "  OR: sudo dnf install websocat"
    exit 1
fi

# Function to send messages with delay
test_basic_flow() {
    echo "Testing basic WebSocket flow..."
    echo "(This sends messages automatically with delays to avoid timeout)"
    echo ""

    {
        sleep 0.5
        echo '{"type":"client_hello","version":"1.0"}'
        sleep 0.5
        echo '{"type":"register","username":"wstest'$$'","email":"wstest'$$'@test.com","password":"SecurePass123!"}'
        sleep 0.5
        echo '{"type":"ping"}'
        sleep 0.5
        echo '{"type":"create_room","name":"WS Test Room '$$(date +%s)'"}'
        sleep 0.5
        echo '{"type":"list_rooms"}'
        sleep 2
    } | websocat -t "ws://$SERVER/ws" 2>&1
}

test_pre_auth() {
    echo ""
    echo "Testing pre-authentication flow..."

    # Get token via HTTP
    RESPONSE=$(curl -s -X POST "http://$SERVER/api/v1/auth/register" \
        -H "Content-Type: application/json" \
        -d '{"username":"preauth'$$'","email":"preauth'$$'@test.com","password":"SecurePass123!"}')

    TOKEN=$(echo "$RESPONSE" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

    if [ -z "$TOKEN" ]; then
        echo "Failed to get token. Response: $RESPONSE"
        return 1
    fi

    echo "Got token: ${TOKEN:0:20}..."
    echo "Connecting with pre-auth..."

    {
        sleep 1
        echo '{"type":"ping"}'
        sleep 1
        echo '{"type":"get_current_user"}'
        sleep 2
    } | websocat -t "ws://$SERVER/ws?token=$TOKEN" 2>&1
}

echo "Choose test:"
echo "1) Basic flow (register, create room, list rooms)"
echo "2) Pre-authentication flow"
echo "3) Interactive mode (you type, 30s timeout)"
read -p "Enter choice [1-3]: " choice

case $choice in
    1) test_basic_flow ;;
    2) test_pre_auth ;;
    3)
        echo "Interactive mode. Type JSON messages, one per line."
        echo "Server will send hello first. Then send client_hello within 30s."
        websocat -t "ws://$SERVER/ws"
        ;;
    *) echo "Invalid choice" ;;
esac
