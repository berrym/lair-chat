//! Protocol message types for TCP communication.
//!
//! These types match the wire protocol specification in docs/protocols/TCP.md.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Core Types
// ============================================================================

/// User ID.
pub type UserId = Uuid;

/// Room ID.
pub type RoomId = Uuid;

/// Message ID.
pub type MessageId = Uuid;

/// Session ID.
pub type SessionId = Uuid;

/// Invitation ID.
pub type InvitationId = Uuid;

// ============================================================================
// Client -> Server Messages
// ============================================================================

/// Messages sent from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    // Handshake
    ClientHello {
        version: String,
        client_name: String,
        #[serde(default)]
        features: Vec<String>,
    },

    // Authentication
    /// Authenticate using a JWT token obtained from HTTP API.
    /// This is the recommended method (see ADR-013).
    Authenticate {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        /// JWT token from HTTP POST /auth/login or /auth/register
        token: String,
    },
    /// DEPRECATED: Use HTTP POST /auth/login + Authenticate instead.
    Login {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        identifier: String,
        password: String,
    },
    /// DEPRECATED: Use HTTP POST /auth/register + Authenticate instead.
    Register {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        username: String,
        email: String,
        password: String,
    },
    Logout {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },

    // Messaging
    SendMessage {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        target: MessageTarget,
        content: String,
    },
    EditMessage {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        message_id: MessageId,
        content: String,
    },
    DeleteMessage {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        message_id: MessageId,
    },
    GetMessages {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        target: MessageTarget,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        before: Option<DateTime<Utc>>,
    },

    // Rooms
    CreateRoom {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        settings: Option<RoomSettings>,
    },
    JoinRoom {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        room_id: RoomId,
    },
    LeaveRoom {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        room_id: RoomId,
    },
    ListRooms {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        filter: Option<RoomFilter>,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        offset: Option<u32>,
    },
    GetRoom {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        room_id: RoomId,
    },

    // Invitations
    InviteToRoom {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        room_id: RoomId,
        user_id: UserId,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    AcceptInvitation {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        invitation_id: InvitationId,
    },
    DeclineInvitation {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        invitation_id: InvitationId,
    },
    ListInvitations {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },

    // Users
    GetUser {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        user_id: UserId,
    },
    ListUsers {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        filter: Option<UserFilter>,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        offset: Option<u32>,
    },
    GetCurrentUser {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },

    // Typing indicator
    Typing {
        target: MessageTarget,
    },

    // Keepalive
    Ping,

    // Key Exchange (encryption handshake)
    KeyExchange {
        public_key: String,
    },
}

// ============================================================================
// Server -> Client Messages
// ============================================================================

/// Messages sent from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    // Handshake
    ServerHello {
        version: String,
        server_name: String,
        #[serde(default)]
        features: Vec<String>,
        #[serde(default)]
        encryption_required: bool,
    },

    // Authentication responses
    /// Response to Authenticate command (JWT token validation).
    AuthenticateResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<User>,
        #[serde(skip_serializing_if = "Option::is_none")]
        session: Option<Session>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    /// DEPRECATED: Response to legacy Login command.
    LoginResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<User>,
        #[serde(skip_serializing_if = "Option::is_none")]
        session: Option<Session>,
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    /// DEPRECATED: Response to legacy Register command.
    RegisterResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<User>,
        #[serde(skip_serializing_if = "Option::is_none")]
        session: Option<Session>,
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    LogoutResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
    },

    // Message responses
    SendMessageResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    EditMessageResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    DeleteMessageResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    GetMessagesResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(default)]
        messages: Vec<Message>,
        #[serde(default)]
        has_more: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },

    // Room responses
    CreateRoomResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        room: Option<Room>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    JoinRoomResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        room: Option<Room>,
        #[serde(skip_serializing_if = "Option::is_none")]
        membership: Option<RoomMembership>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    LeaveRoomResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    ListRoomsResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(default)]
        rooms: Vec<RoomListItem>,
        #[serde(default)]
        has_more: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        total_count: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    GetRoomResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        room: Option<Room>,
        #[serde(skip_serializing_if = "Option::is_none")]
        membership: Option<RoomMembership>,
        #[serde(skip_serializing_if = "Option::is_none")]
        member_count: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },

    // Invitation responses
    InviteToRoomResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        invitation: Option<Invitation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    AcceptInvitationResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        membership: Option<RoomMembership>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    DeclineInvitationResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    ListInvitationsResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(default)]
        invitations: Vec<Invitation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },

    // User responses
    GetUserResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<User>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    ListUsersResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(default)]
        users: Vec<User>,
        #[serde(default)]
        online_user_ids: Vec<String>,
        #[serde(default)]
        has_more: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        total_count: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    GetCurrentUserResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<User>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },

    // Keepalive
    Pong {
        #[serde(skip_serializing_if = "Option::is_none")]
        server_time: Option<DateTime<Utc>>,
    },

    // Key Exchange Response
    KeyExchangeResponse {
        public_key: String,
    },

    // Error
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        code: String,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<serde_json::Value>,
    },

    // Server-pushed events
    MessageReceived {
        message: Message,
        author_username: String,
    },
    MessageEdited {
        message: Message,
        #[serde(skip_serializing_if = "Option::is_none")]
        previous_content: Option<String>,
    },
    MessageDeleted {
        message_id: MessageId,
        target: MessageTarget,
        deleted_by: UserId,
    },
    UserJoinedRoom {
        room_id: RoomId,
        user: User,
        membership: RoomMembership,
    },
    UserLeftRoom {
        room_id: RoomId,
        user_id: UserId,
        reason: String,
    },
    RoomUpdated {
        room: Room,
        changed_by: UserId,
        #[serde(default)]
        changes: Vec<String>,
    },
    RoomDeleted {
        room_id: RoomId,
        room_name: String,
        deleted_by: UserId,
    },
    UserOnline {
        user_id: UserId,
        username: String,
    },
    UserOffline {
        user_id: UserId,
        username: String,
    },
    UserTyping {
        user_id: UserId,
        target: MessageTarget,
    },
    InvitationReceived {
        invitation: Invitation,
    },
    ServerNotice {
        message: String,
        severity: String,
    },
    SessionExpiring {
        session_id: SessionId,
        expires_at: DateTime<Utc>,
    },
}

// ============================================================================
// Shared Types
// ============================================================================

/// Target for a message (room or direct message).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageTarget {
    Room {
        room_id: RoomId,
    },
    #[serde(rename = "dm")]
    DirectMessage {
        recipient: UserId,
    },
}

/// User information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub email: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

/// Session information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub expires_at: DateTime<Utc>,
}

/// Chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub author: UserId,
    pub target: MessageTarget,
    pub content: String,
    pub edited: bool,
    pub created_at: DateTime<Utc>,
}

/// Room information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: RoomId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub owner: UserId,
    pub settings: RoomSettings,
    pub created_at: DateTime<Utc>,
}

/// Room settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoomSettings {
    #[serde(default)]
    pub public: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_members: Option<u32>,
    #[serde(default)]
    pub moderated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Room membership.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMembership {
    pub room_id: RoomId,
    pub user_id: UserId,
    pub room_role: String,
    pub joined_at: DateTime<Utc>,
}

/// Room list item (with extra info).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomListItem {
    pub room: Room,
    pub member_count: u64,
    pub is_member: bool,
}

/// Room filter for list queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoomFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(default)]
    pub joined_only: bool,
    #[serde(default)]
    pub public_only: bool,
}

/// User filter for list queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(default)]
    pub online_only: bool,
}

/// Invitation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invitation {
    pub id: InvitationId,
    pub room_id: RoomId,
    pub room_name: String,
    pub inviter_id: UserId,
    pub inviter_name: String,
    pub invitee_id: UserId,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Error information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

// ============================================================================
// Helper Methods
// ============================================================================

impl ClientMessage {
    /// Create an authenticate message (recommended - uses JWT from HTTP API).
    pub fn authenticate(token: impl Into<String>) -> Self {
        Self::Authenticate {
            request_id: Some(Uuid::new_v4().to_string()),
            token: token.into(),
        }
    }

    /// Create a login message.
    /// DEPRECATED: Use HTTP POST /auth/login + authenticate() instead.
    #[allow(dead_code)]
    pub fn login(identifier: impl Into<String>, password: impl Into<String>) -> Self {
        Self::Login {
            request_id: Some(Uuid::new_v4().to_string()),
            identifier: identifier.into(),
            password: password.into(),
        }
    }

    /// Create a register message.
    /// DEPRECATED: Use HTTP POST /auth/register + authenticate() instead.
    #[allow(dead_code)]
    pub fn register(
        username: impl Into<String>,
        email: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self::Register {
            request_id: Some(Uuid::new_v4().to_string()),
            username: username.into(),
            email: email.into(),
            password: password.into(),
        }
    }

    /// Create a send message request.
    pub fn send_message(target: MessageTarget, content: impl Into<String>) -> Self {
        Self::SendMessage {
            request_id: Some(Uuid::new_v4().to_string()),
            target,
            content: content.into(),
        }
    }

    /// Create a client hello message.
    pub fn client_hello() -> Self {
        Self::ClientHello {
            version: "1.0".to_string(),
            client_name: "Lair Chat TUI".to_string(),
            features: vec![],
        }
    }

    /// Create a client hello message with encryption support.
    pub fn client_hello_with_encryption() -> Self {
        Self::ClientHello {
            version: "1.0".to_string(),
            client_name: "Lair Chat TUI".to_string(),
            features: vec!["encryption".to_string()],
        }
    }

    /// Create a key exchange message.
    pub fn key_exchange(public_key: String) -> Self {
        Self::KeyExchange { public_key }
    }
}

#[allow(dead_code)]
impl ServerMessage {
    /// Check if this is an error message.
    pub fn is_error(&self) -> bool {
        matches!(self, ServerMessage::Error { .. })
    }

    /// Get the request ID if present.
    pub fn request_id(&self) -> Option<&str> {
        match self {
            ServerMessage::AuthenticateResponse { request_id, .. }
            | ServerMessage::LoginResponse { request_id, .. }
            | ServerMessage::RegisterResponse { request_id, .. }
            | ServerMessage::LogoutResponse { request_id, .. }
            | ServerMessage::SendMessageResponse { request_id, .. }
            | ServerMessage::EditMessageResponse { request_id, .. }
            | ServerMessage::DeleteMessageResponse { request_id, .. }
            | ServerMessage::GetMessagesResponse { request_id, .. }
            | ServerMessage::CreateRoomResponse { request_id, .. }
            | ServerMessage::JoinRoomResponse { request_id, .. }
            | ServerMessage::LeaveRoomResponse { request_id, .. }
            | ServerMessage::ListRoomsResponse { request_id, .. }
            | ServerMessage::GetRoomResponse { request_id, .. }
            | ServerMessage::InviteToRoomResponse { request_id, .. }
            | ServerMessage::AcceptInvitationResponse { request_id, .. }
            | ServerMessage::DeclineInvitationResponse { request_id, .. }
            | ServerMessage::ListInvitationsResponse { request_id, .. }
            | ServerMessage::GetUserResponse { request_id, .. }
            | ServerMessage::ListUsersResponse { request_id, .. }
            | ServerMessage::GetCurrentUserResponse { request_id, .. }
            | ServerMessage::Error { request_id, .. } => request_id.as_deref(),
            _ => None,
        }
    }
}
