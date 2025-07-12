//! Error handling framework for the TCP server
//!
//! This module provides comprehensive error handling with structured error types,
//! logging, metrics collection, and recovery mechanisms.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn, Level};

pub mod retry;
pub mod types;
pub use retry::*;
pub use types::*;

use crate::server::storage::current_timestamp;

/// Result type for TCP operations
pub type TcpResult<T> = Result<T, TcpError>;

/// Error handler for managing errors across the TCP server
pub struct ErrorHandler {
    /// Error statistics
    stats: Arc<RwLock<ErrorStats>>,
    /// Recovery policies
    recovery_policies: HashMap<String, RetryPolicy>,
    /// Circuit breakers for error-prone operations
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    retry_executor: RetryExecutor,
}

/// Error statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct ErrorStats {
    pub total_errors: u64,
    pub errors_by_type: HashMap<String, u64>,
    pub errors_by_severity: HashMap<String, u64>,
    pub recovery_attempts: u64,
    pub successful_recoveries: u64,
    pub failed_recoveries: u64,
    pub last_error_time: Option<u64>,
}

/// Circuit breaker for preventing cascading failures
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Current state of the circuit breaker
    state: CircuitBreakerState,
    /// Number of failures in the current window
    failure_count: u32,
    /// Failure threshold before opening the circuit
    failure_threshold: u32,
    /// Timeout duration for the circuit breaker
    timeout_duration: Duration,
    /// Last failure time
    last_failure_time: Option<Instant>,
    /// Next attempt time (for half-open state)
    next_attempt_time: Option<Instant>,
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Error response for TCP clients
#[derive(Debug, Clone)]
pub struct ErrorResponse {
    pub error_code: String,
    pub message: String,
    pub details: Option<String>,
    pub timestamp: u64,
    pub request_id: Option<String>,
}

impl ErrorHandler {
    /// Create a new error handler
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(ErrorStats::default())),
            recovery_policies: HashMap::new(),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            retry_executor: RetryExecutor::new(),
        }
    }

    /// Handle an error with logging, metrics, and recovery
    pub async fn handle_error(
        &self,
        error: TcpError,
        context: Option<ErrorContext>,
    ) -> ErrorResponse {
        // Log the error
        self.log_error(&error, context.as_ref()).await;

        // Update statistics
        self.update_stats(&error).await;

        // Attempt recovery if applicable
        if let Some(recovery_action) = error.recovery_action() {
            self.attempt_recovery(&error, &recovery_action).await;
        }

        // Create response
        self.create_error_response(&error, context)
    }

    /// Log error with appropriate level and context
    async fn log_error(&self, error: &TcpError, context: Option<&ErrorContext>) {
        let level = error.log_level();
        let error_code = error.error_code();
        let message = error.user_message();

        let mut fields = vec![
            ("error_code", error_code.to_string()),
            ("error_type", format!("{:?}", error)),
            ("severity", format!("{:?}", error.severity())),
        ];

        if let Some(ctx) = context {
            fields.push(("operation", ctx.operation.clone()));
            if let Some(user_id) = &ctx.user_id {
                fields.push(("user_id", user_id.clone()));
            }
            if let Some(room_id) = &ctx.room_id {
                fields.push(("room_id", room_id.clone()));
            }
        }

        match level {
            Level::ERROR => {
                error!(
                    error_code = error_code,
                    error_type = format!("{:?}", error),
                    severity = format!("{:?}", error.severity()),
                    context = format!("{:?}", context),
                    "TCP Error: {}",
                    message
                );
            }
            Level::WARN => {
                warn!(
                    error_code = error_code,
                    error_type = format!("{:?}", error),
                    severity = format!("{:?}", error.severity()),
                    context = format!("{:?}", context),
                    "TCP Warning: {}",
                    message
                );
            }
            Level::INFO => {
                info!(
                    error_code = error_code,
                    error_type = format!("{:?}", error),
                    severity = format!("{:?}", error.severity()),
                    context = format!("{:?}", context),
                    "TCP Info: {}",
                    message
                );
            }
            Level::DEBUG => {
                debug!(
                    error_code = error_code,
                    error_type = format!("{:?}", error),
                    severity = format!("{:?}", error.severity()),
                    context = format!("{:?}", context),
                    "TCP Debug: {}",
                    message
                );
            }
            _ => {
                info!(
                    error_code = error_code,
                    error_type = format!("{:?}", error),
                    severity = format!("{:?}", error.severity()),
                    context = format!("{:?}", context),
                    "TCP: {}",
                    message
                );
            }
        }
    }

    /// Update error statistics
    async fn update_stats(&self, error: &TcpError) {
        let mut stats = self.stats.write().await;
        stats.total_errors += 1;

        let error_type = format!("{:?}", error);
        *stats.errors_by_type.entry(error_type).or_insert(0) += 1;

        let severity = format!("{:?}", error.severity());
        *stats.errors_by_severity.entry(severity).or_insert(0) += 1;

        stats.last_error_time = Some(current_timestamp());
    }

    /// Attempt error recovery
    async fn attempt_recovery(&self, error: &TcpError, recovery_action: &RecoveryAction) {
        let mut stats = self.stats.write().await;
        stats.recovery_attempts += 1;

        match recovery_action {
            RecoveryAction::Retry(policy) => {
                debug!(
                    error_code = error.error_code(),
                    max_retries = policy.max_retries,
                    backoff_strategy = format!("{:?}", policy.backoff_strategy),
                    "Attempting error recovery with retry policy"
                );
                // Retry logic would be implemented by the caller
            }
            RecoveryAction::Fallback(fallback_msg) => {
                info!(
                    error_code = error.error_code(),
                    fallback = fallback_msg,
                    "Using fallback recovery strategy"
                );
            }
            RecoveryAction::Disconnect => {
                warn!(
                    error_code = error.error_code(),
                    "Recovery action requires client disconnection"
                );
            }
            RecoveryAction::RateLimitDelay(duration) => {
                info!(
                    error_code = error.error_code(),
                    delay_ms = duration.as_millis(),
                    "Rate limit recovery delay applied"
                );
            }
            RecoveryAction::Authenticate => {
                info!(
                    error_code = error.error_code(),
                    "Recovery requires re-authentication"
                );
            }
            _ => {
                debug!(
                    error_code = error.error_code(),
                    recovery_action = format!("{:?}", recovery_action),
                    "Recovery action logged"
                );
            }
        }
    }

    /// Create error response for client
    fn create_error_response(
        &self,
        error: &TcpError,
        context: Option<ErrorContext>,
    ) -> ErrorResponse {
        ErrorResponse {
            error_code: error.error_code().to_string(),
            message: error.user_message(),
            details: None,
            timestamp: current_timestamp(),
            request_id: context.and_then(|ctx| ctx.additional_info.get("request_id").cloned()),
        }
    }

    /// Get error statistics
    pub async fn get_stats(&self) -> ErrorStats {
        self.stats.read().await.clone()
    }

    /// Reset error statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = ErrorStats::default();
    }

    /// Get or create circuit breaker for an operation
    pub async fn get_circuit_breaker(&self, operation: &str) -> CircuitBreaker {
        let circuit_breakers = self.circuit_breakers.read().await;
        circuit_breakers.get(operation).cloned().unwrap_or_else(|| {
            CircuitBreaker::new(5, Duration::from_secs(30)) // Default: 5 failures, 30s timeout
        })
    }

    /// Update circuit breaker state
    pub async fn update_circuit_breaker(&self, operation: &str, circuit_breaker: CircuitBreaker) {
        let mut circuit_breakers = self.circuit_breakers.write().await;
        circuit_breakers.insert(operation.to_string(), circuit_breaker);
    }

    /// Check if operation should be executed based on circuit breaker state
    pub async fn should_execute_operation(&self, operation: &str) -> bool {
        let circuit_breaker = self.get_circuit_breaker(operation).await;
        circuit_breaker.can_execute()
    }

    /// Record operation success for circuit breaker
    pub async fn record_success(&self, operation: &str) {
        let mut circuit_breaker = self.get_circuit_breaker(operation).await;
        circuit_breaker.record_success();
        self.update_circuit_breaker(operation, circuit_breaker)
            .await;
    }

    /// Record operation failure for circuit breaker
    pub async fn record_failure(&self, operation: &str) {
        let mut circuit_breaker = self.get_circuit_breaker(operation).await;
        circuit_breaker.record_failure();
        self.update_circuit_breaker(operation, circuit_breaker)
            .await;
    }

    /// Execute operation with retry logic
    pub async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, TcpError>>,
    {
        self.retry_executor.execute(operation).await
    }

    /// Execute operation with custom retry executor
    pub async fn execute_with_custom_retry<F, Fut, T>(
        &self,
        executor: &RetryExecutor,
        operation: F,
    ) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, TcpError>>,
    {
        executor.execute(operation).await
    }

    /// Execute operation with circuit breaker and retry
    pub async fn execute_with_circuit_breaker<F, Fut, T>(
        &self,
        operation_name: &str,
        operation: F,
    ) -> Result<T, TcpError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, TcpError>>,
    {
        // Check circuit breaker
        if !self.should_execute_operation(operation_name).await {
            return Err(TcpError::SystemError("Circuit breaker is open".to_string()));
        }

        // Execute with retry
        let result = self.retry_executor.execute(operation).await;

        // Update circuit breaker based on result
        match &result.result {
            Ok(_) => {
                self.record_success(operation_name).await;
            }
            Err(_) => {
                self.record_failure(operation_name).await;
            }
        }

        result.result
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(failure_threshold: u32, timeout_duration: Duration) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            failure_threshold,
            timeout_duration,
            last_failure_time: None,
            next_attempt_time: None,
        }
    }

    /// Check if the circuit breaker allows execution
    pub fn can_execute(&self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                if let Some(next_attempt) = self.next_attempt_time {
                    Instant::now() >= next_attempt
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    /// Record a successful operation
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitBreakerState::Closed;
        self.next_attempt_time = None;
    }

    /// Record a failed operation
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());

        if self.failure_count >= self.failure_threshold {
            self.state = CircuitBreakerState::Open;
            self.next_attempt_time = Some(Instant::now() + self.timeout_duration);
        } else if self.state == CircuitBreakerState::HalfOpen {
            self.state = CircuitBreakerState::Open;
            self.next_attempt_time = Some(Instant::now() + self.timeout_duration);
        }
    }

    /// Get current state
    pub fn state(&self) -> CircuitBreakerState {
        self.state.clone()
    }

    /// Get failure count
    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }
}

impl ErrorResponse {
    /// Convert to TCP protocol message
    pub fn to_tcp_message(&self) -> String {
        format!(
            "ERROR:{}:{}:{}",
            self.error_code, self.message, self.timestamp
        )
    }

    /// Convert to system message format
    pub fn to_system_message(&self) -> String {
        format!("SYSTEM_MESSAGE:ERROR: {}", self.message)
    }
}

/// Convenience macro for creating error context
#[macro_export]
macro_rules! error_context {
    ($operation:expr) => {
        ErrorContext {
            operation: $operation.to_string(),
            user_id: None,
            room_id: None,
            timestamp: current_timestamp(),
            additional_info: std::collections::HashMap::new(),
        }
    };
    ($operation:expr, user_id = $user_id:expr) => {
        ErrorContext {
            operation: $operation.to_string(),
            user_id: Some($user_id.to_string()),
            room_id: None,
            timestamp: current_timestamp(),
            additional_info: std::collections::HashMap::new(),
        }
    };
    ($operation:expr, user_id = $user_id:expr, room_id = $room_id:expr) => {
        ErrorContext {
            operation: $operation.to_string(),
            user_id: Some($user_id.to_string()),
            room_id: Some($room_id.to_string()),
            timestamp: current_timestamp(),
            additional_info: std::collections::HashMap::new(),
        }
    };
}

/// Convenience macro for handling errors with context
#[macro_export]
macro_rules! handle_error {
    ($error_handler:expr, $error:expr, $context:expr) => {
        $error_handler.handle_error($error, Some($context)).await
    };
    ($error_handler:expr, $error:expr) => {
        $error_handler.handle_error($error, None).await
    };
}

/// Global error handler instance
static ERROR_HANDLER: std::sync::OnceLock<ErrorHandler> = std::sync::OnceLock::new();

/// Get the global error handler
pub fn get_error_handler() -> &'static ErrorHandler {
    ERROR_HANDLER.get_or_init(|| ErrorHandler::new())
}

/// Initialize the global error handler
pub fn init_error_handler() -> &'static ErrorHandler {
    get_error_handler()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_error_handler_creation() {
        let handler = ErrorHandler::new();
        let stats = handler.get_stats().await;
        assert_eq!(stats.total_errors, 0);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let handler = ErrorHandler::new();
        let error = TcpError::ValidationError(ValidationError::InvalidFormat("test".to_string()));
        let context = error_context!("test_operation", user_id = "user123");

        let response = handler.handle_error(error, Some(context)).await;
        assert_eq!(response.error_code, "1101");
        assert!(response.message.contains("Invalid format"));
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let mut breaker = CircuitBreaker::new(2, Duration::from_secs(1));

        // Should allow execution initially
        assert!(breaker.can_execute());

        // Record failures
        breaker.record_failure();
        assert!(breaker.can_execute());

        breaker.record_failure();
        assert!(!breaker.can_execute()); // Should be open now

        // Record success should close the circuit
        breaker.record_success();
        assert!(breaker.can_execute());
    }

    #[tokio::test]
    async fn test_error_stats() {
        let handler = ErrorHandler::new();
        let error = TcpError::AuthenticationFailed("test".to_string());

        handler.handle_error(error, None).await;

        let stats = handler.get_stats().await;
        assert_eq!(stats.total_errors, 1);
        assert!(stats.errors_by_type.contains_key("AuthenticationFailed"));
    }
}
