#!/bin/bash
# Development mode startup script

export RUST_LOG=debug,lair_chat=trace
export RUST_BACKTRACE=1

echo "🔧 Starting in development mode..."
echo "📋 Logs will be more verbose"

# Run the quick start
./quick_start.sh
