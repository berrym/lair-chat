# Lair Chat API Documentation v0.6.0

**Version**: 0.6.0  
**Last Updated**: December 12, 2025  
**API Stability**: Stable

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Core APIs](#core-apis)
4. [Transport Layer](#transport-layer)
5. [Encryption Services](#encryption-services)
6. [Authentication](#authentication)
7. [Error Handling](#error-handling)
8. [Observer Pattern](#observer-pattern)
9. [Configuration](#configuration)
10. [Examples](#examples)
11. [Migration Guide](#migration-guide)

## Overview

The Lair Chat v0.6.0 API provides a modern, async-first interface for building chat applications. The API is built around the `ConnectionManager` which orchestrates transport, encryption, and authentication services.

### Core Principles

- **Async/Await**: All I/O operations are async
- **Type Safety**: Comprehensive error types
- **Dependency Injection**: Configurable components
- **Observer Pattern**: Event-driven notifications
- **Thread Safety**: Arc<Mutex<>> for shared state

## Quick Start

### Basic Usage

```rust
use lair_chat::client::{ConnectionManager, Credentials, TcpTransport};
use lair_chat::transport::ConnectionConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create configuration
    let config = ConnectionConfig {
        address: "127.0.0.1:8080".parse()?,
        timeout_ms: 5000,
    };

    // 2. Create connection manager
    let mut connection_manager = ConnectionManager::new(config.clone());

    // 3. Configure transport
    let transport = Box::new(TcpTransport::new(config));
    connection_manager.with_transport(transport);

    // 4. Configure encryption
    let encryption = lair_chat::client::create_server_compatible_encryption();
    connection_manager.with_encryption(encryption);

    // 5. Connect and authenticate
    connection_manager.connect().await?;
    
    let credentials = Credentials {
        username: "alice".to_string(),
        password: "password123".to_string(),
    };
    connection_manager.login(credentials).await?;

    // 6. Send a message
    connection_manager.send_message("Hello, World!".to_string()).await?;

    Ok(())
}
```

## Core APIs

### ConnectionManager

The `ConnectionManager` is the primary interface for all chat operations.

```rust
pub struct ConnectionManager {
    // Private fields
}

impl ConnectionManager {
    /// Create a new connection manager with the given configuration
    pub fn new(config: ConnectionConfig) -> Self

    /// Configure the transport layer
    pub fn with_transport(&mut self, transport: Box<dyn Transport + Send + Sync>) -> &mut Self

    /// Configure the encryption service
    pub fn with_encryption(&mut self, encryption: Box<dyn EncryptionService + Send + Sync>) -> &mut Self

    /// Register an observer for connection events
    pub fn register_observer(&mut self, observer: Arc<dyn ConnectionObserver + Send + Sync>)

    /// Establish connection to the server
    pub async fn connect(&mut self) -> Result<(), TransportError>

    /// Close the connection
    pub async fn disconnect(&mut self) -> Result<(), TransportError>

    /// Get current connection status
    pub async fn get_status(&self) -> ConnectionStatus

    /// Authenticate with the server (login)
    pub async fn login(&mut self, credentials: Credentials) -> Result<(), AuthError>

    /// Register a new user account
    pub async fn register(&mut self, credentials: Credentials) -> Result<(), AuthError>

    /// Check if currently authenticated
    pub async fn is_authenticated(&self) -> bool

    /// Send a message
    pub async fn send_message(&mut self, message: &str) -> Result<(), MessageError>

    /// Get the message store for retrieving chat history
    pub fn get_message_store(&self) -> Arc<Mutex<MessageStore>>
}
```

#### Usage Examples

**Basic Connection:**
```rust
let mut manager = ConnectionManager::new(config);
manager.with_transport(Box::new(TcpTransport::new(config)));
manager.connect().await?;
```

**With Encryption:**
```rust
let encryption = lair_chat::client::create_server_compatible_encryption();
manager.with_encryption(encryption);
manager.connect().await?;
```

**Authentication:**
```rust
let credentials = Credentials {
    username: "user123".to_string(),
    password: "secure_password".to_string(),
};
manager.login(credentials).await?;
```

### ConnectionStatus

Represents the current connection state.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    CONNECTED,
    DISCONNECTED,
}
```

### Credentials

User authentication credentials.

```rust
#[derive(Debug, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl Credentials {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }
}
```

### ConnectionConfig

Configuration for connection establishment.

```rust
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub address: std::net::SocketAddr,
    pub timeout_ms: u64,
}

impl ConnectionConfig {
    pub fn new(address: std::net::SocketAddr) -> Self {
        Self {
            address,
            timeout_ms: 5000,
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}
```

## Transport Layer

### Transport Trait

The core transport abstraction for network communication.

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    /// Establish a connection
    async fn connect(&mut self) -> Result<(), TransportError>;

    /// Send data over the transport
    async fn send(&mut self, data: &str) -> Result<(), TransportError>;

    /// Receive data from the transport
    async fn receive(&mut self) -> Result<Option<String>, TransportError>;

    /// Close the transport connection
    async fn close(&mut self) -> Result<(), TransportError>;
}
```

### TcpTransport

Production TCP transport implementation.

```rust
pub struct TcpTransport {
    // Private fields
}

impl TcpTransport {
    /// Create a new TCP transport with the given configuration
    pub fn new(config: ConnectionConfig) -> Self

    /// Get the local address of the connection (if connected)
    pub fn local_addr(&self) -> Option<std::net::SocketAddr>

    /// Get the remote address of the connection (if connected)
    pub fn remote_addr(&self) -> Option<std::net::SocketAddr>
}
```

#### Usage Examples

**Basic TCP Transport:**
```rust
let config = ConnectionConfig::new("127.0.0.1:8080".parse()?);
let transport = TcpTransport::new(config);
connection_manager.with_transport(Box::new(transport));
```

**Custom Timeout:**
```rust
let config = ConnectionConfig::new("127.0.0.1:8080".parse()?)
    .with_timeout(10000); // 10 second timeout
let transport = TcpTransport::new(config);
```

### TransportError

Errors that can occur during transport operations.

```rust
#[derive(Debug)]
pub enum TransportError {
    /// Connection-related errors
    ConnectionError(std::io::Error),
    
    /// Encryption-related errors
    EncryptionError(EncryptionError),
    
    /// Operation timeout
    TimeoutError,
    
    /// Protocol violation
    ProtocolError(String),
}

impl std::fmt::Display for TransportError { /* ... */ }
impl std::error::Error for TransportError { /* ... */ }
impl From<std::io::Error> for TransportError { /* ... */ }
impl From<EncryptionError> for TransportError { /* ... */ }
```

## Encryption Services

### EncryptionService Trait

Abstraction for encryption operations.

```rust
#[async_trait]
pub trait EncryptionService: Send + Sync {
    /// Encrypt plaintext
    fn encrypt(&self, key: &str, plaintext: &str) -> Result<String, EncryptionError>;

    /// Decrypt ciphertext
    fn decrypt(&self, key: &str, ciphertext: &str) -> Result<String, EncryptionError>;

    /// Perform encryption handshake with remote peer
    async fn perform_handshake(&mut self, transport: &mut dyn Transport) -> Result<(), TransportError>;
}
```

### ServerCompatibleEncryption

Production encryption service compatible with the Lair Chat server.

```rust
pub struct ServerCompatibleEncryption {
    // Private fields
}

impl ServerCompatibleEncryption {
    /// Create a new server-compatible encryption service
    pub fn new() -> Self

    /// Check if handshake has been completed
    pub fn is_handshake_complete(&self) -> bool

    /// Get the shared key (if handshake completed)
    pub fn get_shared_key(&self) -> Option<&str>
}

/// Factory function for server-compatible encryption
pub fn create_server_compatible_encryption() -> Box<dyn EncryptionService + Send + Sync>
```

### AesGcmEncryption

Alternative AES-GCM encryption implementation.

```rust
pub struct AesGcmEncryption {
    // Private fields
}

impl AesGcmEncryption {
    /// Create with password-derived key
    pub fn new(password: &str) -> Self

    /// Create with raw 32-byte key
    pub fn from_key(key: [u8; 32]) -> Self

    /// Generate a random 32-byte key
    pub fn generate_random_key() -> [u8; 32]

    /// Get the raw key bytes
    pub fn get_key(&self) -> &[u8; 32]
}

/// Factory functions
pub fn create_aes_gcm_encryption(password: &str) -> Box<dyn EncryptionService + Send + Sync>
pub fn create_aes_gcm_encryption_with_random_key() -> Box<dyn EncryptionService + Send + Sync>
```

#### Usage Examples

**Server-Compatible Encryption (Recommended):**
```rust
let encryption = lair_chat::client::create_server_compatible_encryption();
connection_manager.with_encryption(encryption);
```

**AES-GCM with Password:**
```rust
let encryption = lair_chat::client::create_aes_gcm_encryption("my_secure_password");
connection_manager.with_encryption(encryption);
```

**AES-GCM with Random Key:**
```rust
let encryption = lair_chat::client::create_aes_gcm_encryption_with_random_key();
connection_manager.with_encryption(encryption);
```

### EncryptionError

Errors that can occur during encryption operations.

```rust
#[derive(Debug)]
pub enum EncryptionError {
    /// Encryption operation failed
    EncryptionError(String),
    
    /// Decryption operation failed
    DecryptionError(String),
    
    /// Encoding/decoding error
    EncodingError(String),
    
    /// Key derivation error
    KeyDerivationError(String),
    
    /// Handshake failed
    HandshakeError(String),
}
```

## Authentication

### AuthManager

Handles user authentication and session management.

```rust
pub struct AuthManager {
    // Private fields
}

impl AuthManager {
    /// Create a new auth manager
    pub fn new(
        transport: Arc<Mutex<Box<dyn Transport + Send + Sync>>>,
        token_storage: Box<dyn TokenStorage + Send + Sync>
    ) -> Self

    /// Authenticate with username/password
    pub async fn login(&self, credentials: Credentials) -> Result<AuthState, AuthError>

    /// Register a new user
    pub async fn register(&self, credentials: Credentials) -> Result<AuthState, AuthError>

    /// Logout and clear session
    pub async fn logout(&self) -> Result<(), AuthError>

    /// Get current authentication state
    pub async fn get_auth_state(&self) -> AuthState

    /// Check if currently authenticated
    pub async fn is_authenticated(&self) -> bool

    /// Get current session information
    pub async fn get_session(&self) -> Option<Session>
}
```

### AuthState

Represents the current authentication state.

```rust
#[derive(Debug, Clone)]
pub enum AuthState {
    Unauthenticated,
    Authenticating,
    Authenticated {
        username: String,
        user_id: String,
        roles: Vec<String>,
        expires_at: u64,
    },
    AuthenticationFailed(String),
}
```

### Session

User session information.

```rust
#[derive(Debug, Clone)]
pub struct Session {
    pub user_id: String,
    pub username: String,
    pub token: String,
    pub roles: Vec<String>,
    pub expires_at: u64,
    pub created_at: u64,
}

impl Session {
    /// Check if the session is expired
    pub fn is_expired(&self) -> bool

    /// Get remaining time until expiration
    pub fn time_until_expiry(&self) -> Duration

    /// Refresh the session (extend expiration)
    pub fn refresh(&mut self, duration_secs: u64)
}
```

### AuthError

Authentication-related errors.

```rust
#[derive(Debug)]
pub enum AuthError {
    /// Invalid username or password
    InvalidCredentials,
    
    /// User account not found
    UserNotFound,
    
    /// Username already taken (during registration)
    UsernameTaken,
    
    /// Authentication token expired
    TokenExpired,
    
    /// Invalid authentication token
    InvalidToken,
    
    /// Session storage error
    StorageError(String),
    
    /// Network communication error
    NetworkError(TransportError),
    
    /// Internal authentication error
    InternalError(String),
}
```

## Error Handling

### Error Hierarchy

All errors implement `std::error::Error` and can be converted between types:

```rust
// Top-level application error
#[derive(Debug)]
pub enum AppError {
    Transport(TransportError),
    Auth(AuthError),
    Message(MessageError),
    Config(ConfigError),
}

// Automatic conversions
impl From<TransportError> for AppError { /* ... */ }
impl From<AuthError> for AppError { /* ... */ }
impl From<MessageError> for AppError { /* ... */ }
impl From<ConfigError> for AppError { /* ... */ }
```

### MessageError

Message-related errors.

```rust
#[derive(Debug)]
pub enum MessageError {
    /// Not connected to server
    NotConnected,
    
    /// Message encryption failed
    EncryptionFailed(EncryptionError),
    
    /// Message too large
    MessageTooLarge(usize),
    
    /// Send operation failed
    SendFailed(TransportError),
    
    /// Invalid message format
    InvalidFormat(String),
}
```

### ConfigError

Configuration-related errors.

```rust
#[derive(Debug)]
pub enum ConfigError {
    /// Invalid server address
    InvalidAddress(std::net::AddrParseError),
    
    /// Invalid timeout value
    InvalidTimeout,
    
    /// Missing required configuration field
    MissingField(String),
    
    /// Invalid configuration value
    InvalidValue(String, String),
}
```

## Observer Pattern

### ConnectionObserver Trait

Interface for receiving connection events.

```rust
pub trait ConnectionObserver: Send + Sync {
    /// Called when a message is received
    fn on_message(&self, message: String);

    /// Called when an error occurs
    fn on_error(&self, error: String);

    /// Called when connection status changes
    fn on_status_change(&self, connected: bool);
}
```

### Implementation Example

```rust
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct ChatObserver {
    action_sender: mpsc::UnboundedSender<Action>,
}

impl ChatObserver {
    pub fn new(action_sender: mpsc::UnboundedSender<Action>) -> Self {
        Self { action_sender }
    }
}

impl ConnectionObserver for ChatObserver {
    fn on_message(&self, message: String) {
        let _ = self.action_sender.send(Action::ReceiveMessage(message));
    }

    fn on_error(&self, error: String) {
        let _ = self.action_sender.send(Action::Error(error));
    }

    fn on_status_change(&self, connected: bool) {
        let status = if connected {
            ConnectionStatus::CONNECTED
        } else {
            ConnectionStatus::DISCONNECTED
        };
        let _ = self.action_sender.send(Action::ConnectionStatusChanged(status));
    }
}

// Usage
let observer = Arc::new(ChatObserver::new(action_tx));
connection_manager.register_observer(observer);
```

### MessageStore

Storage for chat messages.

```rust
pub struct MessageStore {
    pub messages: Vec<Message>,
    max_messages: usize,
}

impl MessageStore {
    pub fn new() -> Self
    pub fn with_capacity(max_messages: usize) -> Self
    pub fn add_message(&mut self, message: Message)
    pub fn get_messages(&self) -> &[Message]
    pub fn clear(&mut self)
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
}

#[derive(Debug, Clone)]
pub struct Message {
    pub content: String,
    pub timestamp: u64,
    pub sender: Option<String>,
    pub message_type: MessageType,
}

#[derive(Debug, Clone)]
pub enum MessageType {
    Chat,
    System,
    Error,
}
```

## Configuration

### Environment Variables

The library supports configuration via environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `LAIR_SERVER_ADDRESS` | Default server address | "127.0.0.1:8080" |
| `LAIR_TIMEOUT_MS` | Connection timeout in milliseconds | 5000 |
| `LAIR_LOG_LEVEL` | Logging level (trace, debug, info, warn, error) | "info" |
| `LAIR_ENCRYPTION_KEY` | Default encryption key | None |

### Configuration Builder

```rust
use lair_chat::client::ConfigBuilder;

let config = ConfigBuilder::new()
    .address("127.0.0.1:8080".parse()?)
    .timeout_ms(10000)
    .enable_encryption(true)
    .build()?;

let connection_manager = ConnectionManager::new(config);
```

## Examples

### Complete Chat Client

```rust
use lair_chat::client::{ConnectionManager, Credentials, TcpTransport};
use lair_chat::transport::{ConnectionConfig, ConnectionObserver, ConnectionStatus};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Action {
    ReceiveMessage(String),
    Error(String),
    ConnectionStatusChanged(ConnectionStatus),
}

pub struct ChatClient {
    connection_manager: ConnectionManager,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

impl ChatClient {
    pub async fn new(server_address: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config = ConnectionConfig::new(server_address.parse()?);
        let mut connection_manager = ConnectionManager::new(config.clone());

        // Configure transport
        let transport = Box::new(TcpTransport::new(config));
        connection_manager.with_transport(transport);

        // Configure encryption
        let encryption = lair_chat::client::create_server_compatible_encryption();
        connection_manager.with_encryption(encryption);

        // Set up observer
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let observer = Arc::new(ChatObserver::new(action_tx));
        connection_manager.register_observer(observer);

        Ok(Self {
            connection_manager,
            action_rx,
        })
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connection_manager.connect().await?;
        Ok(())
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let credentials = Credentials {
            username: username.to_string(),
            password: password.to_string(),
        };
        self.connection_manager.login(credentials).await?;
        Ok(())
    }

    pub async fn send_message(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.connection_manager.send_message(message).await?;
        Ok(())
    }

    pub async fn receive_action(&mut self) -> Option<Action> {
        self.action_rx.recv().await
    }

    pub async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connection_manager.disconnect().await?;
        Ok(())
    }
}

// Observer implementation
struct ChatObserver {
    action_sender: mpsc::UnboundedSender<Action>,
}

impl ChatObserver {
    fn new(action_sender: mpsc::UnboundedSender<Action>) -> Self {
        Self { action_sender }
    }
}

impl ConnectionObserver for ChatObserver {
    fn on_message(&self, message: String) {
        let _ = self.action_sender.send(Action::ReceiveMessage(message));
    }

    fn on_error(&self, error: String) {
        let _ = self.action_sender.send(Action::Error(error));
    }

    fn on_status_change(&self, connected: bool) {
        let status = if connected {
            ConnectionStatus::CONNECTED
        } else {
            ConnectionStatus::DISCONNECTED
        };
        let _ = self.action_sender.send(Action::ConnectionStatusChanged(status));
    }
}

// Usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ChatClient::new("127.0.0.1:8080").await?;
    
    client.connect().await?;
    client.login("alice", "password123").await?;
    
    // Send a message
    client.send_message("Hello, World!").await?;
    
    // Handle incoming events
    while let Some(action) = client.receive_action().await {
        match action {
            Action::ReceiveMessage(msg) => println!("Received: {}", msg),
            Action::Error(err) => eprintln!("Error: {}", err),
            Action::ConnectionStatusChanged(status) => println!("Status: {:?}", status),
        }
    }
    
    client.disconnect().await?;
    Ok(())
}
```

### Custom Transport Implementation

```rust
use lair_chat::client::transport::{Transport, TransportError};
use async_trait::async_trait;

pub struct MockTransport {
    connected: bool,
    messages: Vec<String>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            connected: false,
            messages: Vec::new(),
        }
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn connect(&mut self) -> Result<(), TransportError> {
        self.connected = true;
        Ok(())
    }

    async fn send(&mut self, data: &str) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected")
            ));
        }
        self.messages.push(data.to_string());
        Ok(())
    }

    async fn receive(&mut self) -> Result<Option<String>, TransportError> {
        if !self.connected {
            return Err(TransportError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected")
            ));
        }
        Ok(self.messages.pop())
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.connected = false;
        Ok(())
    }
}

// Usage
let transport = Box::new(MockTransport::new());
connection_manager.with_transport(transport);
```

## Migration Guide

### From v0.5.x to v0.6.0

**Breaking Changes:**

1. **Global State Removed**
   ```rust
   // OLD (v0.5.x)
   let status = CLIENT_STATUS.lock().unwrap();
   add_text_message("Hello");
   
   // NEW (v0.6.0)
   let status = connection_manager.get_status().await;
   connection_manager.send_message("Hello").await?;
   ```

2. **Error Types Changed**
   ```rust
   // OLD (v0.5.x)
   fn connect() -> Result<(), String>
   
   // NEW (v0.6.0)
   async fn connect(&mut self) -> Result<(), TransportError>
   ```

3. **Configuration Structure**
   ```rust
   // OLD (v0.5.x)
   connect_client("127.0.0.1:8080").await?;
   
   // NEW (v0.6.0)
   let config = ConnectionConfig::new("127.0.0.1:8080".parse()?);
   let mut manager = ConnectionManager::new(config);
   manager.connect().await?;
   ```

**Migration Steps:**

1. **Replace global state access** with ConnectionManager methods
2. **Update error handling** to use typed errors  
3. **Add async/await** to all I/O operations
4. **Use observer pattern** instead of direct UI calls
5. **Configure dependencies** explicitly via injection

### Common Migration Patterns

**Authentication:**
```rust
// OLD
authenticate_compat(username, password).await?;

// NEW
let credentials = Credentials { username, password };
connection_manager.login(credentials).await?;
```

**Message Sending:**
```rust
// OLD
add_outgoing_message(msg);

// NEW  
connection_manager.send_message(&msg).await?;
```

**Status Checking:**
```rust
// OLD
let status = CLIENT_STATUS.lock().unwrap().status;

// NEW
let status = connection_manager.get_status().await;
```

## Best Practices

### Error Handling

```rust
use lair_chat::client::{TransportError, AuthError, MessageError};

// Use typed errors and the ? operator
async fn handle_chat_operation() -> Result<(), ChatError> {
    connection_manager.connect().await?;  // TransportError -> ChatError
    connection_manager.login(credentials).await?;  // AuthError -> ChatError
    connection_manager.send_message("Hello").await?;  // MessageError -> ChatError
    Ok(())
}
```

### Resource Management

```rust
// Proper cleanup in Drop implementation
impl Drop for ChatClient {
    fn drop(&mut self) {
        // Clean up resources
        tokio::spawn(async move {
            let _ = self.connection_manager.disconnect().await;
        });
    }
}
```

### Concurrent Operations

```rust
// Use Arc for sharing ConnectionManager across tasks
let manager = Arc::new(Mutex::new(connection_manager));

// Clone for background task
let manager_clone = manager.clone();
tokio::spawn(async move {
    let mut manager = manager_clone.lock().await;
    manager.send_message("Background message").await;
});
```

## Troubleshooting

### Common Issues

**Connection Timeout:**
```rust
// Increase timeout
let config = ConnectionConfig::new(addr).with_timeout(10000);
```

**Authentication Failure:**
```rust
// Check credentials and server status
match connection_manager.login(credentials).await {
    Err(AuthError::InvalidCredentials) => {
        println!("Check username and password");
    }
    Err(AuthError::NetworkError(e)) => {
        println!("Network issue: {}", e);
    }
    _ => {}
}
```

**Observer Not Receiving Events:**
```rust
// Ensure observer is registered before connecting
connection_manager.register_observer(observer);
connection_manager.connect().await?;
```

---

**API Stability**: This API is considered stable as of v0.6.0. Breaking changes will only be introduced in major version releases.

**Support**: For questions and issues, refer to the project's issue tracker and documentation.