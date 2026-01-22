//! Chat room management module for Lair-Chat
//! Provides comprehensive room management, user tracking, and message handling.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub mod direct_messages;
pub mod dm_conversations;
pub mod dm_manager;
pub mod messages;
pub mod rooms;
pub mod user_manager;
pub mod users;

pub use direct_messages::{
    ConversationId, ConversationSummary, DirectConversation, DirectMessage, MessageDeliveryStatus,
    MessageTarget,
};
pub use dm_conversations::{DMConversation, DMConversationManager, DMMessage};
pub use dm_manager::{
    DirectMessageError, DirectMessageEvent, DirectMessageManager, DirectMessageObserver,
    DirectMessageResult, DirectMessageStats,
};
pub use messages::{ChatMessage, MessageStatus, MessageType};
pub use rooms::{Room, RoomManager, RoomSettings, RoomType};
pub use user_manager::{UserFilter, UserManager, UserPresence, UserProfile, UserStats};
pub use users::{RoomUser, UserRole, UserStatus};

/// Chat room identifier
pub type RoomId = Uuid;

/// User identifier
pub type UserId = Uuid;

/// Message identifier
pub type MessageId = Uuid;

/// Helper function to create a new message ID
pub fn new_message_id() -> MessageId {
    Uuid::new_v4()
}

/// Helper function to create a new user ID
pub fn new_user_id() -> UserId {
    Uuid::new_v4()
}

/// Helper function to create a new room ID
pub fn new_room_id() -> RoomId {
    Uuid::new_v4()
}

/// Chat room event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomEvent {
    /// User joined the room
    UserJoined {
        user_id: UserId,
        username: String,
        timestamp: u64,
    },
    /// User left the room
    UserLeft {
        user_id: UserId,
        username: String,
        timestamp: u64,
    },
    /// User's role changed
    RoleChanged {
        user_id: UserId,
        username: String,
        old_role: UserRole,
        new_role: UserRole,
        changed_by: UserId,
        timestamp: u64,
    },
    /// Room settings changed
    SettingsChanged {
        changed_by: UserId,
        changes: Vec<String>,
        timestamp: u64,
    },
    /// Message sent
    MessageSent {
        message_id: MessageId,
        user_id: UserId,
        username: String,
        timestamp: u64,
    },
    /// Message edited
    MessageEdited {
        message_id: MessageId,
        user_id: UserId,
        timestamp: u64,
    },
    /// Message deleted
    MessageDeleted {
        message_id: MessageId,
        deleted_by: UserId,
        timestamp: u64,
    },
    /// User started typing
    UserStartedTyping {
        user_id: UserId,
        username: String,
        timestamp: u64,
    },
    /// User stopped typing
    UserStoppedTyping {
        user_id: UserId,
        username: String,
        timestamp: u64,
    },
}

impl RoomEvent {
    /// Get the timestamp of the event
    pub fn timestamp(&self) -> u64 {
        match self {
            RoomEvent::UserJoined { timestamp, .. }
            | RoomEvent::UserLeft { timestamp, .. }
            | RoomEvent::RoleChanged { timestamp, .. }
            | RoomEvent::SettingsChanged { timestamp, .. }
            | RoomEvent::MessageSent { timestamp, .. }
            | RoomEvent::MessageEdited { timestamp, .. }
            | RoomEvent::MessageDeleted { timestamp, .. }
            | RoomEvent::UserStartedTyping { timestamp, .. }
            | RoomEvent::UserStoppedTyping { timestamp, .. } => *timestamp,
        }
    }

    /// Get the user ID associated with the event (if any)
    pub fn user_id(&self) -> Option<UserId> {
        match self {
            RoomEvent::UserJoined { user_id, .. }
            | RoomEvent::UserLeft { user_id, .. }
            | RoomEvent::RoleChanged { user_id, .. }
            | RoomEvent::MessageSent { user_id, .. }
            | RoomEvent::MessageEdited { user_id, .. }
            | RoomEvent::UserStartedTyping { user_id, .. }
            | RoomEvent::UserStoppedTyping { user_id, .. } => Some(*user_id),
            RoomEvent::SettingsChanged { changed_by, .. } => Some(*changed_by),
            RoomEvent::MessageDeleted { deleted_by, .. } => Some(*deleted_by),
        }
    }

    /// Create a current timestamp
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Create a user joined event
    pub fn user_joined(user_id: UserId, username: String) -> Self {
        Self::UserJoined {
            user_id,
            username,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create a user left event
    pub fn user_left(user_id: UserId, username: String) -> Self {
        Self::UserLeft {
            user_id,
            username,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create a message sent event
    pub fn message_sent(message_id: MessageId, user_id: UserId, username: String) -> Self {
        Self::MessageSent {
            message_id,
            user_id,
            username,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create a typing started event
    pub fn typing_started(user_id: UserId, username: String) -> Self {
        Self::UserStartedTyping {
            user_id,
            username,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create a typing stopped event
    pub fn typing_stopped(user_id: UserId, username: String) -> Self {
        Self::UserStoppedTyping {
            user_id,
            username,
            timestamp: Self::current_timestamp(),
        }
    }
}

/// Chat room error types
#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("Room not found: {0}")]
    RoomNotFound(RoomId),

    #[error("User not found: {0}")]
    UserNotFound(UserId),

    #[error("Message not found: {0}")]
    MessageNotFound(MessageId),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Room is full")]
    RoomFull,

    #[error("Room is private")]
    RoomPrivate,

    #[error("User already in room")]
    UserAlreadyInRoom,

    #[error("User not in room")]
    UserNotInRoom,

    #[error("Invalid room name: {0}")]
    InvalidRoomName(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for chat operations
pub type ChatResult<T> = Result<T, ChatError>;

/// Typing indicator management
#[derive(Debug, Clone)]
pub struct TypingIndicators {
    /// Map of user ID to when they started typing
    typing_users: HashMap<UserId, u64>,
    /// Timeout for typing indicators (in seconds)
    timeout: u64,
}

impl TypingIndicators {
    /// Create new typing indicators manager
    pub fn new(timeout_seconds: u64) -> Self {
        Self {
            typing_users: HashMap::new(),
            timeout: timeout_seconds,
        }
    }

    /// User started typing
    pub fn start_typing(&mut self, user_id: UserId) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let was_typing = self.typing_users.contains_key(&user_id);
        self.typing_users.insert(user_id, now);
        !was_typing
    }

    /// User stopped typing
    pub fn stop_typing(&mut self, user_id: UserId) -> bool {
        self.typing_users.remove(&user_id).is_some()
    }

    /// Get list of currently typing users
    pub fn get_typing_users(&mut self) -> Vec<UserId> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Remove expired typing indicators
        self.typing_users
            .retain(|_, &mut start_time| now - start_time < self.timeout);

        self.typing_users.keys().copied().collect()
    }

    /// Check if user is currently typing
    pub fn is_typing(&self, user_id: &UserId) -> bool {
        if let Some(&start_time) = self.typing_users.get(user_id) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            now - start_time < self.timeout
        } else {
            false
        }
    }

    /// Clear all typing indicators
    pub fn clear(&mut self) {
        self.typing_users.clear();
    }
}

impl Default for TypingIndicators {
    fn default() -> Self {
        Self::new(5) // 5 second timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_event_creation() {
        let user_id = Uuid::new_v4();
        let username = "testuser".to_string();

        let event = RoomEvent::user_joined(user_id, username.clone());
        assert_eq!(event.user_id(), Some(user_id));
        assert!(event.timestamp() > 0);

        if let RoomEvent::UserJoined {
            user_id: id,
            username: name,
            ..
        } = event
        {
            assert_eq!(id, user_id);
            assert_eq!(name, username);
        } else {
            panic!("Expected UserJoined event");
        }
    }

    #[test]
    fn test_typing_indicators() {
        let mut indicators = TypingIndicators::new(5);
        let user_id = Uuid::new_v4();

        // User starts typing
        assert!(indicators.start_typing(user_id));
        assert!(indicators.is_typing(&user_id));
        assert_eq!(indicators.get_typing_users(), vec![user_id]);

        // User starts typing again (should return false)
        assert!(!indicators.start_typing(user_id));

        // User stops typing
        assert!(indicators.stop_typing(user_id));
        assert!(!indicators.is_typing(&user_id));
        assert!(indicators.get_typing_users().is_empty());
    }

    #[test]
    fn test_typing_timeout() {
        let mut indicators = TypingIndicators::new(0); // Immediate timeout
        let user_id = Uuid::new_v4();

        indicators.start_typing(user_id);
        // Sleep to ensure timeout
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Should be cleared due to timeout
        assert!(indicators.get_typing_users().is_empty());
        assert!(!indicators.is_typing(&user_id));
    }
}
