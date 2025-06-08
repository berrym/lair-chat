use base64::prelude::*;
use color_eyre::eyre::Result;
use futures::{select, FutureExt, SinkExt};
use md5;
use once_cell::sync::Lazy;
use std::{net::SocketAddr, pin::Pin, sync::Mutex};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    time::{sleep, Duration},
};
use tokio_stream::{wrappers::LinesStream, Stream, StreamExt};
use tokio_util::{
    codec::{FramedWrite, LinesCodec},
    sync::CancellationToken,
};
use tui_input::Input;
use x25519_dalek::{EphemeralSecret, PublicKey};
use async_trait::async_trait;

use crate::components::home::get_user_input;
use super::encryption::{encrypt, decrypt, EncryptionError};

/// Trait abstraction for encryption operations
#[async_trait]
pub trait EncryptionService: Send + Sync {
    /// Encrypt plaintext with the given key
    fn encrypt(&self, key: &str, plaintext: &str) -> Result<String, EncryptionError>;
    
    /// Decrypt ciphertext with the given key
    fn decrypt(&self, key: &str, ciphertext: &str) -> Result<String, EncryptionError>;
    
    /// Perform key exchange handshake with remote peer
    async fn perform_handshake(&mut self, transport: &mut dyn Transport) -> Result<(), TransportError>;
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

/// Default implementation of EncryptionService using our existing functions
pub struct DefaultEncryptionService;

#[async_trait]
impl EncryptionService for DefaultEncryptionService {
    fn encrypt(&self, key: &str, plaintext: &str) -> Result<String, EncryptionError> {
        encrypt(key.to_string(), plaintext.to_string())
    }
    
    fn decrypt(&self, key: &str, ciphertext: &str) -> Result<String, EncryptionError> {
        decrypt(key.to_string(), ciphertext.to_string())
    }
    
    async fn perform_handshake(&mut self, _transport: &mut dyn Transport) -> Result<(), TransportError> {
        // Default implementation - no handshake performed
        Ok(())
    }
}

/// Default implementation of ConnectionObserver using existing global functions
pub struct DefaultConnectionObserver;

impl ConnectionObserver for DefaultConnectionObserver {
    fn on_message(&self, message: String) {
        add_text_message(message);
    }
    
    fn on_error(&self, error: String) {
        add_text_message(format!("Error: {}", error));
    }
    
    fn on_status_change(&self, connected: bool) {
        if connected {
            add_text_message("Connected to server.".to_string());
        } else {
            add_text_message("Disconnected from server.".to_string());
        }
    }
}

/// TUI-specific implementation of ConnectionObserver for better UI integration
pub struct TuiObserver;

impl ConnectionObserver for TuiObserver {
    fn on_message(&self, message: String) {
        add_text_message(message);
    }
    
    fn on_error(&self, error: String) {
        add_text_message(format!("ERROR: {}", error));
    }
    
    fn on_status_change(&self, connected: bool) {
        if connected {
            add_text_message("STATUS: Connected to server.".to_string());
        } else {
            add_text_message("STATUS: Disconnected from server.".to_string());
        }
    }
}

/// Configuration for establishing a connection
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub address: std::net::SocketAddr,
    pub timeout_ms: u64,
}

impl ConnectionConfig {
    pub fn new(address: std::net::SocketAddr) -> Self {
        Self {
            address,
            timeout_ms: 5000, // 5 second default timeout
        }
    }
    
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// Represents a chat message with metadata
#[derive(Debug, Clone)]
pub struct Message {
    pub content: String,
    pub timestamp: std::time::SystemTime,
    pub message_type: MessageType,
}

/// Type of message for display purposes
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    UserMessage,
    ReceivedMessage, 
    SystemMessage,
    ErrorMessage,
}

impl Message {
    pub fn user_message(content: String) -> Self {
        Self {
            content,
            timestamp: std::time::SystemTime::now(),
            message_type: MessageType::UserMessage,
        }
    }
    
    pub fn received_message(content: String) -> Self {
        Self {
            content,
            timestamp: std::time::SystemTime::now(),
            message_type: MessageType::ReceivedMessage,
        }
    }
    
    pub fn system_message(content: String) -> Self {
        Self {
            content,
            timestamp: std::time::SystemTime::now(),
            message_type: MessageType::SystemMessage,
        }
    }
    
    pub fn error_message(content: String) -> Self {
        Self {
            content,
            timestamp: std::time::SystemTime::now(),
            message_type: MessageType::ErrorMessage,
        }
    }
    
    /// Format message for display
    pub fn format_for_display(&self) -> String {
        match self.message_type {
            MessageType::UserMessage => format!("You: {}", self.content),
            MessageType::ReceivedMessage => self.content.clone(),
            MessageType::SystemMessage => self.content.clone(),
            MessageType::ErrorMessage => format!("Error: {}", self.content),
        }
    }
}

/// Container for storing messages with better organization
#[derive(Debug, Clone)]
pub struct MessageStore {
    pub messages: Vec<Message>,
    pub outgoing: Vec<String>, // Keep for backward compatibility
}

impl MessageStore {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            outgoing: Vec::new(),
        }
    }
    
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }
    
    pub fn get_display_messages(&self) -> Vec<String> {
        self.messages.iter()
            .map(|msg| msg.format_for_display())
            .collect()
    }
    
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }
    
    pub fn clear_outgoing(&mut self) {
        self.outgoing.clear();
    }
}

/// Create a connection configuration from an address string
pub fn create_connection_config(address: std::net::SocketAddr) -> ConnectionConfig {
    ConnectionConfig::new(address)
}

/// Helper function to create a formatted message for display
pub fn create_user_message(content: String) -> Message {
    Message::user_message(content)
}

/// Helper function to create system messages
pub fn create_system_message(content: String) -> Message {
    Message::system_message(content)
}

/// Helper function to create error messages
pub fn create_error_message(content: String) -> Message {
    Message::error_message(content)
}

/// Create a default encryption service instance
pub fn create_encryption_service() -> DefaultEncryptionService {
    DefaultEncryptionService
}

/// Create a default connection observer instance
pub fn create_connection_observer() -> DefaultConnectionObserver {
    DefaultConnectionObserver
}

/// Factory function to create a boxed encryption service trait object
pub fn create_boxed_encryption_service() -> Box<dyn EncryptionService> {
    Box::new(DefaultEncryptionService)
}

/// Factory function to create a boxed connection observer trait object
pub fn create_boxed_connection_observer() -> Box<dyn ConnectionObserver> {
    Box::new(DefaultConnectionObserver)
}

/// Create a TUI observer instance
pub fn create_tui_observer() -> TuiObserver {
    TuiObserver
}

/// Factory function to create a boxed TUI observer trait object
pub fn create_boxed_tui_observer() -> Box<dyn ConnectionObserver> {
    Box::new(TuiObserver)
}

/// Perform key exchange with the server and return the shared AES key
async fn perform_key_exchange(
    sink: &mut ClientSink,
    stream: &mut ClientStream,
) -> Result<String, TransportError> {
    // create private/public keys
    let client_secret_key = EphemeralSecret::random();
    let client_public_key = PublicKey::from(&client_secret_key);
    
    // start handshake by sending public key to server
    sink.tx
        .send(BASE64_STANDARD.encode(client_public_key))
        .await
        .map_err(|e| TransportError::KeyExchangeError(format!("Failed to send public key: {}", e)))?;
    
    // receive server public key
    let server_public_key_string = match stream.rx.next().await {
        Some(key_string) => key_string,
        None => {
            return Err(TransportError::KeyExchangeError(
                "Failed to get server public key".to_string(),
            ));
        }
    };
    
    // keep converting until key is a 32 byte u8 array
    let server_public_key_vec = match server_public_key_string {
        Ok(key_vec) => BASE64_STANDARD.decode(key_vec).map_err(|e| {
            TransportError::KeyExchangeError(format!(
                "Failed to decode server public key: {}",
                e
            ))
        })?,
        Err(_) => {
            return Err(TransportError::KeyExchangeError(
                "Failed to receive server public key".to_string(),
            ));
        }
    };
    
    let server_public_key_slice: &[u8] = server_public_key_vec.as_slice().try_into().map_err(|_| {
        TransportError::KeyExchangeError(
            "Failed to convert server public key to byte slice".to_string(),
        )
    })?;
    
    let server_public_key_array: [u8; 32] = server_public_key_slice.try_into().map_err(|_| {
        TransportError::KeyExchangeError(
            "Failed to convert public key to 32-byte array".to_string(),
        )
    })?;
    
    // create shared keys
    let shared_secret = client_secret_key.diffie_hellman(&PublicKey::from(server_public_key_array));
    let shared_aes256_key = format!("{:x}", md5::compute(BASE64_STANDARD.encode(shared_secret)));
    
    Ok(shared_aes256_key)
}

/// Process and send any pending outgoing messages
async fn process_outgoing_messages(
    sink: &mut ClientSink,
    shared_key: &str,
) -> Result<(), TransportError> {
    if !MESSAGES.lock().unwrap().outgoing.is_empty() {
        let outgoing_messages: Vec<String> = MESSAGES
            .lock()
            .unwrap()
            .outgoing
            .clone();
            
        for original_message in outgoing_messages {
            match encrypt(shared_key.to_string(), original_message.clone()) {
                Ok(encrypted_message) => {
                    add_text_message(format!("You: {}", original_message));
                    let _ = sink.tx.send(encrypted_message).await;
                }
                Err(e) => {
                    add_text_message(format!("Failed to encrypt message: {}", e));
                }
            }
        }
        MESSAGES.lock().unwrap().outgoing.clear();
    }
    Ok(())
}

/// Handle incoming message by decrypting and displaying it
fn handle_incoming_message(message: String, shared_key: &str) {
    match decrypt(shared_key.to_string(), message) {
        Ok(decrypted_message) => {
            add_text_message(decrypted_message);
        }
        Err(e) => {
            add_text_message(format!("Failed to decrypt message: {}", e));
        }
    }
}

/// Handle user input by encrypting and sending the message
async fn handle_user_input(
    sink: &mut ClientSink,
    message: String,
    shared_key: &str,
) -> Result<(), TransportError> {
    match encrypt(shared_key.to_string(), message.clone()) {
        Ok(encrypted_message) => {
            let _ = sink.tx.send(encrypted_message).await;
            add_text_message(format!("You: {}", message));
        }
        Err(e) => {
            add_text_message(format!("Failed to encrypt message: {}", e));
        }
    }
    Ok(())
}

/// Handle connection closure and cleanup
fn handle_connection_closed() {
    CANCEL_TOKEN.lock().unwrap().token.cancel();
    CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::DISCONNECTED;
    CANCEL_TOKEN.lock().unwrap().token = CancellationToken::new();
}

/// Transport-specific error types
#[derive(Debug)]
pub enum TransportError {
    ConnectionError(std::io::Error),
    EncryptionError(EncryptionError),
    KeyExchangeError(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::ConnectionError(e) => write!(f, "Connection error: {}", e),
            TransportError::EncryptionError(e) => write!(f, "Encryption error: {}", e),
            TransportError::KeyExchangeError(msg) => write!(f, "Key exchange error: {}", msg),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<EncryptionError> for TransportError {
    fn from(err: EncryptionError) -> Self {
        TransportError::EncryptionError(err)
    }
}

/// Shorthand for a pinned boxed stream
pub type BoxedStream<Item> = Pin<Box<dyn Stream<Item = Item> + Send>>;
/// Shorthand for a lines framed BoxedStream type we will use
pub type ClientTcpStream = BoxedStream<Result<String, std::io::Error>>;

#[derive(PartialEq, Debug, Clone)]
pub enum ConnectionStatus {
    CONNECTED,
    DISCONNECTED,
}

/// Client connection status
pub struct ClientStatus {
    pub status: ConnectionStatus,
}

impl ClientStatus {
    pub fn new() -> Self {
        let status = ConnectionStatus::DISCONNECTED;
        Self { status }
    }
}

/// Lazy Mutex wrapped global client connection status
pub static CLIENT_STATUS: Lazy<Mutex<ClientStatus>> = Lazy::new(|| {
    let m = ClientStatus::new();
    Mutex::new(m)
});

/// Task cancellation token
pub struct CancelClient {
    pub token: CancellationToken,
}

impl CancelClient {
    pub fn new() -> Self {
        let token = CancellationToken::new();
        Self { token }
    }
}

/// Lazy Mutex wrapped global cancellation token
pub static CANCEL_TOKEN: Lazy<Mutex<CancelClient>> = Lazy::new(|| {
    let m = CancelClient::new();
    Mutex::new(m)
});

/// Wrapped read half of a TcpStream
pub struct ClientStream {
    pub rx: ClientTcpStream,
}

impl ClientStream {
    pub fn new(reader: OwnedReadHalf) -> Self {
        let rx = Box::pin(LinesStream::new(BufReader::new(reader).lines()));
        Self { rx }
    }
}

/// Wrapped write half of a TcpStream
pub struct ClientSink {
    pub tx: FramedWrite<OwnedWriteHalf, LinesCodec>,
}

impl ClientSink {
    pub fn new(writer: OwnedWriteHalf) -> Self {
        let tx = FramedWrite::new(writer, LinesCodec::new());
        Self { tx }
    }
}

/// Messages buffers
pub struct Messages {
    pub outgoing: Vec<String>,
    pub text: Vec<String>,
}

impl Messages {
    pub fn new() -> Self {
        let outgoing = Vec::new();
        let text = Vec::new();
        Self { outgoing, text }
    }
}

/// Lazy Mutex wrapped global message buffers
pub static MESSAGES: Lazy<Mutex<Messages>> = Lazy::new(|| {
    let m = Messages::new();
    Mutex::new(m)
});

/// Add a message to displayed in the main window
pub fn add_text_message(s: String) {
    MESSAGES.lock().unwrap().text.push(s);
}

/// Add a message to the outgoing buffer
pub fn add_outgoing_message(s: String) {
    MESSAGES.lock().unwrap().outgoing.insert(0, s);
}

pub fn split_tcp_stream(stream: TcpStream) -> Result<(ClientStream, ClientSink)> {
    let (reader, writer) = stream.into_split();
    Ok((ClientStream::new(reader), ClientSink::new(writer)))
}

pub async fn connect_client(input: Input, address: SocketAddr) {
    add_text_message(format!("Connecting to {}", address.clone()));
    let stream = TcpStream::connect(address).await;
    if stream.is_ok() {
        add_text_message("Connected to server.".to_owned());
        add_text_message("".to_owned());
        tokio::task::spawn_blocking(move || {
            CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::CONNECTED;
            let (reader, writer) = split_tcp_stream(stream.unwrap()).unwrap();
            client_io_select_loop(input, reader, writer);
        });
    } else {
        add_text_message("Failed to connect to server.".to_owned());
        add_text_message("".to_owned());
    }
}

pub fn client_io_select_loop(
    input: Input,
    reader: ClientStream,
    writer: ClientSink) {
    // create a sink and stream for transport
    let mut stream = reader;
    let mut sink = writer;
    let cancel_token = CANCEL_TOKEN.lock().unwrap().token.clone();
    tokio::spawn(async move {
        // perform key exchange with server
        let shared_aes256_key = match perform_key_exchange(&mut sink, &mut stream).await {
            Ok(key) => key,
            Err(e) => {
                add_text_message(format!("Key exchange failed: {}", e));
                return;
            }
        };
        // main client loop
        loop {
            // process any outgoing messages
            let _ = process_outgoing_messages(&mut sink, &shared_aes256_key).await;
            
            // select on futures
            select! {
                _ = cancel_token.cancelled().fuse() => {
                    let mut writer = sink.tx.into_inner();
                    let _ = writer.shutdown().await;
                    return;
                },
                message = stream.rx.next().fuse() => match message {
                    Some(Ok(message)) => {
                        handle_incoming_message(message, &shared_aes256_key);
                        continue;
                    },
                    None => {
                        add_text_message("The Lair has CLOSED.".to_string());
                        handle_connection_closed();
                        return;
                    },
                    Some(Err(e)) => {
                        add_text_message(format!("Closed connection with error: {e}"));
                        handle_connection_closed();
                        return;
                    }
                },
                message = get_user_input(input.clone()).fuse() => match message {
                    Some(message) => {
                        let _ = handle_user_input(&mut sink, message, &shared_aes256_key).await;
                    },
                    None => {
                        sleep(Duration::from_millis(250)).await;
                    },
                }
            }
        }
    });
}

pub async fn disconnect_client() {
    if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::CONNECTED {
        CANCEL_TOKEN.lock().unwrap().token.cancel();
        MESSAGES.lock().unwrap().text.clear();
        add_text_message("Disconnected from server.".to_string());
        CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::DISCONNECTED;
        sleep(Duration::from_secs(2)).await;
        MESSAGES.lock().unwrap().text.clear();
    } else {
        add_text_message("Not connected to a server.".to_string());
    }
    CANCEL_TOKEN.lock().unwrap().token = CancellationToken::new();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::EncryptionError;

    #[test]
    fn test_transport_error_display() {
        let conn_error = TransportError::ConnectionError(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "test"));
        let enc_error = TransportError::EncryptionError(EncryptionError::EncryptionError("test".to_string()));
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

    #[test]
    fn test_handle_incoming_message() {
        // Test successful decryption
        let key = "test_key_32_bytes_exactly_here!!";
        let original_message = "Hello, World!";
        
        // First encrypt a message
        let encrypted = crate::encryption::encrypt(key.to_string(), original_message.to_string()).unwrap();
        
        // Then test decryption through handle_incoming_message
        // Note: This function adds to global MESSAGES, so we can't easily test the output
        // without affecting global state, but we can verify it doesn't panic
        handle_incoming_message(encrypted, key);
        
        // Test with invalid message - should not panic
        handle_incoming_message("invalid_base64!@#".to_string(), key);
    }

    #[test]
    fn test_handle_connection_closed() {
        // Test that connection cleanup doesn't panic
        handle_connection_closed();
        
        // Verify the status was set to disconnected
        assert_eq!(CLIENT_STATUS.lock().unwrap().status, ConnectionStatus::DISCONNECTED);
    }

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
    fn test_encryption_service_trait() {
        let service = DefaultEncryptionService;
        let key = "test_key_32_bytes_exactly_here!!";
        let message = "Hello, World!";
        
        let encrypted = service.encrypt(key, message).expect("Encryption should succeed");
        let decrypted = service.decrypt(key, &encrypted).expect("Decryption should succeed");
        
        assert_eq!(message, decrypted);
    }

    #[test]
    fn test_connection_observer_trait() {
        let observer = DefaultConnectionObserver;
        
        // Test that these don't panic - they modify global state
        observer.on_message("Test message".to_string());
        observer.on_error("Test error".to_string());
        observer.on_status_change(true);
        observer.on_status_change(false);
    }

    #[test]
    fn test_tui_observer() {
        let observer = TuiObserver;
        
        // Test that these don't panic - they modify global state
        observer.on_message("Test TUI message".to_string());
        observer.on_error("Test TUI error".to_string());
        observer.on_status_change(true);
        observer.on_status_change(false);
    }

    #[test]
    fn test_tui_observer_helper_functions() {
        // Test factory functions
        let observer = create_tui_observer();
        observer.on_message("Factory test message".to_string());

        let boxed_observer = create_boxed_tui_observer();
        boxed_observer.on_message("Boxed factory test message".to_string());
    }

    #[test]
    fn test_observer_message_formatting() {
        // Clear messages first to ensure clean test state
        MESSAGES.lock().unwrap().text.clear();
        
        let tui_observer = TuiObserver;
        let default_observer = DefaultConnectionObserver;
        
        // Test error message formatting differences
        tui_observer.on_error("Test error".to_string());
        default_observer.on_error("Test error".to_string());
        
        let messages = MESSAGES.lock().unwrap();
        let tui_error = &messages.text[messages.text.len() - 2]; // Second to last
        let default_error = &messages.text[messages.text.len() - 1]; // Last
        
        assert!(tui_error.contains("ERROR: Test error"));
        assert!(default_error.contains("Error: Test error"));
        assert!(!default_error.contains("ERROR:"));
    }

    #[test]
    fn test_status_change_formatting() {
        // Clear messages first to ensure clean test state
        MESSAGES.lock().unwrap().text.clear();
        
        let tui_observer = TuiObserver;
        let default_observer = DefaultConnectionObserver;
        
        // Test connected status formatting
        tui_observer.on_status_change(true);
        default_observer.on_status_change(true);
        
        let messages = MESSAGES.lock().unwrap();
        let tui_connected = &messages.text[messages.text.len() - 2]; // Second to last
        let default_connected = &messages.text[messages.text.len() - 1]; // Last
        
        assert!(tui_connected.contains("STATUS: Connected to server."));
        assert!(default_connected.contains("Connected to server."));
        assert!(!default_connected.contains("STATUS:"));
        
        // Clear for disconnected test
        drop(messages);
        MESSAGES.lock().unwrap().text.clear();
        
        // Test disconnected status formatting
        tui_observer.on_status_change(false);
        default_observer.on_status_change(false);
        
        let messages = MESSAGES.lock().unwrap();
        let tui_disconnected = &messages.text[messages.text.len() - 2]; // Second to last
        let default_disconnected = &messages.text[messages.text.len() - 1]; // Last
        
        assert!(tui_disconnected.contains("STATUS: Disconnected from server."));
        assert!(default_disconnected.contains("Disconnected from server."));
        assert!(!default_disconnected.contains("STATUS:"));
    }
}



