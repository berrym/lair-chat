# Migration Guide: Upgrading to Lair Chat v0.6.0

**Version**: 0.6.0  
**Last Updated**: December 12, 2025  
**Migration Difficulty**: Moderate to High

## Overview

Lair Chat v0.6.0 introduces a complete architectural modernization that removes all legacy code and implements modern async/await patterns. This guide helps you migrate from v0.5.x to v0.6.0.

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Breaking Changes](#breaking-changes)
3. [Step-by-Step Migration](#step-by-step-migration)
4. [Code Examples](#code-examples)
5. [Common Migration Patterns](#common-migration-patterns)
6. [Error Handling Migration](#error-handling-migration)
7. [Testing Your Migration](#testing-your-migration)
8. [Troubleshooting](#troubleshooting)
9. [Performance Considerations](#performance-considerations)
10. [Support](#support)

## Executive Summary

### What's Changed

- **🔄 Global State Removed**: No more `CLIENT_STATUS`, `MESSAGES`, `ACTION_SENDER`
- **🏗️ Modern Architecture**: New `ConnectionManager` with dependency injection
- **⚡ Async/Await**: All I/O operations are now properly async
- **🛡️ Type Safety**: Comprehensive error types replace string errors
- **👁️ Observer Pattern**: Event-driven communication replaces direct calls

### Migration Impact

- **High Impact**: Applications using global state directly
- **Medium Impact**: Applications using compatibility layers
- **Low Impact**: Applications already using modern patterns

### Timeline Estimate

- **Small Projects** (< 1000 LOC): 1-2 days
- **Medium Projects** (1000-5000 LOC): 3-5 days  
- **Large Projects** (> 5000 LOC): 1-2 weeks

## Breaking Changes

### 1. Global State Removal

**What Changed:**
All global state variables have been removed from the public API.

**v0.5.x Code:**
```rust
use lair_chat::transport::{CLIENT_STATUS, MESSAGES, add_text_message};

let status = CLIENT_STATUS.lock().unwrap();
add_text_message("Hello, World!".to_string());
```

**v0.6.0 Replacement:**
```rust
use lair_chat::client::{ConnectionManager, TcpTransport};
use lair_chat::transport::ConnectionConfig;

let config = ConnectionConfig::new("127.0.0.1:8080".parse()?);
let mut connection_manager = ConnectionManager::new(config);
let status = connection_manager.get_status().await;
connection_manager.send_message("Hello, World!").await?;
```

### 2. Function Signatures

**What Changed:**
Most functions are now async and return typed errors.

**v0.5.x Code:**
```rust
fn connect_client(addr: &str) -> Result<(), String>
fn send_message(msg: String) -> Result<(), String>
```

**v0.6.0 Replacement:**
```rust
async fn connect(&mut self) -> Result<(), TransportError>
async fn send_message(&mut self, msg: &str) -> Result<(), MessageError>
```

### 3. Authentication API

**What Changed:**
Authentication now uses structured credentials and typed responses.

**v0.5.x Code:**
```rust
authenticate_compat(username.clone(), password.clone()).await?;
```

**v0.6.0 Replacement:**
```rust
let credentials = Credentials {
    username: username.clone(),
    password: password.clone(),
};
connection_manager.login(credentials).await?;
```

### 4. Configuration Structure

**What Changed:**
Configuration is now structured and type-safe.

**v0.5.x Code:**
```rust
connect_client("127.0.0.1:8080").await?;
```

**v0.6.0 Replacement:**
```rust
let config = ConnectionConfig {
    address: "127.0.0.1:8080".parse()?,
    timeout_ms: 5000,
};
let connection_manager = ConnectionManager::new(config);
```

### 5. Error Types

**What Changed:**
String-based errors replaced with comprehensive typed errors.

**v0.5.x Error:**
```rust
Err("Connection failed".to_string())
```

**v0.6.0 Error:**
```rust
Err(TransportError::ConnectionError(io::Error::new(
    io::ErrorKind::ConnectionRefused,
    "Connection refused"
)))
```

## Step-by-Step Migration

### Phase 1: Preparation (Day 1)

#### 1.1 Update Dependencies

**Cargo.toml:**
```toml
[dependencies]
lair-chat = "0.6.0"
tokio = { version = "1.0", features = ["full"] }
```

#### 1.2 Audit Existing Code

**Find Legacy Usage:**
```bash
# Search for global state usage
grep -r "CLIENT_STATUS\|MESSAGES\|add_text_message" src/

# Search for compatibility layer usage  
grep -r "connect_client_compat\|authenticate_compat" src/

# Search for string-based error handling
grep -r "Result<.*String>" src/
```

#### 1.3 Plan Migration Strategy

**Create Migration Checklist:**
- [ ] Identify all global state access points
- [ ] List functions that need async conversion
- [ ] Plan observer implementation
- [ ] Design error handling strategy

### Phase 2: Core Migration (Days 2-3)

#### 2.1 Replace Global State

**Before:**
```rust
use lair_chat::transport::{CLIENT_STATUS, ConnectionStatus};

fn check_connection() -> bool {
    let status = CLIENT_STATUS.lock().unwrap();
    status.status == ConnectionStatus::CONNECTED
}
```

**After:**
```rust
use lair_chat::client::ConnectionManager;
use lair_chat::transport::{ConnectionConfig, ConnectionStatus};

struct MyApp {
    connection_manager: ConnectionManager,
}

impl MyApp {
    async fn check_connection(&self) -> bool {
        self.connection_manager.get_status().await == ConnectionStatus::CONNECTED
    }
}
```

#### 2.2 Implement ConnectionManager

**Basic Setup:**
```rust
use lair_chat::client::{ConnectionManager, TcpTransport};
use lair_chat::transport::ConnectionConfig;

pub struct ChatApplication {
    connection_manager: ConnectionManager,
}

impl ChatApplication {
    pub fn new(server_address: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Create configuration
        let config = ConnectionConfig {
            address: server_address.parse()?,
            timeout_ms: 5000,
        };

        // Create connection manager
        let mut connection_manager = ConnectionManager::new(config.clone());

        // Configure transport
        let transport = Box::new(TcpTransport::new(config));
        connection_manager.with_transport(transport);

        // Configure encryption
        let encryption = lair_chat::client::create_server_compatible_encryption();
        connection_manager.with_encryption(encryption);

        Ok(Self { connection_manager })
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connection_manager.connect().await?;
        Ok(())
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let credentials = lair_chat::client::Credentials {
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
}
```

#### 2.3 Convert to Async

**Before:**
```rust
fn handle_user_input(input: &str) -> Result<(), String> {
    match input {
        "connect" => connect_to_server()?,
        "login" => authenticate_user()?,
        msg => send_message(msg)?,
    }
    Ok(())
}
```

**After:**
```rust
async fn handle_user_input(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
    match input {
        "connect" => self.connect().await?,
        "login" => self.authenticate_user().await?,
        msg => self.send_message(msg).await?,
    }
    Ok(())
}
```

### Phase 3: Observer Implementation (Day 3-4)

#### 3.1 Create Observer

**Define Observer:**
```rust
use lair_chat::transport::{ConnectionObserver, ConnectionStatus};
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum ChatEvent {
    MessageReceived(String),
    ConnectionStatusChanged(ConnectionStatus),
    Error(String),
}

pub struct ChatObserver {
    event_sender: mpsc::UnboundedSender<ChatEvent>,
}

impl ChatObserver {
    pub fn new(event_sender: mpsc::UnboundedSender<ChatEvent>) -> Self {
        Self { event_sender }
    }
}

impl ConnectionObserver for ChatObserver {
    fn on_message(&self, message: String) {
        let _ = self.event_sender.send(ChatEvent::MessageReceived(message));
    }

    fn on_error(&self, error: String) {
        let _ = self.event_sender.send(ChatEvent::Error(error));
    }

    fn on_status_change(&self, connected: bool) {
        let status = if connected {
            ConnectionStatus::CONNECTED
        } else {
            ConnectionStatus::DISCONNECTED
        };
        let _ = self.event_sender.send(ChatEvent::ConnectionStatusChanged(status));
    }
}
```

#### 3.2 Integrate Observer

**Registration:**
```rust
impl ChatApplication {
    pub fn new(server_address: &str) -> Result<(Self, mpsc::UnboundedReceiver<ChatEvent>), Box<dyn std::error::Error>> {
        // ... previous setup code ...

        // Create event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Create and register observer
        let observer = std::sync::Arc::new(ChatObserver::new(event_tx));
        connection_manager.register_observer(observer);

        Ok((Self { connection_manager }, event_rx))
    }

    pub async fn run_event_loop(&mut self, mut event_rx: mpsc::UnboundedReceiver<ChatEvent>) {
        while let Some(event) = event_rx.recv().await {
            match event {
                ChatEvent::MessageReceived(msg) => {
                    println!("Received: {}", msg);
                }
                ChatEvent::ConnectionStatusChanged(status) => {
                    println!("Connection status: {:?}", status);
                }
                ChatEvent::Error(err) => {
                    eprintln!("Error: {}", err);
                }
            }
        }
    }
}
```

### Phase 4: Error Handling (Day 4-5)

#### 4.1 Define Application Errors

**Create Error Types:**
```rust
use lair_chat::client::{AuthError, MessageError, TransportError};

#[derive(Debug)]
pub enum ChatAppError {
    Transport(TransportError),
    Authentication(AuthError),
    Message(MessageError),
    Configuration(String),
    Internal(String),
}

impl std::fmt::Display for ChatAppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatAppError::Transport(e) => write!(f, "Transport error: {}", e),
            ChatAppError::Authentication(e) => write!(f, "Authentication error: {}", e),
            ChatAppError::Message(e) => write!(f, "Message error: {}", e),
            ChatAppError::Configuration(e) => write!(f, "Configuration error: {}", e),
            ChatAppError::Internal(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl std::error::Error for ChatAppError {}

// Automatic conversions
impl From<TransportError> for ChatAppError {
    fn from(error: TransportError) -> Self {
        ChatAppError::Transport(error)
    }
}

impl From<AuthError> for ChatAppError {
    fn from(error: AuthError) -> Self {
        ChatAppError::Authentication(error)
    }
}

impl From<MessageError> for ChatAppError {
    fn from(error: MessageError) -> Self {
        ChatAppError::Message(error)
    }
}
```

#### 4.2 Update Function Signatures

**Before:**
```rust
fn connect_and_login(addr: &str, user: &str, pass: &str) -> Result<(), String> {
    // ...
}
```

**After:**
```rust
async fn connect_and_login(&mut self, user: &str, pass: &str) -> Result<(), ChatAppError> {
    self.connection_manager.connect().await?;  // TransportError -> ChatAppError
    
    let credentials = lair_chat::client::Credentials {
        username: user.to_string(),
        password: pass.to_string(),
    };
    self.connection_manager.login(credentials).await?;  // AuthError -> ChatAppError
    
    Ok(())
}
```

## Code Examples

### Complete Migration Example

**v0.5.x Application:**
```rust
// OLD: Global state-based application
use lair_chat::transport::{CLIENT_STATUS, add_text_message, connect_client_compat};

pub struct OldChatApp {
    username: String,
}

impl OldChatApp {
    pub fn new(username: String) -> Self {
        Self { username }
    }

    pub async fn start(&self, server_addr: &str) -> Result<(), String> {
        // Connect using compatibility layer
        let addr = server_addr.parse().map_err(|e| format!("Invalid address: {}", e))?;
        connect_client_compat(tui_input::Input::default(), addr).await
            .map_err(|e| format!("Connection failed: {}", e))?;

        // Check status via global state
        let status = CLIENT_STATUS.lock().unwrap();
        if status.status == lair_chat::transport::ConnectionStatus::CONNECTED {
            add_text_message("Connected successfully!".to_string());
        }

        Ok(())
    }

    pub fn send_message(&self, message: &str) -> Result<(), String> {
        add_text_message(format!("{}: {}", self.username, message));
        Ok(())
    }
}
```

**v0.6.0 Application:**
```rust
// NEW: Modern architecture-based application
use lair_chat::client::{ConnectionManager, TcpTransport, Credentials};
use lair_chat::transport::{ConnectionConfig, ConnectionObserver, ConnectionStatus};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum AppEvent {
    MessageReceived(String),
    StatusChanged(ConnectionStatus),
    Error(String),
}

pub struct ModernChatApp {
    connection_manager: ConnectionManager,
    username: String,
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl ModernChatApp {
    pub fn new(username: String, server_addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Create configuration
        let config = ConnectionConfig {
            address: server_addr.parse()?,
            timeout_ms: 5000,
        };

        // Create connection manager
        let mut connection_manager = ConnectionManager::new(config.clone());

        // Configure transport
        let transport = Box::new(TcpTransport::new(config));
        connection_manager.with_transport(transport);

        // Configure encryption
        let encryption = lair_chat::client::create_server_compatible_encryption();
        connection_manager.with_encryption(encryption);

        // Set up event handling
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let observer = Arc::new(AppObserver::new(event_tx));
        connection_manager.register_observer(observer);

        Ok(Self {
            connection_manager,
            username,
            event_rx,
        })
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Connect using modern API
        self.connection_manager.connect().await?;

        // Check status via API
        let status = self.connection_manager.get_status().await;
        if status == ConnectionStatus::CONNECTED {
            println!("Connected successfully!");
        }

        Ok(())
    }

    pub async fn login(&mut self, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let credentials = Credentials {
            username: self.username.clone(),
            password: password.to_string(),
        };
        self.connection_manager.login(credentials).await?;
        Ok(())
    }

    pub async fn send_message(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let formatted_message = format!("{}: {}", self.username, message);
        self.connection_manager.send_message(&formatted_message).await?;
        Ok(())
    }

    pub async fn handle_events(&mut self) {
        while let Some(event) = self.event_rx.recv().await {
            match event {
                AppEvent::MessageReceived(msg) => println!("Received: {}", msg),
                AppEvent::StatusChanged(status) => println!("Status: {:?}", status),
                AppEvent::Error(err) => eprintln!("Error: {}", err),
            }
        }
    }
}

// Observer implementation
struct AppObserver {
    event_sender: mpsc::UnboundedSender<AppEvent>,
}

impl AppObserver {
    fn new(event_sender: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self { event_sender }
    }
}

impl ConnectionObserver for AppObserver {
    fn on_message(&self, message: String) {
        let _ = self.event_sender.send(AppEvent::MessageReceived(message));
    }

    fn on_error(&self, error: String) {
        let _ = self.event_sender.send(AppEvent::Error(error));
    }

    fn on_status_change(&self, connected: bool) {
        let status = if connected {
            ConnectionStatus::CONNECTED
        } else {
            ConnectionStatus::DISCONNECTED
        };
        let _ = self.event_sender.send(AppEvent::StatusChanged(status));
    }
}

// Usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = ModernChatApp::new("alice".to_string(), "127.0.0.1:8080")?;
    
    app.start().await?;
    app.login("password123").await?;
    app.send_message("Hello, World!").await?;
    
    // Handle events in background
    tokio::spawn(async move {
        app.handle_events().await;
    });

    Ok(())
}
```

## Common Migration Patterns

### Pattern 1: Connection Management

**Before:**
```rust
// Global state access
if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::CONNECTED {
    // Do something
}
```

**After:**
```rust
// Instance method
if self.connection_manager.get_status().await == ConnectionStatus::CONNECTED {
    // Do something
}
```

### Pattern 2: Message Handling

**Before:**
```rust
add_text_message(format!("User {} says: {}", username, message));
```

**After:**
```rust
// Through observer pattern
impl ConnectionObserver for MyObserver {
    fn on_message(&self, message: String) {
        println!("User says: {}", message);
    }
}
```

### Pattern 3: Error Propagation

**Before:**
```rust
fn do_something() -> Result<(), String> {
    connect_client("127.0.0.1:8080")
        .map_err(|e| format!("Connection failed: {}", e))?;
    Ok(())
}
```

**After:**
```rust
async fn do_something(&mut self) -> Result<(), MyAppError> {
    self.connection_manager.connect().await?;  // Auto-converts via From trait
    Ok(())
}
```

### Pattern 4: Async Conversion

**Before:**
```rust
fn handle_input(input: &str) -> Result<(), String> {
    match input {
        "quit" => std::process::exit(0),
        msg => send_message(msg)?,
    }
    Ok(())
}
```

**After:**
```rust
async fn handle_input(&mut self, input: &str) -> Result<(), MyAppError> {
    match input {
        "quit" => std::process::exit(0),
        msg => self.connection_manager.send_message(msg).await?,
    }
    Ok(())
}
```

### Pattern 5: Authentication Flow

**Before:**
```rust
authenticate_compat(username.clone(), password.clone()).await
    .map_err(|e| format!("Auth failed: {}", e))?;
```

**After:**
```rust
let credentials = Credentials {
    username: username.clone(),
    password: password.clone(),
};
self.connection_manager.login(credentials).await?;
```

## Error Handling Migration

### Error Type Mapping

| v0.5.x | v0.6.0 | Migration Notes |
|--------|--------|-----------------|
| `Result<T, String>` | `Result<T, TransportError>` | Replace string errors with typed errors |
| `"Connection failed"` | `TransportError::ConnectionError(io::Error)` | Use specific error variants |
| `"Auth failed"` | `AuthError::InvalidCredentials` | Auth errors are now structured |
| `"Invalid input"` | `ConfigError::InvalidValue(field, value)` | Configuration errors are detailed |

### Error Handling Best Practices

**Use Error Hierarchies:**
```rust
#[derive(Debug)]
pub enum MyAppError {
    Transport(TransportError),
    Auth(AuthError),
    Message(MessageError),
    Custom(String),
}

// Implement From traits for automatic conversion
impl From<TransportError> for MyAppError {
    fn from(error: TransportError) -> Self {
        MyAppError::Transport(error)
    }
}
```

**Handle Specific Error Cases:**
```rust
match self.connection_manager.login(credentials).await {
    Ok(()) => println!("Login successful"),
    Err(AuthError::InvalidCredentials) => {
        println!("Please check your username and password");
    }
    Err(AuthError::NetworkError(transport_err)) => {
        println!("Network issue: {}", transport_err);
        // Maybe retry connection
    }
    Err(e) => println!("Login failed: {}", e),
}
```

## Testing Your Migration

### Unit Tests

**Test Connection Manager:**
```rust
#[tokio::test]
async fn test_connection_manager() {
    let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
    let connection_manager = ConnectionManager::new(config);
    
    assert_eq!(connection_manager.get_status().await, ConnectionStatus::DISCONNECTED);
}
```

**Test Observer Pattern:**
```rust
#[tokio::test]
async fn test_observer_notifications() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let observer = Arc::new(TestObserver::new(tx));
    
    observer.on_message("test message".to_string());
    
    let received = rx.recv().await.unwrap();
    assert_eq!(received, "test message");
}
```

### Integration Tests

**Test Full Workflow:**
```rust
#[tokio::test]
async fn test_full_chat_workflow() {
    let mut app = ModernChatApp::new("test_user".to_string(), "127.0.0.1:8080").unwrap();
    
    // Test connection
    app.start().await.unwrap();
    
    // Test authentication
    app.login("test_password").await.unwrap();
    
    // Test message sending
    app.send_message("Hello, test!").await.unwrap();
}
```

### Manual Testing

**Verification Checklist:**
- [ ] Connection establishment works
- [ ] Authentication succeeds with valid credentials
- [ ] Authentication fails with invalid credentials
- [ ] Messages can be sent and received
- [ ] Observer notifications are triggered
- [ ] Error handling works as expected
- [ ] Cleanup/disconnect works properly

## Troubleshooting

### Common Migration Issues

#### Issue 1: Async Runtime Not Available

**Error:**
```
thread 'main' panicked at 'there is no reactor running'
```

**Solution:**
```rust
// Ensure you have a Tokio runtime
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Your async code here
    Ok(())
}

// Or create runtime manually
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Your async code here
    })
}
```

#### Issue 2: Borrow Checker Issues with Arc<Mutex<>>

**Error:**
```
cannot borrow `*connection_manager` as mutable because it is behind a shared reference
```

**Solution:**
```rust
// Use proper async locking
let connection_manager = Arc::new(tokio::sync::Mutex::new(connection_manager));

// In async context
let mut manager = connection_manager.lock().await;
manager.send_message("Hello").await?;
```

#### Issue 3: Observer Not Receiving Events

**Problem:** Observer is registered but events are not received.

**Solution:**
```rust
// Ensure observer is registered BEFORE connecting
connection_manager.register_observer(observer);
connection_manager.connect().await?;

// Ensure event loop is running
tokio::spawn(async move {
    while let Some(event) = event_rx.recv().await {
        // Handle event
    }
});
```

#### Issue 4: Type Conversion Errors

**Error:**
```
the trait `From<String>` is not implemented for `TransportError`
```

**Solution:**
```rust
// Implement proper error conversion
impl From<String> for MyAppError {
    fn from(error: String) -> Self {
        MyAppError::Custom(error)
    }
}

// Or handle explicitly
let result = some_function().map_err(|e| MyAppError::Custom(e.to_string()))?;
```

## Performance Considerations

### Memory Usage

**v0.6.0 Improvements:**
- Removed global state eliminates memory leaks
- Arc<Mutex<>> usage is more efficient than global locks
- Observer pattern reduces memory overhead

**Best Practices:**
```rust
// Limit message store size to prevent memory growth
let mut connection_manager = ConnectionManager::new(config);
// connection_manager.set_max_messages(1000);  // If available

// Use weak references where appropriate
use std::sync::Weak;
let weak_observer: Weak<dyn ConnectionObserver> = Arc::downgrade(&observer);
```

### CPU Usage

**v0.6.0 Improvements:**
- Async I/O reduces CPU blocking
- Proper connection pooling
- Efficient message routing

**Optimization Tips:**
```rust
// Use buffered I/O for high-throughput scenarios
let config = ConnectionConfig::new(addr).with_buffer_size(8192);

// Batch messages when possible
let messages = vec!["msg1", "msg2", "msg3"];
for msg in messages {
    connection_manager.send_message(msg).await?;
}
```

## Support

### Migration Assistance

**Documentation Resources:**
- [API Documentation](./API_DOCUMENTATION.md)
- [Architecture Guide](./TRANSPORT_ARCHITECTURE.md)
- [Examples](./examples/)

**Common Questions:**

**Q: Can I gradually migrate parts of my application?**
A: Yes, but v0.6.0 removes all compatibility layers. You'll need to migrate the entire connection handling at once.

**Q: How do I handle existing stored data?**
A: Message formats are compatible. User data and configuration may need migration depending on your storage format.

**Q: What about custom transport implementations?**
A: Implement the new `Transport` trait. The interface is cleaner and more consistent.

**Q: Are there performance regressions?**
A: No, v0.6.0 is significantly faster due to proper async I/O and removal of global state contention.

### Getting Help

**If you encounter issues:**

1. **Check the examples** in the `examples/` directory
2. **Review error messages** - v0.6.0 has much better error reporting
3. **Consult the API documentation** for correct usage patterns
4. **Test with a minimal example** to isolate issues

**For complex migrations:**
- Start with a small proof-of-concept
- Migrate one component at a time
- Keep v0.5.x version available for reference
- Test thoroughly before deploying

---

**Migration Checklist:**
- [ ] Updated dependencies to v0.6.0
- [ ] Replaced all global state access
- [ ] Converted functions to async
- [ ] Implemented observer pattern
- [ ] Updated error handling
- [ ] Added proper type conversions
- [ ] Updated tests
- [ ] Verified performance
- [ ] Tested full application workflow

**This migration guide is comprehensive but every application is different. Take time to understand the new patterns and don't hesitate to start with small changes before tackling the entire migration.**