# Lair Chat

A secure, high-performance chat system built with Rust, featuring real-time messaging, a terminal-based client, and both TCP and REST API interfaces.

[![CI](https://github.com/berrym/lair-chat/actions/workflows/ci.yml/badge.svg)](https://github.com/berrym/lair-chat/actions/workflows/ci.yml)
[![Security Audit](https://github.com/berrym/lair-chat/actions/workflows/security.yml/badge.svg)](https://github.com/berrym/lair-chat/actions/workflows/security.yml)
[![codecov](https://codecov.io/gh/berrym/lair-chat/graph/badge.svg)](https://codecov.io/gh/berrym/lair-chat)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

## Features

### Core Functionality
- **Real-time messaging** via persistent TCP connections
- **REST API** for stateless HTTP access
- **Terminal-based client** with modern TUI interface
- **Room-based chat** with invitations and membership
- **Direct messaging** between users
- **Role-based access control** (Admin, Moderator, User)

### Technical Highlights
- **Protocol-first design** - TCP and HTTP protocols fully documented
- **Clean architecture** - Domain-driven design with trait-based abstractions
- **Async throughout** - Built on Tokio for high performance
- **SQLite storage** - With migration support and connection pooling
- **Comprehensive tests** - 100+ unit and integration tests

## Quick Start

### Prerequisites
- **Rust 1.70+** ([rustup.rs](https://rustup.rs/))

### Run the Server

```bash
# Clone the repository
git clone https://github.com/your-org/lair-chat.git
cd lair-chat

# Start the server (TCP on 8080, HTTP on 8082)
cargo run --package lair-chat-server
```

The server starts with:
- **TCP**: `telnet localhost 8080`
- **HTTP**: `curl http://localhost:8082/health`

### Run the Client

```bash
# In another terminal
cargo run --package lair-chat-client
```

### Configuration

Environment variables:
```bash
LAIR_TCP_PORT=8080        # TCP server port
LAIR_HTTP_PORT=8082       # HTTP server port
LAIR_DATABASE_URL=sqlite:lair-chat.db  # Database path
RUST_LOG=info             # Log level
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        LAIR CHAT                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   PROTOCOL ADAPTERS                      │   │
│  │  ┌─────────┐  ┌─────────┐                               │   │
│  │  │   TCP   │  │  HTTP   │                               │   │
│  │  │ :8080   │  │ :8082   │                               │   │
│  │  └────┬────┘  └────┬────┘                               │   │
│  └───────┼────────────┼────────────────────────────────────┘   │
│          │            │                                         │
│          └─────┬──────┘                                         │
│                │                                                │
│                ▼                                                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                     CORE ENGINE                          │   │
│  │  Auth, Messaging, Rooms, Sessions, Events                │   │
│  └──────────────────────────┬───────────────────────────────┘   │
│                             │                                   │
│                             ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   STORAGE LAYER                          │   │
│  │                      SQLite                              │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
lair-chat/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── lair-chat-server/         # Server implementation
│   │   └── src/
│   │       ├── main.rs           # Unified binary entry point
│   │       ├── domain/           # Pure domain types
│   │       ├── core/             # Business logic
│   │       ├── storage/          # SQLite implementation
│   │       ├── adapters/         # TCP and HTTP adapters
│   │       └── config/           # Configuration
│   │
│   └── lair-chat-client/         # TUI client
│       └── src/
│           ├── main.rs           # Client entry point
│           ├── app.rs            # Application state
│           ├── protocol/         # TCP protocol implementation
│           └── components/       # TUI components
│
└── docs/
    ├── architecture/             # Architecture documentation
    └── protocols/                # Protocol specifications
        ├── TCP.md                # TCP wire protocol
        └── HTTP.md               # REST API specification
```

## Protocol Documentation

Lair Chat is designed to be protocol-first. You can implement clients in any language using the documented protocols:

- **[TCP Protocol](docs/protocols/TCP.md)** - Length-prefixed JSON over TCP
- **[HTTP API](docs/protocols/HTTP.md)** - RESTful JSON API

## API Examples

### HTTP API

```bash
# Health check
curl http://localhost:8082/health

# Register
curl -X POST http://localhost:8082/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","email":"alice@example.com","password":"Secret123!"}'

# Login
curl -X POST http://localhost:8082/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"identifier":"alice","password":"Secret123!"}'
```

### TCP Protocol

See [docs/protocols/TCP.md](docs/protocols/TCP.md) for the complete wire protocol specification.

## Testing

```bash
# Run all tests
cargo test --workspace

# Run server tests only
cargo test --package lair-chat-server

# Run with logging
RUST_LOG=debug cargo test --workspace
```

## Development

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace

# Build release
cargo build --release --workspace
```

## License

MIT License - see [LICENSE](LICENSE) for details.

---

**Built with Rust**
