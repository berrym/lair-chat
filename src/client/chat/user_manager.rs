//! User management and presence tracking for Lair-Chat
//! Handles online user discovery, presence updates, and user profile caching.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use super::{UserId, UserStatus};

/// User presence information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresence {
    /// User identifier
    pub user_id: UserId,
    /// Username
    pub username: String,
    /// Display name (optional)
    pub display_name: Option<String>,
    /// Current status
    pub status: UserStatus,
    /// Last seen timestamp
    pub last_seen: u64,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Custom status message
    pub status_message: Option<String>,
    /// Whether user is currently typing (and to whom)
    pub is_typing_to: Option<UserId>,
    /// User's current device/client
    pub device: Option<String>,
    /// When user came online
    pub online_since: Option<u64>,
}

impl UserPresence {
    /// Create new user presence
    pub fn new(user_id: UserId, username: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            user_id,
            username,
            display_name: None,
            status: UserStatus::Online,
            last_seen: now,
            avatar_url: None,
            status_message: None,
            is_typing_to: None,
            device: None,
            online_since: Some(now),
        }
    }

    /// Update user status
    pub fn set_status(&mut self, status: UserStatus) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.status = status.clone();
        self.last_seen = now;

        // Set online_since when coming online
        if matches!(status, UserStatus::Online) && self.online_since.is_none() {
            self.online_since = Some(now);
        }

        // Clear online_since when going offline
        if matches!(status, UserStatus::Offline) {
            self.online_since = None;
        }
    }

    /// Update last seen timestamp
    pub fn update_last_seen(&mut self) {
        self.last_seen = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Set typing status
    pub fn set_typing(&mut self, typing_to: Option<UserId>) {
        self.is_typing_to = typing_to;
        if typing_to.is_some() {
            self.update_last_seen();
        }
    }

    /// Check if user is available for messaging
    pub fn is_available(&self) -> bool {
        self.status.is_available()
    }

    /// Check if user is currently online
    pub fn is_online(&self) -> bool {
        matches!(self.status, UserStatus::Online | UserStatus::Idle)
    }

    /// Get display name or username
    pub fn display_name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.username)
    }

    /// Get time since last seen (seconds)
    pub fn time_since_last_seen(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(self.last_seen)
    }

    /// Get human-readable last seen time
    pub fn human_last_seen(&self) -> String {
        let time_since = self.time_since_last_seen();

        if time_since < 60 {
            "Just now".to_string()
        } else if time_since < 3600 {
            format!("{}m ago", time_since / 60)
        } else if time_since < 86400 {
            format!("{}h ago", time_since / 3600)
        } else {
            format!("{}d ago", time_since / 86400)
        }
    }

    /// Get online duration (seconds)
    pub fn online_duration(&self) -> Option<u64> {
        self.online_since.map(|since| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_sub(since)
        })
    }

    /// Check if user is typing to specific recipient
    pub fn is_typing_to(&self, recipient: UserId) -> bool {
        self.is_typing_to == Some(recipient)
    }
}

/// User profile information (extended user data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// User identifier
    pub user_id: UserId,
    /// Username
    pub username: String,
    /// Display name
    pub display_name: Option<String>,
    /// Email address
    pub email: Option<String>,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// User bio/description
    pub bio: Option<String>,
    /// User preferences
    pub preferences: HashMap<String, String>,
    /// When user account was created
    pub created_at: u64,
    /// Last profile update
    pub updated_at: u64,
    /// User verification status
    pub is_verified: bool,
    /// User role/permissions
    pub role: String,
}

impl UserProfile {
    /// Create new user profile
    pub fn new(user_id: UserId, username: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            user_id,
            username,
            display_name: None,
            email: None,
            avatar_url: None,
            bio: None,
            preferences: HashMap::new(),
            created_at: now,
            updated_at: now,
            is_verified: false,
            role: "user".to_string(),
        }
    }

    /// Get display name or username
    pub fn display_name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.username)
    }

    /// Update profile
    pub fn update(&mut self) {
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Set preference
    pub fn set_preference(&mut self, key: String, value: String) {
        self.preferences.insert(key, value);
        self.update();
    }

    /// Get preference
    pub fn get_preference(&self, key: &str) -> Option<&String> {
        self.preferences.get(key)
    }

    /// Convert to user presence
    pub fn to_presence(&self, status: UserStatus) -> UserPresence {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        UserPresence {
            user_id: self.user_id,
            username: self.username.clone(),
            display_name: self.display_name.clone(),
            status: status.clone(),
            last_seen: now,
            avatar_url: self.avatar_url.clone(),
            status_message: None,
            is_typing_to: None,
            device: None,
            online_since: if matches!(status, UserStatus::Online) {
                Some(now)
            } else {
                None
            },
        }
    }
}

/// User search and filtering criteria
#[derive(Debug, Clone, Default)]
pub struct UserFilter {
    /// Filter by status
    pub status: Option<UserStatus>,
    /// Search by username/display name
    pub search_term: Option<String>,
    /// Only online users
    pub online_only: bool,
    /// Only available users (online/idle)
    pub available_only: bool,
    /// Exclude specific users
    pub exclude_users: Vec<UserId>,
    /// Maximum results
    pub limit: Option<usize>,
}

impl UserFilter {
    /// Create new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter for online users only
    pub fn online_only() -> Self {
        Self {
            online_only: true,
            ..Default::default()
        }
    }

    /// Filter for available users only
    pub fn available_only() -> Self {
        Self {
            available_only: true,
            ..Default::default()
        }
    }

    /// Filter with search term
    pub fn with_search(search_term: String) -> Self {
        Self {
            search_term: Some(search_term),
            ..Default::default()
        }
    }

    /// Filter excluding specific users
    pub fn excluding(users: Vec<UserId>) -> Self {
        Self {
            exclude_users: users,
            ..Default::default()
        }
    }

    /// Set limit on results
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Check if user matches filter criteria
    pub fn matches(&self, presence: &UserPresence) -> bool {
        // Check status filter
        if let Some(status) = &self.status {
            if presence.status != *status {
                return false;
            }
        }

        // Check online only
        if self.online_only && !presence.is_online() {
            return false;
        }

        // Check available only
        if self.available_only && !presence.is_available() {
            return false;
        }

        // Check exclusions
        if self.exclude_users.contains(&presence.user_id) {
            return false;
        }

        // Check search term
        if let Some(term) = &self.search_term {
            let term_lower = term.to_lowercase();
            let username_matches = presence.username.to_lowercase().contains(&term_lower);
            let display_name_matches = presence
                .display_name
                .as_ref()
                .map(|name| name.to_lowercase().contains(&term_lower))
                .unwrap_or(false);

            if !username_matches && !display_name_matches {
                return false;
            }
        }

        true
    }
}

/// User manager for tracking online users and managing presence
pub struct UserManager {
    /// Online users by ID
    online_users: Arc<RwLock<HashMap<UserId, UserPresence>>>,
    /// User profile cache
    user_cache: Arc<RwLock<HashMap<UserId, UserProfile>>>,
    /// Presence update timeout (seconds)
    presence_timeout: u64,
}

impl UserManager {
    /// Create new user manager
    pub fn new() -> Self {
        Self {
            online_users: Arc::new(RwLock::new(HashMap::new())),
            user_cache: Arc::new(RwLock::new(HashMap::new())),
            presence_timeout: 300, // 5 minutes
        }
    }

    /// Create user manager with custom timeout
    pub fn with_timeout(timeout_seconds: u64) -> Self {
        Self {
            online_users: Arc::new(RwLock::new(HashMap::new())),
            user_cache: Arc::new(RwLock::new(HashMap::new())),
            presence_timeout: timeout_seconds,
        }
    }

    /// Add or update user presence
    pub async fn update_user_presence(&self, presence: UserPresence) {
        let mut users = self.online_users.write().await;
        users.insert(presence.user_id, presence);
    }

    /// Remove user (went offline)
    pub async fn remove_user(&self, user_id: UserId) {
        let mut users = self.online_users.write().await;
        users.remove(&user_id);
    }

    /// Get user presence
    pub async fn get_user_presence(&self, user_id: UserId) -> Option<UserPresence> {
        let users = self.online_users.read().await;
        users.get(&user_id).cloned()
    }

    /// Get all online users
    pub async fn get_online_users(&self) -> Vec<UserPresence> {
        let users = self.online_users.read().await;
        users.values().cloned().collect()
    }

    /// Get filtered users
    pub async fn get_users_filtered(&self, filter: &UserFilter) -> Vec<UserPresence> {
        let users = self.online_users.read().await;
        let mut filtered: Vec<UserPresence> = users
            .values()
            .filter(|presence| filter.matches(presence))
            .cloned()
            .collect();

        // Sort by username
        filtered.sort_by(|a, b| a.username.cmp(&b.username));

        // Apply limit
        if let Some(limit) = filter.limit {
            filtered.truncate(limit);
        }

        filtered
    }

    /// Search users by name
    pub async fn search_users(&self, search_term: String) -> Vec<UserPresence> {
        let filter = UserFilter::with_search(search_term);
        self.get_users_filtered(&filter).await
    }

    /// Get available users (online/idle)
    pub async fn get_available_users(&self) -> Vec<UserPresence> {
        let filter = UserFilter::available_only();
        self.get_users_filtered(&filter).await
    }

    /// Update user typing status
    pub async fn set_user_typing(&self, user_id: UserId, typing_to: Option<UserId>) {
        let mut users = self.online_users.write().await;
        if let Some(presence) = users.get_mut(&user_id) {
            presence.set_typing(typing_to);
        }
    }

    /// Get users typing to specific recipient
    pub async fn get_users_typing_to(&self, recipient: UserId) -> Vec<UserPresence> {
        let users = self.online_users.read().await;
        users
            .values()
            .filter(|presence| presence.is_typing_to(recipient))
            .cloned()
            .collect()
    }

    /// Clean up stale presence data
    pub async fn cleanup_stale_presence(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut users = self.online_users.write().await;
        users.retain(|_, presence| now.saturating_sub(presence.last_seen) < self.presence_timeout);
    }

    /// Get user count
    pub async fn user_count(&self) -> usize {
        let users = self.online_users.read().await;
        users.len()
    }

    /// Check if user is online
    pub async fn is_user_online(&self, user_id: UserId) -> bool {
        let users = self.online_users.read().await;
        users.contains_key(&user_id)
    }

    /// Cache user profile
    pub async fn cache_user_profile(&self, profile: UserProfile) {
        let mut cache = self.user_cache.write().await;
        cache.insert(profile.user_id, profile);
    }

    /// Get cached user profile
    pub async fn get_user_profile(&self, user_id: UserId) -> Option<UserProfile> {
        let cache = self.user_cache.read().await;
        cache.get(&user_id).cloned()
    }

    /// Get user display name (from cache or presence)
    pub async fn get_user_display_name(&self, user_id: UserId) -> Option<String> {
        // Try profile cache first
        if let Some(profile) = self.get_user_profile(user_id).await {
            return Some(profile.display_name().to_string());
        }

        // Fall back to presence data
        if let Some(presence) = self.get_user_presence(user_id).await {
            return Some(presence.display_name().to_string());
        }

        None
    }

    /// Bulk update user list (from server response)
    pub async fn update_user_list(&self, users: Vec<UserPresence>) {
        let mut online_users = self.online_users.write().await;
        online_users.clear();

        for user in users {
            online_users.insert(user.user_id, user);
        }
    }

    /// Get user statistics
    pub async fn get_user_stats(&self) -> UserStats {
        let users = self.online_users.read().await;
        let total_users = users.len();
        let online_users = users.values().filter(|u| u.is_online()).count();
        let available_users = users.values().filter(|u| u.is_available()).count();
        let typing_users = users.values().filter(|u| u.is_typing_to.is_some()).count();

        UserStats {
            total_users,
            online_users,
            available_users,
            typing_users,
        }
    }
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}

/// User statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    pub total_users: usize,
    pub online_users: usize,
    pub available_users: usize,
    pub typing_users: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_presence_creation() {
        let user_id = uuid::Uuid::new_v4();
        let username = "testuser".to_string();

        let presence = UserPresence::new(user_id, username.clone());

        assert_eq!(presence.user_id, user_id);
        assert_eq!(presence.username, username);
        assert_eq!(presence.status, UserStatus::Online);
        assert!(presence.is_available());
    }

    #[tokio::test]
    async fn test_user_manager_operations() {
        let manager = UserManager::new();
        let user_id = uuid::Uuid::new_v4();
        let presence = UserPresence::new(user_id, "testuser".to_string());

        // Add user
        manager.update_user_presence(presence.clone()).await;
        assert!(manager.is_user_online(user_id).await);
        assert_eq!(manager.user_count().await, 1);

        // Get user
        let retrieved = manager.get_user_presence(user_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().username, "testuser");

        // Remove user
        manager.remove_user(user_id).await;
        assert!(!manager.is_user_online(user_id).await);
        assert_eq!(manager.user_count().await, 0);
    }

    #[tokio::test]
    async fn test_user_filtering() {
        let manager = UserManager::new();

        // Add test users
        let user1 = UserPresence::new(uuid::Uuid::new_v4(), "alice".to_string());
        let mut user2 = UserPresence::new(uuid::Uuid::new_v4(), "bob".to_string());
        user2.set_status(UserStatus::Away);

        manager.update_user_presence(user1).await;
        manager.update_user_presence(user2).await;

        // Test online only filter
        let online_users = manager.get_users_filtered(&UserFilter::online_only()).await;
        assert_eq!(online_users.len(), 1);
        assert_eq!(online_users[0].username, "alice");

        // Test search filter
        let search_results = manager.search_users("ali".to_string()).await;
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].username, "alice");
    }

    #[tokio::test]
    async fn test_typing_indicators() {
        let manager = UserManager::new();
        let user1_id = uuid::Uuid::new_v4();
        let user2_id = uuid::Uuid::new_v4();

        let presence = UserPresence::new(user1_id, "testuser".to_string());
        manager.update_user_presence(presence).await;

        // Set typing status
        manager.set_user_typing(user1_id, Some(user2_id)).await;

        let typing_users = manager.get_users_typing_to(user2_id).await;
        assert_eq!(typing_users.len(), 1);
        assert_eq!(typing_users[0].user_id, user1_id);

        // Clear typing status
        manager.set_user_typing(user1_id, None).await;

        let typing_users = manager.get_users_typing_to(user2_id).await;
        assert_eq!(typing_users.len(), 0);
    }

    #[test]
    fn test_user_filter_matching() {
        let presence = UserPresence::new(uuid::Uuid::new_v4(), "testuser".to_string());

        // Test basic filter
        let filter = UserFilter::new();
        assert!(filter.matches(&presence));

        // Test search filter
        let search_filter = UserFilter::with_search("test".to_string());
        assert!(search_filter.matches(&presence));

        let no_match_filter = UserFilter::with_search("nomatch".to_string());
        assert!(!no_match_filter.matches(&presence));

        // Test exclusion filter
        let exclude_filter = UserFilter::excluding(vec![presence.user_id]);
        assert!(!exclude_filter.matches(&presence));
    }
}
