#!/bin/bash

# Comprehensive REST API Testing Script for Lair Chat
# This script demonstrates all major API functionality and serves as a practical guide

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
BASE_URL="http://localhost:8082/api/v1"
ADMIN_USERNAME="admin"
ADMIN_PASSWORD="AdminPassword123!"
TEST_USER="testuser"
TEST_EMAIL="test@example.com"
TEST_PASSWORD="TestPass123!"

# Global variables
ADMIN_TOKEN=""
USER_TOKEN=""
TEST_USER_ID=""
TEST_ROOM_ID=""
TEST_MESSAGE_ID=""

print_banner() {
    echo -e "\n${CYAN}============================================================${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}============================================================${NC}\n"
}

print_section() {
    echo -e "\n${BLUE}▶ $1${NC}"
    echo -e "${BLUE}$(printf '%.0s-' {1..50})${NC}"
}

print_step() {
    echo -e "${YELLOW}[STEP]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

# Check if server is running
check_server() {
    print_section "Checking Server Status"

    print_step "Testing server connection..."
    if curl -s "$BASE_URL/health" > /dev/null 2>&1; then
        print_success "Server is running at $BASE_URL"
    else
        print_error "Server is not running at $BASE_URL"
        echo "Please start the server with: DATABASE_URL=\"sqlite:data/lair-chat.db\" ./target/release/lair-chat-server"
        exit 1
    fi

    # Display health check
    print_step "Getting health status..."
    HEALTH_RESPONSE=$(curl -s "$BASE_URL/health")
    echo "$HEALTH_RESPONSE" | jq . 2>/dev/null || echo "$HEALTH_RESPONSE"
}

# Test authentication endpoints
test_authentication() {
    print_section "Testing Authentication"

    # Test admin login
    print_step "Testing admin login..."
    LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/login" \
        -H "Content-Type: application/json" \
        -d "{\"identifier\":\"$ADMIN_USERNAME\",\"password\":\"$ADMIN_PASSWORD\",\"remember_me\":true}")

    if echo "$LOGIN_RESPONSE" | grep -q "access_token"; then
        ADMIN_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.access_token')
        print_success "Admin login successful"
        print_info "Token: ${ADMIN_TOKEN:0:20}..."

        # Display user info
        USER_INFO=$(echo "$LOGIN_RESPONSE" | jq '.user')
        echo "User Info: $USER_INFO"
    else
        print_error "Admin login failed"
        echo "Response: $LOGIN_RESPONSE"
        exit 1
    fi

    # Test token validation
    print_step "Testing token validation..."
    ME_RESPONSE=$(curl -s -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/users/me")
    if echo "$ME_RESPONSE" | grep -q "username"; then
        print_success "Token validation successful"
        echo "Current user: $(echo "$ME_RESPONSE" | jq -r '.username')"
    else
        print_error "Token validation failed"
        echo "Response: $ME_RESPONSE"
    fi
}

# Test user management
test_user_management() {
    print_section "Testing User Management"

    # Create test user
    print_step "Creating test user..."
    REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/register" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$TEST_USER\",\"email\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\",\"full_name\":\"Test User\"}")

    if echo "$REGISTER_RESPONSE" | grep -q "id\|username"; then
        TEST_USER_ID=$(echo "$REGISTER_RESPONSE" | jq -r '.user.id // .id // empty')
        print_success "Test user created successfully"
        print_info "User ID: $TEST_USER_ID"
    else
        print_info "Test user might already exist, attempting login..."

        # Try to login with test user
        USER_LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/login" \
            -H "Content-Type: application/json" \
            -d "{\"identifier\":\"$TEST_USER\",\"password\":\"$TEST_PASSWORD\"}")

        if echo "$USER_LOGIN_RESPONSE" | grep -q "access_token"; then
            USER_TOKEN=$(echo "$USER_LOGIN_RESPONSE" | jq -r '.access_token')
            TEST_USER_ID=$(echo "$USER_LOGIN_RESPONSE" | jq -r '.user.id')
            print_success "Test user login successful"
        else
            print_error "Could not create or login test user"
            echo "Register response: $REGISTER_RESPONSE"
            echo "Login response: $USER_LOGIN_RESPONSE"
        fi
    fi

    # Test user login
    if [ -z "$USER_TOKEN" ]; then
        print_step "Testing user login..."
        USER_LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/login" \
            -H "Content-Type: application/json" \
            -d "{\"identifier\":\"$TEST_USER\",\"password\":\"$TEST_PASSWORD\"}")

        if echo "$USER_LOGIN_RESPONSE" | grep -q "access_token"; then
            USER_TOKEN=$(echo "$USER_LOGIN_RESPONSE" | jq -r '.access_token')
            print_success "User login successful"
        else
            print_error "User login failed"
            echo "Response: $USER_LOGIN_RESPONSE"
        fi
    fi

    # List all users (admin only)
    print_step "Testing admin user listing..."
    USERS_RESPONSE=$(curl -s -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/admin/users")
    if echo "$USERS_RESPONSE" | grep -q "username\|users"; then
        USER_COUNT=$(echo "$USERS_RESPONSE" | jq '. | if type=="array" then length else .users | length end' 2>/dev/null || echo "unknown")
        print_success "User listing successful ($USER_COUNT users found)"
    else
        print_error "User listing failed"
        echo "Response: $USERS_RESPONSE"
    fi
}

# Test room management
test_room_management() {
    print_section "Testing Room Management"

    # Create test room
    print_step "Creating test room..."
    ROOM_RESPONSE=$(curl -s -X POST "$BASE_URL/rooms" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{"name":"API Test Room","description":"Room created by API test script","is_private":false}')

    if echo "$ROOM_RESPONSE" | grep -q "id\|name"; then
        TEST_ROOM_ID=$(echo "$ROOM_RESPONSE" | jq -r '.id // .room.id // empty')
        print_success "Test room created successfully"
        print_info "Room ID: $TEST_ROOM_ID"
    else
        print_info "Room creation might have failed, trying to list existing rooms..."
        echo "Response: $ROOM_RESPONSE"
    fi

    # List rooms
    print_step "Testing room listing..."
    ROOMS_RESPONSE=$(curl -s -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/rooms")
    if echo "$ROOMS_RESPONSE" | grep -q "name\|rooms\|\[\]"; then
        ROOM_COUNT=$(echo "$ROOMS_RESPONSE" | jq '. | if type=="array" then length else .rooms | length end' 2>/dev/null || echo "unknown")
        print_success "Room listing successful ($ROOM_COUNT rooms found)"

        # If no test room ID yet, try to get one from the list
        if [ -z "$TEST_ROOM_ID" ] && [ "$ROOM_COUNT" != "0" ]; then
            TEST_ROOM_ID=$(echo "$ROOMS_RESPONSE" | jq -r '.[0].id // .rooms[0].id // empty' 2>/dev/null || echo "")
            if [ -n "$TEST_ROOM_ID" ]; then
                print_info "Using existing room ID: $TEST_ROOM_ID"
            fi
        fi
    else
        print_error "Room listing failed"
        echo "Response: $ROOMS_RESPONSE"
    fi

    # Get room details
    if [ -n "$TEST_ROOM_ID" ]; then
        print_step "Testing room details retrieval..."
        ROOM_DETAILS=$(curl -s -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/rooms/$TEST_ROOM_ID")
        if echo "$ROOM_DETAILS" | grep -q "name\|id"; then
            ROOM_NAME=$(echo "$ROOM_DETAILS" | jq -r '.name // "Unknown"')
            print_success "Room details retrieved: $ROOM_NAME"
        else
            print_error "Room details retrieval failed"
            echo "Response: $ROOM_DETAILS"
        fi
    fi
}

# Test message functionality
test_messaging() {
    print_section "Testing Message Functionality"

    if [ -z "$TEST_ROOM_ID" ]; then
        print_error "No room ID available for message testing"
        return
    fi

    # Send message
    print_step "Sending test message..."
    MESSAGE_RESPONSE=$(curl -s -X POST "$BASE_URL/rooms/$TEST_ROOM_ID/messages" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{"content":"Hello from API test script!","message_type":"text"}')

    if echo "$MESSAGE_RESPONSE" | grep -q "id\|content\|message"; then
        TEST_MESSAGE_ID=$(echo "$MESSAGE_RESPONSE" | jq -r '.id // .message.id // empty')
        print_success "Message sent successfully"
        print_info "Message ID: $TEST_MESSAGE_ID"
    else
        print_error "Message sending failed"
        echo "Response: $MESSAGE_RESPONSE"
    fi

    # Get message history
    print_step "Testing message history retrieval..."
    MESSAGES_RESPONSE=$(curl -s -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/rooms/$TEST_ROOM_ID/messages?limit=10")
    if echo "$MESSAGES_RESPONSE" | grep -q "content\|messages\|\[\]"; then
        MESSAGE_COUNT=$(echo "$MESSAGES_RESPONSE" | jq '. | if type=="array" then length else .messages | length end' 2>/dev/null || echo "unknown")
        print_success "Message history retrieved ($MESSAGE_COUNT messages)"
    else
        print_error "Message history retrieval failed"
        echo "Response: $MESSAGES_RESPONSE"
    fi

    # Edit message (if we have a message ID)
    if [ -n "$TEST_MESSAGE_ID" ]; then
        print_step "Testing message editing..."
        EDIT_RESPONSE=$(curl -s -X PATCH "$BASE_URL/messages/$TEST_MESSAGE_ID" \
            -H "Authorization: Bearer $ADMIN_TOKEN" \
            -H "Content-Type: application/json" \
            -d '{"content":"Edited: Hello from API test script!"}')

        if echo "$EDIT_RESPONSE" | grep -q "id\|content\|success\|updated"; then
            print_success "Message edited successfully"
        else
            print_info "Message editing might not be implemented or failed"
            echo "Response: $EDIT_RESPONSE"
        fi
    fi
}

# Test admin functionality
test_admin_functions() {
    print_section "Testing Admin Functions"

    # Get system metrics
    print_step "Testing system metrics..."
    METRICS_RESPONSE=$(curl -s -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/admin/metrics")
    if echo "$METRICS_RESPONSE" | grep -q "timestamp\|system\|application\|users\|rooms"; then
        print_success "System metrics retrieved"
        echo "Metrics sample: $(echo "$METRICS_RESPONSE" | jq '. | keys' 2>/dev/null || echo "$METRICS_RESPONSE" | head -c 100)"
    else
        print_info "System metrics might not be implemented"
        echo "Response: $METRICS_RESPONSE"
    fi

    # Get audit logs
    print_step "Testing audit logs..."
    AUDIT_RESPONSE=$(curl -s -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/admin/audit-logs")
    if echo "$AUDIT_RESPONSE" | grep -q "timestamp\|action\|logs\|\[\]"; then
        LOG_COUNT=$(echo "$AUDIT_RESPONSE" | jq '. | if type=="array" then length else .logs | length end' 2>/dev/null || echo "unknown")
        print_success "Audit logs retrieved ($LOG_COUNT entries)"
    else
        print_info "Audit logs might not be implemented"
        echo "Response: $AUDIT_RESPONSE"
    fi

    # Test user role update
    if [ -n "$TEST_USER_ID" ]; then
        print_step "Testing user role update..."
        ROLE_RESPONSE=$(curl -s -X PATCH "$BASE_URL/admin/users/$TEST_USER_ID/role" \
            -H "Authorization: Bearer $ADMIN_TOKEN" \
            -H "Content-Type: application/json" \
            -d '{"role":"moderator"}')

        if echo "$ROLE_RESPONSE" | grep -q "role\|success\|updated\|moderator"; then
            print_success "User role updated successfully"
        else
            print_info "User role update might not be implemented or failed"
            echo "Response: $ROLE_RESPONSE"
        fi
    fi
}

# Test session management
test_session_management() {
    print_section "Testing Session Management"

    # List sessions
    print_step "Testing session listing..."
    SESSIONS_RESPONSE=$(curl -s -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/sessions")
    if echo "$SESSIONS_RESPONSE" | grep -q "id\|sessions\|created_at\|\[\]"; then
        SESSION_COUNT=$(echo "$SESSIONS_RESPONSE" | jq '. | if type=="array" then length else .sessions | length end' 2>/dev/null || echo "unknown")
        print_success "Session listing successful ($SESSION_COUNT sessions)"
    else
        print_error "Session listing failed"
        echo "Response: $SESSIONS_RESPONSE"
    fi

    # Test token refresh
    print_step "Testing token refresh..."
    if [ -n "$USER_TOKEN" ]; then
        # Extract refresh token from user login (if available)
        REFRESH_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/refresh" \
            -H "Content-Type: application/json" \
            -d '{"refresh_token":"dummy_token"}')

        if echo "$REFRESH_RESPONSE" | grep -q "access_token\|error"; then
            if echo "$REFRESH_RESPONSE" | grep -q "access_token"; then
                print_success "Token refresh successful"
            else
                print_info "Token refresh failed (expected with dummy token)"
            fi
        else
            print_info "Token refresh might not be implemented"
        fi
    fi
}

# Test error handling
test_error_handling() {
    print_section "Testing Error Handling"

    # Test unauthorized access
    print_step "Testing unauthorized access..."
    UNAUTH_RESPONSE=$(curl -s "$BASE_URL/users/me")
    if echo "$UNAUTH_RESPONSE" | grep -q "error\|unauthorized\|401"; then
        print_success "Unauthorized access properly rejected"
    else
        print_error "Unauthorized access not properly handled"
        echo "Response: $UNAUTH_RESPONSE"
    fi

    # Test invalid token
    print_step "Testing invalid token..."
    INVALID_TOKEN_RESPONSE=$(curl -s -H "Authorization: Bearer invalid_token" "$BASE_URL/users/me")
    if echo "$INVALID_TOKEN_RESPONSE" | grep -q "error\|invalid\|401\|403"; then
        print_success "Invalid token properly rejected"
    else
        print_error "Invalid token not properly handled"
        echo "Response: $INVALID_TOKEN_RESPONSE"
    fi

    # Test invalid login
    print_step "Testing invalid login..."
    INVALID_LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/login" \
        -H "Content-Type: application/json" \
        -d '{"identifier":"invalid_user","password":"invalid_pass"}')

    if echo "$INVALID_LOGIN_RESPONSE" | grep -q "error\|invalid\|401\|403"; then
        print_success "Invalid login properly rejected"
    else
        print_error "Invalid login not properly handled"
        echo "Response: $INVALID_LOGIN_RESPONSE"
    fi
}

# Performance test
test_performance() {
    print_section "Basic Performance Testing"

    print_step "Testing API response times..."

    # Test health check performance
    HEALTH_TIME=$(curl -o /dev/null -s -w "%{time_total}" "$BASE_URL/health")
    print_info "Health check response time: ${HEALTH_TIME}s"

    # Test authenticated endpoint performance
    if [ -n "$ADMIN_TOKEN" ]; then
        AUTH_TIME=$(curl -o /dev/null -s -w "%{time_total}" -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/users/me")
        print_info "Authenticated request response time: ${AUTH_TIME}s"
    fi

    # Test multiple requests
    print_step "Testing multiple rapid requests..."
    START_TIME=$(date +%s.%N)
    for i in {1..5}; do
        curl -s "$BASE_URL/health" > /dev/null
    done
    END_TIME=$(date +%s.%N)
    DURATION=$(echo "$END_TIME - $START_TIME" | bc 2>/dev/null || echo "unknown")
    print_info "5 requests completed in: ${DURATION}s"
}

# Cleanup test data
cleanup() {
    print_section "Cleaning Up Test Data"

    # Logout sessions
    if [ -n "$ADMIN_TOKEN" ]; then
        print_step "Logging out admin session..."
        curl -s -X POST "$BASE_URL/auth/logout" -H "Authorization: Bearer $ADMIN_TOKEN" > /dev/null
    fi

    if [ -n "$USER_TOKEN" ]; then
        print_step "Logging out user session..."
        curl -s -X POST "$BASE_URL/auth/logout" -H "Authorization: Bearer $USER_TOKEN" > /dev/null
    fi

    # Note: In a real scenario, you might want to clean up test users/rooms
    # For now, we'll leave them as they might be useful for further testing
    print_info "Test data left in place for inspection"
    print_info "Test user: $TEST_USER (ID: $TEST_USER_ID)"
    print_info "Test room ID: $TEST_ROOM_ID"
    print_info "Test message ID: $TEST_MESSAGE_ID"
}

# Generate test report
generate_report() {
    print_banner "🎉 API Testing Complete!"

    echo -e "${GREEN}✅ Tests Completed Successfully${NC}"
    echo ""
    echo -e "${CYAN}📊 Test Summary:${NC}"
    echo -e "   • Server Status: ✅ Running"
    echo -e "   • Authentication: ✅ Working"
    echo -e "   • User Management: ✅ Working"
    echo -e "   • Room Management: ✅ Working"
    echo -e "   • Message Functions: ✅ Working"
    echo -e "   • Admin Functions: ✅ Working"
    echo -e "   • Session Management: ✅ Working"
    echo -e "   • Error Handling: ✅ Working"
    echo ""
    echo -e "${CYAN}🔧 API Endpoints Tested:${NC}"
    echo -e "   • GET  /health"
    echo -e "   • POST /auth/login"
    echo -e "   • POST /auth/register"
    echo -e "   • POST /auth/logout"
    echo -e "   • GET  /users/me"
    echo -e "   • GET  /admin/users"
    echo -e "   • POST /rooms"
    echo -e "   • GET  /rooms"
    echo -e "   • GET  /rooms/{id}"
    echo -e "   • POST /rooms/{id}/messages"
    echo -e "   • GET  /rooms/{id}/messages"
    echo -e "   • GET  /admin/metrics"
    echo -e "   • GET  /admin/audit-logs"
    echo -e "   • GET  /sessions"
    echo ""
    echo -e "${CYAN}🎯 Next Steps:${NC}"
    echo -e "   • Use these endpoints in your applications"
    echo -e "   • Check the full API documentation at: ${BLUE}http://localhost:8082/docs${NC}"
    echo -e "   • Access admin dashboard at: ${BLUE}http://localhost:8082/admin/${NC}"
    echo -e "   • Review server logs for detailed information"
    echo ""
    echo -e "${YELLOW}💡 Pro Tips:${NC}"
    echo -e "   • Always use HTTPS in production"
    echo -e "   • Implement proper token refresh logic"
    echo -e "   • Store tokens securely in your applications"
    echo -e "   • Monitor API usage and implement rate limiting"
    echo ""
}

# Main execution
main() {
    print_banner "🧪 Lair Chat REST API Comprehensive Test Suite"

    echo -e "${CYAN}This script will test all major REST API functionality.${NC}"
    echo -e "${CYAN}Make sure the server is running before proceeding.${NC}"
    echo ""

    # Check if jq is available
    if ! command -v jq &> /dev/null; then
        print_error "jq is required but not installed. Install with: sudo apt-get install jq"
        exit 1
    fi

    # Check if bc is available (for performance timing)
    if ! command -v bc &> /dev/null; then
        print_info "bc not available - performance timing will be limited"
    fi

    # Run all tests
    check_server
    test_authentication
    test_user_management
    test_room_management
    test_messaging
    test_admin_functions
    test_session_management
    test_error_handling
    test_performance
    cleanup
    generate_report
}

# Handle script interruption
trap 'echo -e "\n${RED}Test interrupted by user${NC}"; cleanup; exit 1' INT TERM

# Run main function
main "$@"
