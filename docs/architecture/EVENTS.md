# Events

This document defines all real-time events that the Lair Chat system broadcasts to connected clients. Events enable clients to receive updates without polling.

## Design Principles

1. **Events are Notifications**: Events inform about state changes, they don't request actions
2. **Targeted Delivery**: Each event specifies who should receive it
3. **Idempotent Handling**: Receiving an event multiple times should be safe
4. **Self-Contained**: Events include all data needed to process them

---

## Event Structure

All events follow this structure:

```rust
pub struct Event {
    /// Unique event ID (for deduplication)
    pub id: EventId,
    
    /// Event type and payload
    pub payload: EventPayload,
    
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
}

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
    
    // User events
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
```

---

## Event Delivery

### Delivery Targets

Events are delivered to specific audiences:

```rust
pub enum EventTarget {
    /// Send to a specific user (all their sessions)
    User(UserId),
    
    /// Send to all members of a room
    Room(RoomId),
    
    /// Send to participants in a DM conversation
    DirectMessage { user1: UserId, user2: UserId },
    
    /// Send to all connected clients (rare, admin only)
    Broadcast,
}
```

### Delivery Guarantees

- **TCP**: Events pushed immediately over persistent connection
- **WebSocket**: Events pushed immediately
- **HTTP**: Clients must poll or use long-polling (no push)

### Event Ordering

Events within a single target (room, DM conversation) are guaranteed to be delivered in order. Events across different targets may arrive out of order.

---

## Message Events

### MessageReceived

A new message was sent.

```rust
pub struct MessageReceivedEvent {
    pub message: Message,
}
```

**Trigger**: `SendMessage` command succeeds

**Recipients**: 
- Room message: All members of the room
- DM: Both participants in the conversation

**Client Action**: Add message to chat history, update unread count

**Example Payload (JSON)**:
```json
{
  "type": "message_received",
  "message": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "author": "123e4567-e89b-12d3-a456-426614174000",
    "target": {
      "type": "room",
      "room_id": "789e0123-e45b-67c8-d901-234567890abc"
    },
    "content": "Hello everyone!",
    "edited": false,
    "created_at": "2025-01-21T12:00:00Z"
  },
  "timestamp": "2025-01-21T12:00:00Z"
}
```

---

### MessageEdited

A message was edited.

```rust
pub struct MessageEditedEvent {
    pub message: Message,
    pub previous_content: String,  // For audit/display purposes
}
```

**Trigger**: `EditMessage` command succeeds

**Recipients**: Same as original message target

**Client Action**: Update message in chat history, optionally show edit indicator

**Example Payload (JSON)**:
```json
{
  "type": "message_edited",
  "message": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "author": "123e4567-e89b-12d3-a456-426614174000",
    "target": {
      "type": "room",
      "room_id": "789e0123-e45b-67c8-d901-234567890abc"
    },
    "content": "Hello everyone! (edited)",
    "edited": true,
    "created_at": "2025-01-21T12:00:00Z",
    "edited_at": "2025-01-21T12:05:00Z"
  },
  "previous_content": "Hello everyone!",
  "timestamp": "2025-01-21T12:05:00Z"
}
```

---

### MessageDeleted

A message was deleted.

```rust
pub struct MessageDeletedEvent {
    pub message_id: MessageId,
    pub target: MessageTarget,
    pub deleted_by: UserId,       // Who deleted it (author or moderator)
}
```

**Trigger**: `DeleteMessage` command succeeds

**Recipients**: Same as original message target

**Client Action**: Remove message from chat history or show "[deleted]" placeholder

**Example Payload (JSON)**:
```json
{
  "type": "message_deleted",
  "message_id": "550e8400-e29b-41d4-a716-446655440000",
  "target": {
    "type": "room",
    "room_id": "789e0123-e45b-67c8-d901-234567890abc"
  },
  "deleted_by": "123e4567-e89b-12d3-a456-426614174000",
  "timestamp": "2025-01-21T12:10:00Z"
}
```

---

## Room Events

### UserJoinedRoom

A user joined a room.

```rust
pub struct UserJoinedRoomEvent {
    pub room_id: RoomId,
    pub user: User,
    pub membership: RoomMembership,
}
```

**Trigger**: 
- `JoinRoom` command succeeds
- `AcceptInvitation` command succeeds

**Recipients**: All members of the room (including the joining user)

**Client Action**: Add user to room member list, optionally show join message

**Example Payload (JSON)**:
```json
{
  "type": "user_joined_room",
  "room_id": "789e0123-e45b-67c8-d901-234567890abc",
  "user": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "username": "alice",
    "role": "user",
    "created_at": "2025-01-01T00:00:00Z"
  },
  "membership": {
    "room_id": "789e0123-e45b-67c8-d901-234567890abc",
    "user_id": "123e4567-e89b-12d3-a456-426614174000",
    "room_role": "member",
    "joined_at": "2025-01-21T12:00:00Z"
  },
  "timestamp": "2025-01-21T12:00:00Z"
}
```

---

### UserLeftRoom

A user left a room.

```rust
pub struct UserLeftRoomEvent {
    pub room_id: RoomId,
    pub user_id: UserId,
    pub reason: LeaveReason,
}

pub enum LeaveReason {
    /// User chose to leave
    Voluntary,
    /// User was kicked by moderator
    Kicked { by: UserId },
    /// User was banned from room
    Banned { by: UserId },
    /// Room was deleted
    RoomDeleted,
}
```

**Trigger**: 
- `LeaveRoom` command succeeds
- User kicked/banned by moderator
- Room deleted

**Recipients**: All members of the room (excluding the leaving user, unless kicked/banned)

**Client Action**: Remove user from room member list, optionally show leave message

**Example Payload (JSON)**:
```json
{
  "type": "user_left_room",
  "room_id": "789e0123-e45b-67c8-d901-234567890abc",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "reason": "voluntary",
  "timestamp": "2025-01-21T12:00:00Z"
}
```

---

### RoomUpdated

Room settings or details changed.

```rust
pub struct RoomUpdatedEvent {
    pub room: Room,
    pub changed_by: UserId,
    pub changes: Vec<RoomChange>,
}

pub enum RoomChange {
    Name { old: String, new: String },
    Description { old: Option<String>, new: Option<String> },
    Settings { field: String },
}
```

**Trigger**: `UpdateRoom` command succeeds

**Recipients**: All members of the room

**Client Action**: Update room details in UI

**Example Payload (JSON)**:
```json
{
  "type": "room_updated",
  "room": {
    "id": "789e0123-e45b-67c8-d901-234567890abc",
    "name": "General Chat (Renamed)",
    "description": "A place for general discussion",
    "owner": "123e4567-e89b-12d3-a456-426614174000",
    "settings": {
      "public": true,
      "max_members": null,
      "moderated": false,
      "join_role": "user"
    },
    "created_at": "2025-01-01T00:00:00Z"
  },
  "changed_by": "123e4567-e89b-12d3-a456-426614174000",
  "changes": [
    { "name": { "old": "General Chat", "new": "General Chat (Renamed)" } }
  ],
  "timestamp": "2025-01-21T12:00:00Z"
}
```

---

### RoomDeleted

A room was deleted.

```rust
pub struct RoomDeletedEvent {
    pub room_id: RoomId,
    pub room_name: String,  // For display purposes
    pub deleted_by: UserId,
}
```

**Trigger**: `DeleteRoom` or `AdminDeleteRoom` command succeeds

**Recipients**: All members of the room

**Client Action**: Remove room from list, show notification, clean up local state

**Example Payload (JSON)**:
```json
{
  "type": "room_deleted",
  "room_id": "789e0123-e45b-67c8-d901-234567890abc",
  "room_name": "Old Room",
  "deleted_by": "123e4567-e89b-12d3-a456-426614174000",
  "timestamp": "2025-01-21T12:00:00Z"
}
```

---

## User Presence Events

### UserOnline

A user came online.

```rust
pub struct UserOnlineEvent {
    pub user_id: UserId,
    pub username: String,
}
```

**Trigger**: User establishes their first active connection

**Recipients**: 
- All users who share a room with this user
- All users who have a DM conversation with this user

**Client Action**: Update user's online status in UI

**Example Payload (JSON)**:
```json
{
  "type": "user_online",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "username": "alice",
  "timestamp": "2025-01-21T12:00:00Z"
}
```

---

### UserOffline

A user went offline.

```rust
pub struct UserOfflineEvent {
    pub user_id: UserId,
    pub username: String,
}
```

**Trigger**: User's last active connection closes

**Recipients**: Same as UserOnline

**Client Action**: Update user's online status in UI

**Example Payload (JSON)**:
```json
{
  "type": "user_offline",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "username": "alice",
  "timestamp": "2025-01-21T12:00:00Z"
}
```

---

### UserTyping

A user is typing a message.

```rust
pub struct UserTypingEvent {
    pub user_id: UserId,
    pub target: MessageTarget,
}
```

**Trigger**: Client sends typing indicator

**Recipients**: 
- Room: All other members
- DM: The other participant

**Client Action**: Show typing indicator, auto-dismiss after timeout

**Notes**:
- Typing events are transient - not persisted
- Clients should send at most once per 3 seconds
- Clients should auto-dismiss after 5 seconds without refresh

**Example Payload (JSON)**:
```json
{
  "type": "user_typing",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "target": {
    "type": "room",
    "room_id": "789e0123-e45b-67c8-d901-234567890abc"
  },
  "timestamp": "2025-01-21T12:00:00Z"
}
```

---

## Invitation Events

### InvitationReceived

User received a room invitation.

```rust
pub struct InvitationReceivedEvent {
    pub invitation: Invitation,
    pub room: Room,
    pub inviter: User,
}
```

**Trigger**: `InviteToRoom` command succeeds

**Recipients**: The invited user only

**Client Action**: Show notification, add to pending invitations list

**Example Payload (JSON)**:
```json
{
  "type": "invitation_received",
  "invitation": {
    "id": "aaa11111-bbbb-cccc-dddd-eeeeeeeeeeee",
    "room_id": "789e0123-e45b-67c8-d901-234567890abc",
    "inviter": "123e4567-e89b-12d3-a456-426614174000",
    "invitee": "456e7890-e12b-34c5-d678-901234567890",
    "status": "pending",
    "created_at": "2025-01-21T12:00:00Z",
    "expires_at": "2025-01-28T12:00:00Z"
  },
  "room": {
    "id": "789e0123-e45b-67c8-d901-234567890abc",
    "name": "Secret Club",
    "description": "Members only",
    "owner": "123e4567-e89b-12d3-a456-426614174000",
    "settings": { "public": false },
    "created_at": "2025-01-01T00:00:00Z"
  },
  "inviter": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "username": "alice",
    "role": "user"
  },
  "timestamp": "2025-01-21T12:00:00Z"
}
```

---

### InvitationCancelled

An invitation was cancelled or expired.

```rust
pub struct InvitationCancelledEvent {
    pub invitation_id: InvitationId,
    pub reason: CancelReason,
}

pub enum CancelReason {
    /// Inviter cancelled
    CancelledByInviter,
    /// Invitation expired
    Expired,
    /// Room was deleted
    RoomDeleted,
}
```

**Trigger**: 
- Inviter cancels invitation
- Invitation expires
- Room is deleted

**Recipients**: The invited user

**Client Action**: Remove from pending invitations, optionally show notification

---

## System Events

### ServerNotice

System-wide announcement.

```rust
pub struct ServerNoticeEvent {
    pub message: String,
    pub severity: NoticeSeverity,
}

pub enum NoticeSeverity {
    Info,
    Warning,
    Critical,
}
```

**Trigger**: Admin action or system event (maintenance, etc.)

**Recipients**: All connected users (Broadcast)

**Client Action**: Display prominent notification

**Example Payload (JSON)**:
```json
{
  "type": "server_notice",
  "message": "Server will restart for maintenance in 10 minutes",
  "severity": "warning",
  "timestamp": "2025-01-21T12:00:00Z"
}
```

---

### SessionExpiring

User's session is about to expire.

```rust
pub struct SessionExpiringEvent {
    pub session_id: SessionId,
    pub expires_at: DateTime<Utc>,
}
```

**Trigger**: Session approaching expiration (e.g., 5 minutes before)

**Recipients**: The specific user session

**Client Action**: Prompt user to refresh token or re-authenticate

**Example Payload (JSON)**:
```json
{
  "type": "session_expiring",
  "session_id": "fff00000-1111-2222-3333-444444444444",
  "expires_at": "2025-01-21T12:05:00Z",
  "timestamp": "2025-01-21T12:00:00Z"
}
```

---

## Event Subscription

### Automatic Subscriptions

Clients are automatically subscribed to relevant events:

| Event Type | Auto-subscribed When |
|------------|---------------------|
| MessageReceived (room) | User is room member |
| MessageReceived (DM) | User is DM participant |
| UserJoinedRoom | User is room member |
| UserLeftRoom | User is room member |
| RoomUpdated | User is room member |
| RoomDeleted | User is room member |
| UserOnline/Offline | User shares room or DM |
| UserTyping | User is in target room/DM |
| InvitationReceived | User is invitee |
| ServerNotice | Always |
| SessionExpiring | Always (own sessions) |

### Manual Subscriptions (Future)

In future versions, clients may be able to:
- Subscribe to specific rooms without joining
- Subscribe to specific user's presence
- Unsubscribe from typing events

---

## Event Deduplication

Events include a unique `EventId` to support deduplication:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventId(Uuid);
```

Clients should track recently received event IDs and ignore duplicates. This is important for:
- Network reconnection scenarios
- Multi-device scenarios
- At-least-once delivery guarantees

Recommended: Keep last 1000 event IDs or events from last 5 minutes.

---

## Wire Format

Events are serialized as JSON with a consistent structure:

```json
{
  "id": "event-uuid",
  "type": "event_type_name",
  "timestamp": "2025-01-21T12:00:00Z",
  // ... type-specific fields
}
```

The `type` field uses snake_case and maps to the event variant name.

---

## Event Summary Table

| Event | Trigger | Recipients | Real-time Critical |
|-------|---------|------------|-------------------|
| MessageReceived | SendMessage | Room/DM participants | Yes |
| MessageEdited | EditMessage | Room/DM participants | Yes |
| MessageDeleted | DeleteMessage | Room/DM participants | Yes |
| UserJoinedRoom | JoinRoom, AcceptInvitation | Room members | Medium |
| UserLeftRoom | LeaveRoom, Kick, Ban | Room members | Medium |
| RoomUpdated | UpdateRoom | Room members | Low |
| RoomDeleted | DeleteRoom | Room members | High |
| UserOnline | First connection | Shared room/DM users | Medium |
| UserOffline | Last disconnect | Shared room/DM users | Medium |
| UserTyping | Client indication | Room/DM participants | Low |
| InvitationReceived | InviteToRoom | Invitee | Medium |
| InvitationCancelled | Cancel/Expire | Invitee | Low |
| ServerNotice | Admin action | All users | High |
| SessionExpiring | Near expiration | Session owner | High |
