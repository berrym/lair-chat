# Lair Chat Architecture Overview

This document provides a high-level overview of the Lair Chat system architecture. For detailed specifications, see the linked documents.

---

## Vision

Lair Chat is a secure, high-performance chat system built with Rust. It provides:

- **Real-time messaging** via TCP and WebSocket
- **REST API** for stateless interactions
- **End-to-end encryption** for secure communications
- **Terminal-based client** with modern TUI
- **Production-ready** architecture for deployment

---

## System Diagram

```
                            ┌──────────────────────────────────────────────┐
                            │              LAIR CHAT SERVER                │
                            │                                              │
┌─────────┐                 │  ┌────────────────────────────────────────┐ │
│   TUI   │─────TCP:8080────│──│           PROTOCOL ADAPTERS            │ │
│ Client  │                 │  │  ┌────────┐ ┌────────┐ ┌────────┐     │ │
└─────────┘                 │  │  │  TCP   │ │  HTTP  │ │   WS   │     │ │
                            │  │  │Adapter │ │Adapter │ │Adapter │     │ │
┌─────────┐                 │  │  └───┬────┘ └───┬────┘ └───┬────┘     │ │
│   Web   │────HTTP:8082────│──│      │          │          │          │ │
│ Client  │                 │  └──────┼──────────┼──────────┼──────────┘ │
└─────────┘                 │         └──────────┼──────────┘            │
                            │                    │                       │
┌─────────┐                 │  ┌─────────────────▼────────────────────┐ │
│ Mobile  │────HTTP:8082────│──│            CORE ENGINE               │ │
│  App    │                 │  │                                      │ │
└─────────┘                 │  │  ┌──────────┐ ┌──────────┐          │ │
                            │  │  │  Auth    │ │ Messaging│          │ │
┌─────────┐                 │  │  │ Service  │ │ Service  │          │ │
│   Bot   │─────TCP:8080────│──│  └──────────┘ └──────────┘          │ │
│         │                 │  │  ┌──────────┐ ┌──────────┐          │ │
└─────────┘                 │  │  │  Room    │ │ Session  │          │ │
                            │  │  │ Service  │ │ Manager  │          │ │
                            │  │  └──────────┘ └──────────┘          │ │
                            │  │                    │                 │ │
                            │  └────────────────────┼─────────────────┘ │
                            │                       │                   │
                            │  ┌────────────────────▼─────────────────┐ │
                            │  │           STORAGE LAYER              │ │
                            │  │  ┌──────────┐ ┌──────────┐          │ │
                            │  │  │  SQLite  │ │PostgreSQL│          │ │
                            │  │  │  (impl)  │ │ (future) │          │ │
                            │  │  └──────────┘ └──────────┘          │ │
                            │  └──────────────────────────────────────┘ │
                            │                       │                   │
                            └───────────────────────┼───────────────────┘
                                                    │
                                                    ▼
                                            ┌──────────────┐
                                            │   Database   │
                                            │   (SQLite)   │
                                            └──────────────┘
```

---

## Architectural Layers

### 1. Protocol Adapters (`adapters/`)

Thin layer that translates between wire protocols and the core engine.

| Adapter | Port | Purpose |
|---------|------|---------|
| TCP | 8080 | Persistent connections, real-time messaging |
| HTTP | 8082 | REST API, stateless requests |
| WebSocket | 8082 | Real-time for web clients (future) |

**Responsibilities:**
- Parse incoming wire format (JSON over TCP, HTTP requests)
- Translate to core Commands
- Execute commands via Core Engine
- Serialize responses back to wire format
- Push Events to connected clients

**See:** [TCP Protocol](../protocols/TCP.md), [HTTP API](../protocols/HTTP.md)

### 2. Core Engine (`core/`)

Protocol-agnostic business logic. The heart of the system.

**Components:**

| Component | Purpose |
|-----------|---------|
| ChatEngine | Main coordinator, routes commands |
| AuthService | Authentication, password hashing, JWT |
| MessagingService | Send, edit, delete messages |
| RoomService | Room CRUD, membership management |
| SessionManager | Session lifecycle, connection tracking |
| EventDispatcher | Broadcast events to subscribers |

**Responsibilities:**
- Validate all inputs
- Enforce business rules
- Coordinate between services
- Emit events for state changes

**See:** [Commands](COMMANDS.md), [Events](EVENTS.md)

### 3. Domain Types (`domain/`)

Pure Rust types with no I/O dependencies.

**Entities:**
- User, UserId, Username, Email, Role
- Room, RoomId, RoomName, RoomSettings, RoomMembership
- Message, MessageId, MessageContent, MessageTarget
- Session, SessionId, Protocol
- Invitation, InvitationId, InvitationStatus

**See:** [Domain Model](DOMAIN_MODEL.md)

### 4. Storage Layer (`storage/`)

Database abstraction via repository traits.

**Traits:**
- UserRepository
- RoomRepository
- MessageRepository
- SessionRepository
- InvitationRepository

**Implementations:**
- SQLite (current)
- PostgreSQL (future)
- MySQL (future)

**See:** [ADR-003](DECISIONS.md#adr-003-trait-based-storage-abstraction)

---

## Data Flow

### Request Flow

```
Client Request
      │
      ▼
┌─────────────┐
│  Protocol   │  Parse wire format
│  Adapter    │  Create Command
└──────┬──────┘
       │
       ▼
┌─────────────┐
│    Core     │  Validate
│   Engine    │  Execute business logic
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Storage    │  Persist changes
│   Layer     │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Event     │  Broadcast state changes
│ Dispatcher  │
└──────┬──────┘
       │
       ▼
Response to Client
```

### Example: Send Message

1. **Client** sends `send_message` via TCP
2. **TCP Adapter** parses JSON, creates `SendMessage` command
3. **Core Engine** receives command:
   - Validates session
   - Validates user is room member
   - Creates message with timestamp
4. **Storage** persists message to database
5. **EventDispatcher** broadcasts `MessageReceived` event
6. **TCP Adapter** sends response to sender
7. **TCP Adapter** pushes event to all room members

---

## Authentication Flow

```
┌────────┐                  ┌────────┐                  ┌────────┐
│ Client │                  │ Server │                  │Database│
└───┬────┘                  └───┬────┘                  └───┬────┘
    │                           │                           │
    │──── Login Request ───────▶│                           │
    │     (identifier, pass)    │                           │
    │                           │                           │
    │                           │──── Find User ───────────▶│
    │                           │◀─── User + Hash ──────────│
    │                           │                           │
    │                           │ Verify Password (Argon2)  │
    │                           │                           │
    │                           │──── Create Session ──────▶│
    │                           │◀─── Session ID ───────────│
    │                           │                           │
    │                           │ Generate JWT              │
    │                           │                           │
    │◀─── Login Response ───────│                           │
    │     (user, session, jwt)  │                           │
    │                           │                           │
```

---

## Event System

Events enable real-time updates without polling.

### Event Flow

```
State Change (e.g., new message)
           │
           ▼
    ┌─────────────┐
    │   Event     │  Create event
    │  Emitter    │
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │   Event     │  Determine recipients
    │ Dispatcher  │
    └──────┬──────┘
           │
           ├──────────────┬──────────────┐
           ▼              ▼              ▼
      ┌────────┐    ┌────────┐    ┌────────┐
      │  TCP   │    │  TCP   │    │   WS   │
      │Client 1│    │Client 2│    │Client 3│
      └────────┘    └────────┘    └────────┘
```

### Event Targeting

| Event | Recipients |
|-------|------------|
| MessageReceived | Room members or DM participants |
| UserJoinedRoom | Room members |
| UserOnline | Users sharing rooms/DMs |
| InvitationReceived | Invitee only |
| ServerNotice | All connected users |

**See:** [Events](EVENTS.md)

---

## Security Architecture

### Transport Security

- **TLS 1.3** for HTTP connections
- **Optional E2E encryption** for TCP (AES-256-GCM)

### Authentication

- **Password hashing**: Argon2id
- **Session tokens**: JWT with expiration
- **Session storage**: Server-side for revocation

### Authorization

- **Role-based**: User, Moderator, Admin
- **Room-based**: Member, Moderator, Owner
- **Permission checks** in Core Engine

### Data Protection

- **Input validation** at adapter and core layers
- **SQL injection prevention** via parameterized queries (SQLx)
- **Rate limiting** per user/IP

---

## Directory Structure

```
lair-chat/
├── crates/
│   ├── lair-chat-server/
│   │   └── src/
│   │       ├── main.rs           # Entry point
│   │       ├── domain/           # Pure types
│   │       ├── core/             # Business logic
│   │       ├── storage/          # Database layer
│   │       ├── adapters/         # TCP, HTTP, WS
│   │       ├── crypto/           # Encryption
│   │       └── config/           # Configuration
│   │
│   └── lair-chat-client/
│       └── src/                  # TUI client
│
├── docs/
│   ├── architecture/             # This documentation
│   └── protocols/                # Wire format specs
│
└── tests/                        # Integration tests
```

---

## Configuration

Server configuration via environment or config file:

```toml
[server]
tcp_port = 8080
http_port = 8082

[database]
url = "sqlite:data/lair_chat.db"
max_connections = 20

[auth]
jwt_secret = "your-secret-key"
session_duration_hours = 24

[features]
encryption_required = false
```

---

## Deployment

### Single Binary

```bash
# Build
cargo build --release

# Run
./target/release/lair-chat-server
```

### Docker

```bash
docker build -t lair-chat .
docker run -p 8080:8080 -p 8082:8082 lair-chat
```

### Scaling

Current: Single instance
Future: Multiple instances with shared database + Redis for sessions

---

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Protocol-agnostic core | Write business logic once, support multiple protocols |
| Trait-based storage | Swap databases without changing core code |
| Single binary | Simpler deployment than microservices |
| No global state | Testability, explicit dependencies |
| Command pattern | Uniform interface from all protocols |

**See:** [Architecture Decision Records](DECISIONS.md)

---

## Documentation Index

| Document | Purpose |
|----------|---------|
| [OVERVIEW.md](OVERVIEW.md) | This document - high-level architecture |
| [DECISIONS.md](DECISIONS.md) | Why decisions were made (ADRs) |
| [DOMAIN_MODEL.md](DOMAIN_MODEL.md) | Entity definitions and relationships |
| [COMMANDS.md](COMMANDS.md) | All operations the system supports |
| [EVENTS.md](EVENTS.md) | Real-time events and subscriptions |
| [TCP.md](../protocols/TCP.md) | TCP wire protocol specification |
| [HTTP.md](../protocols/HTTP.md) | REST API specification |

---

## Success Criteria

The architecture is successful when:

1. **Single binary** runs TCP + HTTP with shared state
2. **No global state** - all state flows through ChatEngine
3. **No file over 500 lines** - clean module boundaries
4. **All operations go through core** - adapters are thin
5. **Protocol documentation** sufficient to build clients
6. **TUI client** proves the architecture works

---

## Version

**Architecture Version**: 1.0  
**Last Updated**: January 2025
