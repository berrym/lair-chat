//! Room management structures for Lair-Chat
//! Handles room creation, settings, and lifecycle management.

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

use super::{RoomId, UserId, MessageId, RoomEvent, ChatError, ChatResult, TypingIndicators};
use super::users::{RoomUser, UserRole};
use super::messages::ChatMessage;

/// Room type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoomType {
    /// Public room - anyone can join
    Public,
    /// Private room - invitation only
    Private,
    /// Direct message between two users
    DirectMessage,
    /// Group direct message
    GroupMessage,
}

/// Room settings and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSettings {
    /// Room name
    pub name: String,
    /// Room description
    pub description: Option<String>,
    /// Room type
    pub room_type: RoomType,
    /// Maximum number of users (None = unlimited)
    pub max_users: Option<usize>,
    /// Whether messages are persistent
    pub persistent_messages: bool,
    /// Message history limit (None = unlimited)
    pub message_limit: Option<usize>,
    /// Whether typing indicators are enabled
    pub typing_indicators: bool,
    /// Whether read receipts are enabled
    pub read_receipts: bool,
    /// Whether file sharing is allowed
    pub file_sharing: bool,
    /// Whether message editing is allowed
    pub message_editing: bool,
    /// Whether message deletion is allowed
    pub message_deletion: bool,
    /// Room password (for protected rooms)
    pub password: Option<String>,
}

impl RoomSettings {
    /// Create default public room settings
    pub fn public(name: String) -> Self {
        Self {
            name,
            description: None,
            room_type: RoomType::Public,
            max_users: None,
            persistent_messages: true,
            message_limit: Some(1000),
            typing_indicators: true,
            read_receipts: true,
            file_sharing: true,
            message_editing: true,
            message_deletion: true,
            password: None,
        }
    }

    /// Create default private room settings
    pub fn private(name: String) -> Self {
        Self {
            name,
            description: None,
            room_type: RoomType::Private,
            max_users: Some(50),
            persistent_messages: true,
            message_limit: Some(1000),
            typing_indicators: true,
            read_receipts: true,
            file_sharing: true,
            message_editing: true,
            message_deletion: true,
            password: None,
        }
    }

    /// Create direct message settings
    pub fn direct_message(user1: UserId, user2: UserId) -> Self {
        Self {
            name: format!("DM-{}-{}", user1, user2),
            description: None,
            room_type: RoomType::DirectMessage,
            max_users: Some(2),
            persistent_messages: true,
            message_limit: Some(500),
            typing_indicators: true,
            read_receipts: true,
            file_sharing: true,
            message_editing: true,
            message_deletion: true,
            password: None,
        }
    }

    /// Create group message settings
    pub fn group_message(name: String, max_users: usize) -> Self {
        Self {
            name,
            description: None,
            room_type: RoomType::GroupMessage,
            max_users: Some(max_users),
            persistent_messages: true,
            message_limit: Some(1000),
            typing_indicators: true,
            read_receipts: true,
            file_sharing: true,
            message_editing: true,
            message_deletion: true,
            password: None,
        }
    }

    /// Validate room settings
    pub fn validate(&self) -> ChatResult<()> {
        if self.name.trim().is_empty() {
            return Err(ChatError::InvalidRoomName("Room name cannot be empty".to_string()));
        }

        if self.name.len() > 100 {
            return Err(ChatError::InvalidRoomName("Room name too long".to_string()));
        }

        if let Some(max) = self.max_users {
            if max == 0 {
                return Err(ChatError::InvalidRoomName("Max users must be greater than 0".to_string()));
            }
        }

        Ok(())
    }
}

/// Chat room structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    /// Unique room identifier
    pub id: RoomId,
    /// Room settings
    pub settings: RoomSettings,
    /// Users currently in the room
    pub users: HashMap<UserId, RoomUser>,
    /// Recent messages
    pub messages: Vec<ChatMessage>,
    /// Room creation timestamp
    pub created_at: u64,
    /// Last activity timestamp
    pub last_activity: u64,
    /// Room creator
    pub created_by: UserId,
    /// Typing indicators
    #[serde(skip)]
    pub typing_indicators: TypingIndicators,
    /// Room events history
    pub events: Vec<RoomEvent>,
}

impl Room {
    /// Create a new room
    pub fn new(id: RoomId, settings: RoomSettings, created_by: UserId) -> ChatResult<Self> {
        settings.validate()?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(Self {
            id,
            settings,
            users: HashMap::new(),
            messages: Vec::new(),
            created_at: now,
            last_activity: now,
            created_by,
            typing_indicators: TypingIndicators::default(),
            events: Vec::new(),
        })
    }

    /// Add a user to the room
    pub fn add_user(&mut self, user: RoomUser) -> ChatResult<()> {
        // Check if room is full
        if let Some(max) = self.settings.max_users {
            if self.users.len() >= max {
                return Err(ChatError::RoomFull);
            }
        }

        // Check if user is already in room
        if self.users.contains_key(&user.user_id) {
            return Err(ChatError::UserAlreadyInRoom);
        }

        // Add user
        let user_id = user.user_id;
        let username = user.username.clone();
        self.users.insert(user_id, user);

        // Record event
        let event = RoomEvent::user_joined(user_id, username);
        self.events.push(event);
        self.update_activity();

        Ok(())
    }

    /// Remove a user from the room
    pub fn remove_user(&mut self, user_id: &UserId) -> ChatResult<RoomUser> {
        let user = self.users.remove(user_id).ok_or(ChatError::UserNotInRoom)?;

        // Stop typing indicator
        self.typing_indicators.stop_typing(*user_id);

        // Record event
        let event = RoomEvent::user_left(*user_id, user.username.clone());
        self.events.push(event);
        self.update_activity();

        Ok(user)
    }

    /// Get a user in the room
    pub fn get_user(&self, user_id: &UserId) -> Option<&RoomUser> {
        self.users.get(user_id)
    }

    /// Get a mutable reference to a user in the room
    pub fn get_user_mut(&mut self, user_id: &UserId) -> Option<&mut RoomUser> {
        self.users.get_mut(user_id)
    }

    /// Check if user is in room
    pub fn has_user(&self, user_id: &UserId) -> bool {
        self.users.contains_key(user_id)
    }

    /// Get all users in the room
    pub fn get_users(&self) -> Vec<&RoomUser> {
        self.users.values().collect()
    }

    /// Get user count
    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    /// Add a message to the room
    pub fn add_message(&mut self, message: ChatMessage) -> ChatResult<()> {
        // Check if user is in room
        if !self.has_user(&message.sender_id) {
            return Err(ChatError::UserNotInRoom);
        }

        // Add message
        self.messages.push(message.clone());

        // Enforce message limit
        if let Some(limit) = self.settings.message_limit {
            while self.messages.len() > limit {
                self.messages.remove(0);
            }
        }

        // Record event
        if let Some(user) = self.get_user(&message.sender_id) {
            let event = RoomEvent::message_sent(message.id, message.sender_id, user.username.clone());
            self.events.push(event);
        }

        self.update_activity();
        Ok(())
    }

    /// Get recent messages
    pub fn get_messages(&self, limit: Option<usize>) -> &[ChatMessage] {
        if let Some(limit) = limit {
            let start = self.messages.len().saturating_sub(limit);
            &self.messages[start..]
        } else {
            &self.messages
        }
    }

    /// Find a message by ID
    pub fn get_message(&self, message_id: &MessageId) -> Option<&ChatMessage> {
        self.messages.iter().find(|m| m.id == *message_id)
    }

    /// Find a mutable message by ID
    pub fn get_message_mut(&mut self, message_id: &MessageId) -> Option<&mut ChatMessage> {
        self.messages.iter_mut().find(|m| m.id == *message_id)
    }

    /// Start typing indicator for user
    pub fn start_typing(&mut self, user_id: UserId) -> ChatResult<bool> {
        if !self.has_user(&user_id) {
            return Err(ChatError::UserNotInRoom);
        }

        if !self.settings.typing_indicators {
            return Ok(false);
        }

        let started = self.typing_indicators.start_typing(user_id);
        if started {
            if let Some(user) = self.get_user(&user_id) {
                let event = RoomEvent::typing_started(user_id, user.username.clone());
                self.events.push(event);
            }
        }

        Ok(started)
    }

    /// Stop typing indicator for user
    pub fn stop_typing(&mut self, user_id: UserId) -> ChatResult<bool> {
        if !self.has_user(&user_id) {
            return Err(ChatError::UserNotInRoom);
        }

        let stopped = self.typing_indicators.stop_typing(user_id);
        if stopped {
            if let Some(user) = self.get_user(&user_id) {
                let event = RoomEvent::typing_stopped(user_id, user.username.clone());
                self.events.push(event);
            }
        }

        Ok(stopped)
    }

    /// Get currently typing users
    pub fn get_typing_users(&mut self) -> Vec<UserId> {
        if !self.settings.typing_indicators {
            return Vec::new();
        }
        self.typing_indicators.get_typing_users()
    }

    /// Check if user can perform action
    pub fn can_user_perform_action(&self, user_id: &UserId, action: &str) -> bool {
        let user = match self.get_user(user_id) {
            Some(user) => user,
            None => return false,
        };

        match action {
            "send_message" => true, // All users can send messages
            "edit_message" => self.settings.message_editing,
            "delete_message" => {
                self.settings.message_deletion && 
                (user.role == UserRole::Admin || user.role == UserRole::Moderator)
            }
            "kick_user" => user.role == UserRole::Admin || user.role == UserRole::Moderator,
            "change_settings" => user.role == UserRole::Admin,
            "delete_room" => user.role == UserRole::Admin,
            _ => false,
        }
    }

    /// Update last activity timestamp
    fn update_activity(&mut self) {
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Check if room is empty
    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }

    /// Check if room is full
    pub fn is_full(&self) -> bool {
        if let Some(max) = self.settings.max_users {
            self.users.len() >= max
        } else {
            false
        }
    }

    /// Get room statistics
    pub fn get_stats(&mut self) -> RoomStats {
        RoomStats {
            user_count: self.user_count(),
            message_count: self.messages.len(),
            created_at: self.created_at,
            last_activity: self.last_activity,
            typing_users: self.typing_indicators.get_typing_users().len(),
        }
    }
}

/// Room statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomStats {
    pub user_count: usize,
    pub message_count: usize,
    pub created_at: u64,
    pub last_activity: u64,
    pub typing_users: usize,
}

/// Room manager for handling multiple rooms
#[derive(Debug, Clone)]
pub struct RoomManager {
    /// All rooms indexed by ID
    rooms: HashMap<RoomId, Room>,
    /// User to room mappings
    user_rooms: HashMap<UserId, HashSet<RoomId>>,
}

impl RoomManager {
    /// Create a new room manager
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            user_rooms: HashMap::new(),
        }
    }

    /// Create a new room
    pub fn create_room(&mut self, settings: RoomSettings, created_by: UserId) -> ChatResult<RoomId> {
        let room_id = uuid::Uuid::new_v4();
        let room = Room::new(room_id, settings, created_by)?;
        
        self.rooms.insert(room_id, room);
        Ok(room_id)
    }

    /// Get a room by ID
    pub fn get_room(&self, room_id: &RoomId) -> Option<&Room> {
        self.rooms.get(room_id)
    }

    /// Get a mutable room by ID
    pub fn get_room_mut(&mut self, room_id: &RoomId) -> Option<&mut Room> {
        self.rooms.get_mut(room_id)
    }

    /// Delete a room
    pub fn delete_room(&mut self, room_id: &RoomId) -> ChatResult<Room> {
        let room = self.rooms.remove(room_id).ok_or(ChatError::RoomNotFound(*room_id))?;

        // Remove room from all user mappings
        for user_id in room.users.keys() {
            if let Some(user_rooms) = self.user_rooms.get_mut(user_id) {
                user_rooms.remove(room_id);
                if user_rooms.is_empty() {
                    self.user_rooms.remove(user_id);
                }
            }
        }

        Ok(room)
    }

    /// Join a user to a room
    pub fn join_room(&mut self, room_id: &RoomId, user: RoomUser) -> ChatResult<()> {
        let room = self.rooms.get_mut(room_id).ok_or(ChatError::RoomNotFound(*room_id))?;
        
        // Check room type permissions
        match room.settings.room_type {
            RoomType::Private => {
                // Would need invitation system here
                return Err(ChatError::RoomPrivate);
            }
            RoomType::DirectMessage | RoomType::GroupMessage => {
                // These are typically invitation-only
                return Err(ChatError::PermissionDenied("Cannot join this type of room".to_string()));
            }
            RoomType::Public => {
                // Anyone can join public rooms
            }
        }

        let user_id = user.user_id;
        room.add_user(user)?;

        // Update user-room mapping
        self.user_rooms.entry(user_id).or_default().insert(*room_id);

        Ok(())
    }

    /// Remove a user from a room
    pub fn leave_room(&mut self, room_id: &RoomId, user_id: &UserId) -> ChatResult<()> {
        let room = self.rooms.get_mut(room_id).ok_or(ChatError::RoomNotFound(*room_id))?;
        room.remove_user(user_id)?;

        // Update user-room mapping
        if let Some(user_rooms) = self.user_rooms.get_mut(user_id) {
            user_rooms.remove(room_id);
            if user_rooms.is_empty() {
                self.user_rooms.remove(user_id);
            }
        }

        // Delete room if empty and it's a DM or group message
        if room.is_empty() && matches!(room.settings.room_type, RoomType::DirectMessage | RoomType::GroupMessage) {
            self.rooms.remove(room_id);
        }

        Ok(())
    }

    /// Get all rooms a user is in
    pub fn get_user_rooms(&self, user_id: &UserId) -> Vec<&Room> {
        if let Some(room_ids) = self.user_rooms.get(user_id) {
            room_ids.iter()
                .filter_map(|id| self.rooms.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all public rooms
    pub fn get_public_rooms(&self) -> Vec<&Room> {
        self.rooms.values()
            .filter(|room| room.settings.room_type == RoomType::Public)
            .collect()
    }

    /// Get all rooms
    pub fn get_all_rooms(&self) -> Vec<&Room> {
        self.rooms.values().collect()
    }

    /// Find rooms by name pattern
    pub fn find_rooms(&self, pattern: &str) -> Vec<&Room> {
        let pattern = pattern.to_lowercase();
        self.rooms.values()
            .filter(|room| {
                room.settings.name.to_lowercase().contains(&pattern) ||
                room.settings.description.as_ref()
                    .map(|d| d.to_lowercase().contains(&pattern))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Get room count
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    /// Get total user count across all rooms
    pub fn total_user_count(&self) -> usize {
        self.user_rooms.len()
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
    use super::super::users::{RoomUser, UserRole, UserStatus};

    #[test]
    fn test_room_creation() {
        let room_id = uuid::Uuid::new_v4();
        let user_id = uuid::Uuid::new_v4();
        let settings = RoomSettings::public("Test Room".to_string());
        
        let room = Room::new(room_id, settings, user_id).unwrap();
        
        assert_eq!(room.id, room_id);
        assert_eq!(room.settings.name, "Test Room");
        assert_eq!(room.created_by, user_id);
        assert!(room.users.is_empty());
        assert!(room.messages.is_empty());
    }

    #[test]
    fn test_room_user_management() {
        let room_id = uuid::Uuid::new_v4();
        let creator_id = uuid::Uuid::new_v4();
        let user_id = uuid::Uuid::new_v4();
        let settings = RoomSettings::public("Test Room".to_string());
        
        let mut room = Room::new(room_id, settings, creator_id).unwrap();
        
        let user = RoomUser::new(user_id, "testuser".to_string(), UserRole::User);
        room.add_user(user).unwrap();
        
        assert_eq!(room.user_count(), 1);
        assert!(room.has_user(&user_id));
        
        let removed_user = room.remove_user(&user_id).unwrap();
        assert_eq!(removed_user.user_id, user_id);
        assert_eq!(room.user_count(), 0);
    }

    #[test]
    fn test_room_manager() {
        let mut manager = RoomManager::new();
        let creator_id = uuid::Uuid::new_v4();
        let user_id = uuid::Uuid::new_v4();
        
        let settings = RoomSettings::public("Test Room".to_string());
        let room_id = manager.create_room(settings, creator_id).unwrap();
        
        assert_eq!(manager.room_count(), 1);
        assert!(manager.get_room(&room_id).is_some());
        
        let user = RoomUser::new(user_id, "testuser".to_string(), UserRole::User);
        manager.join_room(&room_id, user).unwrap();
        
        let user_rooms = manager.get_user_rooms(&user_id);
        assert_eq!(user_rooms.len(), 1);
        assert_eq!(user_rooms[0].id, room_id);
    }
}