//! Integration tests for Phase 7 frameworks
//!
//! This module provides comprehensive testing for the integration between
//! error handling, validation, monitoring, and security frameworks implemented
//! in Phase 7.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

// Import all frameworks for integration testing
use lair_chat::common::errors::{CommonError, CommonResult};
use lair_chat::server::error::{
    get_error_handler, init_error_handler, CircuitBreakerState, ErrorHandler, TcpError,
};
use lair_chat::server::monitoring::{
    get_performance_monitor, init_performance_monitor, AlertLevel, AlertType, PerformanceMonitor,
};
use lair_chat::server::security::{get_security_manager, init_security_manager, SecurityManager};
use lair_chat::server::validation::{
    get_validation_system, init_validation_system, ValidationSystem,
};

/// Setup function to initialize all frameworks for integration testing
async fn setup_integrated_systems() -> (
    ErrorHandler,
    ValidationSystem,
    PerformanceMonitor,
    SecurityManager,
) {
    // Initialize all systems
    init_error_handler().await;
    init_validation_system().await;
    init_performance_monitor().await;
    init_security_manager().await;

    let error_handler = get_error_handler().await.unwrap();
    let validation_system = get_validation_system().await.unwrap();
    let performance_monitor = get_performance_monitor().await.unwrap();
    let security_manager = get_security_manager().await.unwrap();

    (
        error_handler,
        validation_system,
        performance_monitor,
        security_manager,
    )
}

#[tokio::test]
async fn test_validation_error_handling_integration() {
    let (error_handler, validation_system, performance_monitor, _) =
        setup_integrated_systems().await;

    let user_id = "integration_test_user";
    let malicious_input = "'; DROP TABLE users; --";

    let start_time = Instant::now();

    // Attempt validation of malicious input
    let validation_result = validation_system
        .validate_input(malicious_input, user_id)
        .await;

    // Should fail validation
    assert!(
        validation_result.is_err(),
        "Malicious input should fail validation"
    );

    // Handle the validation error through error system
    if let Err(validation_error) = validation_result {
        let error_response = error_handler
            .handle_error(&validation_error, "input_validation")
            .await
            .unwrap();

        // Should generate appropriate error response
        assert!(
            error_response.error_code.contains("VALIDATION")
                || error_response.error_code.contains("SECURITY"),
            "Should generate validation/security error code"
        );

        // Record the operation in monitoring system
        let operation_duration = start_time.elapsed();
        performance_monitor
            .record_operation_error(
                "input_validation",
                "ValidationError",
                operation_duration,
                "malicious_input_detected",
            )
            .await;
    }

    // Verify integration worked
    let error_stats = error_handler.get_stats().await;
    let performance_metrics = performance_monitor.get_metrics().await;
    let validation_stats = validation_system.get_stats().await;

    assert!(
        error_stats.total_errors > 0,
        "Should track validation errors"
    );
    assert!(
        performance_metrics
            .operations
            .contains_key("input_validation"),
        "Should track validation operations"
    );
    assert!(
        validation_stats.security_violations > 0,
        "Should track security violations"
    );
}

#[tokio::test]
async fn test_rate_limiting_circuit_breaker_integration() {
    let (error_handler, validation_system, performance_monitor, security_manager) =
        setup_integrated_systems().await;

    let user_id = "rate_limit_test_user";
    let operation_key = "user_validation";

    // Perform rapid validation attempts to trigger rate limiting
    let mut successful_validations = 0;
    let mut rate_limited = 0;
    let mut circuit_broken = 0;

    for i in 0..30 {
        let input = format!("/join room_{}", i);

        // Check if circuit breaker allows execution
        if !error_handler.should_execute_operation(operation_key).await {
            circuit_broken += 1;
            continue;
        }

        // Attempt validation
        let start_time = Instant::now();
        let result = validation_system.validate_input(&input, user_id).await;
        let duration = start_time.elapsed();

        match result {
            Ok(_) => {
                successful_validations += 1;
                error_handler.record_success(operation_key).await;
                performance_monitor
                    .record_operation("validation", duration)
                    .await;
            }
            Err(ref e) if format!("{:?}", e).contains("RateLimit") => {
                rate_limited += 1;
                error_handler.record_failure(operation_key).await;
                performance_monitor
                    .record_operation_error(
                        "validation",
                        "RateLimitError",
                        duration,
                        "rate_limit_exceeded",
                    )
                    .await;

                // Record security event for rate limiting
                security_manager
                    .record_security_event("rate_limit_violation", user_id, "Rapid requests")
                    .await;
            }
            Err(error) => {
                error_handler.record_failure(operation_key).await;
                let _ = error_handler
                    .handle_error(&error, "validation_failure")
                    .await;
            }
        }

        // Small delay to allow systems to process
        if i % 10 == 0 {
            sleep(Duration::from_millis(10)).await;
        }
    }

    // Verify integration behavior
    assert!(
        successful_validations > 0,
        "Some validations should succeed initially"
    );
    assert!(
        rate_limited > 0 || circuit_broken > 0,
        "Should trigger rate limiting or circuit breaking"
    );

    // Check circuit breaker state
    let circuit_breaker = error_handler.get_circuit_breaker(operation_key).await;
    if let Some(cb) = circuit_breaker {
        // Circuit breaker should have recorded failures
        assert!(cb.failure_count() > 0, "Should record failures");
    }

    // Check security metrics
    let security_metrics = security_manager.get_security_metrics().await;
    if rate_limited > 0 {
        assert!(
            security_metrics.rate_limit_violations > 0,
            "Should track rate limit violations"
        );
    }
}

#[tokio::test]
async fn test_security_monitoring_integration() {
    let (error_handler, validation_system, performance_monitor, security_manager) =
        setup_integrated_systems().await;

    let attacker_ip = "192.168.1.100";
    let user_id = "potential_attacker";

    // Simulate various attack patterns
    let attack_patterns = vec![
        "'; DROP TABLE users; --",
        "<script>alert('xss')</script>",
        "../../../../etc/passwd",
        "SELECT * FROM sensitive_data",
        "../../../windows/system32/config/sam",
    ];

    for (i, attack) in attack_patterns.iter().enumerate() {
        let start_time = Instant::now();

        // Attempt validation (should fail)
        let result = validation_system.validate_input(attack, user_id).await;
        let duration = start_time.elapsed();

        if let Err(validation_error) = result {
            // Handle validation error
            let error_response = error_handler
                .handle_error(&validation_error, "security_validation")
                .await
                .unwrap();

            // Record security event
            security_manager
                .record_security_event(
                    "malicious_input_attempt",
                    attacker_ip,
                    &format!("Attack pattern {}: {}", i + 1, attack),
                )
                .await;

            // Record performance metrics
            performance_monitor
                .record_operation_error(
                    "security_validation",
                    "SecurityViolation",
                    duration,
                    "malicious_input_blocked",
                )
                .await;

            // Check if IP should be blocked after multiple attempts
            if i >= 2 {
                let should_block = security_manager.should_block_ip(attacker_ip).await;
                if should_block {
                    security_manager
                        .record_security_event(
                            "automated_ip_block",
                            attacker_ip,
                            "Multiple security violations",
                        )
                        .await;
                }
            }
        }
    }

    // Verify security integration
    let security_metrics = security_manager.get_security_metrics().await;
    let error_stats = error_handler.get_stats().await;
    let performance_metrics = performance_monitor.get_metrics().await;

    assert!(
        security_metrics.malicious_attempts > 0,
        "Should track malicious attempts"
    );
    assert!(
        error_stats.errors_by_type.contains_key("SecurityViolation")
            || error_stats.errors_by_type.contains_key("ValidationError"),
        "Should track security-related errors"
    );
    assert!(
        performance_metrics
            .operations
            .contains_key("security_validation"),
        "Should monitor security operations"
    );

    // Check if IP was eventually blocked
    let is_blocked = security_manager.is_ip_blocked(attacker_ip).await;
    if is_blocked {
        assert!(
            security_metrics.blocked_ips.contains(attacker_ip),
            "Should track blocked IPs"
        );
    }
}

#[tokio::test]
async fn test_performance_alerting_integration() {
    let (error_handler, validation_system, performance_monitor, _) =
        setup_integrated_systems().await;

    // Set low performance thresholds to trigger alerts
    performance_monitor
        .set_threshold("max_response_time", Duration::from_millis(20))
        .await;
    performance_monitor
        .set_threshold("max_error_rate", Duration::from_millis(10)) // 10% error rate
        .await;

    let user_id = "performance_test_user";

    // Simulate slow operations that should trigger alerts
    for i in 0..20 {
        let input = format!("/join room_{}", i);
        let start_time = Instant::now();

        // Artificially slow down some operations
        if i % 3 == 0 {
            sleep(Duration::from_millis(50)).await; // Slow operation
        }

        let result = validation_system.validate_input(&input, user_id).await;
        let duration = start_time.elapsed();

        match result {
            Ok(_) => {
                performance_monitor
                    .record_operation("validation", duration)
                    .await;
            }
            Err(error) => {
                performance_monitor
                    .record_operation_error(
                        "validation",
                        "ValidationError",
                        duration,
                        "validation_failed",
                    )
                    .await;

                // Handle error through error system
                let _ = error_handler.handle_error(&error, "performance_test").await;
            }
        }

        // Generate some errors to increase error rate
        if i % 5 == 0 {
            let error = TcpError::TemporaryFailure("Simulated failure".to_string());
            error_handler
                .handle_error(&error, "performance_test")
                .await
                .unwrap();
            performance_monitor
                .record_error("SimulatedError", "performance_test")
                .await;
        }
    }

    // Check for threshold violations and alerts
    performance_monitor.check_thresholds().await;

    let alerts = performance_monitor.get_active_alerts().await;
    let error_stats = error_handler.get_stats().await;
    let performance_metrics = performance_monitor.get_metrics().await;

    // Should generate performance-related alerts
    assert!(!alerts.is_empty(), "Should generate performance alerts");

    let has_latency_alert = alerts
        .iter()
        .any(|alert| matches!(alert.alert_type, AlertType::HighLatency));
    let has_error_rate_alert = alerts
        .iter()
        .any(|alert| matches!(alert.alert_type, AlertType::HighErrorRate));

    assert!(
        has_latency_alert || has_error_rate_alert,
        "Should have performance-related alerts"
    );

    // Error handler should have processed errors
    assert!(error_stats.total_errors > 0, "Should track errors");

    // Performance monitor should have comprehensive metrics
    assert!(
        performance_metrics.operations.contains_key("validation"),
        "Should track operations"
    );
    assert!(
        performance_metrics.errors.total_count > 0,
        "Should track error metrics"
    );
}

#[tokio::test]
async fn test_end_to_end_framework_integration() {
    let (error_handler, validation_system, performance_monitor, security_manager) =
        setup_integrated_systems().await;

    // Configure systems for integration test
    performance_monitor
        .set_threshold("max_response_time", Duration::from_millis(100))
        .await;

    let test_scenarios = vec![
        ("normal_user", "/join general", true),
        ("normal_user", "/msg friend hello", true),
        ("attacker", "'; DROP TABLE users; --", false),
        ("spammer", "/spam message", false),
        ("normal_user", "/help", true),
        ("attacker", "<script>alert('xss')</script>", false),
    ];

    let mut successful_operations = 0;
    let mut blocked_operations = 0;
    let mut security_violations = 0;

    for (user_id, input, should_succeed) in test_scenarios {
        let operation_start = Instant::now();
        let operation_key = format!("user_operation_{}", user_id);

        // Check security status first
        let is_blocked = security_manager.is_ip_blocked(user_id).await;
        if is_blocked {
            blocked_operations += 1;
            continue;
        }

        // Check circuit breaker
        if !error_handler.should_execute_operation(&operation_key).await {
            blocked_operations += 1;
            continue;
        }

        // Attempt validation
        let validation_result = validation_system.validate_input(input, user_id).await;
        let operation_duration = operation_start.elapsed();

        match validation_result {
            Ok(validated_input) => {
                if should_succeed {
                    successful_operations += 1;
                    error_handler.record_success(&operation_key).await;
                    performance_monitor
                        .record_operation("user_operation", operation_duration)
                        .await;
                } else {
                    // This shouldn't happen - malicious input should fail validation
                    panic!("Malicious input passed validation: {}", input);
                }
            }
            Err(validation_error) => {
                if !should_succeed {
                    security_violations += 1;

                    // Record security event
                    security_manager
                        .record_security_event(
                            "validation_security_violation",
                            user_id,
                            &format!("Malicious input: {}", input),
                        )
                        .await;

                    // Handle error
                    let _ = error_handler
                        .handle_error(&validation_error, "security_validation")
                        .await;

                    // Record performance metrics for security operations
                    performance_monitor
                        .record_operation_error(
                            "security_validation",
                            "SecurityViolation",
                            operation_duration,
                            "malicious_input_blocked",
                        )
                        .await;

                    // Record circuit breaker failure
                    error_handler.record_failure(&operation_key).await;
                } else {
                    // Valid input failed - this might be due to rate limiting
                    let _ = error_handler
                        .handle_error(&validation_error, "validation_failure")
                        .await;
                }
            }
        }
    }

    // Check integration results
    let error_stats = error_handler.get_stats().await;
    let validation_stats = validation_system.get_stats().await;
    let performance_metrics = performance_monitor.get_metrics().await;
    let security_metrics = security_manager.get_security_metrics().await;

    // Verify comprehensive framework integration
    assert!(
        successful_operations > 0,
        "Should have successful operations for valid users"
    );
    assert!(
        security_violations > 0,
        "Should detect and block security violations"
    );

    // All systems should have recorded activity
    assert!(
        error_stats.total_errors > 0,
        "Error system should be active"
    );
    assert!(
        validation_stats.total_validations > 0,
        "Validation system should be active"
    );
    assert!(
        performance_metrics.operations.len() > 0,
        "Performance monitoring should be active"
    );
    assert!(
        security_metrics.security_events > 0,
        "Security system should be active"
    );

    // Check cross-system data consistency
    assert!(
        validation_stats.security_violations <= security_metrics.security_events,
        "Security events should be consistent across systems"
    );

    // Generate comprehensive report
    let performance_report = performance_monitor.get_performance_report().await;
    assert!(
        performance_report.contains("Security"),
        "Performance report should include security metrics"
    );

    // Check for performance alerts if thresholds were exceeded
    performance_monitor.check_thresholds().await;
    let alerts = performance_monitor.get_active_alerts().await;

    // System should be stable and responsive
    let total_operations: u64 = performance_metrics
        .operations
        .values()
        .map(|m| m.total_count)
        .sum();
    assert!(
        total_operations > 0,
        "Should have processed operations efficiently"
    );
}

#[tokio::test]
async fn test_framework_resilience_under_load() {
    let (error_handler, validation_system, performance_monitor, security_manager) =
        setup_integrated_systems().await;

    let concurrent_users = 50;
    let operations_per_user = 20;
    let mut handles = vec![];

    // Spawn concurrent tasks to test framework integration under load
    for user_i in 0..concurrent_users {
        let error_handler_clone = error_handler.clone();
        let validation_system_clone = validation_system.clone();
        let performance_monitor_clone = performance_monitor.clone();
        let security_manager_clone = security_manager.clone();

        let handle = tokio::spawn(async move {
            let user_id = format!("load_test_user_{}", user_i);
            let mut successful = 0;
            let mut failed = 0;

            for op_i in 0..operations_per_user {
                let input = if user_i % 10 == 0 && op_i % 5 == 0 {
                    // Inject some malicious inputs
                    "'; DROP TABLE users; --".to_string()
                } else {
                    format!("/join room_{}_{}", user_i, op_i)
                };

                let operation_start = Instant::now();
                let operation_key = format!("load_test_{}_{}", user_i, op_i);

                // Check circuit breaker
                if !error_handler_clone
                    .should_execute_operation(&operation_key)
                    .await
                {
                    failed += 1;
                    continue;
                }

                // Validate input
                match validation_system_clone
                    .validate_input(&input, &user_id)
                    .await
                {
                    Ok(_) => {
                        successful += 1;
                        error_handler_clone.record_success(&operation_key).await;
                        performance_monitor_clone
                            .record_operation("load_test_operation", operation_start.elapsed())
                            .await;
                    }
                    Err(error) => {
                        failed += 1;
                        error_handler_clone.record_failure(&operation_key).await;

                        // Handle security violations
                        if format!("{:?}", error).contains("Security") {
                            security_manager_clone
                                .record_security_event(
                                    "load_test_security_violation",
                                    &user_id,
                                    "Malicious input during load test",
                                )
                                .await;
                        }

                        let _ = error_handler_clone.handle_error(&error, "load_test").await;
                        performance_monitor_clone
                            .record_operation_error(
                                "load_test_operation",
                                "LoadTestError",
                                operation_start.elapsed(),
                                "operation_failed",
                            )
                            .await;
                    }
                }
            }

            (successful, failed)
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut total_successful = 0;
    let mut total_failed = 0;

    for handle in handles {
        let (successful, failed) = handle.await.unwrap();
        total_successful += successful;
        total_failed += failed;
    }

    // Verify framework resilience
    assert!(
        total_successful > 0,
        "Should handle successful operations under load"
    );
    assert!(
        total_failed > 0,
        "Should handle failures appropriately under load"
    );

    // Check that all systems maintained data integrity
    let error_stats = error_handler.get_stats().await;
    let validation_stats = validation_system.get_stats().await;
    let performance_metrics = performance_monitor.get_metrics().await;
    let security_metrics = security_manager.get_security_metrics().await;

    // All systems should have processed events
    assert!(
        error_stats.total_errors > 0,
        "Error system should handle load"
    );
    assert!(
        validation_stats.total_validations > 0,
        "Validation system should handle load"
    );
    assert!(
        performance_metrics.operations.len() > 0,
        "Performance monitoring should handle load"
    );

    // Security system should have detected violations
    if security_metrics.security_events > 0 {
        assert!(
            security_metrics.malicious_attempts > 0,
            "Should detect security violations under load"
        );
    }

    // Check system performance under load
    let total_operations: u64 = performance_metrics
        .operations
        .values()
        .map(|m| m.total_count)
        .sum();

    assert!(
        total_operations >= (total_successful + total_failed) as u64,
        "Should track all operations accurately under load"
    );

    // System should remain responsive
    if let Some(load_test_metrics) = performance_metrics.operations.get("load_test_operation") {
        assert!(
            load_test_metrics.average_duration < Duration::from_millis(100),
            "Should maintain performance under load"
        );
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;

    #[tokio::test]
    async fn test_framework_integration_memory_efficiency() {
        let (error_handler, validation_system, performance_monitor, security_manager) =
            setup_integrated_systems().await;

        // Generate sustained load to test memory efficiency
        for batch in 0..100 {
            for i in 0..50 {
                let user_id = format!("memory_test_{}_{}", batch, i);
                let input = format!("/test command {}", i);
                let operation_key = format!("memory_test_{}_{}", batch, i);

                // Perform full framework integration
                let start_time = Instant::now();

                match validation_system.validate_input(&input, &user_id).await {
                    Ok(_) => {
                        error_handler.record_success(&operation_key).await;
                        performance_monitor
                            .record_operation("memory_test", start_time.elapsed())
                            .await;
                    }
                    Err(error) => {
                        error_handler.record_failure(&operation_key).await;
                        let _ = error_handler.handle_error(&error, "memory_test").await;
                        performance_monitor
                            .record_operation_error(
                                "memory_test",
                                "MemoryTestError",
                                start_time.elapsed(),
                                "test_error",
                            )
                            .await;
                    }
                }
            }

            // Periodically reset stats to prevent unbounded growth
            if batch % 20 == 0 {
                error_handler.reset_stats().await;
                validation_system.reset_stats().await;
                performance_monitor.clear_alerts().await;
            }
        }

        // Verify memory efficiency
        let final_error_stats = error_handler.get_stats().await;
        let final_validation_stats = validation_system.get_stats().await;
        let final_performance_metrics = performance_monitor.get_metrics().await;

        // Should manage memory efficiently with periodic resets
        assert!(
            final_error_stats.total_errors < 5000,
            "Should manage error statistics memory efficiently"
        );
        assert!(
            final_validation_stats.total_validations < 5000,
            "Should manage validation statistics memory efficiently"
        );
        assert!(
            final_performance_metrics.operations.len() < 100,
            "Should manage performance metrics memory efficiently"
        );
    }
}
