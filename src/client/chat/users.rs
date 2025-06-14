//! Room user structures for Lair-Chat
//! Handles user roles, status, and permissions within chat rooms.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::UserId;

/// User role within a room
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserRole {
    /// Room administrator (full permissions)
    Admin,
    /// Room moderator (can kick users, delete messages)
    Moderator,
    /// Regular user
    User,
    /// Guest user (limited permissions)
    Guest,
}

impl UserRole {
    /// Check if role has permission for action
    pub fn can_perform(&self, action: &str) -> bool {
        match action {
            "send_message" => true, // All roles can send messages
            "edit_own_message" => !matches!(self, UserRole::Guest),
            "delete_own_message" => !matches!(self, UserRole::Guest),
            "add_reaction" => true, // All roles can react
            "upload_file" => !matches!(self, UserRole::Guest),
            "kick_user" => matches!(self, UserRole::Admin | UserRole::Moderator),
            "ban_user" => matches!(self, UserRole::Admin | UserRole::Moderator),
            "delete_any_message" => matches!(self, UserRole::Admin | UserRole::Moderator),
            "change_room_settings" => matches!(self, UserRole::Admin),
            "delete_room" => matches!(self, UserRole::Admin),
            "promote_user" => matches!(self, UserRole::Admin),
            "demote_user" => matches!(self, UserRole::Admin),
            _ => false,
        }
    }

    /// Get role priority (higher number = more permissions)
    pub fn priority(&self) -> u8 {
        match self {
            UserRole::Guest => 0,
            UserRole::User => 1,
            UserRole::Moderator => 2,
            UserRole::Admin => 3,
        }
    }

    /// Check if this role can modify another role
    pub fn can_modify(&self, other: &UserRole) -> bool {
        self.priority() > other.priority()
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "Admin"),
            UserRole::Moderator => write!(f, "Moderator"),
            UserRole::User => write!(f, "User"),
            UserRole::Guest => write!(f, "Guest"),
        }
    }
}

/// User status within a room
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserStatus {
    /// User is online and active
    Online,
    /// User is online but idle
    Idle,
    /// User is online but away
    Away,
    /// User is offline
    Offline,
    /// User is banned from the room
    Banned,
    /// User has left the room
    Left,
}

impl UserStatus {
    /// Check if user is available for interaction
    pub fn is_available(&self) -> bool {
        matches!(self, UserStatus::Online | UserStatus::Idle)
    }

    /// Check if user is present in the room
    pub fn is_present(&self) -> bool {
        !matches!(
            self,
            UserStatus::Offline | UserStatus::Banned | UserStatus::Left
        )
    }
}

impl std::fmt::Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserStatus::Online => write!(f, "Online"),
            UserStatus::Idle => write!(f, "Idle"),
            UserStatus::Away => write!(f, "Away"),
            UserStatus::Offline => write!(f, "Offline"),
            UserStatus::Banned => write!(f, "Banned"),
            UserStatus::Left => write!(f, "Left"),
        }
    }
}

/// User permissions within a room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissions {
    /// Can send messages
    pub send_messages: bool,
    /// Can edit own messages
    pub edit_messages: bool,
    /// Can delete own messages
    pub delete_messages: bool,
    /// Can add reactions
    pub add_reactions: bool,
    /// Can upload files
    pub upload_files: bool,
    /// Can mention all users
    pub mention_all: bool,
    /// Can create threads
    pub create_threads: bool,
    /// Can use voice features
    pub use_voice: bool,
}

impl UserPermissions {
    /// Create default permissions for role
    pub fn for_role(role: &UserRole) -> Self {
        match role {
            UserRole::Admin => Self {
                send_messages: true,
                edit_messages: true,
                delete_messages: true,
                add_reactions: true,
                upload_files: true,
                mention_all: true,
                create_threads: true,
                use_voice: true,
            },
            UserRole::Moderator => Self {
                send_messages: true,
                edit_messages: true,
                delete_messages: true,
                add_reactions: true,
                upload_files: true,
                mention_all: true,
                create_threads: true,
                use_voice: true,
            },
            UserRole::User => Self {
                send_messages: true,
                edit_messages: true,
                delete_messages: true,
                add_reactions: true,
                upload_files: true,
                mention_all: false,
                create_threads: true,
                use_voice: true,
            },
            UserRole::Guest => Self {
                send_messages: true,
                edit_messages: false,
                delete_messages: false,
                add_reactions: true,
                upload_files: false,
                mention_all: false,
                create_threads: false,
                use_voice: false,
            },
        }
    }

    /// Check if all permissions are disabled
    pub fn is_muted(&self) -> bool {
        !self.send_messages
    }
}

/// User activity tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivity {
    /// When user joined the room
    pub joined_at: u64,
    /// Last seen timestamp
    pub last_seen: u64,
    /// Last message timestamp
    pub last_message: Option<u64>,
    /// Total messages sent
    pub message_count: u64,
    /// Total time spent in room (seconds)
    pub total_time: u64,
}

impl UserActivity {
    /// Create new activity tracker
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            joined_at: now,
            last_seen: now,
            last_message: None,
            message_count: 0,
            total_time: 0,
        }
    }

    /// Update last seen timestamp
    pub fn update_last_seen(&mut self) {
        self.last_seen = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Record a message sent
    pub fn record_message(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.last_message = Some(now);
        self.message_count += 1;
        self.update_last_seen();
    }

    /// Get time since last seen (seconds)
    pub fn time_since_last_seen(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(self.last_seen)
    }

    /// Get time in room (seconds)
    pub fn time_in_room(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(self.joined_at)
    }
}

impl Default for UserActivity {
    fn default() -> Self {
        Self::new()
    }
}

/// Room user structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomUser {
    /// User identifier
    pub user_id: UserId,
    /// Username
    pub username: String,
    /// Display name (optional)
    pub display_name: Option<String>,
    /// User role in this room
    pub role: UserRole,
    /// Current status
    pub status: UserStatus,
    /// User permissions
    pub permissions: UserPermissions,
    /// Activity tracking
    pub activity: UserActivity,
    /// User avatar URL
    pub avatar_url: Option<String>,
    /// Custom user color
    pub color: Option<String>,
    /// Whether user is currently typing
    pub is_typing: bool,
    /// User's current device/client
    pub device: Option<String>,
}

impl RoomUser {
    /// Create a new room user
    pub fn new(user_id: UserId, username: String, role: UserRole) -> Self {
        let permissions = UserPermissions::for_role(&role);

        Self {
            user_id,
            username,
            display_name: None,
            role: role.clone(),
            status: UserStatus::Online,
            permissions,
            activity: UserActivity::new(),
            avatar_url: None,
            color: None,
            is_typing: false,
            device: None,
        }
    }

    /// Create an admin user
    pub fn new_admin(user_id: UserId, username: String) -> Self {
        Self::new(user_id, username, UserRole::Admin)
    }

    /// Create a moderator user
    pub fn new_moderator(user_id: UserId, username: String) -> Self {
        Self::new(user_id, username, UserRole::Moderator)
    }

    /// Create a guest user
    pub fn new_guest(user_id: UserId, username: String) -> Self {
        Self::new(user_id, username, UserRole::Guest)
    }

    /// Get display name or username
    pub fn display_name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.username)
    }

    /// Update user status
    pub fn set_status(&mut self, status: UserStatus) {
        self.status = status;
        self.activity.update_last_seen();
    }

    /// Update user role
    pub fn set_role(&mut self, role: UserRole) {
        self.role = role.clone();
        self.permissions = UserPermissions::for_role(&role);
    }

    /// Set typing status
    pub fn set_typing(&mut self, typing: bool) {
        self.is_typing = typing;
        if typing {
            self.activity.update_last_seen();
        }
    }

    /// Record a message sent by this user
    pub fn record_message(&mut self) {
        self.activity.record_message();
    }

    /// Check if user can perform action
    pub fn can_perform(&self, action: &str) -> bool {
        // Check if user is banned or has left
        if !self.status.is_present() {
            return false;
        }

        // Check role permissions
        if !self.role.can_perform(action) {
            return false;
        }

        // Check specific permissions
        match action {
            "send_message" => self.permissions.send_messages,
            "edit_message" => self.permissions.edit_messages,
            "delete_message" => self.permissions.delete_messages,
            "add_reaction" => self.permissions.add_reactions,
            "upload_file" => self.permissions.upload_files,
            "mention_all" => self.permissions.mention_all,
            "create_thread" => self.permissions.create_threads,
            "use_voice" => self.permissions.use_voice,
            _ => self.role.can_perform(action),
        }
    }

    /// Check if user can modify another user
    pub fn can_modify_user(&self, other: &RoomUser) -> bool {
        self.status.is_present() && self.role.can_modify(&other.role)
    }

    /// Get user statistics
    pub fn get_stats(&self) -> UserStats {
        UserStats {
            message_count: self.activity.message_count,
            time_in_room: self.activity.time_in_room(),
            time_since_last_seen: self.activity.time_since_last_seen(),
            joined_at: self.activity.joined_at,
            last_message: self.activity.last_message,
        }
    }

    /// Check if user is active (recently seen)
    pub fn is_active(&self, threshold_seconds: u64) -> bool {
        self.activity.time_since_last_seen() < threshold_seconds
    }

    /// Mute user (remove send permission)
    pub fn mute(&mut self) {
        self.permissions.send_messages = false;
    }

    /// Unmute user (restore send permission)
    pub fn unmute(&mut self) {
        self.permissions.send_messages = true;
    }

    /// Check if user is muted
    pub fn is_muted(&self) -> bool {
        self.permissions.is_muted()
    }
}

/// User statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    pub message_count: u64,
    pub time_in_room: u64,
    pub time_since_last_seen: u64,
    pub joined_at: u64,
    pub last_message: Option<u64>,
}

impl UserStats {
    /// Get messages per hour rate
    pub fn messages_per_hour(&self) -> f64 {
        if self.time_in_room == 0 {
            0.0
        } else {
            (self.message_count as f64) / ((self.time_in_room as f64) / 3600.0)
        }
    }

    /// Get human-readable time in room
    pub fn human_time_in_room(&self) -> String {
        let hours = self.time_in_room / 3600;
        let minutes = (self.time_in_room % 3600) / 60;

        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_permissions() {
        assert!(UserRole::Admin.can_perform("delete_room"));
        assert!(UserRole::Moderator.can_perform("kick_user"));
        assert!(!UserRole::User.can_perform("kick_user"));
        assert!(!UserRole::Guest.can_perform("upload_file"));

        assert!(UserRole::Admin.can_modify(&UserRole::User));
        assert!(!UserRole::User.can_modify(&UserRole::Admin));
    }

    #[test]
    fn test_user_status() {
        assert!(UserStatus::Online.is_available());
        assert!(UserStatus::Idle.is_available());
        assert!(!UserStatus::Away.is_available());
        assert!(!UserStatus::Offline.is_available());

        assert!(UserStatus::Online.is_present());
        assert!(!UserStatus::Banned.is_present());
        assert!(!UserStatus::Left.is_present());
    }

    #[test]
    fn test_room_user_creation() {
        let user_id = uuid::Uuid::new_v4();
        let username = "testuser".to_string();

        let user = RoomUser::new(user_id, username.clone(), UserRole::User);

        assert_eq!(user.user_id, user_id);
        assert_eq!(user.username, username);
        assert_eq!(user.role, UserRole::User);
        assert_eq!(user.status, UserStatus::Online);
        assert!(user.can_perform("send_message"));
        assert!(!user.can_perform("kick_user"));
    }

    #[test]
    fn test_user_activity() {
        let mut activity = UserActivity::new();

        assert_eq!(activity.message_count, 0);
        assert!(activity.last_message.is_none());

        activity.record_message();

        assert_eq!(activity.message_count, 1);
        assert!(activity.last_message.is_some());
    }

    #[test]
    fn test_user_permissions() {
        let admin_perms = UserPermissions::for_role(&UserRole::Admin);
        let guest_perms = UserPermissions::for_role(&UserRole::Guest);

        assert!(admin_perms.mention_all);
        assert!(!guest_perms.mention_all);
        assert!(!guest_perms.upload_files);
        assert!(admin_perms.upload_files);
    }

    #[test]
    fn test_user_muting() {
        let mut user = RoomUser::new(uuid::Uuid::new_v4(), "testuser".to_string(), UserRole::User);

        assert!(!user.is_muted());
        assert!(user.can_perform("send_message"));

        user.mute();

        assert!(user.is_muted());
        assert!(!user.can_perform("send_message"));

        user.unmute();

        assert!(!user.is_muted());
        assert!(user.can_perform("send_message"));
    }
}
