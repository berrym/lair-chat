# Lair Chat 🦎

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-0.7.0-green.svg)](docs/releases/CHANGELOG.md)
[![Phase](https://img.shields.io/badge/Phase%207-Complete-success.svg)](PHASE_7_COMPLETION_HANDOFF.md)

A secure, terminal-based chat application built with Rust, featuring end-to-end encryption, direct messaging, real-time communication, comprehensive error handling, advanced security hardening, and production-ready performance monitoring.

## 🚀 Quick Start

```bash
# Install from source
git clone https://github.com/yourusername/lair-chat.git
cd lair-chat
cargo build --release

# Option 1: Quick automated test with multiple clients
./scripts/quick_start.sh

# Option 2: Manual testing
# Terminal 1 - Start server
./target/release/lair-chat-server

# Terminal 2+ - Start clients
./target/release/lair-chat-client

# Option 3: Automated multi-client testing
./scripts/test_multiple_clients.sh -n 3 -a
```

### Testing with Multiple Clients

For comprehensive real-world testing scenarios:

```bash
# Basic multi-client test (3 clients)
make test-multi-client

# Load testing with 10 clients
./scripts/test_multiple_clients.sh -n 10 -a -d 300

# Network testing across machines
./scripts/quick_start.sh  # On server machine
./target/release/lair-chat-client  # On client machines
```

See [Real-World Testing Guide](docs/REAL_WORLD_TESTING_GUIDE.md) for detailed testing scenarios.

## 🎯 Project Status

**Phase 7: Error Handling and Validation - COMPLETED ✅**
- Comprehensive error handling framework with 50+ structured error types
- Input validation system with rate limiting and security checks
- ACID-compliant database transaction management with rollback support
- Advanced security hardening with threat detection and automated response
- Real-time performance monitoring with metrics collection and alerting

**Next: Phase 8 - Testing and Validation**

## ✨ Features

- 🔐 **End-to-end encryption** with AES-GCM and X25519 key exchange
- 💬 **Direct messaging** with conversation history
- 🏠 **Chat rooms** with user management
- 📱 **Modern TUI** with intuitive navigation
- 🔔 **Unread message tracking** with visual indicators
- ⚡ **Real-time messaging** with efficient transport layer
- 🎨 **Customizable styling** and font support
- 🏥 **System health monitoring** with real-time metrics
- 📋 **Comprehensive audit logging** for all user actions
- 🛡️ **Admin dashboard** with complete system oversight
- ⚠️ **Production-ready error handling** with structured recovery mechanisms
- 🛡️ **Advanced security hardening** with threat detection and automated response
- 📊 **Real-time performance monitoring** with metrics and alerting
- 💾 **ACID-compliant transactions** with automatic rollback support
- ✅ **Input validation framework** with rate limiting and security checks

## 📚 Documentation

| Topic | Description |
|-------|-------------|
| [**User Guide**](docs/guides/USER_GUIDE.md) | Complete guide for end users |
| [**Admin Documentation**](docs/admin/README.md) | System administration and management |
| [**API Documentation**](docs/api/README.md) | REST API and WebSocket reference |
| [**Development Guide**](docs/development/DEVELOPMENT_GUIDE.md) | Setup, testing, and contribution guide |
| [**Architecture**](docs/architecture/README.md) | System design and technical details |
| [**Project Progress**](docs/PROJECT_PROGRESS.md) | Current status and phase tracking |
| [**Project Roadmap**](docs/ROADMAP.md) | Strategic direction and future plans |
| [**Phase 7 Completion**](PHASE_7_COMPLETION_HANDOFF.md) | Error handling and validation framework |
| [**Migration Guide**](docs/guides/migration-v0.6.0.md) | Upgrading between versions |
| [**Performance Baselines**](docs/development/performance-baselines.md) | Benchmarks and optimization |
| [**Real-World Testing Guide**](docs/REAL_WORLD_TESTING_GUIDE.md) | Multi-client testing and deployment |

## 🏗️ Architecture

The project follows a clean, modular architecture with clear separation of concerns:

```
┌─────────────────┐    ┌─────────────────┐
│   TUI Client    │◄──►│     Server      │
│                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │     UI      │ │    │ │    App      │ │
│ │ Components  │ │    │ │   Logic     │ │
│ │             │ │    │ └─────────────┘ │
│ └─────────────┘ │    │ ┌─────────────┐ │
│ ┌─────────────┐ │    │ │    Chat     │ │
│ │    Chat     │ │    │ │ Management  │ │
│ │ Management  │ │    │ └─────────────┘ │
│ └─────────────┘ │    │ ┌─────────────┐ │
│                 │    │ │   Network   │ │
│                 │    │ │  Sessions   │ │
│                 │    │ └─────────────┘ │
└─────────────────┘    └─────────────────┘
        │                        │
        └─────── Common ─────────┘
             ┌─────────────┐
             │   Protocol  │
             │    Crypto   │
             │  Transport  │
             └─────────────┘
```

### Project Structure

```
src/
├── bin/                    # Binary entry points
│   ├── client.rs          # Client application
│   └── server.rs          # Server application
├── common/                 # Shared functionality
│   ├── protocol/          # Message types & protocols
│   ├── crypto/            # Encryption utilities
│   ├── transport/         # Network abstractions
│   └── errors/            # Common error types
├── client/                 # Client-specific code
│   ├── ui/components/     # UI components
│   ├── chat/              # Chat functionality
│   ├── auth/              # Authentication
│   └── network/           # Client networking
└── server/                 # Server-specific code
    ├── app/               # Application logic
    ├── chat/              # Message handling
    ├── auth/              # Authentication
    └── network/           # Connection management
```

## 🛠️ Development

```bash
# Run tests
cargo test

# Run performance monitoring tests
cargo test --bin test_performance_monitoring

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
- **Monitoring overhead**: <1ms per operation
- **Framework overhead**: <8.6ms total per operation

Full benchmarks: [Performance Baselines](docs/development/performance-baselines.md)

## 🔒 Security

- **AES-256-GCM** encryption for message content
- **X25519** key exchange for forward secrecy
- **Argon2** password hashing
- **Rate limiting** and session management
- **Memory-safe** Rust implementation
- **Advanced threat detection** with automated IP blocking
- **Security audit logging** with comprehensive event tracking
- **Input validation** with security pattern detection

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

*Last updated: December 2024*