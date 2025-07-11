//! Performance monitoring framework for the TCP server
//!
//! This module provides comprehensive performance monitoring with metrics collection,
//! alerting, and performance optimization patterns.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::server::error::TcpError;
use crate::server::storage::current_timestamp;

/// Performance metrics collector
pub struct PerformanceMonitor {
    /// Metrics storage
    metrics: Arc<RwLock<MetricsStorage>>,
    /// Alerting system
    alerting: Arc<RwLock<AlertingSystem>>,
    /// Performance thresholds
    thresholds: Arc<RwLock<PerformanceThresholds>>,
    /// Security metrics
    security_metrics: Arc<RwLock<SecurityMetrics>>,
}

/// Metrics storage
#[derive(Debug, Clone, Default)]
pub struct MetricsStorage {
    /// Operation metrics
    pub operations: HashMap<String, OperationMetrics>,
    /// Error metrics
    pub errors: HashMap<String, ErrorMetrics>,
    /// System metrics
    pub system: SystemMetrics,
    /// Connection metrics
    pub connections: ConnectionMetrics,
}

/// Operation performance metrics
#[derive(Debug, Clone, Default)]
pub struct OperationMetrics {
    /// Total number of operations
    pub total_count: u64,
    /// Total duration of all operations
    pub total_duration: Duration,
    /// Average duration
    pub average_duration: Duration,
    /// Minimum duration
    pub min_duration: Duration,
    /// Maximum duration
    pub max_duration: Duration,
    /// Recent operation times (for moving average)
    pub recent_durations: Vec<Duration>,
    /// Last operation timestamp
    pub last_operation: Option<i64>,
}

/// Error metrics
#[derive(Debug, Clone, Default)]
pub struct ErrorMetrics {
    /// Total error count
    pub total_count: u64,
    /// Error count by type
    pub error_types: HashMap<String, u64>,
    /// Error rate (errors per minute)
    pub error_rate: f64,
    /// Last error timestamp
    pub last_error: Option<i64>,
}

/// System performance metrics
#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    /// Server uptime
    pub uptime: Duration,
    /// Start time
    pub start_time: Instant,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Number of active connections
    pub active_connections: u32,
    /// Number of active rooms
    pub active_rooms: u32,
    /// Number of active users
    pub active_users: u32,
}

/// Connection metrics
#[derive(Debug, Clone, Default)]
pub struct ConnectionMetrics {
    /// Total connections established
    pub total_connections: u64,
    /// Active connections
    pub active_connections: u32,
    /// Connection duration statistics
    pub connection_durations: Vec<Duration>,
    /// Average connection duration
    pub average_duration: Duration,
    /// Connection failures
    pub connection_failures: u64,
}

/// Security-related metrics
#[derive(Debug, Clone, Default)]
pub struct SecurityMetrics {
    pub failed_logins: u64,
    pub blocked_ips: u64,
    pub suspicious_activities: u64,
    pub automated_blocks: u64,
    pub security_events: HashMap<String, u64>,
    pub threat_levels: HashMap<String, u64>,
    pub last_security_event: Option<u64>,
}

/// Alerting system
#[derive(Debug, Clone, Default)]
pub struct AlertingSystem {
    /// Active alerts
    pub active_alerts: Vec<Alert>,
    /// Alert history
    pub alert_history: Vec<Alert>,
    /// Alert configuration
    pub alert_config: AlertConfig,
}

/// Performance alert
#[derive(Debug, Clone)]
pub struct Alert {
    /// Alert type
    pub alert_type: AlertType,
    /// Alert level
    pub level: AlertLevel,
    /// Alert message
    pub message: String,
    /// Timestamp when alert was raised
    pub timestamp: i64,
    /// Whether alert is still active
    pub active: bool,
}

/// Alert types
#[derive(Debug, Clone)]
pub enum AlertType {
    HighLatency,
    HighErrorRate,
    HighMemoryUsage,
    HighConnectionCount,
    SystemOverload,
    DatabaseIssue,
    SecurityThreat,
    AutomatedBlock,
    SuspiciousActivity,
}

/// Alert levels
#[derive(Debug, Clone)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Alert configuration
#[derive(Debug, Clone)]
pub struct AlertConfig {
    /// Maximum response time before alert
    pub max_response_time: Duration,
    /// Maximum error rate before alert
    pub max_error_rate: f64,
    /// Maximum memory usage before alert
    pub max_memory_usage: u64,
    /// Maximum connection count before alert
    pub max_connections: u32,
}

/// Performance thresholds
#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    /// Response time thresholds
    pub response_times: HashMap<String, Duration>,
    /// Error rate thresholds
    pub error_rates: HashMap<String, f64>,
    /// System resource thresholds
    pub system_thresholds: SystemThresholds,
}

/// System resource thresholds
#[derive(Debug, Clone)]
pub struct SystemThresholds {
    /// CPU usage threshold
    pub cpu_threshold: f64,
    /// Memory usage threshold
    pub memory_threshold: u64,
    /// Connection count threshold
    pub connection_threshold: u32,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(MetricsStorage::default())),
            alerting: Arc::new(RwLock::new(AlertingSystem::default())),
            thresholds: Arc::new(RwLock::new(PerformanceThresholds::default())),
            security_metrics: Arc::new(RwLock::new(SecurityMetrics::default())),
        }
    }

    /// Record operation performance
    pub async fn record_operation(&self, operation: &str, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        let op_metrics = metrics.operations.entry(operation.to_string()).or_default();

        op_metrics.total_count += 1;
        op_metrics.total_duration += duration;
        op_metrics.average_duration = op_metrics.total_duration / op_metrics.total_count as u32;
        op_metrics.last_operation = Some(current_timestamp());

        // Update min/max durations
        if op_metrics.min_duration == Duration::ZERO || duration < op_metrics.min_duration {
            op_metrics.min_duration = duration;
        }
        if duration > op_metrics.max_duration {
            op_metrics.max_duration = duration;
        }

        // Maintain recent durations for moving average (keep last 100)
        op_metrics.recent_durations.push(duration);
        if op_metrics.recent_durations.len() > 100 {
            op_metrics.recent_durations.remove(0);
        }

        debug!(
            operation = operation,
            duration_ms = duration.as_millis(),
            total_count = op_metrics.total_count,
            average_ms = op_metrics.average_duration.as_millis(),
            "Operation performance recorded"
        );
    }

    /// Record error occurrence
    pub async fn record_error(&self, error: &TcpError) {
        let mut metrics = self.metrics.write().await;
        let error_metrics = metrics.errors.entry("all".to_string()).or_default();

        error_metrics.total_count += 1;
        error_metrics.last_error = Some(current_timestamp());

        let error_type = format!("{:?}", error);
        *error_metrics.error_types.entry(error_type).or_insert(0) += 1;

        // Calculate error rate (errors per minute)
        // This is a simplified calculation - in production, you'd want a proper sliding window
        error_metrics.error_rate = error_metrics.total_count as f64 / 60.0; // Simplified

        debug!(
            error_type = format!("{:?}", error),
            total_errors = error_metrics.total_count,
            error_rate = error_metrics.error_rate,
            "Error recorded"
        );
    }

    /// Record operation error with string message (helper for server integration)
    pub async fn record_operation_error(&self, operation: &str, error_message: String) {
        let mut metrics = self.metrics.write().await;
        let error_metrics = metrics.errors.entry(operation.to_string()).or_default();

        error_metrics.total_count += 1;
        error_metrics.last_error = Some(current_timestamp());

        let error_type = format!("{}Error", operation);
        *error_metrics
            .error_types
            .entry(error_type.clone())
            .or_insert(0) += 1;

        // Calculate error rate (errors per minute)
        error_metrics.error_rate = error_metrics.total_count as f64 / 60.0;

        debug!(
            operation = operation,
            error_message = error_message,
            error_type = error_type,
            total_errors = error_metrics.total_count,
            error_rate = error_metrics.error_rate,
            "Operation error recorded"
        );
    }

    /// Record validation metrics
    pub async fn record_validation(
        &self,
        validation_result: &Result<(), crate::server::error::ValidationError>,
    ) {
        let operation = "validation";
        let start_time = Instant::now();

        match validation_result {
            Ok(_) => {
                self.record_operation(operation, start_time.elapsed()).await;
            }
            Err(err) => {
                let tcp_error = TcpError::ValidationError(err.clone());
                self.record_error(&tcp_error).await;
            }
        }
    }

    /// Record security event
    pub async fn record_security_event(&self, event_type: &str, description: &str) {
        let mut security_metrics = self.security_metrics.write().await;

        // Update security event counters
        *security_metrics
            .security_events
            .entry(event_type.to_string())
            .or_insert(0) += 1;

        match event_type {
            "failed_login" => security_metrics.failed_logins += 1,
            "ip_blocked" => security_metrics.blocked_ips += 1,
            "suspicious_activity" => security_metrics.suspicious_activities += 1,
            "automated_block" => security_metrics.automated_blocks += 1,
            _ => {}
        }

        security_metrics.last_security_event = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );

        // Check if we should create security alerts
        self.check_security_thresholds(&security_metrics).await;
    }

    /// Check security thresholds and create alerts
    async fn check_security_thresholds(&self, security_metrics: &SecurityMetrics) {
        let mut alerting = self.alerting.write().await;

        // Alert on high number of failed logins
        if security_metrics.failed_logins > 10 {
            let alert = Alert {
                alert_type: AlertType::SecurityThreat,
                level: AlertLevel::Warning,
                message: format!(
                    "High number of failed logins: {}",
                    security_metrics.failed_logins
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                active: true,
            };
            alerting.active_alerts.push(alert);
        }

        // Alert on automated blocks
        if security_metrics.automated_blocks > 5 {
            let alert = Alert {
                alert_type: AlertType::AutomatedBlock,
                level: AlertLevel::Critical,
                message: format!(
                    "Multiple automated blocks triggered: {}",
                    security_metrics.automated_blocks
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                active: true,
            };
            alerting.active_alerts.push(alert);
        }
    }

    /// Get security metrics
    pub async fn get_security_metrics(&self) -> SecurityMetrics {
        let security_metrics = self.security_metrics.read().await;
        security_metrics.clone()
    }

    /// Update system metrics
    pub async fn update_system_metrics(&self, connections: u32, rooms: u32, users: u32) {
        let mut metrics = self.metrics.write().await;
        let system_metrics = &mut metrics.system;

        system_metrics.uptime = system_metrics.start_time.elapsed();
        system_metrics.active_connections = connections;
        system_metrics.active_rooms = rooms;
        system_metrics.active_users = users;

        // In a real implementation, you'd gather actual CPU and memory metrics
        // This is a placeholder
        system_metrics.cpu_usage = 0.0;
        system_metrics.memory_usage = 0;
    }

    /// Check performance thresholds and raise alerts
    pub async fn check_thresholds(&self) -> Vec<Alert> {
        let metrics = self.metrics.read().await;
        let thresholds = self.thresholds.read().await;
        let mut alerts = Vec::new();

        // Check response time thresholds
        for (operation, op_metrics) in &metrics.operations {
            if let Some(threshold) = thresholds.response_times.get(operation) {
                if op_metrics.average_duration > *threshold {
                    alerts.push(Alert {
                        alert_type: AlertType::HighLatency,
                        level: AlertLevel::Warning,
                        message: format!(
                            "High latency detected for operation {}: {}ms (threshold: {}ms)",
                            operation,
                            op_metrics.average_duration.as_millis(),
                            threshold.as_millis()
                        ),
                        timestamp: current_timestamp(),
                        active: true,
                    });
                }
            }
        }

        // Check error rate thresholds
        for (error_type, error_metrics) in &metrics.errors {
            if error_metrics.error_rate > 10.0 {
                // Threshold of 10 errors per minute
                alerts.push(Alert {
                    alert_type: AlertType::HighErrorRate,
                    level: AlertLevel::Critical,
                    message: format!(
                        "High error rate detected for {}: {:.2} errors/min",
                        error_type, error_metrics.error_rate
                    ),
                    timestamp: current_timestamp(),
                    active: true,
                });
            }
        }

        // Check system thresholds
        if metrics.system.active_connections > thresholds.system_thresholds.connection_threshold {
            alerts.push(Alert {
                alert_type: AlertType::HighConnectionCount,
                level: AlertLevel::Warning,
                message: format!(
                    "High connection count: {} (threshold: {})",
                    metrics.system.active_connections,
                    thresholds.system_thresholds.connection_threshold
                ),
                timestamp: current_timestamp(),
                active: true,
            });
        }

        // Update alerting system
        if !alerts.is_empty() {
            let mut alerting = self.alerting.write().await;
            for alert in &alerts {
                alerting.active_alerts.push(alert.clone());
                alerting.alert_history.push(alert.clone());

                match alert.level {
                    AlertLevel::Critical | AlertLevel::Emergency => {
                        warn!("Performance alert: {}", alert.message);
                    }
                    AlertLevel::Warning => {
                        info!("Performance alert: {}", alert.message);
                    }
                    AlertLevel::Info => {
                        debug!("Performance alert: {}", alert.message);
                    }
                }
            }
        }

        alerts
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> MetricsStorage {
        self.metrics.read().await.clone()
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let alerting = self.alerting.read().await;
        alerting.active_alerts.clone()
    }

    /// Clear active alerts
    pub async fn clear_alerts(&self) {
        let mut alerting = self.alerting.write().await;
        alerting.active_alerts.clear();
    }

    /// Set performance threshold
    pub async fn set_threshold(&self, operation: &str, threshold: Duration) {
        let mut thresholds = self.thresholds.write().await;
        thresholds
            .response_times
            .insert(operation.to_string(), threshold);
    }

    /// Get performance report
    pub async fn get_performance_report(&self) -> String {
        let metrics = self.get_metrics().await;
        let security_metrics = self.get_security_metrics().await;
        let alerts = self.get_active_alerts().await;

        let mut report = String::new();
        report.push_str("=== Performance Report ===\n\n");

        // System metrics
        report.push_str(&format!("System Uptime: {:?}\n", metrics.system.uptime));
        report.push_str(&format!(
            "Active Connections: {}\n",
            metrics.system.active_connections
        ));
        report.push_str(&format!("Active Rooms: {}\n", metrics.system.active_rooms));
        report.push_str(&format!("Active Users: {}\n", metrics.system.active_users));
        report.push_str("\n");

        // Security metrics
        report.push_str("Security Summary:\n");
        report.push_str(&format!(
            "  Failed Logins: {}\n",
            security_metrics.failed_logins
        ));
        report.push_str(&format!(
            "  Blocked IPs: {}\n",
            security_metrics.blocked_ips
        ));
        report.push_str(&format!(
            "  Suspicious Activities: {}\n",
            security_metrics.suspicious_activities
        ));
        report.push_str(&format!(
            "  Automated Blocks: {}\n",
            security_metrics.automated_blocks
        ));

        if !security_metrics.security_events.is_empty() {
            report.push_str("  Security Events:\n");
            for (event_type, count) in &security_metrics.security_events {
                report.push_str(&format!("    {}: {}\n", event_type, count));
            }
        }
        report.push_str("\n");

        // Operation metrics
        report.push_str("Operation Performance:\n");
        for (operation, op_metrics) in &metrics.operations {
            report.push_str(&format!(
                "  {}: {} calls, avg {}ms, min {}ms, max {}ms\n",
                operation,
                op_metrics.total_count,
                op_metrics.average_duration.as_millis(),
                op_metrics.min_duration.as_millis(),
                op_metrics.max_duration.as_millis()
            ));
        }
        report.push_str("\n");

        // Error metrics
        report.push_str("Error Summary:\n");
        for (error_type, error_metrics) in &metrics.errors {
            report.push_str(&format!(
                "  {}: {} errors, rate: {:.2}/min\n",
                error_type, error_metrics.total_count, error_metrics.error_rate
            ));
        }
        report.push_str("\n");

        // Active alerts
        if !alerts.is_empty() {
            report.push_str("Active Alerts:\n");
            for alert in &alerts {
                report.push_str(&format!("  [{:?}] {}\n", alert.level, alert.message));
            }
        } else {
            report.push_str("No active alerts.\n");
        }

        report
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        let mut response_times = HashMap::new();
        response_times.insert("LOGIN".to_string(), Duration::from_millis(100));
        response_times.insert("REGISTER".to_string(), Duration::from_millis(200));
        response_times.insert("MESSAGE".to_string(), Duration::from_millis(50));
        response_times.insert("CREATE_ROOM".to_string(), Duration::from_millis(150));
        response_times.insert("JOIN_ROOM".to_string(), Duration::from_millis(100));
        response_times.insert("INVITE_USER".to_string(), Duration::from_millis(100));

        Self {
            response_times,
            error_rates: HashMap::new(),
            system_thresholds: SystemThresholds {
                cpu_threshold: 80.0,
                memory_threshold: 1024 * 1024 * 1024, // 1GB
                connection_threshold: 1000,
            },
        }
    }
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            max_response_time: Duration::from_millis(500),
            max_error_rate: 5.0,
            max_memory_usage: 1024 * 1024 * 1024, // 1GB
            max_connections: 1000,
        }
    }
}

impl SystemMetrics {
    /// Create new system metrics with current timestamp
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            ..Default::default()
        }
    }
}

/// Global performance monitor instance
static PERFORMANCE_MONITOR: std::sync::OnceLock<PerformanceMonitor> = std::sync::OnceLock::new();

/// Get the global performance monitor
pub fn get_performance_monitor() -> &'static PerformanceMonitor {
    PERFORMANCE_MONITOR.get_or_init(|| PerformanceMonitor::new())
}

/// Initialize the global performance monitor
pub fn init_performance_monitor() -> &'static PerformanceMonitor {
    get_performance_monitor()
}

/// Convenience macro for recording operation performance
#[macro_export]
macro_rules! record_operation {
    ($operation:expr, $duration:expr) => {
        $crate::server::monitoring::get_performance_monitor()
            .record_operation($operation, $duration)
            .await;
    };
}

/// Convenience macro for recording operation with timing
#[macro_export]
macro_rules! time_operation {
    ($operation:expr, $code:block) => {{
        let start = std::time::Instant::now();
        let result = $code;
        $crate::server::monitoring::get_performance_monitor()
            .record_operation($operation, start.elapsed())
            .await;
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new();
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.operations.len(), 0);
    }

    #[tokio::test]
    async fn test_record_operation() {
        let monitor = PerformanceMonitor::new();
        let duration = Duration::from_millis(100);

        monitor.record_operation("test_operation", duration).await;

        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.operations.len(), 1);
        assert_eq!(metrics.operations["test_operation"].total_count, 1);
        assert_eq!(
            metrics.operations["test_operation"].average_duration,
            duration
        );
    }

    #[tokio::test]
    async fn test_record_error() {
        let monitor = PerformanceMonitor::new();
        let error = TcpError::ValidationError(
            crate::server::error::ValidationError::InvalidFormat("test".to_string()),
        );

        monitor.record_error(&error).await;

        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.errors.len(), 1);
        assert_eq!(metrics.errors["all"].total_count, 1);
    }

    #[tokio::test]
    async fn test_system_metrics() {
        let monitor = PerformanceMonitor::new();

        monitor.update_system_metrics(10, 5, 15).await;

        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.system.active_connections, 10);
        assert_eq!(metrics.system.active_rooms, 5);
        assert_eq!(metrics.system.active_users, 15);
    }

    #[tokio::test]
    async fn test_alerts() {
        let monitor = PerformanceMonitor::new();

        // Set a low threshold
        monitor
            .set_threshold("test_operation", Duration::from_millis(50))
            .await;

        // Record an operation that exceeds the threshold
        monitor
            .record_operation("test_operation", Duration::from_millis(100))
            .await;

        let alerts = monitor.check_thresholds().await;
        assert!(!alerts.is_empty());
        assert!(matches!(alerts[0].alert_type, AlertType::HighLatency));
    }
}
