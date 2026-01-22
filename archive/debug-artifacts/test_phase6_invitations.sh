#!/bin/bash

# Test script for Phase 6 invitation system implementation
# This script tests the database-backed invitation system

set -e

echo "🎯 Phase 6 Invitation System Test"
echo "================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
TEST_DB="test_invitations.db"
SERVER_PID=""
CLIENT1_PID=""
CLIENT2_PID=""

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up test environment...${NC}"

    # Kill processes
    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
    fi
    if [ ! -z "$CLIENT1_PID" ]; then
        kill $CLIENT1_PID 2>/dev/null || true
    fi
    if [ ! -z "$CLIENT2_PID" ]; then
        kill $CLIENT2_PID 2>/dev/null || true
    fi

    # Remove test database
    rm -f "$TEST_DB" 2>/dev/null || true

    echo -e "${GREEN}Cleanup completed${NC}"
}

# Set up cleanup trap
trap cleanup EXIT

# Build the project
echo -e "${YELLOW}Building project...${NC}"
cargo build --release --quiet

# Start server with test database
echo -e "${YELLOW}Starting server with test database...${NC}"
RUST_LOG=info DATABASE_URL="sqlite:$TEST_DB" ./target/release/server &
SERVER_PID=$!

# Wait for server to start
sleep 3

# Check if server is running
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo -e "${RED}❌ Server failed to start${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Server started successfully (PID: $SERVER_PID)${NC}"

# Test functions
test_user_registration() {
    echo -e "\n${YELLOW}Testing user registration...${NC}"

    # Register alice
    echo "REGISTER:alice:password123" | nc localhost 8080 | head -1 | grep -q "REGISTRATION_SUCCESS"
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Alice registered successfully${NC}"
    else
        echo -e "${RED}❌ Alice registration failed${NC}"
        return 1
    fi

    # Register bob
    echo "REGISTER:bob:password456" | nc localhost 8080 | head -1 | grep -q "REGISTRATION_SUCCESS"
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Bob registered successfully${NC}"
    else
        echo -e "${RED}❌ Bob registration failed${NC}"
        return 1
    fi

    return 0
}

test_room_creation() {
    echo -e "\n${YELLOW}Testing room creation...${NC}"

    # Alice creates a room
    {
        echo "LOGIN:alice:password123"
        sleep 1
        echo "CREATE_ROOM:test-room"
        sleep 1
    } | nc localhost 8080 | grep -q "ROOM_CREATED"

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Room created successfully${NC}"
        return 0
    else
        echo -e "${RED}❌ Room creation failed${NC}"
        return 1
    fi
}

test_invitation_creation() {
    echo -e "\n${YELLOW}Testing invitation creation...${NC}"

    # Alice invites Bob to the room
    {
        echo "LOGIN:alice:password123"
        sleep 1
        echo "INVITE_USER:bob:test-room"
        sleep 2
    } | nc localhost 8080 | grep -q "You invited bob"

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Invitation created successfully${NC}"
        return 0
    else
        echo -e "${RED}❌ Invitation creation failed${NC}"
        return 1
    fi
}

test_invitation_listing() {
    echo -e "\n${YELLOW}Testing invitation listing...${NC}"

    # Bob lists his invitations
    {
        echo "LOGIN:bob:password456"
        sleep 1
        echo "LIST_INVITATIONS"
        sleep 2
    } | nc localhost 8080 | grep -q "Pending invitations"

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Invitation listing works${NC}"
        return 0
    else
        echo -e "${RED}❌ Invitation listing failed${NC}"
        return 1
    fi
}

test_invitation_acceptance() {
    echo -e "\n${YELLOW}Testing invitation acceptance...${NC}"

    # Bob accepts the invitation
    {
        echo "LOGIN:bob:password456"
        sleep 1
        echo "ACCEPT_INVITATION:test-room"
        sleep 2
    } | nc localhost 8080 | grep -q "joined room"

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Invitation acceptance works${NC}"
        return 0
    else
        echo -e "${RED}❌ Invitation acceptance failed${NC}"
        return 1
    fi
}

test_invitation_decline() {
    echo -e "\n${YELLOW}Testing invitation decline...${NC}"

    # Alice creates another room and invites Bob
    {
        echo "LOGIN:alice:password123"
        sleep 1
        echo "CREATE_ROOM:test-room2"
        sleep 1
        echo "INVITE_USER:bob:test-room2"
        sleep 2
    } | nc localhost 8080 > /dev/null

    # Bob declines the invitation
    {
        echo "LOGIN:bob:password456"
        sleep 1
        echo "DECLINE_INVITATION:test-room2"
        sleep 2
    } | nc localhost 8080 | grep -q "declined the invitation"

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Invitation decline works${NC}"
        return 0
    else
        echo -e "${RED}❌ Invitation decline failed${NC}"
        return 1
    fi
}

test_duplicate_invitation_prevention() {
    echo -e "\n${YELLOW}Testing duplicate invitation prevention...${NC}"

    # Alice tries to invite Bob to the same room again
    {
        echo "LOGIN:alice:password123"
        sleep 1
        echo "CREATE_ROOM:test-room3"
        sleep 1
        echo "INVITE_USER:bob:test-room3"
        sleep 1
        echo "INVITE_USER:bob:test-room3"
        sleep 2
    } | nc localhost 8080 | grep -q "already has a pending invitation"

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Duplicate invitation prevention works${NC}"
        return 0
    else
        echo -e "${RED}❌ Duplicate invitation prevention failed${NC}"
        return 1
    fi
}

test_nonexistent_user_invitation() {
    echo -e "\n${YELLOW}Testing invitation to nonexistent user...${NC}"

    # Alice tries to invite a nonexistent user
    {
        echo "LOGIN:alice:password123"
        sleep 1
        echo "INVITE_USER:nonexistent:test-room"
        sleep 2
    } | nc localhost 8080 | grep -q "does not exist"

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Nonexistent user invitation handling works${NC}"
        return 0
    else
        echo -e "${RED}❌ Nonexistent user invitation handling failed${NC}"
        return 1
    fi
}

test_nonexistent_room_invitation() {
    echo -e "\n${YELLOW}Testing invitation to nonexistent room...${NC}"

    # Alice tries to invite Bob to a nonexistent room
    {
        echo "LOGIN:alice:password123"
        sleep 1
        echo "INVITE_USER:bob:nonexistent-room"
        sleep 2
    } | nc localhost 8080 | grep -q "does not exist"

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Nonexistent room invitation handling works${NC}"
        return 0
    else
        echo -e "${RED}❌ Nonexistent room invitation handling failed${NC}"
        return 1
    fi
}

# Run all tests
echo -e "\n${YELLOW}Running Phase 6 invitation system tests...${NC}"

TESTS_PASSED=0
TESTS_TOTAL=0

# Test user registration
TESTS_TOTAL=$((TESTS_TOTAL + 1))
if test_user_registration; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi

# Test room creation
TESTS_TOTAL=$((TESTS_TOTAL + 1))
if test_room_creation; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi

# Test invitation creation
TESTS_TOTAL=$((TESTS_TOTAL + 1))
if test_invitation_creation; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi

# Test invitation listing
TESTS_TOTAL=$((TESTS_TOTAL + 1))
if test_invitation_listing; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi

# Test invitation acceptance
TESTS_TOTAL=$((TESTS_TOTAL + 1))
if test_invitation_acceptance; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi

# Test invitation decline
TESTS_TOTAL=$((TESTS_TOTAL + 1))
if test_invitation_decline; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi

# Test duplicate invitation prevention
TESTS_TOTAL=$((TESTS_TOTAL + 1))
if test_duplicate_invitation_prevention; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi

# Test nonexistent user invitation
TESTS_TOTAL=$((TESTS_TOTAL + 1))
if test_nonexistent_user_invitation; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi

# Test nonexistent room invitation
TESTS_TOTAL=$((TESTS_TOTAL + 1))
if test_nonexistent_room_invitation; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi

# Summary
echo -e "\n${YELLOW}Test Results${NC}"
echo "============"
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}/$TESTS_TOTAL"

if [ $TESTS_PASSED -eq $TESTS_TOTAL ]; then
    echo -e "\n${GREEN}🎉 All Phase 6 invitation tests passed!${NC}"
    echo -e "${GREEN}✅ Database-backed invitation system is working correctly${NC}"
    exit 0
else
    echo -e "\n${RED}❌ Some tests failed${NC}"
    echo -e "${RED}Phase 6 invitation system needs attention${NC}"
    exit 1
fi
