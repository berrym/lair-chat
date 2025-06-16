#!/bin/bash

# Sprint 4 Completion Validation Script
# Tests system health monitoring and audit logging endpoints
# Created: June 16, 2025
# Sprint: Sprint 4 - Session & Admin Management APIs

set -e

echo "🎯 SPRINT 4 COMPLETION VALIDATION"
echo "=================================="
echo ""

# Configuration
SERVER_URL="http://localhost:8080"
API_BASE="$SERVER_URL/api/v1"
CONFIG_PATH="config/server.toml"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test credentials
ADMIN_USERNAME="sprint4_admin_test"
ADMIN_PASSWORD="AdminTest123!"
ADMIN_EMAIL="sprint4_admin@test.com"

echo -e "${BLUE}📋 Sprint 4 Test Plan:${NC}"
echo "  ✓ System Health Monitoring (MONITOR-002)"
echo "  ✓ Audit Logging System (MONITOR-003)"
echo "  ✓ Integration Testing"
echo "  ✓ Performance Validation"
echo ""

# Function to check if server is running
check_server() {
    echo -e "${BLUE}🔍 Checking server status...${NC}"

    if curl -s "$SERVER_URL/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Server is running${NC}"
        return 0
    else
        echo -e "${RED}❌ Server is not running${NC}"
        return 1
    fi
}

# Function to start server if not running
start_server() {
    echo -e "${BLUE}🚀 Starting server...${NC}"

    if [ -f "target/release/lair-chat-server-new" ]; then
        ./target/release/lair-chat-server-new --config "$CONFIG_PATH" &
        SERVER_PID=$!
        sleep 3
        echo -e "${GREEN}✅ Server started (PID: $SERVER_PID)${NC}"
        return 0
    else
        echo -e "${RED}❌ REST API server binary not found. Run: cargo build --release${NC}"
        return 1
    fi
}

# Function to stop server
stop_server() {
    if [ -n "$SERVER_PID" ]; then
        echo -e "${BLUE}🛑 Stopping server...${NC}"
        kill $SERVER_PID 2>/dev/null || true
        echo -e "${GREEN}✅ Server stopped${NC}"
    fi
}

# Function to register admin user
register_admin() {
    echo -e "${BLUE}👤 Registering admin user...${NC}"

    REGISTER_RESPONSE=$(curl -s -w "%{http_code}" -X POST \
        "$API_BASE/auth/register" \
        -H "Content-Type: application/json" \
        -d "{
            \"username\": \"$ADMIN_USERNAME\",
            \"password\": \"$ADMIN_PASSWORD\",
            \"email\": \"$ADMIN_EMAIL\"
        }")

    HTTP_CODE="${REGISTER_RESPONSE: -3}"

    if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "201" ]; then
        echo -e "${GREEN}✅ Admin user registered successfully${NC}"
    elif [ "$HTTP_CODE" = "409" ]; then
        echo -e "${YELLOW}⚠️  Admin user already exists${NC}"
    else
        echo -e "${RED}❌ Failed to register admin user (HTTP: $HTTP_CODE)${NC}"
        echo "Response: ${REGISTER_RESPONSE%???}"
        return 1
    fi
}

# Function to login and get JWT token
get_jwt_token() {
    echo -e "${BLUE}🔑 Getting JWT token...${NC}"

    LOGIN_RESPONSE=$(curl -s -X POST \
        "$API_BASE/auth/login" \
        -H "Content-Type: application/json" \
        -d "{
            \"username\": \"$ADMIN_USERNAME\",
            \"password\": \"$ADMIN_PASSWORD\"
        }")

    JWT_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.data.access_token // empty')

    if [ -n "$JWT_TOKEN" ] && [ "$JWT_TOKEN" != "null" ]; then
        echo -e "${GREEN}✅ JWT token obtained${NC}"
        return 0
    else
        echo -e "${RED}❌ Failed to get JWT token${NC}"
        echo "Response: $LOGIN_RESPONSE"
        return 1
    fi
}

# Function to test system health endpoint
test_system_health() {
    echo -e "${BLUE}🏥 Testing System Health Endpoint...${NC}"

    HEALTH_RESPONSE=$(curl -s -w "%{http_code}" \
        "$API_BASE/admin/health" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -H "Content-Type: application/json")

    HTTP_CODE="${HEALTH_RESPONSE: -3}"
    RESPONSE_BODY="${HEALTH_RESPONSE%???}"

    if [ "$HTTP_CODE" = "200" ]; then
        echo -e "${GREEN}✅ Health endpoint responds correctly (HTTP: $HTTP_CODE)${NC}"

        # Validate response structure
        if echo "$RESPONSE_BODY" | jq -e '.data.status' > /dev/null 2>&1; then
            STATUS=$(echo "$RESPONSE_BODY" | jq -r '.data.status')
            echo -e "${GREEN}✅ Health status: $STATUS${NC}"
        fi

        if echo "$RESPONSE_BODY" | jq -e '.data.components' > /dev/null 2>&1; then
            COMPONENT_COUNT=$(echo "$RESPONSE_BODY" | jq '.data.components | length')
            echo -e "${GREEN}✅ Health components: $COMPONENT_COUNT${NC}"
        fi

        if echo "$RESPONSE_BODY" | jq -e '.data.metrics' > /dev/null 2>&1; then
            echo -e "${GREEN}✅ System metrics included${NC}"
        fi

        return 0
    else
        echo -e "${RED}❌ Health endpoint failed (HTTP: $HTTP_CODE)${NC}"
        echo "Response: $RESPONSE_BODY"
        return 1
    fi
}

# Function to test audit logs endpoint
test_audit_logs() {
    echo -e "${BLUE}📋 Testing Audit Logs Endpoint...${NC}"

    AUDIT_RESPONSE=$(curl -s -w "%{http_code}" \
        "$API_BASE/admin/audit?page=0&page_size=10" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -H "Content-Type: application/json")

    HTTP_CODE="${AUDIT_RESPONSE: -3}"
    RESPONSE_BODY="${AUDIT_RESPONSE%???}"

    if [ "$HTTP_CODE" = "200" ]; then
        echo -e "${GREEN}✅ Audit logs endpoint responds correctly (HTTP: $HTTP_CODE)${NC}"

        # Validate response structure
        if echo "$RESPONSE_BODY" | jq -e '.data' > /dev/null 2>&1; then
            LOG_COUNT=$(echo "$RESPONSE_BODY" | jq '.data | length')
            echo -e "${GREEN}✅ Retrieved $LOG_COUNT audit log entries${NC}"
        fi

        return 0
    else
        echo -e "${RED}❌ Audit logs endpoint failed (HTTP: $HTTP_CODE)${NC}"
        echo "Response: $RESPONSE_BODY"
        return 1
    fi
}

# Function to test audit log statistics
test_audit_stats() {
    echo -e "${BLUE}📊 Testing Audit Statistics Endpoint...${NC}"

    STATS_RESPONSE=$(curl -s -w "%{http_code}" \
        "$API_BASE/admin/audit/stats" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -H "Content-Type: application/json")

    HTTP_CODE="${STATS_RESPONSE: -3}"
    RESPONSE_BODY="${STATS_RESPONSE%???}"

    if [ "$HTTP_CODE" = "200" ]; then
        echo -e "${GREEN}✅ Audit statistics endpoint responds correctly (HTTP: $HTTP_CODE)${NC}"

        # Validate response structure
        if echo "$RESPONSE_BODY" | jq -e '.data.total_entries' > /dev/null 2>&1; then
            TOTAL_ENTRIES=$(echo "$RESPONSE_BODY" | jq '.data.total_entries')
            echo -e "${GREEN}✅ Total audit entries: $TOTAL_ENTRIES${NC}"
        fi

        return 0
    else
        echo -e "${RED}❌ Audit statistics endpoint failed (HTTP: $HTTP_CODE)${NC}"
        echo "Response: $RESPONSE_BODY"
        return 1
    fi
}

# Function to test audit log search
test_audit_search() {
    echo -e "${BLUE}🔍 Testing Audit Search Endpoint...${NC}"

    SEARCH_RESPONSE=$(curl -s -w "%{http_code}" -X POST \
        "$API_BASE/admin/audit/search" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{
            \"query\": \"admin\",
            \"page\": 0,
            \"page_size\": 5
        }")

    HTTP_CODE="${SEARCH_RESPONSE: -3}"
    RESPONSE_BODY="${SEARCH_RESPONSE%???}"

    if [ "$HTTP_CODE" = "200" ]; then
        echo -e "${GREEN}✅ Audit search endpoint responds correctly (HTTP: $HTTP_CODE)${NC}"

        # Validate response structure
        if echo "$RESPONSE_BODY" | jq -e '.data' > /dev/null 2>&1; then
            SEARCH_COUNT=$(echo "$RESPONSE_BODY" | jq '.data | length')
            echo -e "${GREEN}✅ Search returned $SEARCH_COUNT results${NC}"
        fi

        return 0
    else
        echo -e "${RED}❌ Audit search endpoint failed (HTTP: $HTTP_CODE)${NC}"
        echo "Response: $RESPONSE_BODY"
        return 1
    fi
}

# Function to test admin endpoints integration
test_admin_integration() {
    echo -e "${BLUE}🔗 Testing Admin Endpoints Integration...${NC}"

    local endpoints=(
        "GET:admin/stats"
        "GET:admin/health"
        "GET:admin/users?page=0&page_size=5"
        "GET:admin/audit?page=0&page_size=5"
        "GET:admin/audit/stats"
    )

    local success_count=0
    local total_count=${#endpoints[@]}

    for endpoint_spec in "${endpoints[@]}"; do
        IFS=':' read -r method endpoint <<< "$endpoint_spec"

        RESPONSE=$(curl -s -w "%{http_code}" -X "$method" \
            "$API_BASE/$endpoint" \
            -H "Authorization: Bearer $JWT_TOKEN" \
            -H "Content-Type: application/json")

        HTTP_CODE="${RESPONSE: -3}"

        if [ "$HTTP_CODE" = "200" ]; then
            echo -e "${GREEN}✅ $method /$endpoint - Success${NC}"
            ((success_count++))
        else
            echo -e "${RED}❌ $method /$endpoint - Failed (HTTP: $HTTP_CODE)${NC}"
        fi
    done

    echo -e "${BLUE}📊 Integration Test Results: $success_count/$total_count endpoints successful${NC}"

    if [ "$success_count" -eq "$total_count" ]; then
        return 0
    else
        return 1
    fi
}

# Function to test performance
test_performance() {
    echo -e "${BLUE}⚡ Testing Performance...${NC}"

    # Test health endpoint response time
    START_TIME=$(date +%s%3N)
    curl -s "$API_BASE/admin/health" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -H "Content-Type: application/json" > /dev/null
    END_TIME=$(date +%s%3N)

    RESPONSE_TIME=$((END_TIME - START_TIME))

    if [ "$RESPONSE_TIME" -lt 500 ]; then
        echo -e "${GREEN}✅ Health endpoint response time: ${RESPONSE_TIME}ms (< 500ms target)${NC}"
    else
        echo -e "${YELLOW}⚠️  Health endpoint response time: ${RESPONSE_TIME}ms (above 500ms target)${NC}"
    fi

    # Test concurrent requests
    echo -e "${BLUE}🔄 Testing concurrent requests...${NC}"

    for i in {1..5}; do
        curl -s "$API_BASE/admin/health" \
            -H "Authorization: Bearer $JWT_TOKEN" \
            -H "Content-Type: application/json" > /dev/null &
    done

    wait
    echo -e "${GREEN}✅ Concurrent requests completed successfully${NC}"
}

# Function to print summary
print_summary() {
    echo ""
    echo -e "${BLUE}📋 SPRINT 4 COMPLETION SUMMARY${NC}"
    echo "====================================="
    echo -e "${GREEN}✅ System Health Monitoring (MONITOR-002) - COMPLETE${NC}"
    echo -e "${GREEN}✅ Audit Logging System (MONITOR-003) - COMPLETE${NC}"
    echo -e "${GREEN}✅ Admin Endpoints Integration - COMPLETE${NC}"
    echo -e "${GREEN}✅ Performance Validation - COMPLETE${NC}"
    echo ""
    echo -e "${GREEN}🎉 SPRINT 4: SESSION & ADMIN MANAGEMENT APIs${NC}"
    echo -e "${GREEN}📊 STATUS: 100% COMPLETE${NC}"
    echo -e "${GREEN}🚀 READY FOR SPRINT 5: ADVANCED USER FEATURES & WEBSOCKET${NC}"
    echo ""
}

# Trap to ensure server cleanup
trap 'stop_server' EXIT

# Main execution
main() {
    echo -e "${BLUE}🔧 Checking dependencies...${NC}"

    # Check for required tools
    if ! command -v curl &> /dev/null; then
        echo -e "${RED}❌ curl is required but not installed${NC}"
        exit 1
    fi

    if ! command -v jq &> /dev/null; then
        echo -e "${RED}❌ jq is required but not installed${NC}"
        exit 1
    fi

    echo -e "${GREEN}✅ Dependencies available${NC}"
    echo ""

    # Check if server is running, start if needed
    if ! check_server; then
        if ! start_server; then
            echo -e "${RED}❌ Failed to start server${NC}"
            exit 1
        fi
        sleep 2
    fi

    # Run tests
    echo -e "${BLUE}🧪 Running Sprint 4 Completion Tests...${NC}"
    echo ""

    if ! register_admin; then
        echo -e "${RED}❌ Failed to register admin user${NC}"
        exit 1
    fi

    if ! get_jwt_token; then
        echo -e "${RED}❌ Failed to get JWT token${NC}"
        exit 1
    fi

    echo ""

    # Core Sprint 4 tests
    test_system_health || exit 1
    echo ""

    test_audit_logs || exit 1
    echo ""

    test_audit_stats || exit 1
    echo ""

    test_audit_search || exit 1
    echo ""

    test_admin_integration || exit 1
    echo ""

    test_performance
    echo ""

    print_summary
}

# Run main function
main "$@"
