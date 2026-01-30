//! Domain events for real-time updates.
//!
//! See [EVENTS.md](../../../../docs/architecture/EVENTS.md) for full specification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

use super::{
    EnrichedInvitation, InvitationId, Message, MessageId, MessageTarget, Room, RoomId,
    RoomMembership, RoomRole, SessionId, User, UserId, ValidationError,
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
    MemberRoleChanged(MemberRoleChangedEvent),
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
            EventPayload::MemberRoleChanged(_) => "member_role_changed",
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
            EventPayload::MemberRoleChanged(e) => EventTarget::Room(e.room_id),
            EventPayload::RoomUpdated(e) => EventTarget::Room(e.room.id),
            EventPayload::RoomDeleted(e) => EventTarget::Room(e.room_id),
            EventPayload::UserOnline(e) => EventTarget::UserConnections(e.user_id),
            EventPayload::UserOffline(e) => EventTarget::UserConnections(e.user_id),
            EventPayload::UserTyping(e) => e.target(),
            EventPayload::InvitationReceived(e) => EventTarget::User(e.invitation.invitee_id),
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

/// A member's role was changed in a room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberRoleChangedEvent {
    pub room_id: RoomId,
    pub user_id: UserId,
    pub username: String,
    pub old_role: RoomRole,
    pub new_role: RoomRole,
    pub changed_by: UserId,
}

impl MemberRoleChangedEvent {
    pub fn new(
        room_id: RoomId,
        user_id: UserId,
        username: String,
        old_role: RoomRole,
        new_role: RoomRole,
        changed_by: UserId,
    ) -> Self {
        Self {
            room_id,
            user_id,
            username,
            old_role,
            new_role,
            changed_by,
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
    pub invitation: EnrichedInvitation,
}

impl InvitationReceivedEvent {
    pub fn new(invitation: EnrichedInvitation) -> Self {
        Self { invitation }
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
        use crate::domain::{Role, RoomRole};

        let room_id = RoomId::new();
        let user_id = UserId::new();
        let username = Username::new("alice").unwrap();
        let email = Email::new("alice@example.com").unwrap();
        let user = User::new(username, email, Role::User);
        let membership = RoomMembership::new(room_id, user_id, RoomRole::Member);

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

    #[test]
    fn test_event_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = EventId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn test_event_id_default() {
        let id = EventId::default();
        assert!(!id.as_uuid().is_nil());
    }

    #[test]
    fn test_event_id_from_trait() {
        let uuid = Uuid::new_v4();
        let id: EventId = uuid.into();
        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn test_event_id_parse_invalid() {
        let result = EventId::parse("not-a-uuid");
        assert!(result.is_err());
    }

    #[test]
    fn test_event_id_display() {
        let id = EventId::new();
        let display = format!("{}", id);
        assert!(!display.is_empty());
        // Should be valid UUID format
        assert!(EventId::parse(&display).is_ok());
    }

    #[test]
    fn test_message_edited_event() {
        let author = UserId::new();
        let room_id = RoomId::new();
        let content = MessageContent::new("Edited").unwrap();
        let message = Message::to_room(author, room_id, content);
        let event = MessageEditedEvent::new(message, "Original".to_string());

        assert_eq!(event.previous_content, "Original");
        assert!(matches!(event.target(), EventTarget::Room(id) if id == room_id));
    }

    #[test]
    fn test_message_edited_event_dm_target() {
        let author = UserId::new();
        let recipient = UserId::new();
        let content = MessageContent::new("Edited DM").unwrap();
        let message = Message::to_user(author, recipient, content);
        let event = MessageEditedEvent::new(message, "Original DM".to_string());

        match event.target() {
            EventTarget::DirectMessage { user1, user2 } => {
                assert_eq!(user1, author);
                assert_eq!(user2, recipient);
            }
            _ => panic!("Expected DirectMessage target"),
        }
    }

    #[test]
    fn test_message_deleted_event() {
        let message_id = MessageId::new();
        let room_id = RoomId::new();
        let deleted_by = UserId::new();
        let target = MessageTarget::Room { room_id };
        let event = MessageDeletedEvent::new(message_id, target, deleted_by);

        assert_eq!(event.message_id, message_id);
        assert_eq!(event.deleted_by, deleted_by);
        assert!(matches!(event.target(), EventTarget::Room(id) if id == room_id));
    }

    #[test]
    fn test_message_deleted_event_dm_target() {
        let message_id = MessageId::new();
        let deleted_by = UserId::new();
        let recipient = UserId::new();
        let target = MessageTarget::DirectMessage { recipient };
        let event = MessageDeletedEvent::new(message_id, target, deleted_by);

        match event.target() {
            EventTarget::DirectMessage { user1, user2 } => {
                assert_eq!(user1, deleted_by);
                assert_eq!(user2, recipient);
            }
            _ => panic!("Expected DirectMessage target"),
        }
    }

    #[test]
    fn test_user_typing_event_room() {
        let user_id = UserId::new();
        let room_id = RoomId::new();
        let target = MessageTarget::Room { room_id };
        let event = UserTypingEvent::new(user_id, target);

        assert_eq!(event.user_id, user_id);
        assert!(matches!(event.target(), EventTarget::Room(id) if id == room_id));
    }

    #[test]
    fn test_user_typing_event_dm() {
        let user_id = UserId::new();
        let recipient = UserId::new();
        let target = MessageTarget::DirectMessage { recipient };
        let event = UserTypingEvent::new(user_id, target);

        match event.target() {
            EventTarget::DirectMessage { user1, user2 } => {
                assert_eq!(user1, user_id);
                assert_eq!(user2, recipient);
            }
            _ => panic!("Expected DirectMessage target"),
        }
    }

    #[test]
    fn test_user_online_event() {
        let user_id = UserId::new();
        let event = UserOnlineEvent::new(user_id, "testuser".to_string());
        assert_eq!(event.user_id, user_id);
        assert_eq!(event.username, "testuser");
    }

    #[test]
    fn test_user_offline_event() {
        let user_id = UserId::new();
        let event = UserOfflineEvent::new(user_id, "testuser".to_string());
        assert_eq!(event.user_id, user_id);
        assert_eq!(event.username, "testuser");
    }

    #[test]
    fn test_room_deleted_event() {
        let room_id = RoomId::new();
        let deleted_by = UserId::new();
        let event = RoomDeletedEvent::new(room_id, "testroom".to_string(), deleted_by);

        assert_eq!(event.room_id, room_id);
        assert_eq!(event.room_name, "testroom");
        assert_eq!(event.deleted_by, deleted_by);
    }

    #[test]
    fn test_session_expiring_event() {
        use chrono::Utc;

        let session_id = SessionId::new();
        let expires_at = Utc::now();
        let event = SessionExpiringEvent::new(session_id, expires_at);

        assert_eq!(event.session_id, session_id);
        assert_eq!(event.expires_at, expires_at);
    }

    #[test]
    fn test_cancel_reason_display() {
        assert_eq!(
            CancelReason::CancelledByInviter.to_string(),
            "cancelled by inviter"
        );
        assert_eq!(CancelReason::Expired.to_string(), "expired");
        assert_eq!(CancelReason::RoomDeleted.to_string(), "room deleted");
    }

    #[test]
    fn test_notice_severity_display() {
        assert_eq!(NoticeSeverity::Info.to_string(), "info");
        assert_eq!(NoticeSeverity::Warning.to_string(), "warning");
        assert_eq!(NoticeSeverity::Critical.to_string(), "critical");
    }

    #[test]
    fn test_event_payload_type_names() {
        let user_id = UserId::new();
        let room_id = RoomId::new();
        let session_id = SessionId::new();
        let content = MessageContent::new("test").unwrap();
        let message = Message::to_room(user_id, room_id, content);

        // Test all event types return correct type names
        let events = [
            (
                EventPayload::MessageReceived(MessageReceivedEvent::new(message.clone())),
                "message_received",
            ),
            (
                EventPayload::MessageEdited(MessageEditedEvent::new(
                    message.clone(),
                    "old".to_string(),
                )),
                "message_edited",
            ),
            (
                EventPayload::MessageDeleted(MessageDeletedEvent::new(
                    MessageId::new(),
                    MessageTarget::Room { room_id },
                    user_id,
                )),
                "message_deleted",
            ),
            (
                EventPayload::UserOnline(UserOnlineEvent::new(user_id, "test".to_string())),
                "user_online",
            ),
            (
                EventPayload::UserOffline(UserOfflineEvent::new(user_id, "test".to_string())),
                "user_offline",
            ),
            (
                EventPayload::UserTyping(UserTypingEvent::new(
                    user_id,
                    MessageTarget::Room { room_id },
                )),
                "user_typing",
            ),
            (
                EventPayload::RoomDeleted(RoomDeletedEvent::new(
                    room_id,
                    "room".to_string(),
                    user_id,
                )),
                "room_deleted",
            ),
            (
                EventPayload::ServerNotice(ServerNoticeEvent::info("notice")),
                "server_notice",
            ),
            (
                EventPayload::SessionExpiring(SessionExpiringEvent::new(
                    session_id,
                    chrono::Utc::now(),
                )),
                "session_expiring",
            ),
        ];

        for (payload, expected_type) in events {
            assert_eq!(payload.event_type(), expected_type);
        }
    }

    #[test]
    fn test_event_target_variants() {
        let user_id = UserId::new();
        let room_id = RoomId::new();
        let session_id = SessionId::new();

        // Test all EventTarget variants can be created and compared
        let targets = [
            EventTarget::User(user_id),
            EventTarget::Room(room_id),
            EventTarget::DirectMessage {
                user1: user_id,
                user2: UserId::new(),
            },
            EventTarget::UserConnections(user_id),
            EventTarget::Session(session_id),
            EventTarget::Broadcast,
        ];

        // Test that different targets are not equal
        for i in 0..targets.len() {
            for j in (i + 1)..targets.len() {
                assert_ne!(targets[i], targets[j]);
            }
        }

        // Test equality for same variant types
        let target1 = EventTarget::Room(room_id);
        let target2 = EventTarget::Room(room_id);
        assert_eq!(target1, target2);

        let target3 = EventTarget::Broadcast;
        let target4 = EventTarget::Broadcast;
        assert_eq!(target3, target4);
    }

    #[test]
    fn test_room_change_serialization() {
        let change = RoomChange::Name {
            old: "old_name".to_string(),
            new: "new_name".to_string(),
        };
        let json = serde_json::to_string(&change).unwrap();
        assert!(json.contains("\"field\":\"name\""));

        let change = RoomChange::Public {
            old: false,
            new: true,
        };
        let json = serde_json::to_string(&change).unwrap();
        assert!(json.contains("\"field\":\"public\""));

        let change = RoomChange::Description {
            old: None,
            new: Some("New description".to_string()),
        };
        let json = serde_json::to_string(&change).unwrap();
        assert!(json.contains("\"field\":\"description\""));

        let change = RoomChange::MaxMembers {
            old: Some(10),
            new: Some(20),
        };
        let json = serde_json::to_string(&change).unwrap();
        assert!(json.contains("\"field\":\"max_members\""));

        let change = RoomChange::Moderated {
            old: false,
            new: true,
        };
        let json = serde_json::to_string(&change).unwrap();
        assert!(json.contains("\"field\":\"moderated\""));
    }

    #[test]
    fn test_leave_reason_serialization() {
        let reason = LeaveReason::Voluntary;
        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("\"type\":\"voluntary\""));

        let reason = LeaveReason::Kicked { by: UserId::new() };
        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("\"type\":\"kicked\""));

        let reason = LeaveReason::Banned { by: UserId::new() };
        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("\"type\":\"banned\""));

        let reason = LeaveReason::RoomDeleted;
        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("\"type\":\"room_deleted\""));
    }

    #[test]
    fn test_invitation_cancelled_event() {
        let invitation_id = InvitationId::new();
        let invitee = UserId::new();
        let event = InvitationCancelledEvent::new(invitation_id, invitee, CancelReason::Expired);

        assert_eq!(event.invitation_id, invitation_id);
        assert_eq!(event.invitee, invitee);
        assert_eq!(event.reason, CancelReason::Expired);
    }

    #[test]
    fn test_member_role_changed_event() {
        let room_id = RoomId::new();
        let user_id = UserId::new();
        let changed_by = UserId::new();

        let event = MemberRoleChangedEvent::new(
            room_id,
            user_id,
            "testuser".to_string(),
            RoomRole::Member,
            RoomRole::Moderator,
            changed_by,
        );

        assert_eq!(event.room_id, room_id);
        assert_eq!(event.user_id, user_id);
        assert_eq!(event.username, "testuser");
        assert_eq!(event.old_role, RoomRole::Member);
        assert_eq!(event.new_role, RoomRole::Moderator);
        assert_eq!(event.changed_by, changed_by);
    }

    #[test]
    fn test_member_role_changed_event_target() {
        let room_id = RoomId::new();
        let event = MemberRoleChangedEvent::new(
            room_id,
            UserId::new(),
            "user".to_string(),
            RoomRole::Member,
            RoomRole::Moderator,
            UserId::new(),
        );

        let payload = EventPayload::MemberRoleChanged(event);
        assert_eq!(payload.event_type(), "member_role_changed");
        assert!(matches!(payload.target(), EventTarget::Room(id) if id == room_id));
    }

    #[test]
    fn test_member_role_changed_event_serialization() {
        let event = MemberRoleChangedEvent::new(
            RoomId::new(),
            UserId::new(),
            "testuser".to_string(),
            RoomRole::Member,
            RoomRole::Owner,
            UserId::new(),
        );

        let payload = EventPayload::MemberRoleChanged(event);
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"type\":\"member_role_changed\""));
        assert!(json.contains("\"username\":\"testuser\""));
    }
}
