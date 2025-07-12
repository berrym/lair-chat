#!/bin/bash

# PHASE 8 TESTING: Comprehensive Test Runner Script
# This script orchestrates all testing phases for the lair-chat application

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
TEST_RESULTS_DIR="${PROJECT_ROOT}/test_results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
TEST_LOG="${TEST_RESULTS_DIR}/test_run_${TIMESTAMP}.log"

# Test configuration
MIN_COVERAGE=95
CONCURRENT_USERS=100
STRESS_DURATION=300  # 5 minutes
LOAD_DURATION=180    # 3 minutes

# Ensure test results directory exists
mkdir -p "${TEST_RESULTS_DIR}"

# Logging function
log() {
    local level=$1
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    case $level in
        INFO)  echo -e "${BLUE}[INFO]${NC} ${message}" | tee -a "${TEST_LOG}" ;;
        WARN)  echo -e "${YELLOW}[WARN]${NC} ${message}" | tee -a "${TEST_LOG}" ;;
        ERROR) echo -e "${RED}[ERROR]${NC} ${message}" | tee -a "${TEST_LOG}" ;;
        SUCCESS) echo -e "${GREEN}[SUCCESS]${NC} ${message}" | tee -a "${TEST_LOG}" ;;
    esac
}

# Function to run unit tests
run_unit_tests() {
    log INFO "Starting unit tests..."

    cd "${PROJECT_ROOT}"

    # Run unit tests with coverage
    log INFO "Running unit tests with coverage analysis..."

    if command -v cargo-tarpaulin >/dev/null 2>&1; then
        log INFO "Using tarpaulin for coverage analysis..."
        cargo tarpaulin --out Html --output-dir "${TEST_RESULTS_DIR}/coverage" \
            --exclude-files 'src/bin/*' 'tests/*' 'scripts/*' 'benches/*' \
            --timeout 300 \
            --verbose 2>&1 | tee -a "${TEST_LOG}"

        # Extract coverage percentage
        local coverage=$(grep -o 'Coverage: [0-9.]*%' "${TEST_LOG}" | tail -1 | grep -o '[0-9.]*')

        if (( $(echo "$coverage >= $MIN_COVERAGE" | bc -l) )); then
            log SUCCESS "Unit test coverage: ${coverage}% (meets minimum ${MIN_COVERAGE}%)"
        else
            log ERROR "Unit test coverage: ${coverage}% (below minimum ${MIN_COVERAGE}%)"
            return 1
        fi
    else
        log WARN "cargo-tarpaulin not found, running tests without coverage..."
        cargo test --lib --bins --tests 2>&1 | tee -a "${TEST_LOG}"
    fi

    log SUCCESS "Unit tests completed"
}

# Function to run integration tests
run_integration_tests() {
    log INFO "Starting integration tests..."

    cd "${PROJECT_ROOT}"

    # Run integration tests
    log INFO "Running integration test suite..."
    cargo test --test '*' --release 2>&1 | tee -a "${TEST_LOG}"

    # Run specific integration scenarios
    log INFO "Running cross-framework integration tests..."
    cargo test integration:: --release 2>&1 | tee -a "${TEST_LOG}"

    log SUCCESS "Integration tests completed"
}

# Function to run performance tests
run_performance_tests() {
    log INFO "Starting performance tests..."

    cd "${PROJECT_ROOT}"

    # Run benchmark tests
    log INFO "Running benchmark suite..."
    cargo bench 2>&1 | tee -a "${TEST_LOG}"

    # Performance regression tests
    log INFO "Running performance regression tests..."
    if [ -f "${TEST_RESULTS_DIR}/baseline_performance.json" ]; then
        log INFO "Comparing against baseline performance..."
        cargo test --test performance_regression --release 2>&1 | tee -a "${TEST_LOG}"
    else
        log WARN "No baseline performance data found, creating baseline..."
        cargo bench --output-format json > "${TEST_RESULTS_DIR}/baseline_performance.json"
    fi

    log SUCCESS "Performance tests completed"
}

# Function to run load tests
run_load_tests() {
    log INFO "Starting load tests..."

    cd "${PROJECT_ROOT}"

    # Build server if needed
    log INFO "Building lair-chat server..."
    cargo build --release --bin lair-chat-server

    # Start server in background for load testing
    log INFO "Starting test server..."
    LAIR_CHAT_CONFIG="${PROJECT_ROOT}/config/test.toml" \
    ./target/release/lair-chat-server > "${TEST_RESULTS_DIR}/server_${TIMESTAMP}.log" 2>&1 &
    local server_pid=$!

    # Wait for server to start and verify
    local server_ready=false
    for i in {1..30}; do
        if timeout 2 bash -c "</dev/tcp/127.0.0.1/3335" 2>/dev/null; then
            server_ready=true
            break
        fi
        sleep 1
    done

    if [ "$server_ready" = false ]; then
        log ERROR "Test server failed to start or become ready"
        kill $server_pid 2>/dev/null || true
        return 1
    fi

    log SUCCESS "Test server is ready"

    # Start performance monitoring
    log INFO "Starting performance monitoring..."
    "${SCRIPT_DIR}/monitor_performance.sh" --start --duration "${LOAD_DURATION}" \
        --alert-cpu 80 --alert-memory 85 || log WARN "Could not start monitoring"

    # Run comprehensive load tests
    log INFO "Running load tests with ${CONCURRENT_USERS} concurrent users for ${LOAD_DURATION}s..."

    if [ -f "${SCRIPT_DIR}/load_test.sh" ]; then
        "${SCRIPT_DIR}/load_test.sh" \
            --users "${CONCURRENT_USERS}" \
            --duration "${LOAD_DURATION}" \
            --messages-per-user 10 \
            --message-rate 10 \
            --connection-rate 50 \
            --output "${TEST_RESULTS_DIR}/load_test_${TIMESTAMP}.json" \
            2>&1 | tee -a "${TEST_LOG}"
        local load_test_result=$?
    else
        log ERROR "Load test script not found at ${SCRIPT_DIR}/load_test.sh"
        load_test_result=1
    fi

    # Run Rust load tests
    log INFO "Running Rust load test suite..."
    cargo test --test '*' --release -- load 2>&1 | tee -a "${TEST_LOG}"

    # Stop performance monitoring
    "${SCRIPT_DIR}/monitor_performance.sh" --stop || log WARN "Could not stop monitoring"

    # Stop test server
    log INFO "Stopping test server..."
    kill $server_pid 2>/dev/null || true
    wait $server_pid 2>/dev/null || true

    if [ $load_test_result -eq 0 ]; then
        log SUCCESS "Load tests completed successfully"
        return 0
    else
        log ERROR "Load tests failed"
        return 1
    fi
}

# Function to run stress tests
run_stress_tests() {
    log INFO "Starting stress tests..."

    cd "${PROJECT_ROOT}"

    # Build server if needed
    log INFO "Building lair-chat server..."
    cargo build --release --bin lair-chat-server

    # Start server in background for stress testing
    log INFO "Starting test server for stress testing..."
    LAIR_CHAT_CONFIG="${PROJECT_ROOT}/config/test.toml" \
    ./target/release/lair-chat-server > "${TEST_RESULTS_DIR}/stress_server_${TIMESTAMP}.log" 2>&1 &
    local server_pid=$!

    # Wait for server to start and verify
    local server_ready=false
    for i in {1..30}; do
        if timeout 2 bash -c "</dev/tcp/127.0.0.1/3335" 2>/dev/null; then
            server_ready=true
            break
        fi
        sleep 1
    done

    if [ "$server_ready" = false ]; then
        log ERROR "Test server failed to start for stress testing"
        kill $server_pid 2>/dev/null || true
        return 1
    fi

    log SUCCESS "Test server ready for stress testing"

    # Start intensive performance monitoring for stress tests
    log INFO "Starting intensive performance monitoring..."
    "${SCRIPT_DIR}/monitor_performance.sh" --start --duration "${STRESS_DURATION}" \
        --interval 1 --alert-cpu 90 --alert-memory 95 --alert-connections 500 \
        || log WARN "Could not start stress monitoring"

    # Run comprehensive stress tests
    log INFO "Running stress tests with up to 500 users for ${STRESS_DURATION}s..."

    if [ -f "${SCRIPT_DIR}/stress_test.sh" ]; then
        "${SCRIPT_DIR}/stress_test.sh" \
            --max-users 500 \
            --duration "${STRESS_DURATION}" \
            --memory-stress 1024 \
            --connection-burst 100 \
            --increment 50 \
            --output "${TEST_RESULTS_DIR}/stress_test_${TIMESTAMP}.json" \
            2>&1 | tee -a "${TEST_LOG}"
        local stress_test_result=$?
    else
        log ERROR "Stress test script not found at ${SCRIPT_DIR}/stress_test.sh"
        stress_test_result=1
    fi

    # Run Rust stress tests
    log INFO "Running Rust stress test suite..."
    cargo test --test '*' --release -- stress 2>&1 | tee -a "${TEST_LOG}"

    # Memory leak detection with valgrind if available
    log INFO "Running memory leak detection..."
    if command -v valgrind >/dev/null 2>&1; then
        log INFO "Starting valgrind memory analysis..."
        timeout 60 valgrind --tool=memcheck --leak-check=full --show-leak-kinds=all \
            --track-origins=yes --log-file="${TEST_RESULTS_DIR}/valgrind_${TIMESTAMP}.log" \
            ./target/release/lair-chat-server --config config/test.toml &
        local valgrind_pid=$!

        sleep 30  # Let it run for 30 seconds under load
        kill $valgrind_pid 2>/dev/null || true
        wait $valgrind_pid 2>/dev/null || true

        log INFO "Memory leak analysis saved to valgrind_${TIMESTAMP}.log"
    else
        log WARN "Valgrind not available, skipping memory leak detection"
    fi

    # Stop performance monitoring
    "${SCRIPT_DIR}/monitor_performance.sh" --stop || log WARN "Could not stop stress monitoring"

    # Stop test server
    log INFO "Stopping stress test server..."
    kill $server_pid 2>/dev/null || true
    wait $server_pid 2>/dev/null || true

    if [ $stress_test_result -eq 0 ]; then
        log SUCCESS "Stress tests completed successfully"
        return 0
    else
        log ERROR "Stress tests failed"
        return 1
    fi
}

# Function to run security tests
run_security_tests() {
    log INFO "Starting security tests..."

    cd "${PROJECT_ROOT}"

    # Run security-focused tests
    log INFO "Running security validation tests..."
    cargo test security:: --release 2>&1 | tee -a "${TEST_LOG}"

    # Input validation tests
    log INFO "Running input validation tests..."
    cargo test validation:: --release 2>&1 | tee -a "${TEST_LOG}"

    # Authentication and authorization tests
    log INFO "Running authentication tests..."
    cargo test auth:: --release 2>&1 | tee -a "${TEST_LOG}"

    log SUCCESS "Security tests completed"
}

# Function to generate test report
generate_test_report() {
    log INFO "Generating comprehensive test report..."

    local report_file="${TEST_RESULTS_DIR}/test_report_${TIMESTAMP}.md"

    cat > "${report_file}" << EOF
# PHASE 8 TESTING REPORT

**Test Run:** ${TIMESTAMP}
**Date:** $(date)
**Duration:** $((SECONDS / 60)) minutes

## Executive Summary

This report summarizes the comprehensive testing results for the lair-chat application Phase 8 validation.

## Test Results Summary

### Unit Tests
- **Status:** $(grep -q "Unit tests completed" "${TEST_LOG}" && echo "PASSED" || echo "FAILED")
- **Coverage:** $(grep -o 'Coverage: [0-9.]*%' "${TEST_LOG}" | tail -1 || echo "Not measured")

### Integration Tests
- **Status:** $(grep -q "Integration tests completed" "${TEST_LOG}" && echo "PASSED" || echo "FAILED")

### Performance Tests
- **Status:** $(grep -q "Performance tests completed" "${TEST_LOG}" && echo "PASSED" || echo "FAILED")
- **Benchmarks:** Available in test results directory

### Load Tests
- **Status:** $(grep -q "Load tests completed successfully" "${TEST_LOG}" && echo "PASSED" || echo "FAILED")
- **Concurrent Users:** ${CONCURRENT_USERS}
- **Duration:** ${LOAD_DURATION} seconds
- **Results:** $([ -f "${TEST_RESULTS_DIR}/load_test_${TIMESTAMP}.json" ] && echo "Available" || echo "Not generated")

### Stress Tests
- **Status:** $(grep -q "Stress tests completed successfully" "${TEST_LOG}" && echo "PASSED" || echo "FAILED")
- **Max Users Tested:** Up to 500 concurrent users
- **Duration:** ${STRESS_DURATION} seconds
- **Memory Analysis:** $([ -f "${TEST_RESULTS_DIR}/valgrind_${TIMESTAMP}.log" ] && echo "Available" || echo "Not performed")
- **Results:** $([ -f "${TEST_RESULTS_DIR}/stress_test_${TIMESTAMP}.json" ] && echo "Available" || echo "Not generated")

### Security Tests
- **Status:** $(grep -q "Security tests completed" "${TEST_LOG}" && echo "PASSED" || echo "FAILED")

### Performance Monitoring
- **Load Test Monitoring:** $(find "${TEST_RESULTS_DIR}" -name "monitoring_report_*.md" -newer "${TEST_LOG}" | wc -l) reports generated
- **System Alerts:** $(find "${TEST_RESULTS_DIR}" -name "alerts_*.log" -newer "${TEST_LOG}" | wc -l) alert logs created

## Detailed Results

Full test logs are available in: \`test_run_${TIMESTAMP}.log\`

## Recommendations

$(if grep -q "ERROR" "${TEST_LOG}"; then
    echo "- **CRITICAL:** Address failed tests before production deployment"
    grep "ERROR" "${TEST_LOG}" | head -5
else
    echo "- All tests passed successfully"
    echo "- System ready for production deployment"
fi)

## Files Generated

- Test log: \`test_run_${TIMESTAMP}.log\`
- Coverage report: \`coverage/tarpaulin-report.html\`
- Performance data: \`baseline_performance.json\`
$([ -f "${TEST_RESULTS_DIR}/load_test_${TIMESTAMP}.json" ] && echo "- Load test results: \`load_test_${TIMESTAMP}.json\`")
$([ -f "${TEST_RESULTS_DIR}/stress_test_${TIMESTAMP}.json" ] && echo "- Stress test results: \`stress_test_${TIMESTAMP}.json\`")
$([ -f "${TEST_RESULTS_DIR}/valgrind_${TIMESTAMP}.log" ] && echo "- Memory analysis: \`valgrind_${TIMESTAMP}.log\`")
$(find "${TEST_RESULTS_DIR}" -name "monitoring_report_*.md" -newer "${TEST_LOG}" | head -1 | sed 's/.*\//- Performance monitoring: /' || echo "")
$(find "${TEST_RESULTS_DIR}" -name "metrics_*.csv" -newer "${TEST_LOG}" | head -1 | sed 's/.*\//- System metrics: /' || echo "")

EOF

    log SUCCESS "Test report generated: ${report_file}"
}

# Function to cleanup test environment
cleanup() {
    log INFO "Cleaning up test environment..."

    # Kill any remaining processes
    pkill -f "lair-chat-server" 2>/dev/null || true
    pkill -f "lair-chat-client" 2>/dev/null || true

    # Clean up temporary files
    rm -f /tmp/lair-chat-test-* 2>/dev/null || true

    log INFO "Cleanup completed"
}

# Function to setup test environment
setup_test_environment() {
    log INFO "Setting up test environment..."

    cd "${PROJECT_ROOT}"

    # Install required tools if not present
    if ! command -v cargo-tarpaulin >/dev/null 2>&1; then
        log INFO "Installing cargo-tarpaulin for coverage analysis..."
        cargo install cargo-tarpaulin || log WARN "Failed to install cargo-tarpaulin"
    fi

    # Build all binaries
    log INFO "Building all binaries..."
    cargo build --release --all-targets

    # Setup test database
    log INFO "Setting up test database..."
    if [ -f "${PROJECT_ROOT}/reset_database.sh" ]; then
        "${PROJECT_ROOT}/reset_database.sh" test
    fi

    # Ensure test configuration exists
    if [ ! -f "${PROJECT_ROOT}/config/test.toml" ]; then
        log WARN "Test configuration not found, creating default..."
        mkdir -p "${PROJECT_ROOT}/config"
        cat > "${PROJECT_ROOT}/config/test.toml" << 'EOF'
[database]
url = "sqlite://test_lair_chat.db"

[server]
host = "127.0.0.1"
port = 8081
tcp_port = 3335

[security]
rate_limit = 1000
max_connections = 1000

[logging]
level = "debug"
EOF
    fi

    log SUCCESS "Test environment setup completed"
}

# Main execution function
main() {
    local test_suite="all"
    local quick_mode=false

    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --suite)
                test_suite="$2"
                shift 2
                ;;
            --quick)
                quick_mode=true
                shift
                ;;
            --help)
                echo "Usage: $0 [--suite SUITE] [--quick]"
                echo "Suites: all, unit, integration, performance, load, stress, security"
                echo "Options:"
                echo "  --quick    Run abbreviated test suite for faster feedback"
                exit 0
                ;;
            *)
                log ERROR "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    log INFO "Starting Phase 8 comprehensive testing suite..."
    log INFO "Test suite: ${test_suite}"
    log INFO "Quick mode: ${quick_mode}"

    # Setup trap for cleanup
    trap cleanup EXIT

    # Setup test environment
    setup_test_environment

    local start_time=$SECONDS
    local failed_tests=0

    # Run test suites based on selection
    case $test_suite in
        "all")
            run_unit_tests || ((failed_tests++))
            run_integration_tests || ((failed_tests++))
            if [ "$quick_mode" = false ]; then
                run_performance_tests || ((failed_tests++))
                run_load_tests || ((failed_tests++))
                if [ "$quick_mode" = false ]; then
                    run_stress_tests || ((failed_tests++))
                fi
            fi
            run_security_tests || ((failed_tests++))
            ;;
        "unit")
            run_unit_tests || ((failed_tests++))
            ;;
        "integration")
            run_integration_tests || ((failed_tests++))
            ;;
        "performance")
            run_performance_tests || ((failed_tests++))
            ;;
        "load")
            run_load_tests || ((failed_tests++))
            ;;
        "stress")
            run_stress_tests || ((failed_tests++))
            ;;
        "load,stress")
            run_load_tests || ((failed_tests++))
            run_stress_tests || ((failed_tests++))
            ;;
        "security")
            run_security_tests || ((failed_tests++))
            ;;
        *)
            log ERROR "Unknown test suite: ${test_suite}"
            exit 1
            ;;
    esac

    # Generate comprehensive report
    generate_test_report

    local duration=$((SECONDS - start_time))

    if [ $failed_tests -eq 0 ]; then
        log SUCCESS "All tests completed successfully in ${duration} seconds"
        log SUCCESS "Test results available in: ${TEST_RESULTS_DIR}"
        exit 0
    else
        log ERROR "${failed_tests} test suite(s) failed"
        log ERROR "Check logs for details: ${TEST_LOG}"
        exit 1
    fi
}

# Execute main function with all arguments
main "$@"
