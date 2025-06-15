//! User management API models
//!
//! This module contains all data structures related to user management,
//! including user profiles, settings, and user-related operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::api::models::auth::{UserRole, UserStatus};

/// User profile information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    /// User ID
    pub id: Uuid,

    /// Username
    pub username: String,

    /// Email address
    pub email: String,

    /// Display name
    pub display_name: String,

    /// User role
    pub role: UserRole,

    /// Account status
    pub status: UserStatus,

    /// Avatar URL
    pub avatar_url: Option<String>,

    /// User timezone
    pub timezone: String,

    /// Last login timestamp
    pub last_login: Option<DateTime<Utc>>,

    /// Account creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Update user profile request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateProfileRequest {
    /// Display name (1-100 characters)
    #[validate(length(min = 1, max = 100))]
    pub display_name: Option<String>,

    /// Avatar URL
    #[validate(url)]
    pub avatar_url: Option<String>,

    /// User timezone (IANA timezone identifier)
    pub timezone: Option<String>,
}

/// User settings
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserSettings {
    /// User ID
    pub user_id: Uuid,

    /// Email notifications enabled
    pub email_notifications: bool,

    /// Push notifications enabled
    pub push_notifications: bool,

    /// Desktop notifications enabled
    pub desktop_notifications: bool,

    /// Sound notifications enabled
    pub sound_notifications: bool,

    /// Theme preference
    pub theme: UserTheme,

    /// Language preference
    pub language: String,

    /// Timezone
    pub timezone: String,

    /// Privacy settings
    pub privacy: PrivacySettings,

    /// Settings last updated
    pub updated_at: DateTime<Utc>,
}

/// User theme preference
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum UserTheme {
    /// Light theme
    Light,
    /// Dark theme
    Dark,
    /// System theme (follows OS setting)
    System,
}

impl Default for UserTheme {
    fn default() -> Self {
        Self::System
    }
}

/// Privacy settings
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrivacySettings {
    /// Show online status to others
    pub show_online_status: bool,

    /// Allow direct messages from non-friends
    pub allow_direct_messages: bool,

    /// Show read receipts
    pub show_read_receipts: bool,

    /// Show typing indicators
    pub show_typing_indicators: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            show_online_status: true,
            allow_direct_messages: true,
            show_read_receipts: true,
            show_typing_indicators: true,
        }
    }
}

/// Update user settings request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateSettingsRequest {
    /// Email notifications enabled
    pub email_notifications: Option<bool>,

    /// Push notifications enabled
    pub push_notifications: Option<bool>,

    /// Desktop notifications enabled
    pub desktop_notifications: Option<bool>,

    /// Sound notifications enabled
    pub sound_notifications: Option<bool>,

    /// Theme preference
    pub theme: Option<UserTheme>,

    /// Language preference (ISO 639-1 code)
    pub language: Option<String>,

    /// Timezone (IANA timezone identifier)
    pub timezone: Option<String>,

    /// Privacy settings
    pub privacy: Option<PrivacySettings>,
}

/// User search request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UserSearchRequest {
    /// Search query (username, display name, or email)
    #[validate(length(min = 2, max = 100))]
    pub query: String,

    /// Maximum number of results
    #[validate(range(min = 1, max = 50))]
    pub limit: Option<u32>,
}

/// User search result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserSearchResult {
    /// User ID
    pub id: Uuid,

    /// Username
    pub username: String,

    /// Display name
    pub display_name: String,

    /// Avatar URL
    pub avatar_url: Option<String>,

    /// Online status
    pub is_online: bool,

    /// Last seen timestamp (if not online)
    pub last_seen: Option<DateTime<Utc>>,
}

/// User activity information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserActivity {
    /// User ID
    pub user_id: Uuid,

    /// Online status
    pub is_online: bool,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Current status message
    pub status_message: Option<String>,

    /// Activity type (typing, idle, etc.)
    pub activity_type: ActivityType,
}

/// User activity types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ActivityType {
    /// User is actively using the application
    Active,
    /// User is idle
    Idle,
    /// User is typing
    Typing,
    /// User is away
    Away,
    /// User is in do not disturb mode
    DoNotDisturb,
}

impl Default for ActivityType {
    fn default() -> Self {
        Self::Active
    }
}

/// User statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserStatistics {
    /// User ID
    pub user_id: Uuid,

    /// Total messages sent
    pub messages_sent: u64,

    /// Total messages received
    pub messages_received: u64,

    /// Number of rooms joined
    pub rooms_joined: u32,

    /// Number of direct message conversations
    pub dm_conversations: u32,

    /// Account age in days
    pub account_age_days: u32,

    /// Last login timestamp
    pub last_login: Option<DateTime<Utc>>,

    /// Total online time in minutes
    pub total_online_minutes: u64,

    /// Statistics last updated
    pub updated_at: DateTime<Utc>,
}

/// Block/unblock user request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlockUserRequest {
    /// User ID to block/unblock
    pub user_id: Uuid,

    /// Reason for blocking (optional)
    pub reason: Option<String>,
}

/// Blocked user information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlockedUser {
    /// Blocked user ID
    pub user_id: Uuid,

    /// Blocked user username
    pub username: String,

    /// Blocked user display name
    pub display_name: String,

    /// Reason for blocking
    pub reason: Option<String>,

    /// When the user was blocked
    pub blocked_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_theme_default() {
        let theme = UserTheme::default();
        assert!(matches!(theme, UserTheme::System));
    }

    #[test]
    fn test_privacy_settings_default() {
        let privacy = PrivacySettings::default();
        assert!(privacy.show_online_status);
        assert!(privacy.allow_direct_messages);
        assert!(privacy.show_read_receipts);
        assert!(privacy.show_typing_indicators);
    }

    #[test]
    fn test_activity_type_default() {
        let activity = ActivityType::default();
        assert!(matches!(activity, ActivityType::Active));
    }

    #[test]
    fn test_user_search_request_validation() {
        use validator::Validate;

        let valid_request = UserSearchRequest {
            query: "test user".to_string(),
            limit: Some(10),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = UserSearchRequest {
            query: "a".to_string(), // Too short
            limit: Some(10),
        };
        assert!(invalid_request.validate().is_err());

        let invalid_request = UserSearchRequest {
            query: "valid query".to_string(),
            limit: Some(100), // Too high
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_update_profile_request_validation() {
        use validator::Validate;

        let valid_request = UpdateProfileRequest {
            display_name: Some("Valid Name".to_string()),
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
            timezone: Some("UTC".to_string()),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = UpdateProfileRequest {
            display_name: Some("".to_string()),        // Empty string
            avatar_url: Some("not-a-url".to_string()), // Invalid URL
            timezone: None,
        };
        assert!(invalid_request.validate().is_err());
    }
}
