# Transport Architecture Usage Examples

This document provides practical examples for using the lair-chat transport architecture in various scenarios.

## Table of Contents

1. [Basic Usage Examples](#basic-usage-examples)
2. [Migration Examples](#migration-examples)
3. [Advanced Configuration](#advanced-configuration)
4. [Testing Examples](#testing-examples)
5. [Error Handling Examples](#error-handling-examples)
6. [Custom Implementation Examples](#custom-implementation-examples)

## Basic Usage Examples

### Simple Connection with Migration Facade

```rust
use crate::client::migration_facade;
use tui_input::Input;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize input handler
    let input = Input::default();
    
    // Parse server address
    let address: SocketAddr = "127.0.0.1:8080".parse()?;
    
    // Connect using migration facade (recommended for existing code)
    migration_facade::connect_client(input, address).await?;
    
    // Send a message
    migration_facade::send_message("Hello, server!".to_string()).await?;
    
    // Disconnect gracefully
    migration_facade::disconnect_client().await?;
    
    Ok(())
}
```

### Direct ConnectionManager Usage

```rust
use crate::client::{
    connection_manager::{ConnectionManager, ConnectionConfig},
    tcp_transport::TcpTransport,
    aes_gcm_encryption::AesGcmEncryption,
    transport::TuiObserver,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration
    let address: SocketAddr = "127.0.0.1:8080".parse()?;
    let config = ConnectionConfig::new(address);
    
    // Create and configure connection manager
    let mut manager = ConnectionManager::new(config.clone());
    manager.set_transport(Box::new(TcpTransport::new(config.clone())));
    manager.set_encryption(Box::new(AesGcmEncryption::new("my_secure_password")));
    manager.register_observer(Box::new(TuiObserver::new()));
    
    // Establish connection
    manager.connect().await?;
    println!("Connected successfully!");
    
    // Send multiple messages
    let messages = vec![
        "Hello from client!",
        "How are you doing?",
        "This is a test message",
    ];
    
    for message in messages {
        manager.send_message(message.to_string()).await?;
        println!("Sent: {}", message);
    }
    
    // Check connection status
    let status = manager.get_status().await;
    println!("Connection status: {:?}", status);
    
    // Disconnect
    manager.disconnect().await?;
    println!("Disconnected successfully!");
    
    Ok(())
}
```

## Migration Examples

### Migrating from Legacy Transport

**Before (Legacy Code):**
```rust
use crate::transport::{connect_client, send_message, disconnect_client};

async fn legacy_client_example() {
    let input = tui_input::Input::default();
    let addr = "127.0.0.1:8080".parse().unwrap();
    
    // Legacy API calls
    connect_client(input, addr).await;
    // Note: Legacy API doesn't return Result, making error handling difficult
    
    send_message("Hello".to_string()).await;
    disconnect_client().await;
}
```

**After (Using Migration Facade):**
```rust
use crate::client::migration_facade;

async fn migrated_client_example() -> Result<(), crate::client::transport::TransportError> {
    let input = tui_input::Input::default();
    let addr = "127.0.0.1:8080".parse().unwrap();
    
    // Migration facade with proper error handling
    migration_facade::connect_client(input, addr).await?;
    migration_facade::send_message("Hello".to_string()).await?;
    migration_facade::disconnect_client().await?;
    
    Ok(())
}
```

### Gradual Migration Strategy

```rust
use crate::client::{migration_facade, transport::TransportError};

// Step 1: Wrap legacy calls with error handling
async fn step1_add_error_handling() -> Result<(), TransportError> {
    let input = tui_input::Input::default();
    let addr = "127.0.0.1:8080".parse().unwrap();
    
    // Still using legacy, but with error wrapper
    migration_facade::connect_client(input, addr).await?;
    migration_facade::send_message("Hello".to_string()).await?;
    migration_facade::disconnect_client().await?;
    
    Ok(())
}

// Step 2: Move to direct ConnectionManager usage
async fn step2_direct_manager() -> Result<(), TransportError> {
    use crate::client::connection_manager::{ConnectionManager, ConnectionConfig};
    
    let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
    let mut manager = ConnectionManager::new(config);
    
    manager.connect().await?;
    manager.send_message("Hello".to_string()).await?;
    manager.disconnect().await?;
    
    Ok(())
}
```

## Advanced Configuration

### Custom Connection Configuration

```rust
use crate::client::{
    connection_manager::{ConnectionManager, ConnectionConfig},
    tcp_transport::TcpTransport,
    aes_gcm_encryption::AesGcmEncryption,
    transport::{TuiObserver, ConnectionObserver},
};
use std::sync::Arc;
use tokio::sync::Mutex;

// Custom observer for logging
struct LoggingObserver {
    log_file: Arc<Mutex<std::fs::File>>,
}

impl LoggingObserver {
    fn new(log_path: &str) -> Result<Self, std::io::Error> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        
        Ok(Self {
            log_file: Arc::new(Mutex::new(file)),
        })
    }
}

impl ConnectionObserver for LoggingObserver {
    fn on_status_change(&self, old_status: crate::client::transport::ConnectionStatus, new_status: crate::client::transport::ConnectionStatus) {
        println!("Status changed: {:?} -> {:?}", old_status, new_status);
    }
    
    fn on_message_received(&self, message: &str) {
        println!("Received: {}", message);
    }
    
    fn on_message_sent(&self, message: &str) {
        println!("Sent: {}", message);
    }
    
    fn on_error(&self, error: &str) {
        eprintln!("Error: {}", error);
    }
}

async fn advanced_configuration_example() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration with custom timeout
    let config = ConnectionConfig::new("127.0.0.1:8080".parse()?)
        .with_timeout(10000); // 10 seconds timeout
    
    // Create manager with custom components
    let mut manager = ConnectionManager::new(config.clone());
    
    // Use TCP transport
    manager.set_transport(Box::new(TcpTransport::new(config.clone())));
    
    // Use AES-GCM encryption with custom password
    manager.set_encryption(Box::new(AesGcmEncryption::new("super_secure_password_123")));
    
    // Register multiple observers
    manager.register_observer(Box::new(TuiObserver::new()));
    manager.register_observer(Box::new(LoggingObserver::new("connection.log")?));
    
    // Connect and use
    manager.connect().await?;
    manager.send_message("Hello from advanced client!".to_string()).await?;
    manager.disconnect().await?;
    
    Ok(())
}
```

### Environment-Based Configuration

```rust
use crate::client::migration_facade::{MigrationConfig, init_migration};
use std::env;

fn setup_environment_based_config() {
    // Read configuration from environment variables
    let use_new_arch = env::var("LAIR_USE_NEW_TRANSPORT")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);
    
    let verbose_logging = env::var("LAIR_VERBOSE_LOGGING")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
    
    let env_var_name = env::var("LAIR_MIGRATION_FLAG")
        .unwrap_or_else(|_| "LAIR_USE_NEW_TRANSPORT".to_string());
    
    // Configure migration system
    let config = MigrationConfig {
        use_new_architecture: use_new_arch,
        auto_detect: true,
        env_var_name,
        verbose_logging,
    };
    
    init_migration(config);
    
    if verbose_logging {
        println!("Migration configured with new architecture: {}", use_new_arch);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup configuration
    setup_environment_based_config();
    
    // Use the configured system
    let input = tui_input::Input::default();
    let address = "127.0.0.1:8080".parse()?;
    
    migration_facade::connect_client(input, address).await?;
    migration_facade::send_message("Environment configured!".to_string()).await?;
    migration_facade::disconnect_client().await?;
    
    Ok(())
}
```

## Testing Examples

### Unit Testing with Mocks

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{
        connection_manager::{ConnectionManager, ConnectionConfig},
        transport::{Transport, TransportError, ConnectionStatus},
    };
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}, Mutex};
    use async_trait::async_trait;
    
    // Mock transport for testing
    struct MockTransport {
        connected: Arc<AtomicBool>,
        messages: Arc<Mutex<Vec<String>>>,
        should_fail: bool,
    }
    
    impl MockTransport {
        fn new() -> Self {
            Self {
                connected: Arc::new(AtomicBool::new(false)),
                messages: Arc::new(Mutex::new(Vec::new())),
                should_fail: false,
            }
        }
        
        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }
        
        fn get_sent_messages(&self) -> Vec<String> {
            self.messages.lock().unwrap().clone()
        }
    }
    
    #[async_trait]
    impl Transport for MockTransport {
        async fn connect(&mut self) -> Result<(), TransportError> {
            if self.should_fail {
                return Err(TransportError::ConnectionError(
                    std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Mock failure")
                ));
            }
            self.connected.store(true, Ordering::SeqCst);
            Ok(())
        }
        
        async fn send(&mut self, data: &str) -> Result<(), TransportError> {
            if !self.connected.load(Ordering::SeqCst) {
                return Err(TransportError::ConnectionError(
                    std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected")
                ));
            }
            self.messages.lock().unwrap().push(data.to_string());
            Ok(())
        }
        
        async fn receive(&mut self) -> Result<String, TransportError> {
            Ok("Mock response".to_string())
        }
        
        async fn close(&mut self) -> Result<(), TransportError> {
            self.connected.store(false, Ordering::SeqCst);
            Ok(())
        }
    }
    
    #[tokio::test]
    async fn test_connection_manager_with_mock() {
        let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let mut manager = ConnectionManager::new(config);
        let mock_transport = MockTransport::new();
        
        manager.set_transport(Box::new(mock_transport));
        
        // Test connection
        let result = manager.connect().await;
        assert!(result.is_ok());
        
        // Test message sending
        let result = manager.send_message("Test message".to_string()).await;
        assert!(result.is_ok());
        
        // Verify message was sent (would need access to mock instance)
        // This is a simplified example
    }
    
    #[tokio::test]
    async fn test_connection_failure() {
        let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let mut manager = ConnectionManager::new(config);
        let mock_transport = MockTransport::new().with_failure();
        
        manager.set_transport(Box::new(mock_transport));
        
        // Test connection failure
        let result = manager.connect().await;
        assert!(result.is_err());
        
        // Verify error type
        match result {
            Err(TransportError::ConnectionError(_)) => {
                // Expected error type
            }
            _ => panic!("Expected ConnectionError"),
        }
    }
}
```

### Integration Testing

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tokio::time::{timeout, Duration};
    
    #[tokio::test]
    async fn test_full_connection_cycle() {
        // This test requires a running server
        let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let mut manager = ConnectionManager::new(config.clone());
        
        // Configure with real components
        manager.set_transport(Box::new(TcpTransport::new(config)));
        manager.set_encryption(Box::new(AesGcmEncryption::new("test_password")));
        
        // Test with timeout to avoid hanging
        let connect_result = timeout(Duration::from_secs(5), manager.connect()).await;
        
        match connect_result {
            Ok(Ok(())) => {
                // Connection successful, test messaging
                let send_result = manager.send_message("Integration test".to_string()).await;
                assert!(send_result.is_ok());
                
                // Test disconnection
                let disconnect_result = manager.disconnect().await;
                assert!(disconnect_result.is_ok());
            }
            Ok(Err(e)) => {
                println!("Connection failed (expected if no server): {:?}", e);
            }
            Err(_) => {
                println!("Connection timed out (expected if no server)");
            }
        }
    }
}
```

## Error Handling Examples

### Comprehensive Error Handling

```rust
use crate::client::{migration_facade, transport::TransportError};
use std::time::Duration;
use tokio::time::timeout;

async fn robust_client_with_retries() -> Result<(), Box<dyn std::error::Error>> {
    let input = tui_input::Input::default();
    let address = "127.0.0.1:8080".parse()?;
    let max_retries = 3;
    let retry_delay = Duration::from_secs(2);
    
    // Connection with retries
    for attempt in 1..=max_retries {
        println!("Connection attempt {} of {}", attempt, max_retries);
        
        match timeout(Duration::from_secs(10), migration_facade::connect_client(input.clone(), address)).await {
            Ok(Ok(())) => {
                println!("Connected successfully!");
                break;
            }
            Ok(Err(e)) => {
                eprintln!("Connection failed: {:?}", e);
                if attempt == max_retries {
                    return Err(Box::new(e));
                }
                tokio::time::sleep(retry_delay).await;
            }
            Err(_) => {
                eprintln!("Connection timed out");
                if attempt == max_retries {
                    return Err("Connection timeout after all retries".into());
                }
                tokio::time::sleep(retry_delay).await;
            }
        }
    }
    
    // Send message with error handling
    match migration_facade::send_message("Hello with retry logic!".to_string()).await {
        Ok(()) => println!("Message sent successfully"),
        Err(TransportError::ConnectionError(e)) => {
            eprintln!("Network error: {}", e);
            return Err(Box::new(e));
        }
        Err(TransportError::EncryptionError(e)) => {
            eprintln!("Encryption error: {:?}", e);
            return Err("Encryption failed".into());
        }
        Err(e) => {
            eprintln!("Other error: {:?}", e);
            return Err(Box::new(e));
        }
    }
    
    // Graceful disconnection
    if let Err(e) = migration_facade::disconnect_client().await {
        eprintln!("Warning: Disconnect failed: {:?}", e);
        // Not critical, continue
    }
    
    Ok(())
}

// Error recovery example
async fn client_with_automatic_recovery() -> Result<(), TransportError> {
    let input = tui_input::Input::default();
    let address = "127.0.0.1:8080".parse().unwrap();
    
    // Initial connection
    migration_facade::connect_client(input.clone(), address).await?;
    
    // Send messages with automatic reconnection
    let messages = vec!["Message 1", "Message 2", "Message 3"];
    
    for message in messages {
        loop {
            match migration_facade::send_message(message.to_string()).await {
                Ok(()) => {
                    println!("Sent: {}", message);
                    break; // Success, move to next message
                }
                Err(TransportError::ConnectionError(_)) => {
                    println!("Connection lost, attempting to reconnect...");
                    
                    // Try to reconnect
                    match migration_facade::connect_client(input.clone(), address).await {
                        Ok(()) => {
                            println!("Reconnected successfully");
                            // Retry sending the message
                            continue;
                        }
                        Err(e) => {
                            eprintln!("Reconnection failed: {:?}", e);
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Non-recoverable error: {:?}", e);
                    return Err(e);
                }
            }
        }
    }
    
    migration_facade::disconnect_client().await?;
    Ok(())
}
```

## Custom Implementation Examples

### Custom Transport Implementation

```rust
use crate::client::transport::{Transport, TransportError};
use async_trait::async_trait;
use std::collections::VecDeque;
use tokio::sync::Mutex;

// Example: In-memory transport for testing
pub struct InMemoryTransport {
    connected: bool,
    outbound_queue: Mutex<VecDeque<String>>,
    inbound_queue: Mutex<VecDeque<String>>,
}

impl InMemoryTransport {
    pub fn new() -> Self {
        Self {
            connected: false,
            outbound_queue: Mutex::new(VecDeque::new()),
            inbound_queue: Mutex::new(VecDeque::new()),
        }
    }
    
    // Test helper: inject a message to be received
    pub async fn inject_message(&self, message: String) {
        self.inbound_queue.lock().await.push_back(message);
    }
    
    // Test helper: get sent messages
    pub async fn get_sent_messages(&self) -> Vec<String> {
        self.outbound_queue.lock().await.iter().cloned().collect()
    }
}

#[async_trait]
impl Transport for InMemoryTransport {
    async fn connect(&mut self) -> Result<(), TransportError> {
        if self.connected {
            return Err(TransportError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Already connected")
            ));
        }
        self.connected = true;
        Ok(())
    }
    
    async fn send(&mut self, data: &str) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected")
            ));
        }
        
        self.outbound_queue.lock().await.push_back(data.to_string());
        Ok(())
    }
    
    async fn receive(&mut self) -> Result<String, TransportError> {
        if !self.connected {
            return Err(TransportError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected")
            ));
        }
        
        match self.inbound_queue.lock().await.pop_front() {
            Some(message) => Ok(message),
            None => Err(TransportError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::WouldBlock, "No messages available")
            )),
        }
    }
    
    async fn close(&mut self) -> Result<(), TransportError> {
        self.connected = false;
        self.outbound_queue.lock().await.clear();
        self.inbound_queue.lock().await.clear();
        Ok(())
    }
}

// Usage example
async fn custom_transport_example() -> Result<(), TransportError> {
    use crate::client::connection_manager::{ConnectionManager, ConnectionConfig};
    
    let config = ConnectionConfig::new("127.0.0.1:0".parse().unwrap()); // Dummy address
    let mut manager = ConnectionManager::new(config);
    
    let custom_transport = InMemoryTransport::new();
    custom_transport.inject_message("Hello from server!".to_string()).await;
    
    manager.set_transport(Box::new(custom_transport));
    
    manager.connect().await?;
    manager.send_message("Hello from client!".to_string()).await?;
    
    // In a real scenario, you'd have a receive loop
    // let received = manager.receive_message().await?;
    
    manager.disconnect().await?;
    Ok(())
}
```

### Custom Observer Implementation

```rust
use crate::client::transport::{ConnectionObserver, ConnectionStatus};
use std::sync::Arc;
use tokio::sync::mpsc;

// Example: Event-driven observer that sends events to a channel
pub struct ChannelObserver {
    sender: mpsc::UnboundedSender<ConnectionEvent>,
}

#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    StatusChanged { old: ConnectionStatus, new: ConnectionStatus },
    MessageReceived(String),
    MessageSent(String),
    Error(String),
}

impl ChannelObserver {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<ConnectionEvent>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self { sender }, receiver)
    }
}

impl ConnectionObserver for ChannelObserver {
    fn on_status_change(&self, old_status: ConnectionStatus, new_status: ConnectionStatus) {
        let _ = self.sender.send(ConnectionEvent::StatusChanged {
            old: old_status,
            new: new_status,
        });
    }
    
    fn on_message_received(&self, message: &str) {
        let _ = self.sender.send(ConnectionEvent::MessageReceived(message.to_string()));
    }
    
    fn on_message_sent(&self, message: &str) {
        let _ = self.sender.send(ConnectionEvent::MessageSent(message.to_string()));
    }
    
    fn on_error(&self, error: &str) {
        let _ = self.sender.send(ConnectionEvent::Error(error.to_string()));
    }
}

// Usage example
async fn custom_observer_example() -> Result<(), Box<dyn std::error::Error>> {
    use crate::client::connection_manager::{ConnectionManager, ConnectionConfig};
    
    let config = ConnectionConfig::new("127.0.0.1:8080".parse()?);
    let mut manager = ConnectionManager::new(config);
    
    let (observer, mut event_receiver) = ChannelObserver::new();
    manager.register_observer(Box::new(observer));
    
    // Spawn a task to handle events
    let event_handler = tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            match event {
                ConnectionEvent::StatusChanged { old, new } => {
                    println!("Status: {:?} -> {:?}", old, new);
                }
                ConnectionEvent::MessageReceived(msg) => {
                    println!("Received: {}", msg);
                }
                ConnectionEvent::MessageSent(msg) => {
                    println!("Sent: {}", msg);
                }
                ConnectionEvent::Error(err) => {
                    eprintln!("Error: {}", err);
                }
            }
        }
    });
    
    // Use the manager
    manager.connect().await?;
    manager.send_message("Hello!".to_string()).await?;
    manager.disconnect().await?;
    
    // Clean up
    event_handler.abort();
    
    Ok(())
}
```

## Best Practices

### Connection Management

```rust
// Good: Use RAII pattern for connection management
async fn good_connection_management() -> Result<(), TransportError> {
    let mut manager = create_connection_manager()?;
    
    // Connect
    manager.connect().await?;
    
    // Ensure cleanup even on error
    let result = async {
        manager.send_message("Hello".to_string()).await?;
        // ... other operations
        Ok(())
    }.await;
    
    // Always disconnect
    let _ = manager.disconnect().await;
    
    result
}

// Better: Use a connection guard
struct ConnectionGuard {
    manager: ConnectionManager,
}

impl ConnectionGuard {
    async fn new(config: ConnectionConfig) -> Result<Self, TransportError> {
        let mut manager = ConnectionManager::new(config);
        manager.connect().await?;
        Ok(Self { manager })
    }
    
    async fn send_message(&mut self, message: String) -> Result<(), TransportError> {
        self.manager.send_message(message).await
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        // Note: In real code, you'd need an async drop or explicit cleanup
        // This is a simplified example
    }
}
```

### Error Handling Patterns

```rust
// Pattern: Specific error handling with recovery
async fn handle_specific_errors() -> Result<(), Box<dyn std::error::Error>> {
    let result = migration_facade::connect_client(
        tui_input::Input::default(),
        "127.0.0.1:8080".parse()?
    ).await;
    
    match result {
        Ok(()) => {
            println!("Connected successfully");
        }
        Err(TransportError::ConnectionError(e)) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
            println!("Server not available, starting in offline mode");
            return start_offline_mode().await;
        }
        Err(TransportError::EncryptionError(e)) => {
            println!("Encryption setup failed: {:?}", e);
            return Err("Failed to establish secure connection".into());
        }
        Err(e) => {
            println!("Unexpected error: {:?}", e);
            return Err(Box::new(e));
        }
    }
    
    Ok(())
}

async fn start_offline_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running in offline mode");
    // Implement offline functionality
    Ok(())
}
```

These examples demonstrate practical usage patterns for the lair-chat transport architecture, from basic connections to advanced custom implementations. Choose the approach that best fits your use case and gradually migrate from legacy patterns to the new architecture.