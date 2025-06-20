# Monitoring & Logging 📊

This document provides comprehensive guidance for monitoring Lair Chat systems, analyzing logs, and setting up alerting mechanisms.

## 📋 Table of Contents

- [System Monitoring Overview](#system-monitoring-overview)
- [Real-time Metrics Dashboard](#real-time-metrics-dashboard)
- [Log Management](#log-management)
- [Performance Monitoring](#performance-monitoring)
- [Security Monitoring](#security-monitoring)
- [Alerting and Notifications](#alerting-and-notifications)
- [Health Checks](#health-checks)
- [Capacity Planning](#capacity-planning)
- [Troubleshooting with Logs](#troubleshooting-with-logs)

## 🖥️ System Monitoring Overview

### Monitoring Architecture
```
┌─────────────────────────────────────────────────────────────┐
│                    Monitoring Stack                        │
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │  Grafana    │  │ Prometheus  │  │     AlertManager    │ │
│  │ (Dashboard) │  │ (Metrics)   │  │   (Notifications)   │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
│         │                │                      │           │
│         └────────────────┼──────────────────────┘           │
│                          │                                  │
└──────────────────────────┼──────────────────────────────────┘
                           │
┌──────────────────────────┼──────────────────────────────────┐
│                   Lair Chat Server                         │
│                          │                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Metrics   │  │    Logs     │  │      Health         │ │
│  │ Exporter    │  │  Collector  │  │     Checks          │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Key Monitoring Components
- **Metrics Collection**: Prometheus-compatible metrics endpoint
- **Log Aggregation**: Structured logging with JSON format
- **Health Monitoring**: Built-in health check endpoints
- **Performance Tracking**: Request/response time monitoring
- **Resource Monitoring**: CPU, memory, disk, and network usage

## 📈 Real-time Metrics Dashboard

### Server Metrics Endpoint
```bash
# Access metrics endpoint
curl http://localhost:9090/metrics

# Sample metrics output
# HELP lair_chat_active_connections Number of active WebSocket connections
# TYPE lair_chat_active_connections gauge
lair_chat_active_connections 42

# HELP lair_chat_messages_total Total number of messages processed
# TYPE lair_chat_messages_total counter
lair_chat_messages_total{type="text"} 1234567
lair_chat_messages_total{type="file"} 890
```

### Essential Metrics to Monitor

#### Application Metrics
```prometheus
# Connection metrics
lair_chat_active_connections
lair_chat_connection_duration_seconds
lair_chat_connections_total

# Message metrics
lair_chat_messages_total{type, room}
lair_chat_message_size_bytes{type}
lair_chat_message_processing_duration_seconds

# User metrics
lair_chat_active_users
lair_chat_user_sessions_total
lair_chat_authentication_attempts{result}

# Room metrics
lair_chat_active_rooms
lair_chat_room_participants{room}
lair_chat_room_messages_total{room}
```

#### System Metrics
```prometheus
# Resource utilization
process_cpu_seconds_total
process_resident_memory_bytes
process_open_fds
process_max_fds

# HTTP metrics
http_requests_total{method, path, status}
http_request_duration_seconds{method, path}
http_requests_in_flight

# Database metrics
database_connections_active
database_connections_idle
database_query_duration_seconds{query}
```

### Grafana Dashboard Configuration

#### Dashboard JSON Template
```json
{
  "dashboard": {
    "title": "Lair Chat System Overview",
    "panels": [
      {
        "title": "Active Connections",
        "type": "stat",
        "targets": [
          {
            "expr": "lair_chat_active_connections",
            "legendFormat": "Connections"
          }
        ]
      },
      {
        "title": "Message Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(lair_chat_messages_total[5m])",
            "legendFormat": "Messages/sec"
          }
        ]
      },
      {
        "title": "Response Time",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "95th percentile"
          }
        ]
      }
    ]
  }
}
```

## 📝 Log Management

### Log Configuration
```toml
# config/logging.toml
[logging]
level = "info"
format = "json"
output = "file"
file_path = "/var/log/lair-chat/server.log"
max_file_size = "100MB"
max_files = 10
compress = true

[logging.filters]
security = "warn"
performance = "info"
user_actions = "info"
system_events = "debug"
```

### Log Categories and Levels

#### Application Logs
```json
// Authentication events
{
  "timestamp": "2024-12-21T10:30:00Z",
  "level": "INFO",
  "category": "auth",
  "event": "user_login",
  "user_id": "john.doe",
  "ip_address": "192.168.1.100",
  "user_agent": "LairChat/1.0"
}

// Message events
{
  "timestamp": "2024-12-21T10:31:15Z",
  "level": "INFO",
  "category": "message",
  "event": "message_sent",
  "user_id": "john.doe",
  "room_id": "general",
  "message_id": "msg_12345",
  "message_size": 156
}

// Error events
{
  "timestamp": "2024-12-21T10:32:00Z",
  "level": "ERROR",
  "category": "system",
  "event": "database_error",
  "error": "Connection timeout",
  "query": "SELECT * FROM messages WHERE room_id = ?",
  "duration_ms": 5000
}
```

#### Security Logs
```json
// Failed login attempt
{
  "timestamp": "2024-12-21T10:33:00Z",
  "level": "WARN",
  "category": "security",
  "event": "failed_login",
  "username": "admin",
  "ip_address": "203.0.113.50",
  "attempt_count": 3,
  "reason": "invalid_password"
}

// Privilege escalation attempt
{
  "timestamp": "2024-12-21T10:34:00Z",
  "level": "WARN",
  "category": "security",
  "event": "privilege_escalation",
  "user_id": "john.doe",
  "requested_role": "admin",
  "current_role": "user",
  "denied": true
}
```

### Log Analysis Tools

#### Command Line Analysis
```bash
# Real-time log monitoring
tail -f /var/log/lair-chat/server.log | jq '.'

# Filter by log level
cat server.log | jq 'select(.level == "ERROR")'

# Count events by category
cat server.log | jq -r '.category' | sort | uniq -c

# Find authentication failures
cat server.log | jq 'select(.category == "security" and .event == "failed_login")'

# Performance analysis
cat server.log | jq 'select(.category == "performance" and .duration_ms > 1000)'
```

#### Log Aggregation with ELK Stack
```yaml
# logstash.conf
input {
  file {
    path => "/var/log/lair-chat/*.log"
    codec => "json"
  }
}

filter {
  if [category] == "security" {
    mutate {
      add_tag => ["security_event"]
    }
  }
  
  if [level] == "ERROR" {
    mutate {
      add_tag => ["error_event"]
    }
  }
}

output {
  elasticsearch {
    hosts => ["localhost:9200"]
    index => "lair-chat-logs-%{+YYYY.MM.dd}"
  }
}
```

## ⚡ Performance Monitoring

### Response Time Monitoring
```bash
# Monitor API response times
lair-chat-admin monitor api-performance --endpoint /api/v1/messages --duration 60s

# Sample output:
# Endpoint: /api/v1/messages
# Average response time: 45ms
# 95th percentile: 120ms
# 99th percentile: 250ms
# Error rate: 0.5%
```

### Resource Usage Monitoring
```bash
# System resource monitoring
lair-chat-admin monitor system-resources

# CPU Usage: 25.3%
# Memory Usage: 1.2GB / 4GB (30%)
# Disk Usage: 45GB / 100GB (45%)
# Network I/O: 2.5MB/s in, 3.1MB/s out
```

### Database Performance
```sql
-- Monitor slow queries
SELECT 
    query, 
    avg_duration_ms, 
    call_count, 
    total_duration_ms
FROM performance_metrics 
WHERE avg_duration_ms > 100 
ORDER BY total_duration_ms DESC;

-- Monitor connection pool
SELECT 
    active_connections,
    idle_connections,
    max_connections,
    wait_count
FROM connection_pool_metrics;
```

## 🛡️ Security Monitoring

### Security Event Dashboard
```bash
# Security event summary
lair-chat-admin security dashboard

# Sample output:
# ┌─────────────────────────────────────┐
# │        Security Dashboard           │
# ├─────────────────────────────────────┤
# │ Failed Logins (24h):           15   │
# │ Blocked IPs:                    3   │
# │ Admin Access Attempts:          2   │
# │ Suspicious Activity Alerts:     0   │
# │ Certificate Expiry:        45 days   │
# └─────────────────────────────────────┘
```

### Intrusion Detection Rules
```yaml
# security_rules.yaml
rules:
  - name: "Brute Force Detection"
    condition: "failed_login_count > 5 within 300s from same IP"
    action: "block_ip"
    duration: "1h"
    
  - name: "Admin Access from New Location"
    condition: "admin_login from new_country"
    action: "alert"
    notification: "security_team"
    
  - name: "Mass Message Deletion"
    condition: "messages_deleted > 50 within 60s by same user"
    action: "suspend_user"
    duration: "1h"
```

### Audit Log Analysis
```bash
# Generate security audit report
lair-chat-admin audit security-report \
  --start-date "2024-12-01" \
  --end-date "2024-12-21" \
  --format pdf \
  --output security_audit_december.pdf

# Check for compliance violations
lair-chat-admin audit compliance-check \
  --standard "SOC2" \
  --generate-report
```

## 🚨 Alerting and Notifications

### Alert Configuration
```toml
# config/alerts.toml
[alerts]
enabled = true

[alerts.thresholds]
high_cpu = 80.0
high_memory = 85.0
high_disk = 90.0
response_time_ms = 1000
error_rate_percent = 5.0

[alerts.channels]
email = ["admin@company.com", "ops@company.com"]
slack_webhook = "https://hooks.slack.com/services/..."
pagerduty_key = "your-pagerduty-integration-key"
```

### Alert Rules Examples
```yaml
# alert_rules.yaml
groups:
  - name: "lair_chat_alerts"
    rules:
      - alert: "HighCPUUsage"
        expr: "process_cpu_seconds_total > 0.8"
        for: "5m"
        labels:
          severity: "warning"
        annotations:
          summary: "High CPU usage detected"
          
      - alert: "DatabaseConnectionFailure"
        expr: "database_connections_failed_total > 0"
        for: "1m"
        labels:
          severity: "critical"
        annotations:
          summary: "Database connection failures detected"
          
      - alert: "HighErrorRate"
        expr: "rate(http_requests_total{status=~'5..'}[5m]) > 0.05"
        for: "2m"
        labels:
          severity: "critical"
        annotations:
          summary: "High error rate detected"
```

### Notification Templates
```json
{
  "slack_template": {
    "channel": "#ops-alerts",
    "username": "Lair Chat Monitor",
    "icon_emoji": ":warning:",
    "attachments": [
      {
        "color": "danger",
        "title": "{{.Alert.Summary}}",
        "text": "{{.Alert.Description}}",
        "fields": [
          {
            "title": "Severity",
            "value": "{{.Alert.Severity}}",
            "short": true
          },
          {
            "title": "Time",
            "value": "{{.Alert.Timestamp}}",
            "short": true
          }
        ]
      }
    ]
  }
}
```

## 🏥 Health Checks

### Built-in Health Endpoints
```bash
# Basic health check
curl http://localhost:8080/health
# Response: {"status": "healthy", "timestamp": "2024-12-21T10:30:00Z"}

# Detailed health check
curl http://localhost:8080/health/detailed
# Response includes database connectivity, memory usage, etc.

# Readiness check (for load balancers)
curl http://localhost:8080/ready
# Response: {"ready": true, "services": ["database", "redis", "filesystem"]}
```

### Custom Health Checks
```rust
// Example health check implementation
pub struct HealthChecker {
    database: DatabasePool,
    redis: RedisPool,
}

impl HealthChecker {
    pub async fn check_health(&self) -> HealthStatus {
        let mut status = HealthStatus::new();
        
        // Database connectivity
        match self.database.get().await {
            Ok(_) => status.add_check("database", true, "Connected"),
            Err(e) => status.add_check("database", false, &e.to_string()),
        }
        
        // Redis connectivity
        match self.redis.get().await {
            Ok(_) => status.add_check("redis", true, "Connected"),
            Err(e) => status.add_check("redis", false, &e.to_string()),
        }
        
        // Disk space
        let disk_usage = get_disk_usage("/var/lib/lair-chat").await;
        let disk_healthy = disk_usage < 0.9;
        status.add_check("disk_space", disk_healthy, &format!("Usage: {:.1}%", disk_usage * 100.0));
        
        status
    }
}
```

### Health Check Monitoring
```bash
# Automated health monitoring script
#!/bin/bash
HEALTH_URL="http://localhost:8080/health"
ALERT_EMAIL="admin@company.com"

while true; do
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" $HEALTH_URL)
    
    if [ $HTTP_CODE -ne 200 ]; then
        echo "Health check failed with code $HTTP_CODE" | mail -s "Lair Chat Health Alert" $ALERT_EMAIL
    fi
    
    sleep 30
done
```

## 📊 Capacity Planning

### Resource Usage Trends
```bash
# Generate capacity planning report
lair-chat-admin capacity-report \
  --period "3months" \
  --include-projections \
  --output capacity_report.pdf
```

### Growth Projections
```
Current Usage Trends (90 days):
├── Users: +15% monthly growth
├── Messages: +22% monthly growth  
├── Storage: +18% monthly growth
├── Bandwidth: +20% monthly growth

Projected Requirements (6 months):
├── CPU: Current + 45% (recommend 2x current)
├── RAM: Current + 40% (recommend 1.5x current)
├── Storage: Current + 65% (recommend 2x current)
├── Bandwidth: Current + 55% (recommend 2x current)
```

### Scaling Recommendations
```yaml
# scaling_recommendations.yaml
current_resources:
  cpu_cores: 4
  memory_gb: 16
  storage_gb: 500
  max_connections: 1000

recommended_scaling:
  6_months:
    cpu_cores: 8
    memory_gb: 24
    storage_gb: 1000
    max_connections: 2000
    
  12_months:
    cpu_cores: 16
    memory_gb: 48
    storage_gb: 2000
    max_connections: 5000
```

## 🔍 Troubleshooting with Logs

### Common Issues and Log Patterns

#### High Memory Usage
```bash
# Find memory-related errors
cat server.log | jq 'select(.category == "system" and (.event | contains("memory")))'

# Monitor memory allocation patterns
cat server.log | jq 'select(.memory_mb) | {timestamp, memory_mb, active_connections}'
```

#### Database Connection Issues
```bash
# Find database connection errors
cat server.log | jq 'select(.category == "database" and .level == "ERROR")'

# Monitor connection pool status
cat server.log | jq 'select(.event == "connection_pool_status") | {timestamp, active, idle, waiting}'
```

#### Performance Degradation
```bash
# Find slow requests
cat server.log | jq 'select(.duration_ms and (.duration_ms | tonumber) > 1000)'

# Analyze response time trends
cat server.log | jq 'select(.category == "http") | {timestamp, path, duration_ms}' | 
  awk '{sum+=$3; count++} END {print "Average response time:", sum/count, "ms"}'
```

### Log-based Diagnostics
```bash
# System diagnostic script
#!/bin/bash
LOG_FILE="/var/log/lair-chat/server.log"
LAST_HOUR=$(date -d '1 hour ago' --iso-8601=seconds)

echo "=== Lair Chat System Diagnostics ==="
echo "Time range: Last 1 hour"
echo "Log file: $LOG_FILE"
echo

# Error summary
echo "Errors in last hour:"
cat $LOG_FILE | jq -r "select(.timestamp > \"$LAST_HOUR\" and .level == \"ERROR\") | .event" | sort | uniq -c

# Performance issues
echo -e "\nSlow requests (>1s) in last hour:"
cat $LOG_FILE | jq -r "select(.timestamp > \"$LAST_HOUR\" and .duration_ms and (.duration_ms | tonumber) > 1000) | \"\(.path): \(.duration_ms)ms\""

# Security events
echo -e "\nSecurity events in last hour:"
cat $LOG_FILE | jq -r "select(.timestamp > \"$LAST_HOUR\" and .category == \"security\") | \"\(.event): \(.user_id // .ip_address)\""
```

## 📋 Monitoring Checklists

### Daily Monitoring Tasks
- [ ] Check system health dashboard
- [ ] Review error logs from last 24 hours
- [ ] Verify backup completion status
- [ ] Monitor resource usage trends
- [ ] Check security alerts and incidents
- [ ] Validate SSL certificate status

### Weekly Monitoring Tasks
- [ ] Generate performance trend report
- [ ] Review capacity planning metrics
- [ ] Analyze user activity patterns
- [ ] Check for software updates
- [ ] Review and update alert thresholds
- [ ] Test disaster recovery procedures

### Monthly Monitoring Tasks
- [ ] Comprehensive security audit
- [ ] Database maintenance and optimization
- [ ] Log rotation and archival
- [ ] Capacity planning review
- [ ] Performance baseline update
- [ ] Documentation updates

---

**Last Updated**: December 2024  
**Document Version**: 1.0  
**Reviewed by**: System Administration Team