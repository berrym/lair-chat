#!/bin/bash

# PHASE 8 TASK 8.2: Performance Monitoring Script
# Real-time system monitoring during load and stress testing

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
MONITOR_RESULTS_DIR="${PROJECT_ROOT}/test_results/monitoring"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Default monitoring parameters
DEFAULT_DURATION=300
DEFAULT_INTERVAL=2
DEFAULT_SERVER_PORT=3335
DEFAULT_API_PORT=8081
DEFAULT_ALERT_CPU=80
DEFAULT_ALERT_MEMORY=85
DEFAULT_ALERT_CONNECTIONS=1000
DEFAULT_ALERT_RESPONSE_TIME=1000

# Monitoring parameters
MONITOR_DURATION=${DEFAULT_DURATION}
SAMPLE_INTERVAL=${DEFAULT_INTERVAL}
SERVER_PORT=${DEFAULT_SERVER_PORT}
API_PORT=${DEFAULT_API_PORT}
ALERT_CPU_THRESHOLD=${DEFAULT_ALERT_CPU}
ALERT_MEMORY_THRESHOLD=${DEFAULT_ALERT_MEMORY}
ALERT_CONNECTIONS_THRESHOLD=${DEFAULT_ALERT_CONNECTIONS}
ALERT_RESPONSE_TIME_THRESHOLD=${DEFAULT_ALERT_RESPONSE_TIME}

# Monitoring state
MONITORING_ACTIVE=false
MONITOR_PID=""
ALERT_COUNT=0
CRITICAL_ALERTS=0

# Ensure results directory exists
mkdir -p "${MONITOR_RESULTS_DIR}"

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
        ALERT) echo -e "${RED}[ALERT]${NC} ${message}" ;;
    esac

    # Also log to file with timestamp
    echo "[$timestamp] [$level] $message" >> "${MONITOR_RESULTS_DIR}/monitor_${TIMESTAMP}.log"
}

# Usage function
usage() {
    cat << EOF
PHASE 8 TASK 8.2 - Performance Monitoring Script

Usage: $0 [OPTIONS]

OPTIONS:
    --duration SECONDS        Monitoring duration in seconds (default: ${DEFAULT_DURATION})
    --interval SECONDS        Sample interval in seconds (default: ${DEFAULT_INTERVAL})
    --server-port PORT        Server port to monitor (default: ${DEFAULT_SERVER_PORT})
    --api-port PORT          API port to monitor (default: ${DEFAULT_API_PORT})
    --alert-cpu PERCENT      CPU usage alert threshold (default: ${DEFAULT_ALERT_CPU})
    --alert-memory PERCENT   Memory usage alert threshold (default: ${DEFAULT_ALERT_MEMORY})
    --alert-connections NUM  Connection count alert threshold (default: ${DEFAULT_ALERT_CONNECTIONS})
    --alert-response-time MS Response time alert threshold (default: ${DEFAULT_ALERT_RESPONSE_TIME})
    --start                  Start monitoring in background
    --stop                   Stop background monitoring
    --status                 Show monitoring status
    --help                   Show this help message

EXAMPLES:
    $0 --duration 600 --interval 1
    $0 --start --duration 1800
    $0 --stop
    $0 --alert-cpu 90 --alert-memory 95

MONITORING METRICS:
    - CPU usage percentage
    - Memory usage percentage and available memory
    - Network connections and bandwidth
    - Disk I/O operations and usage
    - Application-specific metrics
    - Response time measurements
    - Connection counts and rates

EOF
}

# Parse command line arguments
parse_args() {
    local action="monitor"

    while [[ $# -gt 0 ]]; do
        case $1 in
            --duration)
                MONITOR_DURATION="$2"
                shift 2
                ;;
            --interval)
                SAMPLE_INTERVAL="$2"
                shift 2
                ;;
            --server-port)
                SERVER_PORT="$2"
                shift 2
                ;;
            --api-port)
                API_PORT="$2"
                shift 2
                ;;
            --alert-cpu)
                ALERT_CPU_THRESHOLD="$2"
                shift 2
                ;;
            --alert-memory)
                ALERT_MEMORY_THRESHOLD="$2"
                shift 2
                ;;
            --alert-connections)
                ALERT_CONNECTIONS_THRESHOLD="$2"
                shift 2
                ;;
            --alert-response-time)
                ALERT_RESPONSE_TIME_THRESHOLD="$2"
                shift 2
                ;;
            --start)
                action="start"
                shift
                ;;
            --stop)
                action="stop"
                shift
                ;;
            --status)
                action="status"
                shift
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

    case $action in
        "monitor") main_monitor ;;
        "start") start_background_monitoring ;;
        "stop") stop_background_monitoring ;;
        "status") show_monitoring_status ;;
    esac
}

# Get CPU usage
get_cpu_usage() {
    if command -v top >/dev/null 2>&1; then
        # Use top command
        top -bn1 | grep "Cpu(s)" | sed "s/.*, *\([0-9.]*\)%* id.*/\1/" | awk '{print 100 - $1}'
    elif command -v sar >/dev/null 2>&1; then
        # Use sar if available
        sar 1 1 | grep "Average" | awk '{print 100 - $8}'
    elif [ -f /proc/stat ]; then
        # Fallback to /proc/stat
        grep 'cpu ' /proc/stat | awk '{usage=($2+$4)*100/($2+$3+$4+$5)} END {print usage}'
    else
        echo "0"
    fi
}

# Get memory usage
get_memory_usage() {
    if command -v free >/dev/null 2>&1; then
        # Get memory percentage used
        free | grep Mem | awk '{printf("%.1f"), ($3/$2) * 100.0}'
    elif [ -f /proc/meminfo ]; then
        # Fallback to /proc/meminfo
        local total=$(grep MemTotal /proc/meminfo | awk '{print $2}')
        local available=$(grep MemAvailable /proc/meminfo | awk '{print $2}')
        if [ -n "$total" ] && [ -n "$available" ]; then
            local used=$((total - available))
            echo "scale=1; $used * 100 / $total" | bc
        else
            echo "0"
        fi
    else
        echo "0"
    fi
}

# Get available memory in MB
get_available_memory() {
    if command -v free >/dev/null 2>&1; then
        free -m | grep Mem | awk '{print $7}'
    else
        echo "0"
    fi
}

# Get network connections
get_connection_count() {
    local port=$1

    if command -v netstat >/dev/null 2>&1; then
        netstat -an | grep ":${port}" | grep ESTABLISHED | wc -l
    elif command -v ss >/dev/null 2>&1; then
        ss -an | grep ":${port}" | grep ESTAB | wc -l
    else
        echo "0"
    fi
}

# Get disk usage
get_disk_usage() {
    if command -v df >/dev/null 2>&1; then
        df -h / | awk 'NR==2 {print $5}' | sed 's/%//'
    else
        echo "0"
    fi
}

# Get disk I/O stats
get_disk_io() {
    if command -v iostat >/dev/null 2>&1; then
        iostat -d 1 1 | tail -n +4 | head -1 | awk '{print $3","$4}'
    elif [ -f /proc/diskstats ]; then
        # Simple read/write sectors from /proc/diskstats
        awk '/sda/ {print $6","$10; exit}' /proc/diskstats 2>/dev/null || echo "0,0"
    else
        echo "0,0"
    fi
}

# Test server response time
test_response_time() {
    local host="127.0.0.1"
    local port=$1
    local start_time
    local end_time
    local response_time

    start_time=$(date +%s%3N)

    if timeout 5 bash -c "</dev/tcp/${host}/${port}" 2>/dev/null; then
        end_time=$(date +%s%3N)
        response_time=$((end_time - start_time))
        echo "$response_time"
    else
        echo "-1"  # Connection failed
    fi
}

# Test API response time
test_api_response_time() {
    local host="127.0.0.1"
    local port=$1
    local endpoint="/api/v1/health"
    local start_time
    local end_time
    local response_time

    start_time=$(date +%s%3N)

    if curl -s -f "http://${host}:${port}${endpoint}" >/dev/null 2>&1; then
        end_time=$(date +%s%3N)
        response_time=$((end_time - start_time))
        echo "$response_time"
    else
        echo "-1"  # API call failed
    fi
}

# Get process-specific metrics for lair-chat server
get_server_metrics() {
    local server_pid=$(pgrep -f "lair-chat-server" | head -1)

    if [ -n "$server_pid" ]; then
        if command -v ps >/dev/null 2>&1; then
            # Get process CPU and memory usage
            ps -p "$server_pid" -o %cpu,%mem,vsz,rss --no-headers 2>/dev/null || echo "0 0 0 0"
        else
            echo "0 0 0 0"
        fi
    else
        echo "0 0 0 0"
    fi
}

# Check for alerts
check_alerts() {
    local cpu_usage=$1
    local memory_usage=$2
    local connections=$3
    local response_time=$4
    local available_memory=$5

    local alerts_triggered=false

    # CPU usage alert
    if (( $(echo "$cpu_usage > $ALERT_CPU_THRESHOLD" | bc -l 2>/dev/null || echo "0") )); then
        log ALERT "CPU usage critical: ${cpu_usage}% (threshold: ${ALERT_CPU_THRESHOLD}%)"
        ((ALERT_COUNT++))
        alerts_triggered=true
    fi

    # Memory usage alert
    if (( $(echo "$memory_usage > $ALERT_MEMORY_THRESHOLD" | bc -l 2>/dev/null || echo "0") )); then
        log ALERT "Memory usage critical: ${memory_usage}% (threshold: ${ALERT_MEMORY_THRESHOLD}%)"
        ((ALERT_COUNT++))
        alerts_triggered=true
    fi

    # Low available memory alert
    if (( available_memory < 100 )); then
        log ALERT "Available memory critically low: ${available_memory}MB"
        ((ALERT_COUNT++))
        alerts_triggered=true
    fi

    # Connection count alert
    if (( connections > ALERT_CONNECTIONS_THRESHOLD )); then
        log ALERT "Connection count high: ${connections} (threshold: ${ALERT_CONNECTIONS_THRESHOLD})"
        ((ALERT_COUNT++))
        alerts_triggered=true
    fi

    # Response time alert
    if (( response_time > ALERT_RESPONSE_TIME_THRESHOLD && response_time != -1 )); then
        log ALERT "Response time high: ${response_time}ms (threshold: ${ALERT_RESPONSE_TIME_THRESHOLD}ms)"
        ((ALERT_COUNT++))
        alerts_triggered=true
    fi

    # Server unresponsive alert
    if (( response_time == -1 )); then
        log ALERT "Server unresponsive"
        ((CRITICAL_ALERTS++))
        alerts_triggered=true
    fi

    return $([ "$alerts_triggered" = true ] && echo 1 || echo 0)
}

# Main monitoring function
main_monitor() {
    log INFO "Starting performance monitoring..."
    log INFO "Duration: ${MONITOR_DURATION} seconds, Interval: ${SAMPLE_INTERVAL} seconds"
    log INFO "Monitoring ports: TCP ${SERVER_PORT}, API ${API_PORT}"
    log INFO "Alert thresholds: CPU ${ALERT_CPU_THRESHOLD}%, Memory ${ALERT_MEMORY_THRESHOLD}%, Connections ${ALERT_CONNECTIONS_THRESHOLD}, Response ${ALERT_RESPONSE_TIME_THRESHOLD}ms"

    local metrics_file="${MONITOR_RESULTS_DIR}/metrics_${TIMESTAMP}.csv"
    local alerts_file="${MONITOR_RESULTS_DIR}/alerts_${TIMESTAMP}.log"

    # Write CSV header
    echo "timestamp,elapsed,cpu_percent,memory_percent,available_memory_mb,disk_usage_percent,disk_read,disk_write,tcp_connections,api_connections,server_response_ms,api_response_ms,server_cpu,server_memory,server_vsz,server_rss" > "$metrics_file"

    local start_time=$(date +%s)
    local sample_count=0
    MONITORING_ACTIVE=true

    # Install signal handlers
    trap 'MONITORING_ACTIVE=false' INT TERM

    while [ "$MONITORING_ACTIVE" = true ]; do
        local current_time=$(date +%s)
        local elapsed=$((current_time - start_time))

        # Check if monitoring duration exceeded
        if (( elapsed >= MONITOR_DURATION )); then
            log INFO "Monitoring duration reached, stopping..."
            break
        fi

        # Collect system metrics
        local cpu_usage=$(get_cpu_usage)
        local memory_usage=$(get_memory_usage)
        local available_memory=$(get_available_memory)
        local disk_usage=$(get_disk_usage)
        local disk_io=$(get_disk_io)
        local disk_read=$(echo "$disk_io" | cut -d',' -f1)
        local disk_write=$(echo "$disk_io" | cut -d',' -f2)

        # Collect network metrics
        local tcp_connections=$(get_connection_count "$SERVER_PORT")
        local api_connections=$(get_connection_count "$API_PORT")

        # Collect response time metrics
        local server_response=$(test_response_time "$SERVER_PORT")
        local api_response=$(test_api_response_time "$API_PORT")

        # Collect server process metrics
        local server_metrics=$(get_server_metrics)
        local server_cpu=$(echo "$server_metrics" | awk '{print $1}')
        local server_memory=$(echo "$server_metrics" | awk '{print $2}')
        local server_vsz=$(echo "$server_metrics" | awk '{print $3}')
        local server_rss=$(echo "$server_metrics" | awk '{print $4}')

        # Write metrics to CSV
        echo "${current_time},${elapsed},${cpu_usage:-0},${memory_usage:-0},${available_memory:-0},${disk_usage:-0},${disk_read:-0},${disk_write:-0},${tcp_connections:-0},${api_connections:-0},${server_response:-0},${api_response:-0},${server_cpu:-0},${server_memory:-0},${server_vsz:-0},${server_rss:-0}" >> "$metrics_file"

        # Check for alerts
        if check_alerts "$cpu_usage" "$memory_usage" "$tcp_connections" "$server_response" "$available_memory"; then
            echo "$(date '+%Y-%m-%d %H:%M:%S') - Alert triggered at sample $sample_count" >> "$alerts_file"
        fi

        # Display real-time metrics every 10 samples
        if (( sample_count % 10 == 0 )); then
            log METRIC "Sample $sample_count (${elapsed}s): CPU ${cpu_usage:-0}%, Mem ${memory_usage:-0}%, Conns ${tcp_connections:-0}, Response ${server_response:-0}ms"
        fi

        ((sample_count++))
        sleep "$SAMPLE_INTERVAL"
    done

    MONITORING_ACTIVE=false

    # Generate summary report
    generate_monitoring_report "$metrics_file" "$alerts_file"

    log SUCCESS "Monitoring completed. Total samples: $sample_count, Alerts: $ALERT_COUNT, Critical: $CRITICAL_ALERTS"
}

# Generate monitoring report
generate_monitoring_report() {
    local metrics_file=$1
    local alerts_file=$2
    local report_file="${MONITOR_RESULTS_DIR}/monitoring_report_${TIMESTAMP}.md"

    log INFO "Generating monitoring report..."

    # Calculate statistics from metrics
    local avg_cpu=$(tail -n +2 "$metrics_file" | awk -F',' '{sum+=$3; count++} END {print sum/count}')
    local max_cpu=$(tail -n +2 "$metrics_file" | awk -F',' 'BEGIN{max=0} {if($3>max) max=$3} END {print max}')
    local avg_memory=$(tail -n +2 "$metrics_file" | awk -F',' '{sum+=$4; count++} END {print sum/count}')
    local max_memory=$(tail -n +2 "$metrics_file" | awk -F',' 'BEGIN{max=0} {if($4>max) max=$4} END {print max}')
    local min_available_memory=$(tail -n +2 "$metrics_file" | awk -F',' 'BEGIN{min=999999} {if($5<min) min=$5} END {print min}')
    local max_connections=$(tail -n +2 "$metrics_file" | awk -F',' 'BEGIN{max=0} {if($9>max) max=$9} END {print max}')
    local avg_response_time=$(tail -n +2 "$metrics_file" | awk -F',' '{if($11>0) {sum+=$11; count++}} END {if(count>0) print sum/count; else print 0}')
    local max_response_time=$(tail -n +2 "$metrics_file" | awk -F',' 'BEGIN{max=0} {if($11>max) max=$11} END {print max}')

    cat > "$report_file" << EOF
# Performance Monitoring Report

**Monitoring Session:** ${TIMESTAMP}
**Duration:** ${MONITOR_DURATION} seconds
**Sample Interval:** ${SAMPLE_INTERVAL} seconds
**Generated:** $(date)

## Executive Summary

This report summarizes system performance during load/stress testing for the lair-chat application.

## System Metrics Summary

### CPU Usage
- **Average CPU Usage:** ${avg_cpu:-0}%
- **Peak CPU Usage:** ${max_cpu:-0}%
- **CPU Alert Threshold:** ${ALERT_CPU_THRESHOLD}%

### Memory Usage
- **Average Memory Usage:** ${avg_memory:-0}%
- **Peak Memory Usage:** ${max_memory:-0}%
- **Minimum Available Memory:** ${min_available_memory:-0} MB
- **Memory Alert Threshold:** ${ALERT_MEMORY_THRESHOLD}%

### Network Performance
- **Peak Concurrent Connections:** ${max_connections:-0}
- **Connection Alert Threshold:** ${ALERT_CONNECTIONS_THRESHOLD}

### Response Time
- **Average Response Time:** ${avg_response_time:-0} ms
- **Peak Response Time:** ${max_response_time:-0} ms
- **Response Time Alert Threshold:** ${ALERT_RESPONSE_TIME_THRESHOLD} ms

## Alert Summary

- **Total Alerts Generated:** ${ALERT_COUNT}
- **Critical Alerts:** ${CRITICAL_ALERTS}

$(if [ -f "$alerts_file" ] && [ -s "$alerts_file" ]; then
    echo "### Alert Details"
    echo '```'
    cat "$alerts_file"
    echo '```'
else
    echo "No alerts were generated during monitoring."
fi)

## Performance Analysis

### System Stability
$(if (( $(echo "${max_cpu:-0} < $ALERT_CPU_THRESHOLD" | bc -l 2>/dev/null || echo "1") )); then
    echo "✅ CPU usage remained within acceptable limits"
else
    echo "⚠️  CPU usage exceeded alert threshold"
fi)

$(if (( $(echo "${max_memory:-0} < $ALERT_MEMORY_THRESHOLD" | bc -l 2>/dev/null || echo "1") )); then
    echo "✅ Memory usage remained within acceptable limits"
else
    echo "⚠️  Memory usage exceeded alert threshold"
fi)

$(if (( ${max_connections:-0} < ALERT_CONNECTIONS_THRESHOLD )); then
    echo "✅ Connection count remained within acceptable limits"
else
    echo "⚠️  Connection count exceeded alert threshold"
fi)

$(if (( $(echo "${max_response_time:-0} < $ALERT_RESPONSE_TIME_THRESHOLD" | bc -l 2>/dev/null || echo "1") )); then
    echo "✅ Response times remained within acceptable limits"
else
    echo "⚠️  Response times exceeded alert threshold"
fi)

### Recommendations

$(if (( ALERT_COUNT == 0 )); then
    echo "- System performed well under load with no alerts triggered"
    echo "- Current configuration appears suitable for tested load levels"
else
    echo "- Review system resources and configuration due to ${ALERT_COUNT} alerts"
    echo "- Consider optimizing performance or increasing system resources"
fi)

$(if (( CRITICAL_ALERTS > 0 )); then
    echo "- **CRITICAL:** System became unresponsive ${CRITICAL_ALERTS} times"
    echo "- Immediate investigation required for system stability"
fi)

## Data Files

- **Detailed Metrics:** \`$(basename "$metrics_file")\`
- **Alert Log:** \`$(basename "$alerts_file")\`
- **Raw Data Location:** \`${MONITOR_RESULTS_DIR}\`

## Test Environment

- **Server Port:** ${SERVER_PORT}
- **API Port:** ${API_PORT}
- **Monitoring Host:** $(hostname)
- **System:** $(uname -a)

EOF

    log SUCCESS "Monitoring report generated: $report_file"
}

# Start background monitoring
start_background_monitoring() {
    local pid_file="${MONITOR_RESULTS_DIR}/monitor.pid"

    if [ -f "$pid_file" ] && kill -0 "$(cat "$pid_file")" 2>/dev/null; then
        log WARN "Monitoring is already running (PID: $(cat "$pid_file"))"
        exit 1
    fi

    log INFO "Starting background monitoring..."

    # Start monitoring in background
    nohup "$0" --duration "$MONITOR_DURATION" --interval "$SAMPLE_INTERVAL" \
          --server-port "$SERVER_PORT" --api-port "$API_PORT" \
          --alert-cpu "$ALERT_CPU_THRESHOLD" --alert-memory "$ALERT_MEMORY_THRESHOLD" \
          --alert-connections "$ALERT_CONNECTIONS_THRESHOLD" \
          --alert-response-time "$ALERT_RESPONSE_TIME_THRESHOLD" \
          > "${MONITOR_RESULTS_DIR}/monitor_background_${TIMESTAMP}.log" 2>&1 &

    echo $! > "$pid_file"
    log SUCCESS "Background monitoring started (PID: $!)"
}

# Stop background monitoring
stop_background_monitoring() {
    local pid_file="${MONITOR_RESULTS_DIR}/monitor.pid"

    if [ ! -f "$pid_file" ]; then
        log WARN "No background monitoring process found"
        exit 1
    fi

    local monitor_pid=$(cat "$pid_file")

    if kill -0 "$monitor_pid" 2>/dev/null; then
        log INFO "Stopping background monitoring (PID: $monitor_pid)..."
        kill "$monitor_pid"
        rm -f "$pid_file"
        log SUCCESS "Background monitoring stopped"
    else
        log WARN "Background monitoring process not running"
        rm -f "$pid_file"
    fi
}

# Show monitoring status
show_monitoring_status() {
    local pid_file="${MONITOR_RESULTS_DIR}/monitor.pid"

    if [ -f "$pid_file" ] && kill -0 "$(cat "$pid_file")" 2>/dev/null; then
        local monitor_pid=$(cat "$pid_file")
        log INFO "Background monitoring is running (PID: $monitor_pid)"

        # Show recent metrics if available
        local latest_metrics=$(find "$MONITOR_RESULTS_DIR" -name "metrics_*.csv" -type f -printf '%T@ %p\n' | sort -n | tail -1 | cut -d' ' -f2-)
        if [ -n "$latest_metrics" ] && [ -f "$latest_metrics" ]; then
            log INFO "Latest metrics from: $(basename "$latest_metrics")"
            local latest_line=$(tail -1 "$latest_metrics")
            local timestamp=$(echo "$latest_line" | cut -d',' -f1)
            local cpu=$(echo "$latest_line" | cut -d',' -f3)
            local memory=$(echo "$latest_line" | cut -d',' -f4)
            local connections=$(echo "$latest_line" | cut -d',' -f9)
            log METRIC "Current: CPU ${cpu}%, Memory ${memory}%, Connections ${connections}"
        fi
    else
        log INFO "No background monitoring is currently running"
        rm -f "$pid_file" 2>/dev/null || true
    fi

    # Show recent monitoring sessions
    local recent_reports=$(find "$MONITOR_RESULTS_DIR" -name "monitoring_report_*.md" -type f -printf '%T@ %p\n' | sort -n | tail -3)
    if [ -n "$recent_reports" ]; then
        log INFO "Recent monitoring sessions:"
        echo "$recent_reports" | while read -r timestamp_path; do
            local report_file=$(echo "$timestamp_path" | cut -d' ' -f2-)
            local report_name=$(basename "$report_file" .md)
            local session_time=$(echo "$report_name" | grep -o '[0-9]*_[0-9]*$')
            log INFO "  - Session $session_time: $(basename "$report_file")"
        done
    fi
}

# Cleanup function
cleanup() {
    if [ "$MONITORING_ACTIVE" = true ]; then
        log INFO "Stopping monitoring due to signal..."
        MONITORING_ACTIVE=false
    fi
}

# Set up signal handlers
trap cleanup INT TERM

# Check dependencies
check_dependencies() {
    local missing_deps=()

    if ! command -v bc >/dev/null 2>&1; then
        missing_deps+=("bc")
    fi

    if [ ${#missing_deps[@]} -gt 0 ]; then
        log ERROR "Missing required dependencies: ${missing_deps[*]}"
        log ERROR "Please install: sudo apt-get install ${missing_deps[*]}"
        exit 1
    fi
}

# Main execution
main() {
    log INFO "Performance Monitoring Script - PHASE 8 TASK 8.2"

    # Check dependencies
    check_dependencies

    # Parse arguments and execute appropriate action
    if [ $# -eq 0 ]; then
        # Default action: start monitoring
        main_monitor
    else
        parse_args "$@"
    fi
}

# Execute main function with all arguments
main "$@"
