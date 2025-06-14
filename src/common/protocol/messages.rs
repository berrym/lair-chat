//! Protocol extensions for Lair-Chat direct messaging
//! Defines message types and protocol structures for DM functionality.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::chat::{ConversationId, DirectMessage, MessageId, MessageTarget, UserId, UserPresence};

/// Protocol message types for client-server communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolMessage {
    // Existing chat messages
    /// Regular chat message to all users
    ChatMessage {
        content: String,
        message_id: MessageId,
        timestamp: u64,
    },

    // Direct messaging extensions
    /// Direct message to specific user
    DirectMessage {
        recipient_id: UserId,
        content: String,
        message_id: MessageId,
        timestamp: u64,
        attachments: Vec<String>, // File attachment URLs
    },

    /// Request list of online users
    UserListRequest,

    /// Response with list of online users
    UserListResponse {
        users: Vec<UserPresence>,
        timestamp: u64,
    },

    /// User presence update notification
    PresenceUpdate {
        user_id: UserId,
        presence: UserPresence,
    },

    /// User went online notification
    UserOnline {
        user_id: UserId,
        username: String,
        timestamp: u64,
    },

    /// User went offline notification
    UserOffline {
        user_id: UserId,
        username: String,
        timestamp: u64,
    },

    /// Typing indicator message
    TypingIndicator {
        user_id: UserId,
        recipient_id: UserId,
        is_typing: bool,
        timestamp: u64,
    },

    /// Read receipt for direct message
    ReadReceipt {
        message_id: MessageId,
        reader_id: UserId,
        read_at: u64,
    },

    /// Message delivery confirmation
    DeliveryConfirmation {
        message_id: MessageId,
        delivered_to: UserId,
        delivered_at: u64,
    },

    /// Request conversation history
    ConversationHistoryRequest {
        conversation_id: ConversationId,
        before_timestamp: Option<u64>,
        limit: Option<u32>,
    },

    /// Response with conversation history
    ConversationHistoryResponse {
        conversation_id: ConversationId,
        messages: Vec<DirectMessage>,
        has_more: bool,
    },

    /// List conversations for user
    ConversationListRequest,

    /// Response with user's conversations
    ConversationListResponse {
        conversations: Vec<ConversationSummary>,
    },

    // Authentication and connection
    /// User login request
    LoginRequest { username: String, password: String },

    /// Login response
    LoginResponse {
        success: bool,
        user_id: Option<UserId>,
        token: Option<String>,
        error: Option<String>,
    },

    /// User registration request
    RegisterRequest {
        username: String,
        password: String,
        email: Option<String>,
    },

    /// Registration response
    RegisterResponse {
        success: bool,
        user_id: Option<UserId>,
        error: Option<String>,
    },

    /// Heartbeat/ping message
    Ping { timestamp: u64 },

    /// Heartbeat/pong response
    Pong { timestamp: u64 },

    // Error handling
    /// Generic error message
    Error {
        code: ErrorCode,
        message: String,
        context: Option<HashMap<String, String>>,
    },

    /// Acknowledgment message
    Ack {
        message_id: MessageId,
        status: AckStatus,
    },
}

/// Conversation summary for protocol responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub conversation_id: ConversationId,
    pub other_user_id: UserId,
    pub other_username: String,
    pub last_message_preview: Option<String>,
    pub last_activity: u64,
    pub unread_count: u32,
    pub is_muted: bool,
}

/// Error codes for protocol errors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCode {
    /// Authentication failed
    AuthenticationFailed,
    /// User not found
    UserNotFound,
    /// Message not found
    MessageNotFound,
    /// Conversation not found
    ConversationNotFound,
    /// Permission denied
    PermissionDenied,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Invalid message format
    InvalidMessage,
    /// Server error
    ServerError,
    /// Connection error
    ConnectionError,
    /// Timeout error
    TimeoutError,
    /// Unknown error
    Unknown,
}

/// Acknowledgment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AckStatus {
    /// Message received successfully
    Success,
    /// Message failed to process
    Failed,
    /// Message is being processed
    Processing,
}

impl ProtocolMessage {
    /// Get the message ID if available
    pub fn message_id(&self) -> Option<MessageId> {
        match self {
            ProtocolMessage::ChatMessage { message_id, .. } => Some(*message_id),
            ProtocolMessage::DirectMessage { message_id, .. } => Some(*message_id),
            ProtocolMessage::ReadReceipt { message_id, .. } => Some(*message_id),
            ProtocolMessage::DeliveryConfirmation { message_id, .. } => Some(*message_id),
            ProtocolMessage::Ack { message_id, .. } => Some(*message_id),
            _ => None,
        }
    }

    /// Get the user ID if available
    pub fn user_id(&self) -> Option<UserId> {
        match self {
            ProtocolMessage::PresenceUpdate { user_id, .. } => Some(*user_id),
            ProtocolMessage::UserOnline { user_id, .. } => Some(*user_id),
            ProtocolMessage::UserOffline { user_id, .. } => Some(*user_id),
            ProtocolMessage::TypingIndicator { user_id, .. } => Some(*user_id),
            ProtocolMessage::ReadReceipt { reader_id, .. } => Some(*reader_id),
            _ => None,
        }
    }

    /// Get timestamp if available
    pub fn timestamp(&self) -> Option<u64> {
        match self {
            ProtocolMessage::ChatMessage { timestamp, .. } => Some(*timestamp),
            ProtocolMessage::DirectMessage { timestamp, .. } => Some(*timestamp),
            ProtocolMessage::UserListResponse { timestamp, .. } => Some(*timestamp),
            ProtocolMessage::UserOnline { timestamp, .. } => Some(*timestamp),
            ProtocolMessage::UserOffline { timestamp, .. } => Some(*timestamp),
            ProtocolMessage::TypingIndicator { timestamp, .. } => Some(*timestamp),
            ProtocolMessage::Ping { timestamp } => Some(*timestamp),
            ProtocolMessage::Pong { timestamp } => Some(*timestamp),
            _ => None,
        }
    }

    /// Check if this is a direct message
    pub fn is_direct_message(&self) -> bool {
        matches!(self, ProtocolMessage::DirectMessage { .. })
    }

    /// Check if this is a user presence related message
    pub fn is_presence_message(&self) -> bool {
        matches!(
            self,
            ProtocolMessage::PresenceUpdate { .. }
                | ProtocolMessage::UserOnline { .. }
                | ProtocolMessage::UserOffline { .. }
                | ProtocolMessage::UserListResponse { .. }
        )
    }

    /// Check if this is a typing indicator
    pub fn is_typing_indicator(&self) -> bool {
        matches!(self, ProtocolMessage::TypingIndicator { .. })
    }

    /// Create a direct message
    pub fn new_direct_message(
        recipient_id: UserId,
        content: String,
        message_id: MessageId,
        timestamp: u64,
    ) -> Self {
        Self::DirectMessage {
            recipient_id,
            content,
            message_id,
            timestamp,
            attachments: Vec::new(),
        }
    }

    /// Create a typing indicator
    pub fn new_typing_indicator(
        user_id: UserId,
        recipient_id: UserId,
        is_typing: bool,
        timestamp: u64,
    ) -> Self {
        Self::TypingIndicator {
            user_id,
            recipient_id,
            is_typing,
            timestamp,
        }
    }

    /// Create a read receipt
    pub fn new_read_receipt(message_id: MessageId, reader_id: UserId, read_at: u64) -> Self {
        Self::ReadReceipt {
            message_id,
            reader_id,
            read_at,
        }
    }

    /// Create an error message
    pub fn new_error(code: ErrorCode, message: String) -> Self {
        Self::Error {
            code,
            message,
            context: None,
        }
    }

    /// Create an error message with context
    pub fn new_error_with_context(
        code: ErrorCode,
        message: String,
        context: HashMap<String, String>,
    ) -> Self {
        Self::Error {
            code,
            message,
            context: Some(context),
        }
    }

    /// Create a user list request
    pub fn new_user_list_request() -> Self {
        Self::UserListRequest
    }

    /// Create a conversation list request
    pub fn new_conversation_list_request() -> Self {
        Self::ConversationListRequest
    }

    /// Create a ping message
    pub fn new_ping(timestamp: u64) -> Self {
        Self::Ping { timestamp }
    }

    /// Create a pong message
    pub fn new_pong(timestamp: u64) -> Self {
        Self::Pong { timestamp }
    }
}

/// Message routing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRoute {
    /// Target for the message
    pub target: MessageTarget,
    /// Priority level
    pub priority: MessagePriority,
    /// Whether delivery confirmation is required
    pub requires_confirmation: bool,
    /// Maximum delivery attempts
    pub max_attempts: u32,
    /// Timeout for delivery (seconds)
    pub timeout_seconds: u64,
}

/// Message priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Urgent,
}

impl Default for MessagePriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl MessageRoute {
    /// Create a new broadcast route
    pub fn broadcast() -> Self {
        Self {
            target: MessageTarget::Broadcast,
            priority: MessagePriority::Normal,
            requires_confirmation: false,
            max_attempts: 3,
            timeout_seconds: 30,
        }
    }

    /// Create a new direct message route
    pub fn direct_message(recipient: UserId) -> Self {
        Self {
            target: MessageTarget::DirectMessage(recipient),
            priority: MessagePriority::Normal,
            requires_confirmation: true,
            max_attempts: 5,
            timeout_seconds: 60,
        }
    }

    /// Create a high priority direct message route
    pub fn urgent_direct_message(recipient: UserId) -> Self {
        Self {
            target: MessageTarget::DirectMessage(recipient),
            priority: MessagePriority::Urgent,
            requires_confirmation: true,
            max_attempts: 10,
            timeout_seconds: 30,
        }
    }

    /// Set message priority
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set confirmation requirement
    pub fn with_confirmation(mut self, requires_confirmation: bool) -> Self {
        self.requires_confirmation = requires_confirmation;
        self
    }

    /// Set maximum attempts
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }
}

/// Protocol serialization/deserialization utilities
pub mod protocol_utils {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Serialize a protocol message to JSON
    pub fn serialize_message(message: &ProtocolMessage) -> Result<String, serde_json::Error> {
        serde_json::to_string(message)
    }

    /// Deserialize a protocol message from JSON
    pub fn deserialize_message(json: &str) -> Result<ProtocolMessage, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Get current timestamp in seconds
    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Create a message envelope with routing information
    pub fn create_envelope(message: ProtocolMessage, route: MessageRoute) -> MessageEnvelope {
        MessageEnvelope {
            id: uuid::Uuid::new_v4(),
            message,
            route,
            created_at: current_timestamp(),
            attempts: 0,
        }
    }
}

/// Message envelope for routing and delivery tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Unique envelope ID
    pub id: MessageId,
    /// The actual message
    pub message: ProtocolMessage,
    /// Routing information
    pub route: MessageRoute,
    /// When envelope was created
    pub created_at: u64,
    /// Number of delivery attempts
    pub attempts: u32,
}

impl MessageEnvelope {
    /// Create a new message envelope
    pub fn new(message: ProtocolMessage, route: MessageRoute) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            message,
            route,
            created_at: protocol_utils::current_timestamp(),
            attempts: 0,
        }
    }

    /// Increment attempt counter
    pub fn increment_attempts(&mut self) {
        self.attempts += 1;
    }

    /// Check if max attempts exceeded
    pub fn is_max_attempts_exceeded(&self) -> bool {
        self.attempts >= self.route.max_attempts
    }

    /// Check if envelope has expired
    pub fn is_expired(&self) -> bool {
        let now = protocol_utils::current_timestamp();
        now.saturating_sub(self.created_at) > self.route.timeout_seconds
    }

    /// Get message priority
    pub fn priority(&self) -> MessagePriority {
        self.route.priority.clone()
    }

    /// Check if delivery confirmation is required
    pub fn requires_confirmation(&self) -> bool {
        self.route.requires_confirmation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_message_creation() {
        let user_id = uuid::Uuid::new_v4();
        let message_id = uuid::Uuid::new_v4();
        let timestamp = protocol_utils::current_timestamp();

        let dm = ProtocolMessage::new_direct_message(
            user_id,
            "Hello!".to_string(),
            message_id,
            timestamp,
        );

        assert!(dm.is_direct_message());
        assert_eq!(dm.message_id(), Some(message_id));
        assert_eq!(dm.timestamp(), Some(timestamp));
    }

    #[test]
    fn test_message_route_creation() {
        let user_id = uuid::Uuid::new_v4();
        let route = MessageRoute::direct_message(user_id);

        assert_eq!(route.target, MessageTarget::DirectMessage(user_id));
        assert_eq!(route.priority, MessagePriority::Normal);
        assert!(route.requires_confirmation);
    }

    #[test]
    fn test_message_envelope() {
        let user_id = uuid::Uuid::new_v4();
        let message = ProtocolMessage::new_direct_message(
            user_id,
            "Test".to_string(),
            uuid::Uuid::new_v4(),
            protocol_utils::current_timestamp(),
        );
        let route = MessageRoute::direct_message(user_id);

        let mut envelope = MessageEnvelope::new(message, route);

        assert_eq!(envelope.attempts, 0);
        assert!(!envelope.is_max_attempts_exceeded());

        envelope.increment_attempts();
        assert_eq!(envelope.attempts, 1);
    }

    #[test]
    fn test_protocol_serialization() {
        let message = ProtocolMessage::new_ping(protocol_utils::current_timestamp());

        let serialized = protocol_utils::serialize_message(&message).unwrap();
        let deserialized = protocol_utils::deserialize_message(&serialized).unwrap();

        assert!(matches!(deserialized, ProtocolMessage::Ping { .. }));
    }

    #[test]
    fn test_error_message_creation() {
        let error =
            ProtocolMessage::new_error(ErrorCode::UserNotFound, "User does not exist".to_string());

        if let ProtocolMessage::Error { code, message, .. } = error {
            assert_eq!(code, ErrorCode::UserNotFound);
            assert_eq!(message, "User does not exist");
        } else {
            panic!("Expected error message");
        }
    }
}
