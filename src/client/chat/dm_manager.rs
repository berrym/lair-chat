//! Direct message manager for Lair-Chat
//! Handles direct message business logic, conversation management, and integration with ConnectionManager.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use tokio::time::Instant;

use crate::common::protocol::{protocol_utils, MessageEnvelope, MessageRoute, ProtocolMessage};
use crate::common::transport::{ConnectionObserver, TransportError};
use crate::connection_manager::ConnectionManager;

use super::{
    ConversationId, ConversationSummary, DirectConversation, DirectMessage, MessageDeliveryStatus,
    MessageId, UserId, UserManager, UserPresence,
};

/// Error types for direct message operations
#[derive(Debug, thiserror::Error)]
pub enum DirectMessageError {
    #[error("User not found: {0}")]
    UserNotFound(UserId),

    #[error("Conversation not found: {0}")]
    ConversationNotFound(ConversationId),

    #[error("Message not found: {0}")]
    MessageNotFound(MessageId),

    #[error("Transport error: {0}")]
    TransportError(#[from] TransportError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Connection not available")]
    ConnectionNotAvailable,
}

/// Result type for direct message operations
pub type DirectMessageResult<T> = Result<T, DirectMessageError>;

/// Event types for direct message notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DirectMessageEvent {
    /// New direct message received
    MessageReceived {
        conversation_id: ConversationId,
        message: DirectMessage,
    },
    /// Message delivery status updated
    MessageStatusUpdated {
        message_id: MessageId,
        status: MessageDeliveryStatus,
    },
    /// User started typing
    TypingStarted {
        conversation_id: ConversationId,
        user_id: UserId,
    },
    /// User stopped typing
    TypingStopped {
        conversation_id: ConversationId,
        user_id: UserId,
    },
    /// New conversation created
    ConversationCreated { conversation: DirectConversation },
    /// User presence updated
    PresenceUpdated {
        user_id: UserId,
        presence: UserPresence,
    },
    /// Read receipt received
    ReadReceiptReceived {
        message_id: MessageId,
        reader_id: UserId,
        read_at: u64,
    },
}

/// Observer trait for direct message events
pub trait DirectMessageObserver: Send + Sync {
    /// Called when a direct message event occurs
    fn on_dm_event(&self, event: DirectMessageEvent);
}

/// Direct message manager
pub struct DirectMessageManager {
    /// Active conversations
    conversations: Arc<RwLock<HashMap<ConversationId, DirectConversation>>>,
    /// User manager for presence tracking
    user_manager: Arc<UserManager>,
    /// Connection manager for transport
    connection_manager: Arc<Mutex<ConnectionManager>>,
    /// Event observers
    observers: Arc<RwLock<Vec<Arc<dyn DirectMessageObserver>>>>,
    /// Current user ID
    current_user_id: Option<UserId>,
    /// Message delivery tracking
    pending_messages: Arc<RwLock<HashMap<MessageId, MessageEnvelope>>>,
    /// Typing indicators timeout (seconds)
    typing_timeout: u64,
    /// Rate limiting for message sending
    rate_limiter: Arc<RwLock<RateLimiter>>,
    /// Connection retry configuration
    retry_config: RetryConfig,
}

impl DirectMessageManager {
    /// Create new direct message manager
    pub fn new(
        user_manager: Arc<UserManager>,
        connection_manager: Arc<Mutex<ConnectionManager>>,
    ) -> Self {
        Self {
            conversations: Arc::new(RwLock::new(HashMap::new())),
            user_manager,
            connection_manager,
            observers: Arc::new(RwLock::new(Vec::new())),
            current_user_id: None,
            pending_messages: Arc::new(RwLock::new(HashMap::new())),
            typing_timeout: 5, // 5 seconds
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new(10, Duration::from_secs(60)))), // 10 messages per minute
            retry_config: RetryConfig::default(),
        }
    }

    /// Create new direct message manager with custom rate limiting
    pub fn new_with_rate_limit(
        user_manager: Arc<UserManager>,
        connection_manager: Arc<Mutex<ConnectionManager>>,
        max_messages: u32,
        window_duration: Duration,
    ) -> Self {
        Self {
            conversations: Arc::new(RwLock::new(HashMap::new())),
            user_manager,
            connection_manager,
            observers: Arc::new(RwLock::new(Vec::new())),
            current_user_id: None,
            pending_messages: Arc::new(RwLock::new(HashMap::new())),
            typing_timeout: 5,
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new(max_messages, window_duration))),
            retry_config: RetryConfig::default(),
        }
    }

    /// Set the current user ID
    pub fn set_current_user(&mut self, user_id: UserId) {
        self.current_user_id = Some(user_id);
    }

    /// Add event observer
    pub async fn add_observer(&self, observer: Arc<dyn DirectMessageObserver>) {
        let mut observers = self.observers.write().await;
        observers.push(observer);
    }

    /// Remove event observer
    pub async fn remove_observer(&self, observer: Arc<dyn DirectMessageObserver>) {
        let mut observers = self.observers.write().await;
        observers.retain(|obs| !Arc::ptr_eq(obs, &observer));
    }

    /// Send a direct message
    pub async fn send_direct_message(
        &self,
        recipient_id: UserId,
        content: String,
    ) -> DirectMessageResult<MessageId> {
        // Check rate limit
        {
            let mut rate_limiter = self.rate_limiter.write().await;
            if !rate_limiter.allow_request() {
                return Err(DirectMessageError::RateLimitExceeded);
            }
        }

        let sender_id = self
            .current_user_id
            .ok_or(DirectMessageError::ConnectionNotAvailable)?;

        // Create direct message
        let message = DirectMessage::new_text(sender_id, recipient_id, content.clone());
        let message_id = message.id;

        // Get or create conversation
        let conversation_id = ConversationId::from_participants(sender_id, recipient_id);
        self.get_or_create_conversation(sender_id, recipient_id)
            .await?;

        // Add message to conversation
        {
            let mut conversations = self.conversations.write().await;
            if let Some(conversation) = conversations.get_mut(&conversation_id) {
                conversation.add_message(message.clone());
            }
        }

        // Create protocol message
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let protocol_message =
            ProtocolMessage::new_direct_message(recipient_id, content, message_id, timestamp);

        // Create message route
        let route = MessageRoute::direct_message(recipient_id);
        let envelope = MessageEnvelope::new(protocol_message, route);

        // Track pending message
        {
            let mut pending = self.pending_messages.write().await;
            pending.insert(message_id, envelope.clone());
        }

        // Send via connection manager
        let serialized = protocol_utils::serialize_message(&envelope.message)?;
        let mut connection_manager = self.connection_manager.lock().await;
        connection_manager
            .send_message(serialized)
            .await
            .map_err(DirectMessageError::TransportError)?;

        // Notify observers
        self.notify_observers(DirectMessageEvent::MessageReceived {
            conversation_id,
            message,
        })
        .await;

        Ok(message_id)
    }

    /// Send a direct message with file attachment
    pub async fn send_direct_message_with_file(
        &self,
        recipient_id: UserId,
        content: String,
        attachment: crate::client::messages::FileAttachment,
    ) -> DirectMessageResult<MessageId> {
        let sender_id = self
            .current_user_id
            .ok_or(DirectMessageError::ConnectionNotAvailable)?;

        // Create direct message with file attachment
        let message = DirectMessage::new_file(sender_id, recipient_id, content.clone(), attachment);
        let message_id = message.id;

        // Get or create conversation
        let conversation_id = ConversationId::from_participants(sender_id, recipient_id);
        self.get_or_create_conversation(sender_id, recipient_id)
            .await?;

        // Add message to conversation
        {
            let mut conversations = self.conversations.write().await;
            if let Some(conversation) = conversations.get_mut(&conversation_id) {
                conversation.add_message(message.clone());
            }
        }

        // Create protocol message with attachment URLs
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let protocol_message = ProtocolMessage::DirectMessage {
            recipient_id,
            content,
            message_id,
            timestamp,
            attachments: vec![message.attachments[0].url.clone()],
        };

        // Create message route
        let route = MessageRoute::direct_message(recipient_id);
        let envelope = MessageEnvelope::new(protocol_message, route);

        // Track pending message
        {
            let mut pending = self.pending_messages.write().await;
            pending.insert(message_id, envelope.clone());
        }

        // Send via connection manager
        let serialized = protocol_utils::serialize_message(&envelope.message)?;
        let mut connection_manager = self.connection_manager.lock().await;
        connection_manager
            .send_message(serialized)
            .await
            .map_err(DirectMessageError::TransportError)?;

        // Notify observers
        self.notify_observers(DirectMessageEvent::MessageReceived {
            conversation_id,
            message,
        })
        .await;

        Ok(message_id)
    }

    /// Get or create a conversation
    pub async fn get_or_create_conversation(
        &self,
        user1: UserId,
        user2: UserId,
    ) -> DirectMessageResult<ConversationId> {
        let conversation_id = ConversationId::from_participants(user1, user2);

        // Check if conversation already exists
        {
            let conversations = self.conversations.read().await;
            if conversations.contains_key(&conversation_id) {
                return Ok(conversation_id);
            }
        }

        // Create new conversation
        let conversation = DirectConversation::new(user1, user2);
        let id = conversation.id.clone();

        {
            let mut conversations = self.conversations.write().await;
            conversations.insert(conversation_id.clone(), conversation.clone());
        }

        // Notify observers
        self.notify_observers(DirectMessageEvent::ConversationCreated { conversation })
            .await;

        Ok(id)
    }

    /// Get conversation by ID
    pub async fn get_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> Option<DirectConversation> {
        let conversations = self.conversations.read().await;
        conversations.get(conversation_id).cloned()
    }

    /// Get all conversations for current user
    pub async fn get_user_conversations(&self) -> DirectMessageResult<Vec<ConversationSummary>> {
        let user_id = self
            .current_user_id
            .ok_or(DirectMessageError::ConnectionNotAvailable)?;
        let conversations = self.conversations.read().await;

        let mut summaries = Vec::new();
        for conversation in conversations.values() {
            if conversation.has_participant(user_id) {
                if let Some(summary) = ConversationSummary::from_conversation(conversation, user_id)
                {
                    summaries.push(summary);
                }
            }
        }

        // Sort by last activity (most recent first)
        summaries.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));

        Ok(summaries)
    }

    /// Mark messages as read in a conversation
    pub async fn mark_messages_read(
        &self,
        conversation_id: &ConversationId,
        up_to_message_id: Option<MessageId>,
    ) -> DirectMessageResult<()> {
        let user_id = self
            .current_user_id
            .ok_or(DirectMessageError::ConnectionNotAvailable)?;

        // Update conversation
        let message_ids = {
            let mut conversations = self.conversations.write().await;
            let conversation = conversations
                .get_mut(conversation_id)
                .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

            conversation.mark_messages_read(user_id, up_to_message_id);

            // Collect message IDs that were marked as read
            conversation
                .messages
                .iter()
                .filter(|msg| msg.recipient_id == user_id && msg.is_read())
                .map(|msg| msg.id)
                .collect::<Vec<_>>()
        };

        // Send read receipts
        for message_id in message_ids {
            let read_receipt = ProtocolMessage::new_read_receipt(
                message_id,
                user_id,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );

            let serialized = protocol_utils::serialize_message(&read_receipt)?;
            let mut connection_manager = self.connection_manager.lock().await;
            if let Err(e) = connection_manager.send_message(serialized).await {
                eprintln!("Failed to send read receipt: {}", e);
            }
        }

        Ok(())
    }

    /// Set typing indicator
    pub async fn set_typing_indicator(
        &self,
        recipient_id: UserId,
        is_typing: bool,
    ) -> DirectMessageResult<()> {
        let user_id = self
            .current_user_id
            .ok_or(DirectMessageError::ConnectionNotAvailable)?;

        // Update user manager
        self.user_manager
            .set_user_typing(user_id, if is_typing { Some(recipient_id) } else { None })
            .await;

        // Send typing indicator
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let typing_message =
            ProtocolMessage::new_typing_indicator(user_id, recipient_id, is_typing, timestamp);

        let serialized = protocol_utils::serialize_message(&typing_message)?;
        let mut connection_manager = self.connection_manager.lock().await;
        connection_manager
            .send_message(serialized)
            .await
            .map_err(DirectMessageError::TransportError)?;

        // Notify observers
        let conversation_id = ConversationId::from_participants(user_id, recipient_id);
        let event = if is_typing {
            DirectMessageEvent::TypingStarted {
                conversation_id,
                user_id,
            }
        } else {
            DirectMessageEvent::TypingStopped {
                conversation_id,
                user_id,
            }
        };
        self.notify_observers(event).await;

        Ok(())
    }

    /// Handle incoming protocol message
    pub async fn handle_protocol_message(
        &self,
        message: ProtocolMessage,
    ) -> DirectMessageResult<()> {
        match message {
            ProtocolMessage::DirectMessage {
                recipient_id,
                content,
                message_id,
                timestamp,
                attachments: _,
            } => {
                self.handle_incoming_direct_message(recipient_id, content, message_id, timestamp)
                    .await?;
            }
            ProtocolMessage::TypingIndicator {
                user_id,
                recipient_id,
                is_typing,
                timestamp: _,
            } => {
                self.handle_typing_indicator(user_id, recipient_id, is_typing)
                    .await?;
            }
            ProtocolMessage::ReadReceipt {
                message_id,
                reader_id,
                read_at,
            } => {
                self.handle_read_receipt(message_id, reader_id, read_at)
                    .await?;
            }
            ProtocolMessage::DeliveryConfirmation {
                message_id,
                delivered_to: _,
                delivered_at: _,
            } => {
                self.handle_delivery_confirmation(message_id).await?;
            }
            ProtocolMessage::PresenceUpdate { user_id, presence } => {
                self.handle_presence_update(user_id, presence).await?;
            }
            _ => {
                // Ignore other message types
            }
        }

        Ok(())
    }

    /// Handle incoming direct message
    async fn handle_incoming_direct_message(
        &self,
        sender_id: UserId,
        content: String,
        message_id: MessageId,
        timestamp: u64,
    ) -> DirectMessageResult<()> {
        let current_user_id = self
            .current_user_id
            .ok_or(DirectMessageError::ConnectionNotAvailable)?;

        // Create direct message
        let mut message = DirectMessage::new_text(sender_id, current_user_id, content);
        message.id = message_id;
        message.created_at = timestamp;
        message.mark_as_delivered();

        // Get or create conversation
        let conversation_id = self
            .get_or_create_conversation(sender_id, current_user_id)
            .await?;

        // Add message to conversation
        {
            let mut conversations = self.conversations.write().await;
            if let Some(conversation) = conversations.get_mut(&conversation_id) {
                conversation.add_message(message.clone());
            }
        }

        // Notify observers
        self.notify_observers(DirectMessageEvent::MessageReceived {
            conversation_id,
            message,
        })
        .await;

        Ok(())
    }

    /// Handle typing indicator
    async fn handle_typing_indicator(
        &self,
        user_id: UserId,
        recipient_id: UserId,
        is_typing: bool,
    ) -> DirectMessageResult<()> {
        // Update user manager
        self.user_manager
            .set_user_typing(user_id, if is_typing { Some(recipient_id) } else { None })
            .await;

        // Notify observers
        let conversation_id = ConversationId::from_participants(user_id, recipient_id);
        let event = if is_typing {
            DirectMessageEvent::TypingStarted {
                conversation_id,
                user_id,
            }
        } else {
            DirectMessageEvent::TypingStopped {
                conversation_id,
                user_id,
            }
        };
        self.notify_observers(event).await;

        Ok(())
    }

    /// Handle read receipt
    async fn handle_read_receipt(
        &self,
        message_id: MessageId,
        reader_id: UserId,
        read_at: u64,
    ) -> DirectMessageResult<()> {
        // Update message status in conversations
        {
            let mut conversations = self.conversations.write().await;
            for conversation in conversations.values_mut() {
                for message in &mut conversation.messages {
                    if message.id == message_id && message.recipient_id == reader_id {
                        message.read_at = Some(read_at);
                        message.delivery_status = MessageDeliveryStatus::Read;
                        break;
                    }
                }
            }
        }

        // Remove from pending messages
        {
            let mut pending = self.pending_messages.write().await;
            pending.remove(&message_id);
        }

        // Notify observers
        self.notify_observers(DirectMessageEvent::ReadReceiptReceived {
            message_id,
            reader_id,
            read_at,
        })
        .await;

        Ok(())
    }

    /// Handle delivery confirmation
    async fn handle_delivery_confirmation(&self, message_id: MessageId) -> DirectMessageResult<()> {
        // Update message status
        {
            let mut conversations = self.conversations.write().await;
            for conversation in conversations.values_mut() {
                for message in &mut conversation.messages {
                    if message.id == message_id {
                        message.mark_as_delivered();
                        break;
                    }
                }
            }
        }

        // Notify observers
        self.notify_observers(DirectMessageEvent::MessageStatusUpdated {
            message_id,
            status: MessageDeliveryStatus::Delivered,
        })
        .await;

        Ok(())
    }

    /// Handle presence update
    async fn handle_presence_update(
        &self,
        user_id: UserId,
        presence: UserPresence,
    ) -> DirectMessageResult<()> {
        // Update user manager
        self.user_manager
            .update_user_presence(presence.clone())
            .await;

        // Notify observers
        self.notify_observers(DirectMessageEvent::PresenceUpdated { user_id, presence })
            .await;

        Ok(())
    }

    /// Notify all observers of an event
    async fn notify_observers(&self, event: DirectMessageEvent) {
        let observers = self.observers.read().await;
        for observer in observers.iter() {
            observer.on_dm_event(event.clone());
        }
    }

    /// Request user list from server
    pub async fn request_user_list(&self) -> DirectMessageResult<()> {
        let message = ProtocolMessage::new_user_list_request();
        let serialized = protocol_utils::serialize_message(&message)?;

        let mut connection_manager = self.connection_manager.lock().await;
        connection_manager
            .send_message(serialized)
            .await
            .map_err(DirectMessageError::TransportError)?;

        Ok(())
    }

    /// Get filtered list of users available for messaging
    pub async fn get_available_users(
        &self,
        exclude_current: bool,
    ) -> DirectMessageResult<Vec<UserPresence>> {
        let mut users = self.user_manager.get_available_users().await;

        // Exclude current user if requested
        if exclude_current {
            if let Some(current_user_id) = self.current_user_id {
                users.retain(|user| user.user_id != current_user_id);
            }
        }

        Ok(users)
    }

    /// Search for users by username or display name
    pub async fn search_users(&self, query: &str) -> DirectMessageResult<Vec<UserPresence>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let users = self.user_manager.search_users(query.to_string()).await;

        // Exclude current user from search results
        let filtered_users = if let Some(current_user_id) = self.current_user_id {
            users
                .into_iter()
                .filter(|user| user.user_id != current_user_id)
                .collect()
        } else {
            users
        };

        Ok(filtered_users)
    }

    /// Get user presence by ID
    pub async fn get_user_presence(&self, user_id: UserId) -> DirectMessageResult<UserPresence> {
        self.user_manager
            .get_user_presence(user_id)
            .await
            .ok_or(DirectMessageError::UserNotFound(user_id))
    }

    /// Check if a user is currently online
    pub async fn is_user_online(&self, user_id: UserId) -> bool {
        self.user_manager.is_user_online(user_id).await
    }

    /// Get users currently typing to current user
    pub async fn get_users_typing_to_me(&self) -> DirectMessageResult<Vec<UserPresence>> {
        let current_user_id = self
            .current_user_id
            .ok_or(DirectMessageError::ConnectionNotAvailable)?;

        let typing_users = self.user_manager.get_users_typing_to(current_user_id).await;
        Ok(typing_users)
    }

    /// Request conversation list from server
    pub async fn request_conversation_list(&self) -> DirectMessageResult<()> {
        let message = ProtocolMessage::new_conversation_list_request();
        let serialized = protocol_utils::serialize_message(&message)?;

        let mut connection_manager = self.connection_manager.lock().await;
        connection_manager
            .send_message(serialized)
            .await
            .map_err(DirectMessageError::TransportError)?;

        Ok(())
    }

    /// Get conversation message history
    pub async fn get_conversation_messages(
        &self,
        conversation_id: &ConversationId,
        limit: Option<usize>,
    ) -> DirectMessageResult<Vec<DirectMessage>> {
        let conversations = self.conversations.read().await;
        let conversation = conversations
            .get(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        let messages = if let Some(limit) = limit {
            conversation
                .recent_messages(limit)
                .into_iter()
                .cloned()
                .collect()
        } else {
            conversation.messages.clone()
        };

        Ok(messages)
    }

    /// Get messages after a specific timestamp
    pub async fn get_messages_after(
        &self,
        conversation_id: &ConversationId,
        timestamp: u64,
    ) -> DirectMessageResult<Vec<DirectMessage>> {
        let conversations = self.conversations.read().await;
        let conversation = conversations
            .get(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        let messages = conversation
            .messages_after(timestamp)
            .into_iter()
            .cloned()
            .collect();

        Ok(messages)
    }

    /// Search messages within a conversation
    pub async fn search_conversation_messages(
        &self,
        conversation_id: &ConversationId,
        query: &str,
        limit: Option<usize>,
    ) -> DirectMessageResult<Vec<DirectMessage>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let conversations = self.conversations.read().await;
        let conversation = conversations
            .get(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        let query_lower = query.to_lowercase();
        let mut matching_messages: Vec<DirectMessage> = conversation
            .messages
            .iter()
            .filter(|msg| !msg.is_deleted() && msg.content.to_lowercase().contains(&query_lower))
            .cloned()
            .collect();

        // Sort by timestamp (newest first)
        matching_messages.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply limit if specified
        if let Some(limit) = limit {
            matching_messages.truncate(limit);
        }

        Ok(matching_messages)
    }

    /// Get unread message count for current user across all conversations
    pub async fn get_total_unread_count(&self) -> DirectMessageResult<u32> {
        let current_user_id = self
            .current_user_id
            .ok_or(DirectMessageError::ConnectionNotAvailable)?;

        let conversations = self.conversations.read().await;
        let total_unread = conversations
            .values()
            .filter(|conv| conv.has_participant(current_user_id))
            .map(|conv| conv.unread_count_for_user(current_user_id))
            .sum();

        Ok(total_unread)
    }

    /// Get recent conversations sorted by last activity
    pub async fn get_recent_conversations(
        &self,
        limit: Option<usize>,
    ) -> DirectMessageResult<Vec<ConversationSummary>> {
        let mut summaries = self.get_user_conversations().await?;

        // Sort by last activity (most recent first)
        summaries.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));

        // Apply limit if specified
        if let Some(limit) = limit {
            summaries.truncate(limit);
        }

        Ok(summaries)
    }

    /// Archive a conversation
    pub async fn archive_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> DirectMessageResult<()> {
        let mut conversations = self.conversations.write().await;
        let conversation = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        conversation.archive();
        Ok(())
    }

    /// Unarchive a conversation
    pub async fn unarchive_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> DirectMessageResult<()> {
        let mut conversations = self.conversations.write().await;
        let conversation = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        conversation.unarchive();
        Ok(())
    }

    /// Mute a conversation
    pub async fn mute_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> DirectMessageResult<()> {
        let mut conversations = self.conversations.write().await;
        let conversation = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        conversation.mute();
        Ok(())
    }

    /// Unmute a conversation
    pub async fn unmute_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> DirectMessageResult<()> {
        let mut conversations = self.conversations.write().await;
        let conversation = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        conversation.unmute();
        Ok(())
    }

    /// Delete a message (mark as deleted)
    pub async fn delete_message(
        &self,
        conversation_id: &ConversationId,
        message_id: MessageId,
    ) -> DirectMessageResult<()> {
        let current_user_id = self
            .current_user_id
            .ok_or(DirectMessageError::ConnectionNotAvailable)?;

        let mut conversations = self.conversations.write().await;
        let conversation = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        // Find and delete the message
        let message = conversation
            .messages
            .iter_mut()
            .find(|msg| msg.id == message_id)
            .ok_or(DirectMessageError::MessageNotFound(message_id))?;

        // Only allow sender to delete their own messages
        if message.sender_id != current_user_id {
            return Err(DirectMessageError::PermissionDenied(
                "Can only delete your own messages".to_string(),
            ));
        }

        message.delete();

        // Notify observers
        self.notify_observers(DirectMessageEvent::MessageStatusUpdated {
            message_id,
            status: MessageDeliveryStatus::Deleted,
        })
        .await;

        Ok(())
    }

    /// Edit a message
    pub async fn edit_message(
        &self,
        conversation_id: &ConversationId,
        message_id: MessageId,
        new_content: String,
    ) -> DirectMessageResult<()> {
        let current_user_id = self
            .current_user_id
            .ok_or(DirectMessageError::ConnectionNotAvailable)?;

        let mut conversations = self.conversations.write().await;
        let conversation = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        // Find and edit the message
        let message = conversation
            .messages
            .iter_mut()
            .find(|msg| msg.id == message_id)
            .ok_or(DirectMessageError::MessageNotFound(message_id))?;

        // Only allow sender to edit their own messages
        if message.sender_id != current_user_id {
            return Err(DirectMessageError::PermissionDenied(
                "Can only edit your own messages".to_string(),
            ));
        }

        // Don't allow editing deleted messages
        if message.is_deleted() {
            return Err(DirectMessageError::InvalidMessage(
                "Cannot edit deleted message".to_string(),
            ));
        }

        message.edit(new_content);

        // Notify observers
        self.notify_observers(DirectMessageEvent::MessageStatusUpdated {
            message_id,
            status: MessageDeliveryStatus::Edited,
        })
        .await;

        Ok(())
    }

    /// Send file attachment to conversation
    pub async fn send_file_attachment(
        &self,
        conversation_id: &ConversationId,
        file_path: &str,
        description: Option<String>,
    ) -> DirectMessageResult<MessageId> {
        // Get conversation to find recipient
        let conversation = self
            .get_conversation(conversation_id)
            .await
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        let current_user_id = self
            .current_user_id
            .ok_or(DirectMessageError::ConnectionNotAvailable)?;

        let recipient_id = conversation.other_participant(current_user_id).ok_or(
            DirectMessageError::PermissionDenied(
                "Not a participant in this conversation".to_string(),
            ),
        )?;

        // Create file attachment from file path
        let file_attachment = self.create_file_attachment(file_path).await?;
        let content =
            description.unwrap_or_else(|| format!("Sent a file: {}", file_attachment.filename));

        self.send_direct_message_with_file(recipient_id, content, file_attachment)
            .await
    }

    /// Create file attachment from file path
    async fn create_file_attachment(
        &self,
        file_path: &str,
    ) -> DirectMessageResult<crate::client::messages::FileAttachment> {
        use std::path::Path;
        use tokio::fs;

        let path = Path::new(file_path);
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| DirectMessageError::InvalidMessage("Invalid file path".to_string()))?
            .to_string();

        // Get file metadata
        let metadata = fs::metadata(file_path)
            .await
            .map_err(|e| DirectMessageError::InvalidMessage(format!("Cannot read file: {}", e)))?;

        let size = metadata.len();

        // Determine MIME type from file extension
        let mime_type = match path.extension().and_then(|ext| ext.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
            Some("png") => "image/png".to_string(),
            Some("gif") => "image/gif".to_string(),
            Some("pdf") => "application/pdf".to_string(),
            Some("txt") => "text/plain".to_string(),
            Some("mp4") => "video/mp4".to_string(),
            Some("mp3") => "audio/mpeg".to_string(),
            _ => "application/octet-stream".to_string(),
        };

        // For now, we'll use a placeholder URL - in a real implementation,
        // this would upload the file to a server and get back a URL
        let file_url = format!("file://{}", file_path);

        Ok(crate::client::messages::FileAttachment::new(
            filename, size, mime_type, file_url,
        ))
    }

    /// Get file attachments in a conversation
    pub async fn get_conversation_attachments(
        &self,
        conversation_id: &ConversationId,
    ) -> DirectMessageResult<Vec<crate::client::messages::FileAttachment>> {
        let conversations = self.conversations.read().await;
        let conversation = conversations
            .get(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        let attachments = conversation
            .messages
            .iter()
            .filter(|msg| !msg.is_deleted() && !msg.attachments.is_empty())
            .flat_map(|msg| msg.attachments.iter())
            .cloned()
            .collect();

        Ok(attachments)
    }

    /// Clean up expired pending messages
    pub async fn cleanup_expired_messages(&self) {
        let mut pending = self.pending_messages.write().await;
        pending.retain(|_, envelope| !envelope.is_expired());
    }

    /// Get statistics
    pub async fn get_stats(&self) -> DirectMessageStats {
        let conversations = self.conversations.read().await;
        let pending = self.pending_messages.read().await;

        let total_conversations = conversations.len();
        let total_messages = conversations.values().map(|c| c.message_count()).sum();
        let unread_conversations = conversations
            .values()
            .filter(|c| {
                if let Some(user_id) = self.current_user_id {
                    c.unread_count_for_user(user_id) > 0
                } else {
                    false
                }
            })
            .count();
        let pending_messages = pending.len();

        DirectMessageStats {
            total_conversations,
            total_messages,
            unread_conversations,
            pending_messages,
        }
    }

    /// Export conversation data for persistence
    pub async fn export_conversation_data(
        &self,
        conversation_id: &ConversationId,
    ) -> DirectMessageResult<serde_json::Value> {
        let conversations = self.conversations.read().await;
        let conversation = conversations
            .get(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        let export_data = serde_json::to_value(conversation)
            .map_err(|e| DirectMessageError::SerializationError(e))?;

        Ok(export_data)
    }

    /// Import conversation data from persistence
    pub async fn import_conversation_data(
        &self,
        conversation_data: serde_json::Value,
    ) -> DirectMessageResult<ConversationId> {
        let conversation: DirectConversation = serde_json::from_value(conversation_data)
            .map_err(|e| DirectMessageError::SerializationError(e))?;

        let conversation_id = conversation.id.clone();

        {
            let mut conversations = self.conversations.write().await;
            conversations.insert(conversation_id.clone(), conversation);
        }

        Ok(conversation_id)
    }

    /// Get conversation metadata
    pub async fn get_conversation_metadata(
        &self,
        conversation_id: &ConversationId,
    ) -> DirectMessageResult<HashMap<String, String>> {
        let conversations = self.conversations.read().await;
        let conversation = conversations
            .get(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        Ok(conversation.metadata.clone())
    }

    /// Set conversation metadata
    pub async fn set_conversation_metadata(
        &self,
        conversation_id: &ConversationId,
        key: String,
        value: String,
    ) -> DirectMessageResult<()> {
        let mut conversations = self.conversations.write().await;
        let conversation = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        conversation.set_metadata(key, value);
        Ok(())
    }

    /// Get message by ID across all conversations
    pub async fn find_message(
        &self,
        message_id: MessageId,
    ) -> Option<(ConversationId, DirectMessage)> {
        let conversations = self.conversations.read().await;

        for (conv_id, conversation) in conversations.iter() {
            if let Some(message) = conversation
                .messages
                .iter()
                .find(|msg| msg.id == message_id)
            {
                return Some((conv_id.clone(), message.clone()));
            }
        }

        None
    }

    /// Get conversation summary for display
    pub async fn get_conversation_summary(
        &self,
        conversation_id: &ConversationId,
    ) -> DirectMessageResult<ConversationSummary> {
        let current_user_id = self
            .current_user_id
            .ok_or(DirectMessageError::ConnectionNotAvailable)?;

        let conversations = self.conversations.read().await;
        let conversation = conversations
            .get(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        let summary = ConversationSummary::from_conversation(conversation, current_user_id)
            .ok_or_else(|| {
                DirectMessageError::PermissionDenied(
                    "Not a participant in this conversation".to_string(),
                )
            })?;

        Ok(summary)
    }

    /// Auto-cleanup old conversations based on criteria
    pub async fn cleanup_old_conversations(
        &self,
        days_threshold: u64,
    ) -> DirectMessageResult<usize> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let threshold_time = current_time.saturating_sub(days_threshold * 24 * 60 * 60);

        let mut conversations = self.conversations.write().await;
        let initial_count = conversations.len();

        // Remove conversations that are archived and older than threshold
        conversations.retain(|_, conv| !conv.is_archived || conv.last_activity > threshold_time);

        let removed_count = initial_count - conversations.len();
        Ok(removed_count)
    }

    /// Bulk mark multiple conversations as read
    pub async fn bulk_mark_conversations_read(
        &self,
        conversation_ids: Vec<ConversationId>,
    ) -> DirectMessageResult<()> {
        for conversation_id in conversation_ids {
            self.mark_messages_read(&conversation_id, None).await?;
        }
        Ok(())
    }

    /// Get conversation activity summary
    pub async fn get_conversation_activity(
        &self,
        conversation_id: &ConversationId,
        days: u32,
    ) -> DirectMessageResult<ConversationActivity> {
        let conversations = self.conversations.read().await;
        let conversation = conversations
            .get(conversation_id)
            .ok_or_else(|| DirectMessageError::ConversationNotFound(conversation_id.clone()))?;

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let threshold_time = current_time.saturating_sub(days as u64 * 24 * 60 * 60);

        let recent_messages: Vec<&DirectMessage> = conversation
            .messages
            .iter()
            .filter(|msg| msg.created_at > threshold_time && !msg.is_deleted())
            .collect();

        let message_count = recent_messages.len();
        let participants = conversation.participants.to_vec();

        // Count messages per participant
        let mut message_counts = HashMap::new();
        for message in &recent_messages {
            *message_counts.entry(message.sender_id).or_insert(0) += 1;
        }

        Ok(ConversationActivity {
            conversation_id: conversation_id.clone(),
            period_days: days,
            message_count,
            participants,
            message_counts,
            first_message_time: recent_messages.first().map(|msg| msg.created_at),
            last_message_time: recent_messages.last().map(|msg| msg.created_at),
        })
    }

    /// Send message with retry logic
    async fn send_with_retry(&self, message: String) -> DirectMessageResult<()> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < self.retry_config.max_attempts {
            match self.try_send_message(&message).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);
                    attempts += 1;

                    if attempts < self.retry_config.max_attempts {
                        let delay = self.retry_config.get_delay(attempts);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(DirectMessageError::ConnectionNotAvailable))
    }

    /// Try to send a message once
    async fn try_send_message(&self, message: &str) -> DirectMessageResult<()> {
        let mut connection_manager = self.connection_manager.lock().await;
        connection_manager
            .send_message(message.to_string())
            .await
            .map_err(DirectMessageError::TransportError)
    }

    /// Update retry configuration
    pub fn set_retry_config(&mut self, config: RetryConfig) {
        self.retry_config = config;
    }

    /// Get current rate limit status
    pub async fn get_rate_limit_status(&self) -> RateLimitStatus {
        let rate_limiter = self.rate_limiter.read().await;
        rate_limiter.get_status()
    }

    /// Reset rate limiter (admin function)
    pub async fn reset_rate_limiter(&self) {
        let mut rate_limiter = self.rate_limiter.write().await;
        rate_limiter.reset();
    }
}

impl ConnectionObserver for DirectMessageManager {
    fn on_message(&self, message: String) {
        // Parse and handle protocol messages
        if let Ok(protocol_message) = protocol_utils::deserialize_message(&message) {
            let manager = self.clone();
            tokio::spawn(async move {
                if let Err(e) = manager.handle_protocol_message(protocol_message).await {
                    eprintln!("Failed to handle protocol message: {}", e);
                }
            });
        }
    }

    fn on_error(&self, error: String) {
        eprintln!("Transport error in DirectMessageManager: {}", error);
    }

    fn on_status_change(&self, connected: bool) {
        if connected {
            // Request user list when connected
            let manager = self.clone();
            tokio::spawn(async move {
                if let Err(e) = manager.request_user_list().await {
                    eprintln!("Failed to request user list: {}", e);
                }
            });
        }
    }
}

// Need to implement Clone for use in async spawns
impl Clone for DirectMessageManager {
    fn clone(&self) -> Self {
        Self {
            conversations: Arc::clone(&self.conversations),
            user_manager: Arc::clone(&self.user_manager),
            connection_manager: Arc::clone(&self.connection_manager),
            observers: Arc::clone(&self.observers),
            current_user_id: self.current_user_id,
            pending_messages: Arc::clone(&self.pending_messages),
            typing_timeout: self.typing_timeout,
            rate_limiter: Arc::clone(&self.rate_limiter),
            retry_config: self.retry_config.clone(),
        }
    }
}

/// Direct message statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessageStats {
    pub total_conversations: usize,
    pub total_messages: usize,
    pub unread_conversations: usize,
    pub pending_messages: usize,
}

/// Conversation activity summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationActivity {
    pub conversation_id: ConversationId,
    pub period_days: u32,
    pub message_count: usize,
    pub participants: Vec<UserId>,
    pub message_counts: HashMap<UserId, usize>,
    pub first_message_time: Option<u64>,
    pub last_message_time: Option<u64>,
}

/// Rate limiter for controlling message sending frequency
#[derive(Debug)]
pub struct RateLimiter {
    max_requests: u32,
    window_duration: Duration,
    requests: Vec<Instant>,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(max_requests: u32, window_duration: Duration) -> Self {
        Self {
            max_requests,
            window_duration,
            requests: Vec::new(),
        }
    }

    /// Check if request is allowed under rate limit
    pub fn allow_request(&mut self) -> bool {
        let now = Instant::now();

        // Remove old requests outside the window
        self.requests
            .retain(|&request_time| now.duration_since(request_time) < self.window_duration);

        // Check if we're under the limit
        if self.requests.len() < self.max_requests as usize {
            self.requests.push(now);
            true
        } else {
            false
        }
    }

    /// Get current rate limit status
    pub fn get_status(&self) -> RateLimitStatus {
        let now = Instant::now();
        let recent_requests = self
            .requests
            .iter()
            .filter(|&&request_time| now.duration_since(request_time) < self.window_duration)
            .count();

        RateLimitStatus {
            requests_used: recent_requests as u32,
            max_requests: self.max_requests,
            window_duration: self.window_duration,
            time_until_reset: if recent_requests >= self.max_requests as usize {
                self.requests.first().map(|&first_request| {
                    self.window_duration
                        .saturating_sub(now.duration_since(first_request))
                })
            } else {
                None
            },
        }
    }

    /// Reset the rate limiter
    pub fn reset(&mut self) {
        self.requests.clear();
    }
}

/// Rate limit status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub requests_used: u32,
    pub max_requests: u32,
    pub window_duration: Duration,
    pub time_until_reset: Option<Duration>,
}

/// Retry configuration for failed operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Get delay for attempt number (1-based)
    pub fn get_delay(&self, attempt: u32) -> Duration {
        let delay_ms = (self.base_delay.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32 - 1)) as u64;

        Duration::from_millis(delay_ms.min(self.max_delay.as_millis() as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::Duration;

    struct TestObserver {
        event_count: Arc<AtomicUsize>,
    }

    impl TestObserver {
        fn new() -> (Self, Arc<AtomicUsize>) {
            let counter = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    event_count: Arc::clone(&counter),
                },
                counter,
            )
        }
    }

    impl DirectMessageObserver for TestObserver {
        fn on_dm_event(&self, _event: DirectMessageEvent) {
            self.event_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_dm_manager_creation() {
        let user_manager = Arc::new(UserManager::new());
        let config = crate::transport::ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let connection_manager = Arc::new(Mutex::new(ConnectionManager::new(config)));

        let mut dm_manager = DirectMessageManager::new(user_manager, connection_manager);
        let user_id = uuid::Uuid::new_v4();
        dm_manager.set_current_user(user_id);

        assert_eq!(dm_manager.current_user_id, Some(user_id));
    }

    #[tokio::test]
    async fn test_conversation_creation() {
        let user_manager = Arc::new(UserManager::new());
        let config = crate::transport::ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let connection_manager = Arc::new(Mutex::new(ConnectionManager::new(config)));

        let dm_manager = DirectMessageManager::new(user_manager, connection_manager);
        let user1 = uuid::Uuid::new_v4();
        let user2 = uuid::Uuid::new_v4();

        let conversation_id = dm_manager
            .get_or_create_conversation(user1, user2)
            .await
            .unwrap();
        let conversation = dm_manager.get_conversation(&conversation_id).await;

        assert!(conversation.is_some());
        let conv = conversation.unwrap();
        assert!(conv.has_participant(user1));
        assert!(conv.has_participant(user2));
    }

    #[tokio::test]
    async fn test_observer_notifications() {
        let user_manager = Arc::new(UserManager::new());
        let config = crate::transport::ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let connection_manager = Arc::new(Mutex::new(ConnectionManager::new(config)));

        let dm_manager = DirectMessageManager::new(user_manager, connection_manager);
        let (observer, counter) = TestObserver::new();

        dm_manager.add_observer(Arc::new(observer)).await;

        // Simulate an event
        let event = DirectMessageEvent::PresenceUpdated {
            user_id: uuid::Uuid::new_v4(),
            presence: UserPresence::new(uuid::Uuid::new_v4(), "test".to_string()),
        };

        dm_manager.notify_observers(event).await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_user_discovery() {
        let user_manager = Arc::new(UserManager::new());
        let config = crate::transport::ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let connection_manager = Arc::new(Mutex::new(ConnectionManager::new(config)));

        let mut dm_manager = DirectMessageManager::new(user_manager.clone(), connection_manager);
        let current_user = uuid::Uuid::new_v4();
        dm_manager.set_current_user(current_user);

        // Add test users to user manager
        let user1 = UserPresence::new(uuid::Uuid::new_v4(), "alice".to_string());
        let user2 = UserPresence::new(uuid::Uuid::new_v4(), "bob".to_string());
        let current_user_presence = UserPresence::new(current_user, "current".to_string());

        user_manager.update_user_presence(user1.clone()).await;
        user_manager.update_user_presence(user2.clone()).await;
        user_manager
            .update_user_presence(current_user_presence)
            .await;

        // Test getting available users excluding current user
        let available_users = dm_manager.get_available_users(true).await.unwrap();
        assert_eq!(available_users.len(), 2);
        assert!(!available_users.iter().any(|u| u.user_id == current_user));

        // Test user search
        let search_results = dm_manager.search_users("ali").await.unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].username, "alice");
    }

    #[tokio::test]
    async fn test_conversation_management() {
        let user_manager = Arc::new(UserManager::new());
        let config = crate::transport::ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let connection_manager = Arc::new(Mutex::new(ConnectionManager::new(config)));

        let mut dm_manager = DirectMessageManager::new(user_manager, connection_manager);
        let user1 = uuid::Uuid::new_v4();
        let user2 = uuid::Uuid::new_v4();
        dm_manager.set_current_user(user1);

        // Create conversation
        let conversation_id = dm_manager
            .get_or_create_conversation(user1, user2)
            .await
            .unwrap();

        // Test archiving
        dm_manager
            .archive_conversation(&conversation_id)
            .await
            .unwrap();
        let conversation = dm_manager.get_conversation(&conversation_id).await.unwrap();
        assert!(conversation.is_archived);

        // Test unarchiving
        dm_manager
            .unarchive_conversation(&conversation_id)
            .await
            .unwrap();
        let conversation = dm_manager.get_conversation(&conversation_id).await.unwrap();
        assert!(!conversation.is_archived);

        // Test muting
        dm_manager
            .mute_conversation(&conversation_id)
            .await
            .unwrap();
        let conversation = dm_manager.get_conversation(&conversation_id).await.unwrap();
        assert!(conversation.is_muted);

        // Test metadata
        dm_manager
            .set_conversation_metadata(&conversation_id, "theme".to_string(), "dark".to_string())
            .await
            .unwrap();
        let metadata = dm_manager
            .get_conversation_metadata(&conversation_id)
            .await
            .unwrap();
        assert_eq!(metadata.get("theme"), Some(&"dark".to_string()));
    }

    #[tokio::test]
    async fn test_message_search() {
        let user_manager = Arc::new(UserManager::new());
        let config = crate::transport::ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let connection_manager = Arc::new(Mutex::new(ConnectionManager::new(config)));

        let mut dm_manager = DirectMessageManager::new(user_manager, connection_manager);
        let user1 = uuid::Uuid::new_v4();
        let user2 = uuid::Uuid::new_v4();
        dm_manager.set_current_user(user1);

        // Create conversation and add messages
        let conversation_id = dm_manager
            .get_or_create_conversation(user1, user2)
            .await
            .unwrap();

        // Add test messages directly to conversation
        {
            let mut conversations = dm_manager.conversations.write().await;
            let conversation = conversations.get_mut(&conversation_id).unwrap();

            let msg1 = DirectMessage::new_text(user1, user2, "Hello world".to_string());
            let msg2 = DirectMessage::new_text(user2, user1, "How are you?".to_string());
            let msg3 = DirectMessage::new_text(user1, user2, "I'm doing great!".to_string());

            conversation.add_message(msg1);
            conversation.add_message(msg2);
            conversation.add_message(msg3);
        }

        // Test message search
        let search_results = dm_manager
            .search_conversation_messages(&conversation_id, "hello", None)
            .await
            .unwrap();
        assert_eq!(search_results.len(), 1);
        assert!(search_results[0].content.to_lowercase().contains("hello"));

        let search_results = dm_manager
            .search_conversation_messages(&conversation_id, "doing", Some(1))
            .await
            .unwrap();
        assert_eq!(search_results.len(), 1);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let mut rate_limiter = RateLimiter::new(2, Duration::from_secs(1));

        // First two requests should be allowed
        assert!(rate_limiter.allow_request());
        assert!(rate_limiter.allow_request());

        // Third request should be denied
        assert!(!rate_limiter.allow_request());

        // Check status
        let status = rate_limiter.get_status();
        assert_eq!(status.requests_used, 2);
        assert_eq!(status.max_requests, 2);
        assert!(status.time_until_reset.is_some());

        // Reset and try again
        rate_limiter.reset();
        assert!(rate_limiter.allow_request());
    }

    #[tokio::test]
    async fn test_retry_config() {
        let config = RetryConfig::default();

        // Test delay calculation
        let delay1 = config.get_delay(1);
        let delay2 = config.get_delay(2);
        let delay3 = config.get_delay(3);

        assert!(delay2 > delay1);
        assert!(delay3 > delay2);
        assert!(delay3 <= config.max_delay);
    }

    #[tokio::test]
    async fn test_conversation_activity() {
        let user_manager = Arc::new(UserManager::new());
        let config = crate::transport::ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let connection_manager = Arc::new(Mutex::new(ConnectionManager::new(config)));

        let mut dm_manager = DirectMessageManager::new(user_manager, connection_manager);
        let user1 = uuid::Uuid::new_v4();
        let user2 = uuid::Uuid::new_v4();
        dm_manager.set_current_user(user1);

        // Create conversation with messages
        let conversation_id = dm_manager
            .get_or_create_conversation(user1, user2)
            .await
            .unwrap();

        {
            let mut conversations = dm_manager.conversations.write().await;
            let conversation = conversations.get_mut(&conversation_id).unwrap();

            let msg1 = DirectMessage::new_text(user1, user2, "Message 1".to_string());
            let msg2 = DirectMessage::new_text(user2, user1, "Message 2".to_string());

            conversation.add_message(msg1);
            conversation.add_message(msg2);
        }

        // Test activity summary
        let activity = dm_manager
            .get_conversation_activity(&conversation_id, 7)
            .await
            .unwrap();

        assert_eq!(activity.message_count, 2);
        assert_eq!(activity.participants.len(), 2);
        assert_eq!(activity.period_days, 7);
        assert!(activity.message_counts.get(&user1).unwrap_or(&0) > &0);
        assert!(activity.message_counts.get(&user2).unwrap_or(&0) > &0);
    }
}
