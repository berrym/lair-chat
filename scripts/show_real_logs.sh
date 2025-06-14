#!/bin/bash

# Script to find and display the actual lair-chat debug logs
# The logs are written to a file in the data directory, not to stdout

echo "=== LAIR CHAT LOG FINDER ==="
echo ""

# Find the actual log file location
LOG_DIR="$HOME/.config/the-lair-chat/data"
LOG_FILE="$LOG_DIR/lair-chat-client.log"

echo "Looking for logs in: $LOG_FILE"
echo ""

if [ -f "$LOG_FILE" ]; then
    echo "✅ Found log file!"
    echo "File size: $(du -h "$LOG_FILE" | cut -f1)"
    echo "Last modified: $(stat -f "%Sm" "$LOG_FILE" 2>/dev/null || stat -c "%y" "$LOG_FILE" 2>/dev/null)"
    echo ""
    
    echo "=== RECENT LOG ENTRIES ==="
    echo "Showing last 50 lines:"
    echo ""
    tail -50 "$LOG_FILE"
    
    echo ""
    echo "=== TRANSPORT DEBUG LOGS ==="
    echo "Filtering for transport-related entries:"
    echo ""
    grep -i "transport\|connect\|outgoing\|client_io_select\|instance\|queue" "$LOG_FILE" | tail -20
    
    echo ""
    echo "=== MESSAGE QUEUE OPERATIONS ==="
    echo "Filtering for message operations:"
    echo ""
    grep -i "add_outgoing_message\|process_outgoing_messages\|DEBUG.*message" "$LOG_FILE" | tail -20
    
    echo ""
    echo "=== FULL LOG COMMANDS ==="
    echo "To view full log: cat '$LOG_FILE'"
    echo "To follow live: tail -f '$LOG_FILE'"
    echo "To search: grep 'pattern' '$LOG_FILE'"
    
else
    echo "❌ Log file not found at: $LOG_FILE"
    echo ""
    echo "Checking if directory exists..."
    if [ -d "$LOG_DIR" ]; then
        echo "✅ Directory exists: $LOG_DIR"
        echo "Contents:"
        ls -la "$LOG_DIR"
    else
        echo "❌ Directory does not exist: $LOG_DIR"
        echo ""
        echo "Trying to find log files elsewhere..."
        find "$HOME" -name "*lair*chat*.log" -type f 2>/dev/null | head -10
    fi
    
    echo ""
    echo "💡 Try running the client first to generate logs:"
    echo "   cd lair-chat"
    echo "   RUST_LOG=debug cargo run --bin lair-chat-client"
    echo ""
    echo "Then run this script again to see the logs."
fi

echo ""
echo "=== DEBUG TIPS ==="
echo "If no debug logs appear, check:"
echo "1. RUST_LOG environment variable is set to 'debug' or 'trace'"
echo "2. The client is actually running and connecting"
echo "3. Try: RUST_LOG=trace cargo run --bin lair-chat-client"