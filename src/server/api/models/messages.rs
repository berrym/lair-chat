//! Message-related API models
//!
//! This module contains all data structures related to message operations,
//! including message creation, editing, reactions, and search functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Message information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Message {
    /// Message ID
    pub id: Uuid,

    /// Room ID where message was sent
    pub room_id: Uuid,

    /// User ID who sent the message
    pub user_id: Uuid,

    /// Message content
    pub content: String,

    /// Message type
    pub message_type: MessageType,

    /// Parent message ID (for replies)
    pub parent_id: Option<Uuid>,

    /// Thread ID (for threaded conversations)
    pub thread_id: Option<Uuid>,

    /// Message creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: Option<DateTime<Utc>>,

    /// Whether message was edited
    pub is_edited: bool,

    /// Whether message is deleted
    pub is_deleted: bool,

    /// Message metadata
    pub metadata: serde_json::Value,
}

/// Message types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    /// Regular text message
    Text,
    /// Image message
    Image,
    /// File attachment
    File,
    /// System message
    System,
    /// Audio message
    Audio,
    /// Video message
    Video,
}

/// Send message request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct SendMessageRequest {
    /// Message content (1-4000 characters)
    #[validate(length(min = 1, max = 4000))]
    pub content: String,

    /// Message type
    pub message_type: MessageType,

    /// Parent message ID (for replies)
    pub parent_id: Option<Uuid>,

    /// Thread ID (for threaded conversations)
    pub thread_id: Option<Uuid>,
}

/// Edit message request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct EditMessageRequest {
    /// New message content
    #[validate(length(min = 1, max = 4000))]
    pub content: String,
}

/// Message reaction
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MessageReaction {
    /// Message ID
    pub message_id: Uuid,

    /// User ID who reacted
    pub user_id: Uuid,

    /// Emoji reaction
    pub emoji: String,

    /// Reaction timestamp
    pub created_at: DateTime<Utc>,
}

/// Add reaction request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct AddReactionRequest {
    /// Emoji reaction (1-50 characters)
    #[validate(length(min = 1, max = 50))]
    pub emoji: String,
}

/// Message search request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct MessageSearchRequest {
    /// Search query
    #[validate(length(min = 2, max = 200))]
    pub query: String,

    /// Room ID filter
    pub room_id: Option<Uuid>,

    /// User ID filter
    pub user_id: Option<Uuid>,

    /// Message type filter
    pub message_type: Option<MessageType>,

    /// Date range filter
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,

    /// Maximum results
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,
}

impl Default for MessageSearchRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            room_id: None,
            user_id: None,
            message_type: None,
            date_from: None,
            date_to: None,
            limit: Some(20),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_send_message_request_validation() {
        let valid_request = SendMessageRequest {
            content: "Hello world!".to_string(),
            message_type: MessageType::Text,
            parent_id: None,
            thread_id: None,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = SendMessageRequest {
            content: "".to_string(), // Empty content
            message_type: MessageType::Text,
            parent_id: None,
            thread_id: None,
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_message_search_request_validation() {
        let valid_request = MessageSearchRequest {
            query: "test".to_string(),
            room_id: Some(Uuid::new_v4()),
            user_id: None,
            message_type: Some(MessageType::Text),
            date_from: None,
            date_to: None,
            limit: Some(10),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = MessageSearchRequest {
            query: "a".to_string(), // Too short
            ..Default::default()
        };
        assert!(invalid_request.validate().is_err());
    }
}
