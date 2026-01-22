# Domain Model

This document defines all core entities in the Lair Chat system. These are the fundamental building blocks that the entire system operates on.

## Design Principles

1. **Pure Types**: Domain types have no I/O dependencies
2. **Validation in Constructors**: Invalid states are unrepresentable
3. **Newtype IDs**: Type-safe identifiers prevent mixing up entity types
4. **Immutable by Default**: Types are immutable unless mutation is explicitly needed

---

## Entity Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              DOMAIN MODEL                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐         ┌─────────────┐         ┌─────────────┐          │
│  │    User     │ owns    │    Room     │ has     │   Message   │          │
│  │             │────────▶│             │────────▶│             │          │
│  │  - id       │         │  - id       │         │  - id       │          │
│  │  - username │ member  │  - name     │         │  - author   │          │
│  │  - email    │◀────────│  - owner    │         │  - target   │          │
│  │  - role     │         │  - members  │         │  - content  │          │
│  └─────────────┘         │  - settings │         │  - edited   │          │
│         │                └─────────────┘         └─────────────┘          │
│         │                       │                       ▲                  │
│         │                       │ invites               │                  │
│         ▼                       ▼                       │                  │
│  ┌─────────────┐         ┌─────────────┐               │                  │
│  │   Session   │         │ Invitation  │               │                  │
│  │             │         │             │               │                  │
│  │  - id       │         │  - id       │               │                  │
│  │  - user     │         │  - room     │───────────────┘                  │
│  │  - protocol │         │  - inviter  │   (DM messages                   │
│  │  - expires  │         │  - invitee  │    target users)                 │
│  └─────────────┘         │  - status   │                                  │
│                          └─────────────┘                                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## ID Types

All entity identifiers use the newtype pattern for type safety.

### UserId

Uniquely identifies a user account.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(Uuid);

impl UserId {
    /// Create a new random UserId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    /// Parse from string representation
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

### RoomId

Uniquely identifies a chat room.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomId(Uuid);
// Same implementation pattern as UserId
```

### MessageId

Uniquely identifies a message.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(Uuid);
// Same implementation pattern as UserId
```

### SessionId

Uniquely identifies an active session.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);
// Same implementation pattern as UserId
```

### InvitationId

Uniquely identifies a room invitation.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InvitationId(Uuid);
// Same implementation pattern as UserId
```

---

## Core Entities

### User

Represents a registered user account.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier
    pub id: UserId,
    
    /// Unique username (3-32 characters, alphanumeric + underscore)
    pub username: Username,
    
    /// Email address (unique, validated format)
    pub email: Email,
    
    /// User's role determining permissions
    pub role: Role,
    
    /// Account creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Last profile update timestamp
    pub updated_at: DateTime<Utc>,
}
```

**Note**: Password hash is NOT part of the User domain type. It's stored separately in the storage layer and never exposed to application code outside of authentication.

#### Username

Validated username type.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Username(String);

impl Username {
    /// Create a new username with validation
    /// 
    /// Rules:
    /// - 3-32 characters
    /// - Alphanumeric and underscore only
    /// - Cannot start with underscore
    /// - Case-insensitive for uniqueness, preserves original case
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();
        
        if s.len() < 3 {
            return Err(ValidationError::TooShort { min: 3, actual: s.len() });
        }
        if s.len() > 32 {
            return Err(ValidationError::TooLong { max: 32, actual: s.len() });
        }
        if s.starts_with('_') {
            return Err(ValidationError::InvalidFormat { 
                reason: "cannot start with underscore".into() 
            });
        }
        if !s.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(ValidationError::InvalidFormat { 
                reason: "must be alphanumeric or underscore".into() 
            });
        }
        
        Ok(Self(s))
    }
    
    /// Get the username as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Get lowercase version for comparison
    pub fn normalized(&self) -> String {
        self.0.to_lowercase()
    }
}
```

#### Email

Validated email type.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    /// Create a new email with validation
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();
        
        // Basic email validation
        if !s.contains('@') {
            return Err(ValidationError::InvalidFormat { 
                reason: "missing @ symbol".into() 
            });
        }
        
        let parts: Vec<&str> = s.split('@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(ValidationError::InvalidFormat { 
                reason: "invalid email format".into() 
            });
        }
        
        if !parts[1].contains('.') {
            return Err(ValidationError::InvalidFormat { 
                reason: "domain must contain a dot".into() 
            });
        }
        
        if s.len() > 254 {
            return Err(ValidationError::TooLong { max: 254, actual: s.len() });
        }
        
        Ok(Self(s.to_lowercase()))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

#### Role

User permission level.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Regular user - can chat, join rooms, send DMs
    User,
    
    /// Moderator - can moderate rooms they're assigned to
    Moderator,
    
    /// Administrator - full system access
    Admin,
}

impl Role {
    /// Check if this role has at least the given permission level
    pub fn has_permission(&self, required: Role) -> bool {
        match (self, required) {
            (Role::Admin, _) => true,
            (Role::Moderator, Role::Moderator | Role::User) => true,
            (Role::User, Role::User) => true,
            _ => false,
        }
    }
}

impl Default for Role {
    fn default() -> Self {
        Role::User
    }
}
```

---

### Room

Represents a chat room.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    /// Unique identifier
    pub id: RoomId,
    
    /// Room name (unique, 1-64 characters)
    pub name: RoomName,
    
    /// Optional description
    pub description: Option<String>,
    
    /// User who created/owns the room
    pub owner: UserId,
    
    /// Room configuration
    pub settings: RoomSettings,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}
```

#### RoomName

Validated room name.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomName(String);

impl RoomName {
    /// Create a new room name with validation
    /// 
    /// Rules:
    /// - 1-64 characters
    /// - Cannot be only whitespace
    /// - Trimmed of leading/trailing whitespace
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into().trim().to_string();
        
        if s.is_empty() {
            return Err(ValidationError::Empty);
        }
        if s.len() > 64 {
            return Err(ValidationError::TooLong { max: 64, actual: s.len() });
        }
        
        Ok(Self(s))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

#### RoomSettings

Room configuration options.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSettings {
    /// Whether the room is publicly visible and joinable
    pub public: bool,
    
    /// Maximum number of members (None = unlimited)
    pub max_members: Option<u32>,
    
    /// Whether only moderators/admins can send messages
    pub moderated: bool,
    
    /// Minimum role required to join
    pub join_role: Role,
}

impl Default for RoomSettings {
    fn default() -> Self {
        Self {
            public: true,
            max_members: None,
            moderated: false,
            join_role: Role::User,
        }
    }
}
```

#### RoomMembership

Represents a user's membership in a room.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMembership {
    /// The room
    pub room_id: RoomId,
    
    /// The member
    pub user_id: UserId,
    
    /// Member's role within this room
    pub room_role: RoomRole,
    
    /// When they joined
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoomRole {
    /// Regular member
    Member,
    
    /// Room moderator (can kick, mute)
    Moderator,
    
    /// Room owner (full control)
    Owner,
}
```

---

### Message

Represents a chat message.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier
    pub id: MessageId,
    
    /// Who sent the message
    pub author: UserId,
    
    /// Where the message was sent
    pub target: MessageTarget,
    
    /// Message content
    pub content: MessageContent,
    
    /// Whether the message has been edited
    pub edited: bool,
    
    /// When the message was sent
    pub created_at: DateTime<Utc>,
    
    /// When the message was last edited (if edited)
    pub edited_at: Option<DateTime<Utc>>,
}
```

#### MessageTarget

Where a message is sent.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageTarget {
    /// Message to a room
    Room { room_id: RoomId },
    
    /// Direct message to a user
    DirectMessage { recipient: UserId },
}
```

#### MessageContent

The content of a message.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent(String);

impl MessageContent {
    /// Create new message content with validation
    /// 
    /// Rules:
    /// - 1-4096 characters
    /// - Cannot be only whitespace
    /// - Trimmed of leading/trailing whitespace
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();
        let trimmed = s.trim();
        
        if trimmed.is_empty() {
            return Err(ValidationError::Empty);
        }
        if s.len() > 4096 {
            return Err(ValidationError::TooLong { max: 4096, actual: s.len() });
        }
        
        // Preserve original (including internal whitespace), just validate
        Ok(Self(s))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

---

### Session

Represents an authenticated session.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier
    pub id: SessionId,
    
    /// The authenticated user
    pub user_id: UserId,
    
    /// Which protocol created this session
    pub protocol: Protocol,
    
    /// When the session was created
    pub created_at: DateTime<Utc>,
    
    /// When the session expires
    pub expires_at: DateTime<Utc>,
    
    /// Last activity timestamp
    pub last_active: DateTime<Utc>,
}

impl Session {
    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
    
    /// Check if the session is still valid
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }
}
```

#### Protocol

The protocol used to connect.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    /// TCP socket connection
    Tcp,
    
    /// HTTP REST API
    Http,
    
    /// WebSocket connection
    WebSocket,
}
```

---

### Invitation

Represents a room invitation.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invitation {
    /// Unique identifier
    pub id: InvitationId,
    
    /// Room being invited to
    pub room_id: RoomId,
    
    /// User who sent the invitation
    pub inviter: UserId,
    
    /// User being invited
    pub invitee: UserId,
    
    /// Current status
    pub status: InvitationStatus,
    
    /// When the invitation was created
    pub created_at: DateTime<Utc>,
    
    /// When the invitation expires (None = never)
    pub expires_at: Option<DateTime<Utc>>,
}

impl Invitation {
    /// Check if the invitation can still be accepted
    pub fn is_pending(&self) -> bool {
        matches!(self.status, InvitationStatus::Pending) 
            && self.expires_at.map_or(true, |exp| Utc::now() < exp)
    }
}
```

#### InvitationStatus

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvitationStatus {
    /// Waiting for response
    Pending,
    
    /// Invitation accepted
    Accepted,
    
    /// Invitation declined
    Declined,
    
    /// Invitation cancelled by inviter
    Cancelled,
    
    /// Invitation expired
    Expired,
}
```

---

## Supporting Types

### Pagination

For paginated queries.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    /// Number of items to skip
    pub offset: u32,
    
    /// Maximum items to return (default: 50, max: 100)
    pub limit: u32,
}

impl Default for Pagination {
    fn default() -> Self {
        Self { offset: 0, limit: 50 }
    }
}

impl Pagination {
    pub fn new(offset: u32, limit: u32) -> Self {
        Self {
            offset,
            limit: limit.min(100), // Enforce maximum
        }
    }
}
```

### Timestamps

All timestamps use `DateTime<Utc>` from the `chrono` crate for consistency.

---

## Validation Errors

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Value is empty when it shouldn't be
    Empty,
    
    /// Value is too short
    TooShort { min: usize, actual: usize },
    
    /// Value is too long
    TooLong { max: usize, actual: usize },
    
    /// Value has invalid format
    InvalidFormat { reason: String },
    
    /// Value failed a custom validation rule
    Custom { message: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "value cannot be empty"),
            Self::TooShort { min, actual } => {
                write!(f, "value too short: minimum {min}, got {actual}")
            }
            Self::TooLong { max, actual } => {
                write!(f, "value too long: maximum {max}, got {actual}")
            }
            Self::InvalidFormat { reason } => write!(f, "invalid format: {reason}"),
            Self::Custom { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ValidationError {}
```

---

## Relationships Summary

| Entity | Relates To | Relationship |
|--------|-----------|--------------|
| User | Room | owns (as owner), belongs to (as member) |
| User | Message | authors |
| User | Session | has many |
| User | Invitation | sends, receives |
| Room | Message | contains |
| Room | User | has members |
| Room | Invitation | is target of |
| Message | Room | belongs to (if room message) |
| Message | User | belongs to (if DM) |
| Session | User | belongs to |
| Invitation | Room | for |
| Invitation | User | from (inviter), to (invitee) |

---

## Constraints Summary

| Type | Constraint |
|------|-----------|
| Username | 3-32 chars, alphanumeric + underscore, unique |
| Email | Valid email format, max 254 chars, unique |
| RoomName | 1-64 chars, unique |
| MessageContent | 1-4096 chars |
| Pagination.limit | Max 100 |
| Session | Has expiration time |
| Invitation | Has optional expiration |

---

## Serialization

All domain types derive `Serialize` and `Deserialize` from serde, enabling:
- JSON serialization for HTTP API
- JSON serialization for TCP wire protocol
- Storage serialization (if using JSON columns)

Field naming uses `snake_case` in serialized form. Enums use lowercase variants.
