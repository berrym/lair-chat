//! Direct Message (DM) conversation management
//!
//! This module provides structures and functionality for managing DM conversations,
//! including message storage, conversation state, and history management.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use super::MessageType;

/// Unique identifier for a DM conversation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConversationId {
    /// Sorted pair of user IDs to ensure consistency
    pub participants: (String, String),
}

impl ConversationId {
    /// Create a new conversation ID from two participants
    pub fn new(user1: String, user2: String) -> Self {
        // Sort to ensure consistent ordering regardless of who initiates
        let participants = if user1 <= user2 {
            (user1, user2)
        } else {
            (user2, user1)
        };

        Self { participants }
    }

    /// Get the other participant's username given the current user
    pub fn get_other_participant(&self, current_user: &str) -> Option<&str> {
        if self.participants.0 == current_user {
            Some(&self.participants.1)
        } else if self.participants.1 == current_user {
            Some(&self.participants.0)
        } else {
            None
        }
    }
}

/// A direct message conversation between two users
#[derive(Debug, Clone)]
pub struct DMConversation {
    /// Unique conversation identifier
    pub id: ConversationId,
    /// List of messages in chronological order
    pub messages: Vec<DMMessage>,
    /// Timestamp of conversation creation
    pub created_at: u64,
    /// Timestamp of last activity
    pub last_activity: u64,
    /// Number of unread messages for the current user
    pub unread_count: u32,
    /// Whether the conversation is archived
    pub is_archived: bool,
}

impl DMConversation {
    /// Create a new DM conversation
    pub fn new(user1: String, user2: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: ConversationId::new(user1, user2),
            messages: Vec::new(),
            created_at: now,
            last_activity: now,
            unread_count: 0,
            is_archived: false,
        }
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, message: DMMessage) {
        self.messages.push(message);
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Increment unread count if message is from the other participant
        // Note: In a real implementation, you'd track which user is "current"
        // For now, we'll handle this in the conversation manager
    }

    /// Get messages in chronological order
    pub fn get_messages(&self) -> &[DMMessage] {
        &self.messages
    }

    /// Get the last message in the conversation
    pub fn get_last_message(&self) -> Option<&DMMessage> {
        self.messages.last()
    }

    /// Mark all messages as read
    pub fn mark_as_read(&mut self) {
        self.unread_count = 0;
    }

    /// Get conversation title (other participant's name)
    pub fn get_title(&self, current_user: &str) -> String {
        self.id
            .get_other_participant(current_user)
            .unwrap_or("Unknown User")
            .to_string()
    }

    /// Check if conversation has unread messages
    pub fn has_unread_messages(&self) -> bool {
        self.unread_count > 0
    }
}

/// A direct message within a conversation
#[derive(Debug, Clone)]
pub struct DMMessage {
    /// Unique message identifier
    pub id: Uuid,
    /// Username of the sender
    pub sender: String,
    /// Message content
    pub content: String,
    /// Timestamp when message was sent
    pub timestamp: u64,
    /// Whether the message has been read
    pub is_read: bool,
    /// Optional message type (text, system, etc.)
    pub message_type: MessageType,
}

impl DMMessage {
    /// Create a new DM message
    pub fn new(sender: String, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender,
            content,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            is_read: false,
            message_type: MessageType::Text,
        }
    }

    /// Create a system message for the conversation
    pub fn new_system(content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender: "System".to_string(),
            content,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            is_read: false,
            message_type: MessageType::System,
        }
    }

    /// Mark message as read
    pub fn mark_as_read(&mut self) {
        self.is_read = true;
    }

    /// Format message for display
    pub fn format_for_display(&self, current_user: &str) -> String {
        match self.message_type {
            MessageType::System => self.content.clone(),
            _ => {
                if self.sender == current_user {
                    format!("You: {}", self.content)
                } else {
                    format!("{}: {}", self.sender, self.content)
                }
            }
        }
    }
}

/// Manages all DM conversations for a user
#[derive(Debug)]
pub struct DMConversationManager {
    /// Map of conversation ID to conversation
    conversations: HashMap<ConversationId, DMConversation>,
    /// Current user's username
    current_user: String,
    /// Currently active conversation
    active_conversation: Option<ConversationId>,
}

impl DMConversationManager {
    /// Create a new conversation manager
    pub fn new(current_user: String) -> Self {
        Self {
            conversations: HashMap::new(),
            current_user,
            active_conversation: None,
        }
    }

    /// Start or get existing conversation with another user
    pub fn get_or_create_conversation(&mut self, other_user: String) -> &mut DMConversation {
        let conversation_id = ConversationId::new(self.current_user.clone(), other_user.clone());

        self.conversations
            .entry(conversation_id.clone())
            .or_insert_with(|| DMConversation::new(self.current_user.clone(), other_user))
    }

    /// Get a conversation by ID
    pub fn get_conversation(&self, id: &ConversationId) -> Option<&DMConversation> {
        self.conversations.get(id)
    }

    /// Get a mutable conversation by ID
    pub fn get_conversation_mut(&mut self, id: &ConversationId) -> Option<&mut DMConversation> {
        self.conversations.get_mut(id)
    }

    /// Get conversation with a specific user
    pub fn get_conversation_with_user(&self, other_user: &str) -> Option<&DMConversation> {
        let conversation_id =
            ConversationId::new(self.current_user.clone(), other_user.to_string());
        self.conversations.get(&conversation_id)
    }

    /// Get mutable conversation with a specific user
    pub fn get_conversation_with_user_mut(
        &mut self,
        other_user: &str,
    ) -> Option<&mut DMConversation> {
        let conversation_id =
            ConversationId::new(self.current_user.clone(), other_user.to_string());
        self.conversations.get_mut(&conversation_id)
    }

    /// Add a message to a conversation
    pub fn add_message(&mut self, other_user: String, message: DMMessage) -> Result<(), String> {
        let current_user = self.current_user.clone();
        let active_conversation_id = self.active_conversation.clone();

        let conversation = self.get_or_create_conversation(other_user);

        // If the message is from the other user and this conversation is not active,
        // increment unread count
        if message.sender != current_user
            && active_conversation_id.as_ref() != Some(&conversation.id)
        {
            conversation.unread_count += 1;
        }

        conversation.add_message(message);
        Ok(())
    }

    /// Send a message to another user
    pub fn send_message(&mut self, other_user: String, content: String) -> Result<(), String> {
        let message = DMMessage::new(self.current_user.clone(), content);
        self.add_message(other_user, message)
    }

    /// Receive a message from another user
    pub fn receive_message(&mut self, sender: String, content: String) -> Result<(), String> {
        let message = DMMessage::new(sender.clone(), content);
        self.add_message(sender, message)
    }

    /// Set the active conversation
    pub fn set_active_conversation(&mut self, other_user: Option<String>) {
        if let Some(user) = other_user {
            let conversation_id = ConversationId::new(self.current_user.clone(), user);
            self.active_conversation = Some(conversation_id.clone());

            // Mark conversation as read when activated
            if let Some(conversation) = self.conversations.get_mut(&conversation_id) {
                conversation.mark_as_read();
            }
        } else {
            self.active_conversation = None;
        }
    }

    /// Get the currently active conversation
    pub fn get_active_conversation(&self) -> Option<&DMConversation> {
        self.active_conversation
            .as_ref()
            .and_then(|id| self.conversations.get(id))
    }

    /// Get the currently active conversation (mutable)
    pub fn get_active_conversation_mut(&mut self) -> Option<&mut DMConversation> {
        let active_id = self.active_conversation.clone();
        active_id.and_then(|id| self.conversations.get_mut(&id))
    }

    /// Get all conversations sorted by last activity (most recent first)
    pub fn get_all_conversations(&self) -> Vec<&DMConversation> {
        let mut conversations: Vec<&DMConversation> = self.conversations.values().collect();
        conversations.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        conversations
    }

    /// Get conversations with unread messages
    pub fn get_conversations_with_unread(&self) -> Vec<&DMConversation> {
        self.conversations
            .values()
            .filter(|conv| conv.has_unread_messages())
            .collect()
    }

    /// Get total unread message count across all conversations
    pub fn get_total_unread_count(&self) -> u32 {
        self.conversations
            .values()
            .map(|conv| conv.unread_count)
            .sum()
    }

    /// Clear all conversations (for testing or reset)
    pub fn clear_all(&mut self) {
        self.conversations.clear();
        self.active_conversation = None;
    }

    /// Get the current user
    pub fn get_current_user(&self) -> &str {
        &self.current_user
    }

    /// Check if we have an active conversation
    pub fn has_active_conversation(&self) -> bool {
        self.active_conversation.is_some()
    }

    /// Get the name of the active conversation partner
    pub fn get_active_conversation_partner(&self) -> Option<String> {
        self.active_conversation
            .as_ref()
            .and_then(|id| id.get_other_participant(&self.current_user))
            .map(|s| s.to_string())
    }

    /// Get unread message count for a specific user
    pub fn get_unread_count_with_user(&self, other_user: &str) -> Result<u32, String> {
        let conversation_id =
            ConversationId::new(self.current_user.clone(), other_user.to_string());

        if let Some(conversation) = self.conversations.get(&conversation_id) {
            Ok(conversation.unread_count)
        } else {
            Ok(0) // No conversation means no unread messages
        }
    }

    /// Mark all conversations as read
    pub fn mark_all_read(&mut self) {
        for conversation in self.conversations.values_mut() {
            conversation.mark_as_read();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_id_creation() {
        let id1 = ConversationId::new("alice".to_string(), "bob".to_string());
        let id2 = ConversationId::new("bob".to_string(), "alice".to_string());

        // Should be the same regardless of order
        assert_eq!(id1, id2);
        assert_eq!(id1.participants, ("alice".to_string(), "bob".to_string()));
    }

    #[test]
    fn test_conversation_id_other_participant() {
        let id = ConversationId::new("alice".to_string(), "bob".to_string());

        assert_eq!(id.get_other_participant("alice"), Some("bob"));
        assert_eq!(id.get_other_participant("bob"), Some("alice"));
        assert_eq!(id.get_other_participant("charlie"), None);
    }

    #[test]
    fn test_dm_conversation_creation() {
        let conv = DMConversation::new("alice".to_string(), "bob".to_string());

        assert_eq!(conv.messages.len(), 0);
        assert_eq!(conv.unread_count, 0);
        assert!(!conv.is_archived);
        assert_eq!(conv.get_title("alice"), "bob");
        assert_eq!(conv.get_title("bob"), "alice");
    }

    #[test]
    fn test_dm_message_creation() {
        let message = DMMessage::new("alice".to_string(), "Hello!".to_string());

        assert_eq!(message.sender, "alice");
        assert_eq!(message.content, "Hello!");
        assert!(!message.is_read);
        assert_eq!(message.message_type, MessageType::Text);
    }

    #[test]
    fn test_conversation_manager() {
        let mut manager = DMConversationManager::new("alice".to_string());

        // Send a message
        manager
            .send_message("bob".to_string(), "Hello Bob!".to_string())
            .unwrap();

        // Check conversation exists
        let conversation = manager.get_conversation_with_user("bob").unwrap();
        assert_eq!(conversation.messages.len(), 1);
        assert_eq!(conversation.messages[0].content, "Hello Bob!");

        // Receive a message
        manager
            .receive_message("bob".to_string(), "Hi Alice!".to_string())
            .unwrap();

        let conversation = manager.get_conversation_with_user("bob").unwrap();
        assert_eq!(conversation.messages.len(), 2);
        assert_eq!(conversation.unread_count, 1); // Bob's message is unread

        // Set active conversation and check unread count is cleared
        manager.set_active_conversation(Some("bob".to_string()));
        let conversation = manager.get_conversation_with_user("bob").unwrap();
        assert_eq!(conversation.unread_count, 0);
    }

    #[test]
    fn test_message_formatting() {
        let message1 = DMMessage::new("alice".to_string(), "Hello!".to_string());
        let message2 = DMMessage::new("bob".to_string(), "Hi there!".to_string());
        let system_msg = DMMessage::new_system("User joined".to_string());

        assert_eq!(message1.format_for_display("alice"), "You: Hello!");
        assert_eq!(message1.format_for_display("bob"), "alice: Hello!");
        assert_eq!(message2.format_for_display("alice"), "bob: Hi there!");
        assert_eq!(system_msg.format_for_display("alice"), "User joined");
    }
}
