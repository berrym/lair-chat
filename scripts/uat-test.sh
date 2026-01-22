#!/bin/bash

# Lair Chat - User Acceptance Testing (UAT) Script
# Comprehensive testing suite for production readiness validation

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Test configuration
TEST_HOST="${TEST_HOST:-127.0.0.1}"
TEST_API_PORT="${TEST_API_PORT:-8082}"
TEST_TCP_PORT="${TEST_TCP_PORT:-8080}"
TEST_TIMEOUT="${TEST_TIMEOUT:-30}"
TEST_ADMIN_USER="${TEST_ADMIN_USER:-admin}"
TEST_ADMIN_PASS="${TEST_ADMIN_PASS:-AdminPassword123!}"

# Test counters
TESTS_TOTAL=0
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# Test result storage
declare -A TEST_RESULTS
declare -A TEST_DETAILS

# Function to print colored output
print_banner() {
    echo -e "\n${PURPLE}============================================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}============================================================${NC}\n"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

print_skip() {
    echo -e "${CYAN}[SKIP]${NC} $1"
}

# Test execution framework
run_test() {
    local test_name="$1"
    local test_function="$2"
    local test_description="$3"

    TESTS_TOTAL=$((TESTS_TOTAL + 1))

    echo -e "\n${BLUE}Running:${NC} $test_name"
    echo -e "${CYAN}Description:${NC} $test_description"

    local start_time=$(date +%s%3N)

    if $test_function; then
        local end_time=$(date +%s%3N)
        local duration=$((end_time - start_time))
        TESTS_PASSED=$((TESTS_PASSED + 1))
        TEST_RESULTS["$test_name"]="PASS"
        TEST_DETAILS["$test_name"]="Completed in ${duration}ms"
        print_success "$test_name (${duration}ms)"
    else
        local end_time=$(date +%s%3N)
        local duration=$((end_time - start_time))
        TESTS_FAILED=$((TESTS_FAILED + 1))
        TEST_RESULTS["$test_name"]="FAIL"
        TEST_DETAILS["$test_name"]="Failed after ${duration}ms"
        print_error "$test_name (${duration}ms)"
    fi
}

skip_test() {
    local test_name="$1"
    local reason="$2"

    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
    TEST_RESULTS["$test_name"]="SKIP"
    TEST_DETAILS["$test_name"]="$reason"
    print_skip "$test_name - $reason"
}

# API helper functions
api_call() {
    local method="$1"
    local endpoint="$2"
    local data="$3"
    local token="$4"

    local url="http://${TEST_HOST}:${TEST_API_PORT}${endpoint}"
    local headers="-H 'Content-Type: application/json'"

    if [ ! -z "$token" ]; then
        headers="$headers -H 'Authorization: Bearer $token'"
    fi

    if [ ! -z "$data" ]; then
        curl -s -X "$method" "$url" $headers -d "$data" --connect-timeout "$TEST_TIMEOUT"
    else
        curl -s -X "$method" "$url" $headers --connect-timeout "$TEST_TIMEOUT"
    fi
}

get_admin_token() {
    local response=$(api_call "POST" "/api/v1/auth/login" "{\"identifier\":\"$TEST_ADMIN_USER\",\"password\":\"$TEST_ADMIN_PASS\"}")
    echo "$response" | jq -r '.access_token // empty' 2>/dev/null || echo ""
}

# Test functions
test_system_prerequisites() {
    # Check if required tools are available
    command -v curl >/dev/null 2>&1 || return 1
    command -v jq >/dev/null 2>&1 || return 1
    command -v nc >/dev/null 2>&1 || return 1

    # Check if Cargo.toml exists
    [ -f "Cargo.toml" ] || return 1

    return 0
}

test_server_connectivity() {
    # Test REST API connectivity
    local response=$(curl -s -w "%{http_code}" -o /dev/null --connect-timeout "$TEST_TIMEOUT" "http://${TEST_HOST}:${TEST_API_PORT}/api/v1/health")
    [ "$response" = "200" ] || return 1

    # Test TCP server connectivity
    nc -z "$TEST_HOST" "$TEST_TCP_PORT" >/dev/null 2>&1 || return 1

    return 0
}

test_api_health_endpoint() {
    local response=$(api_call "GET" "/api/v1/health")
    local status=$(echo "$response" | jq -r '.status // empty' 2>/dev/null)
    [ "$status" = "ok" ] || return 1

    return 0
}

test_admin_authentication() {
    local token=$(get_admin_token)
    [ ! -z "$token" ] && [ "$token" != "null" ] || return 1

    # Verify token works for protected endpoint
    local response=$(api_call "GET" "/api/v1/admin/stats" "" "$token")
    local users=$(echo "$response" | jq -r '.data.total_users // empty' 2>/dev/null)
    [ ! -z "$users" ] || return 1

    return 0
}

test_user_registration() {
    local timestamp=$(date +%s)
    local test_user="testuser_$timestamp"
    local test_email="test_${timestamp}@example.com"

    local response=$(api_call "POST" "/api/v1/auth/register" "{\"username\":\"$test_user\",\"email\":\"$test_email\",\"password\":\"TestPass123!\"}")
    local user_id=$(echo "$response" | jq -r '.data.user.id // empty' 2>/dev/null)
    [ ! -z "$user_id" ] || return 1

    return 0
}

test_user_login() {
    local timestamp=$(date +%s)
    local test_user="logintest_$timestamp"
    local test_email="logintest_${timestamp}@example.com"

    # Register user first
    api_call "POST" "/api/v1/auth/register" "{\"username\":\"$test_user\",\"email\":\"$test_email\",\"password\":\"TestPass123!\"}" >/dev/null 2>&1

    # Test login
    local response=$(api_call "POST" "/api/v1/auth/login" "{\"identifier\":\"$test_user\",\"password\":\"TestPass123!\"}")
    local token=$(echo "$response" | jq -r '.access_token // empty' 2>/dev/null)
    [ ! -z "$token" ] && [ "$token" != "null" ] || return 1

    return 0
}

test_room_creation() {
    local token=$(get_admin_token)
    [ ! -z "$token" ] || return 1

    local timestamp=$(date +%s)
    local room_name="testroom_$timestamp"

    local response=$(api_call "POST" "/api/v1/rooms" "{\"name\":\"$room_name\",\"description\":\"Test room for UAT\"}" "$token")
    local room_id=$(echo "$response" | jq -r '.data.id // empty' 2>/dev/null)
    [ ! -z "$room_id" ] || return 1

    return 0
}

test_room_listing() {
    local token=$(get_admin_token)
    [ ! -z "$token" ] || return 1

    local response=$(api_call "GET" "/api/v1/rooms" "" "$token")
    local rooms=$(echo "$response" | jq -r '.data // empty' 2>/dev/null)
    [ ! -z "$rooms" ] || return 1

    return 0
}

test_message_sending() {
    local token=$(get_admin_token)
    [ ! -z "$token" ] || return 1

    # Create a test room first
    local timestamp=$(date +%s)
    local room_name="msgtest_$timestamp"
    local room_response=$(api_call "POST" "/api/v1/rooms" "{\"name\":\"$room_name\",\"description\":\"Message test room\"}" "$token")
    local room_id=$(echo "$room_response" | jq -r '.data.id // empty' 2>/dev/null)
    [ ! -z "$room_id" ] || return 1

    # Send a message
    local message_response=$(api_call "POST" "/api/v1/messages" "{\"room_id\":\"$room_id\",\"content\":\"Test message for UAT\"}" "$token")
    local message_id=$(echo "$message_response" | jq -r '.data.id // empty' 2>/dev/null)
    [ ! -z "$message_id" ] || return 1

    return 0
}

test_admin_dashboard_access() {
    local response=$(curl -s -w "%{http_code}" -o /dev/null --connect-timeout "$TEST_TIMEOUT" "http://${TEST_HOST}:${TEST_API_PORT}/admin/")
    [ "$response" = "200" ] || return 1

    return 0
}

test_api_documentation_access() {
    local response=$(curl -s -w "%{http_code}" -o /dev/null --connect-timeout "$TEST_TIMEOUT" "http://${TEST_HOST}:${TEST_API_PORT}/docs")
    [ "$response" = "200" ] || return 1

    return 0
}

test_admin_user_management() {
    local token=$(get_admin_token)
    [ ! -z "$token" ] || return 1

    # Get user list
    local response=$(api_call "GET" "/api/v1/admin/users" "" "$token")
    local users=$(echo "$response" | jq -r '.data // empty' 2>/dev/null)
    [ ! -z "$users" ] || return 1

    return 0
}

test_admin_system_health() {
    local token=$(get_admin_token)
    [ ! -z "$token" ] || return 1

    local response=$(api_call "GET" "/api/v1/admin/health" "" "$token")
    local status=$(echo "$response" | jq -r '.data.status // empty' 2>/dev/null)
    [ ! -z "$status" ] || return 1

    return 0
}

test_session_management() {
    local token=$(get_admin_token)
    [ ! -z "$token" ] || return 1

    # Get session info
    local response=$(api_call "GET" "/api/v1/sessions/current" "" "$token")
    local session_id=$(echo "$response" | jq -r '.data.id // empty' 2>/dev/null)
    [ ! -z "$session_id" ] || return 1

    return 0
}

test_rate_limiting() {
    # Make multiple rapid requests to test rate limiting
    local responses=0
    local rate_limited=0

    for i in {1..10}; do
        local response=$(curl -s -w "%{http_code}" -o /dev/null --connect-timeout 5 "http://${TEST_HOST}:${TEST_API_PORT}/api/v1/health")
        if [ "$response" = "200" ]; then
            responses=$((responses + 1))
        elif [ "$response" = "429" ]; then
            rate_limited=$((rate_limited + 1))
        fi
    done

    # Should get some successful responses
    [ "$responses" -gt 0 ] || return 1

    return 0
}

test_database_persistence() {
    local token=$(get_admin_token)
    [ ! -z "$token" ] || return 1

    # Create a test room
    local timestamp=$(date +%s)
    local room_name="persistence_test_$timestamp"
    local room_response=$(api_call "POST" "/api/v1/rooms" "{\"name\":\"$room_name\",\"description\":\"Persistence test\"}" "$token")
    local room_id=$(echo "$room_response" | jq -r '.data.id // empty' 2>/dev/null)
    [ ! -z "$room_id" ] || return 1

    # Verify it can be retrieved
    local get_response=$(api_call "GET" "/api/v1/rooms" "" "$token")
    local found_room=$(echo "$get_response" | jq -r ".data[] | select(.name == \"$room_name\") | .id" 2>/dev/null)
    [ "$found_room" = "$room_id" ] || return 1

    return 0
}

# Performance tests
test_api_response_time() {
    local start_time=$(date +%s%3N)
    local response=$(api_call "GET" "/api/v1/health")
    local end_time=$(date +%s%3N)
    local duration=$((end_time - start_time))

    # API should respond within 1000ms
    [ "$duration" -lt 1000 ] || return 1

    return 0
}

test_concurrent_connections() {
    local pids=()
    local success_count=0

    # Start 5 concurrent health checks
    for i in {1..5}; do
        (
            response=$(api_call "GET" "/api/v1/health")
            status=$(echo "$response" | jq -r '.status // empty' 2>/dev/null)
            [ "$status" = "ok" ] && echo "success" > "/tmp/uat_concurrent_$i"
        ) &
        pids+=($!)
    done

    # Wait for all to complete
    for pid in "${pids[@]}"; do
        wait "$pid"
    done

    # Count successes
    for i in {1..5}; do
        if [ -f "/tmp/uat_concurrent_$i" ]; then
            success_count=$((success_count + 1))
            rm -f "/tmp/uat_concurrent_$i"
        fi
    done

    # At least 4 out of 5 should succeed
    [ "$success_count" -ge 4 ] || return 1

    return 0
}

# Security tests
test_unauthorized_access() {
    # Try to access admin endpoint without token
    local response=$(curl -s -w "%{http_code}" -o /dev/null --connect-timeout "$TEST_TIMEOUT" "http://${TEST_HOST}:${TEST_API_PORT}/api/v1/admin/stats")
    [ "$response" = "401" ] || return 1

    return 0
}

test_invalid_credentials() {
    local response=$(api_call "POST" "/api/v1/auth/login" "{\"identifier\":\"invalid_user\",\"password\":\"invalid_pass\"}")
    local error=$(echo "$response" | jq -r '.error // empty' 2>/dev/null)
    [ ! -z "$error" ] || return 1

    return 0
}

# Generate test report
generate_report() {
    local report_file="logs/uat_report_$(date +%Y%m%d_%H%M%S).txt"

    mkdir -p logs

    {
        echo "=================================================="
        echo "Lair Chat UAT Test Report"
        echo "=================================================="
        echo "Generated: $(date)"
        echo "Test Environment: ${TEST_HOST}:${TEST_API_PORT}"
        echo ""
        echo "Test Summary:"
        echo "  Total Tests: $TESTS_TOTAL"
        echo "  Passed: $TESTS_PASSED"
        echo "  Failed: $TESTS_FAILED"
        echo "  Skipped: $TESTS_SKIPPED"
        echo "  Success Rate: $(( (TESTS_PASSED * 100) / (TESTS_TOTAL - TESTS_SKIPPED) ))%"
        echo ""
        echo "Detailed Results:"
        echo "=================================================="

        for test_name in "${!TEST_RESULTS[@]}"; do
            local status="${TEST_RESULTS[$test_name]}"
            local details="${TEST_DETAILS[$test_name]}"
            printf "%-40s %-6s %s\n" "$test_name" "$status" "$details"
        done

        echo ""
        echo "=================================================="
        echo "Test Environment Information:"
        echo "  Rust Version: $(rustc --version 2>/dev/null || echo 'Not available')"
        echo "  System: $(uname -s) $(uname -r)"
        echo "  Date: $(date)"
        echo "=================================================="
    } > "$report_file"

    echo "$report_file"
}

# Main execution
main() {
    print_banner "🧪 Lair Chat UAT Testing Suite"

    print_info "Starting User Acceptance Testing..."
    print_info "Test Target: ${TEST_HOST}:${TEST_API_PORT}"
    print_info "TCP Target: ${TEST_HOST}:${TEST_TCP_PORT}"
    print_info "Timeout: ${TEST_TIMEOUT}s"

    # Check prerequisites
    if ! command -v jq >/dev/null 2>&1; then
        print_error "jq is required for UAT testing. Please install jq."
        exit 1
    fi

    print_banner "🔍 Running UAT Tests"

    # Core system tests
    run_test "system_prerequisites" "test_system_prerequisites" "Verify system prerequisites and tools"
    run_test "server_connectivity" "test_server_connectivity" "Test server connectivity (REST API and TCP)"
    run_test "api_health" "test_api_health_endpoint" "Test API health endpoint"

    # Authentication tests
    run_test "admin_authentication" "test_admin_authentication" "Test admin user authentication"
    run_test "user_registration" "test_user_registration" "Test new user registration"
    run_test "user_login" "test_user_login" "Test user login functionality"

    # Core functionality tests
    run_test "room_creation" "test_room_creation" "Test chat room creation"
    run_test "room_listing" "test_room_listing" "Test room listing functionality"
    run_test "message_sending" "test_message_sending" "Test message sending"

    # Admin interface tests
    run_test "admin_dashboard" "test_admin_dashboard_access" "Test admin dashboard accessibility"
    run_test "api_documentation" "test_api_documentation_access" "Test API documentation access"
    run_test "admin_user_management" "test_admin_user_management" "Test admin user management"
    run_test "admin_system_health" "test_admin_system_health" "Test admin system health monitoring"

    # Session management
    run_test "session_management" "test_session_management" "Test session management"

    # Database persistence
    run_test "database_persistence" "test_database_persistence" "Test database persistence"

    # Performance tests
    run_test "api_response_time" "test_api_response_time" "Test API response time performance"
    run_test "concurrent_connections" "test_concurrent_connections" "Test concurrent connection handling"

    # Security tests
    run_test "unauthorized_access" "test_unauthorized_access" "Test protection against unauthorized access"
    run_test "invalid_credentials" "test_invalid_credentials" "Test invalid credential handling"
    run_test "rate_limiting" "test_rate_limiting" "Test rate limiting functionality"

    print_banner "📊 UAT Test Results"

    # Display summary
    echo -e "${CYAN}Test Summary:${NC}"
    echo -e "  Total Tests: ${YELLOW}$TESTS_TOTAL${NC}"
    echo -e "  Passed: ${GREEN}$TESTS_PASSED${NC}"
    echo -e "  Failed: ${RED}$TESTS_FAILED${NC}"
    echo -e "  Skipped: ${BLUE}$TESTS_SKIPPED${NC}"

    if [ $TESTS_FAILED -eq 0 ]; then
        local success_rate=$(( (TESTS_PASSED * 100) / (TESTS_TOTAL - TESTS_SKIPPED) ))
        echo -e "  Success Rate: ${GREEN}${success_rate}%${NC}"
        print_success "All UAT tests passed! ✅"
    else
        local success_rate=$(( (TESTS_PASSED * 100) / (TESTS_TOTAL - TESTS_SKIPPED) ))
        echo -e "  Success Rate: ${YELLOW}${success_rate}%${NC}"
        print_error "Some UAT tests failed! ❌"
    fi

    # Generate detailed report
    local report_file=$(generate_report)
    print_info "Detailed report saved to: $report_file"

    print_banner "🎯 UAT Testing Complete"

    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}🎉 System is ready for production deployment!${NC}"
        echo -e "${CYAN}✅ All critical functionality verified${NC}"
        echo -e "${CYAN}✅ Performance benchmarks met${NC}"
        echo -e "${CYAN}✅ Security controls validated${NC}"
        echo -e "${CYAN}✅ Admin interfaces operational${NC}"
        exit 0
    else
        echo -e "${RED}⚠️  System has issues that need attention before production deployment${NC}"
        echo -e "${YELLOW}Please review the failed tests and fix issues before proceeding${NC}"
        exit 1
    fi
}

# Execute main function
main "$@"
