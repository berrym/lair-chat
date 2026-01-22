//! Room domain types.
//!
//! See [DOMAIN_MODEL.md](../../../../docs/architecture/DOMAIN_MODEL.md) for full specification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

use super::{UserId, ValidationError};

// ============================================================================
// RoomId
// ============================================================================

/// Unique identifier for a chat room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoomId(Uuid);

impl RoomId {
    /// Create a new random RoomId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a RoomId from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse a RoomId from a string.
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

impl Default for RoomId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for RoomId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for RoomId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

// ============================================================================
// RoomName
// ============================================================================

/// Validated room name.
///
/// # Rules
/// - 1-64 characters
/// - Cannot be only whitespace
/// - Trimmed of leading/trailing whitespace
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RoomName(String);

impl RoomName {
    /// Maximum room name length.
    pub const MAX_LENGTH: usize = 64;

    /// Create a new room name with validation.
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into().trim().to_string();

        if s.is_empty() {
            return Err(ValidationError::Empty);
        }
        if s.len() > Self::MAX_LENGTH {
            return Err(ValidationError::TooLong {
                max: Self::MAX_LENGTH,
                actual: s.len(),
            });
        }

        Ok(Self(s))
    }

    /// Get the room name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for RoomName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for RoomName {
    type Error = ValidationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<RoomName> for String {
    fn from(name: RoomName) -> Self {
        name.0
    }
}

impl AsRef<str> for RoomName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// RoomSettings
// ============================================================================

/// Room configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSettings {
    /// Whether the room is publicly visible and joinable.
    pub public: bool,
    /// Maximum number of members (None = unlimited).
    pub max_members: Option<u32>,
    /// Whether only moderators/admins can send messages.
    pub moderated: bool,
}

impl Default for RoomSettings {
    fn default() -> Self {
        Self {
            public: true,
            max_members: None,
            moderated: false,
        }
    }
}

impl RoomSettings {
    /// Create settings for a private room.
    pub fn private() -> Self {
        Self {
            public: false,
            ..Default::default()
        }
    }

    /// Create settings for a moderated room.
    pub fn moderated() -> Self {
        Self {
            moderated: true,
            ..Default::default()
        }
    }
}

// ============================================================================
// RoomRole
// ============================================================================

/// A user's role within a specific room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RoomRole {
    /// Regular member.
    #[default]
    Member,
    /// Room moderator (can kick, mute).
    Moderator,
    /// Room owner (full control).
    Owner,
}

impl RoomRole {
    /// Check if this role has at least the given permission level.
    pub fn has_permission(&self, required: RoomRole) -> bool {
        match (self, required) {
            (RoomRole::Owner, _) => true,
            (RoomRole::Moderator, RoomRole::Moderator | RoomRole::Member) => true,
            (RoomRole::Member, RoomRole::Member) => true,
            _ => false,
        }
    }

    /// Check if this role is owner.
    pub fn is_owner(&self) -> bool {
        matches!(self, RoomRole::Owner)
    }

    /// Check if this role is at least moderator.
    pub fn is_moderator(&self) -> bool {
        matches!(self, RoomRole::Owner | RoomRole::Moderator)
    }
}

impl Display for RoomRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RoomRole::Member => write!(f, "member"),
            RoomRole::Moderator => write!(f, "moderator"),
            RoomRole::Owner => write!(f, "owner"),
        }
    }
}

// ============================================================================
// RoomMembership
// ============================================================================

/// A user's membership in a room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMembership {
    /// The room.
    pub room_id: RoomId,
    /// The member.
    pub user_id: UserId,
    /// Member's role within this room.
    pub room_role: RoomRole,
    /// When they joined.
    pub joined_at: DateTime<Utc>,
}

impl RoomMembership {
    /// Create a new membership as a regular member.
    pub fn new(room_id: RoomId, user_id: UserId) -> Self {
        Self {
            room_id,
            user_id,
            room_role: RoomRole::Member,
            joined_at: Utc::now(),
        }
    }

    /// Create a new membership as owner.
    pub fn new_owner(room_id: RoomId, user_id: UserId) -> Self {
        Self {
            room_id,
            user_id,
            room_role: RoomRole::Owner,
            joined_at: Utc::now(),
        }
    }

    /// Check if member has the required permission level.
    pub fn has_permission(&self, required: RoomRole) -> bool {
        self.room_role.has_permission(required)
    }

    /// Check if member is owner.
    pub fn is_owner(&self) -> bool {
        self.room_role.is_owner()
    }

    /// Check if member is at least moderator.
    pub fn is_moderator(&self) -> bool {
        self.room_role.is_moderator()
    }
}

// ============================================================================
// Room
// ============================================================================

/// A chat room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    /// Unique identifier.
    pub id: RoomId,
    /// Room name.
    pub name: RoomName,
    /// Optional description.
    pub description: Option<String>,
    /// User who created/owns the room.
    pub owner: UserId,
    /// Room configuration.
    pub settings: RoomSettings,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl Room {
    /// Create a new room.
    pub fn new(name: RoomName, owner: UserId) -> Self {
        Self {
            id: RoomId::new(),
            name,
            description: None,
            owner,
            settings: RoomSettings::default(),
            created_at: Utc::now(),
        }
    }

    /// Create a new room with settings.
    pub fn with_settings(name: RoomName, owner: UserId, settings: RoomSettings) -> Self {
        Self {
            settings,
            ..Self::new(name, owner)
        }
    }

    /// Check if the room is public.
    pub fn is_public(&self) -> bool {
        self.settings.public
    }

    /// Check if the room is moderated.
    pub fn is_moderated(&self) -> bool {
        self.settings.moderated
    }

    /// Check if the room is full.
    pub fn is_full(&self, current_members: u32) -> bool {
        self.settings
            .max_members
            .map(|max| current_members >= max)
            .unwrap_or(false)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_id() {
        let id1 = RoomId::new();
        let id2 = RoomId::new();
        assert_ne!(id1, id2);

        let parsed = RoomId::parse(&id1.to_string()).unwrap();
        assert_eq!(id1, parsed);
    }

    #[test]
    fn test_room_name_valid() {
        assert!(RoomName::new("General").is_ok());
        assert!(RoomName::new("  Trimmed  ").is_ok());
        assert!(RoomName::new("a").is_ok()); // minimum
        assert!(RoomName::new("a".repeat(64)).is_ok()); // maximum
    }

    #[test]
    fn test_room_name_invalid() {
        // Empty
        assert!(RoomName::new("").is_err());
        assert!(RoomName::new("   ").is_err());

        // Too long
        assert!(RoomName::new("a".repeat(65)).is_err());
    }

    #[test]
    fn test_room_role_permissions() {
        assert!(RoomRole::Owner.has_permission(RoomRole::Owner));
        assert!(RoomRole::Owner.has_permission(RoomRole::Moderator));
        assert!(RoomRole::Owner.has_permission(RoomRole::Member));

        assert!(!RoomRole::Moderator.has_permission(RoomRole::Owner));
        assert!(RoomRole::Moderator.has_permission(RoomRole::Moderator));
        assert!(RoomRole::Moderator.has_permission(RoomRole::Member));

        assert!(!RoomRole::Member.has_permission(RoomRole::Owner));
        assert!(!RoomRole::Member.has_permission(RoomRole::Moderator));
        assert!(RoomRole::Member.has_permission(RoomRole::Member));
    }

    #[test]
    fn test_room_creation() {
        let name = RoomName::new("Test Room").unwrap();
        let owner = UserId::new();
        let room = Room::new(name.clone(), owner);

        assert_eq!(room.name, name);
        assert_eq!(room.owner, owner);
        assert!(room.is_public());
        assert!(!room.is_moderated());
    }

    #[test]
    fn test_room_full() {
        let name = RoomName::new("Test").unwrap();
        let owner = UserId::new();
        let settings = RoomSettings {
            max_members: Some(10),
            ..Default::default()
        };
        let room = Room::with_settings(name, owner, settings);

        assert!(!room.is_full(5));
        assert!(!room.is_full(9));
        assert!(room.is_full(10));
        assert!(room.is_full(15));
    }
}
