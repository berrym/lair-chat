//! # Domain Types
//!
//! Pure Rust types that define the core vocabulary of the Lair Chat system.
//! These types have no I/O dependencies and can be used anywhere.
//!
//! ## Design Principles
//!
//! - **Pure**: No async, no I/O, no database dependencies
//! - **Validated**: Invalid states are unrepresentable
//! - **Type-safe**: Newtype IDs prevent mixing up entity types
//!
//! ## Modules
//!
//! - `user`: User accounts and authentication types
//! - `room`: Chat rooms and membership
//! - `message`: Messages and content
//! - `session`: Active sessions and connections
//! - `invitation`: Room invitations
//! - `events`: Domain events for real-time updates
//!
//! See [DOMAIN_MODEL.md](../../../docs/architecture/DOMAIN_MODEL.md) for full specification.

use serde::{Deserialize, Serialize};
use std::fmt;

pub mod events;
pub mod invitation;
pub mod message;
pub mod room;
pub mod session;
pub mod user;

// Re-export commonly used types
pub use events::{Event, EventId, EventPayload, InvitationReceivedEvent};
pub use invitation::{EnrichedInvitation, Invitation, InvitationId, InvitationStatus, RoomMember};
pub use message::{Message, MessageContent, MessageId, MessageTarget};
pub use room::{Room, RoomId, RoomMembership, RoomName, RoomRole, RoomSettings};
pub use session::{Protocol, Session, SessionId};
pub use user::{Email, Role, User, UserId, Username};

// ============================================================================
// Validation Error
// ============================================================================

/// Validation error for domain types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationError {
    /// Value is empty when it shouldn't be.
    Empty,
    /// Value is too short.
    TooShort { min: usize, actual: usize },
    /// Value is too long.
    TooLong { max: usize, actual: usize },
    /// Value has invalid format.
    InvalidFormat { reason: String },
    /// Value failed a custom validation rule.
    Custom { message: String },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "value cannot be empty"),
            Self::TooShort { min, actual } => {
                write!(f, "value too short: minimum {min}, got {actual}")
            }
            Self::TooLong { max, actual } => {
                write!(f, "value too long: maximum {max}, got {actual}")
            }
            Self::InvalidFormat { reason } => write!(f, "invalid format: {reason}"),
            Self::Custom { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ValidationError {}

// ============================================================================
// Pagination
// ============================================================================

/// Pagination parameters for list queries.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Pagination {
    /// Number of items to skip.
    pub offset: u32,
    /// Maximum items to return.
    pub limit: u32,
}

impl Pagination {
    /// Maximum allowed limit.
    pub const MAX_LIMIT: u32 = 100;
    /// Default limit.
    pub const DEFAULT_LIMIT: u32 = 50;

    /// Create new pagination parameters.
    pub fn new(offset: u32, limit: u32) -> Self {
        Self {
            offset,
            limit: limit.min(Self::MAX_LIMIT),
        }
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: Self::DEFAULT_LIMIT,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ValidationError Tests
    // ========================================================================

    #[test]
    fn test_validation_error_empty_display() {
        let err = ValidationError::Empty;
        assert_eq!(err.to_string(), "value cannot be empty");
    }

    #[test]
    fn test_validation_error_too_short_display() {
        let err = ValidationError::TooShort { min: 5, actual: 3 };
        assert_eq!(err.to_string(), "value too short: minimum 5, got 3");
    }

    #[test]
    fn test_validation_error_too_long_display() {
        let err = ValidationError::TooLong {
            max: 10,
            actual: 15,
        };
        assert_eq!(err.to_string(), "value too long: maximum 10, got 15");
    }

    #[test]
    fn test_validation_error_invalid_format_display() {
        let err = ValidationError::InvalidFormat {
            reason: "bad chars".to_string(),
        };
        assert_eq!(err.to_string(), "invalid format: bad chars");
    }

    #[test]
    fn test_validation_error_custom_display() {
        let err = ValidationError::Custom {
            message: "custom error".to_string(),
        };
        assert_eq!(err.to_string(), "custom error");
    }

    #[test]
    fn test_validation_error_equality() {
        let err1 = ValidationError::Empty;
        let err2 = ValidationError::Empty;
        assert_eq!(err1, err2);

        let err3 = ValidationError::TooShort { min: 5, actual: 3 };
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_validation_error_clone() {
        let err = ValidationError::InvalidFormat {
            reason: "test".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_validation_error_serialization() {
        let err = ValidationError::TooShort { min: 5, actual: 3 };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"TooShort\""));

        let deserialized: ValidationError = serde_json::from_str(&json).unwrap();
        assert_eq!(err, deserialized);
    }

    // ========================================================================
    // Pagination Tests
    // ========================================================================

    #[test]
    fn test_pagination_default() {
        let pagination = Pagination::default();
        assert_eq!(pagination.offset, 0);
        assert_eq!(pagination.limit, Pagination::DEFAULT_LIMIT);
    }

    #[test]
    fn test_pagination_new() {
        let pagination = Pagination::new(10, 25);
        assert_eq!(pagination.offset, 10);
        assert_eq!(pagination.limit, 25);
    }

    #[test]
    fn test_pagination_max_limit_enforced() {
        let pagination = Pagination::new(0, 200);
        assert_eq!(pagination.limit, Pagination::MAX_LIMIT);
    }

    #[test]
    fn test_pagination_constants() {
        assert_eq!(Pagination::MAX_LIMIT, 100);
        assert_eq!(Pagination::DEFAULT_LIMIT, 50);
    }

    #[test]
    fn test_pagination_serialization() {
        let pagination = Pagination::new(5, 20);
        let json = serde_json::to_string(&pagination).unwrap();
        assert!(json.contains("\"offset\":5"));
        assert!(json.contains("\"limit\":20"));

        let deserialized: Pagination = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.offset, 5);
        assert_eq!(deserialized.limit, 20);
    }

    #[test]
    fn test_pagination_clone() {
        let pagination = Pagination::new(10, 50);
        let cloned = pagination;
        assert_eq!(pagination.offset, cloned.offset);
        assert_eq!(pagination.limit, cloned.limit);
    }
}
