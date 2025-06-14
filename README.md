# Lair Chat 🦎

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-0.6.2-green.svg)](docs/releases/CHANGELOG.md)

A secure, terminal-based chat application built with Rust, featuring end-to-end encryption, direct messaging, and real-time communication.

## 🚀 Quick Start

```bash
# Install from source
git clone https://github.com/yourusername/lair-chat.git
cd lair-chat
cargo build --release

# Start server
./target/release/lair-chat-server

# Start client (in another terminal)
./target/release/lair-chat-client
```

## ✨ Features

- 🔐 **End-to-end encryption** with AES-GCM and X25519 key exchange
- 💬 **Direct messaging** with conversation history
- 🏠 **Chat rooms** with user management
- 📱 **Modern TUI** with intuitive navigation
- 🔔 **Unread message tracking** with visual indicators
- ⚡ **Real-time messaging** with efficient transport layer
- 🎨 **Customizable styling** and font support

## 📚 Documentation

| Topic | Description |
|-------|-------------|
| [**User Guide**](docs/guides/USER_GUIDE.md) | Complete guide for end users |
| [**Project Roadmap**](docs/ROADMAP.md) | Strategic direction and future plans |
| [**API Documentation**](docs/api/README.md) | Comprehensive API reference |
| [**Development Guide**](docs/development/DEVELOPMENT_GUIDE.md) | Setup and contribution guide |
| [**Architecture**](docs/architecture/README.md) | System design and components |
| [**Migration Guide**](docs/guides/migration-v0.6.0.md) | Upgrading between versions |

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐
│   TUI Client    │◄──►│     Server      │
│                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │  Chat UI    │ │    │ │ Room Manager│ │
│ │             │ │    │ │             │ │
│ │ DM Interface│ │    │ │Auth Service │ │
│ └─────────────┘ │    │ │             │ │
│                 │    │ │   Storage   │ │
│ ┌─────────────┐ │    │ └─────────────┘ │
│ │ Encryption  │ │    │                 │
│ │   Layer     │ │    │                 │
│ └─────────────┘ │    │                 │
└─────────────────┘    └─────────────────┘
        │                        │
        └──── Encrypted TCP ─────┘
```

## 🛠️ Development

```bash
# Run tests
cargo test

# Run benchmarks
cargo bench

# Check for issues
cargo clippy
cargo fmt --check
```

See [Development Guide](docs/development/DEVELOPMENT_GUIDE.md) for detailed setup instructions.

## 📊 Performance

- **Message throughput**: 10,000+ messages/second
- **Latency**: Sub-millisecond local network
- **Memory usage**: <50MB typical client session
- **Encryption overhead**: <5% performance impact

Full benchmarks: [Performance Baselines](docs/development/performance-baselines.md)

## 🔒 Security

- **AES-256-GCM** encryption for message content
- **X25519** key exchange for forward secrecy
- **Argon2** password hashing
- **Rate limiting** and session management
- **Memory-safe** Rust implementation

Security audit: [Security Documentation](docs/architecture/authentication.md)

## 🤝 Contributing

We welcome contributions! Please see our [Development Guide](docs/development/DEVELOPMENT_GUIDE.md) for:

- Setting up the development environment
- Code style and standards
- Testing requirements
- Submission process

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/lair-chat/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/lair-chat/discussions)
- **Documentation**: [Full Documentation](docs/)

---

**Note**: Lair Chat requires [compatible fonts](docs/guides/font-compatibility.md) for the best visual experience. See the font guide for setup instructions.

---

*Last updated: June 2025*