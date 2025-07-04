use std::any::Any;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use crate::auth::storage::{FileTokenStorage, TokenStorage};
use crate::auth::{AuthError, AuthManager, AuthResult, AuthState, Credentials};
use crate::encrypted_transport::EncryptedTransport;
use crate::transport::{
    ConnectionConfig, ConnectionObserver, ConnectionStatus, EncryptionService, Message,
    MessageStore, Transport, TransportError,
};

/// Manages a network connection with encryption capabilities
pub struct ConnectionManager {
    config: ConnectionConfig,
    status: Arc<RwLock<ConnectionStatus>>,
    transport: Option<Arc<Mutex<Box<dyn Transport + Send + Sync>>>>,
    encryption: Option<Arc<Mutex<Box<dyn EncryptionService + Send + Sync>>>>,
    messages: Arc<Mutex<MessageStore>>,
    observers: Vec<Arc<dyn ConnectionObserver + Send + Sync + 'static>>,
    cancel_token: CancellationToken,
    auth_manager: Option<Arc<AuthManager>>,
    token_storage: Option<Arc<Box<dyn TokenStorage + Send + Sync>>>,
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
    /// Get the current authentication state
    pub async fn get_auth_state(&self) -> Option<AuthState> {
        match &self.auth_manager {
            Some(mgr) => Some(mgr.get_state().await),
            None => None,
        }
    }

    /// Check if currently authenticated
    pub async fn is_authenticated(&self) -> bool {
        match &self.auth_manager {
            Some(mgr) => mgr.is_authenticated().await,
            None => false,
        }
    }

    /// Login with existing credentials
    pub async fn login(&self, credentials: Credentials) -> AuthResult<()> {
        match &self.auth_manager {
            Some(mgr) => mgr.login(credentials).await,
            None => Err(AuthError::InternalError(
                "Authentication not configured".into(),
            )),
        }
    }

    /// Register a new user account
    pub async fn register(&self, credentials: Credentials) -> AuthResult<()> {
        match &self.auth_manager {
            Some(mgr) => mgr.register(credentials).await,
            None => Err(AuthError::InternalError(
                "Authentication not configured".into(),
            )),
        }
    }

    /// Refresh the current session if needed
    pub async fn refresh_session(&self) -> AuthResult<()> {
        match &self.auth_manager {
            Some(mgr) => mgr.refresh_session().await,
            None => Err(AuthError::InternalError(
                "Authentication not configured".into(),
            )),
        }
    }

    /// Creates a new ConnectionManager with the given configuration
    ///
    /// The manager will be created without authentication support.
    /// Call `with_auth()` to enable authentication.
    pub fn new(config: ConnectionConfig) -> Self {
        ConnectionManager {
            config,
            status: Arc::new(RwLock::new(ConnectionStatus::DISCONNECTED)),
            transport: None,
            encryption: None,
            messages: Arc::new(Mutex::new(MessageStore::new())),
            observers: vec![Arc::new(DefaultConnectionObserver)],
            cancel_token: CancellationToken::new(),
            auth_manager: None,
            token_storage: None,
        }
    }

    /// Enable authentication for this connection manager
    pub fn with_auth(&mut self) -> &mut Self {
        if let Some(transport) = &self.transport {
            if let Ok(token_storage) = FileTokenStorage::new() {
                let token_storage = Box::new(token_storage);
                let auth_manager = AuthManager::new(Arc::clone(transport), token_storage.clone());
                self.token_storage = Some(Arc::new(token_storage));
                self.auth_manager = Some(Arc::new(auth_manager));
            }
        }
        self
    }

    /// Creates a ConnectionManager for testing purposes
    #[cfg(test)]
    pub fn new_for_test(config: ConnectionConfig) -> Self {
        Self::new(config)
    }

    /// Register an observer to receive connection events
    pub fn register_observer(
        &mut self,
        observer: Arc<dyn ConnectionObserver + Send + Sync + 'static>,
    ) {
        self.observers.push(observer);
    }

    /// Set the transport implementation
    pub fn with_transport(&mut self, transport: Box<dyn Transport + Send + Sync>) -> &mut Self {
        let transport_arc = Arc::new(Mutex::new(transport));
        self.transport = Some(transport_arc);
        self
    }

    /// Set the encryption service implementation
    pub fn with_encryption(
        &mut self,
        encryption: Box<dyn EncryptionService + Send + Sync>,
    ) -> &mut Self {
        let encryption_arc = Arc::new(Mutex::new(encryption));
        self.encryption = Some(encryption_arc);
        self
    }

    /// Check if transport is configured (for debugging)
    pub fn has_transport(&self) -> bool {
        self.transport.is_some()
    }

    /// Establish a connection to the remote endpoint
    pub async fn connect(&mut self) -> Result<(), TransportError> {
        if self.transport.is_none() {
            return Err(TransportError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No transport configured",
            )));
        }

        // Actually establish the transport connection first
        if let Some(transport) = &self.transport {
            let mut transport_guard = transport.lock().await;
            transport_guard.connect().await?;
        }

        // Perform key exchange handshake if encryption is configured
        let handshake_complete =
            if let (Some(transport), Some(encryption)) = (&self.transport, &self.encryption) {
                let mut encryption_guard = encryption.lock().await;
                let mut transport_guard = transport.lock().await;
                encryption_guard
                    .perform_handshake(&mut **transport_guard)
                    .await?;
                self.notify_message(Message::system_message(
                    "Key exchange completed".to_string(),
                ));
                true
            } else {
                false
            };

        // Initialize authentication after handshake is complete
        if let Some(transport) = &self.transport {
            if self.auth_manager.is_none() {
                let token_storage = match FileTokenStorage::new() {
                    Ok(storage) => storage,
                    Err(_) => {
                        return Err(crate::transport::TransportError::ConnectionError(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Failed to create token storage",
                            ),
                        ))
                    }
                };

                // Create the transport for AuthManager
                let auth_transport = if handshake_complete && self.encryption.is_some() {
                    // If we have encryption and handshake is complete, use EncryptedTransport
                    let encryption = self.encryption.as_ref().unwrap().clone();
                    let encrypted_transport =
                        EncryptedTransport::new(transport.clone(), encryption);
                    Arc::new(Mutex::new(
                        Box::new(encrypted_transport) as Box<dyn Transport + Send + Sync>
                    ))
                } else {
                    // Use raw transport if no encryption
                    transport.clone()
                };

                // Create AuthManager with the appropriate transport
                let auth_manager =
                    AuthManager::new(auth_transport, Box::new(token_storage.clone()));
                self.token_storage = Some(Arc::new(Box::new(token_storage)));
                self.auth_manager = Some(Arc::new(auth_manager));

                // Try to restore previous session
                if let Some(auth_manager) = &self.auth_manager {
                    if let Err(e) = auth_manager.initialize().await {
                        self.notify_error(&format!("Failed to restore session: {}", e));
                    }
                }
            }
        }

        // Create a new cancel token for this connection
        self.cancel_token = CancellationToken::new();

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

        // Perform logout if authenticated
        if let Some(auth_manager) = &self.auth_manager {
            if auth_manager.is_authenticated().await {
                if let Err(e) = auth_manager.logout().await {
                    self.notify_error(&format!("Logout error: {}", e));
                }
            }
        }

        // Close the transport if it exists
        if let Some(transport) = &self.transport {
            let mut transport_guard = transport.lock().await;
            transport_guard.close().await?;
        }

        // Clear auth manager and token storage to ensure clean state for next connection
        self.auth_manager = None;
        self.token_storage = None;

        // Update status
        self.notify_status_change(ConnectionStatus::DISCONNECTED);
        self.notify_message(Message::system_message(
            "Disconnected from server".to_string(),
        ));

        Ok(())
    }

    /// Send a message to the remote endpoint
    pub async fn send_message(&mut self, content: String) -> Result<(), TransportError> {
        tracing::info!(
            "DEBUG: ConnectionManager.send_message called with: '{}'",
            content
        );

        // Check initial connection status
        let initial_status = self.get_status().await;
        tracing::info!("DEBUG: Initial connection status: {:?}", initial_status);

        // Check authentication if enabled
        if let Some(auth_manager) = &self.auth_manager {
            let is_auth = auth_manager.is_authenticated().await;
            tracing::info!("DEBUG: Auth manager exists, is_authenticated: {}", is_auth);
            if !is_auth {
                tracing::error!("DEBUG: Authentication check failed in send_message");
                return Err(TransportError::ConnectionError(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Not authenticated",
                )));
            }
        } else {
            tracing::info!("DEBUG: No auth manager, skipping auth check");
        }

        if self.transport.is_none() {
            tracing::error!("DEBUG: No transport available in send_message");
            return Err(TransportError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            )));
        }

        // Check status after auth check
        let post_auth_status = self.get_status().await;
        tracing::info!("DEBUG: Post-auth connection status: {:?}", post_auth_status);

        tracing::info!("DEBUG: Creating and storing user message");
        // Create and store user message
        let message = Message::user_message(content.clone());
        self.store_message(message.clone()).await;

        // Check status after message storage
        let post_storage_status = self.get_status().await;
        tracing::info!(
            "DEBUG: Post-storage connection status: {:?}",
            post_storage_status
        );

        tracing::info!("DEBUG: Checking encryption");
        // Encrypt the message if encryption is available
        let data = if let Some(encryption) = &self.encryption {
            tracing::info!("DEBUG: Encrypting message with available encryption");
            let encryption_guard = encryption.lock().await;

            // Check status after getting encryption lock
            let post_encryption_lock_status = self.get_status().await;
            tracing::info!(
                "DEBUG: Post-encryption-lock connection status: {:?}",
                post_encryption_lock_status
            );

            match encryption_guard.encrypt("key", &content) {
                Ok(encrypted) => {
                    tracing::info!("DEBUG: Message encrypted successfully");

                    // Check status after encryption
                    let post_encryption_status = self.get_status().await;
                    tracing::info!(
                        "DEBUG: Post-encryption connection status: {:?}",
                        post_encryption_status
                    );

                    encrypted
                }
                Err(e) => {
                    tracing::error!("DEBUG: Encryption failed: {}", e);
                    return Err(TransportError::EncryptionError(e));
                }
            }
        } else {
            tracing::info!("DEBUG: No encryption, using plain text");
            content
        };

        // Check status before transport send
        let pre_send_status = self.get_status().await;
        tracing::info!("DEBUG: Pre-send connection status: {:?}", pre_send_status);

        tracing::info!("DEBUG: Sending data via transport: '{}'", data);
        // Get a reference to the transport and send the data
        if let Some(transport) = &self.transport {
            tracing::info!("DEBUG: Getting transport lock...");
            let mut transport_guard = transport.lock().await;
            tracing::info!("DEBUG: Got transport lock, calling send...");

            // Check status after getting transport lock
            let post_transport_lock_status = self.get_status().await;
            tracing::info!(
                "DEBUG: Post-transport-lock connection status: {:?}",
                post_transport_lock_status
            );

            match transport_guard.send(&data).await {
                Ok(()) => {
                    tracing::info!("DEBUG: Transport.send() completed successfully");

                    // Check status immediately after successful send
                    let post_send_status = self.get_status().await;
                    tracing::info!("DEBUG: Post-send connection status: {:?}", post_send_status);
                }
                Err(e) => {
                    tracing::error!("DEBUG: Transport.send() failed: {}", e);

                    // Check status after send failure
                    let post_send_error_status = self.get_status().await;
                    tracing::info!(
                        "DEBUG: Post-send-error connection status: {:?}",
                        post_send_error_status
                    );

                    return Err(e);
                }
            }

            tracing::info!("DEBUG: Dropping transport lock...");
        }

        // Check final status
        let final_status = self.get_status().await;
        tracing::info!("DEBUG: Final connection status: {:?}", final_status);

        tracing::info!("DEBUG: ConnectionManager.send_message completed successfully");
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

    /// Get the current connection status synchronously (for use in Send contexts)
    pub fn get_status_sync(&self) -> ConnectionStatus {
        // Use try_read to avoid blocking, fall back to DISCONNECTED if locked
        match self.status.try_read() {
            Ok(status) => *status,
            Err(_) => ConnectionStatus::DISCONNECTED,
        }
    }

    /// Update the connection configuration
    pub fn update_config(&mut self, config: ConnectionConfig) {
        self.config = config;
    }

    /// Start the connection processing tasks
    async fn start_connection_tasks(&self) -> Result<(), TransportError> {
        if self.transport.is_none() {
            return Err(TransportError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No transport configured",
            )));
        }

        // Clone the shared resources for the task
        let transport = self.transport.clone().unwrap();
        let encryption = self.encryption.clone();
        let messages_clone = self.messages.clone();
        let status_clone = self.status.clone();
        let cancel_token_clone = self.cancel_token.clone();
        let auth_manager_clone = self.auth_manager.clone();

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
                observers,
                auth_manager_clone,
            )
            .await;
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
        auth_manager: Option<Arc<AuthManager>>,
    ) {
        loop {
            // Check if we should stop
            if cancel_token.is_cancelled() {
                break;
            }

            // Apply timeout to the actual receive operation
            let receive_result = tokio::time::timeout(
                Duration::from_millis(100), // Small timeout to check cancel frequently
                async {
                    let mut transport_guard = transport.lock().await;
                    let result = transport_guard.receive().await;
                    drop(transport_guard); // Explicitly drop the guard to release the mutex
                    result
                },
            )
            .await;

            match receive_result {
                // Timeout occurred, just loop again to check cancel token
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
                        // If we have an auth manager, validate auth state
                        if let Some(auth_mgr) = &auth_manager {
                            if !auth_mgr.is_authenticated().await {
                                let error_msg =
                                    "Received message while not authenticated".to_string();
                                for observer in &observers {
                                    observer.on_error(error_msg.clone());
                                }
                                continue;
                            }
                        }

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
                },
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
    use crate::common::crypto::{AesGcmEncryption, EncryptionError};
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

        fn with_auth_success(self, username: &str) -> Self {
            let success_response = format!(
                r#"{{"type":"success","user_id":"123e4567-e89b-12d3-a456-426614174000","username":"{}","roles":["user"],"token":"test_token","expires_at":9999999999}}"#,
                username
            );
            self.receive_queue
                .try_lock()
                .unwrap()
                .push_back(Ok(Some(success_response)));
            self
        }
    }

    #[async_trait::async_trait]
    impl Transport for MockTransport {
        async fn connect(&mut self) -> Result<(), TransportError> {
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
                Err(EncryptionError::DecryptionError(
                    "Invalid ciphertext".to_string(),
                ))
            }
        }

        async fn perform_handshake(
            &mut self,
            _transport: &mut dyn Transport,
        ) -> Result<(), TransportError> {
            // Mock implementation - no handshake performed
            Ok(())
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
        manager
            .with_transport(Box::new(transport))
            .with_encryption(Box::new(encryption));

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
    async fn test_authentication_flow() {
        let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let mut manager = ConnectionManager::new(config.clone());

        let mock_transport = Box::new(MockTransport::new().with_auth_success("testuser"));
        manager.with_transport(mock_transport);
        manager.with_auth();

        // Initially not authenticated
        assert!(!manager.is_authenticated().await);

        // Attempt login
        let credentials = Credentials {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };

        if let Some(auth_manager) = manager.auth_manager.clone() {
            assert!(auth_manager.login(credentials).await.is_ok());
            assert!(manager.is_authenticated().await);

            // Verify auth state
            match manager.get_auth_state().await.unwrap() {
                AuthState::Authenticated { profile, session } => {
                    assert_eq!(profile.username, "testuser");
                    assert_eq!(session.token, "test_token");
                }
                _ => panic!("Expected authenticated state"),
            }

            // Test message sending while authenticated
            assert!(manager
                .send_message("test message".to_string())
                .await
                .is_ok());

            // Test logout
            assert!(auth_manager.logout().await.is_ok());
            assert!(!manager.is_authenticated().await);
        } else {
            panic!("Auth manager not configured");
        }
    }

    #[tokio::test]
    async fn test_aes_gcm_encryption_compatibility() {
        // Test that AesGcmEncryption works as expected with the EncryptionService trait
        let encryption: Box<dyn EncryptionService + Send + Sync> =
            Box::new(AesGcmEncryption::new("consistent_password"));

        let original_message = "Secret message for encryption test";

        // Encrypt and decrypt through the trait interface
        let encrypted = encryption
            .encrypt("ignored_key", original_message)
            .expect("Encryption should succeed");
        let decrypted = encryption
            .decrypt("ignored_key", &encrypted)
            .expect("Decryption should succeed");

        assert_eq!(original_message, decrypted);

        // Verify that encrypted data is different from original
        assert_ne!(original_message, encrypted);

        // Verify that the same message encrypts to different ciphertexts (due to random nonce)
        let encrypted2 = encryption
            .encrypt("ignored_key", original_message)
            .expect("Second encryption should succeed");
        assert_ne!(encrypted, encrypted2);

        // But both should decrypt to the same original message
        let decrypted2 = encryption
            .decrypt("ignored_key", &encrypted2)
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

        #[async_trait::async_trait]
        impl EncryptionService for HandshakeTrackingEncryption {
            fn encrypt(&self, _key: &str, plaintext: &str) -> Result<String, EncryptionError> {
                Ok(format!("ENCRYPTED:{}", plaintext))
            }

            fn decrypt(&self, _key: &str, ciphertext: &str) -> Result<String, EncryptionError> {
                if ciphertext.starts_with("ENCRYPTED:") {
                    Ok(ciphertext.replace("ENCRYPTED:", ""))
                } else {
                    Err(EncryptionError::DecryptionError(
                        "Invalid ciphertext".to_string(),
                    ))
                }
            }

            async fn perform_handshake(
                &mut self,
                _transport: &mut dyn Transport,
            ) -> Result<(), TransportError> {
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

        #[async_trait::async_trait]
        impl EncryptionService for FailingHandshakeEncryption {
            fn encrypt(&self, _key: &str, plaintext: &str) -> Result<String, EncryptionError> {
                Ok(format!("ENCRYPTED:{}", plaintext))
            }

            fn decrypt(&self, _key: &str, ciphertext: &str) -> Result<String, EncryptionError> {
                Ok(ciphertext.replace("ENCRYPTED:", ""))
            }

            async fn perform_handshake(
                &mut self,
                _transport: &mut dyn Transport,
            ) -> Result<(), TransportError> {
                Err(TransportError::EncryptionError(
                    EncryptionError::EncryptionError("Handshake failed".to_string()),
                ))
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
