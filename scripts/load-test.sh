#!/bin/bash

# Lair Chat - Load Testing Script
# Comprehensive load testing for REST API and TCP server performance

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Default configuration
DEFAULT_CLIENTS=50
DEFAULT_DURATION=60
DEFAULT_API_HOST="127.0.0.1"
DEFAULT_API_PORT="8082"
DEFAULT_TCP_PORT="8080"
DEFAULT_RAMP_TIME=30
DEFAULT_TEST_TYPE="api"

# Test configuration
CLIENTS=$DEFAULT_CLIENTS
DURATION=$DEFAULT_DURATION
API_HOST=$DEFAULT_API_HOST
API_PORT=$DEFAULT_API_PORT
TCP_PORT=$DEFAULT_TCP_PORT
RAMP_TIME=$DEFAULT_RAMP_TIME
TEST_TYPE=$DEFAULT_TEST_TYPE
VERBOSE=false
REPORT_DIR="test_results/load_test_$(date +%Y%m%d_%H%M%S)"

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
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Usage information
show_usage() {
    echo "Lair Chat Load Testing Script"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -c, --clients NUM      Number of concurrent clients (default: $DEFAULT_CLIENTS)"
    echo "  -d, --duration SEC     Test duration in seconds (default: $DEFAULT_DURATION)"
    echo "  -r, --ramp-time SEC    Ramp-up time in seconds (default: $DEFAULT_RAMP_TIME)"
    echo "  -t, --type TYPE        Test type: api, tcp, or both (default: $DEFAULT_TEST_TYPE)"
    echo "  -h, --host HOST        API host (default: $DEFAULT_API_HOST)"
    echo "  -p, --port PORT        API port (default: $DEFAULT_API_PORT)"
    echo "  --tcp-port PORT        TCP port (default: $DEFAULT_TCP_PORT)"
    echo "  -v, --verbose          Verbose output"
    echo "  --help                 Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --clients 100 --duration 300"
    echo "  $0 --type both --clients 50 --duration 120"
    echo "  $0 --type tcp --clients 200 --duration 60"
    echo ""
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -c|--clients)
                CLIENTS="$2"
                shift 2
                ;;
            -d|--duration)
                DURATION="$2"
                shift 2
                ;;
            -r|--ramp-time)
                RAMP_TIME="$2"
                shift 2
                ;;
            -t|--type)
                TEST_TYPE="$2"
                shift 2
                ;;
            -h|--host)
                API_HOST="$2"
                shift 2
                ;;
            -p|--port)
                API_PORT="$2"
                shift 2
                ;;
            --tcp-port)
                TCP_PORT="$2"
                shift 2
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            --help)
                show_usage
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
}

# Validate configuration
validate_config() {
    # Validate numeric values
    if ! [[ "$CLIENTS" =~ ^[0-9]+$ ]] || [ "$CLIENTS" -lt 1 ]; then
        print_error "Invalid number of clients: $CLIENTS"
        exit 1
    fi

    if ! [[ "$DURATION" =~ ^[0-9]+$ ]] || [ "$DURATION" -lt 1 ]; then
        print_error "Invalid duration: $DURATION"
        exit 1
    fi

    if ! [[ "$RAMP_TIME" =~ ^[0-9]+$ ]] || [ "$RAMP_TIME" -lt 0 ]; then
        print_error "Invalid ramp time: $RAMP_TIME"
        exit 1
    fi

    # Validate test type
    if [[ ! "$TEST_TYPE" =~ ^(api|tcp|both)$ ]]; then
        print_error "Invalid test type: $TEST_TYPE (must be: api, tcp, or both)"
        exit 1
    fi

    # Check required tools
    if ! command -v curl &> /dev/null; then
        print_error "curl is required for API testing"
        exit 1
    fi

    if [[ "$TEST_TYPE" == "tcp" || "$TEST_TYPE" == "both" ]] && ! command -v nc &> /dev/null; then
        print_error "netcat (nc) is required for TCP testing"
        exit 1
    fi
}

# Test server connectivity
test_connectivity() {
    print_info "Testing server connectivity..."

    # Test API server
    if [[ "$TEST_TYPE" == "api" || "$TEST_TYPE" == "both" ]]; then
        local api_url="http://${API_HOST}:${API_PORT}/api/v1/health"
        if curl -s -f "$api_url" > /dev/null; then
            print_success "API server is reachable at $api_url"
        else
            print_error "Cannot reach API server at $api_url"
            exit 1
        fi
    fi

    # Test TCP server
    if [[ "$TEST_TYPE" == "tcp" || "$TEST_TYPE" == "both" ]]; then
        if nc -z "$API_HOST" "$TCP_PORT" 2>/dev/null; then
            print_success "TCP server is reachable at ${API_HOST}:${TCP_PORT}"
        else
            print_error "Cannot reach TCP server at ${API_HOST}:${TCP_PORT}"
            exit 1
        fi
    fi
}

# Create test user for load testing
create_test_user() {
    local username="loadtest_user_$$"
    local email="loadtest_$$@example.com"
    local password="LoadTest123!"

    local response=$(curl -s -X POST "http://${API_HOST}:${API_PORT}/api/v1/auth/register" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$username\",\"email\":\"$email\",\"password\":\"$password\"}")

    if echo "$response" | grep -q "success"; then
        # Login to get token
        local login_response=$(curl -s -X POST "http://${API_HOST}:${API_PORT}/api/v1/auth/login" \
            -H "Content-Type: application/json" \
            -d "{\"identifier\":\"$username\",\"password\":\"$password\"}")

        local token=$(echo "$login_response" | grep -o '"access_token":"[^"]*"' | cut -d'"' -f4)

        if [ ! -z "$token" ]; then
            echo "$token"
        else
            print_error "Failed to get authentication token"
            return 1
        fi
    else
        print_error "Failed to create test user"
        return 1
    fi
}

# API Load Test Worker
api_worker() {
    local worker_id=$1
    local token=$2
    local worker_results_file="$REPORT_DIR/worker_${worker_id}_api.results"
    local start_time=$(date +%s)
    local end_time=$((start_time + DURATION))
    local requests=0
    local errors=0
    local total_response_time=0

    while [ $(date +%s) -lt $end_time ]; do
        local request_start=$(date +%s%3N)

        # Make API request
        local response_code=$(curl -s -w "%{http_code}" -o /dev/null \
            -H "Authorization: Bearer $token" \
            "http://${API_HOST}:${API_PORT}/api/v1/users/profile")

        local request_end=$(date +%s%3N)
        local response_time=$((request_end - request_start))

        requests=$((requests + 1))
        total_response_time=$((total_response_time + response_time))

        if [ "$response_code" != "200" ]; then
            errors=$((errors + 1))
            if [ "$VERBOSE" = true ]; then
                echo "Worker $worker_id: HTTP $response_code" >&2
            fi
        fi

        # Brief pause to avoid overwhelming
        sleep 0.01
    done

    # Write results
    echo "$requests $errors $total_response_time" > "$worker_results_file"
}

# TCP Load Test Worker
tcp_worker() {
    local worker_id=$1
    local worker_results_file="$REPORT_DIR/worker_${worker_id}_tcp.results"
    local start_time=$(date +%s)
    local end_time=$((start_time + DURATION))
    local connections=0
    local errors=0
    local total_response_time=0

    while [ $(date +%s) -lt $end_time ]; do
        local request_start=$(date +%s%3N)

        # Test TCP connection
        if echo "PING" | nc -w 1 "$API_HOST" "$TCP_PORT" > /dev/null 2>&1; then
            connections=$((connections + 1))
        else
            errors=$((errors + 1))
            if [ "$VERBOSE" = true ]; then
                echo "Worker $worker_id: TCP connection failed" >&2
            fi
        fi

        local request_end=$(date +%s%3N)
        local response_time=$((request_end - request_start))
        total_response_time=$((total_response_time + response_time))

        # Brief pause
        sleep 0.1
    done

    # Write results
    echo "$connections $errors $total_response_time" > "$worker_results_file"
}

# Run API load test
run_api_test() {
    print_info "Creating test user for API load test..."
    local token=$(create_test_user)

    if [ -z "$token" ]; then
        print_error "Failed to create test user"
        return 1
    fi

    print_info "Starting API load test with $CLIENTS clients for ${DURATION}s..."

    local pids=()
    local ramp_delay=$(echo "scale=2; $RAMP_TIME / $CLIENTS" | bc -l 2>/dev/null || echo "0.1")

    # Start workers with ramp-up
    for ((i=1; i<=CLIENTS; i++)); do
        api_worker $i "$token" &
        pids+=($!)

        if [ "$VERBOSE" = true ]; then
            print_info "Started API worker $i (PID: $!)"
        fi

        # Ramp-up delay
        if [ $(echo "$ramp_delay > 0" | bc -l 2>/dev/null || echo "0") -eq 1 ]; then
            sleep "$ramp_delay"
        fi
    done

    print_info "All API workers started, waiting for completion..."

    # Wait for all workers to complete
    for pid in "${pids[@]}"; do
        wait $pid
    done

    print_success "API load test completed"
}

# Run TCP load test
run_tcp_test() {
    print_info "Starting TCP load test with $CLIENTS clients for ${DURATION}s..."

    local pids=()
    local ramp_delay=$(echo "scale=2; $RAMP_TIME / $CLIENTS" | bc -l 2>/dev/null || echo "0.1")

    # Start workers with ramp-up
    for ((i=1; i<=CLIENTS; i++)); do
        tcp_worker $i &
        pids+=($!)

        if [ "$VERBOSE" = true ]; then
            print_info "Started TCP worker $i (PID: $!)"
        fi

        # Ramp-up delay
        if [ $(echo "$ramp_delay > 0" | bc -l 2>/dev/null || echo "0") -eq 1 ]; then
            sleep "$ramp_delay"
        fi
    done

    print_info "All TCP workers started, waiting for completion..."

    # Wait for all workers to complete
    for pid in "${pids[@]}"; do
        wait $pid
    done

    print_success "TCP load test completed"
}

# Calculate and display results
calculate_results() {
    local test_type=$1
    local total_requests=0
    local total_errors=0
    local total_response_time=0
    local worker_count=0

    # Aggregate results from all workers
    for result_file in "$REPORT_DIR"/worker_*_${test_type}.results; do
        if [ -f "$result_file" ]; then
            local requests errors response_time
            read requests errors response_time < "$result_file"

            total_requests=$((total_requests + requests))
            total_errors=$((total_errors + errors))
            total_response_time=$((total_response_time + response_time))
            worker_count=$((worker_count + 1))
        fi
    done

    if [ $total_requests -eq 0 ]; then
        print_warning "No ${test_type} requests completed"
        return
    fi

    # Calculate metrics
    local success_rate=$(echo "scale=2; ($total_requests - $total_errors) * 100 / $total_requests" | bc -l)
    local rps=$(echo "scale=2; $total_requests / $DURATION" | bc -l)
    local avg_response_time=$(echo "scale=2; $total_response_time / $total_requests" | bc -l)

    # Display results
    echo -e "\n${CYAN}${test_type^^} Load Test Results:${NC}"
    echo "=================================="
    echo "Duration:              ${DURATION}s"
    echo "Concurrent clients:    $worker_count"
    echo "Total requests:        $total_requests"
    echo "Successful requests:   $((total_requests - total_errors))"
    echo "Failed requests:       $total_errors"
    echo "Success rate:          ${success_rate}%"
    echo "Requests per second:   $rps"
    echo "Avg response time:     ${avg_response_time}ms"

    # Write summary to file
    cat > "$REPORT_DIR/${test_type}_summary.txt" << EOF
Lair Chat ${test_type^^} Load Test Results
Generated: $(date)

Test Configuration:
- Duration: ${DURATION}s
- Concurrent clients: $worker_count
- Ramp-up time: ${RAMP_TIME}s
- Target: ${API_HOST}:${API_PORT}

Results:
- Total requests: $total_requests
- Successful requests: $((total_requests - total_errors))
- Failed requests: $total_errors
- Success rate: ${success_rate}%
- Requests per second: $rps
- Average response time: ${avg_response_time}ms

Performance Rating:
EOF

    # Add performance rating
    if (( $(echo "$rps >= 100" | bc -l) )); then
        echo "- Excellent performance (${rps} RPS)" >> "$REPORT_DIR/${test_type}_summary.txt"
    elif (( $(echo "$rps >= 50" | bc -l) )); then
        echo "- Good performance (${rps} RPS)" >> "$REPORT_DIR/${test_type}_summary.txt"
    elif (( $(echo "$rps >= 20" | bc -l) )); then
        echo "- Moderate performance (${rps} RPS)" >> "$REPORT_DIR/${test_type}_summary.txt"
    else
        echo "- Poor performance (${rps} RPS)" >> "$REPORT_DIR/${test_type}_summary.txt"
    fi

    if (( $(echo "$success_rate >= 99" | bc -l) )); then
        echo "- Excellent reliability (${success_rate}% success)" >> "$REPORT_DIR/${test_type}_summary.txt"
    elif (( $(echo "$success_rate >= 95" | bc -l) )); then
        echo "- Good reliability (${success_rate}% success)" >> "$REPORT_DIR/${test_type}_summary.txt"
    else
        echo "- Poor reliability (${success_rate}% success)" >> "$REPORT_DIR/${test_type}_summary.txt"
    fi
}

# Generate comprehensive report
generate_report() {
    local report_file="$REPORT_DIR/load_test_report.html"

    cat > "$report_file" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Lair Chat Load Test Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .header { background: #f4f4f4; padding: 20px; border-radius: 5px; }
        .results { margin: 20px 0; }
        .metric { display: inline-block; margin: 10px 20px 10px 0; }
        .metric-value { font-size: 24px; font-weight: bold; color: #2196F3; }
        .metric-label { font-size: 14px; color: #666; }
        .good { color: #4CAF50; }
        .warning { color: #FF9800; }
        .error { color: #F44336; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🚀 Lair Chat Load Test Report</h1>
        <p>Generated: $(date)</p>
    </div>

    <div class="results">
        <h2>Test Configuration</h2>
        <ul>
            <li>Test Type: $TEST_TYPE</li>
            <li>Duration: ${DURATION}s</li>
            <li>Concurrent Clients: $CLIENTS</li>
            <li>Ramp-up Time: ${RAMP_TIME}s</li>
            <li>Target Server: ${API_HOST}:${API_PORT}</li>
        </ul>
    </div>
EOF

    # Add API results if available
    if [[ "$TEST_TYPE" == "api" || "$TEST_TYPE" == "both" ]] && [ -f "$REPORT_DIR/api_summary.txt" ]; then
        echo "<h2>API Test Results</h2>" >> "$report_file"
        echo "<pre>" >> "$report_file"
        cat "$REPORT_DIR/api_summary.txt" >> "$report_file"
        echo "</pre>" >> "$report_file"
    fi

    # Add TCP results if available
    if [[ "$TEST_TYPE" == "tcp" || "$TEST_TYPE" == "both" ]] && [ -f "$REPORT_DIR/tcp_summary.txt" ]; then
        echo "<h2>TCP Test Results</h2>" >> "$report_file"
        echo "<pre>" >> "$report_file"
        cat "$REPORT_DIR/tcp_summary.txt" >> "$report_file"
        echo "</pre>" >> "$report_file"
    fi

    echo "</body></html>" >> "$report_file"

    print_success "HTML report generated: $report_file"
}

# Cleanup function
cleanup() {
    print_info "Cleaning up test processes..."
    jobs -p | xargs -r kill 2>/dev/null || true
    exit 0
}

# Main execution
main() {
    print_banner "🚀 Lair Chat Load Testing"

    # Parse arguments
    parse_args "$@"

    # Validate configuration
    validate_config

    # Display test configuration
    print_info "Load Test Configuration:"
    print_info "  Test Type: $TEST_TYPE"
    print_info "  Clients: $CLIENTS"
    print_info "  Duration: ${DURATION}s"
    print_info "  Ramp-up: ${RAMP_TIME}s"
    print_info "  Target: ${API_HOST}:${API_PORT}"

    # Setup signal handlers
    trap cleanup SIGINT SIGTERM

    # Create results directory
    mkdir -p "$REPORT_DIR"

    # Test connectivity
    test_connectivity

    # Record test start
    local test_start=$(date +%s)
    print_info "Starting load test at $(date)"

    # Run tests based on type
    case $TEST_TYPE in
        "api")
            run_api_test
            calculate_results "api"
            ;;
        "tcp")
            run_tcp_test
            calculate_results "tcp"
            ;;
        "both")
            print_info "Running combined API and TCP load test..."
            run_api_test &
            API_PID=$!
            run_tcp_test &
            TCP_PID=$!

            wait $API_PID
            wait $TCP_PID

            calculate_results "api"
            calculate_results "tcp"
            ;;
    esac

    # Calculate total test time
    local test_end=$(date +%s)
    local test_duration=$((test_end - test_start))

    print_banner "📊 Load Test Complete"
    print_success "Test completed in ${test_duration}s"
    print_info "Results saved to: $REPORT_DIR"

    # Generate reports
    generate_report

    # Show final summary
    echo -e "\n${CYAN}Summary:${NC}"
    echo "- Test duration: ${test_duration}s"
    echo "- Results directory: $REPORT_DIR"
    echo "- HTML report: $REPORT_DIR/load_test_report.html"

    if [ "$VERBOSE" = true ]; then
        echo "- Individual worker results: $REPORT_DIR/worker_*.results"
    fi

    print_success "Load testing completed successfully! 🎉"
}

# Execute main function with all arguments
main "$@"
