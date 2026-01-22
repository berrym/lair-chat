# Contributing to Lair Chat

Thank you for your interest in contributing to Lair Chat! This document provides guidelines and information for contributors.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/lair-chat.git
   cd lair-chat
   ```
3. **Add the upstream remote**:
   ```bash
   git remote add upstream https://github.com/berrym/lair-chat.git
   ```

## Development Setup

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- SQLite (usually pre-installed on most systems)

### Building

```bash
# Build all crates
cargo build --workspace

# Run the server
cargo run -p lair-chat-server

# Run the client (in another terminal)
cargo run -p lair-chat-client
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run with logging
RUST_LOG=debug cargo test --workspace

# Run a specific test
cargo test test_name
```

### Code Quality

Before submitting a PR, ensure your code passes all checks:

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace -- -D warnings

# Run tests
cargo test --workspace
```

## Pull Request Process

1. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** with clear, descriptive commits

3. **Write tests** for new functionality

4. **Update documentation** if needed (README, doc comments, protocol docs)

5. **Ensure CI passes** - all checks must be green

6. **Submit a Pull Request** with:
   - Clear description of changes
   - Reference to any related issues
   - Screenshots for UI changes (if applicable)

## Code Style

- Follow Rust conventions and idioms
- Use `cargo fmt` for formatting
- Write doc comments for public APIs
- Keep functions focused and small
- Prefer explicit error handling over panics

## Commit Messages

Use clear, descriptive commit messages:

```
Add user presence tracking to chat rooms

- Implement online/offline status broadcasting
- Add presence indicators to room member list
- Update protocol documentation
```

## Architecture Guidelines

- **Domain types** (`src/domain/`) should be pure - no I/O or async
- **Core services** (`src/core/`) contain business logic
- **Adapters** (`src/adapters/`) handle protocol-specific concerns
- **Storage** (`src/storage/`) implements repository traits

See [docs/architecture/](docs/architecture/) for detailed documentation.

## Reporting Issues

When reporting bugs, please include:

- Rust version (`rustc --version`)
- Operating system
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs or error messages

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

## Questions?

Feel free to open an issue for any questions about contributing.
