#!/bin/bash

# PHASE 8 TASK 8.2: Stress Testing Script
# Comprehensive stress testing for lair-chat application

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
STRESS_TEST_RESULTS_DIR="${PROJECT_ROOT}/test_results/stress_tests"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Default stress test parameters
DEFAULT_MAX_USERS=500
DEFAULT_STRESS_DURATION=300
DEFAULT_MEMORY_STRESS_SIZE=1024
DEFAULT_CONNECTION_BURST=100
DEFAULT_SERVER_HOST="127.0.0.1"
DEFAULT_SERVER_PORT=3335
DEFAULT_BREAKING_POINT_INCREMENT=50

# Test parameters (can be overridden by command line)
MAX_USERS=${DEFAULT_MAX_USERS}
STRESS_DURATION=${DEFAULT_STRESS_DURATION}
MEMORY_STRESS_SIZE=${DEFAULT_MEMORY_STRESS_SIZE}
CONNECTION_BURST=${DEFAULT_CONNECTION_BURST}
SERVER_HOST=${DEFAULT_SERVER_HOST}
SERVER_PORT=${DEFAULT_SERVER_PORT}
BREAKING_POINT_INCREMENT=${DEFAULT_BREAKING_POINT_INCREMENT}
OUTPUT_FILE=""

# Stress test tracking
BREAKING_POINT_USERS=0
RECOVERY_TIME=0
MAX_MEMORY_USAGE=0
MAX_CPU_USAGE=0
RESOURCE_EXHAUSTION_DETECTED=false
SYSTEM_FAILURE_DETECTED=false

# Ensure results directory exists
mkdir -p "${STRESS_TEST_RESULTS_DIR}"

# Logging function
log() {
    local level=$1
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    case $level in
        INFO)  echo -e "${BLUE}[INFO]${NC} ${message}" ;;
        WARN)  echo -e "${YELLOW}[WARN]${NC} ${message}" ;;
        ERROR) echo -e "${RED}[ERROR]${NC} ${message}" ;;
        SUCCESS) echo -e "${GREEN}[SUCCESS]${NC} ${message}" ;;
        DEBUG) echo -e "${PURPLE}[DEBUG]${NC} ${message}" ;;
        METRIC) echo -e "${CYAN}[METRIC]${NC} ${message}" ;;
        STRESS) echo -e "${RED}[STRESS]${NC} ${message}" ;;
    esac
}

# Usage function
usage() {
    cat << EOF
PHASE 8 TASK 8.2 - Stress Testing Script

Usage: $0 [OPTIONS]

OPTIONS:
    --max-users NUM           Maximum users to test (default: ${DEFAULT_MAX_USERS})
    --duration SECONDS        Stress test duration in seconds (default: ${DEFAULT_STRESS_DURATION})
    --memory-stress SIZE      Memory stress size in MB (default: ${DEFAULT_MEMORY_STRESS_SIZE})
    --connection-burst NUM    Connection burst size (default: ${DEFAULT_CONNECTION_BURST})
    --host HOST              Server host (default: ${DEFAULT_SERVER_HOST})
    --port PORT              Server port (default: ${DEFAULT_SERVER_PORT})
    --increment NUM          User increment for breaking point test (default: ${DEFAULT_BREAKING_POINT_INCREMENT})
    --output FILE            Output results to file
    --help                   Show this help message

EXAMPLES:
    $0 --max-users 1000 --duration 600
    $0 --memory-stress 2048 --connection-burst 200
    $0 --max-users 750 --output stress_results.json

STRESS TEST SCENARIOS:
    1. Extreme Concurrent Load Testing (500+ users)
    2. Resource Exhaustion Testing
    3. Memory Pressure Testing
    4. Connection Exhaustion Testing
    5. Recovery Testing
    6. Breaking Point Identification

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --max-users)
                MAX_USERS="$2"
                shift 2
                ;;
            --duration)
                STRESS_DURATION="$2"
                shift 2
                ;;
            --memory-stress)
                MEMORY_STRESS_SIZE="$2"
                shift 2
                ;;
            --connection-burst)
                CONNECTION_BURST="$2"
                shift 2
                ;;
            --host)
                SERVER_HOST="$2"
                shift 2
                ;;
            --port)
                SERVER_PORT="$2"
                shift 2
                ;;
            --increment)
                BREAKING_POINT_INCREMENT="$2"
                shift 2
                ;;
            --output)
                OUTPUT_FILE="$2"
                shift 2
                ;;
            --help)
                usage
                exit 0
                ;;
            *)
                log ERROR "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
}

# Check if server is running
check_server() {
    log INFO "Checking if server is running on ${SERVER_HOST}:${SERVER_PORT}..."

    if ! timeout 5 bash -c "</dev/tcp/${SERVER_HOST}/${SERVER_PORT}"; then
        log ERROR "Server is not responding on ${SERVER_HOST}:${SERVER_PORT}"
        return 1
    fi

    log SUCCESS "Server is responding"
    return 0
}

# Monitor system resources
monitor_system_resources() {
    local duration=$1
    local output_file="${STRESS_TEST_RESULTS_DIR}/resource_monitor_${TIMESTAMP}.log"
    local pid_file="${STRESS_TEST_RESULTS_DIR}/monitor_pid.tmp"

    log INFO "Starting system resource monitoring for ${duration} seconds..."

    {
        local start_time=$(date +%s)
        local max_cpu=0
        local max_memory=0
        local max_connections=0

        while true; do
            local current_time=$(date +%s)
            local elapsed=$((current_time - start_time))

            if ((elapsed >= duration)); then
                break
            fi

            # Get CPU usage
            local cpu_usage=0
            if command -v top >/dev/null 2>&1; then
                cpu_usage=$(top -bn1 | grep "Cpu(s)" | sed "s/.*, *\([0-9.]*\)%* id.*/\1/" | awk '{print 100 - $1}')
            fi

            # Get memory usage
            local memory_usage=0
            if command -v free >/dev/null 2>&1; then
                memory_usage=$(free | grep Mem | awk '{printf("%.1f", ($3/$2) * 100.0)}')
            fi

            # Get connection count (approximate)
            local connections=0
            if command -v netstat >/dev/null 2>&1; then
                connections=$(netstat -an | grep ":${SERVER_PORT}" | grep ESTABLISHED | wc -l)
            elif command -v ss >/dev/null 2>&1; then
                connections=$(ss -an | grep ":${SERVER_PORT}" | grep ESTAB | wc -l)
            fi

            # Update maximums
            if (( $(echo "$cpu_usage > $max_cpu" | bc -l 2>/dev/null || echo "0") )); then
                max_cpu=$cpu_usage
            fi

            if (( $(echo "$memory_usage > $max_memory" | bc -l 2>/dev/null || echo "0") )); then
                max_memory=$memory_usage
            fi

            if ((connections > max_connections)); then
                max_connections=$connections
            fi

            # Log current metrics
            echo "${elapsed},${cpu_usage:-0},${memory_usage:-0},${connections}" >> "$output_file"

            # Check for resource exhaustion
            if (( $(echo "${memory_usage:-0} > 90" | bc -l 2>/dev/null || echo "0") )); then
                log STRESS "Memory usage critical: ${memory_usage}%"
                RESOURCE_EXHAUSTION_DETECTED=true
            fi

            if (( $(echo "${cpu_usage:-0} > 95" | bc -l 2>/dev/null || echo "0") )); then
                log STRESS "CPU usage critical: ${cpu_usage}%"
                RESOURCE_EXHAUSTION_DETECTED=true
            fi

            sleep 2
        done

        # Store final metrics
        MAX_CPU_USAGE=$max_cpu
        MAX_MEMORY_USAGE=$max_memory

        log METRIC "Peak CPU Usage: ${max_cpu}%"
        log METRIC "Peak Memory Usage: ${max_memory}%"
        log METRIC "Peak Connections: ${max_connections}"

    } &

    echo $! > "$pid_file"
}

# Stop system monitoring
stop_monitoring() {
    local pid_file="${STRESS_TEST_RESULTS_DIR}/monitor_pid.tmp"

    if [ -f "$pid_file" ]; then
        local monitor_pid=$(cat "$pid_file")
        if kill -0 "$monitor_pid" 2>/dev/null; then
            kill "$monitor_pid" 2>/dev/null || true
            wait "$monitor_pid" 2>/dev/null || true
        fi
        rm -f "$pid_file"
    fi
}

# Simulate extreme user load
simulate_extreme_user_load() {
    local user_count=$1
    local duration=$2
    local result_file="${STRESS_TEST_RESULTS_DIR}/extreme_load_${user_count}_${TIMESTAMP}.log"

    log STRESS "Starting extreme load test with ${user_count} users for ${duration} seconds..."

    local pids=()
    local successful_users=0
    local failed_users=0

    # Start system monitoring
    monitor_system_resources "$duration"

    # Launch users in batches to prevent overwhelming the system instantly
    local batch_size=20
    local batches=$((user_count / batch_size))

    for ((batch=0; batch<batches; batch++)); do
        local batch_start=$((batch * batch_size + 1))
        local batch_end=$(((batch + 1) * batch_size))

        if ((batch_end > user_count)); then
            batch_end=$user_count
        fi

        log DEBUG "Starting user batch ${batch_start}-${batch_end}"

        for ((user=batch_start; user<=batch_end; user++)); do
            {
                local user_start=$(date +%s%3N)
                local user_name="stresstest_user_${user}"

                # Attempt connection with shorter timeout under stress
                if timeout 10 bash -c "echo 'register ${user_name} stresspass${user}' | nc ${SERVER_HOST} ${SERVER_PORT}" >/dev/null 2>&1; then
                    # Send a few messages quickly
                    for ((msg=1; msg<=3; msg++)); do
                        echo "say Stress test message ${msg} from user ${user}" | nc ${SERVER_HOST} ${SERVER_PORT} >/dev/null 2>&1 || break
                        sleep 0.1
                    done

                    echo "quit" | nc ${SERVER_HOST} ${SERVER_PORT} >/dev/null 2>&1 || true

                    local user_end=$(date +%s%3N)
                    local user_duration=$((user_end - user_start))
                    echo "user_success:${user}:${user_duration}" >> "$result_file"
                else
                    echo "user_failed:${user}" >> "$result_file"
                fi
            } &

            pids+=($!)

            # Small delay between user starts within batch
            sleep 0.05
        done

        # Wait briefly between batches to prevent system overload
        sleep 1

        # Check if we should continue based on early failures
        local current_failures=$(grep -c "user_failed" "$result_file" 2>/dev/null || echo "0")
        local current_attempts=$(((batch + 1) * batch_size))

        if ((current_attempts >= 100)); then
            local failure_rate=$(echo "scale=2; $current_failures * 100 / $current_attempts" | bc)
            if (( $(echo "$failure_rate > 50" | bc -l) )); then
                log STRESS "High failure rate detected (${failure_rate}%), stopping user creation"
                break
            fi
        fi
    done

    log INFO "All ${user_count} users started, waiting for completion..."

    # Wait for all users with timeout
    local wait_start=$(date +%s)
    for pid in "${pids[@]}"; do
        local current_time=$(date +%s)
        local wait_elapsed=$((current_time - wait_start))

        if ((wait_elapsed > duration)); then
            log WARN "Killing remaining processes due to timeout"
            kill "$pid" 2>/dev/null || true
        else
            wait "$pid" 2>/dev/null || true
        fi
    done

    # Stop monitoring
    stop_monitoring

    # Analyze results
    if [ -f "$result_file" ]; then
        successful_users=$(grep -c "user_success" "$result_file" 2>/dev/null || echo "0")
        failed_users=$(grep -c "user_failed" "$result_file" 2>/dev/null || echo "0")
    fi

    local total_users=$((successful_users + failed_users))
    local success_rate=0

    if ((total_users > 0)); then
        success_rate=$(echo "scale=2; $successful_users * 100 / $total_users" | bc)
    fi

    log METRIC "Extreme load results: ${successful_users}/${total_users} users successful (${success_rate}%)"

    # Determine if this represents system breaking point
    if (( $(echo "$success_rate < 50" | bc -l) )); then
        log STRESS "System breaking point detected at ${user_count} users"
        BREAKING_POINT_USERS=$user_count
        SYSTEM_FAILURE_DETECTED=true
        return 1
    fi

    return 0
}

# Test memory pressure scenarios
test_memory_pressure() {
    log STRESS "Testing memory pressure scenarios..."

    local memory_test_results="${STRESS_TEST_RESULTS_DIR}/memory_pressure_${TIMESTAMP}.log"
    local chunk_size_mb=$MEMORY_STRESS_SIZE
    local max_chunks=50

    log INFO "Applying memory pressure: ${chunk_size_mb}MB chunks, max ${max_chunks} chunks"

    # Start monitoring
    monitor_system_resources 120

    # Allocate memory in chunks while testing system responsiveness
    for ((chunk=1; chunk<=max_chunks; chunk++)); do
        log DEBUG "Allocating memory chunk ${chunk}/${max_chunks} (${chunk_size_mb}MB each)"

        # Use dd to create memory pressure
        local temp_file="/tmp/stress_memory_chunk_${chunk}_${TIMESTAMP}"
        if dd if=/dev/zero of="$temp_file" bs=1M count="$chunk_size_mb" 2>/dev/null; then
            echo "memory_chunk_allocated:${chunk}:${chunk_size_mb}" >> "$memory_test_results"

            # Test system responsiveness under memory pressure
            local response_start=$(date +%s%3N)
            if timeout 5 bash -c "</dev/tcp/${SERVER_HOST}/${SERVER_PORT}"; then
                local response_end=$(date +%s%3N)
                local response_time=$((response_end - response_start))
                echo "responsiveness_test:${chunk}:${response_time}" >> "$memory_test_results"

                if ((response_time > 1000)); then
                    log STRESS "System response degraded under memory pressure: ${response_time}ms"
                fi
            else
                echo "responsiveness_failed:${chunk}" >> "$memory_test_results"
                log STRESS "System became unresponsive under memory pressure"
                RESOURCE_EXHAUSTION_DETECTED=true
            fi
        else
            echo "memory_chunk_failed:${chunk}" >> "$memory_test_results"
            log STRESS "Failed to allocate memory chunk ${chunk}"
            break
        fi

        # Brief pause between allocations
        sleep 2

        # Check available memory
        if command -v free >/dev/null 2>&1; then
            local available_memory=$(free -m | awk 'NR==2{printf "%.1f", $7/$2*100.0}')
            if (( $(echo "$available_memory < 10" | bc -l 2>/dev/null || echo "0") )); then
                log STRESS "Available memory critically low: ${available_memory}%"
                break
            fi
        fi
    done

    # Gradually release memory and test recovery
    log INFO "Testing memory pressure recovery..."
    local recovery_start=$(date +%s)

    for temp_file in /tmp/stress_memory_chunk_*_${TIMESTAMP}; do
        if [ -f "$temp_file" ]; then
            rm -f "$temp_file"

            # Test recovery after each chunk release
            if timeout 3 bash -c "</dev/tcp/${SERVER_HOST}/${SERVER_PORT}"; then
                echo "recovery_test_success" >> "$memory_test_results"
            else
                echo "recovery_test_failed" >> "$memory_test_results"
            fi

            sleep 1
        fi
    done

    local recovery_end=$(date +%s)
    RECOVERY_TIME=$((recovery_end - recovery_start))

    stop_monitoring

    log METRIC "Memory pressure recovery time: ${RECOVERY_TIME} seconds"
    log SUCCESS "Memory pressure testing completed"
}

# Test connection exhaustion
test_connection_exhaustion() {
    log STRESS "Testing connection exhaustion scenarios..."

    local conn_test_results="${STRESS_TEST_RESULTS_DIR}/connection_exhaustion_${TIMESTAMP}.log"
    local target_connections=200
    local connection_pids=()

    log INFO "Attempting to establish ${target_connections} concurrent connections"

    # Start monitoring
    monitor_system_resources 60

    # Establish connections rapidly
    for ((conn=1; conn<=target_connections; conn++)); do
        {
            local conn_start=$(date +%s%3N)

            # Hold connection open for extended period
            if timeout 30 bash -c "exec 3<>/dev/tcp/${SERVER_HOST}/${SERVER_PORT}; sleep 25; exec 3<&-"; then
                local conn_end=$(date +%s%3N)
                local conn_time=$((conn_end - conn_start))
                echo "connection_success:${conn}:${conn_time}" >> "$conn_test_results"
            else
                echo "connection_failed:${conn}" >> "$conn_test_results"
            fi
        } &

        connection_pids+=($!)

        # Brief delay to prevent instant overwhelming
        sleep 0.02

        # Check server responsiveness every 50 connections
        if ((conn % 50 == 0)); then
            if ! timeout 2 bash -c "</dev/tcp/${SERVER_HOST}/${SERVER_PORT}"; then
                log STRESS "Server became unresponsive after ${conn} connections"
                break
            fi
            log DEBUG "Established ${conn} connections, server still responsive"
        fi
    done

    log INFO "Waiting for connection tests to complete..."

    # Wait for all connection tests
    for pid in "${connection_pids[@]}"; do
        wait "$pid" 2>/dev/null || true
    done

    stop_monitoring

    # Analyze connection exhaustion results
    local successful_connections=$(grep -c "connection_success" "$conn_test_results" 2>/dev/null || echo "0")
    local failed_connections=$(grep -c "connection_failed" "$conn_test_results" 2>/dev/null || echo "0")
    local total_attempts=$((successful_connections + failed_connections))

    log METRIC "Connection exhaustion results: ${successful_connections}/${total_attempts} successful"

    if ((failed_connections > 0)); then
        log STRESS "Connection limit reached: ${successful_connections} max concurrent connections"
    fi

    log SUCCESS "Connection exhaustion testing completed"
}

# Test breaking point identification
test_breaking_point() {
    log STRESS "Identifying system breaking point..."

    local current_users=$BREAKING_POINT_INCREMENT
    local breaking_point_found=false

    while ((current_users <= MAX_USERS)) && [ "$breaking_point_found" = false ]; do
        log INFO "Testing breaking point with ${current_users} users..."

        if simulate_extreme_user_load "$current_users" 60; then
            log SUCCESS "System handled ${current_users} users successfully"
            current_users=$((current_users + BREAKING_POINT_INCREMENT))
        else
            log STRESS "Breaking point found at ${current_users} users"
            BREAKING_POINT_USERS=$current_users
            breaking_point_found=true
        fi

        # Brief recovery period between tests
        log INFO "Recovery period..."
        sleep 10
    done

    if [ "$breaking_point_found" = false ]; then
        log SUCCESS "System handled up to ${MAX_USERS} users without breaking"
        BREAKING_POINT_USERS=$MAX_USERS
    fi
}

# Test failure recovery scenarios
test_failure_recovery() {
    log STRESS "Testing failure recovery scenarios..."

    local recovery_results="${STRESS_TEST_RESULTS_DIR}/failure_recovery_${TIMESTAMP}.log"

    # Test 1: Network interruption simulation
    log INFO "Simulating network interruption..."

    # Block traffic to server port temporarily
    if command -v iptables >/dev/null 2>&1 && [ "$EUID" -eq 0 ]; then
        log INFO "Using iptables to simulate network interruption"

        # Block incoming connections
        iptables -A INPUT -p tcp --dport "$SERVER_PORT" -j DROP 2>/dev/null || log WARN "Could not modify iptables"

        # Wait for interruption period
        sleep 5

        # Restore connections
        iptables -D INPUT -p tcp --dport "$SERVER_PORT" -j DROP 2>/dev/null || log WARN "Could not restore iptables"

        echo "network_interruption_simulated" >> "$recovery_results"
    else
        log WARN "Cannot simulate network interruption (requires root and iptables)"
    fi

    # Test 2: High load followed by recovery
    log INFO "Testing recovery from high load..."

    local recovery_start=$(date +%s)

    # Apply high load
    simulate_extreme_user_load $((BREAKING_POINT_USERS / 2)) 30

    # Wait for recovery
    sleep 5

    # Test if system recovers
    local recovery_attempts=0
    local max_recovery_attempts=10

    while ((recovery_attempts < max_recovery_attempts)); do
        if timeout 3 bash -c "</dev/tcp/${SERVER_HOST}/${SERVER_PORT}"; then
            local recovery_end=$(date +%s)
            local recovery_time=$((recovery_end - recovery_start))

            log SUCCESS "System recovered in ${recovery_time} seconds"
            echo "recovery_success:${recovery_time}" >> "$recovery_results"
            break
        fi

        ((recovery_attempts++))
        sleep 2
    done

    if ((recovery_attempts >= max_recovery_attempts)); then
        log ERROR "System did not recover within expected time"
        echo "recovery_failed" >> "$recovery_results"
    fi

    log SUCCESS "Failure recovery testing completed"
}

# Analyze stress test results
analyze_stress_results() {
    log INFO "Analyzing stress test results..."

    local analysis_file="${STRESS_TEST_RESULTS_DIR}/stress_analysis_${TIMESTAMP}.json"

    # Calculate various metrics from test results
    local total_tests_run=0
    local tests_passed=0
    local tests_failed=0

    # Count test results
    for result_file in "${STRESS_TEST_RESULTS_DIR}"/*_${TIMESTAMP}.log; do
        if [ -f "$result_file" ]; then
            local file_tests=$(wc -l < "$result_file" 2>/dev/null || echo "0")
            total_tests_run=$((total_tests_run + file_tests))

            local file_successes=$(grep -c "success" "$result_file" 2>/dev/null || echo "0")
            local file_failures=$(grep -c "failed" "$result_file" 2>/dev/null || echo "0")

            tests_passed=$((tests_passed + file_successes))
            tests_failed=$((tests_failed + file_failures))
        fi
    done

    # Generate comprehensive analysis
    cat > "$analysis_file" << EOF
{
    "test_parameters": {
        "max_users_tested": ${MAX_USERS},
        "stress_duration": ${STRESS_DURATION},
        "memory_stress_size_mb": ${MEMORY_STRESS_SIZE},
        "connection_burst": ${CONNECTION_BURST}
    },
    "breaking_point_analysis": {
        "maximum_concurrent_users": ${BREAKING_POINT_USERS},
        "system_failure_detected": ${SYSTEM_FAILURE_DETECTED},
        "resource_exhaustion_detected": ${RESOURCE_EXHAUSTION_DETECTED}
    },
    "resource_usage": {
        "peak_cpu_usage_percent": ${MAX_CPU_USAGE},
        "peak_memory_usage_percent": ${MAX_MEMORY_USAGE}
    },
    "recovery_metrics": {
        "recovery_time_seconds": ${RECOVERY_TIME}
    },
    "test_execution": {
        "total_tests_run": ${total_tests_run},
        "tests_passed": ${tests_passed},
        "tests_failed": ${tests_failed},
        "overall_success_rate": $(echo "scale=2; $tests_passed * 100 / ($tests_passed + $tests_failed)" | bc 2>/dev/null || echo "0")
    },
    "timestamp": "${TIMESTAMP}",
    "test_date": "$(date)"
}
EOF

    # Display key results
    log SUCCESS "Stress test analysis completed"
    log METRIC "Maximum Concurrent Users: ${BREAKING_POINT_USERS}"
    log METRIC "Peak CPU Usage: ${MAX_CPU_USAGE}%"
    log METRIC "Peak Memory Usage: ${MAX_MEMORY_USAGE}%"
    log METRIC "Recovery Time: ${RECOVERY_TIME} seconds"
    log METRIC "System Failure Detected: ${SYSTEM_FAILURE_DETECTED}"
    log METRIC "Resource Exhaustion Detected: ${RESOURCE_EXHAUSTION_DETECTED}"

    # Save output file if specified
    if [ -n "$OUTPUT_FILE" ]; then
        cp "$analysis_file" "$OUTPUT_FILE"
        log SUCCESS "Results saved to: $OUTPUT_FILE"
    fi

    log INFO "Detailed analysis available in: $analysis_file"

    # Check success criteria
    local success=true

    if ((BREAKING_POINT_USERS < 100)); then
        log ERROR "Breaking point too low: ${BREAKING_POINT_USERS} users (minimum 100 expected)"
        success=false
    fi

    if ((RECOVERY_TIME > 30)); then
        log ERROR "Recovery time too long: ${RECOVERY_TIME} seconds (maximum 30 expected)"
        success=false
    fi

    if [ "$success" = true ]; then
        log SUCCESS "Stress test SUCCESS CRITERIA MET"
        return 0
    else
        log ERROR "Stress test success criteria NOT MET"
        return 1
    fi
}

# Main execution function
main() {
    log INFO "Starting PHASE 8 TASK 8.2 Stress Testing..."
    log INFO "Timestamp: ${TIMESTAMP}"

    # Parse command line arguments
    parse_args "$@"

    # Display test configuration
    log INFO "Stress Test Configuration:"
    log INFO "  - Maximum Users: ${MAX_USERS}"
    log INFO "  - Stress Duration: ${STRESS_DURATION} seconds"
    log INFO "  - Memory Stress Size: ${MEMORY_STRESS_SIZE} MB"
    log INFO "  - Connection Burst: ${CONNECTION_BURST}"
    log INFO "  - Server: ${SERVER_HOST}:${SERVER_PORT}"
    log INFO "  - Breaking Point Increment: ${BREAKING_POINT_INCREMENT}"

    # Check prerequisites
    if ! check_server; then
        log ERROR "Server check failed. Please ensure the lair-chat server is running."
        exit 1
    fi

    # Check required tools
    if ! command -v bc >/dev/null 2>&1; then
        log ERROR "bc (calculator) not found. Please install: sudo apt-get install bc"
        exit 1
    fi

    if ! command -v nc >/dev/null 2>&1; then
        log ERROR "nc (netcat) not found. Please install: sudo apt-get install netcat"
        exit 1
    fi

    # Run stress tests
    local start_time=$(date +%s)

    # 1. Breaking Point Identification
    test_breaking_point

    # 2. Memory Pressure Testing
    test_memory_pressure

    # 3. Connection Exhaustion Testing
    test_connection_exhaustion

    # 4. Failure Recovery Testing
    test_failure_recovery

    # 5. Final extreme load test
    if ((BREAKING_POINT_USERS > 0)); then
        log INFO "Running final validation at 80% of breaking point..."
        local validation_users=$((BREAKING_POINT_USERS * 4 / 5))
        simulate_extreme_user_load "$validation_users" 60
    fi

    # Analyze all results
    if analyze_stress_results; then
        local end_time=$(date +%s)
        local total_duration=$((end_time - start_time))

        log SUCCESS "Stress testing completed successfully in ${total_duration} seconds"
        log INFO "Results directory: ${STRESS_TEST_RESULTS_DIR}"
        exit 0
    else
        log ERROR "Stress testing completed with failures"
        exit 1
    fi
}

# Cleanup function
cleanup() {
    log INFO "Cleaning up stress test processes..."

    # Stop monitoring
    stop_monitoring

    # Kill any remaining test processes
    pkill -f "stresstest_user" 2>/dev/null || true
    pkill -f "nc ${SERVER_HOST} ${SERVER_PORT}" 2>/dev/null || true

    # Clean up temporary files
    rm -f /tmp/stress_memory_chunk_*_${TIMESTAMP} 2>/dev/null || true
    rm -f "${STRESS_TEST_RESULTS_DIR}/monitor_pid.tmp" 2>/dev/null || true

    log INFO "Cleanup completed"
}

# Set up signal handlers
trap cleanup EXIT INT TERM

# Execute main function with all arguments
main "$@"
