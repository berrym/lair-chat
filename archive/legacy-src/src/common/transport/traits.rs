//! Modern transport layer for Lair-Chat
//! Contains only the core types, traits, and enums needed by the modern architecture.
//! All deprecated global state and legacy functions have been removed.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::SystemTime;

use crate::common::crypto::EncryptionError;

/// Trait abstraction for encryption operations
#[async_trait]
pub trait EncryptionService: Send + Sync {
    /// Encrypt plaintext with the given key
    fn encrypt(&self, key: &str, plaintext: &str) -> Result<String, EncryptionError>;

    /// Decrypt ciphertext with the given key
    fn decrypt(&self, key: &str, ciphertext: &str) -> Result<String, EncryptionError>;

    /// Perform key exchange handshake with remote peer
    async fn perform_handshake(
        &mut self,
        transport: &mut dyn Transport,
    ) -> Result<(), TransportError>;
}

/// Trait abstraction for network transport operations
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    /// Establish a connection to the remote endpoint
    async fn connect(&mut self) -> Result<(), TransportError>;

    /// Send data over the transport
    async fn send(&mut self, data: &str) -> Result<(), TransportError>;

    /// Receive data from the transport
    async fn receive(&mut self) -> Result<Option<String>, TransportError>;

    /// Close the transport connection
    async fn close(&mut self) -> Result<(), TransportError>;
}

/// Trait abstraction for UI notifications and message handling
pub trait ConnectionObserver: Send + Sync {
    /// Called when a message should be displayed to the user
    fn on_message(&self, message: String);

    /// Called when an error occurs that should be shown to the user
    fn on_error(&self, error: String);

    /// Called when connection status changes
    fn on_status_change(&self, connected: bool);
}

/// Transport layer error types
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("Connection error: {0}")]
    ConnectionError(#[from] std::io::Error),

    #[error("Encryption error: {0}")]
    EncryptionError(#[from] EncryptionError),

    #[error("Key exchange error: {0}")]
    KeyExchangeError(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Invalid configuration: {0}")]
    ConfigurationError(String),
}

/// Connection status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    CONNECTED,
    DISCONNECTED,
}

/// Connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub address: SocketAddr,
    pub timeout_ms: u64,
}

impl ConnectionConfig {
    /// Create a new connection configuration
    pub fn new(address: SocketAddr) -> Self {
        Self {
            address,
            timeout_ms: 5000,
        }
    }

    /// Set the timeout in milliseconds
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// Message types for different kinds of chat messages
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    UserMessage,
    ReceivedMessage,
    SystemMessage,
    ErrorMessage,
}

/// A message in the chat system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: SystemTime,
}

impl Message {
    /// Create a new user message
    pub fn user_message(content: String) -> Self {
        Self {
            content,
            message_type: MessageType::UserMessage,
            timestamp: SystemTime::now(),
        }
    }

    /// Create a new received message
    pub fn received_message(content: String) -> Self {
        Self {
            content,
            message_type: MessageType::ReceivedMessage,
            timestamp: SystemTime::now(),
        }
    }

    /// Create a new system message
    pub fn system_message(content: String) -> Self {
        Self {
            content,
            message_type: MessageType::SystemMessage,
            timestamp: SystemTime::now(),
        }
    }

    /// Create a new error message
    pub fn error_message(content: String) -> Self {
        Self {
            content,
            message_type: MessageType::ErrorMessage,
            timestamp: SystemTime::now(),
        }
    }

    /// Format the message for display in the UI
    pub fn format_for_display(&self) -> String {
        match self.message_type {
            MessageType::UserMessage => format!("You: {}", self.content),
            MessageType::ReceivedMessage => self.content.clone(),
            MessageType::SystemMessage => self.content.clone(),
            MessageType::ErrorMessage => format!("Error: {}", self.content),
        }
    }
}

/// Store for managing chat messages
#[derive(Debug, Clone)]
pub struct MessageStore {
    pub messages: Vec<Message>,
}

impl MessageStore {
    /// Create a new message store
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Add a message to the store
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Get all messages formatted for display
    pub fn get_display_messages(&self) -> Vec<String> {
        self.messages
            .iter()
            .map(|msg| msg.format_for_display())
            .collect()
    }

    /// Clear all messages
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    /// Get the number of messages
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

impl Default for MessageStore {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions for creating common types
pub fn create_connection_config(address: SocketAddr) -> ConnectionConfig {
    ConnectionConfig::new(address)
}

pub fn create_user_message(content: String) -> Message {
    Message::user_message(content)
}

pub fn create_system_message(content: String) -> Message {
    Message::system_message(content)
}

pub fn create_error_message(content: String) -> Message {
    Message::error_message(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_config() {
        let addr = "127.0.0.1:8080".parse().unwrap();
        let config = ConnectionConfig::new(addr);

        assert_eq!(config.address, addr);
        assert_eq!(config.timeout_ms, 5000);

        let config_with_timeout = config.with_timeout(10000);
        assert_eq!(config_with_timeout.timeout_ms, 10000);
    }

    #[test]
    fn test_message_creation() {
        let user_msg = Message::user_message("Hello".to_string());
        let received_msg = Message::received_message("Hi there".to_string());
        let system_msg = Message::system_message("Connected".to_string());
        let error_msg = Message::error_message("Failed".to_string());

        assert_eq!(user_msg.message_type, MessageType::UserMessage);
        assert_eq!(received_msg.message_type, MessageType::ReceivedMessage);
        assert_eq!(system_msg.message_type, MessageType::SystemMessage);
        assert_eq!(error_msg.message_type, MessageType::ErrorMessage);

        assert_eq!(user_msg.format_for_display(), "You: Hello");
        assert_eq!(received_msg.format_for_display(), "Hi there");
        assert_eq!(system_msg.format_for_display(), "Connected");
        assert_eq!(error_msg.format_for_display(), "Error: Failed");
    }

    #[test]
    fn test_message_store() {
        let mut store = MessageStore::new();
        assert!(store.messages.is_empty());

        let msg1 = Message::user_message("Hello".to_string());
        let msg2 = Message::system_message("Connected".to_string());

        store.add_message(msg1);
        store.add_message(msg2);

        assert_eq!(store.messages.len(), 2);

        let display_messages = store.get_display_messages();
        assert_eq!(display_messages[0], "You: Hello");
        assert_eq!(display_messages[1], "Connected");

        store.clear_messages();
        assert!(store.messages.is_empty());
    }

    #[test]
    fn test_helper_functions() {
        let addr = "127.0.0.1:8080".parse().unwrap();
        let config = create_connection_config(addr);
        assert_eq!(config.address, addr);

        let user_msg = create_user_message("Test".to_string());
        assert_eq!(user_msg.message_type, MessageType::UserMessage);

        let system_msg = create_system_message("System".to_string());
        assert_eq!(system_msg.message_type, MessageType::SystemMessage);

        let error_msg = create_error_message("Error".to_string());
        assert_eq!(error_msg.message_type, MessageType::ErrorMessage);
    }

    #[test]
    fn test_transport_error_display() {
        let conn_error = TransportError::ConnectionError(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "test",
        ));
        let enc_error =
            TransportError::EncryptionError(EncryptionError::EncryptionError("test".to_string()));
        let key_error = TransportError::KeyExchangeError("test key exchange error".to_string());

        assert!(conn_error.to_string().contains("Connection error"));
        assert!(enc_error.to_string().contains("Encryption error"));
        assert!(key_error.to_string().contains("Key exchange error"));
    }

    #[test]
    fn test_error_conversion() {
        let encryption_error = EncryptionError::EncryptionError("test".to_string());
        let transport_error: TransportError = encryption_error.into();

        match transport_error {
            TransportError::EncryptionError(_) => {
                // This is expected
            }
            _ => panic!("Expected EncryptionError variant"),
        }
    }
}
