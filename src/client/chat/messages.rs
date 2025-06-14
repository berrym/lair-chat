//! Chat message structures for Lair-Chat
//! Handles message creation, editing, reactions, and metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{MessageId, RoomId, UserId};

/// Message type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    /// Regular text message
    Text,
    /// System message (user joined, left, etc.)
    System,
    /// File attachment
    File,
    /// Image attachment
    Image,
    /// Audio message
    Audio,
    /// Video message
    Video,
    /// Link preview
    Link,
    /// Code snippet
    Code,
    /// Quoted/reply message
    Reply,
    /// Edited message
    Edit,
    /// Deleted message
    Deleted,
}

/// Message status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageStatus {
    /// Message is being sent
    Sending,
    /// Message was sent successfully
    Sent,
    /// Message was delivered to server
    Delivered,
    /// Message was read by recipient(s)
    Read,
    /// Message failed to send
    Failed,
    /// Message was edited
    Edited,
    /// Message was deleted
    Deleted,
}

/// Message reaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReaction {
    /// Emoji or reaction identifier
    pub emoji: String,
    /// Users who reacted with this emoji
    pub users: Vec<UserId>,
    /// When the reaction was first added
    pub first_added: u64,
}

impl MessageReaction {
    /// Create a new reaction
    pub fn new(emoji: String, user_id: UserId) -> Self {
        Self {
            emoji,
            users: vec![user_id],
            first_added: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Add a user to this reaction
    pub fn add_user(&mut self, user_id: UserId) -> bool {
        if !self.users.contains(&user_id) {
            self.users.push(user_id);
            true
        } else {
            false
        }
    }

    /// Remove a user from this reaction
    pub fn remove_user(&mut self, user_id: &UserId) -> bool {
        if let Some(pos) = self.users.iter().position(|id| id == user_id) {
            self.users.remove(pos);
            true
        } else {
            false
        }
    }

    /// Check if reaction is empty
    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }

    /// Get reaction count
    pub fn count(&self) -> usize {
        self.users.len()
    }
}

/// File attachment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAttachment {
    /// Original filename
    pub filename: String,
    /// File size in bytes
    pub size: u64,
    /// MIME type
    pub mime_type: String,
    /// File URL or path
    pub url: String,
    /// Thumbnail URL (for images/videos)
    pub thumbnail_url: Option<String>,
    /// File dimensions (for images/videos)
    pub dimensions: Option<(u32, u32)>,
    /// Duration in seconds (for audio/video)
    pub duration: Option<f64>,
}

impl FileAttachment {
    /// Create a new file attachment
    pub fn new(filename: String, size: u64, mime_type: String, url: String) -> Self {
        Self {
            filename,
            size,
            mime_type,
            url,
            thumbnail_url: None,
            dimensions: None,
            duration: None,
        }
    }

    /// Check if this is an image
    pub fn is_image(&self) -> bool {
        self.mime_type.starts_with("image/")
    }

    /// Check if this is a video
    pub fn is_video(&self) -> bool {
        self.mime_type.starts_with("video/")
    }

    /// Check if this is audio
    pub fn is_audio(&self) -> bool {
        self.mime_type.starts_with("audio/")
    }

    /// Get human-readable file size
    pub fn human_size(&self) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = self.size as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if size.fract() == 0.0 {
            format!("{:.0} {}", size, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}

/// Message read receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadReceipt {
    /// User who read the message
    pub user_id: UserId,
    /// When the message was read
    pub read_at: u64,
}

impl ReadReceipt {
    /// Create a new read receipt
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            read_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// Chat message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique message identifier
    pub id: MessageId,
    /// Room this message belongs to
    pub room_id: RoomId,
    /// User who sent the message
    pub sender_id: UserId,
    /// Username of sender (for display)
    pub sender_username: String,
    /// Message content
    pub content: String,
    /// Message type
    pub message_type: MessageType,
    /// Message status
    pub status: MessageStatus,
    /// When the message was created
    pub created_at: u64,
    /// When the message was last edited (if any)
    pub edited_at: Option<u64>,
    /// Original message ID (for replies)
    pub reply_to: Option<MessageId>,
    /// File attachments
    pub attachments: Vec<FileAttachment>,
    /// Message reactions
    pub reactions: HashMap<String, MessageReaction>,
    /// Read receipts
    pub read_receipts: Vec<ReadReceipt>,
    /// Message metadata
    pub metadata: HashMap<String, String>,
}

impl ChatMessage {
    /// Create a new text message
    pub fn new_text(
        room_id: RoomId,
        sender_id: UserId,
        sender_username: String,
        content: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            room_id,
            sender_id,
            sender_username,
            content,
            message_type: MessageType::Text,
            status: MessageStatus::Sending,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            edited_at: None,
            reply_to: None,
            attachments: Vec::new(),
            reactions: HashMap::new(),
            read_receipts: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new system message
    pub fn new_system(room_id: RoomId, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            room_id,
            sender_id: uuid::Uuid::nil(), // System messages have no sender
            sender_username: "System".to_string(),
            content,
            message_type: MessageType::System,
            status: MessageStatus::Sent,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            edited_at: None,
            reply_to: None,
            attachments: Vec::new(),
            reactions: HashMap::new(),
            read_receipts: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Create a reply message
    pub fn new_reply(
        room_id: RoomId,
        sender_id: UserId,
        sender_username: String,
        content: String,
        reply_to: MessageId,
    ) -> Self {
        let mut message = Self::new_text(room_id, sender_id, sender_username, content);
        message.message_type = MessageType::Reply;
        message.reply_to = Some(reply_to);
        message
    }

    /// Create a file message
    pub fn new_file(
        room_id: RoomId,
        sender_id: UserId,
        sender_username: String,
        content: String,
        attachment: FileAttachment,
    ) -> Self {
        let message_type = if attachment.is_image() {
            MessageType::Image
        } else if attachment.is_video() {
            MessageType::Video
        } else if attachment.is_audio() {
            MessageType::Audio
        } else {
            MessageType::File
        };

        let mut message = Self::new_text(room_id, sender_id, sender_username, content);
        message.message_type = message_type;
        message.attachments.push(attachment);
        message
    }

    /// Edit the message content
    pub fn edit(&mut self, new_content: String) {
        self.content = new_content;
        self.edited_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
        self.status = MessageStatus::Edited;
    }

    /// Mark message as deleted
    pub fn delete(&mut self) {
        self.content = "This message was deleted".to_string();
        self.message_type = MessageType::Deleted;
        self.status = MessageStatus::Deleted;
        self.attachments.clear();
        self.reactions.clear();
    }

    /// Add a reaction to the message
    pub fn add_reaction(&mut self, emoji: String, user_id: UserId) -> bool {
        if let Some(reaction) = self.reactions.get_mut(&emoji) {
            reaction.add_user(user_id)
        } else {
            self.reactions
                .insert(emoji.clone(), MessageReaction::new(emoji, user_id));
            true
        }
    }

    /// Remove a reaction from the message
    pub fn remove_reaction(&mut self, emoji: &str, user_id: &UserId) -> bool {
        if let Some(reaction) = self.reactions.get_mut(emoji) {
            let removed = reaction.remove_user(user_id);
            if reaction.is_empty() {
                self.reactions.remove(emoji);
            }
            removed
        } else {
            false
        }
    }

    /// Add a read receipt
    pub fn add_read_receipt(&mut self, user_id: UserId) -> bool {
        // Don't add receipt if user already read it
        if self.read_receipts.iter().any(|r| r.user_id == user_id) {
            return false;
        }

        // Don't add receipt for the sender
        if user_id == self.sender_id {
            return false;
        }

        self.read_receipts.push(ReadReceipt::new(user_id));
        true
    }

    /// Check if message was read by user
    pub fn is_read_by(&self, user_id: &UserId) -> bool {
        self.read_receipts.iter().any(|r| r.user_id == *user_id)
    }

    /// Get read receipt count
    pub fn read_count(&self) -> usize {
        self.read_receipts.len()
    }

    /// Check if message is edited
    pub fn is_edited(&self) -> bool {
        self.edited_at.is_some()
    }

    /// Check if message is deleted
    pub fn is_deleted(&self) -> bool {
        matches!(self.message_type, MessageType::Deleted)
    }

    /// Check if message is a reply
    pub fn is_reply(&self) -> bool {
        self.reply_to.is_some()
    }

    /// Check if message has attachments
    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }

    /// Check if message has reactions
    pub fn has_reactions(&self) -> bool {
        !self.reactions.is_empty()
    }

    /// Get total reaction count
    pub fn reaction_count(&self) -> usize {
        self.reactions.values().map(|r| r.count()).sum()
    }

    /// Get message age in seconds
    pub fn age_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(self.created_at)
    }

    /// Get human-readable age
    pub fn human_age(&self) -> String {
        let age = self.age_seconds();

        if age < 60 {
            format!("{}s", age)
        } else if age < 3600 {
            format!("{}m", age / 60)
        } else if age < 86400 {
            format!("{}h", age / 3600)
        } else {
            format!("{}d", age / 86400)
        }
    }

    /// Set metadata
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Update status
    pub fn update_status(&mut self, status: MessageStatus) {
        self.status = status;
    }

    /// Check if message can be edited by user
    pub fn can_edit(&self, user_id: &UserId) -> bool {
        // Only sender can edit their own messages
        self.sender_id == *user_id
            && !self.is_deleted()
            && matches!(self.message_type, MessageType::Text | MessageType::Reply)
    }

    /// Check if message can be deleted by user
    pub fn can_delete(&self, user_id: &UserId, is_moderator: bool) -> bool {
        // Sender can delete their own messages, moderators can delete any message
        !self.is_deleted() && (self.sender_id == *user_id || is_moderator)
    }
}

/// Message thread for organizing conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageThread {
    /// Root message that started the thread
    pub root_message_id: MessageId,
    /// All messages in the thread
    pub messages: Vec<MessageId>,
    /// Participants in the thread
    pub participants: Vec<UserId>,
    /// Thread creation timestamp
    pub created_at: u64,
    /// Last activity in thread
    pub last_activity: u64,
}

impl MessageThread {
    /// Create a new message thread
    pub fn new(root_message_id: MessageId) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            root_message_id,
            messages: vec![root_message_id],
            participants: Vec::new(),
            created_at: now,
            last_activity: now,
        }
    }

    /// Add a message to the thread
    pub fn add_message(&mut self, message_id: MessageId, sender_id: UserId) {
        self.messages.push(message_id);

        if !self.participants.contains(&sender_id) {
            self.participants.push(sender_id);
        }

        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Get message count in thread
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get participant count
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let room_id = uuid::Uuid::new_v4();
        let sender_id = uuid::Uuid::new_v4();
        let username = "testuser".to_string();
        let content = "Hello, world!".to_string();

        let message = ChatMessage::new_text(room_id, sender_id, username.clone(), content.clone());

        assert_eq!(message.room_id, room_id);
        assert_eq!(message.sender_id, sender_id);
        assert_eq!(message.sender_username, username);
        assert_eq!(message.content, content);
        assert_eq!(message.message_type, MessageType::Text);
        assert_eq!(message.status, MessageStatus::Sending);
    }

    #[test]
    fn test_message_reactions() {
        let room_id = uuid::Uuid::new_v4();
        let sender_id = uuid::Uuid::new_v4();
        let user_id = uuid::Uuid::new_v4();

        let mut message = ChatMessage::new_text(
            room_id,
            sender_id,
            "sender".to_string(),
            "Test message".to_string(),
        );

        // Add reaction
        assert!(message.add_reaction("👍".to_string(), user_id));
        assert_eq!(message.reaction_count(), 1);
        assert!(message.has_reactions());

        // Add same reaction from same user (should not increase count)
        assert!(!message.add_reaction("👍".to_string(), user_id));
        assert_eq!(message.reaction_count(), 1);

        // Remove reaction
        assert!(message.remove_reaction("👍", &user_id));
        assert_eq!(message.reaction_count(), 0);
        assert!(!message.has_reactions());
    }

    #[test]
    fn test_message_editing() {
        let room_id = uuid::Uuid::new_v4();
        let sender_id = uuid::Uuid::new_v4();

        let mut message = ChatMessage::new_text(
            room_id,
            sender_id,
            "sender".to_string(),
            "Original content".to_string(),
        );

        assert!(!message.is_edited());

        message.edit("Edited content".to_string());

        assert_eq!(message.content, "Edited content");
        assert!(message.is_edited());
        assert_eq!(message.status, MessageStatus::Edited);
    }

    #[test]
    fn test_file_attachment() {
        let attachment = FileAttachment::new(
            "test.jpg".to_string(),
            1024 * 1024,
            "image/jpeg".to_string(),
            "/files/test.jpg".to_string(),
        );

        assert!(attachment.is_image());
        assert!(!attachment.is_video());
        assert!(!attachment.is_audio());
        assert_eq!(attachment.human_size(), "1.0 MB");
    }

    #[test]
    fn test_read_receipts() {
        let room_id = uuid::Uuid::new_v4();
        let sender_id = uuid::Uuid::new_v4();
        let reader_id = uuid::Uuid::new_v4();

        let mut message = ChatMessage::new_text(
            room_id,
            sender_id,
            "sender".to_string(),
            "Test message".to_string(),
        );

        // Add read receipt
        assert!(message.add_read_receipt(reader_id));
        assert_eq!(message.read_count(), 1);
        assert!(message.is_read_by(&reader_id));

        // Try to add receipt from sender (should fail)
        assert!(!message.add_read_receipt(sender_id));
        assert_eq!(message.read_count(), 1);

        // Try to add duplicate receipt (should fail)
        assert!(!message.add_read_receipt(reader_id));
        assert_eq!(message.read_count(), 1);
    }
}
