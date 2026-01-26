//! TCP wire protocol parsing and serialization.
//!
//! Implements the length-prefixed JSON protocol specified in docs/protocols/TCP.md.
//!
//! ## Message Format
//!
//! ```text
//! ┌──────────────────┬─────────────────────────────┐
//! │ Length (4 bytes) │ JSON Payload (N bytes)      │
//! │ Big-endian u32   │ UTF-8 encoded               │
//! └──────────────────┴─────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::crypto::{Cipher, NONCE_SIZE};
use crate::domain::{
    Invitation, Message, MessageTarget, Room, RoomMembership, RoomSettings, Session, User,
};

/// Maximum message size (1 MB).
pub const MAX_MESSAGE_SIZE: u32 = 1_048_576;

/// Protocol version.
pub const PROTOCOL_VERSION: &str = "1.0";

/// Server name for handshake.
pub const SERVER_NAME: &str = "Lair Chat";

// ============================================================================
// Frame Reading/Writing
// ============================================================================

/// Read a length-prefixed message from a stream.
pub async fn read_message<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> Result<String, ProtocolError> {
    // Read 4-byte length prefix
    let mut length_bytes = [0u8; 4];
    reader.read_exact(&mut length_bytes).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            ProtocolError::ConnectionClosed
        } else {
            ProtocolError::Io(e)
        }
    })?;

    let length = u32::from_be_bytes(length_bytes);

    // Validate length
    if length > MAX_MESSAGE_SIZE {
        return Err(ProtocolError::MessageTooLarge {
            size: length,
            max: MAX_MESSAGE_SIZE,
        });
    }

    // Read payload
    let mut payload = vec![0u8; length as usize];
    reader.read_exact(&mut payload).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            ProtocolError::ConnectionClosed
        } else {
            ProtocolError::Io(e)
        }
    })?;

    // Decode as UTF-8
    String::from_utf8(payload).map_err(|_| ProtocolError::InvalidUtf8)
}

/// Write a length-prefixed message to a stream.
pub async fn write_message<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    payload: &str,
) -> Result<(), ProtocolError> {
    let bytes = payload.as_bytes();

    if bytes.len() > MAX_MESSAGE_SIZE as usize {
        return Err(ProtocolError::MessageTooLarge {
            size: bytes.len() as u32,
            max: MAX_MESSAGE_SIZE,
        });
    }

    // Write length prefix
    let length = bytes.len() as u32;
    writer
        .write_all(&length.to_be_bytes())
        .await
        .map_err(ProtocolError::Io)?;

    // Write payload
    writer.write_all(bytes).await.map_err(ProtocolError::Io)?;

    // Flush
    writer.flush().await.map_err(ProtocolError::Io)?;

    Ok(())
}

// ============================================================================
// Encrypted Frame Reading/Writing
// ============================================================================

/// Minimum size for encrypted frame: nonce (12) + tag (16) = 28 bytes
const MIN_ENCRYPTED_SIZE: usize = NONCE_SIZE + 16;

/// Read an encrypted message from a stream.
///
/// Frame format:
/// ```text
/// ┌──────────────────┬──────────────────┬─────────────────────────────┐
/// │ Length (4 bytes) │ Nonce (12 bytes) │ Ciphertext (N bytes)        │
/// │ Big-endian u32   │ Random           │ AES-256-GCM + 16-byte tag   │
/// └──────────────────┴──────────────────┴─────────────────────────────┘
/// ```
pub async fn read_encrypted_message<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    cipher: &Cipher,
) -> Result<String, ProtocolError> {
    // Read 4-byte length prefix
    let mut length_bytes = [0u8; 4];
    reader.read_exact(&mut length_bytes).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            ProtocolError::ConnectionClosed
        } else {
            ProtocolError::Io(e)
        }
    })?;

    let length = u32::from_be_bytes(length_bytes) as usize;

    // Validate length
    if length > MAX_MESSAGE_SIZE as usize {
        return Err(ProtocolError::MessageTooLarge {
            size: length as u32,
            max: MAX_MESSAGE_SIZE,
        });
    }

    if length < MIN_ENCRYPTED_SIZE {
        return Err(ProtocolError::EncryptedMessageTooSmall);
    }

    // Read nonce
    let mut nonce = [0u8; NONCE_SIZE];
    reader.read_exact(&mut nonce).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            ProtocolError::ConnectionClosed
        } else {
            ProtocolError::Io(e)
        }
    })?;

    // Read ciphertext (length - nonce size)
    let ciphertext_len = length - NONCE_SIZE;
    let mut ciphertext = vec![0u8; ciphertext_len];
    reader.read_exact(&mut ciphertext).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            ProtocolError::ConnectionClosed
        } else {
            ProtocolError::Io(e)
        }
    })?;

    // Decrypt
    let plaintext = cipher
        .decrypt(&nonce, &ciphertext)
        .map_err(|e| ProtocolError::DecryptionFailed(e.to_string()))?;

    // Decode as UTF-8
    String::from_utf8(plaintext).map_err(|_| ProtocolError::InvalidUtf8)
}

/// Write an encrypted message to a stream.
pub async fn write_encrypted_message<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    payload: &str,
    cipher: &Cipher,
) -> Result<(), ProtocolError> {
    let plaintext = payload.as_bytes();

    // Encrypt
    let (nonce, ciphertext) = cipher
        .encrypt(plaintext)
        .map_err(|e| ProtocolError::EncryptionFailed(e.to_string()))?;

    // Total frame size = nonce + ciphertext
    let frame_size = NONCE_SIZE + ciphertext.len();

    if frame_size > MAX_MESSAGE_SIZE as usize {
        return Err(ProtocolError::MessageTooLarge {
            size: frame_size as u32,
            max: MAX_MESSAGE_SIZE,
        });
    }

    // Write length prefix
    let length = frame_size as u32;
    writer
        .write_all(&length.to_be_bytes())
        .await
        .map_err(ProtocolError::Io)?;

    // Write nonce
    writer.write_all(&nonce).await.map_err(ProtocolError::Io)?;

    // Write ciphertext
    writer
        .write_all(&ciphertext)
        .await
        .map_err(ProtocolError::Io)?;

    // Flush
    writer.flush().await.map_err(ProtocolError::Io)?;

    Ok(())
}

// ============================================================================
// Protocol Errors
// ============================================================================

/// Protocol-level errors.
#[derive(Debug)]
pub enum ProtocolError {
    /// I/O error.
    Io(std::io::Error),
    /// Connection closed by peer.
    ConnectionClosed,
    /// Message exceeds size limit.
    MessageTooLarge { size: u32, max: u32 },
    /// Invalid UTF-8 in payload.
    InvalidUtf8,
    /// Invalid JSON.
    InvalidJson(serde_json::Error),
    /// Unknown message type.
    UnknownMessageType(String),
    /// Missing required field.
    MissingField(String),
    /// Version mismatch.
    VersionMismatch { client: String, server: String },
    /// Encrypted message too small (must have at least nonce + tag).
    EncryptedMessageTooSmall,
    /// Encryption failed.
    EncryptionFailed(String),
    /// Decryption failed.
    DecryptionFailed(String),
    /// Key exchange failed.
    KeyExchangeFailed(String),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::ConnectionClosed => write!(f, "Connection closed"),
            Self::MessageTooLarge { size, max } => {
                write!(f, "Message too large: {} bytes (max: {})", size, max)
            }
            Self::InvalidUtf8 => write!(f, "Invalid UTF-8 in message"),
            Self::InvalidJson(e) => write!(f, "Invalid JSON: {}", e),
            Self::UnknownMessageType(t) => write!(f, "Unknown message type: {}", t),
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
            Self::VersionMismatch { client, server } => {
                write!(f, "Version mismatch: client={}, server={}", client, server)
            }
            Self::EncryptedMessageTooSmall => write!(f, "Encrypted message too small"),
            Self::EncryptionFailed(e) => write!(f, "Encryption failed: {}", e),
            Self::DecryptionFailed(e) => write!(f, "Decryption failed: {}", e),
            Self::KeyExchangeFailed(e) => write!(f, "Key exchange failed: {}", e),
        }
    }
}

impl std::error::Error for ProtocolError {}

// ============================================================================
// Client Messages (requests from client)
// ============================================================================

/// Messages sent by the client.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    // Handshake
    ClientHello {
        version: String,
        client_name: Option<String>,
        #[serde(default)]
        features: Vec<String>,
    },

    // Authentication
    Login {
        request_id: Option<String>,
        identifier: String,
        password: String,
    },
    Register {
        request_id: Option<String>,
        username: String,
        email: String,
        password: String,
    },
    Logout {
        request_id: Option<String>,
    },

    // Messaging
    SendMessage {
        request_id: Option<String>,
        target: MessageTarget,
        content: String,
    },
    EditMessage {
        request_id: Option<String>,
        message_id: String,
        content: String,
    },
    DeleteMessage {
        request_id: Option<String>,
        message_id: String,
    },
    GetMessages {
        request_id: Option<String>,
        target: MessageTarget,
        #[serde(default = "default_limit")]
        limit: u32,
        before: Option<String>,
    },

    // Rooms
    CreateRoom {
        request_id: Option<String>,
        name: String,
        description: Option<String>,
        #[serde(default)]
        settings: Option<RoomSettingsRequest>,
    },
    GetRoom {
        request_id: Option<String>,
        room_id: String,
    },
    ListRooms {
        request_id: Option<String>,
        #[serde(default)]
        filter: Option<RoomFilter>,
        #[serde(default = "default_limit")]
        limit: u32,
        #[serde(default)]
        offset: u32,
    },
    JoinRoom {
        request_id: Option<String>,
        room_id: String,
    },
    LeaveRoom {
        request_id: Option<String>,
        room_id: String,
    },

    // Invitations
    InviteToRoom {
        request_id: Option<String>,
        room_id: String,
        user_id: String,
        message: Option<String>,
    },
    AcceptInvitation {
        request_id: Option<String>,
        invitation_id: String,
    },
    DeclineInvitation {
        request_id: Option<String>,
        invitation_id: String,
    },
    ListInvitations {
        request_id: Option<String>,
    },

    // Users
    GetUser {
        request_id: Option<String>,
        user_id: String,
    },
    ListUsers {
        request_id: Option<String>,
        #[serde(default)]
        filter: Option<UserFilter>,
        #[serde(default = "default_limit")]
        limit: u32,
        #[serde(default)]
        offset: u32,
    },
    GetCurrentUser {
        request_id: Option<String>,
    },

    // Presence
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

fn default_limit() -> u32 {
    50
}

/// Room settings in request format.
#[derive(Debug, Clone, Deserialize)]
pub struct RoomSettingsRequest {
    #[serde(default)]
    pub public: Option<bool>,
    pub max_members: Option<u32>,
}

impl From<RoomSettingsRequest> for RoomSettings {
    fn from(req: RoomSettingsRequest) -> Self {
        RoomSettings {
            description: None,
            is_private: req.public.map(|p| !p).unwrap_or(false),
            max_members: req.max_members,
        }
    }
}

/// Room filter for list queries.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RoomFilter {
    pub search: Option<String>,
    #[serde(default)]
    pub joined_only: bool,
    #[serde(default)]
    pub public_only: bool,
}

/// User filter for list queries.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UserFilter {
    pub search: Option<String>,
    #[serde(default)]
    pub online_only: bool,
}

// ============================================================================
// Server Messages (responses and events)
// ============================================================================

/// Messages sent by the server.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    // Handshake
    ServerHello {
        version: String,
        server_name: String,
        features: Vec<String>,
        encryption_required: bool,
    },

    // Authentication responses
    LoginResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<User>,
        #[serde(skip_serializing_if = "Option::is_none")]
        session: Option<SessionInfo>,
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    RegisterResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<User>,
        #[serde(skip_serializing_if = "Option::is_none")]
        session: Option<SessionInfo>,
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    LogoutResponse {
        request_id: Option<String>,
        success: bool,
    },

    // Message responses
    SendMessageResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    EditMessageResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    DeleteMessageResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    GetMessagesResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        messages: Option<Vec<Message>>,
        has_more: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },

    // Room responses
    CreateRoomResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        room: Option<Room>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    GetRoomResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        room: Option<Room>,
        #[serde(skip_serializing_if = "Option::is_none")]
        membership: Option<RoomMembership>,
        member_count: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    ListRoomsResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        rooms: Option<Vec<RoomListItem>>,
        has_more: bool,
        total_count: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    JoinRoomResponse {
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
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },

    // Invitation responses
    InviteToRoomResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        invitation: Option<Invitation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    AcceptInvitationResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        membership: Option<RoomMembership>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    DeclineInvitationResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    ListInvitationsResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        invitations: Option<Vec<Invitation>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },

    // User responses
    GetUserResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<User>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    ListUsersResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        users: Option<Vec<User>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        online_user_ids: Option<Vec<String>>,
        has_more: bool,
        total_count: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },
    GetCurrentUserResponse {
        request_id: Option<String>,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<User>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorInfo>,
    },

    // Keepalive
    Pong {
        server_time: String,
    },

    // Key Exchange Response
    KeyExchangeResponse {
        public_key: String,
    },

    // Events (server-initiated)
    MessageReceived {
        message: Message,
        author_username: String,
    },
    MessageEdited {
        message: Message,
        previous_content: String,
    },
    MessageDeleted {
        message_id: String,
        target: MessageTarget,
        deleted_by: String,
    },
    UserJoinedRoom {
        room_id: String,
        user: User,
        membership: RoomMembership,
    },
    UserLeftRoom {
        room_id: String,
        user_id: String,
        reason: String,
    },
    RoomUpdated {
        room: Room,
        changed_by: String,
    },
    RoomDeleted {
        room_id: String,
        room_name: String,
        deleted_by: String,
    },
    UserOnline {
        user_id: String,
        username: String,
    },
    UserOffline {
        user_id: String,
        username: String,
    },
    UserTyping {
        user_id: String,
        target: MessageTarget,
    },
    InvitationReceived {
        invitation: Invitation,
    },
    ServerNotice {
        message: String,
        severity: String,
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
}

/// Session info for responses.
#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub expires_at: String,
}

impl From<&Session> for SessionInfo {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.to_string(),
            expires_at: session.expires_at.to_rfc3339(),
        }
    }
}

/// Error info for responses.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

impl ErrorInfo {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Room list item with metadata.
#[derive(Debug, Clone, Serialize)]
pub struct RoomListItem {
    pub room: Room,
    pub member_count: u32,
    pub is_member: bool,
}

// ============================================================================
// Message Parsing
// ============================================================================

impl ClientMessage {
    /// Parse a client message from JSON.
    pub fn parse(json: &str) -> Result<Self, ProtocolError> {
        serde_json::from_str(json).map_err(ProtocolError::InvalidJson)
    }
}

impl ServerMessage {
    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, ProtocolError> {
        serde_json::to_string(self).map_err(ProtocolError::InvalidJson)
    }

    /// Create a server hello message.
    pub fn server_hello() -> Self {
        Self::ServerHello {
            version: PROTOCOL_VERSION.to_string(),
            server_name: SERVER_NAME.to_string(),
            features: vec!["encryption".to_string()],
            encryption_required: false,
        }
    }

    /// Create an error response.
    pub fn error(request_id: Option<String>, code: &str, message: &str) -> Self {
        Self::Error {
            request_id,
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        }
    }

    /// Create a pong response.
    pub fn pong() -> Self {
        Self::Pong {
            server_time: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_client_hello() {
        let json = r#"{"type":"client_hello","version":"1.0","client_name":"Test"}"#;
        let msg = ClientMessage::parse(json).unwrap();
        match msg {
            ClientMessage::ClientHello { version, .. } => {
                assert_eq!(version, "1.0");
            }
            _ => panic!("Expected ClientHello"),
        }
    }

    #[test]
    fn test_parse_login() {
        let json = r#"{"type":"login","identifier":"alice","password":"secret"}"#;
        let msg = ClientMessage::parse(json).unwrap();
        match msg {
            ClientMessage::Login {
                identifier,
                password,
                ..
            } => {
                assert_eq!(identifier, "alice");
                assert_eq!(password, "secret");
            }
            _ => panic!("Expected Login"),
        }
    }

    #[test]
    fn test_parse_ping() {
        let json = r#"{"type":"ping"}"#;
        let msg = ClientMessage::parse(json).unwrap();
        assert!(matches!(msg, ClientMessage::Ping));
    }

    #[test]
    fn test_serialize_server_hello() {
        let msg = ServerMessage::server_hello();
        let json = msg.to_json().unwrap();
        assert!(json.contains("server_hello"));
        assert!(json.contains(PROTOCOL_VERSION));
    }

    #[test]
    fn test_serialize_pong() {
        let msg = ServerMessage::pong();
        let json = msg.to_json().unwrap();
        assert!(json.contains("pong"));
        assert!(json.contains("server_time"));
    }

    #[test]
    fn test_serialize_error() {
        let msg = ServerMessage::error(Some("req-1".to_string()), "not_found", "User not found");
        let json = msg.to_json().unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("not_found"));
    }

    #[tokio::test]
    async fn test_read_write_message() {
        use tokio::io::duplex;

        let (mut client, mut server) = duplex(1024);

        // Write from client side
        let payload = r#"{"type":"ping"}"#;
        write_message(&mut client, payload).await.unwrap();

        // Read from server side
        let received = read_message(&mut server).await.unwrap();
        assert_eq!(received, payload);
    }
}
