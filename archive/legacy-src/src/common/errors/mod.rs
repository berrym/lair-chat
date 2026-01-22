//! Common error types for lair-chat
//!
//! This module provides shared error types and utilities used by both
//! client and server components of the lair-chat application.

use std::fmt;

/// Common error types that can occur across the application
#[derive(Debug, Clone)]
pub enum CommonError {
    /// Network-related errors
    NetworkError(String),
    /// Protocol parsing or validation errors
    ProtocolError(String),
    /// Authentication and authorization errors
    AuthError(String),
    /// Configuration errors
    ConfigError(String),
    /// Serialization/deserialization errors
    SerializationError(String),
    /// I/O errors
    IoError(String),
    /// Generic application errors
    ApplicationError(String),
}

impl fmt::Display for CommonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommonError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            CommonError::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            CommonError::AuthError(msg) => write!(f, "Authentication error: {}", msg),
            CommonError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            CommonError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            CommonError::IoError(msg) => write!(f, "I/O error: {}", msg),
            CommonError::ApplicationError(msg) => write!(f, "Application error: {}", msg),
        }
    }
}

impl std::error::Error for CommonError {}

/// Result type alias for common operations
pub type CommonResult<T> = Result<T, CommonError>;

/// Utility functions for error handling
pub mod utils {
    use super::*;

    /// Convert a generic error to a CommonError
    pub fn to_common_error<E: std::error::Error>(error: E, context: &str) -> CommonError {
        CommonError::ApplicationError(format!("{}: {}", context, error))
    }

    /// Create a network error
    pub fn network_error(msg: impl Into<String>) -> CommonError {
        CommonError::NetworkError(msg.into())
    }

    /// Create a protocol error
    pub fn protocol_error(msg: impl Into<String>) -> CommonError {
        CommonError::ProtocolError(msg.into())
    }

    /// Create an authentication error
    pub fn auth_error(msg: impl Into<String>) -> CommonError {
        CommonError::AuthError(msg.into())
    }

    /// Create a configuration error
    pub fn config_error(msg: impl Into<String>) -> CommonError {
        CommonError::ConfigError(msg.into())
    }
}

// Re-export utility functions at module level for convenience
pub use utils::*;
