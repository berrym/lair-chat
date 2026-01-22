//! Domain events for real-time updates.
//!
//! See [EVENTS.md](../../../../docs/architecture/EVENTS.md) for full specification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

use super::{
    Invitation, InvitationId, Message, MessageId, MessageTarget, Room, RoomId, RoomMembership,
    SessionId, User, UserId, ValidationError,
};

// ============================================================================
// EventId
// ============================================================================

/// Unique identifier for an event (for deduplication).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(Uuid);

impl EventId {
    /// Create a new random EventId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create an EventId from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse an EventId from a string.
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

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for EventId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for EventId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

// ============================================================================
// Event
// ============================================================================

/// A domain event representing a state change in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event ID (for deduplication).
    pub id: EventId,
    /// Event type and payload.
    pub payload: EventPayload,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
}

impl Event {
    /// Create a new event with the given payload.
    pub fn new(payload: EventPayload) -> Self {
        Self {
            id: EventId::new(),
            payload,
            timestamp: Utc::now(),
        }
    }

    /// Get the event type name (for wire protocol).
    pub fn event_type(&self) -> &'static str {
        self.payload.event_type()
    }

    /// Get the target audience for this event.
    pub fn target(&self) -> EventTarget {
        self.payload.target()
    }
}

// ============================================================================
// EventPayload
// ============================================================================

/// The payload of an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventPayload {
    // Message events
    MessageReceived(MessageReceivedEvent),
    MessageEdited(MessageEditedEvent),
    MessageDeleted(MessageDeletedEvent),

    // Room events
    UserJoinedRoom(UserJoinedRoomEvent),
    UserLeftRoom(UserLeftRoomEvent),
    RoomUpdated(RoomUpdatedEvent),
    RoomDeleted(RoomDeletedEvent),

    // User presence events
    UserOnline(UserOnlineEvent),
    UserOffline(UserOfflineEvent),
    UserTyping(UserTypingEvent),

    // Invitation events
    InvitationReceived(InvitationReceivedEvent),
    InvitationCancelled(InvitationCancelledEvent),

    // System events
    ServerNotice(ServerNoticeEvent),
    SessionExpiring(SessionExpiringEvent),
}

impl EventPayload {
    /// Get the event type name (for wire protocol).
    pub fn event_type(&self) -> &'static str {
        match self {
            EventPayload::MessageReceived(_) => "message_received",
            EventPayload::MessageEdited(_) => "message_edited",
            EventPayload::MessageDeleted(_) => "message_deleted",
            EventPayload::UserJoinedRoom(_) => "user_joined_room",
            EventPayload::UserLeftRoom(_) => "user_left_room",
            EventPayload::RoomUpdated(_) => "room_updated",
            EventPayload::RoomDeleted(_) => "room_deleted",
            EventPayload::UserOnline(_) => "user_online",
            EventPayload::UserOffline(_) => "user_offline",
            EventPayload::UserTyping(_) => "user_typing",
            EventPayload::InvitationReceived(_) => "invitation_received",
            EventPayload::InvitationCancelled(_) => "invitation_cancelled",
            EventPayload::ServerNotice(_) => "server_notice",
            EventPayload::SessionExpiring(_) => "session_expiring",
        }
    }

    /// Get the target audience for this event.
    pub fn target(&self) -> EventTarget {
        match self {
            EventPayload::MessageReceived(e) => e.target(),
            EventPayload::MessageEdited(e) => e.target(),
            EventPayload::MessageDeleted(e) => e.target(),
            EventPayload::UserJoinedRoom(e) => EventTarget::Room(e.room_id),
            EventPayload::UserLeftRoom(e) => EventTarget::Room(e.room_id),
            EventPayload::RoomUpdated(e) => EventTarget::Room(e.room.id),
            EventPayload::RoomDeleted(e) => EventTarget::Room(e.room_id),
            EventPayload::UserOnline(e) => EventTarget::UserConnections(e.user_id),
            EventPayload::UserOffline(e) => EventTarget::UserConnections(e.user_id),
            EventPayload::UserTyping(e) => e.target(),
            EventPayload::InvitationReceived(e) => EventTarget::User(e.invitation.invitee),
            EventPayload::InvitationCancelled(e) => EventTarget::User(e.invitee),
            EventPayload::ServerNotice(_) => EventTarget::Broadcast,
            EventPayload::SessionExpiring(e) => EventTarget::Session(e.session_id),
        }
    }
}

// ============================================================================
// EventTarget
// ============================================================================

/// The target audience for an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventTarget {
    /// Send to a specific user (all their sessions).
    User(UserId),
    /// Send to all members of a room.
    Room(RoomId),
    /// Send to participants in a DM conversation.
    DirectMessage { user1: UserId, user2: UserId },
    /// Send to all users who share a room or DM with this user.
    UserConnections(UserId),
    /// Send to a specific session.
    Session(SessionId),
    /// Send to all connected clients (rare, admin only).
    Broadcast,
}

// ============================================================================
// Message Events
// ============================================================================

/// A new message was sent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReceivedEvent {
    pub message: Message,
}

impl MessageReceivedEvent {
    pub fn new(message: Message) -> Self {
        Self { message }
    }

    pub fn target(&self) -> EventTarget {
        match &self.message.target {
            MessageTarget::Room { room_id } => EventTarget::Room(*room_id),
            MessageTarget::DirectMessage { recipient } => EventTarget::DirectMessage {
                user1: self.message.author,
                user2: *recipient,
            },
        }
    }
}

/// A message was edited.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEditedEvent {
    pub message: Message,
    pub previous_content: String,
}

impl MessageEditedEvent {
    pub fn new(message: Message, previous_content: String) -> Self {
        Self {
            message,
            previous_content,
        }
    }

    pub fn target(&self) -> EventTarget {
        match &self.message.target {
            MessageTarget::Room { room_id } => EventTarget::Room(*room_id),
            MessageTarget::DirectMessage { recipient } => EventTarget::DirectMessage {
                user1: self.message.author,
                user2: *recipient,
            },
        }
    }
}

/// A message was deleted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeletedEvent {
    pub message_id: MessageId,
    pub target: MessageTarget,
    pub deleted_by: UserId,
}

impl MessageDeletedEvent {
    pub fn new(message_id: MessageId, target: MessageTarget, deleted_by: UserId) -> Self {
        Self {
            message_id,
            target,
            deleted_by,
        }
    }

    pub fn target(&self) -> EventTarget {
        match &self.target {
            MessageTarget::Room { room_id } => EventTarget::Room(*room_id),
            MessageTarget::DirectMessage { recipient } => EventTarget::DirectMessage {
                user1: self.deleted_by,
                user2: *recipient,
            },
        }
    }
}

// ============================================================================
// Room Events
// ============================================================================

/// A user joined a room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserJoinedRoomEvent {
    pub room_id: RoomId,
    pub user: User,
    pub membership: RoomMembership,
}

impl UserJoinedRoomEvent {
    pub fn new(room_id: RoomId, user: User, membership: RoomMembership) -> Self {
        Self {
            room_id,
            user,
            membership,
        }
    }
}

/// A user left a room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLeftRoomEvent {
    pub room_id: RoomId,
    pub user_id: UserId,
    pub reason: LeaveReason,
}

impl UserLeftRoomEvent {
    pub fn new(room_id: RoomId, user_id: UserId, reason: LeaveReason) -> Self {
        Self {
            room_id,
            user_id,
            reason,
        }
    }

    pub fn voluntary(room_id: RoomId, user_id: UserId) -> Self {
        Self::new(room_id, user_id, LeaveReason::Voluntary)
    }

    pub fn kicked(room_id: RoomId, user_id: UserId, by: UserId) -> Self {
        Self::new(room_id, user_id, LeaveReason::Kicked { by })
    }

    pub fn banned(room_id: RoomId, user_id: UserId, by: UserId) -> Self {
        Self::new(room_id, user_id, LeaveReason::Banned { by })
    }
}

/// Reason for leaving a room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LeaveReason {
    /// User chose to leave.
    Voluntary,
    /// User was kicked by moderator.
    Kicked { by: UserId },
    /// User was banned from room.
    Banned { by: UserId },
    /// Room was deleted.
    RoomDeleted,
}

impl Display for LeaveReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LeaveReason::Voluntary => write!(f, "left"),
            LeaveReason::Kicked { .. } => write!(f, "kicked"),
            LeaveReason::Banned { .. } => write!(f, "banned"),
            LeaveReason::RoomDeleted => write!(f, "room deleted"),
        }
    }
}

/// Room settings or details changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomUpdatedEvent {
    pub room: Room,
    pub changed_by: UserId,
    pub changes: Vec<RoomChange>,
}

impl RoomUpdatedEvent {
    pub fn new(room: Room, changed_by: UserId, changes: Vec<RoomChange>) -> Self {
        Self {
            room,
            changed_by,
            changes,
        }
    }
}

/// A specific change to a room.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "field", rename_all = "snake_case")]
pub enum RoomChange {
    Name {
        old: String,
        new: String,
    },
    Description {
        old: Option<String>,
        new: Option<String>,
    },
    Public {
        old: bool,
        new: bool,
    },
    MaxMembers {
        old: Option<u32>,
        new: Option<u32>,
    },
    Moderated {
        old: bool,
        new: bool,
    },
}

/// A room was deleted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDeletedEvent {
    pub room_id: RoomId,
    pub room_name: String,
    pub deleted_by: UserId,
}

impl RoomDeletedEvent {
    pub fn new(room_id: RoomId, room_name: String, deleted_by: UserId) -> Self {
        Self {
            room_id,
            room_name,
            deleted_by,
        }
    }
}

// ============================================================================
// User Presence Events
// ============================================================================

/// A user came online.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOnlineEvent {
    pub user_id: UserId,
    pub username: String,
}

impl UserOnlineEvent {
    pub fn new(user_id: UserId, username: String) -> Self {
        Self { user_id, username }
    }
}

/// A user went offline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOfflineEvent {
    pub user_id: UserId,
    pub username: String,
}

impl UserOfflineEvent {
    pub fn new(user_id: UserId, username: String) -> Self {
        Self { user_id, username }
    }
}

/// A user is typing a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTypingEvent {
    pub user_id: UserId,
    pub target: MessageTarget,
}

impl UserTypingEvent {
    pub fn new(user_id: UserId, target: MessageTarget) -> Self {
        Self { user_id, target }
    }

    pub fn target(&self) -> EventTarget {
        match &self.target {
            MessageTarget::Room { room_id } => EventTarget::Room(*room_id),
            MessageTarget::DirectMessage { recipient } => EventTarget::DirectMessage {
                user1: self.user_id,
                user2: *recipient,
            },
        }
    }
}

// ============================================================================
// Invitation Events
// ============================================================================

/// User received a room invitation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationReceivedEvent {
    pub invitation: Invitation,
    pub room: Room,
    pub inviter: User,
}

impl InvitationReceivedEvent {
    pub fn new(invitation: Invitation, room: Room, inviter: User) -> Self {
        Self {
            invitation,
            room,
            inviter,
        }
    }
}

/// An invitation was cancelled or expired.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationCancelledEvent {
    pub invitation_id: InvitationId,
    pub invitee: UserId,
    pub reason: CancelReason,
}

impl InvitationCancelledEvent {
    pub fn new(invitation_id: InvitationId, invitee: UserId, reason: CancelReason) -> Self {
        Self {
            invitation_id,
            invitee,
            reason,
        }
    }
}

/// Reason for invitation cancellation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelReason {
    /// Inviter cancelled.
    CancelledByInviter,
    /// Invitation expired.
    Expired,
    /// Room was deleted.
    RoomDeleted,
}

impl Display for CancelReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CancelReason::CancelledByInviter => write!(f, "cancelled by inviter"),
            CancelReason::Expired => write!(f, "expired"),
            CancelReason::RoomDeleted => write!(f, "room deleted"),
        }
    }
}

// ============================================================================
// System Events
// ============================================================================

/// System-wide announcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerNoticeEvent {
    pub message: String,
    pub severity: NoticeSeverity,
}

impl ServerNoticeEvent {
    pub fn new(message: String, severity: NoticeSeverity) -> Self {
        Self { message, severity }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message.into(), NoticeSeverity::Info)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message.into(), NoticeSeverity::Warning)
    }

    pub fn critical(message: impl Into<String>) -> Self {
        Self::new(message.into(), NoticeSeverity::Critical)
    }
}

/// Severity level for server notices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoticeSeverity {
    Info,
    Warning,
    Critical,
}

impl Display for NoticeSeverity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            NoticeSeverity::Info => write!(f, "info"),
            NoticeSeverity::Warning => write!(f, "warning"),
            NoticeSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// User's session is about to expire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExpiringEvent {
    pub session_id: SessionId,
    pub expires_at: DateTime<Utc>,
}

impl SessionExpiringEvent {
    pub fn new(session_id: SessionId, expires_at: DateTime<Utc>) -> Self {
        Self {
            session_id,
            expires_at,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Email, MessageContent, Username};

    #[test]
    fn test_event_id() {
        let id1 = EventId::new();
        let id2 = EventId::new();
        assert_ne!(id1, id2);

        let parsed = EventId::parse(&id1.to_string()).unwrap();
        assert_eq!(id1, parsed);
    }

    #[test]
    fn test_event_creation() {
        let user_id = UserId::new();
        let payload = EventPayload::UserOnline(UserOnlineEvent::new(user_id, "alice".to_string()));
        let event = Event::new(payload);

        assert_eq!(event.event_type(), "user_online");
        assert!(matches!(event.target(), EventTarget::UserConnections(_)));
    }

    #[test]
    fn test_message_received_event_target_room() {
        let author = UserId::new();
        let room_id = RoomId::new();
        let content = MessageContent::new("Hello").unwrap();
        let message = Message::to_room(author, room_id, content);
        let event = MessageReceivedEvent::new(message);

        assert!(matches!(event.target(), EventTarget::Room(id) if id == room_id));
    }

    #[test]
    fn test_message_received_event_target_dm() {
        let author = UserId::new();
        let recipient = UserId::new();
        let content = MessageContent::new("Hello").unwrap();
        let message = Message::to_user(author, recipient, content);
        let event = MessageReceivedEvent::new(message);

        match event.target() {
            EventTarget::DirectMessage { user1, user2 } => {
                assert_eq!(user1, author);
                assert_eq!(user2, recipient);
            }
            _ => panic!("Expected DirectMessage target"),
        }
    }

    #[test]
    fn test_leave_reason_display() {
        assert_eq!(LeaveReason::Voluntary.to_string(), "left");
        assert_eq!(
            LeaveReason::Kicked { by: UserId::new() }.to_string(),
            "kicked"
        );
        assert_eq!(
            LeaveReason::Banned { by: UserId::new() }.to_string(),
            "banned"
        );
        assert_eq!(LeaveReason::RoomDeleted.to_string(), "room deleted");
    }

    #[test]
    fn test_server_notice_constructors() {
        let info = ServerNoticeEvent::info("Test info");
        assert_eq!(info.severity, NoticeSeverity::Info);

        let warning = ServerNoticeEvent::warning("Test warning");
        assert_eq!(warning.severity, NoticeSeverity::Warning);

        let critical = ServerNoticeEvent::critical("Test critical");
        assert_eq!(critical.severity, NoticeSeverity::Critical);
    }

    #[test]
    fn test_event_payload_serialization() {
        let payload =
            EventPayload::UserOnline(UserOnlineEvent::new(UserId::new(), "alice".to_string()));
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"type\":\"user_online\""));
    }

    #[test]
    fn test_user_joined_room_event() {
        let room_id = RoomId::new();
        let user_id = UserId::new();
        let username = Username::new("alice").unwrap();
        let email = Email::new("alice@example.com").unwrap();
        let user = User::new(username, email);
        let membership = RoomMembership::new(room_id, user_id);

        let event = UserJoinedRoomEvent::new(room_id, user, membership);
        assert_eq!(event.room_id, room_id);
    }

    #[test]
    fn test_user_left_room_event_constructors() {
        let room_id = RoomId::new();
        let user_id = UserId::new();
        let moderator = UserId::new();

        let voluntary = UserLeftRoomEvent::voluntary(room_id, user_id);
        assert!(matches!(voluntary.reason, LeaveReason::Voluntary));

        let kicked = UserLeftRoomEvent::kicked(room_id, user_id, moderator);
        assert!(matches!(kicked.reason, LeaveReason::Kicked { by } if by == moderator));

        let banned = UserLeftRoomEvent::banned(room_id, user_id, moderator);
        assert!(matches!(banned.reason, LeaveReason::Banned { by } if by == moderator));
    }
}
