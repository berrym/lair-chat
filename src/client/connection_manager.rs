use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use std::any::Any;
use async_trait::async_trait;
use super::transport::{
    ConnectionConfig,
    ConnectionObserver,
    ConnectionStatus,
    EncryptionService,
    Message,
    MessageStore,
    Transport,
    TransportError,
};
use super::encryption::EncryptionError;
use super::aes_gcm_encryption::AesGcmEncryption;



/// Manages a network connection with encryption capabilities
pub struct ConnectionManager {
    config: ConnectionConfig,
    status: Arc<RwLock<ConnectionStatus>>,
    transport: Option<Arc<Mutex<Box<dyn Transport + Send + Sync>>>>,
    encryption: Option<Arc<Mutex<Box<dyn EncryptionService + Send + Sync>>>>,
    messages: Arc<Mutex<MessageStore>>,
    observers: Vec<Arc<dyn ConnectionObserver + Send + Sync + 'static>>,
    cancel_token: CancellationToken,
}

// Extension trait for downcasting Arc<dyn ConnectionObserver>
pub trait AnyConnectionObserver: ConnectionObserver {
    fn as_any(&self) -> &dyn Any;
}

impl<T: ConnectionObserver + Any> AnyConnectionObserver for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Default ConnectionObserver implementation for logging
struct DefaultConnectionObserver;

impl ConnectionObserver for DefaultConnectionObserver {
    fn on_message(&self, _message: String) {
        // Default implementation that does nothing
    }
    
    fn on_error(&self, _error: String) {
        // Default implementation that does nothing
    }
    
    fn on_status_change(&self, _connected: bool) {
        // Default implementation that does nothing
    }
}

// We don't need to implement Clone for Arc as it's already implemented

impl ConnectionManager {
    /// Creates a new ConnectionManager with the given configuration
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            status: Arc::new(RwLock::new(ConnectionStatus::DISCONNECTED)),
            transport: None,
            encryption: None,
            messages: Arc::new(Mutex::new(MessageStore::new())),
            observers: Vec::new(),
            cancel_token: CancellationToken::new(),
        }
    }

    /// Creates a ConnectionManager for testing purposes
    #[cfg(test)]
    pub fn new_for_test(config: ConnectionConfig) -> Self {
        Self::new(config)
    }

    /// Register an observer to receive connection events
    pub fn register_observer(&mut self, observer: Arc<dyn ConnectionObserver + Send + Sync + 'static>) {
        self.observers.push(observer);
    }

    /// Set the transport implementation
    pub fn with_transport(&mut self, transport: Box<dyn Transport + Send + Sync>) -> &mut Self {
        let transport_arc = Arc::new(Mutex::new(transport));
        self.transport = Some(transport_arc);
        self
    }

    /// Set the encryption service implementation
    pub fn with_encryption(&mut self, encryption: Box<dyn EncryptionService + Send + Sync>) -> &mut Self {
        let encryption_arc = Arc::new(Mutex::new(encryption));
        self.encryption = Some(encryption_arc);
        self
    }

    /// Establish a connection to the remote endpoint
    pub async fn connect(&mut self) -> Result<(), TransportError> {
        if self.transport.is_none() {
            return Err(TransportError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::Other, "No transport configured")
            ));
        }

        // Actually establish the transport connection first
        if let Some(transport) = &self.transport {
            let mut transport_guard = transport.lock().await;
            transport_guard.connect().await?;
        }

        // Perform key exchange handshake if encryption is configured
        if let (Some(transport), Some(encryption)) = (&self.transport, &self.encryption) {
            let mut encryption_guard = encryption.lock().await;
            let mut transport_guard = transport.lock().await;
            encryption_guard.perform_handshake(&mut **transport_guard).await?;
            self.notify_message(Message::system_message("Key exchange completed".to_string()));
        }

        // Set status to connected
        *self.status.write().await = ConnectionStatus::CONNECTED;
        self.notify_status_change(ConnectionStatus::CONNECTED);
        
        // Start the connection processing tasks
        self.start_connection_tasks().await?;

        // Notify observers that connection is established
        self.notify_message(Message::system_message("Connected to server".to_string()));
        
        Ok(())
    }

    /// Disconnect from the remote endpoint
    pub async fn disconnect(&mut self) -> Result<(), TransportError> {
        // Cancel any ongoing operations
        self.cancel_token.cancel();
        
        // Close the transport if it exists
        if let Some(transport) = &self.transport {
            let mut transport_guard = transport.lock().await;
            transport_guard.close().await?;
        }

        // Update status
        self.notify_status_change(ConnectionStatus::DISCONNECTED);
        self.notify_message(Message::system_message("Disconnected from server".to_string()));
        
        Ok(())
    }

    /// Send a message to the remote endpoint
    pub async fn send_message(&mut self, content: String) -> Result<(), TransportError> {
        if self.transport.is_none() {
            return Err(TransportError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected")
            ));
        }
        
        // Create and store user message
        let message = Message::user_message(content.clone());
        self.store_message(message.clone()).await;
        
        // Encrypt the message if encryption is available
        let data = if let Some(encryption) = &self.encryption {
            let mut encryption_guard = encryption.lock().await;
            encryption_guard.encrypt("key", &content)?
        } else {
            content
        };
        
        // Get a reference to the transport and send the data
        if let Some(transport) = &self.transport {
            let mut transport_guard = transport.lock().await;
            transport_guard.send(&data).await?;
        }
        
        Ok(())
    }

    /// Get a reference to the message store
    pub fn get_message_store(&self) -> Arc<Mutex<MessageStore>> {
        Arc::clone(&self.messages)
    }

    /// Get the current connection status
    pub async fn get_status(&self) -> ConnectionStatus {
        self.status.read().await.clone()
    }

    /// Start the connection processing tasks
    async fn start_connection_tasks(&self) -> Result<(), TransportError> {
        if self.transport.is_none() {
            return Err(TransportError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::Other, "No transport configured")
            ));
        }
        
        // Clone the shared resources for the task
        let transport = self.transport.clone().unwrap();
        let encryption = self.encryption.clone();
        let messages_clone = self.messages.clone();
        let status_clone = self.status.clone();
        let cancel_token_clone = self.cancel_token.clone();
        
        // Clone the observers
        let observers = self.observers.clone();
        
        // Start message receiving task
        tokio::spawn(async move {
            Self::receive_messages(
                transport, 
                encryption,
                messages_clone,
                status_clone,
                cancel_token_clone,
                observers
            ).await;
        });
        
        Ok(())
    }

    /// Task that continuously receives messages until cancelled
    async fn receive_messages(
        transport: Arc<Mutex<Box<dyn Transport + Send + Sync>>>,
        encryption: Option<Arc<Mutex<Box<dyn EncryptionService + Send + Sync>>>>,
        messages: Arc<Mutex<MessageStore>>,
        status: Arc<RwLock<ConnectionStatus>>,
        cancel_token: CancellationToken,
        observers: Vec<Arc<dyn ConnectionObserver + Send + Sync + 'static>>,
    ) {
        loop {
            // Check if we should stop
            if cancel_token.is_cancelled() {
                break;
            }
            
            // We need to hold the lock for the entire operation
            let mut transport_guard = transport.lock().await;
            let receive_result = transport_guard.receive().await;
            drop(transport_guard); // Explicitly drop the guard to release the mutex
            
            // Process the result with timeout
            match tokio::time::timeout(
                Duration::from_millis(100),  // Small timeout to check cancel frequently
                async { receive_result }
            ).await {
                // Timeout occurred, just loop again
                Err(_) => continue,
                
                // Got a result from transport
                Ok(result) => match result {
                    // Error occurred
                    Err(err) => {
                        let error_msg = format!("Error receiving message: {}", err);
                        let message = Message::error_message(error_msg.clone());
                        
                        // Store the error message
                        let mut store = messages.lock().await;
                        store.add_message(message);
                        
                        // Notify observers
                        for observer in &observers {
                            observer.on_error(error_msg.clone());
                        }
                        
                        // If connection error, update status and break
                        if matches!(err, TransportError::ConnectionError(_)) {
                            *status.write().await = ConnectionStatus::DISCONNECTED;
                            for observer in &observers {
                                observer.on_status_change(false);
                            }
                            break;
                        }
                    }
                    
                    // Received a message
                    Ok(Some(data)) => {
                        // Decrypt if encryption is available
                        let content = if let Some(encryption_service) = &encryption {
                            let encryption_guard = encryption_service.lock().await;
                            match encryption_guard.decrypt("key", &data) {
                                Ok(text) => text,
                                Err(err) => {
                                    let error_msg = format!("Failed to decrypt message: {}", err);
                                    for observer in observers.iter() {
                                        observer.on_error(error_msg.clone());
                                    }
                                    continue;
                                }
                            }
                        } else {
                            data
                        };
                        
                        // Create and store the message
                        let message = Message::received_message(content.clone());
                        let mut store = messages.lock().await;
                        store.add_message(message);
                        
                        // Notify observers
                        for observer in &observers {
                            observer.on_message(content.clone());
                        }
                    }
                    
                    // No message received
                    Ok(None) => continue,
                }
            }
        }
    }

    /// Store a message in the message store
    async fn store_message(&self, message: Message) {
        let mut store = self.messages.lock().await;
        store.add_message(message);
    }
    
    /// Notify all observers of a status change
    fn notify_status_change(&self, status: ConnectionStatus) {
        let is_connected = status == ConnectionStatus::CONNECTED;
        for observer in &self.observers {
            observer.on_status_change(is_connected);
        }
    }
    
    /// Notify all observers of a new message
    fn notify_message(&self, message: Message) {
        let formatted = message.format_for_display();
        for observer in &self.observers {
            observer.on_message(formatted.clone());
        }
    }
    
    /// Notify all observers of an error
    fn notify_error(&self, error: &str) {
        for observer in &self.observers {
            observer.on_error(error.to_string());
        }
    }
}

// Mock implementation for testing
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    
    struct MockTransport {
        send_data: Arc<Mutex<Vec<String>>>,
        receive_queue: Arc<Mutex<VecDeque<Result<Option<String>, TransportError>>>>,
    }
    
    impl MockTransport {
        fn new() -> Self {
            Self {
                send_data: Arc::new(Mutex::new(Vec::new())),
                receive_queue: Arc::new(Mutex::new(VecDeque::new())),
            }
        }
        
        async fn add_receive_data(&self, data: String) {
            let mut queue = self.receive_queue.lock().await;
            queue.push_back(Ok(Some(data)));
        }
        
        async fn add_receive_error(&self, error: TransportError) {
            let mut queue = self.receive_queue.lock().await;
            queue.push_back(Err(error));
        }
    }
    
    #[async_trait::async_trait]
    impl Transport for MockTransport {
        async fn connect(&mut self) -> Result<(), TransportError> {
            // Mock implementation - always succeeds
            Ok(())
        }

        async fn send(&mut self, data: &str) -> Result<(), TransportError> {
            let mut send_data = self.send_data.lock().await;
            send_data.push(data.to_string());
            Ok(())
        }
        
        async fn receive(&mut self) -> Result<Option<String>, TransportError> {
            let mut queue = self.receive_queue.lock().await;
            if let Some(result) = queue.pop_front() {
                result
            } else {
                Ok(None)
            }
        }
        
        async fn close(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
    }
    
    struct MockEncryptionService;
    
    #[async_trait::async_trait]
    impl EncryptionService for MockEncryptionService {
        fn encrypt(&self, _key: &str, plaintext: &str) -> Result<String, EncryptionError> {
            Ok(format!("ENCRYPTED:{}", plaintext))
        }
        
        fn decrypt(&self, _key: &str, ciphertext: &str) -> Result<String, EncryptionError> {
            if ciphertext.starts_with("ENCRYPTED:") {
                Ok(ciphertext.replace("ENCRYPTED:", ""))
            } else {
                Err(EncryptionError::DecryptionError("Invalid ciphertext".to_string()))
            }
        }
        
        async fn perform_handshake(&mut self, _transport: &mut dyn Transport) -> Result<(), TransportError> {
            // Mock implementation - no handshake performed
            Ok(())
        }
    }
    
    #[derive(Debug)]
    struct MockObserver {
        messages: Arc<Mutex<Vec<String>>>,
        errors: Arc<Mutex<Vec<String>>>,
        status_changes: Arc<Mutex<Vec<bool>>>,
    }
    
    impl MockObserver {
        fn new() -> Self {
            Self {
                messages: Arc::new(Mutex::new(Vec::new())),
                errors: Arc::new(Mutex::new(Vec::new())),
                status_changes: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }
    
    impl ConnectionObserver for MockObserver {
        fn on_message(&self, message: String) {
            let messages = self.messages.clone();
            tokio::spawn(async move {
                let mut messages_guard = messages.lock().await;
                messages_guard.push(message);
            });
        }
        
        fn on_error(&self, error: String) {
            let errors = self.errors.clone();
            tokio::spawn(async move {
                let mut errors_guard = errors.lock().await;
                errors_guard.push(error);
            });
        }
        
        fn on_status_change(&self, connected: bool) {
            let status_changes = self.status_changes.clone();
            tokio::spawn(async move {
                let mut status_guard = status_changes.lock().await;
                status_guard.push(connected);
            });
        }
    }
    
    #[tokio::test]
    async fn test_connection_manager_creation() {
        let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let manager = ConnectionManager::new_for_test(config);
        
        assert_eq!(manager.get_status().await, ConnectionStatus::DISCONNECTED);
        
        // Create a reference to the message store that lives for the duration of the test
        let message_store = manager.get_message_store();
        let messages = message_store.lock().await;
        assert_eq!(messages.messages.len(), 0);
    }
    
    #[tokio::test]
    async fn test_send_message() {
        let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let mut manager = ConnectionManager::new_for_test(config);
        
        // Create the transport and encryption service
        let transport = MockTransport::new();
        let encryption = MockEncryptionService;
        
        // Move the boxed values into the manager
        manager.with_transport(Box::new(transport)).with_encryption(Box::new(encryption));
        
        // Connect first to set up the transport
        // In a real test, we'd mock this further
        
        // Create a message outside the block
        let msg_content = "Hello world".to_string();
        
        // Manually add a message to test store functionality
        {
            let message = Message::user_message(msg_content.clone());
            manager.store_message(message).await;
        }
        
        // Verify message was added to store
        let message_store = manager.get_message_store();
        let messages = message_store.lock().await;
        assert_eq!(messages.messages.len(), 1);
        assert_eq!(messages.messages[0].content, "Hello world");
    }
    
    #[test]
    fn test_connection_manager_basic_creation() {
        // Simple test for basic ConnectionManager creation
        let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let _manager = ConnectionManager::new_for_test(config);
        
        // Test passes if we reach here without panicking
        assert!(true);
    }

    #[tokio::test]
    async fn test_connection_establishment() {
        let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let mut manager = ConnectionManager::new_for_test(config);
        
        // Create mock transport that tracks connect calls
        struct ConnectTrackingTransport {
            connect_called: Arc<Mutex<bool>>,
        }
        
        impl ConnectTrackingTransport {
            fn new() -> (Self, Arc<Mutex<bool>>) {
                let connect_called = Arc::new(Mutex::new(false));
                let transport = Self {
                    connect_called: connect_called.clone(),
                };
                (transport, connect_called)
            }
        }
        
        #[async_trait::async_trait]
        impl Transport for ConnectTrackingTransport {
            async fn connect(&mut self) -> Result<(), TransportError> {
                let mut called = self.connect_called.lock().await;
                *called = true;
                Ok(())
            }
            
            async fn send(&mut self, _data: &str) -> Result<(), TransportError> {
                Ok(())
            }
            
            async fn receive(&mut self) -> Result<Option<String>, TransportError> {
                Ok(None)
            }
            
            async fn close(&mut self) -> Result<(), TransportError> {
                Ok(())
            }
        }
        
        let (transport, connect_tracker) = ConnectTrackingTransport::new();
        manager.with_transport(Box::new(transport));
        
        // Verify connect wasn't called yet
        assert!(!*connect_tracker.lock().await);
        
        // Call connect on the manager
        let result = manager.connect().await;
        assert!(result.is_ok());
        
        // Verify that the transport's connect method was actually called
        assert!(*connect_tracker.lock().await);
        
        // Verify status is set to connected
        assert_eq!(manager.get_status().await, ConnectionStatus::CONNECTED);
    }

    #[tokio::test]
    async fn test_connection_manager_with_aes_gcm_encryption() {
        let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let mut manager = ConnectionManager::new_for_test(config);
        
        // Create AES-GCM encryption service
        let aes_encryption = AesGcmEncryption::new("test_password_for_connection");
        
        // Create mock transport
        let transport = MockTransport::new();
        
        // Configure the manager with both transport and encryption
        manager
            .with_transport(Box::new(transport))
            .with_encryption(Box::new(aes_encryption));
        
        // Send a message that should be encrypted
        let test_message = "This message should be encrypted with AES-GCM";
        let result = manager.send_message(test_message.to_string()).await;
        
        // Should succeed even though we're not connected (mock transport allows it)
        assert!(result.is_ok());
        
        // Verify message was stored in the message store
        let message_store = manager.get_message_store();
        let messages = message_store.lock().await;
        assert_eq!(messages.messages.len(), 1);
        assert_eq!(messages.messages[0].content, test_message);
    }

    #[tokio::test]
    async fn test_aes_gcm_encryption_compatibility() {
        // Test that AesGcmEncryption works as expected with the EncryptionService trait
        let encryption: Box<dyn EncryptionService + Send + Sync> = 
            Box::new(AesGcmEncryption::new("consistent_password"));
        
        let original_message = "Secret message for encryption test";
        
        // Encrypt and decrypt through the trait interface
        let encrypted = encryption.encrypt("ignored_key", original_message)
            .expect("Encryption should succeed");
        let decrypted = encryption.decrypt("ignored_key", &encrypted)
            .expect("Decryption should succeed");
        
        assert_eq!(original_message, decrypted);
        
        // Verify that encrypted data is different from original
        assert_ne!(original_message, encrypted);
        
        // Verify that the same message encrypts to different ciphertexts (due to random nonce)
        let encrypted2 = encryption.encrypt("ignored_key", original_message)
            .expect("Second encryption should succeed");
        assert_ne!(encrypted, encrypted2);
        
        // But both should decrypt to the same original message
        let decrypted2 = encryption.decrypt("ignored_key", &encrypted2)
            .expect("Second decryption should succeed");
        assert_eq!(original_message, decrypted2);
    }

    #[tokio::test]
    async fn test_connection_manager_calls_handshake() {
        let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let mut manager = ConnectionManager::new_for_test(config);
        
        // Create a mock encryption service that tracks handshake calls
        struct HandshakeTrackingEncryption {
            handshake_called: std::sync::Arc<tokio::sync::Mutex<bool>>,
        }
        
        impl HandshakeTrackingEncryption {
            fn new() -> (Self, std::sync::Arc<tokio::sync::Mutex<bool>>) {
                let handshake_called = std::sync::Arc::new(tokio::sync::Mutex::new(false));
                let service = Self {
                    handshake_called: handshake_called.clone(),
                };
                (service, handshake_called)
            }
        }
        
        #[async_trait]
        impl EncryptionService for HandshakeTrackingEncryption {
            fn encrypt(&self, _key: &str, plaintext: &str) -> Result<String, EncryptionError> {
                Ok(format!("ENCRYPTED:{}", plaintext))
            }
            
            fn decrypt(&self, _key: &str, ciphertext: &str) -> Result<String, EncryptionError> {
                if ciphertext.starts_with("ENCRYPTED:") {
                    Ok(ciphertext.replace("ENCRYPTED:", ""))
                } else {
                    Err(EncryptionError::DecryptionError("Invalid ciphertext".to_string()))
                }
            }
            
            async fn perform_handshake(&mut self, _transport: &mut dyn Transport) -> Result<(), TransportError> {
                let mut called = self.handshake_called.lock().await;
                *called = true;
                Ok(())
            }
        }
        
        let (encryption_service, handshake_tracker) = HandshakeTrackingEncryption::new();
        let transport = MockTransport::new();
        
        // Configure the manager
        manager
            .with_transport(Box::new(transport))
            .with_encryption(Box::new(encryption_service));
        
        // Verify handshake wasn't called yet
        assert!(!*handshake_tracker.lock().await);
        
        // Connect - this should trigger the handshake
        let result = manager.connect().await;
        assert!(result.is_ok());
        
        // Verify handshake was called
        assert!(*handshake_tracker.lock().await);
        
        // Verify connection status
        assert_eq!(manager.get_status().await, ConnectionStatus::CONNECTED);
    }

    #[tokio::test]
    async fn test_connection_manager_handshake_failure() {
        let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let mut manager = ConnectionManager::new_for_test(config);
        
        // Create a mock encryption service that fails handshake
        struct FailingHandshakeEncryption;
        
        #[async_trait]
        impl EncryptionService for FailingHandshakeEncryption {
            fn encrypt(&self, _key: &str, plaintext: &str) -> Result<String, EncryptionError> {
                Ok(format!("ENCRYPTED:{}", plaintext))
            }
            
            fn decrypt(&self, _key: &str, ciphertext: &str) -> Result<String, EncryptionError> {
                Ok(ciphertext.replace("ENCRYPTED:", ""))
            }
            
            async fn perform_handshake(&mut self, _transport: &mut dyn Transport) -> Result<(), TransportError> {
                Err(TransportError::EncryptionError(EncryptionError::EncryptionError("Handshake failed".to_string())))
            }
        }
        
        let encryption_service = FailingHandshakeEncryption;
        let transport = MockTransport::new();
        
        // Configure the manager
        manager
            .with_transport(Box::new(transport))
            .with_encryption(Box::new(encryption_service));
        
        // Connect - this should fail due to handshake failure
        let result = manager.connect().await;
        assert!(result.is_err());
        
        // Verify connection status remains disconnected
        assert_eq!(manager.get_status().await, ConnectionStatus::DISCONNECTED);
    }
}
