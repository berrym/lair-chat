//! Input validation framework for the TCP server
//!
//! This module provides comprehensive input validation with sanitization,
//! rate limiting, and security checks for all TCP commands.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing;

pub mod rules;
pub use rules::*;

use crate::server::error::ValidationError;
use crate::server::storage::current_timestamp;

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Validated input with sanitized data
#[derive(Debug, Clone)]
pub struct ValidatedInput {
    pub command: String,
    pub arguments: Vec<String>,
    pub raw_input: String,
    pub sanitized_input: String,
    pub user_id: Option<String>,
    pub timestamp: i64,
}

/// Command validation framework
pub trait CommandValidator {
    /// Validate input format and structure
    fn validate_input(&self, input: &str) -> ValidationResult<ValidatedInput>;

    /// Sanitize input to prevent injection attacks
    fn sanitize_input(&self, input: &str) -> String;

    /// Check rate limits for the user
    fn check_rate_limit(&self, user_id: &str, command: &str) -> ValidationResult<()>;

    /// Validate user permissions for the command
    fn validate_permissions(&self, user_id: &str, command: &ValidatedInput)
        -> ValidationResult<()>;

    /// Validate command-specific parameters
    fn validate_command_params(&self, command: &ValidatedInput) -> ValidationResult<()>;
}

/// Main validation system
pub struct ValidationSystem {
    /// Rate limiter for commands
    rate_limiter: Arc<RwLock<RateLimiter>>,
    /// Command validators
    validators: HashMap<String, Box<dyn CommandValidator + Send + Sync>>,
    /// Validation statistics
    stats: Arc<RwLock<ValidationStats>>,
}

/// Rate limiter for preventing abuse
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Rate limits per user per command
    user_limits: HashMap<String, HashMap<String, RateLimit>>,
    /// Global rate limits
    global_limits: HashMap<String, RateLimit>,
    /// Configuration
    config: RateLimitConfig,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Default requests per minute per user
    pub default_user_rpm: u32,
    /// Default requests per minute globally
    pub default_global_rpm: u32,
    /// Window size for rate limiting
    pub window_size: Duration,
    /// Burst allowance
    pub burst_allowance: u32,
}

/// Rate limit tracking
#[derive(Debug, Clone)]
pub struct RateLimit {
    /// Number of requests in current window
    pub count: u32,
    /// Window start time
    pub window_start: Instant,
    /// Configured limit
    pub limit: u32,
    /// Last request time
    pub last_request: Instant,
}

/// Validation statistics
#[derive(Debug, Clone, Default)]
pub struct ValidationStats {
    pub total_validations: u64,
    pub successful_validations: u64,
    pub failed_validations: u64,
    pub rate_limit_violations: u64,
    pub security_violations: u64,
    pub validation_errors_by_type: HashMap<String, u64>,
    pub commands_by_type: HashMap<String, u64>,
}

/// Security validation middleware
pub struct SecurityValidator {
    /// Suspicious pattern detection
    suspicious_patterns: Vec<String>,
    /// Blocked words/phrases
    blocked_content: Vec<String>,
    /// Maximum input lengths
    max_lengths: HashMap<String, usize>,
}

impl ValidationSystem {
    /// Create a new validation system
    pub fn new() -> Self {
        let config = RateLimitConfig {
            default_user_rpm: 60,
            default_global_rpm: 1000,
            window_size: Duration::from_secs(60),
            burst_allowance: 10,
        };

        Self {
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new(config))),
            validators: HashMap::new(),
            stats: Arc::new(RwLock::new(ValidationStats::default())),
        }
    }

    /// Register a command validator
    pub fn register_validator<V>(&mut self, command: String, validator: V)
    where
        V: CommandValidator + Send + Sync + 'static,
    {
        self.validators.insert(command, Box::new(validator));
    }

    /// Validate input with comprehensive checks
    pub async fn validate_input(
        &self,
        input: &str,
        user_id: Option<&str>,
    ) -> ValidationResult<ValidatedInput> {
        // Update statistics
        self.update_validation_stats(true).await;

        // Basic format validation
        let validated = self.validate_basic_format(input)?;

        // Command-specific validation
        if let Some(validator) = self.validators.get(&validated.command) {
            validator.validate_input(input)?;
            validator.validate_command_params(&validated)?;
        }

        // Rate limiting check
        if let Some(uid) = user_id {
            self.check_rate_limit(uid, &validated.command).await?;
        }

        // Security validation
        self.validate_security(&validated).await?;

        // Update success statistics
        self.update_command_stats(&validated.command).await;

        Ok(validated)
    }

    /// Validate basic input format
    fn validate_basic_format(&self, input: &str) -> ValidationResult<ValidatedInput> {
        // Check for empty input
        if input.trim().is_empty() {
            return Err(ValidationError::RequiredFieldMissing("command".to_string()));
        }

        // Check input length
        if input.len() > 1024 {
            return Err(ValidationError::InvalidLength(
                "Input too long (max 1024 characters)".to_string(),
            ));
        }

        // Parse command and arguments
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Err(ValidationError::InvalidFormat(
                "No command provided".to_string(),
            ));
        }

        let command = parts[0].to_uppercase();
        let arguments: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        // Sanitize input
        let sanitized = self.sanitize_input(input);

        Ok(ValidatedInput {
            command,
            arguments,
            raw_input: input.to_string(),
            sanitized_input: sanitized,
            user_id: None,
            timestamp: current_timestamp(),
        })
    }

    /// Sanitize input to prevent injection attacks
    fn sanitize_input(&self, input: &str) -> String {
        // Remove potentially dangerous characters
        let sanitized = input
            .chars()
            .filter(|c| {
                c.is_alphanumeric()
                    || c.is_whitespace()
                    || "!@#$%^&*()_+-=[]{}|;':\",./<>?".contains(*c)
            })
            .collect::<String>();

        // Trim whitespace
        sanitized.trim().to_string()
    }

    /// Check rate limits for user and command
    async fn check_rate_limit(&self, user_id: &str, command: &str) -> ValidationResult<()> {
        let mut rate_limiter = self.rate_limiter.write().await;

        if !rate_limiter.check_rate_limit(user_id, command) {
            self.update_rate_limit_stats().await;
            return Err(ValidationError::RateLimitExceeded(format!(
                "Rate limit exceeded for command: {}",
                command
            )));
        }

        Ok(())
    }

    /// Validate security aspects of the input
    async fn validate_security(&self, input: &ValidatedInput) -> ValidationResult<()> {
        let security_validator = SecurityValidator::new();

        // Check for suspicious patterns
        if security_validator.has_suspicious_patterns(&input.sanitized_input) {
            self.update_security_violation_stats().await;
            return Err(ValidationError::SecurityViolation(
                "Suspicious pattern detected".to_string(),
            ));
        }

        // Check for blocked content
        if security_validator.has_blocked_content(&input.sanitized_input) {
            self.update_security_violation_stats().await;
            return Err(ValidationError::SecurityViolation(
                "Blocked content detected".to_string(),
            ));
        }

        // Check input length limits
        if let Some(max_length) = security_validator.max_lengths.get(&input.command) {
            if input.sanitized_input.len() > *max_length {
                return Err(ValidationError::InvalidLength(format!(
                    "Input too long for command {} (max {} characters)",
                    input.command, max_length
                )));
            }
        }

        Ok(())
    }

    /// Update validation statistics
    async fn update_validation_stats(&self, success: bool) {
        let mut stats = self.stats.write().await;
        stats.total_validations += 1;

        if success {
            stats.successful_validations += 1;
        } else {
            stats.failed_validations += 1;
        }
    }

    /// Update command statistics
    async fn update_command_stats(&self, command: &str) {
        let mut stats = self.stats.write().await;
        *stats
            .commands_by_type
            .entry(command.to_string())
            .or_insert(0) += 1;
    }

    /// Update rate limit statistics
    async fn update_rate_limit_stats(&self) {
        let mut stats = self.stats.write().await;
        stats.rate_limit_violations += 1;
    }

    /// Update security violation statistics
    async fn update_security_violation_stats(&self) {
        let mut stats = self.stats.write().await;
        stats.security_violations += 1;
    }

    /// Get validation statistics
    pub async fn get_stats(&self) -> ValidationStats {
        self.stats.read().await.clone()
    }

    /// Reset validation statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = ValidationStats::default();
    }
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            user_limits: HashMap::new(),
            global_limits: HashMap::new(),
            config,
        }
    }

    /// Check if user can execute command within rate limits
    pub fn check_rate_limit(&mut self, user_id: &str, command: &str) -> bool {
        let now = Instant::now();

        // Check user-specific rate limit
        if !self.check_user_rate_limit(user_id, command, now) {
            return false;
        }

        // Check global rate limit
        if !self.check_global_rate_limit(command, now) {
            return false;
        }

        true
    }

    /// Check user-specific rate limit
    fn check_user_rate_limit(&mut self, user_id: &str, command: &str, now: Instant) -> bool {
        let user_limits = self
            .user_limits
            .entry(user_id.to_string())
            .or_insert_with(HashMap::new);

        let rate_limit = user_limits
            .entry(command.to_string())
            .or_insert_with(|| RateLimit {
                count: 0,
                window_start: now,
                limit: self.config.default_user_rpm,
                last_request: now,
            });

        self.update_rate_limit(rate_limit, now)
    }

    /// Check global rate limit
    fn check_global_rate_limit(&mut self, command: &str, now: Instant) -> bool {
        let rate_limit = self
            .global_limits
            .entry(command.to_string())
            .or_insert_with(|| RateLimit {
                count: 0,
                window_start: now,
                limit: self.config.default_global_rpm,
                last_request: now,
            });

        self.update_rate_limit(rate_limit, now)
    }

    /// Update rate limit and check if request is allowed
    fn update_rate_limit(&mut self, rate_limit: &mut RateLimit, now: Instant) -> bool {
        // Reset window if needed
        if now.duration_since(rate_limit.window_start) >= self.config.window_size {
            rate_limit.count = 0;
            rate_limit.window_start = now;
        }

        // Check if limit is exceeded
        if rate_limit.count >= rate_limit.limit {
            // Allow burst if within burst allowance and enough time has passed
            let time_since_last = now.duration_since(rate_limit.last_request);
            if rate_limit.count < rate_limit.limit + self.config.burst_allowance
                && time_since_last >= Duration::from_secs(1)
            {
                rate_limit.count += 1;
                rate_limit.last_request = now;
                return true;
            }
            return false;
        }

        // Allow request
        rate_limit.count += 1;
        rate_limit.last_request = now;
        true
    }

    /// Set custom rate limit for user and command
    pub fn set_user_rate_limit(&mut self, user_id: &str, command: &str, limit: u32) {
        let user_limits = self
            .user_limits
            .entry(user_id.to_string())
            .or_insert_with(HashMap::new);
        let rate_limit = user_limits
            .entry(command.to_string())
            .or_insert_with(|| RateLimit {
                count: 0,
                window_start: Instant::now(),
                limit,
                last_request: Instant::now(),
            });
        rate_limit.limit = limit;
    }

    /// Set global rate limit for command
    pub fn set_global_rate_limit(&mut self, command: &str, limit: u32) {
        let rate_limit = self
            .global_limits
            .entry(command.to_string())
            .or_insert_with(|| RateLimit {
                count: 0,
                window_start: Instant::now(),
                limit,
                last_request: Instant::now(),
            });
        rate_limit.limit = limit;
    }
}

impl SecurityValidator {
    /// Create a new security validator
    pub fn new() -> Self {
        Self {
            suspicious_patterns: vec![
                "script".to_string(),
                "eval".to_string(),
                "exec".to_string(),
                "system".to_string(),
                "rm -rf".to_string(),
                "DROP TABLE".to_string(),
                "DELETE FROM".to_string(),
                "INSERT INTO".to_string(),
                "UPDATE SET".to_string(),
                "../".to_string(),
                "..\\".to_string(),
            ],
            blocked_content: vec![
                "password".to_string(),
                "secret".to_string(),
                "private_key".to_string(),
                "api_key".to_string(),
            ],
            max_lengths: [
                ("MESSAGE".to_string(), 1000),
                ("REGISTER".to_string(), 100),
                ("LOGIN".to_string(), 100),
                ("CREATE_ROOM".to_string(), 200),
                ("JOIN_ROOM".to_string(), 100),
                ("INVITE_USER".to_string(), 100),
            ]
            .iter()
            .cloned()
            .collect(),
        }
    }

    /// Check for suspicious patterns in input
    pub fn has_suspicious_patterns(&self, input: &str) -> bool {
        let input_lower = input.to_lowercase();
        self.suspicious_patterns
            .iter()
            .any(|pattern| input_lower.contains(pattern))
    }

    /// Check for blocked content in input
    pub fn has_blocked_content(&self, input: &str) -> bool {
        let input_lower = input.to_lowercase();
        self.blocked_content
            .iter()
            .any(|content| input_lower.contains(content))
    }
}

impl Default for ValidationSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            default_user_rpm: 60,
            default_global_rpm: 1000,
            window_size: Duration::from_secs(60),
            burst_allowance: 10,
        }
    }
}

/// Global validation system instance
static VALIDATION_SYSTEM: std::sync::OnceLock<ValidationSystem> = std::sync::OnceLock::new();

/// Get the global validation system
pub fn get_validation_system() -> &'static ValidationSystem {
    VALIDATION_SYSTEM.get_or_init(|| ValidationSystem::new())
}

/// Initialize the global validation system
pub fn init_validation_system() -> &'static ValidationSystem {
    get_validation_system()
}

/// Convenience macro for validating input
#[macro_export]
macro_rules! validate_input {
    ($input:expr) => {
        $crate::server::validation::get_validation_system()
            .validate_input($input, None)
            .await
    };
    ($input:expr, $user_id:expr) => {
        $crate::server::validation::get_validation_system()
            .validate_input($input, Some($user_id))
            .await
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_system_creation() {
        let system = ValidationSystem::new();
        let stats = system.get_stats().await;
        assert_eq!(stats.total_validations, 0);
    }

    #[tokio::test]
    async fn test_basic_validation() {
        let system = ValidationSystem::new();
        let result = system.validate_input("MESSAGE hello world", None).await;
        assert!(result.is_ok());

        let validated = result.unwrap();
        assert_eq!(validated.command, "MESSAGE");
        assert_eq!(validated.arguments, vec!["hello", "world"]);
    }

    #[tokio::test]
    async fn test_validation_errors() {
        let system = ValidationSystem::new();

        // Empty input
        let result = system.validate_input("", None).await;
        assert!(result.is_err());

        // Too long input
        let long_input = "A".repeat(2000);
        let result = system.validate_input(&long_input, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let mut rate_limiter = RateLimiter::new(RateLimitConfig {
            default_user_rpm: 2,
            default_global_rpm: 10,
            window_size: Duration::from_secs(60),
            burst_allowance: 1,
        });

        // First request should succeed
        assert!(rate_limiter.check_rate_limit("user1", "MESSAGE"));

        // Second request should succeed
        assert!(rate_limiter.check_rate_limit("user1", "MESSAGE"));

        // Third request should fail (exceeds limit)
        assert!(!rate_limiter.check_rate_limit("user1", "MESSAGE"));
    }

    #[tokio::test]
    async fn test_security_validation() {
        let validator = SecurityValidator::new();

        // Test suspicious patterns
        assert!(validator.has_suspicious_patterns("DROP TABLE users"));
        assert!(validator.has_suspicious_patterns("rm -rf /"));
        assert!(!validator.has_suspicious_patterns("hello world"));

        // Test blocked content
        assert!(validator.has_blocked_content("my password is secret"));
        assert!(!validator.has_blocked_content("hello world"));
    }

    #[tokio::test]
    async fn test_input_sanitization() {
        let system = ValidationSystem::new();
        let sanitized = system.sanitize_input("  hello\nworld\t  ");
        assert_eq!(sanitized, "hello world");
    }
}
