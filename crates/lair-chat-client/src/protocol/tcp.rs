//! TCP client implementation.
//!
//! Handles connection, message framing, and communication with the server.

use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use super::messages::{ClientMessage, ServerMessage};
use crate::crypto::{parse_public_key, Cipher, KeyPair, NONCE_SIZE};

/// Maximum message size (1 MB).
const MAX_MESSAGE_SIZE: u32 = 1_048_576;

/// Protocol version.
pub const PROTOCOL_VERSION: &str = "1.0";

/// TCP client errors.
#[derive(Debug, thiserror::Error)]
pub enum TcpError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Message too large: {size} bytes (max {MAX_MESSAGE_SIZE})")]
    MessageTooLarge { size: u32 },

    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Timeout")]
    Timeout,

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Not connected")]
    NotConnected,

    #[error("Encrypted message too small")]
    EncryptedMessageTooSmall,

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Key exchange failed: {0}")]
    KeyExchangeFailed(String),
}

/// Minimum size for encrypted frame: nonce (12) + tag (16) = 28 bytes
const MIN_ENCRYPTED_SIZE: usize = NONCE_SIZE + 16;

/// TCP client for communicating with the server.
pub struct TcpClient {
    stream: Option<TcpStream>,
    server_addr: SocketAddr,
    connect_timeout: Duration,
    read_timeout: Duration,
    /// Cipher for encrypted communication.
    cipher: Option<Cipher>,
    /// Whether encryption is enabled.
    encryption_enabled: bool,
}

#[allow(dead_code)]
impl TcpClient {
    /// Create a new TCP client.
    pub fn new(server_addr: SocketAddr) -> Self {
        Self {
            stream: None,
            server_addr,
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(60),
            cipher: None,
            encryption_enabled: false,
        }
    }

    /// Set connection timeout.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set read timeout.
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// Connect to the server.
    pub async fn connect(&mut self) -> Result<(), TcpError> {
        info!("Connecting to server at {}", self.server_addr);

        let stream = timeout(self.connect_timeout, TcpStream::connect(self.server_addr))
            .await
            .map_err(|_| TcpError::Timeout)?
            .map_err(|e| TcpError::ConnectionFailed(e.to_string()))?;

        stream.set_nodelay(true)?;
        self.stream = Some(stream);

        info!("Connected to server");
        Ok(())
    }

    /// Disconnect from the server.
    pub async fn disconnect(&mut self) {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.shutdown().await;
        }
        info!("Disconnected from server");
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Send a message to the server.
    pub async fn send(&mut self, message: &ClientMessage) -> Result<(), TcpError> {
        let stream = self.stream.as_mut().ok_or(TcpError::NotConnected)?;

        let json = serde_json::to_string(message)?;
        debug!("Sending: {}", json);

        if let Some(ref cipher) = self.cipher {
            // Encrypted write
            let plaintext = json.as_bytes();
            let (nonce, ciphertext) = cipher
                .encrypt(plaintext)
                .map_err(|e| TcpError::EncryptionFailed(e.to_string()))?;

            let frame_size = NONCE_SIZE + ciphertext.len();
            if frame_size > MAX_MESSAGE_SIZE as usize {
                return Err(TcpError::MessageTooLarge {
                    size: frame_size as u32,
                });
            }

            // Write length prefix
            let length = frame_size as u32;
            stream.write_all(&length.to_be_bytes()).await?;

            // Write nonce
            stream.write_all(&nonce).await?;

            // Write ciphertext
            stream.write_all(&ciphertext).await?;
            stream.flush().await?;
        } else {
            // Unencrypted write
            let payload = json.as_bytes();

            if payload.len() > MAX_MESSAGE_SIZE as usize {
                return Err(TcpError::MessageTooLarge {
                    size: payload.len() as u32,
                });
            }

            // Write length prefix (big-endian u32)
            let length = payload.len() as u32;
            stream.write_all(&length.to_be_bytes()).await?;

            // Write payload
            stream.write_all(payload).await?;
            stream.flush().await?;
        }

        Ok(())
    }

    /// Receive a message from the server.
    pub async fn recv(&mut self) -> Result<ServerMessage, TcpError> {
        let stream = self.stream.as_mut().ok_or(TcpError::NotConnected)?;

        // Read length prefix
        let mut length_bytes = [0u8; 4];
        match timeout(self.read_timeout, stream.read_exact(&mut length_bytes)).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(TcpError::ConnectionClosed);
            }
            Ok(Err(e)) => return Err(TcpError::Io(e)),
            Err(_) => return Err(TcpError::Timeout),
        }

        let length = u32::from_be_bytes(length_bytes) as usize;

        if length > MAX_MESSAGE_SIZE as usize {
            return Err(TcpError::MessageTooLarge {
                size: length as u32,
            });
        }

        if let Some(ref cipher) = self.cipher {
            // Encrypted read
            if length < MIN_ENCRYPTED_SIZE {
                return Err(TcpError::EncryptedMessageTooSmall);
            }

            // Read nonce
            let mut nonce = [0u8; NONCE_SIZE];
            stream.read_exact(&mut nonce).await?;

            // Read ciphertext
            let ciphertext_len = length - NONCE_SIZE;
            let mut ciphertext = vec![0u8; ciphertext_len];
            stream.read_exact(&mut ciphertext).await?;

            // Decrypt
            let plaintext = cipher
                .decrypt(&nonce, &ciphertext)
                .map_err(|e| TcpError::DecryptionFailed(e.to_string()))?;

            let json = String::from_utf8_lossy(&plaintext);
            debug!("Received: {}", json);

            let message: ServerMessage = serde_json::from_slice(&plaintext)?;
            Ok(message)
        } else {
            // Unencrypted read
            let mut payload = vec![0u8; length];
            stream.read_exact(&mut payload).await?;

            let json = String::from_utf8_lossy(&payload);
            debug!("Received: {}", json);

            let message: ServerMessage = serde_json::from_slice(&payload)?;
            Ok(message)
        }
    }

    /// Perform the initial handshake with the server.
    pub async fn handshake(&mut self) -> Result<(), TcpError> {
        self.handshake_with_encryption(true).await
    }

    /// Perform the initial handshake, optionally requesting encryption.
    pub async fn handshake_with_encryption(&mut self, enable_encryption: bool) -> Result<(), TcpError> {
        // Wait for ServerHello
        let server_hello = self.recv().await?;

        let (server_supports_encryption, encryption_required) = match server_hello {
            ServerMessage::ServerHello {
                version,
                server_name,
                features,
                encryption_required,
            } => {
                info!(
                    "Server: {} v{}, features: {:?}, encryption_required: {}",
                    server_name, version, features, encryption_required
                );

                if version != PROTOCOL_VERSION {
                    warn!(
                        "Server version {} differs from client version {}",
                        version, PROTOCOL_VERSION
                    );
                }

                let supports_encryption = features.iter().any(|f| f == "encryption");
                (supports_encryption, encryption_required)
            }
            ServerMessage::Error { code, message, .. } => {
                return Err(TcpError::Protocol(format!("{}: {}", code, message)));
            }
            other => {
                return Err(TcpError::Protocol(format!(
                    "Expected ServerHello, got {:?}",
                    other
                )));
            }
        };

        // Determine if we should use encryption
        let use_encryption = enable_encryption && server_supports_encryption;

        if encryption_required && !use_encryption {
            return Err(TcpError::Protocol(
                "Server requires encryption but client cannot provide it".to_string(),
            ));
        }

        // Send ClientHello with encryption feature if we want encryption
        if use_encryption {
            self.send(&ClientMessage::client_hello_with_encryption()).await?;

            // Perform key exchange
            self.perform_key_exchange().await?;
        } else {
            self.send(&ClientMessage::client_hello()).await?;
        }

        Ok(())
    }

    /// Perform X25519 key exchange with the server.
    async fn perform_key_exchange(&mut self) -> Result<(), TcpError> {
        // Generate client keypair
        let keypair = KeyPair::generate();
        let client_public = keypair.public_key_base64();

        // Send our public key
        self.send(&ClientMessage::key_exchange(client_public)).await?;

        // Wait for server's public key
        let response = self.recv().await?;

        match response {
            ServerMessage::KeyExchangeResponse { public_key } => {
                // Parse server's public key
                let server_public = parse_public_key(&public_key).map_err(|e| {
                    TcpError::KeyExchangeFailed(format!("Invalid server public key: {}", e))
                })?;

                // Derive shared secret
                let shared_secret = keypair.diffie_hellman(server_public);

                // Create cipher
                self.cipher = Some(Cipher::new(&shared_secret));
                self.encryption_enabled = true;

                info!("Encryption enabled");
                Ok(())
            }
            ServerMessage::Error { code, message, .. } => {
                Err(TcpError::KeyExchangeFailed(format!("{}: {}", code, message)))
            }
            other => Err(TcpError::KeyExchangeFailed(format!(
                "Expected KeyExchangeResponse, got {:?}",
                other
            ))),
        }
    }
}

/// Connection handle for managing the TCP connection in a background task.
pub struct Connection {
    /// Channel for sending messages to the server.
    pub tx: mpsc::Sender<ClientMessage>,
    /// Channel for receiving messages from the server.
    pub rx: mpsc::Receiver<ServerMessage>,
    /// Shutdown signal.
    shutdown_tx: mpsc::Sender<()>,
    /// Whether encryption is enabled.
    #[allow(dead_code)]
    encryption_enabled: bool,
}

impl Connection {
    /// Create a new connection to the server.
    pub async fn connect(server_addr: SocketAddr) -> Result<Self, TcpError> {
        Self::connect_with_encryption(server_addr, true).await
    }

    /// Create a new connection with optional encryption.
    pub async fn connect_with_encryption(
        server_addr: SocketAddr,
        enable_encryption: bool,
    ) -> Result<Self, TcpError> {
        let mut client = TcpClient::new(server_addr);
        client.connect().await?;
        client.handshake_with_encryption(enable_encryption).await?;

        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<ClientMessage>(32);
        let (incoming_tx, incoming_rx) = mpsc::channel::<ServerMessage>(32);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Take cipher from client (if encryption enabled)
        let cipher: Arc<RwLock<Option<Arc<Cipher>>>> = Arc::new(RwLock::new(
            client.cipher.take().map(Arc::new),
        ));
        let encryption_enabled = client.encryption_enabled;

        // Split the stream for concurrent read/write
        let stream = client.stream.take().unwrap();
        let (mut reader, mut writer) = stream.into_split();

        // Spawn writer task
        let writer_cipher = cipher.clone();
        let writer_shutdown = shutdown_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = outgoing_rx.recv() => {
                        match msg {
                            Some(message) => {
                                let json = match serde_json::to_string(&message) {
                                    Ok(j) => j,
                                    Err(e) => {
                                        error!("Failed to serialize message: {}", e);
                                        continue;
                                    }
                                };

                                debug!("Sent: {}", json);

                                // Clone cipher Arc before releasing lock
                                let cipher_opt = {
                                    let guard = writer_cipher.read().unwrap();
                                    guard.as_ref().cloned()
                                };

                                let result = match cipher_opt {
                                    Some(cipher) => {
                                        // Encrypted write
                                        let plaintext = json.as_bytes();
                                        match cipher.encrypt(plaintext) {
                                            Ok((nonce, ciphertext)) => {
                                                let frame_size = NONCE_SIZE + ciphertext.len();
                                                let length = frame_size as u32;

                                                let mut write_ok = true;
                                                if writer.write_all(&length.to_be_bytes()).await.is_err() {
                                                    write_ok = false;
                                                }
                                                if write_ok && writer.write_all(&nonce).await.is_err() {
                                                    write_ok = false;
                                                }
                                                if write_ok && writer.write_all(&ciphertext).await.is_err() {
                                                    write_ok = false;
                                                }
                                                if write_ok && writer.flush().await.is_err() {
                                                    write_ok = false;
                                                }
                                                if write_ok { Ok(()) } else { Err("write failed") }
                                            }
                                            Err(_) => Err("encryption failed"),
                                        }
                                    }
                                    None => {
                                        // Unencrypted write
                                        let payload = json.as_bytes();
                                        let length = payload.len() as u32;

                                        let mut write_ok = true;
                                        if writer.write_all(&length.to_be_bytes()).await.is_err() {
                                            write_ok = false;
                                        }
                                        if write_ok && writer.write_all(payload).await.is_err() {
                                            write_ok = false;
                                        }
                                        if write_ok && writer.flush().await.is_err() {
                                            write_ok = false;
                                        }
                                        if write_ok { Ok(()) } else { Err("write failed") }
                                    }
                                };

                                if result.is_err() {
                                    error!("Failed to write message");
                                    break;
                                }
                            }
                            None => {
                                debug!("Outgoing channel closed");
                                break;
                            }
                        }
                    }
                    _ = writer_shutdown.closed() => {
                        debug!("Writer shutdown signal received");
                        break;
                    }
                }
            }
        });

        // Spawn reader task
        let reader_cipher = cipher;
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = read_message_with_cipher(&mut reader, &reader_cipher) => {
                        match result {
                            Ok(message) => {
                                if incoming_tx.send(message).await.is_err() {
                                    debug!("Incoming channel closed");
                                    break;
                                }
                            }
                            Err(TcpError::ConnectionClosed) => {
                                info!("Server closed connection");
                                break;
                            }
                            Err(e) => {
                                error!("Read error: {}", e);
                                break;
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Reader shutdown signal received");
                        break;
                    }
                }
            }
        });

        Ok(Self {
            tx: outgoing_tx,
            rx: incoming_rx,
            shutdown_tx,
            encryption_enabled,
        })
    }

    /// Send a message to the server.
    pub async fn send(&self, message: ClientMessage) -> Result<(), TcpError> {
        self.tx
            .send(message)
            .await
            .map_err(|_| TcpError::ConnectionClosed)
    }

    /// Receive the next message from the server.
    #[allow(dead_code)]
    pub async fn recv(&mut self) -> Option<ServerMessage> {
        self.rx.recv().await
    }

    /// Shutdown the connection.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(()).await;
    }
}

/// Read a single message from the stream (unencrypted).
#[allow(dead_code)]
async fn read_message(
    reader: &mut tokio::net::tcp::OwnedReadHalf,
) -> Result<ServerMessage, TcpError> {
    // Read length prefix
    let mut length_bytes = [0u8; 4];
    reader.read_exact(&mut length_bytes).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            TcpError::ConnectionClosed
        } else {
            TcpError::Io(e)
        }
    })?;

    let length = u32::from_be_bytes(length_bytes);

    if length > MAX_MESSAGE_SIZE {
        return Err(TcpError::MessageTooLarge { size: length });
    }

    // Read payload
    let mut payload = vec![0u8; length as usize];
    reader.read_exact(&mut payload).await?;

    let json = String::from_utf8_lossy(&payload);
    debug!("Received: {}", json);

    let message: ServerMessage = serde_json::from_slice(&payload)?;
    Ok(message)
}

/// Read a single message from the stream, handling encryption if cipher is set.
async fn read_message_with_cipher(
    reader: &mut tokio::net::tcp::OwnedReadHalf,
    cipher_holder: &Arc<RwLock<Option<Arc<Cipher>>>>,
) -> Result<ServerMessage, TcpError> {
    // Clone cipher Arc before releasing lock
    let cipher_opt = {
        let guard = cipher_holder.read().unwrap();
        guard.as_ref().cloned()
    };

    // Read length prefix
    let mut length_bytes = [0u8; 4];
    reader.read_exact(&mut length_bytes).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            TcpError::ConnectionClosed
        } else {
            TcpError::Io(e)
        }
    })?;

    let length = u32::from_be_bytes(length_bytes) as usize;

    if length > MAX_MESSAGE_SIZE as usize {
        return Err(TcpError::MessageTooLarge {
            size: length as u32,
        });
    }

    match cipher_opt {
        Some(cipher) => {
            // Encrypted read
            if length < MIN_ENCRYPTED_SIZE {
                return Err(TcpError::EncryptedMessageTooSmall);
            }

            // Read nonce
            let mut nonce = [0u8; NONCE_SIZE];
            reader.read_exact(&mut nonce).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    TcpError::ConnectionClosed
                } else {
                    TcpError::Io(e)
                }
            })?;

            // Read ciphertext
            let ciphertext_len = length - NONCE_SIZE;
            let mut ciphertext = vec![0u8; ciphertext_len];
            reader.read_exact(&mut ciphertext).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    TcpError::ConnectionClosed
                } else {
                    TcpError::Io(e)
                }
            })?;

            // Decrypt
            let plaintext = cipher
                .decrypt(&nonce, &ciphertext)
                .map_err(|e| TcpError::DecryptionFailed(e.to_string()))?;

            let json = String::from_utf8_lossy(&plaintext);
            debug!("Received: {}", json);

            let message: ServerMessage = serde_json::from_slice(&plaintext)?;
            Ok(message)
        }
        None => {
            // Unencrypted read
            let mut payload = vec![0u8; length];
            reader.read_exact(&mut payload).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    TcpError::ConnectionClosed
                } else {
                    TcpError::Io(e)
                }
            })?;

            let json = String::from_utf8_lossy(&payload);
            debug!("Received: {}", json);

            let message: ServerMessage = serde_json::from_slice(&payload)?;
            Ok(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::login("alice", "secret");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"login\""));
        assert!(json.contains("\"identifier\":\"alice\""));
    }

    #[test]
    fn test_server_message_deserialization() {
        let json = r#"{"type":"server_hello","version":"1.0","server_name":"Test","features":[],"encryption_required":false}"#;
        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ServerMessage::ServerHello { .. }));
    }

    #[test]
    fn test_register_response_deserialization() {
        // Test with actual server response format (includes extra fields like updated_at, last_seen_at)
        let json = r#"{"type":"register_response","request_id":null,"success":true,"user":{"id":"b5545427-efa6-4b86-a092-211360f5cfc0","username":"testuser2","email":"test2@example.com","role":"user","created_at":"2026-01-22T19:15:18.123032478Z","updated_at":"2026-01-22T19:15:18.123032478Z","last_seen_at":null},"session":{"id":"ff99c390-4b0f-43cd-bd3b-9900fc69987c","expires_at":"2026-01-23T19:15:18.130987954+00:00"},"token":"ff99c390-4b0f-43cd-bd3b-9900fc69987c"}"#;
        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        match msg {
            ServerMessage::RegisterResponse {
                success,
                user,
                session,
                ..
            } => {
                assert!(success);
                assert!(user.is_some());
                assert!(session.is_some());
                assert_eq!(user.unwrap().username, "testuser2");
            }
            _ => panic!("Expected RegisterResponse"),
        }
    }

    #[test]
    fn test_login_response_deserialization() {
        let json = r#"{"type":"login_response","request_id":null,"success":true,"user":{"id":"3b6a415f-0f73-4eba-a99b-9b0b1c2808f5","username":"testuser","email":"test@example.com","role":"user","created_at":"2026-01-22T19:13:34Z","updated_at":"2026-01-22T19:13:34Z","last_seen_at":null},"session":{"id":"c32ed097-c87e-495d-b742-eaaa05142115","expires_at":"2026-01-23T19:13:44.386295902+00:00"},"token":"c32ed097-c87e-495d-b742-eaaa05142115"}"#;
        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        match msg {
            ServerMessage::LoginResponse {
                success,
                user,
                session,
                ..
            } => {
                assert!(success);
                assert!(user.is_some());
                assert!(session.is_some());
                assert_eq!(user.unwrap().username, "testuser");
            }
            _ => panic!("Expected LoginResponse"),
        }
    }
}
