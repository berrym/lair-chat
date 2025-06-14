//! User management for lair-chat server
//!
//! This module contains user session management, presence tracking, and user-related
//! functionality for the server-side of the lair-chat application.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Unique identifier for users
pub type UserId = Uuid;

/// User presence status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PresenceStatus {
    /// User is online and active
    Online,
    /// User is online but idle
    Away,
    /// User is busy/do not disturb
    Busy,
    /// User is offline
    Offline,
}

/// Connected user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedUser {
    /// User ID
    pub id: UserId,
    /// Username
    pub username: String,
    /// Socket address of the connection
    pub address: SocketAddr,
    /// When the user connected
    pub connected_at: u64,
    /// Current room the user is in
    pub current_room: String,
    /// User's presence status
    pub presence: PresenceStatus,
    /// Last activity timestamp
    pub last_activity: u64,
    /// Optional display name
    pub display_name: Option<String>,
}

impl ConnectedUser {
    /// Create a new connected user
    pub fn new(username: String, address: SocketAddr) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: Uuid::new_v4(),
            username,
            address,
            connected_at: now,
            current_room: "lobby".to_string(),
            presence: PresenceStatus::Online,
            last_activity: now,
            display_name: None,
        }
    }

    /// Update the user's last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Set the user's presence status
    pub fn set_presence(&mut self, status: PresenceStatus) {
        self.presence = status;
        self.update_activity();
    }

    /// Move user to a different room
    pub fn move_to_room(&mut self, room_name: String) {
        self.current_room = room_name;
        self.update_activity();
    }

    /// Check if the user has been idle for the given duration (in seconds)
    pub fn is_idle(&self, idle_threshold_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.last_activity > idle_threshold_seconds
    }

    /// Get connection duration in seconds
    pub fn connection_duration(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.connected_at
    }
}

/// User manager for tracking connected users
pub struct UserManager {
    /// Users indexed by username
    users_by_name: HashMap<String, ConnectedUser>,
    /// Users indexed by socket address
    users_by_addr: HashMap<SocketAddr, String>,
    /// Users indexed by room
    users_by_room: HashMap<String, Vec<String>>,
    /// Idle threshold in seconds
    idle_threshold: u64,
}

impl UserManager {
    /// Create a new user manager
    pub fn new() -> Self {
        Self {
            users_by_name: HashMap::new(),
            users_by_addr: HashMap::new(),
            users_by_room: HashMap::new(),
            idle_threshold: 300, // 5 minutes default
        }
    }

    /// Set the idle threshold
    pub fn set_idle_threshold(&mut self, seconds: u64) {
        self.idle_threshold = seconds;
    }

    /// Add a connected user
    pub fn add_user(&mut self, username: String, address: SocketAddr) -> Result<(), String> {
        if self.users_by_name.contains_key(&username) {
            return Err("User already connected".to_string());
        }

        let user = ConnectedUser::new(username.clone(), address);
        let room = user.current_room.clone();

        // Add to indexes
        self.users_by_addr.insert(address, username.clone());
        self.users_by_name.insert(username.clone(), user);

        // Add to room index
        self.users_by_room
            .entry(room)
            .or_insert_with(Vec::new)
            .push(username);

        Ok(())
    }

    /// Remove a user by username
    pub fn remove_user(&mut self, username: &str) -> Option<ConnectedUser> {
        if let Some(user) = self.users_by_name.remove(username) {
            // Remove from address index
            self.users_by_addr.remove(&user.address);

            // Remove from room index
            if let Some(room_users) = self.users_by_room.get_mut(&user.current_room) {
                room_users.retain(|u| u != username);
                if room_users.is_empty() {
                    self.users_by_room.remove(&user.current_room);
                }
            }

            Some(user)
        } else {
            None
        }
    }

    /// Remove a user by socket address
    pub fn remove_user_by_addr(&mut self, address: &SocketAddr) -> Option<ConnectedUser> {
        if let Some(username) = self.users_by_addr.get(address) {
            let username = username.clone();
            self.remove_user(&username)
        } else {
            None
        }
    }

    /// Get a user by username
    pub fn get_user(&self, username: &str) -> Option<&ConnectedUser> {
        self.users_by_name.get(username)
    }

    /// Get a mutable reference to a user by username
    pub fn get_user_mut(&mut self, username: &str) -> Option<&mut ConnectedUser> {
        self.users_by_name.get_mut(username)
    }

    /// Get a user by socket address
    pub fn get_user_by_addr(&self, address: &SocketAddr) -> Option<&ConnectedUser> {
        if let Some(username) = self.users_by_addr.get(address) {
            self.users_by_name.get(username)
        } else {
            None
        }
    }

    /// Get all connected users
    pub fn get_all_users(&self) -> Vec<&ConnectedUser> {
        self.users_by_name.values().collect()
    }

    /// Get users in a specific room
    pub fn get_users_in_room(&self, room: &str) -> Vec<&ConnectedUser> {
        if let Some(usernames) = self.users_by_room.get(room) {
            usernames
                .iter()
                .filter_map(|username| self.users_by_name.get(username))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Move a user to a different room
    pub fn move_user_to_room(&mut self, username: &str, new_room: &str) -> Result<(), String> {
        if let Some(user) = self.users_by_name.get_mut(username) {
            let old_room = user.current_room.clone();

            // Remove from old room index
            if let Some(old_room_users) = self.users_by_room.get_mut(&old_room) {
                old_room_users.retain(|u| u != username);
                if old_room_users.is_empty() {
                    self.users_by_room.remove(&old_room);
                }
            }

            // Update user's room
            user.move_to_room(new_room.to_string());

            // Add to new room index
            self.users_by_room
                .entry(new_room.to_string())
                .or_insert_with(Vec::new)
                .push(username.to_string());

            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }

    /// Update a user's presence status
    pub fn update_user_presence(
        &mut self,
        username: &str,
        status: PresenceStatus,
    ) -> Result<(), String> {
        if let Some(user) = self.users_by_name.get_mut(username) {
            user.set_presence(status);
            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }

    /// Update a user's activity timestamp
    pub fn update_user_activity(&mut self, username: &str) -> Result<(), String> {
        if let Some(user) = self.users_by_name.get_mut(username) {
            user.update_activity();
            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }

    /// Get idle users
    pub fn get_idle_users(&self) -> Vec<&ConnectedUser> {
        self.users_by_name
            .values()
            .filter(|user| user.is_idle(self.idle_threshold))
            .collect()
    }

    /// Get user count
    pub fn user_count(&self) -> usize {
        self.users_by_name.len()
    }

    /// Get user count by room
    pub fn room_user_count(&self, room: &str) -> usize {
        self.users_by_room
            .get(room)
            .map(|users| users.len())
            .unwrap_or(0)
    }

    /// Get all room names with users
    pub fn get_active_rooms(&self) -> Vec<String> {
        self.users_by_room.keys().cloned().collect()
    }
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connected_user_creation() {
        let addr = "127.0.0.1:8080".parse().unwrap();
        let user = ConnectedUser::new("alice".to_string(), addr);

        assert_eq!(user.username, "alice");
        assert_eq!(user.address, addr);
        assert_eq!(user.current_room, "lobby");
        assert_eq!(user.presence, PresenceStatus::Online);
    }

    #[test]
    fn test_user_activity_update() {
        let addr = "127.0.0.1:8080".parse().unwrap();
        let mut user = ConnectedUser::new("alice".to_string(), addr);

        let initial_activity = user.last_activity;
        std::thread::sleep(std::time::Duration::from_millis(10));

        user.update_activity();
        assert!(user.last_activity > initial_activity);
    }

    #[test]
    fn test_user_manager_basic_operations() {
        let mut manager = UserManager::new();
        let addr = "127.0.0.1:8080".parse().unwrap();

        // Add user
        assert!(manager.add_user("alice".to_string(), addr).is_ok());
        assert_eq!(manager.user_count(), 1);
        assert!(manager.get_user("alice").is_some());

        // Can't add same user twice
        assert!(manager.add_user("alice".to_string(), addr).is_err());

        // Remove user
        assert!(manager.remove_user("alice").is_some());
        assert_eq!(manager.user_count(), 0);
        assert!(manager.get_user("alice").is_none());
    }

    #[test]
    fn test_room_management() {
        let mut manager = UserManager::new();
        let addr1 = "127.0.0.1:8080".parse().unwrap();
        let addr2 = "127.0.0.1:8081".parse().unwrap();

        // Add users
        manager.add_user("alice".to_string(), addr1).unwrap();
        manager.add_user("bob".to_string(), addr2).unwrap();

        // Both should be in lobby initially
        assert_eq!(manager.room_user_count("lobby"), 2);

        // Move alice to general room
        assert!(manager.move_user_to_room("alice", "general").is_ok());
        assert_eq!(manager.room_user_count("lobby"), 1);
        assert_eq!(manager.room_user_count("general"), 1);

        // Verify alice is in general room
        let alice = manager.get_user("alice").unwrap();
        assert_eq!(alice.current_room, "general");
    }

    #[test]
    fn test_presence_management() {
        let mut manager = UserManager::new();
        let addr = "127.0.0.1:8080".parse().unwrap();

        manager.add_user("alice".to_string(), addr).unwrap();

        // Update presence
        assert!(manager
            .update_user_presence("alice", PresenceStatus::Away)
            .is_ok());

        let alice = manager.get_user("alice").unwrap();
        assert_eq!(alice.presence, PresenceStatus::Away);
    }
}
