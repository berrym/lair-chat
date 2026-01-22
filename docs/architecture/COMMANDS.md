# Commands

This document defines all operations the Lair Chat system supports. Commands represent actions that users or systems can perform, and each command has well-defined inputs, outputs, preconditions, and error cases.

## Design Principles

1. **Commands are Data**: Each command is a self-contained request that can be logged, queued, or replayed
2. **Explicit Preconditions**: Every command lists what must be true before execution
3. **Defined Error Cases**: All possible failures are documented
4. **Protocol-Agnostic**: Commands are defined independently of wire format

---

## Command Structure

All commands follow this pattern:

```rust
pub enum Command {
    // Authentication
    Register { ... },
    Login { ... },
    Logout { ... },
    
    // Messaging
    SendMessage { ... },
    EditMessage { ... },
    DeleteMessage { ... },
    GetMessages { ... },
    
    // Rooms
    CreateRoom { ... },
    JoinRoom { ... },
    LeaveRoom { ... },
    // ...
    
    // Users
    GetUser { ... },
    UpdateProfile { ... },
    ListUsers { ... },
    
    // Invitations
    InviteToRoom { ... },
    AcceptInvitation { ... },
    DeclineInvitation { ... },
    
    // Admin
    GetStats { ... },
    BanUser { ... },
    // ...
}
```

---

## Authentication Commands

### Register

Create a new user account.

**Input:**
```rust
Register {
    username: String,    // 3-32 chars, alphanumeric + underscore
    email: String,       // Valid email format
    password: String,    // Minimum 8 chars, complexity requirements
}
```

**Output:**
```rust
RegisterResponse {
    user: User,          // The created user (without password)
    session: Session,    // Auto-logged in session
    token: String,       // JWT token for the session
}
```

**Preconditions:**
- None (unauthenticated command)

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `UsernameInvalid` | Username doesn't meet format requirements |
| `UsernameTaken` | Username already exists (case-insensitive) |
| `EmailInvalid` | Email doesn't meet format requirements |
| `EmailTaken` | Email already registered |
| `PasswordTooWeak` | Password doesn't meet complexity requirements |

**Side Effects:**
- User record created in database
- Session created
- User can immediately authenticate

---

### Login

Authenticate and create a session.

**Input:**
```rust
Login {
    identifier: String,  // Username or email
    password: String,
}
```

**Output:**
```rust
LoginResponse {
    user: User,
    session: Session,
    token: String,       // JWT token
}
```

**Preconditions:**
- None (unauthenticated command)

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `InvalidCredentials` | Username/email not found OR password incorrect |
| `AccountLocked` | Too many failed attempts |
| `AccountBanned` | User has been banned |

**Side Effects:**
- Session created
- Failed attempts tracked (for lockout)

**Security Notes:**
- Error message is intentionally vague ("invalid credentials") to prevent username enumeration
- Implements rate limiting to prevent brute force

---

### Logout

End the current session.

**Input:**
```rust
Logout {
    session: SessionId,
}
```

**Output:**
```rust
LogoutResponse {
    success: bool,
}
```

**Preconditions:**
- Valid session

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `SessionNotFound` | Session doesn't exist or already expired |

**Side Effects:**
- Session invalidated
- Any active connections for this session are closed

---

### RefreshToken

Get a new JWT token before the current one expires.

**Input:**
```rust
RefreshToken {
    session: SessionId,
}
```

**Output:**
```rust
RefreshTokenResponse {
    token: String,       // New JWT token
    expires_at: DateTime<Utc>,
}
```

**Preconditions:**
- Valid, non-expired session

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `SessionNotFound` | Session doesn't exist |
| `SessionExpired` | Session has expired |

---

## Messaging Commands

### SendMessage

Send a message to a room or user.

**Input:**
```rust
SendMessage {
    session: SessionId,
    target: MessageTarget,  // Room { room_id } or DirectMessage { recipient }
    content: String,        // 1-4096 chars
}
```

**Output:**
```rust
SendMessageResponse {
    message: Message,    // The created message with ID and timestamp
}
```

**Preconditions:**
- Valid session
- If room message: user is member of room
- If DM: recipient exists and hasn't blocked sender

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `ContentEmpty` | Message content is empty/whitespace |
| `ContentTooLong` | Message exceeds 4096 characters |
| `RoomNotFound` | Target room doesn't exist |
| `NotRoomMember` | User is not a member of the room |
| `RoomModerated` | Room is moderated and user can't post |
| `UserNotFound` | DM recipient doesn't exist |
| `UserBlocked` | Recipient has blocked sender |
| `RateLimited` | Too many messages sent recently |

**Side Effects:**
- Message stored in database
- `MessageReceived` event broadcast to room members or DM recipient
- Unread count incremented for recipients

---

### EditMessage

Edit a previously sent message.

**Input:**
```rust
EditMessage {
    session: SessionId,
    message_id: MessageId,
    content: String,
}
```

**Output:**
```rust
EditMessageResponse {
    message: Message,    // Updated message
}
```

**Preconditions:**
- Valid session
- User is the message author
- Message exists and isn't deleted

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `MessageNotFound` | Message doesn't exist |
| `NotMessageAuthor` | User didn't write this message |
| `MessageDeleted` | Message was already deleted |
| `ContentEmpty` | New content is empty |
| `ContentTooLong` | New content exceeds limit |

**Side Effects:**
- Message content updated
- `edited` flag set to true
- `edited_at` timestamp updated
- `MessageEdited` event broadcast

---

### DeleteMessage

Delete a message.

**Input:**
```rust
DeleteMessage {
    session: SessionId,
    message_id: MessageId,
}
```

**Output:**
```rust
DeleteMessageResponse {
    success: bool,
}
```

**Preconditions:**
- Valid session
- User is message author OR room moderator/admin OR system admin

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `MessageNotFound` | Message doesn't exist |
| `PermissionDenied` | User can't delete this message |

**Side Effects:**
- Message soft-deleted (content replaced, marked as deleted)
- `MessageDeleted` event broadcast

---

### GetMessages

Retrieve message history.

**Input:**
```rust
GetMessages {
    session: SessionId,
    target: MessageTarget,    // Room or DM conversation
    pagination: Pagination,   // offset, limit
    before: Option<DateTime<Utc>>,  // Messages before this time
    after: Option<DateTime<Utc>>,   // Messages after this time
}
```

**Output:**
```rust
GetMessagesResponse {
    messages: Vec<Message>,
    has_more: bool,          // More messages available
    total_count: u32,        // Total messages matching query
}
```

**Preconditions:**
- Valid session
- If room: user is member
- If DM: user is participant

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `RoomNotFound` | Room doesn't exist |
| `NotRoomMember` | User not in room |
| `UserNotFound` | DM partner doesn't exist |

---

### MarkMessagesRead

Mark messages as read (for unread tracking).

**Input:**
```rust
MarkMessagesRead {
    session: SessionId,
    target: MessageTarget,
    up_to: MessageId,        // Mark all messages up to this one as read
}
```

**Output:**
```rust
MarkMessagesReadResponse {
    unread_count: u32,       // Remaining unread in this target
}
```

**Preconditions:**
- Valid session
- User is participant in target

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `MessageNotFound` | Specified message doesn't exist |

---

## Room Commands

### CreateRoom

Create a new chat room.

**Input:**
```rust
CreateRoom {
    session: SessionId,
    name: String,            // 1-64 chars
    description: Option<String>,
    settings: Option<RoomSettings>,
}
```

**Output:**
```rust
CreateRoomResponse {
    room: Room,
}
```

**Preconditions:**
- Valid session

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `NameInvalid` | Name doesn't meet requirements |
| `NameTaken` | Room name already exists |
| `RoomLimitReached` | User has created too many rooms |

**Side Effects:**
- Room created
- Creator automatically added as owner

---

### JoinRoom

Join a room.

**Input:**
```rust
JoinRoom {
    session: SessionId,
    room_id: RoomId,
}
```

**Output:**
```rust
JoinRoomResponse {
    room: Room,
    membership: RoomMembership,
}
```

**Preconditions:**
- Valid session
- Room is public OR user has invitation

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `RoomNotFound` | Room doesn't exist |
| `RoomPrivate` | Room requires invitation |
| `AlreadyMember` | User is already in room |
| `RoomFull` | Room has reached max members |
| `Banned` | User is banned from room |

**Side Effects:**
- Membership record created
- `UserJoinedRoom` event broadcast
- Any pending invitation marked as accepted

---

### LeaveRoom

Leave a room.

**Input:**
```rust
LeaveRoom {
    session: SessionId,
    room_id: RoomId,
}
```

**Output:**
```rust
LeaveRoomResponse {
    success: bool,
}
```

**Preconditions:**
- Valid session
- User is member of room
- User is not the only owner (must transfer ownership first)

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `RoomNotFound` | Room doesn't exist |
| `NotRoomMember` | User isn't in this room |
| `LastOwner` | Can't leave as only owner |

**Side Effects:**
- Membership deleted
- `UserLeftRoom` event broadcast

---

### GetRoom

Get room details.

**Input:**
```rust
GetRoom {
    session: SessionId,
    room_id: RoomId,
}
```

**Output:**
```rust
GetRoomResponse {
    room: Room,
    membership: Option<RoomMembership>,  // If user is member
    member_count: u32,
}
```

**Preconditions:**
- Valid session
- Room is public OR user is member

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `RoomNotFound` | Room doesn't exist |
| `PermissionDenied` | Private room and not member |

---

### ListRooms

List available rooms.

**Input:**
```rust
ListRooms {
    session: SessionId,
    filter: RoomFilter,
    pagination: Pagination,
}

pub struct RoomFilter {
    pub search: Option<String>,      // Search name/description
    pub joined_only: bool,           // Only rooms user is in
    pub public_only: bool,           // Only public rooms
}
```

**Output:**
```rust
ListRoomsResponse {
    rooms: Vec<RoomSummary>,
    has_more: bool,
    total_count: u32,
}

pub struct RoomSummary {
    pub room: Room,
    pub member_count: u32,
    pub is_member: bool,
}
```

---

### UpdateRoom

Update room settings (owner/moderator only).

**Input:**
```rust
UpdateRoom {
    session: SessionId,
    room_id: RoomId,
    name: Option<String>,
    description: Option<String>,
    settings: Option<RoomSettings>,
}
```

**Output:**
```rust
UpdateRoomResponse {
    room: Room,
}
```

**Preconditions:**
- Valid session
- User is room owner or moderator

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `RoomNotFound` | Room doesn't exist |
| `PermissionDenied` | Not owner/moderator |
| `NameTaken` | New name already exists |

**Side Effects:**
- Room updated
- `RoomUpdated` event broadcast

---

### DeleteRoom

Delete a room (owner only).

**Input:**
```rust
DeleteRoom {
    session: SessionId,
    room_id: RoomId,
}
```

**Output:**
```rust
DeleteRoomResponse {
    success: bool,
}
```

**Preconditions:**
- Valid session
- User is room owner OR system admin

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `RoomNotFound` | Room doesn't exist |
| `PermissionDenied` | Not owner or admin |

**Side Effects:**
- Room and all memberships deleted
- Messages archived (not deleted)
- All members receive `RoomDeleted` event

---

## Invitation Commands

### InviteToRoom

Invite a user to a room.

**Input:**
```rust
InviteToRoom {
    session: SessionId,
    room_id: RoomId,
    user_id: UserId,
    message: Option<String>,  // Optional invitation message
}
```

**Output:**
```rust
InviteToRoomResponse {
    invitation: Invitation,
}
```

**Preconditions:**
- Valid session
- User is member of room
- Target user exists and isn't already a member
- Target hasn't blocked inviter

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `RoomNotFound` | Room doesn't exist |
| `NotRoomMember` | Inviter not in room |
| `UserNotFound` | Target user doesn't exist |
| `AlreadyMember` | Target is already in room |
| `AlreadyInvited` | Pending invitation exists |
| `UserBlocked` | Target blocked inviter |

**Side Effects:**
- Invitation created
- `InvitationReceived` event sent to target

---

### AcceptInvitation

Accept a room invitation.

**Input:**
```rust
AcceptInvitation {
    session: SessionId,
    invitation_id: InvitationId,
}
```

**Output:**
```rust
AcceptInvitationResponse {
    room: Room,
    membership: RoomMembership,
}
```

**Preconditions:**
- Valid session
- User is the invitee
- Invitation is pending and not expired

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `InvitationNotFound` | Invitation doesn't exist |
| `NotInvitee` | User wasn't invited |
| `InvitationExpired` | Invitation has expired |
| `InvitationUsed` | Already accepted/declined |

**Side Effects:**
- Invitation status updated to Accepted
- User added to room
- `UserJoinedRoom` event broadcast

---

### DeclineInvitation

Decline a room invitation.

**Input:**
```rust
DeclineInvitation {
    session: SessionId,
    invitation_id: InvitationId,
}
```

**Output:**
```rust
DeclineInvitationResponse {
    success: bool,
}
```

**Preconditions:**
- Valid session
- User is the invitee
- Invitation is pending

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `InvitationNotFound` | Invitation doesn't exist |
| `NotInvitee` | User wasn't invited |
| `InvitationUsed` | Already accepted/declined |

**Side Effects:**
- Invitation status updated to Declined

---

### ListInvitations

List pending invitations for the current user.

**Input:**
```rust
ListInvitations {
    session: SessionId,
    pagination: Pagination,
}
```

**Output:**
```rust
ListInvitationsResponse {
    invitations: Vec<InvitationDetail>,
    has_more: bool,
}

pub struct InvitationDetail {
    pub invitation: Invitation,
    pub room: Room,
    pub inviter: User,
}
```

---

## User Commands

### GetUser

Get user profile.

**Input:**
```rust
GetUser {
    session: SessionId,
    user_id: UserId,
}
```

**Output:**
```rust
GetUserResponse {
    user: User,
    online: bool,           // Currently connected
}
```

**Preconditions:**
- Valid session

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `UserNotFound` | User doesn't exist |

---

### GetCurrentUser

Get the authenticated user's own profile.

**Input:**
```rust
GetCurrentUser {
    session: SessionId,
}
```

**Output:**
```rust
GetCurrentUserResponse {
    user: User,
    sessions: Vec<Session>,     // All active sessions
    unread_counts: UnreadCounts,
}

pub struct UnreadCounts {
    pub total: u32,
    pub by_room: HashMap<RoomId, u32>,
    pub by_dm: HashMap<UserId, u32>,
}
```

---

### UpdateProfile

Update own profile.

**Input:**
```rust
UpdateProfile {
    session: SessionId,
    email: Option<String>,
    // Note: username changes may be restricted
}
```

**Output:**
```rust
UpdateProfileResponse {
    user: User,
}
```

**Preconditions:**
- Valid session

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `EmailInvalid` | New email invalid format |
| `EmailTaken` | New email already in use |

---

### ChangePassword

Change own password.

**Input:**
```rust
ChangePassword {
    session: SessionId,
    current_password: String,
    new_password: String,
}
```

**Output:**
```rust
ChangePasswordResponse {
    success: bool,
}
```

**Preconditions:**
- Valid session
- Current password is correct

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `IncorrectPassword` | Current password wrong |
| `PasswordTooWeak` | New password doesn't meet requirements |
| `SamePassword` | New password same as current |

**Side Effects:**
- Password updated
- Optionally invalidate other sessions

---

### ListUsers

List users (with optional search).

**Input:**
```rust
ListUsers {
    session: SessionId,
    filter: UserFilter,
    pagination: Pagination,
}

pub struct UserFilter {
    pub search: Option<String>,  // Search username
    pub online_only: bool,
}
```

**Output:**
```rust
ListUsersResponse {
    users: Vec<UserSummary>,
    has_more: bool,
    total_count: u32,
}

pub struct UserSummary {
    pub user: User,
    pub online: bool,
}
```

---

## Admin Commands

### GetStats

Get system statistics.

**Input:**
```rust
GetStats {
    session: SessionId,
}
```

**Output:**
```rust
GetStatsResponse {
    stats: SystemStats,
}

pub struct SystemStats {
    pub users: UserStats,
    pub rooms: RoomStats,
    pub messages: MessageStats,
    pub connections: ConnectionStats,
    pub uptime_seconds: u64,
}

pub struct UserStats {
    pub total: u32,
    pub online: u32,
    pub created_today: u32,
}

pub struct RoomStats {
    pub total: u32,
    pub public: u32,
    pub active: u32,  // Rooms with recent messages
}

pub struct MessageStats {
    pub total: u64,
    pub today: u32,
    pub this_hour: u32,
}

pub struct ConnectionStats {
    pub tcp: u32,
    pub http_sessions: u32,
    pub websocket: u32,
}
```

**Preconditions:**
- Valid session
- User is admin

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `PermissionDenied` | Not admin |

---

### BanUser

Ban a user from the system.

**Input:**
```rust
BanUser {
    session: SessionId,
    user_id: UserId,
    reason: String,
    duration: Option<Duration>,  // None = permanent
}
```

**Output:**
```rust
BanUserResponse {
    success: bool,
    expires_at: Option<DateTime<Utc>>,
}
```

**Preconditions:**
- Valid session
- User is admin
- Target is not an admin (can't ban admins)

**Error Cases:**
| Error | Condition |
|-------|-----------|
| `Unauthorized` | Session invalid |
| `PermissionDenied` | Not admin or target is admin |
| `UserNotFound` | Target doesn't exist |

**Side Effects:**
- User marked as banned
- All user's sessions invalidated
- All user's connections closed
- User cannot log in until unbanned

---

### UnbanUser

Remove a ban.

**Input:**
```rust
UnbanUser {
    session: SessionId,
    user_id: UserId,
}
```

**Output:**
```rust
UnbanUserResponse {
    success: bool,
}
```

---

### AdminDeleteRoom

Admin force-delete a room.

**Input:**
```rust
AdminDeleteRoom {
    session: SessionId,
    room_id: RoomId,
    reason: String,
}
```

**Preconditions:**
- Valid session
- User is admin

---

## Error Response Structure

All command errors follow this structure:

```rust
pub struct CommandError {
    /// Machine-readable error code
    pub code: String,
    
    /// Human-readable message (safe for users)
    pub message: String,
    
    /// Additional context (optional)
    pub details: Option<HashMap<String, Value>>,
}
```

Example error codes:
- `unauthorized` - Session invalid or expired
- `permission_denied` - Action not allowed for user's role
- `not_found` - Requested entity doesn't exist
- `validation_failed` - Input validation failed
- `conflict` - Action conflicts with current state
- `rate_limited` - Too many requests
- `internal_error` - Server error (details hidden from user)

---

## Rate Limiting

Commands are subject to rate limiting:

| Category | Limit |
|----------|-------|
| Authentication | 10 attempts per minute |
| Message sending | 30 messages per minute |
| Room creation | 5 rooms per hour |
| API queries | 100 requests per minute |

Rate limit errors include:
- `retry_after`: Seconds until limit resets
- `limit`: The limit that was exceeded
- `remaining`: Requests remaining (0)
