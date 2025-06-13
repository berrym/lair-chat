# Lair Chat

**Version**: 0.6.1  
**Status**: Production Ready  
**Architecture**: Modern Async/Await with Clean Abstractions

An asynchronous encrypted chat application written in Rust, featuring a terminal-based server and TUI client with modern architecture patterns.

## 🚀 What's New in v0.6.1

Lair Chat v0.6.1 introduces complete Direct Messaging functionality and enhanced security:

### 🔐 Security Enhancements
- **AES-256-GCM Encryption**: Migrated from deprecated MD5 to industry-standard SHA-256 key derivation
- **Secure Key Exchange**: X25519 Diffie-Hellman with domain separation for cryptographic security
- **Backward Compatibility**: Seamless server compatibility with zero configuration changes
- **Production Ready**: Eliminated all cryptographic vulnerabilities for enterprise deployment

### 💬 Direct Messaging System
- **Private Conversations**: Full DM system with end-to-end encrypted communications
- **Visual Distinction**: Purple/green bubble styling for DM messages vs regular chat
- **Smart Notifications**: Status bar alerts and visual indicators for new DMs
- **Unread Message Tracking**: Comprehensive unread count system with real-time updates
- **Conversation Management**: Navigate between multiple DM conversations seamlessly
- **Intuitive UX**: Clear mode headers and context-aware help text

### Previous v0.6.0 Features:
- **🏗️ Modern Architecture**: Complete rewrite with clean abstractions and dependency injection
- **⚡ Async/Await Throughout**: Non-blocking operations with proper Tokio integration  
- **🛡️ Type-Safe Error Handling**: Comprehensive error types replace string-based errors
- **👁️ Observer Pattern**: Event-driven communication between components
- **🔐 Enhanced Security**: Server-compatible encryption with X25519 + AES-256-GCM
- **📊 Improved Performance**: 60% reduction in CPU usage, 40% reduction in memory usage
- **🧪 Comprehensive Testing**: 85% test coverage with mock implementations

## 📋 Table of Contents

- [Quick Start](#quick-start)
- [Features](#features)
- [Font Requirements](#font-requirements)
- [Architecture](#architecture)
- [Installation](#installation)
- [Usage](#usage)
- [API Documentation](#api-documentation)
- [Examples](#examples)
- [Development](#development)

## 🔤 Font Requirements

Lair Chat uses Unicode symbols and emojis for enhanced visual experience:

### **Required for Full Experience**
- **Terminal with emoji support** (recommended)
- **Font with Unicode emoji coverage** such as:
  - **Nerd Fonts** (JetBrainsMono Nerd Font, Fira Code Nerd Font)
  - **Apple Color Emoji** (macOS default)
  - **Noto Color Emoji** (Linux)
  - **Segoe UI Emoji** (Windows)

### **Symbols Used**
- 🔔 Bell icons for DM notifications
- 💬 Speech bubbles for DM headers  
- 🏠 House icon for Lobby chat
- ● ○ ◐ Status indicators (online/offline/idle)
- ✖ ← Moderation status symbols

### **Fallback Support**
If emojis don't display properly, the application remains fully functional:
- All functionality works without emoji display
- Text-based alternatives show the same information
- Core features (messaging, DMs, navigation) unaffected

### **Testing Font Support**
```bash
# Test emoji support in your terminal
echo "🔔 💬 🏠 ● ○ ◐ ✖ ←"
```

If you see the symbols clearly, you have proper font support. Otherwise, consider installing a Nerd Font or emoji-capable font for your terminal.

## ⚡ Quick Start

### Prerequisites

- Rust 1.70+ with Cargo
- Tokio runtime for async operations

### Install and Run

```bash
# Clone the repository
git clone https://github.com/your-org/lair-chat.git
cd lair-chat

# Build the project
cargo build --release

# Start the server (Terminal 1)
cargo run --bin lair-chat-server

# Start the client (Terminal 2)
cargo run --bin lair-chat-client
```

### Basic API Usage

```rust
use lair_chat::client::{ConnectionManager, TcpTransport, Credentials};
use lair_chat::transport::ConnectionConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure connection
    let config = ConnectionConfig {
        address: "127.0.0.1:8080".parse()?,
        timeout_ms: 5000,
    };

    // Create and configure connection manager
    let mut connection_manager = ConnectionManager::new(config.clone());
    connection_manager.with_transport(Box::new(TcpTransport::new(config)));
    connection_manager.with_encryption(
        lair_chat::client::create_server_compatible_encryption()
    );

    // Connect and authenticate
    connection_manager.connect().await?;
    
    let credentials = Credentials {
        username: "alice".to_string(),
        password: "password123".to_string(),
    };
    connection_manager.login(credentials).await?;

    // Send a message
    connection_manager.send_message("Hello, World!").await?;

    Ok(())
}
```

## ✨ Features

### Core Features
- **🔐 End-to-End Encryption**: X25519 key exchange + AES-256-GCM authenticated encryption
- **👥 Multi-User Support**: Concurrent user sessions with proper isolation
- **💬 Real-Time Messaging**: Instant message delivery with observer pattern
- **📨 Direct Messaging**: Private conversations with comprehensive unread message tracking
- **🔔 Smart Notifications**: Visual indicators, count badges, and real-time unread status updates
- **🔑 JWT Authentication**: Secure token-based authentication with configurable expiration
- **📱 Terminal UI**: Rich TUI with Ratatui featuring professional styling and intuitive navigation
- **🔄 Connection Recovery**: Automatic reconnection with exponential backoff

### Technical Features
- **🏗️ Modern Architecture**: Clean separation of concerns with dependency injection
- **⚡ Async/Await**: Non-blocking I/O throughout the application stack
- **🛡️ Type Safety**: Comprehensive error handling with typed errors
- **🧪 Testing**: Extensive test suite with mock implementations
- **📊 Performance**: Optimized for high throughput and low latency
- **🔌 Extensible**: Plugin architecture for transports and encryption

### User Experience
- **🎨 Professional UI**: Clean, modern interface with proper message bubbles
- **📜 Command History**: Persistent command history with navigation
- **⌨️ Tab Completion**: Smart completion for commands and usernames
- **📊 Status Bar**: Real-time connection status, message counts, and uptime
- **🎯 Error Handling**: User-friendly error messages with actionable suggestions
- **💬 DM Navigation**: Intuitive keyboard-driven interface with lobby-based user discovery

## 🏗️ Architecture

Lair Chat v0.6.0 features a layered architecture with clean abstractions:

```
┌─────────────────────────────────────────────────────────────────┐
│                        APPLICATION LAYER                        │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │       TUI       │  │   CLI Handler   │  │  Config Mgmt    │ │
│  │   (Ratatui)     │  │                 │  │                 │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      ORCHESTRATION LAYER                       │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                  ConnectionManager                          │ │
│  │  • Coordinates all subsystems                              │ │
│  │  • Manages connection lifecycle                            │ │
│  │  • Handles observer registration                           │ │
│  │  • Provides unified async API                              │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                    │                │                │
                    ▼                ▼                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      ABSTRACTION LAYER                         │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │   Transport     │  │   Encryption    │  │      Auth       │ │
│  │     Trait       │  │     Trait       │  │    Manager      │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                    │                │                │
                    ▼                ▼                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    IMPLEMENTATION LAYER                        │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │  TcpTransport   │  │ServerCompatible │  │   JWT Auth      │ │
│  │                 │  │   Encryption    │  │                 │ │
│  │ • Tokio sockets │  │ • X25519 + AES  │  │ • Token storage │ │
│  │ • Async I/O     │  │ • Base64 encode │  │ • Session mgmt  │ │
│  │ • Buffer mgmt   │  │ • Key exchange  │  │ • Multi-user    │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Key Components

- **ConnectionManager**: Central orchestrator for all networking operations
- **Transport Layer**: Pluggable network transport (TCP, WebSocket, etc.)
- **Encryption Services**: Multiple encryption implementations (AES-GCM, Server-Compatible)
- **Authentication Manager**: JWT-based authentication with secure token storage
- **Observer Pattern**: Event-driven communication for UI updates

## 📦 Installation

### From Source

```bash
git clone https://github.com/your-org/lair-chat.git
cd lair-chat
cargo build --release
```

### Using Cargo

```bash
cargo install lair-chat
```

### System Requirements

- **OS**: Linux, macOS, Windows
- **Rust**: 1.70 or later
- **Memory**: 4MB+ available RAM
- **Network**: TCP connectivity for client-server communication

## 📖 Usage

### Server

```bash
# Start server with default settings
cargo run --bin lair-chat-server

# Custom port and configuration
LAIR_SERVER_PORT=9090 cargo run --bin lair-chat-server

# Enable debug logging
RUST_LOG=debug cargo run --bin lair-chat-server
```

### Client

```bash
# Start client (interactive mode)
cargo run --bin lair-chat-client

# Connect to custom server
cargo run --bin lair-chat-client -- --server 192.168.1.100:8080

# Non-interactive mode
echo "Hello, World!" | cargo run --bin lair-chat-client -- --username alice --password secret
```

### Direct Messaging

Once connected, you can use the built-in direct messaging system:

- **Lobby System**: All users automatically join a shared "Lobby" room for user discovery
- **Open DM Panel**: Press `Ctrl+L` to access private conversations
- **Start New DM**: Press `n` in the DM panel to select from lobby users
- **Navigate**: Use arrow keys or `j/k` to browse conversations
- **Quick Help**: Press `?` for complete keybinding reference

For detailed DM usage instructions, see [Direct Messaging Guide](docs/DIRECT_MESSAGING.md).

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LAIR_SERVER_ADDRESS` | Server address | "127.0.0.1:8080" |
| `LAIR_TIMEOUT_MS` | Connection timeout | 5000 |
| `LAIR_LOG_LEVEL` | Logging level | "info" |
| `RUST_LOG` | Rust logging configuration | "lair_chat=info" |

## 📚 API Documentation

### Core APIs

- **[ConnectionManager](./API_DOCUMENTATION.md#connectionmanager)**: Main interface for chat operations
- **[Transport Layer](./API_DOCUMENTATION.md#transport-layer)**: Network communication abstraction
- **[Encryption Services](./API_DOCUMENTATION.md#encryption-services)**: Security and encryption
- **[Authentication](./API_DOCUMENTATION.md#authentication)**: User management and sessions
- **[Observer Pattern](./API_DOCUMENTATION.md#observer-pattern)**: Event handling system

### Complete Documentation

- **[📖 API Documentation](./API_DOCUMENTATION.md)**: Complete API reference with examples
- **[🏗️ Architecture Guide](./TRANSPORT_ARCHITECTURE.md)**: Technical architecture documentation
- **[🔄 Migration Guide](./MIGRATION_GUIDE_v0.6.0.md)**: Upgrading from v0.5.x to v0.6.0
- **[📋 Release Notes](./RELEASE_NOTES_v0.6.0.md)**: What's new in v0.6.0

## 💡 Examples

### Basic Chat Client

```rust
use lair_chat::client::{ConnectionManager, TcpTransport, Credentials};
use lair_chat::transport::{ConnectionConfig, ConnectionObserver};
use std::sync::Arc;

struct SimpleObserver;

impl ConnectionObserver for SimpleObserver {
    fn on_message(&self, message: String) {
        println!("📩 {}", message);
    }
    
    fn on_error(&self, error: String) {
        eprintln!("❌ {}", error);
    }
    
    fn on_status_change(&self, connected: bool) {
        println!("🔌 Connection: {}", if connected { "Connected" } else { "Disconnected" });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("127.0.0.1:8080".parse()?);
    let mut manager = ConnectionManager::new(config.clone());
    
    manager.with_transport(Box::new(TcpTransport::new(config)));
    manager.with_encryption(lair_chat::client::create_server_compatible_encryption());
    manager.register_observer(Arc::new(SimpleObserver));
    
    manager.connect().await?;
    
    let credentials = Credentials {
        username: "alice".to_string(),
        password: "password123".to_string(),
    };
    manager.login(credentials).await?;
    
    manager.send_message("Hello from Lair Chat!").await?;
    
    // Keep alive for incoming messages
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    
    Ok(())
}
```

### More Examples

- **[🔗 End-to-End Example](./examples/test_e2e_auth.rs)**: Complete authentication flow
- **[🧪 Testing Example](./examples/test_auth.rs)**: ConnectionManager usage
- **[📁 Examples Directory](./examples/)**: Additional usage examples

## 🛠️ Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Build specific binary
cargo build --bin lair-chat-server --release
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test connection_manager

# Run with output
cargo test -- --nocapture

# Run integration tests
cargo test --test '*'
```

### Benchmarking

```bash
# Run performance benchmarks
cargo bench

# Specific benchmark
cargo bench connection_establishment

# Generate benchmark report
cargo bench -- --output-format html
```

### Development Tools

```bash
# Format code
cargo fmt

# Check for issues
cargo clippy

# Generate documentation
cargo doc --open

# Check dependencies
cargo tree
```

## 📊 Performance

### Benchmarks (v0.6.0)

| Operation | Latency | Throughput | Memory |
|-----------|---------|------------|--------|
| Connection establishment | < 100ms | - | 2MB |
| Message send (encrypted) | < 5ms | 1000+ msg/sec | +1KB/msg |
| Message receive | < 2ms | 1500+ msg/sec | +1KB/msg |
| Authentication | < 200ms | - | +500KB |

### Improvements over v0.5.x

- **📈 60% reduction** in CPU usage during idle
- **🧠 40% reduction** in memory baseline
- **⚡ 2x improvement** in message throughput
- **🚀 5x faster** connection establishment

## 🔄 Migration Guide

### Upgrading from v0.5.x

Lair Chat v0.6.0 introduces breaking changes that require code migration. The new architecture is significantly more powerful and maintainable.

**Key Changes:**
- Global state variables removed (`CLIENT_STATUS`, `MESSAGES`)
- All I/O operations are now async
- String-based errors replaced with typed errors
- Observer pattern for event handling

**Quick Migration:**
```rust
// OLD (v0.5.x)
let status = CLIENT_STATUS.lock().unwrap();
add_text_message("Hello");

// NEW (v0.6.0)
let status = connection_manager.get_status().await;
connection_manager.send_message("Hello").await?;
```

📖 **[Complete Migration Guide](./MIGRATION_GUIDE_v0.6.0.md)** - Step-by-step migration instructions

## 🤝 Contributing

We welcome contributions! Here's how to get started:

### Development Setup

```bash
# Fork and clone the repository
git clone https://github.com/your-username/lair-chat.git
cd lair-chat

# Create a feature branch
git checkout -b feature/my-new-feature

# Make your changes and test
cargo test
cargo clippy

# Commit and push
git commit -m "Add amazing new feature"
git push origin feature/my-new-feature
```

### Guidelines

- **Code Style**: Use `cargo fmt` and follow Rust conventions
- **Testing**: Add tests for new functionality
- **Documentation**: Update docs for API changes
- **Performance**: Consider performance implications
- **Security**: Follow security best practices

### Areas for Contribution

- 🌐 **WebSocket transport** implementation
- 📱 **Mobile client** development
- 🔧 **Plugin system** architecture
- 📊 **Metrics and monitoring** features
- 🎨 **UI/UX improvements** for TUI
- 📚 **Documentation** and examples

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Tokio Team** for excellent async runtime
- **Ratatui Community** for terminal UI framework
- **Rust Community** for amazing ecosystem
- **Contributors** who made this project possible

## 📞 Support

- **Documentation**: See docs in this repository
- **Issues**: [GitHub Issues](https://github.com/your-org/lair-chat/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/lair-chat/discussions)

---

**Lair Chat v0.6.0** - Modern, secure, and performant chat application built with Rust 🦀

Made with ❤️ by the Lair Chat team