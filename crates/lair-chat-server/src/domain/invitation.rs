//! Invitation domain types.
//!
//! See [DOMAIN_MODEL.md](../../../../docs/architecture/DOMAIN_MODEL.md) for full specification.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

use super::{RoomId, UserId, ValidationError};
use crate::domain::RoomRole;

// ============================================================================
// InvitationId
// ============================================================================

/// Unique identifier for a room invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InvitationId(Uuid);

impl InvitationId {
    /// Create a new random InvitationId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create an InvitationId from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse an InvitationId from a string.
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|_| ValidationError::InvalidFormat {
                reason: "invalid UUID format".into(),
            })
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for InvitationId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for InvitationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for InvitationId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

// ============================================================================
// InvitationStatus
// ============================================================================

/// The status of a room invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InvitationStatus {
    /// Waiting for response.
    #[default]
    Pending,
    /// Invitation accepted.
    Accepted,
    /// Invitation declined.
    Declined,
    /// Invitation cancelled by inviter.
    Cancelled,
    /// Invitation expired.
    Expired,
}

impl InvitationStatus {
    /// Check if this status is terminal (no further transitions possible).
    pub fn is_terminal(&self) -> bool {
        !matches!(self, InvitationStatus::Pending)
    }

    /// Check if this status indicates the invitation can still be accepted.
    pub fn is_actionable(&self) -> bool {
        matches!(self, InvitationStatus::Pending)
    }

    /// Get the status as a string for database storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            InvitationStatus::Pending => "pending",
            InvitationStatus::Accepted => "accepted",
            InvitationStatus::Declined => "declined",
            InvitationStatus::Cancelled => "cancelled",
            InvitationStatus::Expired => "expired",
        }
    }

    /// Parse a status from a database string.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "accepted" => InvitationStatus::Accepted,
            "declined" => InvitationStatus::Declined,
            "cancelled" => InvitationStatus::Cancelled,
            "expired" => InvitationStatus::Expired,
            _ => InvitationStatus::Pending,
        }
    }
}

impl Display for InvitationStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InvitationStatus::Pending => write!(f, "pending"),
            InvitationStatus::Accepted => write!(f, "accepted"),
            InvitationStatus::Declined => write!(f, "declined"),
            InvitationStatus::Cancelled => write!(f, "cancelled"),
            InvitationStatus::Expired => write!(f, "expired"),
        }
    }
}

// ============================================================================
// Invitation
// ============================================================================

/// A room invitation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invitation {
    /// Unique identifier.
    pub id: InvitationId,
    /// Room being invited to.
    pub room_id: RoomId,
    /// User who sent the invitation.
    pub inviter: UserId,
    /// User being invited.
    pub invitee: UserId,
    /// Current status.
    pub status: InvitationStatus,
    /// Optional message from the inviter.
    pub message: Option<String>,
    /// When the invitation was created.
    pub created_at: DateTime<Utc>,
    /// When the invitation was responded to (accepted, declined, etc.).
    pub responded_at: Option<DateTime<Utc>>,
    /// When the invitation expires.
    pub expires_at: DateTime<Utc>,
}

impl Invitation {
    /// Default invitation expiration (7 days).
    pub const DEFAULT_EXPIRATION: Duration = Duration::days(7);

    /// Create a new invitation with default expiration.
    pub fn new(room_id: RoomId, inviter: UserId, invitee: UserId) -> Self {
        let now = Utc::now();
        Self {
            id: InvitationId::new(),
            room_id,
            inviter,
            invitee,
            status: InvitationStatus::Pending,
            message: None,
            created_at: now,
            responded_at: None,
            expires_at: now + Self::DEFAULT_EXPIRATION,
        }
    }

    /// Create an invitation with a message.
    pub fn with_message(
        room_id: RoomId,
        inviter: UserId,
        invitee: UserId,
        message: impl Into<String>,
    ) -> Self {
        let mut invitation = Self::new(room_id, inviter, invitee);
        invitation.message = Some(message.into());
        invitation
    }

    /// Create an invitation with custom expiration duration.
    pub fn with_expiration(
        room_id: RoomId,
        inviter: UserId,
        invitee: UserId,
        duration: Duration,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: InvitationId::new(),
            room_id,
            inviter,
            invitee,
            status: InvitationStatus::Pending,
            message: None,
            created_at: now,
            responded_at: None,
            expires_at: now + duration,
        }
    }

    /// Check if the invitation has expired based on time.
    pub fn is_time_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the invitation can still be accepted.
    ///
    /// Returns true if status is Pending and not expired.
    pub fn is_pending(&self) -> bool {
        matches!(self.status, InvitationStatus::Pending) && !self.is_time_expired()
    }

    /// Check if the invitation is actionable (can be accepted or declined).
    pub fn is_actionable(&self) -> bool {
        self.is_pending()
    }

    /// Accept the invitation.
    ///
    /// Returns an error if the invitation is not pending or has expired.
    pub fn accept(&mut self) -> Result<(), InvitationError> {
        if self.is_time_expired() {
            self.status = InvitationStatus::Expired;
            return Err(InvitationError::Expired);
        }
        if !self.status.is_actionable() {
            return Err(InvitationError::NotPending(self.status));
        }
        self.status = InvitationStatus::Accepted;
        self.responded_at = Some(Utc::now());
        Ok(())
    }

    /// Decline the invitation.
    ///
    /// Returns an error if the invitation is not pending or has expired.
    pub fn decline(&mut self) -> Result<(), InvitationError> {
        if self.is_time_expired() {
            self.status = InvitationStatus::Expired;
            return Err(InvitationError::Expired);
        }
        if !self.status.is_actionable() {
            return Err(InvitationError::NotPending(self.status));
        }
        self.status = InvitationStatus::Declined;
        self.responded_at = Some(Utc::now());
        Ok(())
    }

    /// Cancel the invitation (by the inviter).
    ///
    /// Returns an error if the invitation is not pending.
    pub fn cancel(&mut self) -> Result<(), InvitationError> {
        if !self.status.is_actionable() {
            return Err(InvitationError::NotPending(self.status));
        }
        self.status = InvitationStatus::Cancelled;
        self.responded_at = Some(Utc::now());
        Ok(())
    }

    /// Mark the invitation as expired.
    pub fn expire(&mut self) {
        if self.status.is_actionable() {
            self.status = InvitationStatus::Expired;
        }
    }

    /// Get time remaining until expiration.
    pub fn time_remaining(&self) -> Duration {
        let remaining = self.expires_at - Utc::now();
        if remaining < Duration::zero() {
            Duration::zero()
        } else {
            remaining
        }
    }
}

// ============================================================================
// InvitationError
// ============================================================================

/// Errors that can occur when modifying an invitation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvitationError {
    /// The invitation has expired.
    Expired,
    /// The invitation is not in pending status.
    NotPending(InvitationStatus),
}

impl Display for InvitationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InvitationError::Expired => write!(f, "invitation has expired"),
            InvitationError::NotPending(status) => {
                write!(f, "invitation is not pending (current status: {})", status)
            }
        }
    }
}

impl std::error::Error for InvitationError {}

// ============================================================================
// EnrichedInvitation
// ============================================================================

/// An invitation with enriched data (names resolved from IDs).
///
/// This is what the API returns. The server always populates name fields
/// so clients don't need to do additional lookups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedInvitation {
    /// Unique identifier.
    pub id: InvitationId,
    /// Room ID.
    pub room_id: RoomId,
    /// Room name (always populated by server).
    pub room_name: String,
    /// User who sent the invitation.
    pub inviter_id: UserId,
    /// Inviter's username (always populated by server).
    pub inviter_name: String,
    /// User being invited.
    pub invitee_id: UserId,
    /// Invitee's username (always populated by server).
    pub invitee_name: String,
    /// Current status.
    pub status: InvitationStatus,
    /// Optional message from the inviter.
    pub message: Option<String>,
    /// When the invitation was created.
    pub created_at: DateTime<Utc>,
    /// When the invitation expires.
    pub expires_at: DateTime<Utc>,
    /// When the invitation was responded to.
    pub responded_at: Option<DateTime<Utc>>,
}

impl EnrichedInvitation {
    /// Create an enriched invitation from a base invitation with resolved names.
    pub fn from_invitation(
        invitation: &Invitation,
        room_name: String,
        inviter_name: String,
        invitee_name: String,
    ) -> Self {
        Self {
            id: invitation.id,
            room_id: invitation.room_id,
            room_name,
            inviter_id: invitation.inviter,
            inviter_name,
            invitee_id: invitation.invitee,
            invitee_name,
            status: invitation.status,
            message: invitation.message.clone(),
            created_at: invitation.created_at,
            expires_at: invitation.expires_at,
            responded_at: invitation.responded_at,
        }
    }
}

// ============================================================================
// RoomMember
// ============================================================================

/// Enriched room member with user details.
///
/// Used for the members list API endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMember {
    /// User ID.
    pub user_id: UserId,
    /// Username.
    pub username: String,
    /// Role in the room.
    pub role: RoomRole,
    /// When they joined.
    pub joined_at: DateTime<Utc>,
    /// Whether they are currently online.
    pub is_online: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invitation_id() {
        let id1 = InvitationId::new();
        let id2 = InvitationId::new();
        assert_ne!(id1, id2);

        let parsed = InvitationId::parse(&id1.to_string()).unwrap();
        assert_eq!(id1, parsed);

        assert!(InvitationId::parse("not-a-uuid").is_err());
    }

    #[test]
    fn test_invitation_status_properties() {
        assert!(!InvitationStatus::Pending.is_terminal());
        assert!(InvitationStatus::Pending.is_actionable());

        assert!(InvitationStatus::Accepted.is_terminal());
        assert!(!InvitationStatus::Accepted.is_actionable());

        assert!(InvitationStatus::Declined.is_terminal());
        assert!(InvitationStatus::Cancelled.is_terminal());
        assert!(InvitationStatus::Expired.is_terminal());
    }

    #[test]
    fn test_invitation_creation() {
        let room_id = RoomId::new();
        let inviter = UserId::new();
        let invitee = UserId::new();
        let invitation = Invitation::new(room_id, inviter, invitee);

        assert_eq!(invitation.room_id, room_id);
        assert_eq!(invitation.inviter, inviter);
        assert_eq!(invitation.invitee, invitee);
        assert_eq!(invitation.status, InvitationStatus::Pending);
        assert!(invitation.is_pending());
        assert!(invitation.is_actionable());
    }

    #[test]
    fn test_invitation_with_message() {
        let room_id = RoomId::new();
        let inviter = UserId::new();
        let invitee = UserId::new();
        let invitation = Invitation::with_message(room_id, inviter, invitee, "Join our room!");

        assert_eq!(invitation.message, Some("Join our room!".to_string()));
        assert!(invitation.is_pending());
    }

    #[test]
    fn test_invitation_accept() {
        let room_id = RoomId::new();
        let inviter = UserId::new();
        let invitee = UserId::new();
        let mut invitation = Invitation::new(room_id, inviter, invitee);

        assert!(invitation.accept().is_ok());
        assert_eq!(invitation.status, InvitationStatus::Accepted);
        assert!(!invitation.is_pending());
        assert!(!invitation.is_actionable());
    }

    #[test]
    fn test_invitation_decline() {
        let room_id = RoomId::new();
        let inviter = UserId::new();
        let invitee = UserId::new();
        let mut invitation = Invitation::new(room_id, inviter, invitee);

        assert!(invitation.decline().is_ok());
        assert_eq!(invitation.status, InvitationStatus::Declined);
    }

    #[test]
    fn test_invitation_cancel() {
        let room_id = RoomId::new();
        let inviter = UserId::new();
        let invitee = UserId::new();
        let mut invitation = Invitation::new(room_id, inviter, invitee);

        assert!(invitation.cancel().is_ok());
        assert_eq!(invitation.status, InvitationStatus::Cancelled);
    }

    #[test]
    fn test_invitation_double_accept() {
        let room_id = RoomId::new();
        let inviter = UserId::new();
        let invitee = UserId::new();
        let mut invitation = Invitation::new(room_id, inviter, invitee);

        invitation.accept().unwrap();
        let result = invitation.accept();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            InvitationError::NotPending(InvitationStatus::Accepted)
        );
    }

    #[test]
    fn test_invitation_expired() {
        let room_id = RoomId::new();
        let inviter = UserId::new();
        let invitee = UserId::new();
        // Create invitation that expires immediately
        let mut invitation =
            Invitation::with_expiration(room_id, inviter, invitee, Duration::seconds(-1));

        assert!(invitation.is_time_expired());
        assert!(!invitation.is_pending());
        assert!(!invitation.is_actionable());

        // Try to accept expired invitation
        let result = invitation.accept();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), InvitationError::Expired);
        assert_eq!(invitation.status, InvitationStatus::Expired);
    }

    #[test]
    fn test_invitation_time_remaining() {
        let room_id = RoomId::new();
        let inviter = UserId::new();
        let invitee = UserId::new();
        let invitation = Invitation::with_expiration(room_id, inviter, invitee, Duration::hours(1));

        let remaining = invitation.time_remaining();
        assert!(remaining > Duration::minutes(59));
        assert!(remaining <= Duration::hours(1));
    }

    #[test]
    fn test_invitation_expire() {
        let room_id = RoomId::new();
        let inviter = UserId::new();
        let invitee = UserId::new();
        let mut invitation = Invitation::new(room_id, inviter, invitee);

        invitation.expire();
        assert_eq!(invitation.status, InvitationStatus::Expired);
    }

    #[test]
    fn test_invitation_status_serialization() {
        let pending = InvitationStatus::Pending;
        let json = serde_json::to_string(&pending).unwrap();
        assert_eq!(json, "\"pending\"");

        let accepted = InvitationStatus::Accepted;
        let json = serde_json::to_string(&accepted).unwrap();
        assert_eq!(json, "\"accepted\"");
    }
}
