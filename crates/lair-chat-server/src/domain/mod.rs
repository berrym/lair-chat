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
