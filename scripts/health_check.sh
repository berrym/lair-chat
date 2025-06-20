#!/bin/bash
# Server health monitoring script

API_BASE="http://127.0.0.1:8082/api/v1"
LOG_FILE="logs/health_check.log"

check_endpoint() {
    local endpoint="$1"
    local name="$2"

    status_code=$(curl -s -o /dev/null -w "%{http_code}" "$API_BASE$endpoint" 2>/dev/null)

    if [ "$status_code" = "200" ]; then
        echo "$(date): $name - OK ($status_code)" >> "$LOG_FILE"
        return 0
    else
        echo "$(date): $name - FAILED ($status_code)" >> "$LOG_FILE"
        return 1
    fi
}

echo "$(date): Starting health check..." >> "$LOG_FILE"

# Check main endpoints
check_endpoint "/health" "Health Check"
check_endpoint "/docs" "API Documentation"

echo "$(date): Health check completed" >> "$LOG_FILE"
