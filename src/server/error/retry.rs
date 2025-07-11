//! Retry mechanism with exponential backoff and circuit breaker patterns
//!
//! This module provides robust retry logic for handling transient failures
//! in TCP operations with configurable backoff strategies.

use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, info, warn};

use super::types::{BackoffStrategy, RecoveryAction, RetryCondition, TcpError};

/// Retry executor with configurable policies
pub struct RetryExecutor {
    /// Maximum number of retry attempts
    max_retries: u32,
    /// Backoff strategy for delays between retries
    backoff_strategy: BackoffStrategy,
    /// Conditions that trigger retries
    retry_conditions: Vec<RetryCondition>,
}

/// Result of a retry attempt
#[derive(Debug, Clone)]
pub struct RetryResult<T> {
    /// Final result after all retry attempts
    pub result: Result<T, TcpError>,
    /// Number of attempts made
    pub attempts: u32,
    /// Total time spent retrying
    pub total_duration: Duration,
    /// Whether the maximum retry limit was reached
    pub max_retries_reached: bool,
}

/// Retry statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct RetryStats {
    /// Total number of operations that required retries
    pub total_retried_operations: u64,
    /// Total number of successful retries
    pub successful_retries: u64,
    /// Total number of failed retries (exhausted all attempts)
    pub failed_retries: u64,
    /// Average number of attempts per operation
    pub average_attempts: f64,
    /// Total time spent on retries
    pub total_retry_time: Duration,
}

impl RetryExecutor {
    /// Create a new retry executor with default settings
    pub fn new() -> Self {
        Self {
            max_retries: 3,
            backoff_strategy: BackoffStrategy::Exponential(Duration::from_millis(100)),
            retry_conditions: vec![
                RetryCondition::NetworkError,
                RetryCondition::TimeoutError,
                RetryCondition::TransientDatabaseError,
            ],
        }
    }

    /// Create a retry executor with custom settings
    pub fn with_config(
        max_retries: u32,
        backoff_strategy: BackoffStrategy,
        retry_conditions: Vec<RetryCondition>,
    ) -> Self {
        Self {
            max_retries,
            backoff_strategy,
            retry_conditions,
        }
    }

    /// Execute an operation with retry logic
    pub async fn execute<F, Fut, T>(&self, operation: F) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, TcpError>>,
    {
        let start_time = Instant::now();
        let mut attempts = 0;
        let mut last_error = None;

        loop {
            attempts += 1;

            debug!(
                attempt = attempts,
                max_retries = self.max_retries,
                "Executing operation attempt"
            );

            match operation().await {
                Ok(result) => {
                    if attempts > 1 {
                        info!(
                            attempts = attempts,
                            duration_ms = start_time.elapsed().as_millis(),
                            "Operation succeeded after retry"
                        );
                    }
                    return RetryResult {
                        result: Ok(result),
                        attempts,
                        total_duration: start_time.elapsed(),
                        max_retries_reached: false,
                    };
                }
                Err(error) => {
                    last_error = Some(error.clone());

                    // Check if we should retry this error
                    if !self.should_retry(&error) {
                        debug!(error = format!("{:?}", error), "Error is not retryable");
                        return RetryResult {
                            result: Err(error),
                            attempts,
                            total_duration: start_time.elapsed(),
                            max_retries_reached: false,
                        };
                    }

                    // Check if we've reached the maximum number of retries
                    if attempts >= self.max_retries {
                        warn!(
                            attempts = attempts,
                            error = format!("{:?}", error),
                            "Maximum retry attempts reached"
                        );
                        return RetryResult {
                            result: Err(error),
                            attempts,
                            total_duration: start_time.elapsed(),
                            max_retries_reached: true,
                        };
                    }

                    // Calculate delay before next retry
                    let delay = self.calculate_delay(attempts);
                    debug!(
                        attempt = attempts,
                        delay_ms = delay.as_millis(),
                        error = format!("{:?}", error),
                        "Retrying after delay"
                    );

                    sleep(delay).await;
                }
            }
        }
    }

    /// Execute an operation with retry logic and custom retry condition
    pub async fn execute_with_condition<F, Fut, T, C>(
        &self,
        operation: F,
        condition: C,
    ) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, TcpError>>,
        C: Fn(&TcpError) -> bool,
    {
        let start_time = Instant::now();
        let mut attempts = 0;

        loop {
            attempts += 1;

            debug!(
                attempt = attempts,
                max_retries = self.max_retries,
                "Executing operation attempt with custom condition"
            );

            match operation().await {
                Ok(result) => {
                    if attempts > 1 {
                        info!(
                            attempts = attempts,
                            duration_ms = start_time.elapsed().as_millis(),
                            "Operation succeeded after retry with custom condition"
                        );
                    }
                    return RetryResult {
                        result: Ok(result),
                        attempts,
                        total_duration: start_time.elapsed(),
                        max_retries_reached: false,
                    };
                }
                Err(error) => {
                    // Check custom condition
                    if !condition(&error) {
                        debug!(
                            error = format!("{:?}", error),
                            "Custom condition failed, not retrying"
                        );
                        return RetryResult {
                            result: Err(error),
                            attempts,
                            total_duration: start_time.elapsed(),
                            max_retries_reached: false,
                        };
                    }

                    // Check if we've reached the maximum number of retries
                    if attempts >= self.max_retries {
                        warn!(
                            attempts = attempts,
                            error = format!("{:?}", error),
                            "Maximum retry attempts reached with custom condition"
                        );
                        return RetryResult {
                            result: Err(error),
                            attempts,
                            total_duration: start_time.elapsed(),
                            max_retries_reached: true,
                        };
                    }

                    // Calculate delay before next retry
                    let delay = self.calculate_delay(attempts);
                    debug!(
                        attempt = attempts,
                        delay_ms = delay.as_millis(),
                        error = format!("{:?}", error),
                        "Retrying after delay with custom condition"
                    );

                    sleep(delay).await;
                }
            }
        }
    }

    /// Check if an error should trigger a retry
    fn should_retry(&self, error: &TcpError) -> bool {
        for condition in &self.retry_conditions {
            if self.error_matches_condition(error, condition) {
                return true;
            }
        }
        false
    }

    /// Check if an error matches a specific retry condition
    fn error_matches_condition(&self, error: &TcpError, condition: &RetryCondition) -> bool {
        match condition {
            RetryCondition::NetworkError => {
                matches!(
                    error,
                    TcpError::NetworkError(_) | TcpError::ConnectionFailed(_)
                )
            }
            RetryCondition::TimeoutError => {
                matches!(error, TcpError::TimeoutError(_))
            }
            RetryCondition::TransientDatabaseError => {
                matches!(
                    error,
                    TcpError::DatabaseError(_)
                        | TcpError::ConnectionFailed(_)
                        | TcpError::QueryFailed(_)
                )
            }
            RetryCondition::RateLimitExceeded => {
                matches!(error, TcpError::RateLimitExceeded(_))
            }
            RetryCondition::SystemOverload => {
                matches!(
                    error,
                    TcpError::SystemError(_)
                        | TcpError::ResourceExhausted(_)
                        | TcpError::ResourceUnavailable(_)
                )
            }
        }
    }

    /// Calculate the delay before the next retry attempt
    fn calculate_delay(&self, attempt: u32) -> Duration {
        match &self.backoff_strategy {
            BackoffStrategy::Fixed(duration) => *duration,
            BackoffStrategy::Linear(base_duration) => {
                Duration::from_millis(base_duration.as_millis() as u64 * attempt as u64)
            }
            BackoffStrategy::Exponential(base_duration) => {
                let multiplier = 2_u64.pow(attempt.saturating_sub(1));
                Duration::from_millis(base_duration.as_millis() as u64 * multiplier)
            }
        }
    }

    /// Get the maximum number of retries
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Get the backoff strategy
    pub fn backoff_strategy(&self) -> &BackoffStrategy {
        &self.backoff_strategy
    }

    /// Get the retry conditions
    pub fn retry_conditions(&self) -> &[RetryCondition] {
        &self.retry_conditions
    }
}

impl Default for RetryExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create a retry executor for network operations
pub fn network_retry_executor() -> RetryExecutor {
    RetryExecutor::with_config(
        3,
        BackoffStrategy::Exponential(Duration::from_millis(100)),
        vec![RetryCondition::NetworkError, RetryCondition::TimeoutError],
    )
}

/// Convenience function to create a retry executor for database operations
pub fn database_retry_executor() -> RetryExecutor {
    RetryExecutor::with_config(
        2,
        BackoffStrategy::Linear(Duration::from_millis(50)),
        vec![RetryCondition::TransientDatabaseError],
    )
}

/// Convenience function to create a retry executor for rate-limited operations
pub fn rate_limit_retry_executor() -> RetryExecutor {
    RetryExecutor::with_config(
        5,
        BackoffStrategy::Fixed(Duration::from_millis(200)),
        vec![RetryCondition::RateLimitExceeded],
    )
}

/// Macro for retrying operations with default retry executor
#[macro_export]
macro_rules! retry_operation {
    ($operation:expr) => {{
        let executor = $crate::server::error::retry::RetryExecutor::new();
        executor.execute(|| async { $operation }).await
    }};
}

/// Macro for retrying operations with custom retry executor
#[macro_export]
macro_rules! retry_operation_with {
    ($executor:expr, $operation:expr) => {{
        $executor.execute(|| async { $operation }).await
    }};
}

/// Macro for retrying operations with custom condition
#[macro_export]
macro_rules! retry_operation_if {
    ($operation:expr, $condition:expr) => {{
        let executor = $crate::server::error::retry::RetryExecutor::new();
        executor
            .execute_with_condition(|| async { $operation }, $condition)
            .await
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let executor = RetryExecutor::new();
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let result = executor
            .execute(|| async {
                let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err(TcpError::NetworkError("transient error".to_string()))
                } else {
                    Ok("success".to_string())
                }
            })
            .await;

        assert!(result.result.is_ok());
        assert_eq!(result.attempts, 3);
        assert!(!result.max_retries_reached);
    }

    #[tokio::test]
    async fn test_retry_exhaustion() {
        let executor = RetryExecutor::with_config(
            2,
            BackoffStrategy::Fixed(Duration::from_millis(1)),
            vec![RetryCondition::NetworkError],
        );

        let result = executor
            .execute(|| async { Err(TcpError::NetworkError("persistent error".to_string())) })
            .await;

        assert!(result.result.is_err());
        assert_eq!(result.attempts, 2);
        assert!(result.max_retries_reached);
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let executor = RetryExecutor::new();

        let result = executor
            .execute(|| async {
                Err(TcpError::AuthenticationFailed(
                    "invalid credentials".to_string(),
                ))
            })
            .await;

        assert!(result.result.is_err());
        assert_eq!(result.attempts, 1);
        assert!(!result.max_retries_reached);
    }

    #[tokio::test]
    async fn test_backoff_strategies() {
        let executor = RetryExecutor::with_config(
            3,
            BackoffStrategy::Exponential(Duration::from_millis(10)),
            vec![RetryCondition::NetworkError],
        );

        // Test delay calculation
        assert_eq!(executor.calculate_delay(1), Duration::from_millis(10));
        assert_eq!(executor.calculate_delay(2), Duration::from_millis(20));
        assert_eq!(executor.calculate_delay(3), Duration::from_millis(40));

        let executor = RetryExecutor::with_config(
            3,
            BackoffStrategy::Linear(Duration::from_millis(10)),
            vec![RetryCondition::NetworkError],
        );

        assert_eq!(executor.calculate_delay(1), Duration::from_millis(10));
        assert_eq!(executor.calculate_delay(2), Duration::from_millis(20));
        assert_eq!(executor.calculate_delay(3), Duration::from_millis(30));
    }

    #[tokio::test]
    async fn test_custom_condition() {
        let executor = RetryExecutor::new();

        let result = executor
            .execute_with_condition(
                || async {
                    Err(TcpError::AuthenticationFailed(
                        "custom retryable".to_string(),
                    ))
                },
                |error| matches!(error, TcpError::AuthenticationFailed(_)),
            )
            .await;

        assert!(result.result.is_err());
        assert_eq!(result.attempts, 3); // Should retry despite normally not being retryable
        assert!(result.max_retries_reached);
    }

    #[tokio::test]
    async fn test_convenience_executors() {
        let network_executor = network_retry_executor();
        assert_eq!(network_executor.max_retries(), 3);
        assert!(matches!(
            network_executor.backoff_strategy(),
            BackoffStrategy::Exponential(_)
        ));

        let db_executor = database_retry_executor();
        assert_eq!(db_executor.max_retries(), 2);
        assert!(matches!(
            db_executor.backoff_strategy(),
            BackoffStrategy::Linear(_)
        ));

        let rate_limit_executor = rate_limit_retry_executor();
        assert_eq!(rate_limit_executor.max_retries(), 5);
        assert!(matches!(
            rate_limit_executor.backoff_strategy(),
            BackoffStrategy::Fixed(_)
        ));
    }
}
