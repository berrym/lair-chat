use std::fmt;
use std::time::Duration;
use tracing::Level;

use crate::server::storage::StorageError;

/// Comprehensive TCP error types with structured error handling
#[derive(Debug, Clone)]
pub enum TcpError {
    // Authentication errors
    AuthenticationFailed(String),
    AuthorizationDenied(String),
    SessionExpired(String),
    InvalidCredentials(String),
    AccountLocked(String),

    // Validation errors
    ValidationError(ValidationError),
    RateLimitExceeded(String),
    InvalidInput(String),
    InvalidFormat(String),
    InvalidLength(String),
    InvalidCharacters(String),

    // Database errors
    DatabaseError(DatabaseError),
    TransactionFailed(String),
    DataIntegrityError(String),
    ConnectionFailed(String),
    QueryFailed(String),

    // System errors
    SystemError(String),
    NetworkError(String),
    TimeoutError(String),
    InternalError(String),
    ConfigurationError(String),

    // Security errors
    SecurityViolation(String),
    IntrusionDetected(String),
    SuspiciousActivity(String),
    AccessDenied(String),
    SecurityBreach(String),

    // Resource errors
    ResourceNotFound(String),
    ResourceUnavailable(String),
    ResourceExhausted(String),
    PermissionDenied(String),

    // Protocol errors
    ProtocolError(String),
    MalformedRequest(String),
    UnsupportedOperation(String),
    VersionMismatch(String),
}

/// Validation error subtypes
#[derive(Debug, Clone)]
pub enum ValidationError {
    InvalidFormat(String),
    InvalidLength(String),
    InvalidCharacters(String),
    RateLimitExceeded(String),
    PermissionDenied(String),
    SecurityViolation(String),
    RequiredFieldMissing(String),
    InvalidValue(String),
    ConflictingValues(String),
}

/// Database error subtypes
#[derive(Debug, Clone)]
pub enum DatabaseError {
    ConnectionFailed(String),
    QueryFailed(String),
    TransactionFailed(String),
    DataIntegrityError(String),
    UniqueConstraintViolation(String),
    ForeignKeyConstraintViolation(String),
    SerializationError(String),
    DeserializationError(String),
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Recovery actions for error handling
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    Retry(RetryPolicy),
    Fallback(String),
    Disconnect,
    Ignore,
    Escalate,
    Authenticate,
    RateLimitDelay(Duration),
}

/// Retry policy for error recovery
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub backoff_strategy: BackoffStrategy,
    pub retry_conditions: Vec<RetryCondition>,
}

/// Backoff strategies for retry logic
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    Linear(Duration),
    Exponential(Duration),
    Fixed(Duration),
}

/// Conditions that determine if a retry should be attempted
#[derive(Debug, Clone)]
pub enum RetryCondition {
    NetworkError,
    TimeoutError,
    TransientDatabaseError,
    RateLimitExceeded,
    SystemOverload,
}

/// Error context for additional information
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub user_id: Option<String>,
    pub room_id: Option<String>,
    pub timestamp: i64,
    pub additional_info: std::collections::HashMap<String, String>,
}

impl TcpError {
    /// Get the error code for this error type
    pub fn error_code(&self) -> &str {
        match self {
            // Authentication errors (1000-1099)
            TcpError::AuthenticationFailed(_) => "1001",
            TcpError::AuthorizationDenied(_) => "1002",
            TcpError::SessionExpired(_) => "1003",
            TcpError::InvalidCredentials(_) => "1004",
            TcpError::AccountLocked(_) => "1005",

            // Validation errors (1100-1199)
            TcpError::ValidationError(ValidationError::InvalidFormat(_)) => "1101",
            TcpError::ValidationError(ValidationError::InvalidLength(_)) => "1102",
            TcpError::ValidationError(ValidationError::InvalidCharacters(_)) => "1103",
            TcpError::ValidationError(ValidationError::RateLimitExceeded(_)) => "1104",
            TcpError::ValidationError(ValidationError::PermissionDenied(_)) => "1105",
            TcpError::ValidationError(ValidationError::SecurityViolation(_)) => "1106",
            TcpError::ValidationError(ValidationError::RequiredFieldMissing(_)) => "1107",
            TcpError::ValidationError(ValidationError::InvalidValue(_)) => "1108",
            TcpError::ValidationError(ValidationError::ConflictingValues(_)) => "1109",
            TcpError::RateLimitExceeded(_) => "1110",
            TcpError::InvalidInput(_) => "1111",
            TcpError::InvalidFormat(_) => "1112",
            TcpError::InvalidLength(_) => "1113",
            TcpError::InvalidCharacters(_) => "1114",

            // Database errors (1200-1299)
            TcpError::DatabaseError(DatabaseError::ConnectionFailed(_)) => "1201",
            TcpError::DatabaseError(DatabaseError::QueryFailed(_)) => "1202",
            TcpError::DatabaseError(DatabaseError::TransactionFailed(_)) => "1203",
            TcpError::DatabaseError(DatabaseError::DataIntegrityError(_)) => "1204",
            TcpError::DatabaseError(DatabaseError::UniqueConstraintViolation(_)) => "1205",
            TcpError::DatabaseError(DatabaseError::ForeignKeyConstraintViolation(_)) => "1206",
            TcpError::DatabaseError(DatabaseError::SerializationError(_)) => "1207",
            TcpError::DatabaseError(DatabaseError::DeserializationError(_)) => "1208",
            TcpError::TransactionFailed(_) => "1210",
            TcpError::DataIntegrityError(_) => "1211",
            TcpError::ConnectionFailed(_) => "1212",
            TcpError::QueryFailed(_) => "1213",

            // System errors (1300-1399)
            TcpError::SystemError(_) => "1301",
            TcpError::NetworkError(_) => "1302",
            TcpError::TimeoutError(_) => "1303",
            TcpError::InternalError(_) => "1304",
            TcpError::ConfigurationError(_) => "1305",

            // Security errors (1400-1499)
            TcpError::SecurityViolation(_) => "1401",
            TcpError::IntrusionDetected(_) => "1402",
            TcpError::SuspiciousActivity(_) => "1403",
            TcpError::AccessDenied(_) => "1404",
            TcpError::SecurityBreach(_) => "1405",

            // Resource errors (1500-1599)
            TcpError::ResourceNotFound(_) => "1501",
            TcpError::ResourceUnavailable(_) => "1502",
            TcpError::ResourceExhausted(_) => "1503",
            TcpError::PermissionDenied(_) => "1504",

            // Protocol errors (1600-1699)
            TcpError::ProtocolError(_) => "1601",
            TcpError::MalformedRequest(_) => "1602",
            TcpError::UnsupportedOperation(_) => "1603",
            TcpError::VersionMismatch(_) => "1604",
        }
    }

    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            // Authentication errors
            TcpError::AuthenticationFailed(msg) => format!("Authentication failed: {}", msg),
            TcpError::AuthorizationDenied(msg) => format!("Access denied: {}", msg),
            TcpError::SessionExpired(_) => {
                "Your session has expired. Please log in again.".to_string()
            }
            TcpError::InvalidCredentials(_) => "Invalid username or password.".to_string(),
            TcpError::AccountLocked(_) => {
                "Your account has been locked due to security concerns.".to_string()
            }

            // Validation errors
            TcpError::ValidationError(ValidationError::InvalidFormat(msg)) => {
                format!("Invalid format: {}", msg)
            }
            TcpError::ValidationError(ValidationError::InvalidLength(msg)) => {
                format!("Invalid length: {}", msg)
            }
            TcpError::ValidationError(ValidationError::InvalidCharacters(msg)) => {
                format!("Invalid characters: {}", msg)
            }
            TcpError::ValidationError(ValidationError::RateLimitExceeded(msg)) => {
                format!("Rate limit exceeded: {}", msg)
            }
            TcpError::ValidationError(ValidationError::PermissionDenied(msg)) => {
                format!("Permission denied: {}", msg)
            }
            TcpError::ValidationError(ValidationError::SecurityViolation(msg)) => {
                format!("Security violation: {}", msg)
            }
            TcpError::ValidationError(ValidationError::RequiredFieldMissing(msg)) => {
                format!("Required field missing: {}", msg)
            }
            TcpError::ValidationError(ValidationError::InvalidValue(msg)) => {
                format!("Invalid value: {}", msg)
            }
            TcpError::ValidationError(ValidationError::ConflictingValues(msg)) => {
                format!("Conflicting values: {}", msg)
            }
            TcpError::RateLimitExceeded(_) => {
                "Rate limit exceeded. Please try again later.".to_string()
            }
            TcpError::InvalidInput(msg) => format!("Invalid input: {}", msg),
            TcpError::InvalidFormat(msg) => format!("Invalid format: {}", msg),
            TcpError::InvalidLength(msg) => format!("Invalid length: {}", msg),
            TcpError::InvalidCharacters(msg) => format!("Invalid characters: {}", msg),

            // Database errors
            TcpError::DatabaseError(_) => "Database error occurred. Please try again.".to_string(),
            TcpError::TransactionFailed(_) => "Transaction failed. Please try again.".to_string(),
            TcpError::DataIntegrityError(_) => {
                "Data integrity error. Please contact support.".to_string()
            }
            TcpError::ConnectionFailed(_) => "Connection failed. Please try again.".to_string(),
            TcpError::QueryFailed(_) => "Query failed. Please try again.".to_string(),

            // System errors
            TcpError::SystemError(_) => "System error occurred. Please try again.".to_string(),
            TcpError::NetworkError(_) => {
                "Network error occurred. Please check your connection.".to_string()
            }
            TcpError::TimeoutError(_) => "Operation timed out. Please try again.".to_string(),
            TcpError::InternalError(_) => {
                "Internal error occurred. Please contact support.".to_string()
            }
            TcpError::ConfigurationError(_) => {
                "Configuration error. Please contact support.".to_string()
            }

            // Security errors
            TcpError::SecurityViolation(_) => {
                "Security violation detected. Access denied.".to_string()
            }
            TcpError::IntrusionDetected(_) => {
                "Intrusion detected. Connection terminated.".to_string()
            }
            TcpError::SuspiciousActivity(_) => {
                "Suspicious activity detected. Please verify your identity.".to_string()
            }
            TcpError::AccessDenied(_) => "Access denied.".to_string(),
            TcpError::SecurityBreach(_) => {
                "Security breach detected. Please contact support.".to_string()
            }

            // Resource errors
            TcpError::ResourceNotFound(msg) => format!("Resource not found: {}", msg),
            TcpError::ResourceUnavailable(msg) => format!("Resource unavailable: {}", msg),
            TcpError::ResourceExhausted(_) => {
                "Resource exhausted. Please try again later.".to_string()
            }
            TcpError::PermissionDenied(msg) => format!("Permission denied: {}", msg),

            // Protocol errors
            TcpError::ProtocolError(msg) => format!("Protocol error: {}", msg),
            TcpError::MalformedRequest(msg) => format!("Malformed request: {}", msg),
            TcpError::UnsupportedOperation(msg) => format!("Unsupported operation: {}", msg),
            TcpError::VersionMismatch(msg) => format!("Version mismatch: {}", msg),
        }
    }

    /// Get the appropriate log level for this error
    pub fn log_level(&self) -> Level {
        match self {
            // Critical errors that require immediate attention
            TcpError::SecurityBreach(_)
            | TcpError::IntrusionDetected(_)
            | TcpError::DataIntegrityError(_)
            | TcpError::ConfigurationError(_) => Level::ERROR,

            // High priority errors
            TcpError::AuthenticationFailed(_)
            | TcpError::AuthorizationDenied(_)
            | TcpError::SecurityViolation(_)
            | TcpError::DatabaseError(_)
            | TcpError::TransactionFailed(_)
            | TcpError::SystemError(_)
            | TcpError::InternalError(_) => Level::WARN,

            // Medium priority errors
            TcpError::ValidationError(_)
            | TcpError::RateLimitExceeded(_)
            | TcpError::InvalidInput(_)
            | TcpError::ResourceNotFound(_)
            | TcpError::PermissionDenied(_)
            | TcpError::ProtocolError(_) => Level::INFO,

            // Low priority errors
            TcpError::SessionExpired(_)
            | TcpError::NetworkError(_)
            | TcpError::TimeoutError(_)
            | TcpError::ResourceUnavailable(_)
            | TcpError::MalformedRequest(_) => Level::DEBUG,

            // Default to INFO for other errors
            _ => Level::INFO,
        }
    }

    /// Determine if this error should cause a client disconnection
    pub fn should_disconnect(&self) -> bool {
        match self {
            TcpError::SecurityBreach(_)
            | TcpError::IntrusionDetected(_)
            | TcpError::SecurityViolation(_)
            | TcpError::AccountLocked(_)
            | TcpError::SuspiciousActivity(_) => true,
            _ => false,
        }
    }

    /// Get the recovery action for this error
    pub fn recovery_action(&self) -> Option<RecoveryAction> {
        match self {
            // Retry with backoff for transient errors
            TcpError::NetworkError(_)
            | TcpError::TimeoutError(_)
            | TcpError::ConnectionFailed(_) => Some(RecoveryAction::Retry(RetryPolicy {
                max_retries: 3,
                backoff_strategy: BackoffStrategy::Exponential(Duration::from_millis(100)),
                retry_conditions: vec![RetryCondition::NetworkError, RetryCondition::TimeoutError],
            })),

            // Database errors with retry
            TcpError::DatabaseError(_)
            | TcpError::TransactionFailed(_)
            | TcpError::QueryFailed(_) => Some(RecoveryAction::Retry(RetryPolicy {
                max_retries: 2,
                backoff_strategy: BackoffStrategy::Linear(Duration::from_millis(50)),
                retry_conditions: vec![RetryCondition::TransientDatabaseError],
            })),

            // Rate limiting with delay
            TcpError::RateLimitExceeded(_) => {
                Some(RecoveryAction::RateLimitDelay(Duration::from_secs(1)))
            }

            // Authentication errors require re-authentication
            TcpError::SessionExpired(_) | TcpError::InvalidCredentials(_) => {
                Some(RecoveryAction::Authenticate)
            }

            // Security errors require disconnection
            TcpError::SecurityBreach(_)
            | TcpError::IntrusionDetected(_)
            | TcpError::SecurityViolation(_)
            | TcpError::AccountLocked(_) => Some(RecoveryAction::Disconnect),

            // No recovery action for other errors
            _ => None,
        }
    }

    /// Get the error severity
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            TcpError::SecurityBreach(_)
            | TcpError::IntrusionDetected(_)
            | TcpError::DataIntegrityError(_)
            | TcpError::ConfigurationError(_) => ErrorSeverity::Critical,

            TcpError::AuthenticationFailed(_)
            | TcpError::AuthorizationDenied(_)
            | TcpError::SecurityViolation(_)
            | TcpError::DatabaseError(_)
            | TcpError::SystemError(_) => ErrorSeverity::High,

            TcpError::ValidationError(_)
            | TcpError::RateLimitExceeded(_)
            | TcpError::PermissionDenied(_)
            | TcpError::ResourceNotFound(_) => ErrorSeverity::Medium,

            _ => ErrorSeverity::Low,
        }
    }

    /// Convert from storage error
    pub fn from_storage_error(error: StorageError) -> Self {
        match error {
            StorageError::ConnectionError { message } => {
                TcpError::DatabaseError(DatabaseError::ConnectionFailed(message))
            }
            StorageError::QueryError { message } => {
                TcpError::DatabaseError(DatabaseError::QueryFailed(message))
            }
            StorageError::SerializationError { message } => {
                TcpError::DatabaseError(DatabaseError::SerializationError(message))
            }
            StorageError::ValidationError { field: _, message } => {
                TcpError::ValidationError(ValidationError::InvalidValue(message))
            }
            StorageError::NotFound { entity, id } => {
                TcpError::ResourceNotFound(format!("{} with id {}", entity, id))
            }
            StorageError::DuplicateError { entity: _, message } => {
                TcpError::ValidationError(ValidationError::ConflictingValues(message))
            }
            StorageError::ConstraintError { message } => {
                TcpError::ValidationError(ValidationError::ConflictingValues(message))
            }
            StorageError::TimeoutError => {
                TcpError::InternalError("Database operation timed out".to_string())
            }
            StorageError::PoolExhausted => {
                TcpError::InternalError("Database connection pool exhausted".to_string())
            }
            StorageError::TransactionError { message } => {
                TcpError::DatabaseError(DatabaseError::QueryFailed(message))
            }
            StorageError::MigrationError { message } => {
                TcpError::InternalError(format!("Migration failed: {}", message))
            }
            StorageError::DeserializationError { message } => {
                TcpError::DatabaseError(DatabaseError::SerializationError(message))
            }
            StorageError::UnsupportedOperation { operation } => {
                TcpError::InternalError(format!("Unsupported operation: {}", operation))
            }
        }
    }

    /// Create error with context
    pub fn with_context(self, context: ErrorContext) -> TcpErrorWithContext {
        TcpErrorWithContext {
            error: self,
            context,
        }
    }
}

/// Error with additional context information
#[derive(Debug, Clone)]
pub struct TcpErrorWithContext {
    pub error: TcpError,
    pub context: ErrorContext,
}

impl fmt::Display for TcpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            ValidationError::InvalidLength(msg) => write!(f, "Invalid length: {}", msg),
            ValidationError::InvalidCharacters(msg) => write!(f, "Invalid characters: {}", msg),
            ValidationError::RateLimitExceeded(msg) => write!(f, "Rate limit exceeded: {}", msg),
            ValidationError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            ValidationError::SecurityViolation(msg) => write!(f, "Security violation: {}", msg),
            ValidationError::RequiredFieldMissing(msg) => {
                write!(f, "Required field missing: {}", msg)
            }
            ValidationError::InvalidValue(msg) => write!(f, "Invalid value: {}", msg),
            ValidationError::ConflictingValues(msg) => write!(f, "Conflicting values: {}", msg),
        }
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::ConnectionFailed(msg) => {
                write!(f, "Database connection failed: {}", msg)
            }
            DatabaseError::QueryFailed(msg) => write!(f, "Database query failed: {}", msg),
            DatabaseError::TransactionFailed(msg) => {
                write!(f, "Database transaction failed: {}", msg)
            }
            DatabaseError::DataIntegrityError(msg) => write!(f, "Data integrity error: {}", msg),
            DatabaseError::UniqueConstraintViolation(msg) => {
                write!(f, "Unique constraint violation: {}", msg)
            }
            DatabaseError::ForeignKeyConstraintViolation(msg) => {
                write!(f, "Foreign key constraint violation: {}", msg)
            }
            DatabaseError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            DatabaseError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
        }
    }
}

impl std::error::Error for TcpError {}
impl std::error::Error for ValidationError {}
impl std::error::Error for DatabaseError {}
