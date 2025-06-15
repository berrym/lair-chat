//! Room management API models
//!
//! This module contains all data structures related to room management,
//! including room creation, membership, permissions, and room-related operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Room information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Room {
    /// Room ID
    pub id: Uuid,

    /// Room name
    pub name: String,

    /// Room description
    pub description: Option<String>,

    /// Room type
    pub room_type: RoomType,

    /// Privacy level
    pub privacy: PrivacyLevel,

    /// Room owner ID
    pub owner_id: Uuid,

    /// Maximum number of members (None = unlimited)
    pub max_members: Option<u32>,

    /// Room creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Room avatar URL
    pub avatar_url: Option<String>,

    /// Room metadata
    pub metadata: serde_json::Value,
}

/// Room types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum RoomType {
    /// Public channel
    Channel,
    /// Private group
    Group,
    /// Direct message between two users
    DirectMessage,
    /// System room for announcements
    System,
    /// Temporary room that auto-deletes
    Temporary,
}

/// Privacy levels for rooms
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PrivacyLevel {
    /// Anyone can join
    Public,
    /// Invitation required
    Private,
    /// Password protected
    Protected,
    /// System-only access
    System,
}

/// Create room request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateRoomRequest {
    /// Room name (3-100 characters)
    #[validate(length(min = 3, max = 100))]
    pub name: String,

    /// Room description (optional, max 500 characters)
    #[validate(length(max = 500))]
    pub description: Option<String>,

    /// Room type
    pub room_type: RoomType,

    /// Privacy level
    pub privacy: PrivacyLevel,

    /// Maximum number of members
    #[validate(range(min = 2, max = 10000))]
    pub max_members: Option<u32>,

    /// Room password (for protected rooms)
    pub password: Option<String>,

    /// Initial members to invite
    pub initial_members: Option<Vec<Uuid>>,
}

/// Update room request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateRoomRequest {
    /// Room name
    #[validate(length(min = 3, max = 100))]
    pub name: Option<String>,

    /// Room description
    #[validate(length(max = 500))]
    pub description: Option<String>,

    /// Privacy level
    pub privacy: Option<PrivacyLevel>,

    /// Maximum number of members
    #[validate(range(min = 2, max = 10000))]
    pub max_members: Option<u32>,

    /// Room avatar URL
    #[validate(url)]
    pub avatar_url: Option<String>,
}

/// Room membership information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoomMembership {
    /// Room ID
    pub room_id: Uuid,

    /// User ID
    pub user_id: Uuid,

    /// Member role in the room
    pub role: MemberRole,

    /// When the user joined
    pub joined_at: DateTime<Utc>,

    /// Last activity in the room
    pub last_activity: Option<DateTime<Utc>>,

    /// Membership status
    pub status: MembershipStatus,
}

/// Member roles in rooms
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MemberRole {
    /// Room owner (full permissions)
    Owner,
    /// Room administrator
    Admin,
    /// Room moderator
    Moderator,
    /// Regular member
    Member,
    /// Guest (limited permissions)
    Guest,
}

/// Membership status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MembershipStatus {
    /// Active member
    Active,
    /// Invited but not joined
    Invited,
    /// Left the room
    Left,
    /// Kicked from the room
    Kicked,
    /// Banned from the room
    Banned,
}

/// Room member information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoomMember {
    /// User ID
    pub user_id: Uuid,

    /// Username
    pub username: String,

    /// Display name
    pub display_name: String,

    /// Avatar URL
    pub avatar_url: Option<String>,

    /// Role in the room
    pub role: MemberRole,

    /// Online status
    pub is_online: bool,

    /// When they joined the room
    pub joined_at: DateTime<Utc>,

    /// Last activity in the room
    pub last_activity: Option<DateTime<Utc>>,
}

/// Join room request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct JoinRoomRequest {
    /// Room password (for protected rooms)
    pub password: Option<String>,
}

/// Invite users to room request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct InviteUsersRequest {
    /// User IDs to invite
    #[validate(length(min = 1, max = 50))]
    pub user_ids: Vec<Uuid>,

    /// Optional invitation message
    #[validate(length(max = 200))]
    pub message: Option<String>,
}

/// Update member role request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateMemberRoleRequest {
    /// New role for the member
    pub role: MemberRole,

    /// Reason for role change
    pub reason: Option<String>,
}

/// Kick/ban member request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct KickBanMemberRequest {
    /// Reason for kick/ban
    #[validate(length(min = 1, max = 200))]
    pub reason: String,

    /// Whether this is a permanent ban
    #[serde(default)]
    pub permanent: bool,
}

/// Room search request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RoomSearchRequest {
    /// Search query
    #[validate(length(min = 2, max = 100))]
    pub query: String,

    /// Room type filter
    pub room_type: Option<RoomType>,

    /// Privacy level filter
    pub privacy: Option<PrivacyLevel>,

    /// Maximum results
    #[validate(range(min = 1, max = 50))]
    pub limit: Option<u32>,
}

/// Room search result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoomSearchResult {
    /// Room ID
    pub id: Uuid,

    /// Room name
    pub name: String,

    /// Room description
    pub description: Option<String>,

    /// Room type
    pub room_type: RoomType,

    /// Privacy level
    pub privacy: PrivacyLevel,

    /// Member count
    pub member_count: u32,

    /// Maximum members (None = unlimited)
    pub max_members: Option<u32>,

    /// Room avatar URL
    pub avatar_url: Option<String>,

    /// Whether the current user is a member
    pub is_member: bool,

    /// Whether the room requires a password
    pub requires_password: bool,
}

/// Room statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoomStatistics {
    /// Room ID
    pub room_id: Uuid,

    /// Total members
    pub member_count: u32,

    /// Active members (online now)
    pub active_members: u32,

    /// Total messages sent
    pub total_messages: u64,

    /// Messages sent today
    pub messages_today: u32,

    /// Room creation date
    pub created_at: DateTime<Utc>,

    /// Last message timestamp
    pub last_message_at: Option<DateTime<Utc>>,

    /// Statistics last updated
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_create_room_request_validation() {
        let valid_request = CreateRoomRequest {
            name: "Test Room".to_string(),
            description: Some("A test room".to_string()),
            room_type: RoomType::Channel,
            privacy: PrivacyLevel::Public,
            max_members: Some(100),
            password: None,
            initial_members: None,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = CreateRoomRequest {
            name: "AB".to_string(), // Too short
            description: None,
            room_type: RoomType::Channel,
            privacy: PrivacyLevel::Public,
            max_members: None,
            password: None,
            initial_members: None,
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_room_search_request_validation() {
        let valid_request = RoomSearchRequest {
            query: "test".to_string(),
            room_type: Some(RoomType::Channel),
            privacy: Some(PrivacyLevel::Public),
            limit: Some(20),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = RoomSearchRequest {
            query: "a".to_string(), // Too short
            room_type: None,
            privacy: None,
            limit: Some(100), // Too high
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_invite_users_request_validation() {
        let valid_request = InviteUsersRequest {
            user_ids: vec![Uuid::new_v4()],
            message: Some("Join us!".to_string()),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = InviteUsersRequest {
            user_ids: vec![],               // Empty list
            message: Some("a".repeat(300)), // Too long
        };
        assert!(invalid_request.validate().is_err());
    }
}
