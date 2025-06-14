//! Room management for lair-chat server
//!
//! This module contains room creation, management, and user assignment
//! functionality for the server-side of the lair-chat application.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Unique identifier for rooms
pub type RoomId = Uuid;

/// Room configuration and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    /// Unique room identifier
    pub id: RoomId,
    /// Human-readable room name
    pub name: String,
    /// Optional room description
    pub description: Option<String>,
    /// List of user IDs in this room
    pub users: Vec<String>,
    /// Room creation timestamp
    pub created_at: u64,
    /// Whether this is the default lobby room
    pub is_lobby: bool,
    /// Maximum number of users allowed
    pub max_users: Option<usize>,
    /// Whether the room is private (invite-only)
    pub is_private: bool,
    /// Room owner/creator
    pub owner: Option<String>,
}

impl Room {
    /// Create a new room
    pub fn new(name: String, is_lobby: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            users: Vec::new(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            is_lobby,
            max_users: None,
            is_private: false,
            owner: None,
        }
    }

    /// Add a user to the room
    pub fn add_user(&mut self, username: String) -> bool {
        if !self.users.contains(&username) {
            if let Some(max) = self.max_users {
                if self.users.len() >= max {
                    return false;
                }
            }
            self.users.push(username);
            true
        } else {
            false
        }
    }

    /// Remove a user from the room
    pub fn remove_user(&mut self, username: &str) -> bool {
        if let Some(pos) = self.users.iter().position(|u| u == username) {
            self.users.remove(pos);
            true
        } else {
            false
        }
    }

    /// Check if a user is in the room
    pub fn has_user(&self, username: &str) -> bool {
        self.users.contains(&username.to_string())
    }

    /// Get the number of users in the room
    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    /// Check if the room is full
    pub fn is_full(&self) -> bool {
        if let Some(max) = self.max_users {
            self.users.len() >= max
        } else {
            false
        }
    }
}

/// Room manager for handling multiple rooms
pub struct RoomManager {
    rooms: HashMap<String, Room>,
    room_by_id: HashMap<RoomId, String>,
}

impl RoomManager {
    /// Create a new room manager
    pub fn new() -> Self {
        let mut manager = Self {
            rooms: HashMap::new(),
            room_by_id: HashMap::new(),
        };

        // Create default lobby room
        let lobby = Room::new("lobby".to_string(), true);
        manager.add_room(lobby);

        manager
    }

    /// Add a room to the manager
    pub fn add_room(&mut self, room: Room) {
        let name = room.name.clone();
        let id = room.id;
        self.room_by_id.insert(id, name.clone());
        self.rooms.insert(name, room);
    }

    /// Create a new room
    pub fn create_room(&mut self, name: String, owner: Option<String>) -> Result<&Room, String> {
        if self.rooms.contains_key(&name) {
            return Err("Room already exists".to_string());
        }

        let mut room = Room::new(name.clone(), false);
        room.owner = owner;
        self.add_room(room);

        Ok(self.rooms.get(&name).unwrap())
    }

    /// Get a room by name
    pub fn get_room(&self, name: &str) -> Option<&Room> {
        self.rooms.get(name)
    }

    /// Get a mutable reference to a room by name
    pub fn get_room_mut(&mut self, name: &str) -> Option<&mut Room> {
        self.rooms.get_mut(name)
    }

    /// Remove a room
    pub fn remove_room(&mut self, name: &str) -> Option<Room> {
        if let Some(room) = self.rooms.remove(name) {
            self.room_by_id.remove(&room.id);
            Some(room)
        } else {
            None
        }
    }

    /// Get all room names
    pub fn get_room_names(&self) -> Vec<String> {
        self.rooms.keys().cloned().collect()
    }

    /// Get all rooms
    pub fn get_all_rooms(&self) -> Vec<&Room> {
        self.rooms.values().collect()
    }

    /// Move a user from one room to another
    pub fn move_user(
        &mut self,
        username: &str,
        from_room: &str,
        to_room: &str,
    ) -> Result<(), String> {
        // Remove from source room
        if let Some(from) = self.rooms.get_mut(from_room) {
            from.remove_user(username);
        }

        // Add to destination room
        if let Some(to) = self.rooms.get_mut(to_room) {
            if to.add_user(username.to_string()) {
                Ok(())
            } else {
                Err("Failed to add user to destination room".to_string())
            }
        } else {
            Err("Destination room not found".to_string())
        }
    }

    /// Get the room a user is currently in
    pub fn find_user_room(&self, username: &str) -> Option<String> {
        for (room_name, room) in &self.rooms {
            if room.has_user(username) {
                return Some(room_name.clone());
            }
        }
        None
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_creation() {
        let room = Room::new("test_room".to_string(), false);
        assert_eq!(room.name, "test_room");
        assert!(!room.is_lobby);
        assert_eq!(room.users.len(), 0);
    }

    #[test]
    fn test_room_user_management() {
        let mut room = Room::new("test_room".to_string(), false);

        // Add user
        assert!(room.add_user("alice".to_string()));
        assert_eq!(room.user_count(), 1);
        assert!(room.has_user("alice"));

        // Can't add same user twice
        assert!(!room.add_user("alice".to_string()));
        assert_eq!(room.user_count(), 1);

        // Remove user
        assert!(room.remove_user("alice"));
        assert_eq!(room.user_count(), 0);
        assert!(!room.has_user("alice"));

        // Can't remove non-existent user
        assert!(!room.remove_user("bob"));
    }

    #[test]
    fn test_room_capacity() {
        let mut room = Room::new("test_room".to_string(), false);
        room.max_users = Some(2);

        assert!(room.add_user("alice".to_string()));
        assert!(room.add_user("bob".to_string()));
        assert!(room.is_full());

        // Can't add more users when full
        assert!(!room.add_user("charlie".to_string()));
    }

    #[test]
    fn test_room_manager() {
        let mut manager = RoomManager::new();

        // Should have default lobby
        assert!(manager.get_room("lobby").is_some());

        // Create new room
        assert!(manager
            .create_room("general".to_string(), Some("alice".to_string()))
            .is_ok());
        assert!(manager.get_room("general").is_some());

        // Can't create duplicate room
        assert!(manager.create_room("general".to_string(), None).is_err());

        // Move user between rooms
        {
            let lobby = manager.get_room_mut("lobby").unwrap();
            lobby.add_user("alice".to_string());
        }

        assert!(manager.move_user("alice", "lobby", "general").is_ok());
        assert!(!manager.get_room("lobby").unwrap().has_user("alice"));
        assert!(manager.get_room("general").unwrap().has_user("alice"));
    }
}
