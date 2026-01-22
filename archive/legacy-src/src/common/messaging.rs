//! Unified Message Routing System
//!
//! This module provides a centralized system for handling all message routing,
//! formatting, and display throughout the application. It replaces the scattered
//! message handling with a clean, type-safe interface.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Defines who should receive a message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageTarget {
    /// Send to a specific user by username
    User(String),
    /// Send to multiple specific users
    UserList(Vec<String>),
    /// Send to all users in a specific room
    Room(String),
    /// Send to all connected users (global broadcast)
    Broadcast,
    /// Send back to the message sender (confirmations, errors)
    Sender,
    /// Send to users except the sender (for announcements)
    Others,
}

/// All possible system message types with their data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemMessage {
    // Direct Message related
    DirectMessage {
        from: String,
        to: String,
        content: String,
    },
    DirectMessageConfirmation {
        target: String,
        content: String,
    },

    // Authentication related
    Welcome {
        username: String,
    },
    UserJoined {
        username: String,
        room: String,
    },
    UserLeft {
        username: String,
        room: String,
    },
    AuthenticationFailed {
        reason: String,
    },
    RegistrationSuccess {
        username: String,
    },
    Logout {
        username: String,
    },

    // Room related
    RoomCreated {
        room_name: String,
        creator: String,
    },
    RoomJoined {
        room_name: String,
        username: String,
    },
    RoomLeft {
        room_name: String,
        username: String,
    },
    RoomListResponse {
        rooms: Vec<String>,
    },
    RoomNotFound {
        room_name: String,
    },
    RoomAlreadyExists {
        room_name: String,
    },

    // Invitation related
    InvitationReceived {
        from: String,
        room_name: String,
        message: String,
    },
    InvitationSent {
        to: String,
        room_name: String,
    },
    InvitationAccepted {
        from: String,
        room_name: String,
    },
    InvitationDeclined {
        from: String,
        room_name: String,
    },
    InvitationError {
        reason: String,
    },

    // User list updates
    UserListUpdate {
        users: Vec<String>,
    },

    // Error messages
    Error {
        message: String,
    },
    ConnectionError {
        reason: String,
    },
    CommandNotRecognized {
        command: String,
    },
    InsufficientPermissions {
        action: String,
    },

    // Status messages
    StatusUpdate {
        message: String,
    },
    ServerShutdown,
    MaintenanceMode {
        message: String,
    },
}

/// User-generated chat message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub from: String,
    pub content: String,
    pub room: String,
    pub timestamp: u64,
}

/// Complete message envelope that can be sent through the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    /// System-generated message (notifications, errors, etc.)
    System(SystemMessage),
    /// User-generated chat message
    Chat(ChatMessage),
}

/// Priority level for message delivery
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Complete message routing instruction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRoute {
    pub target: MessageTarget,
    pub message: Message,
    pub priority: MessagePriority,
    pub sender_id: Option<String>,
}

impl MessageRoute {
    /// Create a new message route
    pub fn new(target: MessageTarget, message: Message) -> Self {
        Self {
            target,
            message,
            priority: MessagePriority::Normal,
            sender_id: None,
        }
    }

    /// Set the priority of this message
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the sender ID for this message
    pub fn with_sender(mut self, sender_id: String) -> Self {
        self.sender_id = Some(sender_id);
        self
    }
}

/// Display formatting for messages in the UI
impl fmt::Display for SystemMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemMessage::DirectMessage { from, content, .. } => {
                write!(f, "💬 DM from {}: {}", from, content)
            }
            SystemMessage::DirectMessageConfirmation { target, content } => {
                write!(f, "✅ DM sent to {}: {}", target, content)
            }
            SystemMessage::Welcome { username } => {
                write!(f, "🎉 Welcome back, {}!", username)
            }
            SystemMessage::UserJoined { username, room } => {
                write!(f, "👋 {} joined {}", username, room)
            }
            SystemMessage::UserLeft { username, room } => {
                write!(f, "👋 {} left {}", username, room)
            }
            SystemMessage::AuthenticationFailed { reason } => {
                write!(f, "❌ Authentication failed: {}", reason)
            }
            SystemMessage::RegistrationSuccess { username } => {
                write!(f, "✅ Registration successful for {}", username)
            }
            SystemMessage::Logout { username } => {
                write!(f, "👋 {} logged out", username)
            }
            SystemMessage::RoomCreated { room_name, creator } => {
                write!(f, "🏠 Room '{}' created by {}", room_name, creator)
            }
            SystemMessage::RoomJoined {
                room_name,
                username,
            } => {
                write!(f, "✅ {} joined room '{}'", username, room_name)
            }
            SystemMessage::RoomLeft {
                room_name,
                username,
            } => {
                write!(f, "⬅️ {} left room '{}'", username, room_name)
            }
            SystemMessage::RoomListResponse { rooms } => {
                if rooms.is_empty() {
                    write!(f, "📋 No rooms available")
                } else {
                    write!(f, "📋 Available rooms: {}", rooms.join(", "))
                }
            }
            SystemMessage::RoomNotFound { room_name } => {
                write!(f, "❌ Room '{}' not found", room_name)
            }
            SystemMessage::RoomAlreadyExists { room_name } => {
                write!(f, "⚠️ Room '{}' already exists", room_name)
            }
            SystemMessage::InvitationReceived {
                from,
                room_name,
                message,
            } => {
                write!(f, "🔔 {}: {}", from, message)
            }
            SystemMessage::InvitationSent { to, room_name } => {
                write!(f, "📤 Invitation sent to {} for room '{}'", to, room_name)
            }
            SystemMessage::InvitationAccepted { from, room_name } => {
                write!(f, "✅ {} accepted invitation to '{}'", from, room_name)
            }
            SystemMessage::InvitationDeclined { from, room_name } => {
                write!(f, "❌ {} declined invitation to '{}'", from, room_name)
            }
            SystemMessage::InvitationError { reason } => {
                write!(f, "❌ Invitation error: {}", reason)
            }
            SystemMessage::UserListUpdate { users } => {
                write!(f, "👥 Online users: {}", users.join(", "))
            }
            SystemMessage::Error { message } => {
                write!(f, "❌ Error: {}", message)
            }
            SystemMessage::ConnectionError { reason } => {
                write!(f, "🔌 Connection error: {}", reason)
            }
            SystemMessage::CommandNotRecognized { command } => {
                write!(f, "❓ Unknown command: {}", command)
            }
            SystemMessage::InsufficientPermissions { action } => {
                write!(f, "🚫 Insufficient permissions for: {}", action)
            }
            SystemMessage::StatusUpdate { message } => {
                write!(f, "ℹ️ {}", message)
            }
            SystemMessage::ServerShutdown => {
                write!(f, "🔴 Server is shutting down")
            }
            SystemMessage::MaintenanceMode { message } => {
                write!(f, "🔧 Maintenance: {}", message)
            }
        }
    }
}

impl fmt::Display for ChatMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.from, self.content)
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::System(sys_msg) => write!(f, "{}", sys_msg),
            Message::Chat(chat_msg) => write!(f, "{}", chat_msg),
        }
    }
}

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
}

/// Helper functions for creating common message types
impl SystemMessage {
    /// Create a direct message from one user to another
    pub fn direct_message(from: String, to: String, content: String) -> Self {
        SystemMessage::DirectMessage { from, to, content }
    }

    /// Create a DM confirmation message
    pub fn dm_confirmation(target: String, content: String) -> Self {
        SystemMessage::DirectMessageConfirmation { target, content }
    }

    /// Create a user joined message
    pub fn user_joined(username: String, room: String) -> Self {
        SystemMessage::UserJoined { username, room }
    }

    /// Create a user left message
    pub fn user_left(username: String, room: String) -> Self {
        SystemMessage::UserLeft { username, room }
    }

    /// Create a welcome message
    pub fn welcome(username: String) -> Self {
        SystemMessage::Welcome { username }
    }

    /// Create a generic error message
    pub fn error(message: String) -> Self {
        SystemMessage::Error { message }
    }

    /// Create a room created message
    pub fn room_created(room_name: String, creator: String) -> Self {
        SystemMessage::RoomCreated { room_name, creator }
    }

    /// Create an invitation received message
    pub fn invitation_received(from: String, room_name: String, message: String) -> Self {
        SystemMessage::InvitationReceived {
            from,
            room_name,
            message,
        }
    }
}

impl ChatMessage {
    /// Create a new chat message
    pub fn new(from: String, content: String, room: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            content,
            room,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_message_display() {
        let msg = SystemMessage::direct_message(
            "alice".to_string(),
            "bob".to_string(),
            "hello".to_string(),
        );
        assert_eq!(format!("{}", msg), "💬 DM from alice: hello");
    }

    #[test]
    fn test_message_route_creation() {
        let sys_msg = SystemMessage::welcome("alice".to_string());
        let route = MessageRoute::new(
            MessageTarget::User("alice".to_string()),
            Message::System(sys_msg),
        )
        .with_priority(MessagePriority::High);

        assert_eq!(route.priority, MessagePriority::High);
        assert_eq!(route.target, MessageTarget::User("alice".to_string()));
    }

    #[test]
    fn test_chat_message_creation() {
        let msg = ChatMessage::new(
            "alice".to_string(),
            "hello world".to_string(),
            "lobby".to_string(),
        );

        assert_eq!(msg.from, "alice");
        assert_eq!(msg.content, "hello world");
        assert_eq!(msg.room, "lobby");
        assert_eq!(format!("{}", msg), "alice: hello world");
    }
}
