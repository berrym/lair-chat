//! Direct messaging structures for Lair-Chat
//! Handles direct messages between users, conversation management, and message targeting.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{MessageId, MessageType, UserId};
use crate::client::messages::FileAttachment;

/// Message targeting enum for routing messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageTarget {
    /// Broadcast to all users in a room (existing behavior)
    Broadcast,
    /// Direct message to a specific user
    DirectMessage(UserId),
    /// Group message to multiple users (future enhancement)
    GroupMessage(Vec<UserId>),
}

impl MessageTarget {
    /// Check if this is a direct message
    pub fn is_direct(&self) -> bool {
        matches!(self, MessageTarget::DirectMessage(_))
    }

    /// Check if this is a group message
    pub fn is_group(&self) -> bool {
        matches!(self, MessageTarget::GroupMessage(_))
    }

    /// Check if this is a broadcast message
    pub fn is_broadcast(&self) -> bool {
        matches!(self, MessageTarget::Broadcast)
    }

    /// Get the target user ID if this is a direct message
    pub fn target_user(&self) -> Option<UserId> {
        match self {
            MessageTarget::DirectMessage(user_id) => Some(*user_id),
            _ => None,
        }
    }

    /// Get all target users (for group messages)
    pub fn target_users(&self) -> Vec<UserId> {
        match self {
            MessageTarget::DirectMessage(user_id) => vec![*user_id],
            MessageTarget::GroupMessage(users) => users.clone(),
            MessageTarget::Broadcast => vec![],
        }
    }
}

impl std::fmt::Display for MessageTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageTarget::Broadcast => write!(f, "Broadcast"),
            MessageTarget::DirectMessage(user_id) => write!(f, "DM to {}", user_id),
            MessageTarget::GroupMessage(users) => write!(f, "Group to {} users", users.len()),
        }
    }
}

/// Direct message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessage {
    /// Unique message identifier
    pub id: MessageId,
    /// Sender user ID
    pub sender_id: UserId,
    /// Recipient user ID
    pub recipient_id: UserId,
    /// Message content
    pub content: String,
    /// Message type (text, file, etc.)
    pub message_type: MessageType,
    /// Message creation timestamp
    pub created_at: u64,
    /// When message was read (if read)
    pub read_at: Option<u64>,
    /// When message was edited (if edited)
    pub edited_at: Option<u64>,
    /// File attachments
    pub attachments: Vec<FileAttachment>,
    /// Message metadata
    pub metadata: HashMap<String, String>,
    /// Message delivery status
    pub delivery_status: MessageDeliveryStatus,
    /// Original message ID if this is an edit
    pub original_message_id: Option<MessageId>,
}

/// Message delivery status for direct messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageDeliveryStatus {
    /// Message is being sent
    Sending,
    /// Message was sent successfully
    Sent,
    /// Message was delivered to recipient
    Delivered,
    /// Message was read by recipient
    Read,
    /// Message failed to send
    Failed(String),
    /// Message was edited
    Edited,
    /// Message was deleted
    Deleted,
}

impl MessageDeliveryStatus {
    /// Check if message was successfully delivered
    pub fn is_delivered(&self) -> bool {
        matches!(
            self,
            MessageDeliveryStatus::Delivered | MessageDeliveryStatus::Read
        )
    }

    /// Check if message was read
    pub fn is_read(&self) -> bool {
        matches!(self, MessageDeliveryStatus::Read)
    }

    /// Check if message failed
    pub fn is_failed(&self) -> bool {
        matches!(self, MessageDeliveryStatus::Failed(_))
    }

    /// Check if message is still sending
    pub fn is_sending(&self) -> bool {
        matches!(self, MessageDeliveryStatus::Sending)
    }
}

impl std::fmt::Display for MessageDeliveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageDeliveryStatus::Sending => write!(f, "Sending"),
            MessageDeliveryStatus::Sent => write!(f, "Sent"),
            MessageDeliveryStatus::Delivered => write!(f, "Delivered"),
            MessageDeliveryStatus::Read => write!(f, "Read"),
            MessageDeliveryStatus::Failed(reason) => write!(f, "Failed: {}", reason),
            MessageDeliveryStatus::Edited => write!(f, "Edited"),
            MessageDeliveryStatus::Deleted => write!(f, "Deleted"),
        }
    }
}

impl DirectMessage {
    /// Create a new direct message
    pub fn new(
        sender_id: UserId,
        recipient_id: UserId,
        content: String,
        message_type: MessageType,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: uuid::Uuid::new_v4(),
            sender_id,
            recipient_id,
            content,
            message_type,
            created_at: now,
            read_at: None,
            edited_at: None,
            attachments: Vec::new(),
            metadata: HashMap::new(),
            delivery_status: MessageDeliveryStatus::Sending,
            original_message_id: None,
        }
    }

    /// Create a new text direct message
    pub fn new_text(sender_id: UserId, recipient_id: UserId, content: String) -> Self {
        Self::new(sender_id, recipient_id, content, MessageType::Text)
    }

    /// Create a new file direct message
    pub fn new_file(
        sender_id: UserId,
        recipient_id: UserId,
        content: String,
        attachment: FileAttachment,
    ) -> Self {
        let mut message = Self::new(sender_id, recipient_id, content, MessageType::File);
        message.attachments.push(attachment);
        message
    }

    /// Mark message as read
    pub fn mark_as_read(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.read_at = Some(now);
        self.delivery_status = MessageDeliveryStatus::Read;
    }

    /// Mark message as delivered
    pub fn mark_as_delivered(&mut self) {
        if !self.delivery_status.is_read() {
            self.delivery_status = MessageDeliveryStatus::Delivered;
        }
    }

    /// Mark message as sent
    pub fn mark_as_sent(&mut self) {
        if self.delivery_status.is_sending() {
            self.delivery_status = MessageDeliveryStatus::Sent;
        }
    }

    /// Mark message as failed
    pub fn mark_as_failed(&mut self, reason: String) {
        self.delivery_status = MessageDeliveryStatus::Failed(reason);
    }

    /// Edit message content
    pub fn edit(&mut self, new_content: String) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.content = new_content;
        self.edited_at = Some(now);
        self.delivery_status = MessageDeliveryStatus::Edited;
    }

    /// Delete message (mark as deleted)
    pub fn delete(&mut self) {
        self.delivery_status = MessageDeliveryStatus::Deleted;
        self.content = String::new(); // Clear content for privacy
    }

    /// Check if message is read
    pub fn is_read(&self) -> bool {
        self.read_at.is_some()
    }

    /// Check if message is edited
    pub fn is_edited(&self) -> bool {
        self.edited_at.is_some()
    }

    /// Check if message is deleted
    pub fn is_deleted(&self) -> bool {
        matches!(self.delivery_status, MessageDeliveryStatus::Deleted)
    }

    /// Get age of message in seconds
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

    /// Add metadata
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Get conversation ID for this message
    pub fn conversation_id(&self) -> ConversationId {
        ConversationId::from_participants(self.sender_id, self.recipient_id)
    }
}

/// Unique identifier for a conversation
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationId {
    /// Deterministic ID based on participant IDs
    id: String,
}

impl ConversationId {
    /// Create conversation ID from two participants
    pub fn from_participants(user1: UserId, user2: UserId) -> Self {
        // Create deterministic ID by sorting user IDs
        let mut users = vec![user1, user2];
        users.sort();
        let id = format!("conv_{}_{}", users[0], users[1]);

        Self { id }
    }

    /// Create conversation ID from multiple participants (for group chats)
    pub fn from_group(mut participants: Vec<UserId>) -> Self {
        participants.sort();
        let id = format!(
            "group_{}",
            participants
                .iter()
                .map(|u| u.to_string())
                .collect::<Vec<_>>()
                .join("_")
        );

        Self { id }
    }

    /// Get the string representation
    pub fn as_str(&self) -> &str {
        &self.id
    }
}

impl std::fmt::Display for ConversationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl From<String> for ConversationId {
    fn from(id: String) -> Self {
        Self { id }
    }
}

impl From<&str> for ConversationId {
    fn from(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

/// Direct conversation between two users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectConversation {
    /// Conversation identifier
    pub id: ConversationId,
    /// Participants in the conversation (always 2 for direct messages)
    pub participants: [UserId; 2],
    /// Messages in the conversation
    pub messages: Vec<DirectMessage>,
    /// Last activity timestamp
    pub last_activity: u64,
    /// Unread message count per user
    pub unread_count: HashMap<UserId, u32>,
    /// Conversation metadata
    pub metadata: HashMap<String, String>,
    /// Whether conversation is archived
    pub is_archived: bool,
    /// Whether conversation is muted
    pub is_muted: bool,
}

impl DirectConversation {
    /// Create a new conversation between two users
    pub fn new(user1: UserId, user2: UserId) -> Self {
        let id = ConversationId::from_participants(user1, user2);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut unread_count = HashMap::new();
        unread_count.insert(user1, 0);
        unread_count.insert(user2, 0);

        Self {
            id,
            participants: [user1, user2],
            messages: Vec::new(),
            last_activity: now,
            unread_count,
            metadata: HashMap::new(),
            is_archived: false,
            is_muted: false,
        }
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, message: DirectMessage) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Update unread count for recipient
        let recipient_id = message.recipient_id;
        if let Some(count) = self.unread_count.get_mut(&recipient_id) {
            *count += 1;
        }

        self.messages.push(message);
        self.last_activity = now;
    }

    /// Mark messages as read by a user
    pub fn mark_messages_read(&mut self, user_id: UserId, up_to_message_id: Option<MessageId>) {
        // Reset unread count for user
        if let Some(count) = self.unread_count.get_mut(&user_id) {
            *count = 0;
        }

        // Mark messages as read
        for message in &mut self.messages {
            if message.recipient_id == user_id {
                if let Some(msg_id) = up_to_message_id {
                    if message.id == msg_id {
                        message.mark_as_read();
                        break;
                    }
                } else {
                    message.mark_as_read();
                }
            }
        }
    }

    /// Get unread count for a user
    pub fn unread_count_for_user(&self, user_id: UserId) -> u32 {
        self.unread_count.get(&user_id).copied().unwrap_or(0)
    }

    /// Get total message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get the other participant in the conversation
    pub fn other_participant(&self, user_id: UserId) -> Option<UserId> {
        if self.participants[0] == user_id {
            Some(self.participants[1])
        } else if self.participants[1] == user_id {
            Some(self.participants[0])
        } else {
            None
        }
    }

    /// Check if user is participant in conversation
    pub fn has_participant(&self, user_id: UserId) -> bool {
        self.participants.contains(&user_id)
    }

    /// Get messages after a certain timestamp
    pub fn messages_after(&self, timestamp: u64) -> Vec<&DirectMessage> {
        self.messages
            .iter()
            .filter(|msg| msg.created_at > timestamp)
            .collect()
    }

    /// Get recent messages (last N messages)
    pub fn recent_messages(&self, count: usize) -> Vec<&DirectMessage> {
        self.messages
            .iter()
            .rev()
            .take(count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Get last message in conversation
    pub fn last_message(&self) -> Option<&DirectMessage> {
        self.messages.last()
    }

    /// Archive conversation
    pub fn archive(&mut self) {
        self.is_archived = true;
    }

    /// Unarchive conversation
    pub fn unarchive(&mut self) {
        self.is_archived = false;
    }

    /// Mute conversation
    pub fn mute(&mut self) {
        self.is_muted = true;
    }

    /// Unmute conversation
    pub fn unmute(&mut self) {
        self.is_muted = false;
    }

    /// Set conversation metadata
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get conversation metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Get conversation age in seconds
    pub fn age_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(self.last_activity)
    }

    /// Get human-readable last activity
    pub fn human_last_activity(&self) -> String {
        let age = self.age_seconds();
        if age < 60 {
            "Just now".to_string()
        } else if age < 3600 {
            format!("{}m ago", age / 60)
        } else if age < 86400 {
            format!("{}h ago", age / 3600)
        } else {
            format!("{}d ago", age / 86400)
        }
    }
}

/// Conversation summary for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: ConversationId,
    pub other_user_id: UserId,
    pub other_username: String,
    pub last_message: Option<String>,
    pub last_activity: u64,
    pub unread_count: u32,
    pub is_archived: bool,
    pub is_muted: bool,
}

impl ConversationSummary {
    /// Create summary from conversation for a specific user
    pub fn from_conversation(conversation: &DirectConversation, user_id: UserId) -> Option<Self> {
        let other_user_id = conversation.other_participant(user_id)?;

        Some(Self {
            id: conversation.id.clone(),
            other_user_id,
            other_username: format!("User_{}", other_user_id), // TODO: Get from user cache
            last_message: conversation.last_message().map(|msg| {
                if msg.is_deleted() {
                    "Message deleted".to_string()
                } else if msg.content.len() > 50 {
                    format!("{}...", &msg.content[..47])
                } else {
                    msg.content.clone()
                }
            }),
            last_activity: conversation.last_activity,
            unread_count: conversation.unread_count_for_user(user_id),
            is_archived: conversation.is_archived,
            is_muted: conversation.is_muted,
        })
    }

    /// Get human-readable last activity
    pub fn human_last_activity(&self) -> String {
        let age = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(self.last_activity);

        if age < 60 {
            "Just now".to_string()
        } else if age < 3600 {
            format!("{}m ago", age / 60)
        } else if age < 86400 {
            format!("{}h ago", age / 3600)
        } else {
            format!("{}d ago", age / 86400)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_target() {
        let user_id = uuid::Uuid::new_v4();
        let target = MessageTarget::DirectMessage(user_id);

        assert!(target.is_direct());
        assert!(!target.is_broadcast());
        assert_eq!(target.target_user(), Some(user_id));
    }

    #[test]
    fn test_direct_message_creation() {
        let sender = uuid::Uuid::new_v4();
        let recipient = uuid::Uuid::new_v4();
        let content = "Hello world!".to_string();

        let message = DirectMessage::new_text(sender, recipient, content.clone());

        assert_eq!(message.sender_id, sender);
        assert_eq!(message.recipient_id, recipient);
        assert_eq!(message.content, content);
        assert_eq!(message.message_type, MessageType::Text);
        assert!(message.delivery_status.is_sending());
    }

    #[test]
    fn test_conversation_id_generation() {
        let user1 = uuid::Uuid::new_v4();
        let user2 = uuid::Uuid::new_v4();

        let id1 = ConversationId::from_participants(user1, user2);
        let id2 = ConversationId::from_participants(user2, user1);

        // Should be the same regardless of order
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_conversation_management() {
        let user1 = uuid::Uuid::new_v4();
        let user2 = uuid::Uuid::new_v4();

        let mut conversation = DirectConversation::new(user1, user2);

        assert_eq!(conversation.message_count(), 0);
        assert_eq!(conversation.unread_count_for_user(user1), 0);

        let message = DirectMessage::new_text(user1, user2, "Hello".to_string());
        conversation.add_message(message);

        assert_eq!(conversation.message_count(), 1);
        assert_eq!(conversation.unread_count_for_user(user2), 1);

        conversation.mark_messages_read(user2, None);
        assert_eq!(conversation.unread_count_for_user(user2), 0);
    }

    #[test]
    fn test_message_status_updates() {
        let sender = uuid::Uuid::new_v4();
        let recipient = uuid::Uuid::new_v4();
        let mut message = DirectMessage::new_text(sender, recipient, "Test".to_string());

        assert!(message.delivery_status.is_sending());

        message.mark_as_sent();
        assert_eq!(message.delivery_status, MessageDeliveryStatus::Sent);

        message.mark_as_delivered();
        assert!(message.delivery_status.is_delivered());

        message.mark_as_read();
        assert!(message.delivery_status.is_read());
        assert!(message.is_read());
    }

    #[test]
    fn test_message_editing() {
        let sender = uuid::Uuid::new_v4();
        let recipient = uuid::Uuid::new_v4();
        let mut message = DirectMessage::new_text(sender, recipient, "Original".to_string());

        assert!(!message.is_edited());

        message.edit("Edited content".to_string());

        assert!(message.is_edited());
        assert_eq!(message.content, "Edited content");
        assert_eq!(message.delivery_status, MessageDeliveryStatus::Edited);
    }
}
