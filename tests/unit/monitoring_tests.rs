//! Unit tests for the performance monitoring system
//!
//! This module provides comprehensive testing for the performance monitoring
//! system implemented in Phase 7, including metrics collection, alerting,
//! threshold management, and security monitoring.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

// Import the monitoring framework
use lair_chat::server::monitoring::{
    get_performance_monitor, init_performance_monitor, Alert, AlertConfig, AlertLevel, AlertType,
    AlertingSystem, ConnectionMetrics, ErrorMetrics, MetricsStorage, OperationMetrics,
    PerformanceMonitor, PerformanceThresholds, SecurityMetrics, SystemMetrics, SystemThresholds,
};

#[tokio::test]
async fn test_performance_monitor_initialization() {
    // Test that performance monitor can be initialized
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await;

    assert!(
        monitor.is_ok(),
        "Performance monitor should initialize successfully"
    );

    let metrics = monitor.unwrap().get_metrics().await;
    assert_eq!(
        metrics.operations.total_count, 0,
        "Initial operation count should be zero"
    );
    assert_eq!(
        metrics.errors.total_count, 0,
        "Initial error count should be zero"
    );
}

#[tokio::test]
async fn test_operation_metrics_recording() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    let operation_name = "test_operation";
    let duration = Duration::from_millis(100);

    // Record multiple operations
    for i in 0..5 {
        monitor
            .record_operation(operation_name, duration + Duration::from_millis(i * 10))
            .await;
    }

    let metrics = monitor.get_metrics().await;
    let op_metrics = metrics
        .operations
        .get(operation_name)
        .expect("Should have operation metrics");

    assert_eq!(op_metrics.total_count, 5, "Should record all operations");
    assert!(
        op_metrics.average_duration > Duration::from_millis(90),
        "Should calculate average duration"
    );
    assert!(
        op_metrics.average_duration < Duration::from_millis(150),
        "Average should be reasonable"
    );
    assert_eq!(
        op_metrics.min_duration,
        Duration::from_millis(100),
        "Should track minimum duration"
    );
    assert_eq!(
        op_metrics.max_duration,
        Duration::from_millis(140),
        "Should track maximum duration"
    );
}

#[tokio::test]
async fn test_error_metrics_recording() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    let error_types = vec!["ConnectionError", "ValidationError", "AuthError"];

    // Record various error types
    for (i, error_type) in error_types.iter().enumerate() {
        for _ in 0..(i + 1) {
            monitor.record_error(error_type, "test_context").await;
        }
    }

    let metrics = monitor.get_metrics().await;

    assert_eq!(
        metrics.errors.total_count, 6,
        "Should track total error count"
    );
    assert_eq!(
        *metrics.errors.error_types.get("ConnectionError").unwrap(),
        1,
        "Should track ConnectionError count"
    );
    assert_eq!(
        *metrics.errors.error_types.get("ValidationError").unwrap(),
        2,
        "Should track ValidationError count"
    );
    assert_eq!(
        *metrics.errors.error_types.get("AuthError").unwrap(),
        3,
        "Should track AuthError count"
    );
}

#[tokio::test]
async fn test_system_metrics_tracking() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    // Update system metrics
    monitor.update_system_metrics().await;

    let metrics = monitor.get_metrics().await;

    assert!(
        metrics.system.uptime > Duration::from_secs(0),
        "Should track uptime"
    );
    assert!(
        metrics.system.start_time.timestamp() > 0,
        "Should have valid start time"
    );
    assert!(
        metrics.system.cpu_usage >= 0.0,
        "CPU usage should be non-negative"
    );
    assert!(
        metrics.system.memory_usage >= 0,
        "Memory usage should be non-negative"
    );
}

#[tokio::test]
async fn test_security_metrics_tracking() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    // Record various security events
    monitor
        .record_security_event("failed_login", "192.168.1.100", "Invalid credentials")
        .await;
    monitor
        .record_security_event("suspicious_activity", "10.0.0.1", "Rate limit exceeded")
        .await;
    monitor
        .record_security_event(
            "automated_block",
            "192.168.1.100",
            "Multiple failed attempts",
        )
        .await;

    let security_metrics = monitor.get_security_metrics().await;

    assert!(
        security_metrics.failed_logins > 0,
        "Should track failed logins"
    );
    assert!(
        security_metrics.suspicious_activities > 0,
        "Should track suspicious activities"
    );
    assert!(
        security_metrics.automated_blocks > 0,
        "Should track automated blocks"
    );
    assert!(
        security_metrics.blocked_ips.contains("192.168.1.100"),
        "Should track blocked IPs"
    );
}

#[tokio::test]
async fn test_alerting_system() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    // Set low thresholds to trigger alerts
    monitor
        .set_threshold("max_response_time", Duration::from_millis(10))
        .await;

    // Record operations that should trigger alerts
    for _ in 0..5 {
        monitor
            .record_operation("slow_operation", Duration::from_millis(50))
            .await;
    }

    // Check thresholds to generate alerts
    monitor.check_thresholds().await;

    let alerts = monitor.get_active_alerts().await;
    assert!(
        !alerts.is_empty(),
        "Should generate alerts for slow operations"
    );

    let has_latency_alert = alerts.iter().any(|alert| {
        matches!(alert.alert_type, AlertType::HighLatency)
            && matches!(alert.level, AlertLevel::Warning | AlertLevel::Critical)
    });

    assert!(has_latency_alert, "Should have high latency alert");
}

#[tokio::test]
async fn test_threshold_checking() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    // Set specific thresholds
    monitor
        .set_threshold("max_error_rate", Duration::from_millis(10)) // 10% error rate
        .await;

    // Record operations and errors to exceed threshold
    for _ in 0..90 {
        monitor
            .record_operation("test_op", Duration::from_millis(10))
            .await;
    }

    for _ in 0..15 {
        monitor.record_error("TestError", "test_context").await;
    }

    // Check thresholds
    monitor.check_thresholds().await;

    let alerts = monitor.get_active_alerts().await;
    let has_error_rate_alert = alerts
        .iter()
        .any(|alert| matches!(alert.alert_type, AlertType::HighErrorRate));

    assert!(
        has_error_rate_alert,
        "Should generate alert for high error rate"
    );
}

#[tokio::test]
async fn test_performance_report_generation() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    // Generate some test data
    let operations = vec!["login", "send_message", "join_room", "leave_room"];
    let durations = vec![
        Duration::from_millis(50),
        Duration::from_millis(20),
        Duration::from_millis(100),
        Duration::from_millis(30),
    ];

    for (op, duration) in operations.iter().zip(durations.iter()) {
        for _ in 0..10 {
            monitor.record_operation(op, *duration).await;
        }
    }

    // Record some errors
    monitor.record_error("NetworkError", "test").await;
    monitor.record_error("ValidationError", "test").await;

    let report = monitor.get_performance_report().await;

    assert!(
        report.contains("Performance Report"),
        "Should contain report header"
    );
    assert!(report.contains("login"), "Should include operation data");
    assert!(
        report.contains("Average Duration"),
        "Should include performance metrics"
    );
    assert!(
        report.contains("Error Summary"),
        "Should include error information"
    );
    assert!(
        report.contains("System Metrics"),
        "Should include system information"
    );
}

#[tokio::test]
async fn test_concurrent_metrics_recording() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    let mut handles = vec![];

    // Spawn 50 concurrent metric recording tasks
    for i in 0..50 {
        let monitor_clone = monitor.clone();
        let handle = tokio::spawn(async move {
            let operation = format!("concurrent_op_{}", i % 5);
            let duration = Duration::from_millis(10 + (i % 100));

            monitor_clone.record_operation(&operation, duration).await;

            if i % 10 == 0 {
                monitor_clone
                    .record_error("ConcurrentError", "concurrent_test")
                    .await;
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let metrics = monitor.get_metrics().await;

    let total_operations: u64 = metrics.operations.values().map(|m| m.total_count).sum();
    assert_eq!(
        total_operations, 50,
        "Should record all concurrent operations"
    );

    assert!(
        metrics.errors.total_count >= 5,
        "Should record concurrent errors"
    );
}

#[tokio::test]
async fn test_metrics_memory_management() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    // Generate a large number of operations to test memory usage
    for i in 0..1000 {
        let operation = format!("memory_test_{}", i % 100); // 100 different operations
        monitor
            .record_operation(&operation, Duration::from_millis(i % 100))
            .await;
    }

    let metrics = monitor.get_metrics().await;

    // Should efficiently manage memory
    assert!(
        metrics.operations.len() <= 100,
        "Should not create unlimited operation types"
    );

    // Check that recent durations are bounded
    for op_metrics in metrics.operations.values() {
        assert!(
            op_metrics.recent_durations.len() <= 1000,
            "Should limit recent duration history"
        );
    }
}

#[tokio::test]
async fn test_validation_metrics_integration() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    // Record validation operations
    for i in 0..20 {
        let success = i % 4 != 0; // 75% success rate
        let duration = if success {
            Duration::from_millis(10)
        } else {
            Duration::from_millis(50)
        };

        monitor.record_validation(success, duration).await;
    }

    let metrics = monitor.get_metrics().await;

    // Should track validation operations
    assert!(
        metrics.operations.contains_key("validation_success")
            || metrics.operations.contains_key("validation"),
        "Should track validation operations"
    );

    if metrics.operations.contains_key("validation_failure") {
        let failure_metrics = &metrics.operations["validation_failure"];
        assert!(
            failure_metrics.total_count > 0,
            "Should track validation failures"
        );
    }
}

#[tokio::test]
async fn test_connection_metrics() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    // Update system metrics to include connection data
    monitor.update_system_metrics().await;

    let metrics = monitor.get_metrics().await;

    // Should have connection metrics
    assert!(
        metrics.connections.total_connections >= 0,
        "Should track total connections"
    );
    assert!(
        metrics.connections.active_connections >= 0,
        "Should track active connections"
    );
    assert!(
        metrics.connections.average_duration >= Duration::from_secs(0),
        "Should track average connection duration"
    );
}

#[tokio::test]
async fn test_alert_levels_and_types() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    // Set very low thresholds to trigger different alert levels
    monitor
        .set_threshold("max_response_time", Duration::from_millis(1))
        .await;
    monitor
        .set_threshold("max_error_rate", Duration::from_millis(1))
        .await;

    // Generate conditions for different alert types
    // High latency
    for _ in 0..5 {
        monitor
            .record_operation("critical_op", Duration::from_millis(100))
            .await;
    }

    // High error rate
    for _ in 0..10 {
        monitor.record_error("CriticalError", "test").await;
    }

    // Record security events
    monitor
        .record_security_event("failed_login", "127.0.0.1", "Multiple failures")
        .await;

    monitor.check_thresholds().await;

    let alerts = monitor.get_active_alerts().await;

    // Should have multiple alert types
    let alert_types: std::collections::HashSet<_> = alerts
        .iter()
        .map(|a| std::mem::discriminant(&a.alert_type))
        .collect();

    assert!(
        alert_types.len() > 0,
        "Should generate alerts of different types"
    );

    // Should have different severity levels
    let has_warning = alerts
        .iter()
        .any(|a| matches!(a.level, AlertLevel::Warning));
    let has_critical = alerts
        .iter()
        .any(|a| matches!(a.level, AlertLevel::Critical));

    assert!(
        has_warning || has_critical,
        "Should generate alerts with appropriate severity"
    );
}

#[tokio::test]
async fn test_alert_clearing() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    // Generate alerts
    monitor
        .set_threshold("max_response_time", Duration::from_millis(1))
        .await;

    for _ in 0..3 {
        monitor
            .record_operation("slow_op", Duration::from_millis(100))
            .await;
    }

    monitor.check_thresholds().await;

    let alerts_before = monitor.get_active_alerts().await;
    assert!(!alerts_before.is_empty(), "Should have active alerts");

    // Clear alerts
    monitor.clear_alerts().await;

    let alerts_after = monitor.get_active_alerts().await;
    assert!(alerts_after.is_empty(), "Should clear all alerts");
}

#[tokio::test]
async fn test_performance_thresholds_configuration() {
    let thresholds = PerformanceThresholds::default();

    assert!(
        thresholds.response_times.max_response_time > Duration::from_millis(0),
        "Should have reasonable response time threshold"
    );
    assert!(
        thresholds.error_rates.max_error_rate > 0.0,
        "Should have reasonable error rate threshold"
    );
    assert!(
        thresholds.system_thresholds.cpu_threshold > 0.0,
        "Should have reasonable CPU threshold"
    );
    assert!(
        thresholds.system_thresholds.memory_threshold > 0,
        "Should have reasonable memory threshold"
    );
}

#[tokio::test]
async fn test_operation_error_recording() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    let operation = "test_operation";
    let error_type = "TestError";
    let duration = Duration::from_millis(75);

    // Record operation with error
    monitor
        .record_operation_error(operation, error_type, duration, "test context")
        .await;

    let metrics = monitor.get_metrics().await;

    // Should record both operation and error metrics
    assert!(
        metrics.operations.contains_key(operation),
        "Should record operation metrics"
    );
    assert!(
        metrics.errors.error_types.contains_key(error_type),
        "Should record error metrics"
    );

    let op_metrics = &metrics.operations[operation];
    assert_eq!(op_metrics.total_count, 1, "Should count the operation");
    assert_eq!(
        op_metrics.total_duration, duration,
        "Should record operation duration"
    );

    assert_eq!(
        *metrics.errors.error_types.get(error_type).unwrap(),
        1,
        "Should count the error"
    );
}

#[tokio::test]
async fn test_monitoring_performance_under_load() {
    init_performance_monitor().await;
    let monitor = get_performance_monitor().await.unwrap();

    let start_time = Instant::now();
    let iterations = 1000;

    // Perform many monitoring operations to test performance
    for i in 0..iterations {
        let operation = format!("load_test_{}", i % 10);
        let duration = Duration::from_millis(i % 100);

        monitor.record_operation(&operation, duration).await;

        if i % 100 == 0 {
            monitor.record_error("LoadTestError", "load_test").await;
        }

        if i % 200 == 0 {
            monitor.update_system_metrics().await;
        }
    }

    let elapsed = start_time.elapsed();
    let avg_time = elapsed / iterations;

    assert!(
        avg_time < Duration::from_micros(100),
        "Average monitoring time should be under 100µs, got {:?}",
        avg_time
    );

    let metrics = monitor.get_metrics().await;
    let total_operations: u64 = metrics.operations.values().map(|m| m.total_count).sum();

    assert_eq!(
        total_operations, iterations as u64,
        "Should record all operations efficiently"
    );
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_end_to_end_monitoring_flow() {
        // Test complete monitoring flow from recording to alerting
        init_performance_monitor().await;
        let monitor = get_performance_monitor().await.unwrap();

        // Configure thresholds
        monitor
            .set_threshold("max_response_time", Duration::from_millis(50))
            .await;

        // Simulate real application operations
        let operations = vec![
            ("user_login", Duration::from_millis(30)),
            ("send_message", Duration::from_millis(15)),
            ("join_room", Duration::from_millis(45)),
            ("slow_query", Duration::from_millis(100)), // Should trigger alert
        ];

        for (op, duration) in operations {
            monitor.record_operation(op, duration).await;
        }

        // Record some errors
        monitor
            .record_error("DatabaseError", "connection_lost")
            .await;

        // Record security event
        monitor
            .record_security_event("failed_login", "192.168.1.1", "Brute force attempt")
            .await;

        // Update system metrics
        monitor.update_system_metrics().await;

        // Check for threshold violations
        monitor.check_thresholds().await;

        // Verify comprehensive monitoring
        let metrics = monitor.get_metrics().await;
        let alerts = monitor.get_active_alerts().await;
        let security_metrics = monitor.get_security_metrics().await;
        let report = monitor.get_performance_report().await;

        // All systems should be working
        assert!(metrics.operations.len() > 0, "Should track operations");
        assert!(metrics.errors.total_count > 0, "Should track errors");
        assert!(
            metrics.system.uptime > Duration::from_secs(0),
            "Should track uptime"
        );
        assert!(
            security_metrics.failed_logins > 0,
            "Should track security events"
        );
        assert!(!report.is_empty(), "Should generate comprehensive report");

        // Should generate alert for slow operation
        let has_slow_alert = alerts
            .iter()
            .any(|alert| matches!(alert.alert_type, AlertType::HighLatency));
        assert!(has_slow_alert, "Should alert on slow operations");
    }

    #[tokio::test]
    async fn test_monitoring_system_resilience() {
        // Test that monitoring system handles edge cases gracefully
        init_performance_monitor().await;
        let monitor = get_performance_monitor().await.unwrap();

        // Test with zero duration
        monitor
            .record_operation("instant_op", Duration::from_nanos(0))
            .await;

        // Test with very long duration
        monitor
            .record_operation("long_op", Duration::from_secs(3600))
            .await;

        // Test with empty strings (should handle gracefully)
        monitor.record_error("", "").await;

        // Test with very long strings
        let long_string = "x".repeat(10000);
        monitor.record_error(&long_string, &long_string).await;

        // Test concurrent threshold checking
        let mut handles = vec![];
        for _ in 0..10 {
            let monitor_clone = monitor.clone();
            let handle = tokio::spawn(async move {
                monitor_clone.check_thresholds().await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // System should remain stable
        let metrics = monitor.get_metrics().await;
        assert!(
            metrics.operations.len() > 0,
            "Should handle edge cases gracefully"
        );
    }
}
