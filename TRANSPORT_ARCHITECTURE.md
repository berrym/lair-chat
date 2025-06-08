# Transport Architecture Documentation

## Overview

This document provides comprehensive documentation for the lair-chat transport system architecture, which has been refactored to support a modular, testable, and maintainable design.

## Architecture Components

### Core Components

#### 1. ConnectionManager
The central orchestrator that manages the entire connection lifecycle.

**Location:** `src/client/connection_manager.rs`

**Purpose:**
- Coordinates transport, encryption, and observer components
- Manages connection state and lifecycle
- Handles message routing and error propagation
- Provides a unified interface for connection operations

**Key Methods:**
```rust
// Create a new connection manager
pub fn new(config: ConnectionConfig) -> Self

// Establish connection with handshake
pub async fn connect(&mut self) -> Result<(), TransportError>

// Send message through the transport layer
pub async fn send_message(&mut self, content: String) -> Result<(), TransportError>

// Get current connection status
pub async fn get_status(&self) -> ConnectionStatus

// Gracefully disconnect
pub async fn disconnect(&mut self) -> Result<(), TransportError>
```

#### 2. Transport Layer
Abstraction for different transport protocols.

**Location:** `src/client/transport.rs`, `src/client/tcp_transport.rs`

**Transport Trait:**
```rust
pub trait Transport: Send + Sync {
    async fn connect(&mut self) -> Result<(), TransportError>;
    async fn send(&mut self, data: &str) -> Result<(), TransportError>;
    async fn receive(&mut self) -> Result<String, TransportError>;
    async fn close(&mut self) -> Result<(), TransportError>;
}
```

**Current Implementation:**
- `TcpTransport`: TCP-based transport with async I/O

#### 3. Encryption Service
Handles message encryption and key exchange.

**Location:** `src/client/encryption.rs`, `src/client/aes_gcm_encryption.rs`

**EncryptionService Trait:**
```rust
pub trait EncryptionService: Send + Sync {
    fn encrypt(&self, key: &str, plaintext: &str) -> Result<String, EncryptionError>;
    fn decrypt(&self, key: &str, ciphertext: &str) -> Result<String, EncryptionError>;
    async fn perform_handshake(&mut self, transport: &mut dyn Transport) -> Result<(), TransportError>;
}
```

**Current Implementation:**
- `AesGcmEncryption`: AES-GCM encryption with X25519 key exchange

#### 4. Connection Observer
Event-driven notifications for connection events.

**Location:** `src/client/transport.rs`

**ConnectionObserver Trait:**
```rust
pub trait ConnectionObserver: Send + Sync {
    fn on_status_change(&self, old_status: ConnectionStatus, new_status: ConnectionStatus);
    fn on_message_received(&self, message: &str);
    fn on_message_sent(&self, message: &str);
    fn on_error(&self, error: &str);
}
```

**Current Implementation:**
- `TuiObserver`: Updates the TUI with connection events

## Migration Strategy

### Migration Facade
The migration facade provides a seamless transition from the legacy transport system to the new architecture.

**Location:** `src/client/migration_facade.rs`

**Key Functions:**
```rust
// Drop-in replacement for legacy connect_client
pub async fn connect_client(input: Input, address: SocketAddr) -> Result<(), TransportError>

// Drop-in replacement for legacy disconnect_client  
pub async fn disconnect_client() -> Result<(), TransportError>

// Send message through the new architecture
pub async fn send_message(message: String) -> Result<(), TransportError>
```

### Compatibility Layer
Provides backward compatibility for existing code that hasn't been migrated yet.

**Location:** `src/client/compatibility_layer.rs`

**Features:**
- Maintains global state compatibility
- Preserves original function signatures
- Automatic fallback to legacy behavior when needed

## Usage Examples

### Basic Connection Setup

```rust
use crate::client::connection_manager::{ConnectionManager, ConnectionConfig};
use crate::client::tcp_transport::TcpTransport;
use crate::client::aes_gcm_encryption::AesGcmEncryption;
use crate::client::transport::TuiObserver;

// Create configuration
let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());

// Create connection manager
let manager = ConnectionManager::new(config)
    .with_transport(Box::new(TcpTransport::new(config.clone())))
    .with_encryption(Box::new(AesGcmEncryption::new("password123")))
    .with_observer(Box::new(TuiObserver::new()));

// Connect
manager.connect().await?;

// Send message
manager.send_message("Hello, world!".to_string()).await?;

// Disconnect
manager.disconnect().await?;
```

### Using Migration Facade (Recommended for Gradual Migration)

```rust
use crate::client::migration_facade;

// Replace legacy connect_client calls
let input = tui_input::Input::default();
let address = "127.0.0.1:8080".parse().unwrap();
migration_facade::connect_client(input, address).await?;

// Send messages
migration_facade::send_message("Hello!".to_string()).await?;

// Disconnect
migration_facade::disconnect_client().await?;
```

## Configuration

### Environment Variables

The system supports configuration through environment variables:

- `LAIR_USE_NEW_TRANSPORT`: Enable new transport architecture (default: true)
- `LAIR_TRANSPORT_TIMEOUT`: Connection timeout in milliseconds (default: 5000)
- `LAIR_ENCRYPTION_KEY`: Default encryption key for connections
- `LAIR_VERBOSE_LOGGING`: Enable detailed transport logging (default: false)

### Programmatic Configuration

```rust
use crate::client::migration_facade::{MigrationConfig, init_migration};

let config = MigrationConfig {
    use_new_architecture: true,
    auto_detect: true,
    env_var_name: "LAIR_USE_NEW_TRANSPORT".to_string(),
    verbose_logging: false,
};

init_migration(config);
```

## Error Handling

### TransportError Types

```rust
pub enum TransportError {
    ConnectionError(std::io::Error),
    EncryptionError(EncryptionError),
    AuthenticationError(String),
    ProtocolError(String),
    Timeout(String),
    KeyExchangeError(String),
}
```

### Error Propagation

- Errors are propagated through the `Result<T, TransportError>` type
- ConnectionObserver receives error notifications
- Graceful degradation when possible

## Testing

### Unit Tests

The architecture includes comprehensive unit tests:

- **ConnectionManager Tests**: 8 tests covering creation, connection, and message sending
- **Transport Tests**: 18 tests covering TCP transport functionality  
- **Encryption Tests**: 18 tests covering AES-GCM encryption and key exchange
- **Integration Tests**: Cross-component testing scenarios

### Mock Implementations

Mock implementations are provided for testing:

```rust
// Mock transport for testing
struct MockTransport {
    connect_called: Arc<AtomicBool>,
    send_data: Arc<Mutex<Vec<String>>>,
    receive_data: Arc<Mutex<Vec<String>>>,
}

// Mock encryption for testing
struct MockEncryption;

// Mock observer for testing  
struct MockObserver {
    messages: Arc<Mutex<Vec<String>>>,
    status_changes: Arc<Mutex<Vec<(ConnectionStatus, ConnectionStatus)>>>,
}
```

### Running Tests

```bash
# Run all transport tests
cargo test transport

# Run connection manager tests
cargo test connection_manager

# Run encryption tests
cargo test aes_gcm

# Run all tests with coverage
cargo test --all-features
```

## Performance Characteristics

### Benchmarks

- **Connection Establishment**: ~50-100ms for local connections
- **Message Throughput**: ~1000 messages/second for small messages
- **Memory Usage**: ~2MB base overhead for connection management
- **Encryption Overhead**: ~5-10% performance impact for AES-GCM

### Optimization Recommendations

1. **Connection Pooling**: Reuse connections when possible
2. **Message Batching**: Batch small messages to reduce syscall overhead
3. **Async I/O**: Leverage Tokio's async capabilities for high concurrency
4. **Memory Management**: Use `Arc<Mutex<>>` judiciously to avoid contention

## Security Considerations

### Encryption

- **Algorithm**: AES-GCM with 256-bit keys
- **Key Exchange**: X25519 elliptic curve Diffie-Hellman
- **Nonce Generation**: Cryptographically secure random nonces
- **Key Derivation**: SHA-256 based key derivation from passwords

### Best Practices

1. **Key Management**: Store keys securely, rotate regularly
2. **Input Validation**: Validate all incoming data
3. **Error Handling**: Don't leak sensitive information in error messages
4. **Timing Attacks**: Use constant-time operations for cryptographic comparisons

## Monitoring and Debugging

### Logging

The system uses structured logging with different levels:

```rust
use tracing::{info, debug, warn, error};

// Connection events
info!("Connection established to {}", address);

// Debug information
debug!("Sending message: {}", message);

// Warnings
warn!("Connection timeout, retrying...");

// Errors
error!("Failed to encrypt message: {}", error);
```

### Metrics

Key metrics to monitor:

- Connection success/failure rates
- Message throughput and latency
- Encryption/decryption performance
- Memory and CPU usage

### Debugging Tips

1. **Enable Verbose Logging**: Set `LAIR_VERBOSE_LOGGING=true`
2. **Use Mock Components**: Replace real components with mocks for isolation
3. **Connection State**: Monitor connection status changes
4. **Network Issues**: Check firewall and network connectivity

## Migration Guide

### From Legacy to New Architecture

1. **Phase 1**: Use migration facade as drop-in replacement
2. **Phase 2**: Gradually migrate to direct ConnectionManager usage
3. **Phase 3**: Remove compatibility layer and legacy code

### Code Migration Examples

**Before (Legacy):**
```rust
crate::transport::connect_client(input, addr).await;
crate::transport::send_message(message).await;
crate::transport::disconnect_client().await;
```

**After (Migration Facade):**
```rust
migration_facade::connect_client(input, addr).await?;
migration_facade::send_message(message).await?;
migration_facade::disconnect_client().await?;
```

**After (Direct Usage):**
```rust
let mut manager = ConnectionManager::new(config);
manager.connect().await?;
manager.send_message(message).await?;
manager.disconnect().await?;
```

## Troubleshooting

### Common Issues

1. **Connection Timeout**
   - Check network connectivity
   - Verify server is running and accepting connections
   - Increase timeout values

2. **Encryption Failures**
   - Verify matching keys on both sides
   - Check key derivation parameters
   - Ensure proper handshake completion

3. **Message Loss**
   - Check connection stability
   - Verify observer implementations
   - Monitor for transport-level errors

### Support

For issues and questions:
- Check existing tests for usage examples
- Review error messages and logs
- Consult the migration facade for compatibility issues

## Future Enhancements

### Planned Features

1. **WebSocket Transport**: Support for WebSocket connections
2. **Connection Pooling**: Efficient connection reuse
3. **Load Balancing**: Multiple server support
4. **Compression**: Message compression for bandwidth optimization
5. **Metrics Export**: Prometheus/OpenTelemetry integration

### Extensibility

The architecture is designed for extensibility:

- **Transport Protocols**: Implement the Transport trait
- **Encryption Algorithms**: Implement the EncryptionService trait  
- **Observers**: Implement the ConnectionObserver trait
- **Configuration**: Extend ConnectionConfig as needed

## Conclusion

The new transport architecture provides a solid foundation for scalable, secure, and maintainable network communications in lair-chat. The migration strategy ensures a smooth transition while the modular design allows for future enhancements and customizations.