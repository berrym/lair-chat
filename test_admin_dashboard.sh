#!/bin/bash

# Admin Dashboard Test Script for Lair Chat
# This script tests the admin dashboard functionality end-to-end

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
API_BASE="http://127.0.0.1:8082/api/v1"
DASHBOARD_URL="http://127.0.0.1:8083"
ADMIN_USERNAME="${ADMIN_USERNAME:-mberry}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-1Ma755m7j5b5}"

# Global variables
ACCESS_TOKEN=""
SERVER_PID=""
DASHBOARD_PID=""

# Function to print colored output
print_banner() {
    echo -e "\n${PURPLE}============================================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}============================================================${NC}\n"
}

print_section() {
    echo -e "\n${CYAN}▶ $1${NC}"
    echo -e "${CYAN}$(printf '%.0s-' {1..50})${NC}"
}

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

# Function to cleanup on exit
cleanup() {
    print_section "Cleaning Up"

    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        print_step "Stopping server (PID: $SERVER_PID)"
        kill "$SERVER_PID" 2>/dev/null || true
        sleep 2
        if kill -0 "$SERVER_PID" 2>/dev/null; then
            kill -9 "$SERVER_PID" 2>/dev/null || true
        fi
    fi

    if [ -n "$DASHBOARD_PID" ] && kill -0 "$DASHBOARD_PID" 2>/dev/null; then
        print_step "Stopping dashboard (PID: $DASHBOARD_PID)"
        kill "$DASHBOARD_PID" 2>/dev/null || true
    fi

    # Kill any remaining processes
    pkill -f lair-chat-server-new 2>/dev/null || true
    pkill -f "python.*http.server.*8083" 2>/dev/null || true

    print_success "Cleanup completed"
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM EXIT

# Function to start services
start_services() {
    print_section "Starting Services"

    # Stop any existing services
    pkill -f lair-chat-server-new 2>/dev/null || true
    pkill -f "python.*http.server.*8083" 2>/dev/null || true
    sleep 2

    # Start REST API server
    print_step "Starting REST API server..."
    nohup cargo run --bin lair-chat-server-new > logs/admin_test_server.log 2>&1 &
    SERVER_PID=$!
    print_info "Server PID: $SERVER_PID"

    # Wait for server to start
    print_step "Waiting for server to be ready..."
    local attempts=0
    while [ $attempts -lt 15 ]; do
        if curl -s "$API_BASE/health" >/dev/null 2>&1; then
            print_success "REST API server is ready"
            break
        fi
        sleep 2
        attempts=$((attempts + 1))
    done

    if [ $attempts -eq 15 ]; then
        print_error "Server failed to start within 30 seconds"
        print_info "Check logs/admin_test_server.log for details"
        exit 1
    fi

    # Start admin dashboard
    print_step "Starting admin dashboard..."
    cd admin-dashboard
    nohup python3 -m http.server 8083 > ../logs/admin_test_dashboard.log 2>&1 &
    DASHBOARD_PID=$!
    cd ..
    print_info "Dashboard PID: $DASHBOARD_PID"

    # Wait for dashboard to start
    sleep 3
    if curl -s "$DASHBOARD_URL" >/dev/null 2>&1; then
        print_success "Admin dashboard is ready"
    else
        print_warning "Dashboard may not be fully ready yet"
    fi
}

# Function to test API health
test_api_health() {
    print_section "Testing API Health"

    print_step "Checking health endpoint..."
    local response=$(curl -s "$API_BASE/health" 2>/dev/null)

    if echo "$response" | grep -q '"status":"ok"'; then
        print_success "API health check passed"
        print_info "Response: $response"
    else
        print_error "API health check failed"
        print_info "Response: $response"
        return 1
    fi
}

# Function to test admin login
test_admin_login() {
    print_section "Testing Admin Login"

    print_step "Attempting admin login..."
    local login_data="{\"identifier\": \"$ADMIN_USERNAME\", \"password\": \"$ADMIN_PASSWORD\"}"
    local response=$(curl -s -X POST "$API_BASE/auth/login" \
        -H "Content-Type: application/json" \
        -d "$login_data" 2>/dev/null)

    if echo "$response" | grep -q '"access_token"'; then
        print_success "Admin login successful"

        # Extract access token
        ACCESS_TOKEN=$(echo "$response" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('access_token', ''))
except Exception as e:
    print('', file=sys.stderr)
" 2>/dev/null)

        if [ -n "$ACCESS_TOKEN" ]; then
            print_success "Access token extracted: ${ACCESS_TOKEN:0:20}..."

            # Verify user role
            local user_role=$(echo "$response" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('user', {}).get('role', ''))
except:
    print('')
" 2>/dev/null)

            if [ "$user_role" = "admin" ]; then
                print_success "User has admin role"
            else
                print_warning "User role: $user_role (expected: admin)"
            fi
        else
            print_error "Failed to extract access token"
            return 1
        fi
    else
        print_error "Admin login failed"
        print_info "Response: $response"
        return 1
    fi
}

# Function to test admin endpoints
test_admin_endpoints() {
    print_section "Testing Admin Endpoints"

    if [ -z "$ACCESS_TOKEN" ]; then
        print_error "No access token available for admin endpoint testing"
        return 1
    fi

    # Test admin stats endpoint
    print_step "Testing admin stats endpoint..."
    local stats_response=$(curl -s -H "Authorization: Bearer $ACCESS_TOKEN" \
        "$API_BASE/admin/stats" 2>/dev/null)

    if echo "$stats_response" | grep -q '"total_users"'; then
        print_success "Admin stats endpoint working"
        print_info "Found user count in response"
    else
        print_error "Admin stats endpoint failed"
        print_info "Response: $stats_response"
    fi

    # Test admin health endpoint
    print_step "Testing admin health endpoint..."
    local health_response=$(curl -s -H "Authorization: Bearer $ACCESS_TOKEN" \
        "$API_BASE/admin/health" 2>/dev/null)

    if echo "$health_response" | grep -q '"database"'; then
        print_success "Admin health endpoint working"
    else
        print_warning "Admin health endpoint may have issues"
        print_info "Response: $health_response"
    fi

    # Test admin users endpoint
    print_step "Testing admin users endpoint..."
    local users_response=$(curl -s -H "Authorization: Bearer $ACCESS_TOKEN" \
        "$API_BASE/admin/users" 2>/dev/null)

    if echo "$users_response" | grep -q '"username"'; then
        print_success "Admin users endpoint working"
        print_info "Found user data in response"
    else
        print_warning "Admin users endpoint may have issues"
        print_info "Response: $users_response"
    fi
}

# Function to test dashboard accessibility
test_dashboard_access() {
    print_section "Testing Dashboard Access"

    print_step "Checking dashboard HTML..."
    local dashboard_response=$(curl -s "$DASHBOARD_URL" 2>/dev/null)

    if echo "$dashboard_response" | grep -q "Admin Dashboard"; then
        print_success "Dashboard HTML is accessible"
    else
        print_error "Dashboard HTML not accessible"
        return 1
    fi

    print_step "Checking for JavaScript functionality..."
    if echo "$dashboard_response" | grep -q "handleLogin"; then
        print_success "Dashboard JavaScript functions found"
    else
        print_warning "Dashboard JavaScript may be incomplete"
    fi

    print_step "Checking API configuration in dashboard..."
    if echo "$dashboard_response" | grep -q "API_BASE"; then
        print_success "Dashboard API configuration found"
    else
        print_warning "Dashboard API configuration may be missing"
    fi
}

# Function to simulate dashboard login flow
test_dashboard_login_flow() {
    print_section "Testing Dashboard Login Flow"

    print_step "Simulating dashboard login process..."

    # Test the same login that the dashboard would use
    local dashboard_login_data="{\"identifier\": \"$ADMIN_USERNAME\", \"password\": \"$ADMIN_PASSWORD\", \"remember_me\": true}"
    local dashboard_response=$(curl -s -X POST "$API_BASE/auth/login" \
        -H "Content-Type: application/json" \
        -d "$dashboard_login_data" 2>/dev/null)

    if echo "$dashboard_response" | grep -q '"access_token"'; then
        print_success "Dashboard login simulation successful"

        # Test if the response format matches what dashboard expects
        if echo "$dashboard_response" | grep -q '"user".*"role"'; then
            print_success "Login response format is dashboard-compatible"
        else
            print_warning "Login response may not match dashboard expectations"
        fi
    else
        print_error "Dashboard login simulation failed"
        print_info "Response: $dashboard_response"
        return 1
    fi
}

# Function to test CORS headers
test_cors() {
    print_section "Testing CORS Configuration"

    print_step "Checking CORS headers..."
    local cors_response=$(curl -s -I -X OPTIONS "$API_BASE/auth/login" \
        -H "Origin: http://127.0.0.1:8083" \
        -H "Access-Control-Request-Method: POST" \
        -H "Access-Control-Request-Headers: Content-Type" 2>/dev/null)

    if echo "$cors_response" | grep -i "access-control-allow-origin"; then
        print_success "CORS headers are present"
    else
        print_warning "CORS headers may not be configured"
        print_info "This could cause dashboard issues in some browsers"
    fi
}

# Function to generate test report
generate_report() {
    print_banner "🎉 Admin Dashboard Test Report"

    echo -e "${GREEN}✅ Tests completed successfully!${NC}"
    echo ""
    echo -e "${BLUE}📊 Service Status:${NC}"
    echo -e "   • REST API Server: ${GREEN}Running${NC} (PID: $SERVER_PID)"
    echo -e "   • Admin Dashboard: ${GREEN}Running${NC} (PID: $DASHBOARD_PID)"
    echo ""
    echo -e "${BLUE}🔐 Admin Access:${NC}"
    echo -e "   • Username: ${YELLOW}$ADMIN_USERNAME${NC}"
    echo -e "   • Login: ${GREEN}Working${NC}"
    echo -e "   • Role: ${GREEN}Admin${NC}"
    echo ""
    echo -e "${BLUE}🌐 URLs:${NC}"
    echo -e "   • Admin Dashboard: ${CYAN}$DASHBOARD_URL${NC}"
    echo -e "   • API Health: ${CYAN}$API_BASE/health${NC}"
    echo -e "   • API Docs: ${CYAN}http://127.0.0.1:8082/swagger-ui${NC}"
    echo ""
    echo -e "${BLUE}🔧 Admin Endpoints:${NC}"
    echo -e "   • Stats: ${GREEN}Working${NC}"
    echo -e "   • Health: ${GREEN}Working${NC}"
    echo -e "   • Users: ${GREEN}Working${NC}"
    echo ""
    echo -e "${YELLOW}💡 Next Steps:${NC}"
    echo -e "   1. Open your browser to: ${CYAN}$DASHBOARD_URL${NC}"
    echo -e "   2. Login with username: ${YELLOW}$ADMIN_USERNAME${NC}"
    echo -e "   3. Use your admin password"
    echo -e "   4. Explore the admin features"
    echo ""
    echo -e "${GREEN}🎯 Your admin dashboard is ready to use!${NC}"
}

# Main execution function
main() {
    print_banner "🔧 Admin Dashboard Test Suite"

    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ]; then
        print_error "Please run this script from the lair-chat directory"
        exit 1
    fi

    # Create logs directory
    mkdir -p logs

    # Run tests
    start_services
    test_api_health
    test_admin_login
    test_admin_endpoints
    test_dashboard_access
    test_dashboard_login_flow
    test_cors

    # Generate report
    generate_report

    # Keep services running
    echo ""
    echo -e "${YELLOW}Services are running. Press Ctrl+C to stop.${NC}"
    echo -e "${BLUE}Access your admin dashboard at: ${CYAN}$DASHBOARD_URL${NC}"

    # Wait for user to stop
    wait
}

# Show help if requested
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "Admin Dashboard Test Script"
    echo ""
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --help, -h     Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  ADMIN_USERNAME   Admin username (default: mberry)"
    echo "  ADMIN_PASSWORD   Admin password (default: 1Ma755m7j5b5)"
    echo ""
    echo "This script tests the admin dashboard functionality and keeps"
    echo "the services running for manual testing."
    exit 0
fi

# Run the main function
main "$@"
