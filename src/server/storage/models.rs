//! Data models for lair-chat server storage layer
//!
//! This module defines the core data structures used throughout the storage layer,
//! including users, messages, rooms, sessions, and related entities. These models
//! are designed to be database-agnostic and support serialization/deserialization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User account information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    /// Unique user identifier
    pub id: String,

    /// Username (unique)
    pub username: String,

    /// Email address (optional, unique if provided)
    pub email: Option<String>,

    /// Password hash (Argon2)
    pub password_hash: String,

    /// Password salt
    pub salt: String,

    /// Account creation timestamp
    pub created_at: u64,

    /// Last update timestamp
    pub updated_at: u64,

    /// Last seen timestamp
    pub last_seen: Option<u64>,

    /// Account status
    pub is_active: bool,

    /// User role
    pub role: UserRole,

    /// Additional profile data
    pub profile: UserProfile,

    /// Account settings
    pub settings: UserSettings,
}

/// User roles in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    /// System administrator with full access
    Admin,
    /// Moderator with limited administrative privileges
    Moderator,
    /// Regular user
    User,
    /// Guest user with limited privileges
    Guest,
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::User
    }
}

impl UserRole {
    /// Check if role has administrative privileges
    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    /// Check if role has moderation privileges
    pub fn is_moderator(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Moderator)
    }

    /// Check if role can create rooms
    pub fn can_create_rooms(&self) -> bool {
        !matches!(self, UserRole::Guest)
    }
}

/// User profile information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfile {
    /// Display name (can be different from username)
    pub display_name: Option<String>,

    /// User avatar (base64 encoded or URL)
    pub avatar: Option<String>,

    /// User status message
    pub status_message: Option<String>,

    /// User bio/description
    pub bio: Option<String>,

    /// User timezone
    pub timezone: Option<String>,

    /// User language preference
    pub language: Option<String>,

    /// Additional custom fields
    pub custom_fields: HashMap<String, String>,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            display_name: None,
            avatar: None,
            status_message: None,
            bio: None,
            timezone: None,
            language: None,
            custom_fields: HashMap::new(),
        }
    }
}

/// User settings and preferences
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserSettings {
    /// Theme preference
    pub theme: Option<String>,

    /// Notification settings
    pub notifications: NotificationSettings,

    /// Privacy settings
    pub privacy: PrivacySettings,

    /// Chat preferences
    pub chat: ChatSettings,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            theme: None,
            notifications: NotificationSettings::default(),
            privacy: PrivacySettings::default(),
            chat: ChatSettings::default(),
        }
    }
}

/// Notification preferences
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationSettings {
    /// Enable direct message notifications
    pub direct_messages: bool,

    /// Enable room mention notifications
    pub mentions: bool,

    /// Enable room message notifications
    pub room_messages: bool,

    /// Notification sound enabled
    pub sound_enabled: bool,

    /// Do not disturb mode
    pub do_not_disturb: bool,

    /// Quiet hours start (24h format, e.g., "22:00")
    pub quiet_hours_start: Option<String>,

    /// Quiet hours end (24h format, e.g., "08:00")
    pub quiet_hours_end: Option<String>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            direct_messages: true,
            mentions: true,
            room_messages: false,
            sound_enabled: true,
            do_not_disturb: false,
            quiet_hours_start: None,
            quiet_hours_end: None,
        }
    }
}

/// Privacy settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrivacySettings {
    /// Show online status to others
    pub show_online_status: bool,

    /// Allow direct messages from strangers
    pub allow_stranger_dms: bool,

    /// Show typing indicators
    pub show_typing_indicators: bool,

    /// Show read receipts
    pub show_read_receipts: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            show_online_status: true,
            allow_stranger_dms: true,
            show_typing_indicators: true,
            show_read_receipts: true,
        }
    }
}

/// Chat-specific settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatSettings {
    /// Default font size
    pub font_size: Option<u32>,

    /// Enable emoji shortcuts
    pub emoji_shortcuts: bool,

    /// Enable auto-complete
    pub auto_complete: bool,

    /// Message history limit per room
    pub history_limit: Option<u32>,

    /// Enable message previews
    pub message_previews: bool,
}

impl Default for ChatSettings {
    fn default() -> Self {
        Self {
            font_size: None,
            emoji_shortcuts: true,
            auto_complete: true,
            history_limit: None,
            message_previews: true,
        }
    }
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    /// Unique message identifier
    pub id: String,

    /// Room where the message was sent
    pub room_id: String,

    /// User who sent the message
    pub user_id: String,

    /// Message content
    pub content: String,

    /// Message type
    pub message_type: MessageType,

    /// Message timestamp
    pub timestamp: u64,

    /// Edit timestamp (if message was edited)
    pub edited_at: Option<u64>,

    /// Parent message ID for replies/threading
    pub parent_message_id: Option<String>,

    /// Additional message metadata
    pub metadata: MessageMetadata,

    /// Whether the message is deleted (soft delete)
    pub is_deleted: bool,

    /// Deletion timestamp
    pub deleted_at: Option<u64>,
}

/// Types of messages in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    /// Regular text message
    Text,

    /// System message (user joined, left, etc.)
    System,

    /// File attachment
    File,

    /// Image attachment
    Image,

    /// Voice message
    Voice,

    /// Video message
    Video,

    /// Code snippet
    Code,

    /// Markdown formatted message
    Markdown,

    /// Encrypted message
    Encrypted,
}

impl Default for MessageType {
    fn default() -> Self {
        MessageType::Text
    }
}

/// Message metadata and additional information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageMetadata {
    /// Message reactions
    pub reactions: Vec<MessageReaction>,

    /// Read receipts
    pub read_by: Vec<MessageReadReceipt>,

    /// File attachments
    pub attachments: Vec<FileAttachment>,

    /// Message mentions
    pub mentions: Vec<String>,

    /// Message tags/categories
    pub tags: Vec<String>,

    /// Additional custom metadata
    pub custom: HashMap<String, String>,
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self {
            reactions: Vec::new(),
            read_by: Vec::new(),
            attachments: Vec::new(),
            mentions: Vec::new(),
            tags: Vec::new(),
            custom: HashMap::new(),
        }
    }
}

/// Message reaction (emoji)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageReaction {
    /// User who reacted
    pub user_id: String,

    /// Reaction emoji or identifier
    pub reaction: String,

    /// Reaction timestamp
    pub timestamp: u64,
}

/// Message read receipt
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageReadReceipt {
    /// User who read the message
    pub user_id: String,

    /// Read timestamp
    pub timestamp: u64,
}

/// File attachment information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileAttachment {
    /// Unique file identifier
    pub id: String,

    /// Original filename
    pub filename: String,

    /// File size in bytes
    pub size: u64,

    /// MIME type
    pub mime_type: String,

    /// File hash (for deduplication)
    pub hash: String,

    /// Storage path or URL
    pub storage_path: String,

    /// Upload timestamp
    pub uploaded_at: u64,

    /// Additional file metadata
    pub metadata: HashMap<String, String>,
}

/// Chat room
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Room {
    /// Unique room identifier
    pub id: String,

    /// Room name (unique)
    pub name: String,

    /// Display name for the room
    pub display_name: String,

    /// Room description
    pub description: Option<String>,

    /// Room topic
    pub topic: Option<String>,

    /// Room type
    pub room_type: RoomType,

    /// Room privacy level
    pub privacy: RoomPrivacy,

    /// Room settings
    pub settings: RoomSettings,

    /// User who created the room
    pub created_by: String,

    /// Room creation timestamp
    pub created_at: u64,

    /// Last update timestamp
    pub updated_at: u64,

    /// Whether the room is active
    pub is_active: bool,
}

/// Types of rooms in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RoomType {
    /// Public channel
    Channel,

    /// Private group
    Group,

    /// Direct message conversation
    DirectMessage,

    /// System/announcement channel
    System,

    /// Temporary room
    Temporary,
}

impl Default for RoomType {
    fn default() -> Self {
        RoomType::Channel
    }
}

/// Room privacy levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RoomPrivacy {
    /// Anyone can join
    Public,

    /// Invite-only
    Private,

    /// Password-protected
    Protected,

    /// System room (special access rules)
    System,
}

impl Default for RoomPrivacy {
    fn default() -> Self {
        RoomPrivacy::Public
    }
}

/// Room configuration settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoomSettings {
    /// Maximum number of users
    pub max_users: Option<u32>,

    /// Room password (for protected rooms)
    pub password_hash: Option<String>,

    /// Whether to persist message history
    pub persist_history: bool,

    /// Message history retention in days (0 = unlimited)
    pub history_retention: u32,

    /// Whether file uploads are allowed
    pub allow_file_uploads: bool,

    /// Maximum file size for uploads
    pub max_file_size: Option<u64>,

    /// Allowed file types
    pub allowed_file_types: Vec<String>,

    /// Rate limiting settings
    pub rate_limit: RoomRateLimit,

    /// Moderation settings
    pub moderation: ModerationSettings,

    /// Additional custom settings
    pub custom: HashMap<String, String>,
}

impl Default for RoomSettings {
    fn default() -> Self {
        Self {
            max_users: None,
            password_hash: None,
            persist_history: true,
            history_retention: 0,
            allow_file_uploads: true,
            max_file_size: None,
            allowed_file_types: Vec::new(),
            rate_limit: RoomRateLimit::default(),
            moderation: ModerationSettings::default(),
            custom: HashMap::new(),
        }
    }
}

/// Room-specific rate limiting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoomRateLimit {
    /// Messages per minute per user
    pub messages_per_minute: Option<u32>,

    /// Burst limit
    pub burst_limit: Option<u32>,

    /// Cooldown period in seconds
    pub cooldown: Option<u32>,
}

impl Default for RoomRateLimit {
    fn default() -> Self {
        Self {
            messages_per_minute: None,
            burst_limit: None,
            cooldown: None,
        }
    }
}

/// Room moderation settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModerationSettings {
    /// Auto-moderate messages
    pub auto_moderate: bool,

    /// Require approval for new members
    pub require_approval: bool,

    /// Filter profanity
    pub filter_profanity: bool,

    /// Filter spam
    pub filter_spam: bool,

    /// Allowed mentions per message
    pub max_mentions: Option<u32>,

    /// Forbidden words/phrases
    pub forbidden_words: Vec<String>,
}

impl Default for ModerationSettings {
    fn default() -> Self {
        Self {
            auto_moderate: false,
            require_approval: false,
            filter_profanity: false,
            filter_spam: false,
            max_mentions: None,
            forbidden_words: Vec::new(),
        }
    }
}

/// Room membership information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoomMembership {
    /// Unique membership identifier
    pub id: String,

    /// Room identifier
    pub room_id: String,

    /// User identifier
    pub user_id: String,

    /// User role in the room
    pub role: RoomRole,

    /// Join timestamp
    pub joined_at: u64,

    /// Last activity timestamp
    pub last_activity: Option<u64>,

    /// Whether the user is currently active in the room
    pub is_active: bool,

    /// User-specific room settings
    pub settings: RoomMemberSettings,
}

/// Roles within a room
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RoomRole {
    /// Room owner
    Owner,

    /// Room administrator
    Admin,

    /// Room moderator
    Moderator,

    /// Regular member
    Member,

    /// Guest (limited privileges)
    Guest,
}

impl Default for RoomRole {
    fn default() -> Self {
        RoomRole::Member
    }
}

impl RoomRole {
    /// Check if role can manage the room
    pub fn can_manage_room(&self) -> bool {
        matches!(self, RoomRole::Owner | RoomRole::Admin)
    }

    /// Check if role can moderate messages
    pub fn can_moderate(&self) -> bool {
        matches!(
            self,
            RoomRole::Owner | RoomRole::Admin | RoomRole::Moderator
        )
    }

    /// Check if role can invite users
    pub fn can_invite(&self) -> bool {
        !matches!(self, RoomRole::Guest)
    }
}

/// Member-specific room settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoomMemberSettings {
    /// Custom nickname in this room
    pub nickname: Option<String>,

    /// Notification preferences for this room
    pub notifications: bool,

    /// Whether to show typing indicators in this room
    pub show_typing: bool,

    /// Room-specific theme
    pub theme: Option<String>,

    /// Additional custom settings
    pub custom: HashMap<String, String>,
}

impl Default for RoomMemberSettings {
    fn default() -> Self {
        Self {
            nickname: None,
            notifications: true,
            show_typing: true,
            theme: None,
            custom: HashMap::new(),
        }
    }
}

/// User session information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    /// Unique session identifier
    pub id: String,

    /// User identifier
    pub user_id: String,

    /// Session token
    pub token: String,

    /// Session creation timestamp
    pub created_at: u64,

    /// Session expiration timestamp
    pub expires_at: u64,

    /// Last activity timestamp
    pub last_activity: u64,

    /// IP address
    pub ip_address: Option<String>,

    /// User agent
    pub user_agent: Option<String>,

    /// Whether the session is active
    pub is_active: bool,

    /// Session metadata
    pub metadata: SessionMetadata,
}

/// Session metadata and additional information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionMetadata {
    /// Client type (desktop, mobile, web)
    pub client_type: Option<String>,

    /// Client version
    pub client_version: Option<String>,

    /// Device information
    pub device_info: Option<String>,

    /// Location information (if available)
    pub location: Option<String>,

    /// Additional custom metadata
    pub custom: HashMap<String, String>,
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self {
            client_type: None,
            client_version: None,
            device_info: None,
            location: None,
            custom: HashMap::new(),
        }
    }
}

/// Search query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search term
    pub query: String,

    /// Room to search in (optional)
    pub room_id: Option<String>,

    /// User to search for (optional)
    pub user_id: Option<String>,

    /// Message type filter
    pub message_type: Option<MessageType>,

    /// Date range start
    pub date_from: Option<u64>,

    /// Date range end
    pub date_to: Option<u64>,

    /// Maximum results
    pub limit: Option<u64>,

    /// Result offset
    pub offset: Option<u64>,
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Matching messages
    pub messages: Vec<Message>,

    /// Total number of matches
    pub total_count: u64,

    /// Whether there are more results
    pub has_more: bool,

    /// Search execution time in milliseconds
    pub execution_time: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_permissions() {
        assert!(UserRole::Admin.is_admin());
        assert!(UserRole::Admin.is_moderator());
        assert!(UserRole::Admin.can_create_rooms());

        assert!(!UserRole::Moderator.is_admin());
        assert!(UserRole::Moderator.is_moderator());
        assert!(UserRole::Moderator.can_create_rooms());

        assert!(!UserRole::User.is_admin());
        assert!(!UserRole::User.is_moderator());
        assert!(UserRole::User.can_create_rooms());

        assert!(!UserRole::Guest.can_create_rooms());
    }

    #[test]
    fn test_room_role_permissions() {
        assert!(RoomRole::Owner.can_manage_room());
        assert!(RoomRole::Owner.can_moderate());
        assert!(RoomRole::Owner.can_invite());

        assert!(RoomRole::Admin.can_manage_room());
        assert!(RoomRole::Admin.can_moderate());
        assert!(RoomRole::Admin.can_invite());

        assert!(!RoomRole::Moderator.can_manage_room());
        assert!(RoomRole::Moderator.can_moderate());
        assert!(RoomRole::Moderator.can_invite());

        assert!(!RoomRole::Member.can_manage_room());
        assert!(!RoomRole::Member.can_moderate());
        assert!(RoomRole::Member.can_invite());

        assert!(!RoomRole::Guest.can_invite());
    }

    #[test]
    fn test_default_values() {
        let user_role = UserRole::default();
        assert_eq!(user_role, UserRole::User);

        let room_type = RoomType::default();
        assert_eq!(room_type, RoomType::Channel);

        let room_privacy = RoomPrivacy::default();
        assert_eq!(room_privacy, RoomPrivacy::Public);

        let message_type = MessageType::default();
        assert_eq!(message_type, MessageType::Text);
    }

    #[test]
    fn test_serialization() {
        let user = User {
            id: "user1".to_string(),
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            password_hash: "hash".to_string(),
            salt: "salt".to_string(),
            created_at: 1234567890,
            updated_at: 1234567890,
            last_seen: Some(1234567890),
            is_active: true,
            role: UserRole::User,
            profile: UserProfile::default(),
            settings: UserSettings::default(),
        };

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(user, deserialized);
    }
}
