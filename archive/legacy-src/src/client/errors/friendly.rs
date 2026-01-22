//! User-friendly error handling for Lair-Chat
//! Converts technical errors into understandable messages for users.

use std::fmt;
use thiserror::Error;

use crate::{
    client::{
        auth::AuthError,
        transport::TransportError,
        history::HistoryError,
    },
};

/// User-friendly error category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Connection-related errors
    Connection,
    /// Authentication-related errors
    Authentication,
    /// Message-related errors
    Messaging,
    /// Configuration-related errors
    Configuration,
    /// Data-related errors
    Data,
    /// System-related errors
    System,
    /// Unknown error category
    Unknown,
}

impl ErrorCategory {
    /// Get a human-readable name for the category
    pub fn name(&self) -> &'static str {
        match self {
            Self::Connection => "Connection",
            Self::Authentication => "Authentication",
            Self::Messaging => "Messaging",
            Self::Configuration => "Configuration",
            Self::Data => "Data",
            Self::System => "System",
            Self::Unknown => "Unknown",
        }
    }
    
    /// Get a suggested action for the category
    pub fn suggested_action(&self) -> &'static str {
        match self {
            Self::Connection => "Check your network connection and try again.",
            Self::Authentication => "Please verify your credentials and try again.",
            Self::Messaging => "Your message could not be sent. Try again later.",
            Self::Configuration => "Please check your configuration settings.",
            Self::Data => "There was a problem with your data. Try restarting the application.",
            Self::System => "A system error occurred. Try restarting the application.",
            Self::Unknown => "An unexpected error occurred. Please try again.",
        }
    }
}

/// User-friendly error representation
#[derive(Debug, Clone)]
pub struct FriendlyError {
    /// Original error message
    pub original: String,
    /// User-friendly message
    pub message: String,
    /// Error category
    pub category: ErrorCategory,
    /// Suggested action
    pub suggestion: Option<String>,
    /// Detailed technical information (for advanced users)
    pub details: Option<String>,
}

impl FriendlyError {
    /// Create a new friendly error
    pub fn new(
        original: impl Into<String>,
        message: impl Into<String>,
        category: ErrorCategory,
    ) -> Self {
        Self {
            original: original.into(),
            message: message.into(),
            category,
            suggestion: None,
            details: None,
        }
    }
    
    /// Add a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
    
    /// Add technical details
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
    
    /// Format the error for display
    pub fn format(&self) -> String {
        let mut result = format!("{}: {}", self.category.name(), self.message);
        
        if let Some(suggestion) = &self.suggestion {
            result.push_str(&format!("\n{}", suggestion));
        } else {
            result.push_str(&format!("\n{}", self.category.suggested_action()));
        }
        
        result
    }
    
    /// Format the error with technical details for advanced users
    pub fn format_detailed(&self) -> String {
        let mut result = self.format();
        
        if let Some(details) = &self.details {
            result.push_str(&format!("\n\nTechnical details: {}", details));
        } else {
            result.push_str(&format!("\n\nOriginal error: {}", self.original));
        }
        
        result
    }
}

impl fmt::Display for FriendlyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Convert standard errors to friendly errors
pub trait IntoFriendlyError {
    fn into_friendly(self) -> FriendlyError;
}

impl IntoFriendlyError for TransportError {
    fn into_friendly(self) -> FriendlyError {
        match self {
            TransportError::ConnectionError(e) => FriendlyError::new(
                format!("ConnectionError: {}", e),
                "Failed to connect to the server",
                ErrorCategory::Connection,
            ).with_details(format!("IO error: {}", e)),
            
            TransportError::EncryptionError(e) => FriendlyError::new(
                format!("EncryptionError: {}", e),
                "Encryption error occurred",
                ErrorCategory::Connection,
            ).with_suggestion("The secure connection could not be established. Please try again."),
            
            TransportError::ProtocolError(e) => FriendlyError::new(
                format!("ProtocolError: {}", e),
                "Communication protocol error",
                ErrorCategory::Connection,
            ).with_suggestion("There was a problem with the chat protocol. Try reconnecting."),
            
            TransportError::HandshakeError(e) => FriendlyError::new(
                format!("HandshakeError: {}", e),
                "Failed to establish secure connection",
                ErrorCategory::Connection,
            ).with_suggestion("Security handshake failed. Please try reconnecting."),
            
            TransportError::DeserializationError(e) => FriendlyError::new(
                format!("DeserializationError: {}", e),
                "Failed to process server message",
                ErrorCategory::Data,
            ),
            
            TransportError::TimeoutError => FriendlyError::new(
                "TimeoutError",
                "Connection timed out",
                ErrorCategory::Connection,
            ).with_suggestion("The server took too long to respond. Check your connection and try again."),
            
            _ => FriendlyError::new(
                format!("TransportError: {:?}", self),
                "Connection error occurred",
                ErrorCategory::Connection,
            ),
        }
    }
}

impl IntoFriendlyError for AuthError {
    fn into_friendly(self) -> FriendlyError {
        match self {
            AuthError::InvalidCredentials => FriendlyError::new(
                "InvalidCredentials",
                "Invalid username or password",
                ErrorCategory::Authentication,
            ).with_suggestion("Please check your username and password and try again."),
            
            AuthError::UserNotFound => FriendlyError::new(
                "UserNotFound",
                "User not found",
                ErrorCategory::Authentication,
            ).with_suggestion("This username doesn't exist. Would you like to register?"),
            
            AuthError::UsernameTaken => FriendlyError::new(
                "UsernameTaken",
                "Username already taken",
                ErrorCategory::Authentication,
            ).with_suggestion("Please choose a different username."),
            
            AuthError::SessionExpired => FriendlyError::new(
                "SessionExpired",
                "Your session has expired",
                ErrorCategory::Authentication,
            ).with_suggestion("Please log in again."),
            
            AuthError::RateLimitExceeded => FriendlyError::new(
                "RateLimitExceeded",
                "Too many login attempts",
                ErrorCategory::Authentication,
            ).with_suggestion("Please wait a moment before trying again."),
            
            AuthError::AuthenticationFailed(reason) => FriendlyError::new(
                format!("AuthenticationFailed: {}", reason),
                "Authentication failed",
                ErrorCategory::Authentication,
            ).with_details(reason),
            
            AuthError::ConnectionError(reason) => FriendlyError::new(
                format!("ConnectionError: {}", reason),
                "Connection error during authentication",
                ErrorCategory::Connection,
            ).with_details(reason),
            
            AuthError::ProtocolError(reason) => FriendlyError::new(
                format!("ProtocolError: {}", reason),
                "Protocol error during authentication",
                ErrorCategory::Authentication,
            ).with_details(reason),
            
            AuthError::StorageError(reason) => FriendlyError::new(
                format!("StorageError: {}", reason),
                "Failed to access authentication storage",
                ErrorCategory::Data,
            ).with_suggestion("Your login information could not be saved or loaded."),
            
            AuthError::InternalError(reason) => FriendlyError::new(
                format!("InternalError: {}", reason),
                "Internal authentication error",
                ErrorCategory::System,
            ).with_details(reason),
            
            _ => FriendlyError::new(
                format!("AuthError: {:?}", self),
                "Authentication error occurred",
                ErrorCategory::Authentication,
            ),
        }
    }
}

impl IntoFriendlyError for HistoryError {
    fn into_friendly(self) -> FriendlyError {
        match self {
            HistoryError::DirectoryCreation(e) => FriendlyError::new(
                format!("DirectoryCreation: {}", e),
                "Failed to create history directory",
                ErrorCategory::Data,
            ).with_suggestion("Command history couldn't be saved."),
            
            HistoryError::FileRead(e) => FriendlyError::new(
                format!("FileRead: {}", e),
                "Failed to read history file",
                ErrorCategory::Data,
            ).with_suggestion("Command history couldn't be loaded."),
            
            HistoryError::FileWrite(e) => FriendlyError::new(
                format!("FileWrite: {}", e),
                "Failed to write history file",
                ErrorCategory::Data,
            ).with_suggestion("Command history couldn't be saved."),
            
            HistoryError::Serialization(e) => FriendlyError::new(
                format!("Serialization: {}", e),
                "Failed to process history data",
                ErrorCategory::Data,
            ),
            
            HistoryError::System(reason) => FriendlyError::new(
                format!("System: {}", reason),
                "System error occurred",
                ErrorCategory::System,
            ).with_details(reason),
        }
    }
}

impl IntoFriendlyError for std::io::Error {
    fn into_friendly(self) -> FriendlyError {
        match self.kind() {
            std::io::ErrorKind::NotFound => FriendlyError::new(
                format!("NotFound: {}", self),
                "File or resource not found",
                ErrorCategory::Data,
            ),
            
            std::io::ErrorKind::PermissionDenied => FriendlyError::new(
                format!("PermissionDenied: {}", self),
                "Permission denied",
                ErrorCategory::System,
            ).with_suggestion("The application doesn't have permission to access a required file or resource."),
            
            std::io::ErrorKind::ConnectionRefused => FriendlyError::new(
                format!("ConnectionRefused: {}", self),
                "Connection refused by server",
                ErrorCategory::Connection,
            ).with_suggestion("The server is not accepting connections. Please try again later."),
            
            std::io::ErrorKind::ConnectionReset => FriendlyError::new(
                format!("ConnectionReset: {}", self),
                "Connection was reset",
                ErrorCategory::Connection,
            ).with_suggestion("The connection was interrupted. Please try reconnecting."),
            
            std::io::ErrorKind::ConnectionAborted => FriendlyError::new(
                format!("ConnectionAborted: {}", self),
                "Connection was aborted",
                ErrorCategory::Connection,
            ).with_suggestion("The connection was aborted. Please try reconnecting."),
            
            std::io::ErrorKind::NotConnected => FriendlyError::new(
                format!("NotConnected: {}", self),
                "Not connected to server",
                ErrorCategory::Connection,
            ).with_suggestion("You are not connected to the server. Please connect first."),
            
            std::io::ErrorKind::TimedOut => FriendlyError::new(
                format!("TimedOut: {}", self),
                "Connection timed out",
                ErrorCategory::Connection,
            ).with_suggestion("The server took too long to respond. Check your connection and try again."),
            
            _ => FriendlyError::new(
                format!("IOError: {}", self),
                "System I/O error occurred",
                ErrorCategory::System,
            ),
        }
    }
}

impl<E: std::error::Error + 'static> IntoFriendlyError for E {
    default fn into_friendly(self) -> FriendlyError {
        FriendlyError::new(
            format!("{:?}", self),
            "An error occurred",
            ErrorCategory::Unknown,
        ).with_details(format!("{:?}", self))
    }
}

/// Shorthand for creating a connection error
pub fn connection_error(message: impl Into<String>) -> FriendlyError {
    FriendlyError::new(
        message.clone(),
        message,
        ErrorCategory::Connection,
    )
}

/// Shorthand for creating an authentication error
pub fn auth_error(message: impl Into<String>) -> FriendlyError {
    FriendlyError::new(
        message.clone(),
        message,
        ErrorCategory::Authentication,
    )
}

/// Shorthand for creating a system error
pub fn system_error(message: impl Into<String>) -> FriendlyError {
    FriendlyError::new(
        message.clone(),
        message,
        ErrorCategory::System,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_friendly_error_creation() {
        let error = FriendlyError::new(
            "Original error",
            "User-friendly message",
            ErrorCategory::Connection,
        );
        
        assert_eq!(error.original, "Original error");
        assert_eq!(error.message, "User-friendly message");
        assert_eq!(error.category, ErrorCategory::Connection);
    }
    
    #[test]
    fn test_error_formatting() {
        let error = FriendlyError::new(
            "Original error",
            "Failed to connect",
            ErrorCategory::Connection,
        );
        
        let formatted = error.format();
        assert!(formatted.contains("Connection: Failed to connect"));
        assert!(formatted.contains("Check your network connection"));
    }
    
    #[test]
    fn test_detailed_formatting() {
        let error = FriendlyError::new(
            "Original error",
            "Failed to connect",
            ErrorCategory::Connection,
        ).with_details("TCP connection refused");
        
        let formatted = error.format_detailed();
        assert!(formatted.contains("Technical details: TCP connection refused"));
    }
    
    #[test]
    fn test_transport_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Connection refused");
        let transport_error = TransportError::ConnectionError(io_error);
        
        let friendly = transport_error.into_friendly();
        assert_eq!(friendly.category, ErrorCategory::Connection);
        assert!(friendly.message.contains("Failed to connect"));
    }
    
    #[test]
    fn test_auth_error_conversion() {
        let auth_error = AuthError::InvalidCredentials;
        
        let friendly = auth_error.into_friendly();
        assert_eq!(friendly.category, ErrorCategory::Authentication);
        assert!(friendly.message.contains("Invalid username or password"));
    }
}