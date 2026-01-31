# Lair Chat

A secure, high-performance chat system built with Rust, featuring real-time messaging, end-to-end encryption support, and TCP, WebSocket, and REST API interfaces.

[![CI](https://github.com/berrym/lair-chat/actions/workflows/ci.yml/badge.svg)](https://github.com/berrym/lair-chat/actions/workflows/ci.yml)
[![Security Audit](https://github.com/berrym/lair-chat/actions/workflows/security.yml/badge.svg)](https://github.com/berrym/lair-chat/actions/workflows/security.yml)
[![codecov](https://codecov.io/gh/berrym/lair-chat/graph/badge.svg)](https://codecov.io/gh/berrym/lair-chat)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.88+-orange.svg)](https://www.rust-lang.org)

## Features

### Core Functionality
- **Real-time messaging** via persistent TCP or WebSocket connections with server-pushed events
- **REST API** for authentication, CRUD operations, and queries
- **WebSocket support** for browser-compatible real-time connections through HTTP proxies
- **Terminal-based client** with modern TUI interface (supports both TCP and WebSocket)
- **Room-based chat** with invitations and membership management
- **Direct messaging** between users
- **Role-based access control** (Admin, Moderator, User)

### Security
- **Native TLS/HTTPS** support for HTTP API (rustls)
- **End-to-end encryption** support (AES-256-GCM with X25519 key exchange)
- **Argon2id password hashing** for secure credential storage
- **JWT authentication** with session management
- **Rate limiting** and input validation

### Technical Highlights
- **Protocol-first design** - TCP and HTTP protocols fully documented for any client
- **Clean architecture** - Domain-driven design with trait-based abstractions
- **Async throughout** - Built on Tokio for high performance
- **SQLite storage** - With migration support and connection pooling
- **Cross-platform** - Linux, macOS, Windows (see [Platform Support](docs/deployment/PRODUCTION.md#platform-support))
- **Comprehensive tests** - 800+ unit and integration tests with CI/CD

## Quick Start

### Prerequisites
- **Rust 1.88+** ([rustup.rs](https://rustup.rs/))

### Run the Server

```bash
# Clone the repository
git clone https://github.com/berrym/lair-chat.git
cd lair-chat

# Start the server (TCP on 8080, HTTP on 8082)
cargo run --package lair-chat-server
```

The server exposes three protocols:
- **HTTP API** (`http://localhost:8082`) - Authentication, room management, message history
- **TCP** (`localhost:8080`) - Real-time messaging and presence (with E2E encryption option)
- **WebSocket** (`ws://localhost:8082/ws`) - Real-time messaging through HTTP (browser-compatible)

For HTTPS, see [Transport Security](docs/protocols/HTTP.md#transport-security).

### Run the Client

```bash
# In another terminal (default: TCP transport)
cargo run --package lair-chat-client

# Use WebSocket transport (works through HTTP proxies)
cargo run --package lair-chat-client -- --websocket

# Or with custom HTTP URL (for HTTPS)
cargo run --package lair-chat-client -- --http-url https://localhost:8082 --insecure
```

The TUI client authenticates via HTTP and connects to TCP (default) or WebSocket (`--websocket`) for real-time messaging.

### Configuration

Environment variables:
```bash
LAIR_TCP_PORT=8080        # TCP server port (real-time messaging)
LAIR_HTTP_PORT=8082       # HTTP server port (auth, CRUD, queries)
LAIR_DATABASE_URL=sqlite:lair-chat.db  # Database path
LAIR_JWT_SECRET=secret    # JWT signing secret (auto-generated if not set)
RUST_LOG=info             # Log level (error, warn, info, debug, trace)

# TLS/HTTPS (optional)
LAIR_TLS_ENABLED=true     # Enable HTTPS (default: false)
LAIR_TLS_CERT_PATH=/path/to/cert.pem   # Certificate file
LAIR_TLS_KEY_PATH=/path/to/key.pem     # Private key file
```

## Architecture

Lair Chat uses a **protocol responsibility split** (see [ADR-013](docs/architecture/DECISIONS.md#adr-013-protocol-responsibility-split)):

- **HTTP**: Authentication, CRUD operations, queries (stateless, standard tooling)
- **TCP**: Real-time messaging, presence, events (persistent connections, low latency)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              LAIR CHAT SERVER                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                        PROTOCOL ADAPTERS                              │  │
│  │                                                                       │  │
│  │   ┌───────────────────┐  ┌───────────────────┐  ┌───────────────────┐│  │
│  │   │    HTTP :8082     │  │  WebSocket :8082  │  │    TCP :8080      ││  │
│  │   │ ───────────────── │  │ ───────────────── │  │ ───────────────── ││  │
│  │   │ - Auth (login)    │  │ - Real-time msgs  │  │ - Real-time msgs  ││  │
│  │   │ - Room CRUD       │  │ - Events (push)   │  │ - E2E encryption  ││  │
│  │   │ - Message history │  │ - Pre-auth token  │  │ - Events (push)   ││  │
│  │   │ - User queries    │  │ - Browser-compat  │  │ - Presence        ││  │
│  │   │ - Invitations     │  │                   │  │ - Typing          ││  │
│  │   └─────────┬─────────┘  └─────────┬─────────┘  └─────────┬─────────┘│  │
│  │             │                      │                      │          │  │
│  └─────────────┼──────────────────────┼──────────────────────┼──────────┘  │
│                │                      │                      │             │
│                └──────────────────────┼──────────────────────┘             │
│                                 │                                           │
│                                 ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                          CORE ENGINE                                  │  │
│  │                                                                       │  │
│  │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │  │
│  │   │    Auth     │  │  Messaging  │  │    Room     │  │   Session   │ │  │
│  │   │   Service   │  │   Service   │  │   Service   │  │   Manager   │ │  │
│  │   └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘ │  │
│  │                                                                       │  │
│  │   ┌─────────────────────────────────────────────────────────────────┐│  │
│  │   │                    Event Dispatcher                             ││  │
│  │   │              (broadcasts to connected clients)                  ││  │
│  │   └─────────────────────────────────────────────────────────────────┘│  │
│  └───────────────────────────────────┬───────────────────────────────────┘  │
│                                      │                                      │
│                                      ▼                                      │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                         STORAGE LAYER                                 │  │
│  │                           (SQLite)                                    │  │
│  │   Users · Rooms · Messages · Sessions · Invitations · Memberships    │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Connection Flow

```
┌────────┐                    ┌──────────┐                    ┌────────┐
│ Client │                    │   HTTP   │                    │  TCP   │
└───┬────┘                    └────┬─────┘                    └───┬────┘
    │                              │                              │
    │── POST /auth/login ─────────▶│                              │
    │◀── JWT Token + User ─────────│                              │
    │                              │                              │
    │── GET /rooms ───────────────▶│                              │
    │◀── Room List ────────────────│                              │
    │                              │                              │
    │─────────────────── TCP Connect ────────────────────────────▶│
    │◀────────────────── ServerHello ─────────────────────────────│
    │─────────────────── ClientHello ────────────────────────────▶│
    │─────────────────── Authenticate(jwt) ──────────────────────▶│
    │◀────────────────── AuthenticateResponse ────────────────────│
    │                              │                              │
    │◀═══════════════════ Real-time Events ══════════════════════▶│
```

## Project Structure

```
lair-chat/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── lair-chat-server/         # Server implementation
│   │   └── src/
│   │       ├── main.rs           # Unified binary entry point
│   │       ├── domain/           # Pure domain types (User, Room, Message, etc.)
│   │       ├── core/             # Business logic services
│   │       ├── storage/          # SQLite repository implementations
│   │       ├── adapters/         # Protocol adapters
│   │       │   ├── tcp/          # TCP real-time protocol (with E2E encryption)
│   │       │   ├── http/         # REST API (handlers, middleware)
│   │       │   └── ws/           # WebSocket real-time protocol
│   │       ├── crypto/           # AES-256-GCM encryption, X25519 key exchange
│   │       └── config/           # Configuration management
│   │
│   └── lair-chat-client/         # TUI client
│       └── src/
│           ├── main.rs           # Client entry point
│           ├── app.rs            # Application state
│           ├── protocol/         # TCP and WebSocket protocol implementations
│           └── components/       # TUI screens (login, chat, rooms)
│
└── docs/
    ├── architecture/             # Architecture documentation
    │   ├── OVERVIEW.md           # System design overview
    │   ├── DECISIONS.md          # Architecture Decision Records (ADRs)
    │   ├── DOMAIN_MODEL.md       # Entity definitions
    │   ├── COMMANDS.md           # All operations
    │   └── EVENTS.md             # Real-time events
    └── protocols/                # Protocol specifications
        ├── TCP.md                # TCP wire protocol (real-time, E2E encryption)
        ├── HTTP.md               # REST API specification
        └── WEBSOCKET.md          # WebSocket protocol (browser-compatible)
```

## Protocol Documentation

Lair Chat is designed to be protocol-first. You can implement clients in any language using the documented protocols:

- **[HTTP API](docs/protocols/HTTP.md)** - RESTful JSON API for auth, CRUD, queries
- **[TCP Protocol](docs/protocols/TCP.md)** - Length-prefixed JSON for real-time messaging (with E2E encryption)
- **[WebSocket Protocol](docs/protocols/WEBSOCKET.md)** - Plain JSON over WebSocket for browser-compatible real-time messaging

## API Examples

### HTTP API (Auth & CRUD)

```bash
# Health check
curl http://localhost:8082/health

# Register a new user
curl -X POST http://localhost:8082/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","email":"alice@example.com","password":"Secret123!"}'

# Login and get JWT token
curl -X POST http://localhost:8082/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"identifier":"alice","password":"Secret123!"}'
# Response: {"user":{...},"session":{...},"token":"eyJ..."}

# List rooms (with JWT token)
curl http://localhost:8082/api/v1/rooms \
  -H "Authorization: Bearer eyJ..."

# Create a room
curl -X POST http://localhost:8082/api/v1/rooms \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer eyJ..." \
  -d '{"name":"General","description":"General chat room"}'

# Get message history
curl "http://localhost:8082/api/v1/messages?target_type=room&target_id=ROOM_ID" \
  -H "Authorization: Bearer eyJ..."
```

### TCP Protocol (Real-time)

After authenticating via HTTP, connect to TCP for real-time messaging:

```json
// 1. Connect to TCP port 8080, receive ServerHello
// 2. Send ClientHello
{"type":"client_hello","version":"1.1","client_name":"My Client"}

// 3. Authenticate with JWT from HTTP login
{"type":"authenticate","request_id":"1","token":"eyJ..."}

// 4. Send messages in real-time
{"type":"send_message","request_id":"2","target":{"type":"room","room_id":"..."},"content":"Hello!"}

// 5. Receive real-time events (pushed by server)
{"type":"message_received","message":{...}}
{"type":"user_online","user_id":"...","username":"bob"}
```

See [docs/protocols/TCP.md](docs/protocols/TCP.md) for the complete wire protocol specification.

## Testing

```bash
# Run all tests (800+ tests)
cargo test --workspace

# Run server tests only
cargo test --package lair-chat-server

# Run with logging
RUST_LOG=debug cargo test --workspace

# Run specific test
cargo test --package lair-chat-server test_send_message
```

## Development

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace

# Build release
cargo build --release --workspace

# Run with debug logging
RUST_LOG=debug cargo run --package lair-chat-server
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Documentation

- **[Production Deployment](docs/deployment/PRODUCTION.md)** - Deployment guide, platform support, security
- **[Manual Testing Guide](docs/development/TESTING.md)** - Verify TLS, encryption, and client functionality
- **[Architecture Overview](docs/architecture/OVERVIEW.md)** - High-level system design
- **[Architecture Decisions](docs/architecture/DECISIONS.md)** - ADRs explaining why choices were made
- **[Domain Model](docs/architecture/DOMAIN_MODEL.md)** - Entity definitions and relationships
- **[TCP Protocol](docs/protocols/TCP.md)** - Real-time wire protocol with E2E encryption
- **[WebSocket Protocol](docs/protocols/WEBSOCKET.md)** - Browser-compatible real-time protocol
- **[HTTP API](docs/protocols/HTTP.md)** - REST API specification

## License

MIT License - see [LICENSE](LICENSE) for details.

---

**Built with Rust**
