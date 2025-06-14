//! Message handling for lair-chat server
//!
//! This module contains message types, validation, and processing logic
//! for chat messages in the lair-chat server.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Unique identifier for messages
pub type MessageId = Uuid;

/// Unique identifier for users
pub type UserId = Uuid;

/// Message types supported by the server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    /// Regular text message
    Text,
    /// System message (server announcements, user join/leave, etc.)
    System,
    /// Direct message between two users
    DirectMessage,
    /// File attachment
    File,
    /// Image attachment
    Image,
    /// Error message
    Error,
}

/// Message status for tracking delivery and read receipts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageStatus {
    /// Message is being sent
    Sending,
    /// Message has been sent successfully
    Sent,
    /// Message has been delivered to recipient
    Delivered,
    /// Message has been read by recipient
    Read,
    /// Message failed to send
    Failed,
}

/// A chat message with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique message identifier
    pub id: MessageId,
    /// ID of the user who sent the message
    pub sender_id: Option<UserId>,
    /// Username of the sender (for display purposes)
    pub sender_username: String,
    /// ID of the recipient (for direct messages)
    pub recipient_id: Option<UserId>,
    /// Content of the message
    pub content: String,
    /// Type of message
    pub message_type: MessageType,
    /// Current status of the message
    pub status: MessageStatus,
    /// Timestamp when message was created
    pub timestamp: u64,
    /// Room or channel the message belongs to
    pub room: String,
    /// Optional metadata (file info, etc.)
    pub metadata: HashMap<String, String>,
    /// Whether the message has been edited
    pub edited: bool,
    /// Timestamp of last edit (if any)
    pub edited_at: Option<u64>,
}

impl ChatMessage {
    /// Create a new text message
    pub fn new_text_message(
        sender_username: String,
        sender_id: Option<UserId>,
        content: String,
        room: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender_id,
            sender_username,
            recipient_id: None,
            content,
            message_type: MessageType::Text,
            status: MessageStatus::Sending,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            room,
            metadata: HashMap::new(),
            edited: false,
            edited_at: None,
        }
    }

    /// Create a new system message
    pub fn new_system_message(content: String, room: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender_id: None,
            sender_username: "System".to_string(),
            recipient_id: None,
            content,
            message_type: MessageType::System,
            status: MessageStatus::Sent,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            room,
            metadata: HashMap::new(),
            edited: false,
            edited_at: None,
        }
    }

    /// Create a new direct message
    pub fn new_direct_message(
        sender_username: String,
        sender_id: UserId,
        recipient_id: UserId,
        content: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender_id: Some(sender_id),
            sender_username,
            recipient_id: Some(recipient_id),
            content,
            message_type: MessageType::DirectMessage,
            status: MessageStatus::Sending,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            room: "direct".to_string(),
            metadata: HashMap::new(),
            edited: false,
            edited_at: None,
        }
    }

    /// Create an error message
    pub fn new_error_message(content: String, room: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender_id: None,
            sender_username: "System".to_string(),
            recipient_id: None,
            content,
            message_type: MessageType::Error,
            status: MessageStatus::Sent,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            room,
            metadata: HashMap::new(),
            edited: false,
            edited_at: None,
        }
    }

    /// Mark message as sent
    pub fn mark_sent(&mut self) {
        self.status = MessageStatus::Sent;
    }

    /// Mark message as delivered
    pub fn mark_delivered(&mut self) {
        self.status = MessageStatus::Delivered;
    }

    /// Mark message as read
    pub fn mark_read(&mut self) {
        self.status = MessageStatus::Read;
    }

    /// Mark message as failed
    pub fn mark_failed(&mut self) {
        self.status = MessageStatus::Failed;
    }

    /// Edit the message content
    pub fn edit_content(&mut self, new_content: String) {
        self.content = new_content;
        self.edited = true;
        self.edited_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
    }

    /// Add metadata to the message
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata value by key
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Check if this is a direct message
    pub fn is_direct_message(&self) -> bool {
        self.message_type == MessageType::DirectMessage && self.recipient_id.is_some()
    }

    /// Check if this is a system message
    pub fn is_system_message(&self) -> bool {
        self.message_type == MessageType::System
    }

    /// Validate message content
    pub fn validate(&self) -> Result<(), String> {
        if self.content.is_empty() {
            return Err("Message content cannot be empty".to_string());
        }

        if self.content.len() > 2000 {
            return Err("Message content too long (max 2000 characters)".to_string());
        }

        if self.sender_username.is_empty() && !self.is_system_message() {
            return Err("Sender username cannot be empty".to_string());
        }

        if self.room.is_empty() {
            return Err("Room cannot be empty".to_string());
        }

        if self.is_direct_message() && self.recipient_id.is_none() {
            return Err("Direct messages must have a recipient".to_string());
        }

        Ok(())
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Create from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Message history and storage management
pub struct MessageStore {
    /// In-memory message storage (in production, this would be a database)
    messages: HashMap<MessageId, ChatMessage>,
    /// Messages by room
    room_messages: HashMap<String, Vec<MessageId>>,
    /// Direct messages by user pair
    direct_messages: HashMap<(UserId, UserId), Vec<MessageId>>,
}

impl MessageStore {
    /// Create a new message store
    pub fn new() -> Self {
        Self {
            messages: HashMap::new(),
            room_messages: HashMap::new(),
            direct_messages: HashMap::new(),
        }
    }

    /// Store a message
    pub fn store_message(&mut self, message: ChatMessage) -> Result<MessageId, String> {
        message.validate()?;

        let message_id = message.id;
        let room = message.room.clone();

        // Store in main messages map
        self.messages.insert(message_id, message.clone());

        // Index by room
        self.room_messages
            .entry(room)
            .or_insert_with(Vec::new)
            .push(message_id);

        // Index direct messages
        if let (Some(sender_id), Some(recipient_id)) = (message.sender_id, message.recipient_id) {
            let key = if sender_id < recipient_id {
                (sender_id, recipient_id)
            } else {
                (recipient_id, sender_id)
            };

            self.direct_messages
                .entry(key)
                .or_insert_with(Vec::new)
                .push(message_id);
        }

        Ok(message_id)
    }

    /// Get a message by ID
    pub fn get_message(&self, id: &MessageId) -> Option<&ChatMessage> {
        self.messages.get(id)
    }

    /// Get messages for a room
    pub fn get_room_messages(&self, room: &str, limit: Option<usize>) -> Vec<&ChatMessage> {
        if let Some(message_ids) = self.room_messages.get(room) {
            let mut messages: Vec<&ChatMessage> = message_ids
                .iter()
                .rev() // Most recent first
                .filter_map(|id| self.messages.get(id))
                .collect();

            if let Some(limit) = limit {
                messages.truncate(limit);
            }

            messages
        } else {
            Vec::new()
        }
    }

    /// Get direct messages between two users
    pub fn get_direct_messages(
        &self,
        user1: UserId,
        user2: UserId,
        limit: Option<usize>,
    ) -> Vec<&ChatMessage> {
        let key = if user1 < user2 {
            (user1, user2)
        } else {
            (user2, user1)
        };

        if let Some(message_ids) = self.direct_messages.get(&key) {
            let mut messages: Vec<&ChatMessage> = message_ids
                .iter()
                .rev() // Most recent first
                .filter_map(|id| self.messages.get(id))
                .collect();

            if let Some(limit) = limit {
                messages.truncate(limit);
            }

            messages
        } else {
            Vec::new()
        }
    }

    /// Update message status
    pub fn update_message_status(&mut self, id: &MessageId, status: MessageStatus) -> bool {
        if let Some(message) = self.messages.get_mut(id) {
            message.status = status;
            true
        } else {
            false
        }
    }

    /// Edit a message
    pub fn edit_message(&mut self, id: &MessageId, new_content: String) -> Result<(), String> {
        if let Some(message) = self.messages.get_mut(id) {
            message.edit_content(new_content);
            Ok(())
        } else {
            Err("Message not found".to_string())
        }
    }

    /// Delete a message
    pub fn delete_message(&mut self, id: &MessageId) -> bool {
        if let Some(message) = self.messages.remove(id) {
            // Remove from room index
            if let Some(room_messages) = self.room_messages.get_mut(&message.room) {
                room_messages.retain(|msg_id| msg_id != id);
            }

            // Remove from direct messages index
            if let (Some(sender_id), Some(recipient_id)) = (message.sender_id, message.recipient_id)
            {
                let key = if sender_id < recipient_id {
                    (sender_id, recipient_id)
                } else {
                    (recipient_id, sender_id)
                };

                if let Some(dm_messages) = self.direct_messages.get_mut(&key) {
                    dm_messages.retain(|msg_id| msg_id != id);
                }
            }

            true
        } else {
            false
        }
    }

    /// Get total message count
    pub fn total_messages(&self) -> usize {
        self.messages.len()
    }

    /// Get message count for a room
    pub fn room_message_count(&self, room: &str) -> usize {
        self.room_messages
            .get(room)
            .map(|msgs| msgs.len())
            .unwrap_or(0)
    }
}

impl Default for MessageStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_text_message() {
        let message = ChatMessage::new_text_message(
            "alice".to_string(),
            Some(Uuid::new_v4()),
            "Hello, world!".to_string(),
            "general".to_string(),
        );

        assert_eq!(message.sender_username, "alice");
        assert_eq!(message.content, "Hello, world!");
        assert_eq!(message.room, "general");
        assert_eq!(message.message_type, MessageType::Text);
        assert_eq!(message.status, MessageStatus::Sending);
        assert!(!message.edited);
    }

    #[test]
    fn test_create_system_message() {
        let message = ChatMessage::new_system_message(
            "User alice joined the room".to_string(),
            "general".to_string(),
        );

        assert_eq!(message.sender_username, "System");
        assert_eq!(message.message_type, MessageType::System);
        assert_eq!(message.status, MessageStatus::Sent);
        assert!(message.is_system_message());
    }

    #[test]
    fn test_create_direct_message() {
        let sender_id = Uuid::new_v4();
        let recipient_id = Uuid::new_v4();

        let message = ChatMessage::new_direct_message(
            "alice".to_string(),
            sender_id,
            recipient_id,
            "Hello Bob!".to_string(),
        );

        assert_eq!(message.sender_id, Some(sender_id));
        assert_eq!(message.recipient_id, Some(recipient_id));
        assert_eq!(message.message_type, MessageType::DirectMessage);
        assert!(message.is_direct_message());
    }

    #[test]
    fn test_message_validation() {
        let mut message = ChatMessage::new_text_message(
            "alice".to_string(),
            Some(Uuid::new_v4()),
            "Hello!".to_string(),
            "general".to_string(),
        );

        assert!(message.validate().is_ok());

        // Empty content should fail
        message.content = "".to_string();
        assert!(message.validate().is_err());

        // Too long content should fail
        message.content = "a".repeat(2001);
        assert!(message.validate().is_err());
    }

    #[test]
    fn test_message_editing() {
        let mut message = ChatMessage::new_text_message(
            "alice".to_string(),
            Some(Uuid::new_v4()),
            "Hello!".to_string(),
            "general".to_string(),
        );

        assert!(!message.edited);
        assert!(message.edited_at.is_none());

        message.edit_content("Hello, edited!".to_string());

        assert_eq!(message.content, "Hello, edited!");
        assert!(message.edited);
        assert!(message.edited_at.is_some());
    }

    #[test]
    fn test_message_store() {
        let mut store = MessageStore::new();

        let message = ChatMessage::new_text_message(
            "alice".to_string(),
            Some(Uuid::new_v4()),
            "Hello!".to_string(),
            "general".to_string(),
        );

        let message_id = message.id;
        assert!(store.store_message(message).is_ok());

        // Should be able to retrieve the message
        assert!(store.get_message(&message_id).is_some());

        // Should appear in room messages
        let room_messages = store.get_room_messages("general", None);
        assert_eq!(room_messages.len(), 1);

        // Should be able to update status
        assert!(store.update_message_status(&message_id, MessageStatus::Sent));

        // Should be able to delete
        assert!(store.delete_message(&message_id));
        assert!(store.get_message(&message_id).is_none());
    }

    #[test]
    fn test_direct_message_store() {
        let mut store = MessageStore::new();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        let message = ChatMessage::new_direct_message(
            "alice".to_string(),
            user1,
            user2,
            "Hello Bob!".to_string(),
        );

        assert!(store.store_message(message).is_ok());

        let dm_messages = store.get_direct_messages(user1, user2, None);
        assert_eq!(dm_messages.len(), 1);

        // Should work with users in reverse order too
        let dm_messages_reverse = store.get_direct_messages(user2, user1, None);
        assert_eq!(dm_messages_reverse.len(), 1);
    }
}
