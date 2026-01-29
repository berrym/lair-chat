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
┌─────────┐   TCP:8080      │  ┌────────────────────────────────────────┐ │
│   TUI   │───(App-layer────│──│           PROTOCOL ADAPTERS            │ │
│ Client  │    encryption)  │  │  ┌────────┐ ┌────────┐ ┌────────┐     │ │
└─────────┘                 │  │  │  TCP   │ │  HTTP  │ │   WS   │     │ │
                            │  │  │Adapter │ │Adapter │ │Adapter │     │ │
┌─────────┐  HTTP/S:8082    │  │  └───┬────┘ └───┬────┘ └───┬────┘     │ │
│   Web   │───(TLS optional)│──│      │          │          │          │ │
│ Client  │                 │  └──────┼──────────┼──────────┼──────────┘ │
└─────────┘                 │         └──────────┼──────────┘            │
                            │                    │                       │
┌─────────┐  HTTP/S:8082    │  ┌─────────────────▼────────────────────┐ │
│ Mobile  │───(TLS optional)│──│            CORE ENGINE               │ │
│  App    │                 │  │                                      │ │
└─────────┘                 │  │  ┌──────────┐ ┌──────────┐          │ │
                            │  │  │  Auth    │ │ Messaging│          │ │
┌─────────┐   TCP:8080      │  │  │ Service  │ │ Service  │          │ │
│   Bot   │───(App-layer────│──│  └──────────┘ └──────────┘          │ │
│         │    encryption)  │  │  ┌──────────┐ ┌──────────┐          │ │
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

| Adapter | Port | Transport Security | Primary Purpose |
|---------|------|---------------------|-----------------|
| HTTP | 8082 | **TLS optional** (rustls) | **Auth, CRUD, queries** - Login, register, room management, message history |
| TCP | 8080 | **App-layer** (X25519 + AES-256-GCM) | **Real-time only** - Message delivery, presence, typing, live events |
| WebSocket | 8082 | TLS (future) | Real-time for web clients (future) |

> **Protocol Split**: HTTP handles authentication and data operations; TCP handles real-time messaging. See [ADR-013](DECISIONS.md#adr-013-protocol-responsibility-split).

**Responsibilities:**

*HTTP Adapter:*
- User authentication (login, register, logout, token refresh)
- Room CRUD (create, read, update, delete)
- Message history retrieval
- User queries and management
- Invitation management
- Admin operations

*TCP Adapter:*
- Token-based authentication (validates JWT from HTTP)
- Real-time message send/edit/delete
- Room join/leave for presence
- Accept/decline invitations (with real-time notification)
- Typing indicators
- Push events to connected clients

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

### HTTP Login (Primary)

```
┌────────┐                  ┌────────┐                  ┌────────┐
│ Client │                  │HTTP API│                  │Database│
└───┬────┘                  └───┬────┘                  └───┬────┘
    │                           │                           │
    │─ POST /auth/login ───────▶│                           │
    │  (identifier, password)   │                           │
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
    │◀── 200 OK ────────────────│                           │
    │   (user, session, jwt)    │                           │
```

### TCP Authentication (After HTTP Login)

```
┌────────┐                  ┌────────┐                  ┌────────┐
│ Client │                  │TCP Srv │                  │JWTSvc  │
└───┬────┘                  └───┬────┘                  └───┬────┘
    │                           │                           │
    │─── TCP Connect ──────────▶│                           │
    │◀── ServerHello ───────────│                           │
    │─── ClientHello ──────────▶│                           │
    │                           │                           │
    │─── Authenticate(jwt) ────▶│                           │
    │                           │── Validate Token ────────▶│
    │                           │◀─ Claims (user_id, etc) ──│
    │                           │                           │
    │◀── AuthenticateResponse ──│                           │
    │    (success, user, session)                           │
    │                           │                           │
    │◀══ Real-time Events ═════▶│                           │
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

- **HTTP/HTTPS**: Optional TLS via rustls (see [ADR-014](DECISIONS.md#adr-014-native-tls-for-http-transport))
  - Disabled by default for development
  - Enable via `LAIR_TLS_ENABLED=true`
  - Requires `LAIR_TLS_CERT_PATH` and `LAIR_TLS_KEY_PATH`
- **TCP**: Application-layer encryption (X25519 key exchange + AES-256-GCM)

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

Server configuration via environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `LAIR_TCP_PORT` | TCP server port | `8080` |
| `LAIR_HTTP_PORT` | HTTP/HTTPS server port | `8082` |
| `LAIR_DATABASE_URL` | Database connection URL | `sqlite:lair-chat.db?mode=rwc` |
| `LAIR_JWT_SECRET` | JWT signing secret | Auto-generated (dev) |
| `LAIR_TLS_ENABLED` | Enable HTTPS | `false` |
| `LAIR_TLS_CERT_PATH` | TLS certificate path | Required if TLS enabled |
| `LAIR_TLS_KEY_PATH` | TLS private key path | Required if TLS enabled |

Example configuration:

```bash
# Development (HTTP)
LAIR_TCP_PORT=8080 \
LAIR_HTTP_PORT=8082 \
LAIR_DATABASE_URL=sqlite:lair-chat.db?mode=rwc \
cargo run -p lair-chat-server

# Production (HTTPS)
LAIR_TCP_PORT=8080 \
LAIR_HTTP_PORT=8082 \
LAIR_DATABASE_URL=sqlite:data/lair_chat.db?mode=rwc \
LAIR_JWT_SECRET=your-secure-secret-key \
LAIR_TLS_ENABLED=true \
LAIR_TLS_CERT_PATH=/etc/ssl/certs/lair-chat.pem \
LAIR_TLS_KEY_PATH=/etc/ssl/private/lair-chat.key \
cargo run -p lair-chat-server --release
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

**Architecture Version**: 1.1
**Last Updated**: January 2025

### Changelog

- **1.1**: Protocol responsibility split (ADR-013) - HTTP for auth/CRUD, TCP for real-time
- **1.0**: Initial architecture
