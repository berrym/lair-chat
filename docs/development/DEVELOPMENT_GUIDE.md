# Development Guide 🚀

This guide provides comprehensive information for developers working on Lair Chat, including setup instructions, coding standards, testing practices, and contribution guidelines.

## 📋 Table of Contents

- [Quick Start](#quick-start)
- [Development Environment Setup](#development-environment-setup)
- [Project Structure](#project-structure)
- [Coding Standards](#coding-standards)
- [Testing Strategy](#testing-strategy)
- [Build and Deployment](#build-and-deployment)
- [Debugging and Profiling](#debugging-and-profiling)
- [Contributing Guidelines](#contributing-guidelines)
- [Release Process](#release-process)
- [Troubleshooting](#troubleshooting)

## ⚡ Quick Start

### Prerequisites
- **Rust**: 1.70.0 or later
- **Git**: Latest version
- **PostgreSQL**: 13+ (for database features)
- **Redis**: 6+ (for caching and sessions)

### 5-Minute Setup
```bash
# 1. Clone the repository
git clone https://github.com/your-org/lair-chat.git
cd lair-chat

# 2. Install dependencies and setup environment
make setup

# 3. Run tests to verify setup
make test

# 4. Start development servers
make dev
```

### First Contribution
```bash
# 1. Create a feature branch
git checkout -b feature/your-feature-name

# 2. Make your changes
# 3. Run tests and linting
make check

# 4. Commit and push
git commit -m "feat: add your feature description"
git push origin feature/your-feature-name

# 5. Create a pull request
```

## 🛠️ Development Environment Setup

### System Requirements
- **Operating System**: Linux, macOS, or Windows with WSL2
- **Memory**: 8GB RAM minimum, 16GB recommended
- **Storage**: 10GB free space
- **Network**: Internet connection for dependencies

### Rust Installation
```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install required components
rustup component add clippy rustfmt rust-src

# Install cargo tools
cargo install cargo-watch cargo-expand cargo-audit cargo-deny
```

### Database Setup

#### PostgreSQL
```bash
# Install PostgreSQL (Ubuntu/Debian)
sudo apt update
sudo apt install postgresql postgresql-contrib

# Start PostgreSQL service
sudo systemctl start postgresql
sudo systemctl enable postgresql

# Create database and user
sudo -u postgres createdb lair_chat_dev
sudo -u postgres createuser lair_chat_user -P
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE lair_chat_dev TO lair_chat_user;"
```

#### Redis
```bash
# Install Redis (Ubuntu/Debian)
sudo apt install redis-server

# Start Redis service
sudo systemctl start redis-server
sudo systemctl enable redis-server

# Test Redis connection
redis-cli ping
```

### Environment Configuration
```bash
# Copy environment template
cp .env.example .env

# Edit environment variables
nano .env
```

Example `.env` file:
```env
# Database Configuration
DATABASE_URL=postgresql://lair_chat_user:password@localhost/lair_chat_dev
DATABASE_MAX_CONNECTIONS=10

# Redis Configuration
REDIS_URL=redis://localhost:6379
REDIS_MAX_CONNECTIONS=10

# Server Configuration
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
ADMIN_PORT=9090

# Logging Configuration
RUST_LOG=lair_chat=debug,tower_http=debug
LOG_LEVEL=debug

# Security Configuration
JWT_SECRET=your-super-secret-jwt-key-change-this-in-production
SESSION_SECRET=your-session-secret-key

# File Upload Configuration
MAX_FILE_SIZE=10485760  # 10MB
UPLOAD_DIR=./uploads

# Development Settings
RUST_BACKTRACE=1
DEVELOPMENT_MODE=true
```

### IDE Setup

#### VS Code Configuration
Create `.vscode/settings.json`:
```json
{
    "rust-analyzer.cargo.features": ["dev"],
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.rustfmt.rangeFormatting.enable": true,
    "editor.formatOnSave": true,
    "editor.codeActionsOnSave": {
        "source.fixAll": true
    },
    "files.watcherExclude": {
        "**/target/**": true
    }
}
```

#### Recommended Extensions
- rust-analyzer
- CodeLLDB
- Better TOML
- GitLens
- Thunder Client (for API testing)

## 🏗️ Project Structure

### Directory Layout
```
lair-chat/
├── src/                    # Source code
│   ├── bin/               # Binary entry points
│   │   ├── client.rs      # Client application
│   │   └── server.rs      # Server application
│   ├── common/            # Shared functionality
│   │   ├── protocol/      # Message protocols
│   │   ├── crypto/        # Encryption utilities
│   │   ├── transport/     # Network abstractions
│   │   └── errors/        # Common error types
│   ├── client/            # Client-specific code
│   │   ├── ui/           # User interface components
│   │   ├── chat/         # Chat functionality
│   │   ├── auth/         # Authentication
│   │   └── network/      # Client networking
│   └── server/            # Server-specific code
│       ├── app/          # Application logic
│       ├── chat/         # Message handling
│       ├── auth/         # Authentication
│       ├── api/          # REST API handlers
│       └── network/      # Server networking
├── tests/                 # Integration tests
├── benches/              # Benchmarks
├── examples/             # Example applications
├── docs/                 # Documentation
├── scripts/              # Build and deployment scripts
├── config/               # Configuration files
└── migrations/           # Database migrations
```

### Module Organization
```rust
// Example module structure
pub mod common {
    pub mod protocol;
    pub mod crypto;
    pub mod transport;
    pub mod errors;
}

pub mod client {
    pub mod ui;
    pub mod chat;
    pub mod auth;
    pub mod network;
}

pub mod server {
    pub mod app;
    pub mod chat;
    pub mod auth;
    pub mod api;
    pub mod network;
}
```

### Dependency Management
```toml
# Cargo.toml - Main dependencies
[dependencies]
# Async runtime
tokio = { version = "1.0", features = ["full"] }

# Web framework
axum = "0.7"
tower = "0.4"
tower-http = "0.5"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid"] }
redis = { version = "0.24", features = ["tokio-comp"] }

# Cryptography
ring = "0.17"
x25519-dalek = "2.0"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Configuration
config = "0.14"
clap = { version = "4.0", features = ["derive"] }

[dev-dependencies]
# Testing
tokio-test = "0.4"
criterion = "0.5"
proptest = "1.0"
```

## 📝 Coding Standards

### Rust Style Guide
We follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) and use `rustfmt` with the following configuration:

```toml
# rustfmt.toml
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
merge_derives = true
use_try_shorthand = false
use_field_init_shorthand = false
force_explicit_abi = true
edition = "2021"
```

### Naming Conventions
```rust
// Module names: snake_case
mod user_management;

// Type names: PascalCase
struct UserProfile {
    user_id: UserId,        // Field names: snake_case
    display_name: String,
}

enum MessageType {          // Enum variants: PascalCase
    Text,
    Image,
    File,
}

// Function names: snake_case
fn send_message(content: &str) -> Result<MessageId, ChatError> {
    // Local variables: snake_case
    let message_id = generate_id();
    
    // Constants: SCREAMING_SNAKE_CASE
    const MAX_MESSAGE_LENGTH: usize = 4096;
    
    Ok(message_id)
}

// Trait names: PascalCase
trait MessageHandler {
    fn handle_message(&self, message: Message) -> Result<(), Error>;
}
```

### Error Handling Patterns
```rust
// Use thiserror for error types
#[derive(thiserror::Error, Debug)]
pub enum ChatError {
    #[error("User not found: {user_id}")]
    UserNotFound { user_id: String },
    
    #[error("Database error")]
    Database(#[from] sqlx::Error),
    
    #[error("Network error")]
    Network(#[from] std::io::Error),
}

// Use Result<T, E> for fallible operations
pub async fn send_message(
    pool: &PgPool,
    message: NewMessage,
) -> Result<Message, ChatError> {
    let message = sqlx::query_as!(
        Message,
        "INSERT INTO messages (content, user_id, room_id) VALUES ($1, $2, $3) RETURNING *",
        message.content,
        message.user_id,
        message.room_id
    )
    .fetch_one(pool)
    .await?;
    
    Ok(message)
}

// Use anyhow for main functions and tests
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Application logic
    Ok(())
}
```

### Documentation Standards
```rust
//! Module-level documentation
//! 
//! This module provides chat functionality including message sending,
//! room management, and user presence tracking.

/// Send a message to a chat room.
/// 
/// # Arguments
/// 
/// * `pool` - Database connection pool
/// * `message` - The message to send
/// 
/// # Returns
/// 
/// Returns the created message with generated ID and timestamp.
/// 
/// # Errors
/// 
/// This function will return an error if:
/// * The database connection fails
/// * The user doesn't have permission to send messages
/// * The message content exceeds the maximum length
/// 
/// # Examples
/// 
/// ```rust
/// use lair_chat::chat::send_message;
/// 
/// let message = NewMessage {
///     content: "Hello, world!".to_string(),
///     user_id: "user_123".to_string(),
///     room_id: "room_456".to_string(),
/// };
/// 
/// let sent_message = send_message(&pool, message).await?;
/// println!("Message sent: {}", sent_message.id);
/// ```
pub async fn send_message(
    pool: &PgPool,
    message: NewMessage,
) -> Result<Message, ChatError> {
    // Implementation
}
```

### Code Organization Patterns
```rust
// Use modules to organize related functionality
pub mod auth {
    pub mod handlers;
    pub mod middleware;
    pub mod models;
    pub mod service;
    
    pub use service::AuthService;
}

// Use type aliases for complex types
pub type UserId = uuid::Uuid;
pub type Result<T> = std::result::Result<T, ChatError>;

// Use builder pattern for complex configurations
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
}

impl ServerConfig {
    pub fn builder() -> ServerConfigBuilder {
        ServerConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct ServerConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    database_url: Option<String>,
    redis_url: Option<String>,
}

impl ServerConfigBuilder {
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }
    
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }
    
    pub fn build(self) -> Result<ServerConfig, ConfigError> {
        Ok(ServerConfig {
            host: self.host.unwrap_or_else(|| "127.0.0.1".to_string()),
            port: self.port.unwrap_or(8080),
            database_url: self.database_url.ok_or(ConfigError::MissingDatabaseUrl)?,
            redis_url: self.redis_url.ok_or(ConfigError::MissingRedisUrl)?,
        })
    }
}
```

## 🧪 Testing Strategy

### Test Organization
```
tests/
├── integration/           # Integration tests
│   ├── api/              # API endpoint tests
│   ├── auth/             # Authentication tests
│   └── chat/             # Chat functionality tests
├── unit/                 # Unit tests (also in src/ files)
├── fixtures/             # Test data and utilities
└── common/               # Shared test utilities
```

### Unit Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_send_message_success() {
        // Arrange
        let pool = create_test_pool().await;
        let message = NewMessage {
            content: "Test message".to_string(),
            user_id: "user_123".to_string(),
            room_id: "room_456".to_string(),
        };
        
        // Act
        let result = send_message(&pool, message).await;
        
        // Assert
        assert!(result.is_ok());
        let sent_message = result.unwrap();
        assert_eq!(sent_message.content, "Test message");
        assert!(!sent_message.id.is_empty());
    }
    
    #[tokio::test]
    async fn test_send_message_invalid_user() {
        let pool = create_test_pool().await;
        let message = NewMessage {
            content: "Test message".to_string(),
            user_id: "invalid_user".to_string(),
            room_id: "room_456".to_string(),
        };
        
        let result = send_message(&pool, message).await;
        
        assert!(matches!(result, Err(ChatError::UserNotFound { .. })));
    }
}
```

### Integration Testing
```rust
// tests/integration/api/messages.rs
use lair_chat::server::create_app;
use axum_test::TestServer;
use serde_json::json;

#[tokio::test]
async fn test_send_message_api() {
    // Setup test server
    let app = create_app().await;
    let server = TestServer::new(app).unwrap();
    
    // Create test user and get auth token
    let auth_response = server
        .post("/api/v1/auth/login")
        .json(&json!({
            "username": "test_user",
            "password": "test_password"
        }))
        .await;
    
    let token = auth_response.json::<AuthResponse>().access_token;
    
    // Send message
    let response = server
        .post("/api/v1/rooms/test_room/messages")
        .add_header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "content": "Hello, world!",
            "type": "text"
        }))
        .await;
    
    assert_eq!(response.status_code(), 201);
    
    let message: Message = response.json();
    assert_eq!(message.content, "Hello, world!");
}
```

### Property-Based Testing
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_message_validation(
        content in ".*",
        user_id in "[a-zA-Z0-9_]{1,50}",
        room_id in "[a-zA-Z0-9_]{1,50}"
    ) {
        let message = NewMessage {
            content: content.clone(),
            user_id: user_id.clone(),
            room_id: room_id.clone(),
        };
        
        let validation_result = validate_message(&message);
        
        // Messages with empty content should be invalid
        if content.is_empty() {
            assert!(validation_result.is_err());
        }
        
        // Messages exceeding max length should be invalid
        if content.len() > MAX_MESSAGE_LENGTH {
            assert!(validation_result.is_err());
        }
        
        // Valid messages should pass validation
        if !content.is_empty() && content.len() <= MAX_MESSAGE_LENGTH {
            assert!(validation_result.is_ok());
        }
    }
}
```

### Performance Testing
```rust
// benches/message_throughput.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lair_chat::chat::*;

fn benchmark_message_processing(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(create_test_pool());
    
    c.bench_function("send_message", |b| {
        b.to_async(&rt).iter(|| async {
            let message = NewMessage {
                content: black_box("Benchmark message content".to_string()),
                user_id: black_box("user_123".to_string()),
                room_id: black_box("room_456".to_string()),
            };
            
            send_message(&pool, message).await.unwrap()
        })
    });
}

criterion_group!(benches, benchmark_message_processing);
criterion_main!(benches);
```

### Test Utilities
```rust
// tests/common/mod.rs
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://test:test@localhost/lair_chat_test".to_string());
    
    PgPool::connect(&database_url).await.unwrap()
}

pub fn create_test_user() -> User {
    User {
        id: Uuid::new_v4(),
        username: format!("test_user_{}", Uuid::new_v4()),
        email: format!("test_{}@example.com", Uuid::new_v4()),
        created_at: chrono::Utc::now(),
    }
}

pub async fn setup_test_room(pool: &PgPool) -> Room {
    let room = Room {
        id: Uuid::new_v4(),
        name: format!("test_room_{}", Uuid::new_v4()),
        description: Some("Test room".to_string()),
        created_at: chrono::Utc::now(),
    };
    
    sqlx::query!(
        "INSERT INTO rooms (id, name, description, created_at) VALUES ($1, $2, $3, $4)",
        room.id,
        room.name,
        room.description,
        room.created_at
    )
    .execute(pool)
    .await
    .unwrap();
    
    room
}
```

### Running Tests
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_send_message_success

# Run integration tests only
cargo test --test integration

# Run tests with output
cargo test -- --nocapture

# Run tests in parallel
cargo test --jobs 4

# Generate coverage report
cargo tarpaulin --out html --output-dir coverage/
```

## 🔨 Build and Deployment

### Development Build
```bash
# Debug build (fast compilation, slower runtime)
cargo build

# Release build (slow compilation, fast runtime)
cargo build --release

# Build specific binary
cargo build --bin lair-chat-server

# Build with specific features
cargo build --features "database,redis"
```

### Build Scripts
```bash
# scripts/build.sh
#!/bin/bash
set -euo pipefail

echo "Building Lair Chat..."

# Check for required tools
command -v cargo >/dev/null 2>&1 || { echo "Rust/Cargo not found"; exit 1; }

# Run pre-build checks
echo "Running pre-build checks..."
cargo fmt --check
cargo clippy -- -D warnings
cargo audit

# Build the project
echo "Building release binary..."
cargo build --release

# Run tests
echo "Running tests..."
cargo test --release

echo "Build completed successfully!"
```

### Docker Configuration
```dockerfile
# Dockerfile
FROM rust:1.70 as builder

WORKDIR /app
COPY . .

# Build dependencies first for better caching
RUN cargo build --release --bin lair-chat-server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/lair-chat-server /usr/local/bin/
COPY --from=builder /app/config/ /etc/lair-chat/

EXPOSE 8080 9090

CMD ["lair-chat-server", "--config", "/etc/lair-chat/config.toml"]
```

```yaml
# docker-compose.yml
version: '3.8'

services:
  lair-chat:
    build: .
    ports:
      - "8080:8080"
      - "9090:9090"
    environment:
      - DATABASE_URL=postgresql://postgres:password@db:5432/lair_chat
      - REDIS_URL=redis://redis:6379
    depends_on:
      - db
      - redis
    volumes:
      - ./uploads:/app/uploads

  db:
    image: postgres:15
    environment:
      POSTGRES_DB: lair_chat
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./migrations:/docker-entrypoint-initdb.d

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data

volumes:
  postgres_data:
  redis_data:
```

### CI/CD Configuration
```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: lair_chat_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432
          
      redis:
        image: redis:7
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 6379:6379

    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy
        
    - name: Cache cargo
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target/
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
        
    - name: Check formatting
      run: cargo fmt --check
      
    - name: Run clippy
      run: cargo clippy -- -D warnings
      
    - name: Run tests
      run: cargo test --verbose
      env:
        DATABASE_URL: postgresql://postgres:postgres@localhost:5432/lair_chat_test
        REDIS_URL: redis://localhost:6379
        
    - name: Run security audit
      run: cargo audit
```

## 🐛 Debugging and Profiling

### Logging Setup
```rust
// Initialize tracing
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_file(true)
                .with_line_number(true)
                .with_level(true)
                .with_ansi(true)
        )
        .with(EnvFilter::from_default_env())
        .init();
}

// Use structured logging
#[tracing::instrument(skip(pool))]
pub async fn send_message(
    pool: &PgPool,
    message: NewMessage,
) -> Result<Message, ChatError> {
    tracing::info!("Sending message to room {}", message.room_id);
    
    let result = sqlx::query_as!(/* ... */)
        .fetch_one(pool)
        .await;
    
    match &result {
        Ok(msg) => tracing::info!("Message sent successfully: {}", msg.id),
        Err(e) => tracing::error!("Failed to send message: {}", e),
    }
    
    result.map_err(Into::into)
}
```

### Debug Configuration
```bash
# Enable debug logging
export RUST_LOG=lair_chat=debug,sqlx=info,tower_http=debug

# Enable backtraces
export RUST_BACKTRACE=1

# Enable full backtraces
export RUST_BACKTRACE=full

# Run with debug symbols
cargo run --bin lair-chat-server
```

### Performance Profiling
```bash
# Install profiling tools
cargo install flamegraph
sudo apt install linux-tools-common linux-tools-generic

# Generate flame graph
sudo cargo flamegraph --bin lair-chat-server

# Memory profiling with valgrind
cargo build --bin lair-chat-server
valgrind --tool=massif target/debug/lair-chat-server

# CPU profiling with perf
perf record -g cargo run --release --bin lair-chat-server
perf report
```

### Common Debugging Scenarios
```rust
// Debug async code
#[tokio::main]
async fn main() {
    // Enable tokio console
    console_subscriber::init();
    
    // Your async code here
}

// Debug database queries
let query_result = sqlx::query!("SELECT * FROM users WHERE id = $1", user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database query failed: {:?}", e);
        e
    })?;

// Debug WebSocket connections
#[tracing::instrument(skip(socket))]
async fn handle_websocket(socket: WebSocket) {
    tracing::info!("New WebSocket connection established");
    
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(msg) => {
                tracing::debug!("Received message: {:?}", msg);
                // Handle message
            }
            Err(e) => {
                tracing::error!("WebSocket error: {:?}", e);
                break;
            }
        }
    }
    
    tracing::info!("WebSocket connection closed");
}
```

## 🤝 Contributing Guidelines

### Getting Started
1. **Fork the repository** on GitHub
2. **Clone your fork** locally
3. **Create a feature branch** from `develop`
4. **Make your changes** following our coding standards
5. **Write tests** for your changes
6. **Run the test suite** to ensure everything works
7. **Commit your changes** with descriptive messages
8. **Push to your fork** and create a pull request

### Commit Message Format
We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Types:
- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Changes that don't affect meaning (whitespace, formatting)
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvements
- `test`: Adding or modifying tests
- `chore`: Maintenance tasks

Examples:
```
feat(auth): add JWT token refresh functionality

Implement automatic token refresh to improve user experience
and reduce login frequency.

Closes #123

fix(websocket): handle connection drops gracefully

Previously, unexpected connection drops would cause the server
to panic. Now they are handled gracefully with proper cleanup.

docs(api): update authentication endpoint documentation

Add examples for new JWT refresh endpoint and clarify
error response formats.
```

### Pull Request Process
1. **Update documentation** if you've made API changes
2. **Add tests** for new functionality
3. **Ensure all tests pass** and code follows style guidelines
4. **Update CHANGELOG.md** if your changes are user-facing
5. **Request review** from maintainers
6. **Address feedback** and update your PR as needed

### Code Review Checklist
- [ ] Code follows the project's style guidelines
- [ ] Self-review of the code has been performed
- [ ] Code is well-commented, particularly in hard-to-understand areas
- [ ] Tests have been added that prove the fix is effective or feature works
- [ ] New and existing unit tests pass locally
- [ ] Any dependent changes have been merged and published

### Development Workflow
```bash
# 1. Start from develop branch
git checkout develop
git pull origin develop

# 2. Create feature branch
git checkout -b feature/your-feature-name

# 3. Make changes and commit frequently
git add .
git commit -m "feat: implement basic functionality"

# 4. Push to your fork
git push origin feature/your-feature-name

# 5. Create pull request on GitHub

# 6. After review and approval, merge via GitHub

# 7. Clean up
git checkout develop
git pull origin develop
git branch -d feature/your-feature-name
```

## 🚀 Release Process

### Version Management
We use [Semantic Versioning](https://semver.org/):
- **MAJOR** version for incompatible API changes
- **MINOR** version for backwards-compatible functionality additions
- **PATCH** version for backwards-compatible bug fixes

### Release Checklist
- [ ] All tests pass
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated
- [ ] Version numbers are bumped in Cargo.toml
- [ ] Git tag is created
- [ ] Release notes are written
- [ ] Binaries are built and published

### Automated Release
```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
      
    - name: Build release
      run: cargo build --release
      
    - name: Create release
      uses: actions/create-release@v1
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      with:
        tag_