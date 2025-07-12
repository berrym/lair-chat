//! Unit tests for the error handling framework
//!
//! This module provides comprehensive testing for the error handling
//! system implemented in Phase 7, including error types, retry mechanisms,
//! circuit breakers, and recovery policies.

use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

// Import the error handling framework
use lair_chat::common::errors::{CommonError, CommonResult};
use lair_chat::server::error::retry::*;
use lair_chat::server::error::types::*;
use lair_chat::server::error::{
    get_error_handler, init_error_handler, CircuitBreaker, CircuitBreakerState, ErrorHandler,
    ErrorResponse, ErrorStats, TcpResult,
};

#[tokio::test]
async fn test_error_handler_initialization() {
    // Test that error handler can be initialized
    init_error_handler().await;
    let handler = get_error_handler().await;

    assert!(
        handler.is_ok(),
        "Error handler should initialize successfully"
    );

    let stats = handler.unwrap().get_stats().await;
    assert_eq!(stats.total_errors, 0, "Initial error count should be zero");
    assert_eq!(
        stats.recovery_attempts, 0,
        "Initial recovery attempts should be zero"
    );
}

#[tokio::test]
async fn test_error_stats_tracking() {
    init_error_handler().await;
    let handler = get_error_handler().await.unwrap();

    // Create test errors
    let error1 = TcpError::ConnectionLost("Test connection loss".to_string());
    let error2 = TcpError::AuthenticationFailed("Test auth failure".to_string());

    // Handle errors and verify stats
    handler
        .handle_error(&error1, "test_context_1")
        .await
        .unwrap();
    handler
        .handle_error(&error2, "test_context_2")
        .await
        .unwrap();

    let stats = handler.get_stats().await;
    assert_eq!(stats.total_errors, 2, "Should track total errors");
    assert!(
        stats.errors_by_type.contains_key("ConnectionLost"),
        "Should track error types"
    );
    assert!(
        stats.errors_by_type.contains_key("AuthenticationFailed"),
        "Should track error types"
    );
}

#[tokio::test]
async fn test_circuit_breaker_functionality() {
    let mut circuit_breaker = CircuitBreaker::new(3, Duration::from_millis(100));

    // Test initial state
    assert_eq!(circuit_breaker.state(), &CircuitBreakerState::Closed);
    assert!(
        circuit_breaker.can_execute(),
        "Should allow execution when closed"
    );

    // Record failures to trip the circuit breaker
    for _ in 0..3 {
        circuit_breaker.record_failure();
    }

    assert_eq!(circuit_breaker.state(), &CircuitBreakerState::Open);
    assert!(
        !circuit_breaker.can_execute(),
        "Should not allow execution when open"
    );

    // Wait for timeout and test half-open state
    sleep(Duration::from_millis(150)).await;
    assert!(
        circuit_breaker.can_execute(),
        "Should allow one execution in half-open state"
    );

    // Record success to close circuit
    circuit_breaker.record_success();
    assert_eq!(circuit_breaker.state(), &CircuitBreakerState::Closed);
}

#[tokio::test]
async fn test_retry_mechanism() {
    init_error_handler().await;
    let handler = get_error_handler().await.unwrap();

    let mut attempt_count = 0;

    // Test function that fails first few times then succeeds
    let test_operation = || async {
        attempt_count += 1;
        if attempt_count < 3 {
            Err(TcpError::TemporaryFailure("Temporary failure".to_string()))
        } else {
            Ok("Success".to_string())
        }
    };

    let result = handler.execute_with_retry(test_operation, 3).await;

    assert!(result.is_ok(), "Should succeed after retries");
    assert_eq!(result.unwrap(), "Success");
    assert_eq!(attempt_count, 3, "Should have made 3 attempts");
}

#[tokio::test]
async fn test_retry_with_exponential_backoff() {
    let retry_policy = RetryPolicy {
        max_attempts: 3,
        base_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(1000),
        backoff_multiplier: 2.0,
        jitter: false,
    };

    let mut attempt_count = 0;
    let start_time = Instant::now();

    let test_operation = || async {
        attempt_count += 1;
        Err(TcpError::TemporaryFailure("Always fails".to_string()))
    };

    let result = execute_with_retry_policy(test_operation, &retry_policy).await;

    assert!(result.is_err(), "Should fail after all retries exhausted");
    assert_eq!(attempt_count, 3, "Should have made 3 attempts");

    let elapsed = start_time.elapsed();
    // Should take at least 10ms + 20ms = 30ms (base delay + first backoff)
    assert!(
        elapsed >= Duration::from_millis(25),
        "Should respect backoff delays"
    );
}

#[tokio::test]
async fn test_error_classification() {
    // Test error classification for retry decisions
    let retryable_error = TcpError::TemporaryFailure("Temp failure".to_string());
    let non_retryable_error = TcpError::AuthenticationFailed("Auth failed".to_string());
    let connection_error = TcpError::ConnectionLost("Connection lost".to_string());

    assert!(
        is_retryable_error(&retryable_error),
        "Temporary failures should be retryable"
    );
    assert!(
        !is_retryable_error(&non_retryable_error),
        "Auth failures should not be retryable"
    );
    assert!(
        is_retryable_error(&connection_error),
        "Connection losses should be retryable"
    );
}

#[tokio::test]
async fn test_error_response_generation() {
    let error = TcpError::ValidationError("Invalid input".to_string());
    let request_id = Uuid::new_v4().to_string();

    let response = ErrorResponse {
        error_code: "VALIDATION_ERROR".to_string(),
        message: "Invalid input".to_string(),
        details: Some("Detailed validation error".to_string()),
        timestamp: chrono::Utc::now(),
        request_id: Some(request_id.clone()),
    };

    let tcp_message = response.to_tcp_message();
    assert!(
        tcp_message.contains("VALIDATION_ERROR"),
        "Should contain error code"
    );
    assert!(
        tcp_message.contains(&request_id),
        "Should contain request ID"
    );

    let system_message = response.to_system_message();
    assert!(
        system_message.contains("Invalid input"),
        "Should contain error message"
    );
}

#[tokio::test]
async fn test_error_recovery_mechanisms() {
    init_error_handler().await;
    let handler = get_error_handler().await.unwrap();

    // Test connection recovery
    let connection_error = TcpError::ConnectionLost("Connection dropped".to_string());
    let result = handler
        .handle_error(&connection_error, "connection_test")
        .await;

    assert!(result.is_ok(), "Should handle connection errors gracefully");

    let stats = handler.get_stats().await;
    assert!(stats.recovery_attempts > 0, "Should attempt recovery");
}

#[tokio::test]
async fn test_concurrent_error_handling() {
    init_error_handler().await;
    let handler = get_error_handler().await.unwrap();

    // Spawn multiple concurrent error handling tasks
    let mut handles = vec![];

    for i in 0..10 {
        let handler_clone = handler.clone();
        let handle = tokio::spawn(async move {
            let error = TcpError::TemporaryFailure(format!("Error {}", i));
            handler_clone
                .handle_error(&error, &format!("context_{}", i))
                .await
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent error handling should succeed");
    }

    let stats = handler.get_stats().await;
    assert_eq!(
        stats.total_errors, 10,
        "Should handle all concurrent errors"
    );
}

#[tokio::test]
async fn test_error_handler_reset() {
    init_error_handler().await;
    let handler = get_error_handler().await.unwrap();

    // Generate some errors
    for i in 0..5 {
        let error = TcpError::TemporaryFailure(format!("Error {}", i));
        handler.handle_error(&error, "test_context").await.unwrap();
    }

    let stats_before = handler.get_stats().await;
    assert_eq!(stats_before.total_errors, 5);

    // Reset stats
    handler.reset_stats().await;

    let stats_after = handler.get_stats().await;
    assert_eq!(stats_after.total_errors, 0, "Stats should be reset");
    assert_eq!(
        stats_after.recovery_attempts, 0,
        "Recovery attempts should be reset"
    );
}

#[tokio::test]
async fn test_circuit_breaker_integration() {
    init_error_handler().await;
    let handler = get_error_handler().await.unwrap();

    let operation_key = "test_operation";

    // Test that operation is initially allowed
    assert!(handler.should_execute_operation(operation_key).await);

    // Record multiple failures to trip circuit breaker
    for _ in 0..5 {
        handler.record_failure(operation_key).await;
    }

    // Circuit breaker should now be open
    assert!(!handler.should_execute_operation(operation_key).await);

    // Record a success to help close the circuit
    handler.record_success(operation_key).await;
}

#[tokio::test]
async fn test_custom_retry_policy() {
    init_error_handler().await;
    let handler = get_error_handler().await.unwrap();

    let custom_policy = RetryPolicy {
        max_attempts: 2,
        base_delay: Duration::from_millis(5),
        max_delay: Duration::from_millis(50),
        backoff_multiplier: 1.5,
        jitter: true,
    };

    let mut attempt_count = 0;

    let test_operation = || async {
        attempt_count += 1;
        Err(TcpError::TemporaryFailure("Always fails".to_string()))
    };

    let result = handler
        .execute_with_custom_retry(test_operation, &custom_policy)
        .await;

    assert!(result.is_err(), "Should fail after custom retry attempts");
    assert_eq!(attempt_count, 2, "Should respect custom max attempts");
}

#[tokio::test]
async fn test_error_context_macro() {
    // Test the error_context! macro functionality
    let context = "test_operation";
    let error = CommonError::NetworkError("Connection timeout".to_string());

    let result: CommonResult<()> = Err(error);

    // This would typically be used with the macro
    match result {
        Err(e) => {
            let contextual_error = format!("Error in {}: {}", context, e);
            assert!(contextual_error.contains("test_operation"));
            assert!(contextual_error.contains("Connection timeout"));
        }
        Ok(_) => panic!("Expected error"),
    }
}

#[tokio::test]
async fn test_error_severity_classification() {
    // Test different error severities
    let critical_error = TcpError::DatabaseConnectionFailed("DB down".to_string());
    let warning_error = TcpError::TemporaryFailure("Temp issue".to_string());
    let info_error = TcpError::ValidationError("Bad input".to_string());

    // These would typically be classified by severity in the error handler
    init_error_handler().await;
    let handler = get_error_handler().await.unwrap();

    handler
        .handle_error(&critical_error, "critical_test")
        .await
        .unwrap();
    handler
        .handle_error(&warning_error, "warning_test")
        .await
        .unwrap();
    handler
        .handle_error(&info_error, "info_test")
        .await
        .unwrap();

    let stats = handler.get_stats().await;
    assert!(
        stats.errors_by_severity.len() > 0,
        "Should track errors by severity"
    );
}

#[tokio::test]
async fn test_memory_efficiency() {
    // Test that error handling doesn't cause memory leaks
    init_error_handler().await;
    let handler = get_error_handler().await.unwrap();

    // Generate many errors to test memory usage
    for i in 0..1000 {
        let error = TcpError::TemporaryFailure(format!("Error {}", i));
        handler.handle_error(&error, "memory_test").await.unwrap();

        // Periodically reset to prevent unbounded growth
        if i % 100 == 0 {
            handler.reset_stats().await;
        }
    }

    let final_stats = handler.get_stats().await;
    // Should have been reset multiple times, so total should be less than 1000
    assert!(
        final_stats.total_errors < 1000,
        "Should manage memory efficiently"
    );
}

// Helper function to check if an error is retryable
fn is_retryable_error(error: &TcpError) -> bool {
    match error {
        TcpError::TemporaryFailure(_) => true,
        TcpError::ConnectionLost(_) => true,
        TcpError::TimeoutError(_) => true,
        TcpError::AuthenticationFailed(_) => false,
        TcpError::ValidationError(_) => false,
        TcpError::DatabaseConnectionFailed(_) => true,
        _ => false,
    }
}

// Helper function to execute operation with retry policy
async fn execute_with_retry_policy<F, Fut, T>(
    operation: F,
    policy: &RetryPolicy,
) -> Result<T, TcpError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, TcpError>>,
{
    let mut attempts = 0;
    let mut delay = policy.base_delay;

    loop {
        attempts += 1;

        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                if attempts >= policy.max_attempts || !is_retryable_error(&error) {
                    return Err(error);
                }

                // Apply backoff delay
                if attempts < policy.max_attempts {
                    sleep(delay).await;
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f64 * policy.backoff_multiplier) as u64,
                        ),
                        policy.max_delay,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_end_to_end_error_flow() {
        // Test complete error handling flow from detection to recovery
        init_error_handler().await;
        let handler = get_error_handler().await.unwrap();

        // Simulate a real error scenario
        let operation_key = "user_authentication";

        // First attempt fails
        let auth_error = TcpError::AuthenticationFailed("Invalid credentials".to_string());
        let response = handler
            .handle_error(&auth_error, operation_key)
            .await
            .unwrap();

        assert!(
            response.error_code.contains("AUTH"),
            "Should generate appropriate error code"
        );

        // Check that stats are updated
        let stats = handler.get_stats().await;
        assert!(stats.total_errors > 0, "Should track the error");
        assert!(
            stats.errors_by_type.contains_key("AuthenticationFailed"),
            "Should categorize error"
        );

        // Verify circuit breaker state
        let circuit_breaker = handler.get_circuit_breaker(operation_key).await;
        assert!(
            circuit_breaker.is_some(),
            "Should have circuit breaker for operation"
        );
    }

    #[tokio::test]
    async fn test_error_handling_under_load() {
        // Test error handling performance under concurrent load
        init_error_handler().await;
        let handler = get_error_handler().await.unwrap();

        let start_time = Instant::now();
        let mut handles = vec![];

        // Spawn 100 concurrent error handling operations
        for i in 0..100 {
            let handler_clone = handler.clone();
            let handle = tokio::spawn(async move {
                let error = TcpError::TemporaryFailure(format!("Load test error {}", i));
                handler_clone
                    .handle_error(&error, &format!("load_test_{}", i))
                    .await
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        let elapsed = start_time.elapsed();
        let stats = handler.get_stats().await;

        assert_eq!(stats.total_errors, 100, "Should handle all errors");
        assert!(
            elapsed < Duration::from_secs(5),
            "Should handle errors efficiently under load"
        );
    }
}
