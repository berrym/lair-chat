# Lair Chat Server

A secure, high-performance chat server with TCP and HTTP protocol adapters.

## Features

- **Multi-protocol support**: TCP (real-time) and HTTP (REST API) adapters
- **Protocol-agnostic core**: Clean separation between business logic and protocols
- **SQLite storage**: Persistent storage with connection pooling
- **Real-time messaging**: Instant message delivery to connected clients
- **Room management**: Create, join, and manage chat rooms
- **User authentication**: Secure password hashing with Argon2
- **JWT tokens**: Session management via JSON Web Tokens (HTTP API)
- **Graceful shutdown**: Clean connection handling on SIGTERM/Ctrl+C

## Installation

### From Source

```bash
# From the workspace root
cargo build --release -p lair-chat-server

# Binary will be at target/release/lair-chat-server
```

### Running

```bash
# Run with defaults
lair-chat-server

# With custom configuration
LAIR_TCP_PORT=9000 LAIR_HTTP_PORT=9001 lair-chat-server
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LAIR_TCP_PORT` | TCP server port | 8080 |
| `LAIR_HTTP_PORT` | HTTP server port | 8082 |
| `LAIR_DATABASE_URL` | SQLite database URL | sqlite:lair-chat.db?mode=rwc |
| `RUST_LOG` | Log level | info |

### Example

```bash
export LAIR_TCP_PORT=8080
export LAIR_HTTP_PORT=8082
export LAIR_DATABASE_URL="sqlite:data/chat.db?mode=rwc"
export RUST_LOG="info,lair_chat_server=debug"

lair-chat-server
```

## Architecture

The server follows a clean, protocol-agnostic architecture:

```
src/
├── main.rs              # Unified binary entry point
├── lib.rs               # Library root
├── error.rs             # Error types
│
├── domain/              # Pure domain types (no I/O)
│   ├── user.rs          # User, UserId, Role
│   ├── room.rs          # Room, RoomId, RoomSettings
│   ├── message.rs       # Message, MessageId, Target
│   ├── session.rs       # Session, SessionId
│   ├── invitation.rs    # Invitation, InviteStatus
│   └── events.rs        # Domain events
│
├── core/                # Business logic (async, no network)
│   ├── engine.rs        # ChatEngine - main coordinator
│   ├── auth.rs          # AuthService
│   ├── messaging.rs     # MessagingService
│   ├── rooms.rs         # RoomService
│   ├── sessions.rs      # SessionManager
│   └── events.rs        # EventDispatcher
│
├── storage/             # Persistence layer
│   ├── traits.rs        # Repository traits
│   └── sqlite/          # SQLite implementation
│       ├── users.rs
│       ├── rooms.rs
│       ├── messages.rs
│       ├── sessions.rs
│       ├── invitations.rs
│       └── migrations.rs
│
├── adapters/            # Protocol adapters
│   ├── tcp/             # TCP protocol (real-time)
│   │   ├── server.rs
│   │   ├── connection.rs
│   │   ├── protocol.rs
│   │   └── commands.rs
│   ├── http/            # REST API
│   │   ├── server.rs
│   │   ├── routes.rs
│   │   ├── handlers/
│   │   └── middleware/
│   └── ws/              # WebSocket (placeholder)
│
├── crypto/              # Encryption utilities
│   ├── aes_gcm.rs
│   └── key_exchange.rs
│
└── config/              # Configuration
    ├── mod.rs
    └── settings.rs
```

### Design Principles

1. **Dependency Inversion**: Core depends on traits, not implementations
2. **Single Responsibility**: Each module has one clear purpose
3. **Protocol Agnostic**: Same business logic regardless of transport
4. **Testability**: Core logic testable without I/O

## API Reference

### TCP Protocol

The TCP adapter uses length-prefixed JSON messages. See `docs/protocols/TCP.md`
for the complete wire protocol specification.

Quick test with netcat:
```bash
# Connect and send handshake
echo '{"type":"Handshake","version":"1.0.0","client_type":"test"}' | nc localhost 8080
```

### HTTP REST API

The HTTP adapter exposes a RESTful API. See `docs/protocols/HTTP.md` for the
complete OpenAPI specification.

Quick test with curl:
```bash
# Health check
curl http://localhost:8082/health

# Register
curl -X POST http://localhost:8082/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","email":"alice@example.com","password":"secret123"}'

# Login
curl -X POST http://localhost:8082/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"identifier":"alice","password":"secret123"}'
```

## Development

### Building

```bash
cargo build -p lair-chat-server
```

### Testing

```bash
# Unit tests
cargo test -p lair-chat-server

# With logging
RUST_LOG=debug cargo test -p lair-chat-server -- --nocapture
```

### Running Locally

```bash
# Development mode with debug logging
RUST_LOG=debug cargo run -p lair-chat-server
```

### Database

The server automatically creates and migrates the SQLite database on startup.
The default location is `lair-chat.db` in the current directory.

To reset the database:
```bash
rm lair-chat.db
cargo run -p lair-chat-server
```

## Protocol Documentation

- **TCP Protocol**: `docs/protocols/TCP.md` - Wire protocol for real-time clients
- **HTTP API**: `docs/protocols/HTTP.md` - REST API specification (OpenAPI)

These specifications are language-agnostic. You can implement clients in any
language that supports TCP sockets or HTTP requests.

## License

MIT
