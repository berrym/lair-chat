# Contributing to Lair Chat

Thank you for your interest in contributing to Lair Chat! This document provides guidelines and information for contributors.

## 🚀 Quick Start for Contributors

### Prerequisites
- **Rust 1.70+** - [Install from rustup.rs](https://rustup.rs/)
- **Git** - For version control
- **Basic familiarity** with Rust, async programming, and REST APIs

### Setting Up Development Environment

1. **Fork and clone the repository**
   ```bash
   git clone https://github.com/your-username/lair-chat.git
   cd lair-chat
   ```

2. **Start development environment**
   ```bash
   ./scripts/dev.sh
   ```

3. **Verify everything works**
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```

## 🛠️ Development Workflow

### Code Style and Standards

We follow Rust's official style guidelines:

- **Formatting**: Use `cargo fmt` to format code
- **Linting**: Use `cargo clippy` to catch common mistakes
- **Testing**: Write tests for new functionality
- **Documentation**: Document public APIs and complex logic

### Code Quality Requirements

Before submitting a pull request:

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Run tests
cargo test

# Run integration tests
cargo test --test integration

# Verify documentation builds
cargo doc --no-deps
```

### Commit Message Guidelines

We use conventional commits for clear history:

```
type(scope): description

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Examples:**
```
feat(api): add user profile update endpoint

fix(client): resolve message ordering issue

docs(readme): update installation instructions

test(auth): add JWT token validation tests
```

## 📋 Types of Contributions

### 🐛 Bug Reports

When reporting bugs, please include:

1. **Clear description** of the issue
2. **Steps to reproduce** the problem
3. **Expected vs actual behavior**
4. **Environment details** (OS, Rust version, etc.)
5. **Logs or error messages** if available

Use our bug report template:

```markdown
## Bug Description
Brief description of the bug

## Steps to Reproduce
1. Start the server with `./scripts/start.sh`
2. Connect TUI client
3. ...

## Expected Behavior
What should happen

## Actual Behavior
What actually happens

## Environment
- OS: [e.g., Ubuntu 22.04]
- Rust: [e.g., 1.75.0]
- Lair Chat: [e.g., main branch, commit abc123]

## Additional Context
Any other relevant information
```

### ✨ Feature Requests

For new features:

1. **Check existing issues** to avoid duplicates
2. **Describe the use case** and problem it solves
3. **Propose a solution** or approach
4. **Consider backwards compatibility**

### 🔧 Code Contributions

#### Areas for Contribution

**Core Platform:**
- Performance optimizations
- Security enhancements
- Database optimizations
- Error handling improvements

**API Enhancements:**
- New REST endpoints
- WebSocket functionality
- Rate limiting improvements
- Authentication features

**Client Development:**
- Mobile clients (React Native, Flutter)
- Desktop clients (Tauri, Electron)
- CLI clients and tools
- Browser extensions

**Infrastructure:**
- Docker improvements
- Kubernetes deployments
- Monitoring and observability
- Load testing tools

**Documentation:**
- API documentation
- Developer guides
- Deployment guides
- Performance tuning

### 🧪 Testing Contributions

Help improve our test coverage:

- Unit tests for core functionality
- Integration tests for API endpoints
- Performance benchmarks
- Security testing
- Load testing scenarios

## 📝 Pull Request Process

### Before Submitting

1. **Create an issue** for discussion (for significant changes)
2. **Fork the repository** and create a feature branch
3. **Write tests** for your changes
4. **Update documentation** as needed
5. **Ensure CI passes** locally

### Submitting a Pull Request

1. **Create a clear title** describing the change
2. **Fill out the PR template** completely
3. **Reference related issues** using keywords (fixes #123)
4. **Request review** from maintainers
5. **Address feedback** promptly

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Tests added for new functionality

## Related Issues
Fixes #(issue number)
```

### Review Process

1. **Automated checks** must pass (CI/CD)
2. **Code review** by maintainers
3. **Testing** on development environment
4. **Approval** and merge by maintainers

## 🏗️ Architecture Guidelines

### Project Structure

```
src/
├── bin/                    # Executable binaries
├── client/                 # TUI client implementation
├── server/                 # Server implementation
│   ├── api/               # REST API layer
│   ├── storage/           # Database abstraction
│   └── config/            # Configuration management
└── shared_types/          # Common types and utilities
```

### Design Principles

1. **Modularity**: Keep components loosely coupled
2. **Performance**: Optimize for high concurrency
3. **Security**: Security-first design approach
4. **Reliability**: Comprehensive error handling
5. **Maintainability**: Clear, documented code

### API Design Guidelines

- **RESTful principles** for HTTP endpoints
- **Consistent error responses** across all endpoints
- **Comprehensive input validation**
- **Rate limiting** for abuse prevention
- **OpenAPI documentation** for all endpoints

### Database Guidelines

- **Use SQLx** for database operations
- **Write migrations** for schema changes
- **Index frequently queried fields**
- **Use transactions** for multi-step operations
- **Implement connection pooling**

## 🔒 Security Guidelines

### Security Requirements

- **Input validation** on all user inputs
- **SQL injection prevention** via parameterized queries
- **XSS prevention** in web interfaces
- **Rate limiting** to prevent abuse
- **Secure defaults** in configuration

### Reporting Security Issues

**DO NOT** create public issues for security vulnerabilities.

Instead:
1. Email security@lair-chat.org
2. Include detailed description
3. Provide reproduction steps
4. Allow time for fix before disclosure

## 📚 Documentation

### Documentation Standards

- **Clear, concise language**
- **Code examples** for APIs
- **Step-by-step instructions** for setup
- **Screenshots** for UI components
- **Keep documentation updated** with code changes

### Documentation Types

- **API Documentation**: OpenAPI/Swagger specs
- **Developer Guides**: Setup and contribution guides
- **User Documentation**: End-user instructions
- **Architecture Docs**: System design and decisions

## 🤝 Community Guidelines

### Code of Conduct

We are committed to providing a welcoming and inclusive environment:

- **Be respectful** and constructive
- **Welcome newcomers** and help them learn
- **Focus on the technical merits** of contributions
- **Avoid personal attacks** or discriminatory language
- **Follow the Golden Rule**: treat others as you'd like to be treated

### Getting Help

- **GitHub Discussions**: For general questions and ideas
- **GitHub Issues**: For bugs and feature requests
- **Documentation**: Check docs/ directory first
- **Code Review**: Ask questions during PR review

### Recognition

Contributors are recognized through:

- **Contributor list** in README
- **Release notes** for significant contributions
- **Special recognition** for outstanding contributions

## 🚀 Release Process

### Version Numbering

We use [Semantic Versioning](https://semver.org/):
- **MAJOR**: Breaking changes
- **MINOR**: New features (backwards compatible)
- **PATCH**: Bug fixes (backwards compatible)

### Release Cycle

- **Regular releases** every 2-4 weeks
- **Hotfix releases** for critical bugs
- **Beta releases** for major features

## 📊 Performance Guidelines

### Performance Standards

- **API responses**: < 100ms for simple operations
- **Memory usage**: < 100MB for typical workloads
- **Concurrent connections**: Support 10,000+ simultaneous users
- **Database queries**: < 10ms for indexed operations

### Benchmarking

```bash
# Run performance tests
cargo bench

# Load testing
./scripts/load-test.sh --clients 1000 --duration 300

# Memory profiling
cargo build --release
valgrind --tool=massif target/release/lair-chat-server-new
```

## 🎯 Future Roadmap

### Planned Features

- **Real-time notifications** via WebSocket
- **File sharing** capabilities
- **End-to-end encryption** for messages
- **Mobile applications**
- **Federation** with other chat systems
- **Plugin system** for extensibility

### Long-term Goals

- **Horizontal scaling** across multiple servers
- **Cloud-native deployment** options
- **Enterprise features** (SSO, audit logging)
- **Performance optimizations**
- **Security certifications**

## 📞 Contact

- **GitHub**: [Issues](https://github.com/your-org/lair-chat/issues) and [Discussions](https://github.com/your-org/lair-chat/discussions)
- **Email**: contribute@lair-chat.org
- **Documentation**: [docs/](docs/)

---

Thank you for contributing to Lair Chat! Together we're building an amazing secure chat platform. 🚀