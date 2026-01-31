//! Message domain types.
//!
//! See [DOMAIN_MODEL.md](../../../../docs/architecture/DOMAIN_MODEL.md) for full specification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

use super::{RoomId, UserId, ValidationError};

// ============================================================================
// MessageId
// ============================================================================

/// Unique identifier for a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageId(Uuid);

impl MessageId {
    /// Create a new random MessageId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a MessageId from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse a MessageId from a string.
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|_| ValidationError::InvalidFormat {
                reason: "invalid UUID format".into(),
            })
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for MessageId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for MessageId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

// ============================================================================
// MessageContent
// ============================================================================

/// Validated message content.
///
/// # Rules
/// - 1-4096 characters
/// - Cannot be only whitespace
/// - Preserves internal whitespace, validates trimmed content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MessageContent(String);

impl MessageContent {
    /// Maximum message content length.
    pub const MAX_LENGTH: usize = 4096;

    /// Create new message content with validation.
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();

        // Check if content is only whitespace
        if s.trim().is_empty() {
            return Err(ValidationError::Empty);
        }

        if s.len() > Self::MAX_LENGTH {
            return Err(ValidationError::TooLong {
                max: Self::MAX_LENGTH,
                actual: s.len(),
            });
        }

        // Preserve original content (including internal whitespace)
        Ok(Self(s))
    }

    /// Create content without validation (use only for data from trusted sources).
    pub fn new_unchecked(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the content as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the length of the content.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the content is empty (should never be true after validation).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for MessageContent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for MessageContent {
    type Error = ValidationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<MessageContent> for String {
    fn from(content: MessageContent) -> Self {
        content.0
    }
}

impl AsRef<str> for MessageContent {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// MessageTarget
// ============================================================================

/// Where a message is sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageTarget {
    /// Message to a room.
    Room { room_id: RoomId },
    /// Direct message to a user.
    #[serde(rename = "dm")]
    DirectMessage { recipient: UserId },
}

impl MessageTarget {
    /// Create a room message target.
    pub fn room(room_id: RoomId) -> Self {
        Self::Room { room_id }
    }

    /// Create a direct message target.
    pub fn dm(recipient: UserId) -> Self {
        Self::DirectMessage { recipient }
    }

    /// Check if this is a room message.
    pub fn is_room(&self) -> bool {
        matches!(self, Self::Room { .. })
    }

    /// Check if this is a direct message.
    pub fn is_dm(&self) -> bool {
        matches!(self, Self::DirectMessage { .. })
    }

    /// Get the room ID if this is a room message.
    pub fn room_id(&self) -> Option<RoomId> {
        match self {
            Self::Room { room_id } => Some(*room_id),
            Self::DirectMessage { .. } => None,
        }
    }

    /// Get the recipient if this is a direct message.
    pub fn recipient(&self) -> Option<UserId> {
        match self {
            Self::DirectMessage { recipient } => Some(*recipient),
            Self::Room { .. } => None,
        }
    }
}

impl Display for MessageTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Room { room_id } => write!(f, "room:{}", room_id),
            Self::DirectMessage { recipient } => write!(f, "dm:{}", recipient),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier.
    pub id: MessageId,
    /// Who sent the message.
    pub author: UserId,
    /// Where the message was sent.
    pub target: MessageTarget,
    /// Message content.
    pub content: MessageContent,
    /// Whether the message has been edited.
    #[serde(rename = "edited")]
    pub is_edited: bool,
    /// When the message was sent.
    pub created_at: DateTime<Utc>,
}

impl Message {
    /// Create a new message.
    pub fn new(author: UserId, target: MessageTarget, content: MessageContent) -> Self {
        Self {
            id: MessageId::new(),
            author,
            target,
            content,
            is_edited: false,
            created_at: Utc::now(),
        }
    }

    /// Create a new room message.
    pub fn to_room(author: UserId, room_id: RoomId, content: MessageContent) -> Self {
        Self::new(author, MessageTarget::room(room_id), content)
    }

    /// Create a new direct message.
    pub fn to_user(author: UserId, recipient: UserId, content: MessageContent) -> Self {
        Self::new(author, MessageTarget::dm(recipient), content)
    }

    /// Check if this is a room message.
    pub fn is_room_message(&self) -> bool {
        self.target.is_room()
    }

    /// Check if this is a direct message.
    pub fn is_dm(&self) -> bool {
        self.target.is_dm()
    }

    /// Get the room ID if this is a room message.
    pub fn room_id(&self) -> Option<RoomId> {
        self.target.room_id()
    }

    /// Get the recipient if this is a direct message.
    pub fn recipient(&self) -> Option<UserId> {
        self.target.recipient()
    }

    /// Edit the message content.
    pub fn edit(&mut self, new_content: MessageContent) {
        self.content = new_content;
        self.is_edited = true;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_id() {
        let id1 = MessageId::new();
        let id2 = MessageId::new();
        assert_ne!(id1, id2);

        let parsed = MessageId::parse(&id1.to_string()).unwrap();
        assert_eq!(id1, parsed);

        assert!(MessageId::parse("not-a-uuid").is_err());
    }

    #[test]
    fn test_message_content_valid() {
        assert!(MessageContent::new("Hello").is_ok());
        assert!(MessageContent::new("a").is_ok()); // minimum
        assert!(MessageContent::new("a".repeat(4096)).is_ok()); // maximum
        assert!(MessageContent::new("  has spaces  ").is_ok()); // preserves spaces
    }

    #[test]
    fn test_message_content_invalid() {
        // Empty
        assert!(MessageContent::new("").is_err());

        // Only whitespace
        assert!(MessageContent::new("   ").is_err());
        assert!(MessageContent::new("\n\t").is_err());

        // Too long
        assert!(MessageContent::new("a".repeat(4097)).is_err());
    }

    #[test]
    fn test_message_content_preserves_internal_whitespace() {
        let content = MessageContent::new("Hello   World").unwrap();
        assert_eq!(content.as_str(), "Hello   World");
    }

    #[test]
    fn test_message_target_room() {
        let room_id = RoomId::new();
        let target = MessageTarget::room(room_id);

        assert!(target.is_room());
        assert!(!target.is_dm());
        assert_eq!(target.room_id(), Some(room_id));
        assert_eq!(target.recipient(), None);
    }

    #[test]
    fn test_message_target_dm() {
        let user_id = UserId::new();
        let target = MessageTarget::dm(user_id);

        assert!(!target.is_room());
        assert!(target.is_dm());
        assert_eq!(target.room_id(), None);
        assert_eq!(target.recipient(), Some(user_id));
    }

    #[test]
    fn test_message_creation_room() {
        let author = UserId::new();
        let room_id = RoomId::new();
        let content = MessageContent::new("Hello, room!").unwrap();
        let message = Message::to_room(author, room_id, content.clone());

        assert_eq!(message.author, author);
        assert!(message.is_room_message());
        assert_eq!(message.room_id(), Some(room_id));
        assert_eq!(message.content.as_str(), content.as_str());
        assert!(!message.is_edited);
    }

    #[test]
    fn test_message_creation_dm() {
        let author = UserId::new();
        let recipient = UserId::new();
        let content = MessageContent::new("Hello, friend!").unwrap();
        let message = Message::to_user(author, recipient, content);

        assert_eq!(message.author, author);
        assert!(message.is_dm());
        assert_eq!(message.recipient(), Some(recipient));
    }

    #[test]
    fn test_message_edit() {
        let author = UserId::new();
        let room_id = RoomId::new();
        let content = MessageContent::new("Original").unwrap();
        let mut message = Message::to_room(author, room_id, content);

        assert!(!message.is_edited);

        let new_content = MessageContent::new("Edited").unwrap();
        message.edit(new_content.clone());

        assert!(message.is_edited);
        assert_eq!(message.content.as_str(), new_content.as_str());
    }

    #[test]
    fn test_message_target_serialization() {
        let room_id = RoomId::new();
        let room_target = MessageTarget::room(room_id);
        let json = serde_json::to_string(&room_target).unwrap();
        assert!(json.contains("\"type\":\"room\""));

        let user_id = UserId::new();
        let dm_target = MessageTarget::dm(user_id);
        let json = serde_json::to_string(&dm_target).unwrap();
        assert!(json.contains("\"type\":\"dm\""));
    }

    #[test]
    fn test_message_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = MessageId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn test_message_id_default() {
        let id = MessageId::default();
        assert!(!id.as_uuid().is_nil());
    }

    #[test]
    fn test_message_id_display() {
        let id = MessageId::new();
        let display = format!("{}", id);
        assert!(!display.is_empty());
        assert!(MessageId::parse(&display).is_ok());
    }

    #[test]
    fn test_message_id_from_trait() {
        let uuid = Uuid::new_v4();
        let id: MessageId = uuid.into();
        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn test_message_content_display() {
        let content = MessageContent::new("Hello World").unwrap();
        assert_eq!(format!("{}", content), "Hello World");
    }

    #[test]
    fn test_message_content_as_str() {
        let content = MessageContent::new("Test content").unwrap();
        assert_eq!(content.as_str(), "Test content");
    }

    #[test]
    fn test_message_content_len() {
        let content = MessageContent::new("Hello").unwrap();
        assert_eq!(content.len(), 5);
    }

    #[test]
    fn test_message_content_is_empty() {
        let content = MessageContent::new("Not empty").unwrap();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_message_content_new_unchecked() {
        let content = MessageContent::new_unchecked("Unchecked");
        assert_eq!(content.as_str(), "Unchecked");
    }

    #[test]
    fn test_message_content_as_ref() {
        let content = MessageContent::new("Test").unwrap();
        let s: &str = content.as_ref();
        assert_eq!(s, "Test");
    }

    #[test]
    fn test_message_content_try_from() {
        let content: Result<MessageContent, _> = "Valid content".to_string().try_into();
        assert!(content.is_ok());

        let empty: Result<MessageContent, _> = "".to_string().try_into();
        assert!(empty.is_err());
    }

    #[test]
    fn test_message_content_into_string() {
        let content = MessageContent::new("Test").unwrap();
        let s: String = content.into();
        assert_eq!(s, "Test");
    }

    #[test]
    fn test_message_target_display() {
        let room_id = RoomId::new();
        let room_target = MessageTarget::room(room_id);
        let display = format!("{}", room_target);
        assert!(display.starts_with("room:"));

        let user_id = UserId::new();
        let dm_target = MessageTarget::dm(user_id);
        let display = format!("{}", dm_target);
        assert!(display.starts_with("dm:"));
    }

    #[test]
    fn test_message_new() {
        let author = UserId::new();
        let room_id = RoomId::new();
        let target = MessageTarget::room(room_id);
        let content = MessageContent::new("Hello").unwrap();
        let message = Message::new(author, target, content);

        assert_eq!(message.author, author);
        assert!(!message.is_edited);
    }

    #[test]
    fn test_message_serialization() {
        let author = UserId::new();
        let room_id = RoomId::new();
        let content = MessageContent::new("Test message").unwrap();
        let message = Message::to_room(author, room_id, content);

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"edited\":false"));

        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, message.id);
        assert_eq!(deserialized.content.as_str(), "Test message");
    }

    #[test]
    fn test_message_content_serialization() {
        let content = MessageContent::new("Hello").unwrap();
        let json = serde_json::to_string(&content).unwrap();
        assert_eq!(json, "\"Hello\"");

        let deserialized: MessageContent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.as_str(), "Hello");
    }

    #[test]
    fn test_message_target_deserialization() {
        let room_json = r#"{"type":"room","room_id":"00000000-0000-0000-0000-000000000001"}"#;
        let room_target: MessageTarget = serde_json::from_str(room_json).unwrap();
        assert!(room_target.is_room());

        let dm_json = r#"{"type":"dm","recipient":"00000000-0000-0000-0000-000000000002"}"#;
        let dm_target: MessageTarget = serde_json::from_str(dm_json).unwrap();
        assert!(dm_target.is_dm());
    }
}
