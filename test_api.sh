#!/bin/bash

# Lair Chat API Testing Script
# This script tests the REST API endpoints to verify functionality

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# API Configuration
API_BASE="http://127.0.0.1:8082/api/v1"
ACCESS_TOKEN=""

# Function to print colored output
print_header() {
    echo -e "\n${PURPLE}===============================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}===============================================${NC}"
}

print_test() {
    echo -e "\n${BLUE}[TEST]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_info() {
    echo -e "${YELLOW}[INFO]${NC} $1"
}

# Function to make API calls
api_call() {
    local method="$1"
    local endpoint="$2"
    local data="$3"
    local headers="$4"

    local url="${API_BASE}${endpoint}"
    local curl_cmd="curl -s -w '%{http_code}'"

    if [ -n "$headers" ]; then
        curl_cmd="$curl_cmd $headers"
    fi

    if [ -n "$ACCESS_TOKEN" ]; then
        curl_cmd="$curl_cmd -H 'Authorization: Bearer $ACCESS_TOKEN'"
    fi

    curl_cmd="$curl_cmd -H 'Content-Type: application/json'"

    if [ "$method" != "GET" ]; then
        curl_cmd="$curl_cmd -X $method"
    fi

    if [ -n "$data" ]; then
        curl_cmd="$curl_cmd -d '$data'"
    fi

    curl_cmd="$curl_cmd '$url'"

    eval $curl_cmd
}

# Function to extract JSON field
extract_json_field() {
    local json="$1"
    local field="$2"
    echo "$json" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('$field', ''))
except:
    pass
" 2>/dev/null || echo ""
}

print_header "🚀 Lair Chat API Testing Suite"

# Test 1: Health Check
print_test "Health Check"
response=$(api_call "GET" "/health")
status_code="${response: -3}"
body="${response%???}"

if [ "$status_code" = "200" ]; then
    print_success "Health check passed (Status: $status_code)"
    echo "Response: $body"
else
    print_error "Health check failed (Status: $status_code)"
    echo "Response: $body"
fi

# Test 2: API Documentation
print_test "API Documentation Endpoint"
response=$(api_call "GET" "/docs")
status_code="${response: -3}"

if [ "$status_code" = "200" ] || [ "$status_code" = "302" ]; then
    print_success "API docs endpoint accessible (Status: $status_code)"
else
    print_info "API docs may not be available (Status: $status_code)"
fi

print_header "🔐 Authentication Tests"

# Test 3: User Registration
print_test "User Registration"
registration_data='{
    "username": "testuser_' $(date +%s) '",
    "email": "test' $(date +%s) '@example.com",
    "password": "TestPassword123!",
    "display_name": "Test User"
}'

response=$(api_call "POST" "/auth/register" "$registration_data")
status_code="${response: -3}"
body="${response%???}"

if [ "$status_code" = "201" ]; then
    print_success "User registration successful (Status: $status_code)"
    echo "Registration response: $body"

    # Extract token from response
    ACCESS_TOKEN=$(echo "$body" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('access_token', ''))
except:
    pass
" 2>/dev/null)

    if [ -n "$ACCESS_TOKEN" ]; then
        print_success "Access token extracted: ${ACCESS_TOKEN:0:20}..."
    fi
else
    print_error "User registration failed (Status: $status_code)"
    echo "Response: $body"
fi

# Test 4: Admin User Login (if available)
print_test "Admin User Login"
admin_login_data='{
    "identifier": "admin",
    "password": "AdminPassword123!",
    "remember_me": true
}'

response=$(api_call "POST" "/auth/login" "$admin_login_data")
status_code="${response: -3}"
body="${response%???}"

if [ "$status_code" = "200" ]; then
    print_success "Admin login successful (Status: $status_code)"

    # Extract admin token
    ADMIN_TOKEN=$(echo "$body" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('access_token', ''))
except:
    pass
" 2>/dev/null)

    if [ -n "$ADMIN_TOKEN" ]; then
        print_success "Admin token extracted: ${ADMIN_TOKEN:0:20}..."

        # Check if user is admin
        USER_ROLE=$(echo "$body" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('user', {}).get('role', ''))
except:
    pass
" 2>/dev/null)

        if [ "$USER_ROLE" = "admin" ]; then
            print_success "User has admin role"
            ACCESS_TOKEN="$ADMIN_TOKEN"  # Use admin token for subsequent tests
        else
            print_info "User role: $USER_ROLE (not admin)"
        fi
    fi
else
    print_info "Admin login failed or admin user not available (Status: $status_code)"
    echo "Response: $body"
fi

print_header "🔒 Protected Endpoint Tests"

if [ -n "$ACCESS_TOKEN" ]; then
    # Test 5: User Profile
    print_test "Get User Profile"
    response=$(api_call "GET" "/users/profile")
    status_code="${response: -3}"
    body="${response%???}"

    if [ "$status_code" = "200" ]; then
        print_success "Profile retrieval successful (Status: $status_code)"
        echo "Profile: $body"
    else
        print_error "Profile retrieval failed (Status: $status_code)"
        echo "Response: $body"
    fi

    # Test 6: Admin Endpoints (if admin token available)
    if [ "$USER_ROLE" = "admin" ]; then
        print_test "Admin - Server Statistics"
        response=$(api_call "GET" "/admin/stats")
        status_code="${response: -3}"
        body="${response%???}"

        if [ "$status_code" = "200" ]; then
            print_success "Admin stats retrieval successful (Status: $status_code)"
            echo "Stats: $body"
        else
            print_error "Admin stats retrieval failed (Status: $status_code)"
            echo "Response: $body"
        fi

        print_test "Admin - System Health"
        response=$(api_call "GET" "/admin/health")
        status_code="${response: -3}"
        body="${response%???}"

        if [ "$status_code" = "200" ]; then
            print_success "Admin health check successful (Status: $status_code)"
            echo "Health: $body"
        else
            print_error "Admin health check failed (Status: $status_code)"
            echo "Response: $body"
        fi

        print_test "Admin - User Management"
        response=$(api_call "GET" "/admin/users")
        status_code="${response: -3}"
        body="${response%???}"

        if [ "$status_code" = "200" ]; then
            print_success "Admin user list retrieval successful (Status: $status_code)"
            echo "Users: $body"
        else
            print_error "Admin user list retrieval failed (Status: $status_code)"
            echo "Response: $body"
        fi
    else
        print_info "Skipping admin tests - no admin privileges"
    fi

else
    print_info "Skipping protected endpoint tests - no access token available"
fi

print_header "🏠 Room Management Tests"

if [ -n "$ACCESS_TOKEN" ]; then
    # Test 7: Create Room
    print_test "Create Room"
    room_data='{
        "name": "test-room-' $(date +%s) '",
        "description": "Test room created by API test",
        "is_private": false
    }'

    response=$(api_call "POST" "/rooms" "$room_data")
    status_code="${response: -3}"
    body="${response%???}"

    if [ "$status_code" = "201" ]; then
        print_success "Room creation successful (Status: $status_code)"
        echo "Room: $body"

        # Extract room ID for further tests
        ROOM_ID=$(echo "$body" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('data', {}).get('id', ''))
except:
    pass
" 2>/dev/null)

        if [ -n "$ROOM_ID" ]; then
            print_success "Room ID extracted: $ROOM_ID"
        fi
    else
        print_error "Room creation failed (Status: $status_code)"
        echo "Response: $body"
    fi

    # Test 8: List Rooms
    print_test "List Rooms"
    response=$(api_call "GET" "/rooms")
    status_code="${response: -3}"
    body="${response%???}"

    if [ "$status_code" = "200" ]; then
        print_success "Room listing successful (Status: $status_code)"
        echo "Rooms: $body"
    else
        print_error "Room listing failed (Status: $status_code)"
        echo "Response: $body"
    fi
fi

print_header "📨 Message Tests"

if [ -n "$ACCESS_TOKEN" ] && [ -n "$ROOM_ID" ]; then
    # Test 9: Send Message
    print_test "Send Message"
    message_data='{
        "content": "Hello, this is a test message from API test suite!",
        "room_id": "' $ROOM_ID '"
    }'

    response=$(api_call "POST" "/messages" "$message_data")
    status_code="${response: -3}"
    body="${response%???}"

    if [ "$status_code" = "201" ]; then
        print_success "Message sending successful (Status: $status_code)"
        echo "Message: $body"
    else
        print_error "Message sending failed (Status: $status_code)"
        echo "Response: $body"
    fi

    # Test 10: Get Messages
    print_test "Get Room Messages"
    response=$(api_call "GET" "/messages?room_id=$ROOM_ID&limit=10")
    status_code="${response: -3}"
    body="${response%???}"

    if [ "$status_code" = "200" ]; then
        print_success "Message retrieval successful (Status: $status_code)"
        echo "Messages: $body"
    else
        print_error "Message retrieval failed (Status: $status_code)"
        echo "Response: $body"
    fi
fi

print_header "🔄 Token Management Tests"

if [ -n "$ACCESS_TOKEN" ]; then
    # Test 11: Token Refresh (if refresh token available)
    print_test "Token Refresh"
    print_info "Note: Refresh token tests require implementation of refresh token storage"

    # Test 12: Logout
    print_test "User Logout"
    response=$(api_call "POST" "/auth/logout" '{"all_devices": false}')
    status_code="${response: -3}"
    body="${response%???}"

    if [ "$status_code" = "200" ]; then
        print_success "Logout successful (Status: $status_code)"
        echo "Response: $body"
    else
        print_error "Logout failed (Status: $status_code)"
        echo "Response: $body"
    fi
fi

print_header "📊 Test Summary"

echo -e "\n${BLUE}Test execution completed!${NC}"
echo -e "${YELLOW}Key findings:${NC}"

if [ -n "$ACCESS_TOKEN" ]; then
    echo -e "${GREEN}✅ Authentication system is working${NC}"
else
    echo -e "${RED}❌ Authentication system needs attention${NC}"
fi

if [ "$USER_ROLE" = "admin" ]; then
    echo -e "${GREEN}✅ Admin functionality is available${NC}"
else
    echo -e "${YELLOW}⚠️  Admin functionality not tested${NC}"
fi

echo -e "\n${BLUE}Next steps:${NC}"
echo -e "1. Check server logs for any errors: tail -f logs/server.log"
echo -e "2. Try the admin dashboard: http://127.0.0.1:8083"
echo -e "3. Review API documentation: http://127.0.0.1:8082/swagger-ui"
echo -e "4. Run debug scripts: cargo run --bin debug_jwt_auth"

echo -e "\n${PURPLE}Happy testing! 🚀${NC}"
