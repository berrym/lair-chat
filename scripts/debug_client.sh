#!/bin/bash

# Debug script for lair-chat client with comprehensive logging
# This script captures all debug output to a file for analysis

echo "Starting lair-chat client with debug logging..."
echo "Logs will be saved to: debug_output.log"
echo "Press Ctrl+C to stop"
echo ""

# Clean up any previous log file
rm -f debug_output.log

# Set environment variables for maximum debug output
export RUST_LOG=trace
export RUST_BACKTRACE=1

# Run the client with all output redirected to log file
echo "=== LAIR CHAT DEBUG SESSION STARTED: $(date) ===" > debug_output.log
echo "Environment: RUST_LOG=$RUST_LOG" >> debug_output.log
echo "Build: $(cargo version 2>/dev/null || echo 'cargo not found')" >> debug_output.log
echo "======================================================" >> debug_output.log
echo "" >> debug_output.log

# Run the client and capture all output
cargo run --bin lair-chat-client 2>&1 | tee -a debug_output.log

echo ""
echo "Debug session ended. Log saved to: debug_output.log"
echo "To view the log: cat debug_output.log"
echo "To search for specific patterns:"
echo "  grep 'DEBUG:' debug_output.log"
echo "  grep 'transport' debug_output.log"
echo "  grep 'outgoing' debug_output.log"