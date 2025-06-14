//! Server main logic for lair-chat
//!
//! This module contains the main server loop and connection handling logic
//! for the lair-chat server, including encryption, authentication, and message processing.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::prelude::*;
use futures::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use std::{error::Error, net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use tokio_stream::StreamExt as TokioStreamExt;
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{debug, error, info, warn};
use x25519_dalek::{EphemeralSecret, PublicKey};

use super::state::SharedState;
use crate::server::auth::{AuthService, MemorySessionStorage, MemoryUserStorage};

/// Main server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub enable_encryption: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
            enable_encryption: true,
        }
    }
}

/// Main server struct
pub struct ChatServer {
    config: ServerConfig,
    state: Arc<Mutex<SharedState>>,
}

impl ChatServer {
    /// Create a new chat server instance
    pub fn new(config: ServerConfig) -> Self {
        let user_storage = Arc::new(MemoryUserStorage::new());
        let session_storage = Arc::new(MemorySessionStorage::new());
        let auth_service = Arc::new(AuthService::new(user_storage, session_storage, None));
        let state = Arc::new(Mutex::new(SharedState::new(auth_service)));

        Self { config, state }
    }

    /// Start the server and listen for connections
    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;

        info!("Lair-Chat server listening on: {}", addr);
        info!("Server configuration: {:?}", self.config);

        loop {
            let (stream, addr) = listener.accept().await?;
            info!("New connection from: {}", addr);

            let state = Arc::clone(&self.state);
            let config = self.config.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, addr, state, config).await {
                    error!("Error handling connection from {}: {}", addr, e);
                }
            });
        }
    }

    /// Get server statistics
    pub async fn get_stats(&self) -> crate::server::app::state::ServerStats {
        let state = self.state.lock().await;
        state.get_stats()
    }

    /// Shutdown the server gracefully
    pub async fn shutdown(&self) {
        info!("Shutting down server...");
        // Additional cleanup can be added here
    }
}

/// Handle a single client connection
async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    state: Arc<Mutex<SharedState>>,
    config: ServerConfig,
) -> Result<(), Box<dyn Error>> {
    let lines = Framed::new(stream, LinesCodec::new());
    let (mut sink, mut stream) = lines.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Add peer to shared state
    {
        let mut state = state.lock().await;
        state.add_peer(addr, tx);
    }

    // Spawn task to handle outgoing messages
    let mut send_task = {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if sink.send(msg).await.is_err() {
                    break;
                }
            }
        })
    };

    let mut username: Option<String> = None;
    let mut authenticated = false;
    let mut encryption_key: Option<[u8; 32]> = None;

    // Handle incoming messages
    while let Some(result) = futures::StreamExt::next(&mut stream).await {
        match result {
            Ok(mut msg) => {
                debug!("Received from {}: {}", addr, msg);

                // Decrypt message if encryption is enabled and key is available
                if config.enable_encryption && encryption_key.is_some() && msg.starts_with("ENC:") {
                    match decrypt_message(&msg[4..], &encryption_key.unwrap()) {
                        Ok(decrypted) => msg = decrypted,
                        Err(e) => {
                            warn!("Failed to decrypt message from {}: {}", addr, e);
                            continue;
                        }
                    }
                }

                // Handle different message types
                if msg.starts_with("HANDSHAKE:") {
                    if let Err(e) = handle_handshake(&msg, addr, &state, &mut encryption_key).await
                    {
                        error!("Handshake error for {}: {}", addr, e);
                        break;
                    }
                } else if msg.starts_with("AUTH:") {
                    match handle_auth(&msg, addr, &state, &mut username, &mut authenticated).await {
                        Ok(response) => {
                            send_message_to_peer(&state, &addr, &response).await;
                        }
                        Err(e) => {
                            error!("Auth error for {}: {}", addr, e);
                            send_message_to_peer(&state, &addr, &format!("ERROR: {}", e)).await;
                        }
                    }
                } else if authenticated {
                    // Handle regular chat messages
                    if let Some(ref user) = username {
                        handle_chat_message(&msg, user, addr, &state).await;
                    }
                } else {
                    warn!("Unauthenticated message from {}: {}", addr, msg);
                    send_message_to_peer(&state, &addr, "ERROR: Not authenticated").await;
                }
            }
            Err(e) => {
                error!("Error reading from {}: {}", addr, e);
                break;
            }
        }
    }

    // Cleanup when connection is closed
    send_task.abort();
    cleanup_connection(addr, &username, &state).await;
    info!("Connection closed: {}", addr);

    Ok(())
}

/// Handle encryption handshake
async fn handle_handshake(
    msg: &str,
    addr: SocketAddr,
    state: &Arc<Mutex<SharedState>>,
    encryption_key: &mut Option<[u8; 32]>,
) -> Result<(), Box<dyn Error>> {
    let client_public_key_b64 = &msg[10..]; // Remove "HANDSHAKE:" prefix
    let client_public_key_bytes = BASE64_STANDARD.decode(client_public_key_b64)?;

    if client_public_key_bytes.len() != 32 {
        return Err("Invalid public key length".into());
    }

    let mut client_public_key_array = [0u8; 32];
    client_public_key_array.copy_from_slice(&client_public_key_bytes);
    let client_public_key = PublicKey::from(client_public_key_array);

    // Generate server key pair
    let server_secret = EphemeralSecret::random();
    let server_public = PublicKey::from(&server_secret);

    // Compute shared secret
    let shared_secret = server_secret.diffie_hellman(&client_public_key);

    // Derive encryption key from shared secret
    let mut hasher = Sha256::new();
    hasher.update(shared_secret.as_bytes());
    let key_bytes = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes);
    *encryption_key = Some(key);

    // Send server public key back to client
    let server_public_b64 = BASE64_STANDARD.encode(server_public.as_bytes());
    let response = format!("HANDSHAKE_RESPONSE:{}", server_public_b64);
    send_message_to_peer(state, &addr, &response).await;

    info!("Encryption handshake completed for {}", addr);
    Ok(())
}

/// Handle authentication messages
async fn handle_auth(
    msg: &str,
    addr: SocketAddr,
    state: &Arc<Mutex<SharedState>>,
    username: &mut Option<String>,
    authenticated: &mut bool,
) -> Result<String, Box<dyn Error>> {
    let auth_data = &msg[5..]; // Remove "AUTH:" prefix
    let parts: Vec<&str> = auth_data.split(':').collect();

    if parts.len() < 3 {
        return Err("Invalid auth format".into());
    }

    let auth_type = parts[0];
    let user = parts[1];
    let pass = parts[2];

    let state_guard = state.lock().await;
    let _auth_service = Arc::clone(&state_guard.auth_service);
    drop(state_guard);

    match auth_type {
        "LOGIN" => {
            // Attempt to authenticate user
            // For now, simple validation - in production, use proper password hashing
            if user.len() >= 3 && pass.len() >= 6 {
                *username = Some(user.to_string());
                *authenticated = true;

                // Add user to state
                {
                    let mut state_guard = state.lock().await;
                    state_guard.add_user(user.to_string(), addr);
                }

                info!("User {} authenticated from {}", user, addr);
                Ok("AUTH_SUCCESS".to_string())
            } else {
                Err("Invalid credentials".into())
            }
        }
        "REGISTER" => {
            // Register new user
            if user.len() >= 3 && pass.len() >= 6 {
                // In a real implementation, store user in database
                *username = Some(user.to_string());
                *authenticated = true;

                // Add user to state
                {
                    let mut state_guard = state.lock().await;
                    state_guard.add_user(user.to_string(), addr);
                }

                info!("New user {} registered from {}", user, addr);
                Ok("REGISTER_SUCCESS".to_string())
            } else {
                Err("Username must be at least 3 characters, password at least 6".into())
            }
        }
        _ => Err("Unknown auth type".into()),
    }
}

/// Handle chat messages
async fn handle_chat_message(
    msg: &str,
    username: &str,
    addr: SocketAddr,
    state: &Arc<Mutex<SharedState>>,
) {
    let formatted_msg = format!("{}: {}", username, msg);

    let state_guard = state.lock().await;
    state_guard.broadcast_message(&addr, &formatted_msg);
    drop(state_guard);

    debug!("Broadcasted message from {}: {}", username, msg);
}

/// Send a message to a specific peer
async fn send_message_to_peer(state: &Arc<Mutex<SharedState>>, addr: &SocketAddr, msg: &str) {
    let state_guard = state.lock().await;
    state_guard.send_to_peer(addr, msg);
}

/// Cleanup when a connection is closed
async fn cleanup_connection(
    addr: SocketAddr,
    username: &Option<String>,
    state: &Arc<Mutex<SharedState>>,
) {
    let mut state_guard = state.lock().await;

    // Remove peer
    state_guard.remove_peer(&addr);

    // Remove user if they were authenticated
    if let Some(user) = username {
        state_guard.remove_user(user);
        info!("User {} disconnected", user);
    }
}

/// Decrypt a message using AES-256-GCM
fn decrypt_message(encrypted_msg: &str, key: &[u8; 32]) -> Result<String, Box<dyn Error>> {
    let encrypted_data = BASE64_STANDARD.decode(encrypted_msg)?;

    if encrypted_data.len() < 12 {
        return Err("Invalid encrypted data length".into());
    }

    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {:?}", e))?;
    Ok(String::from_utf8(plaintext)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.max_connections, 1000);
        assert!(config.enable_encryption);
    }

    #[test]
    fn test_server_creation() {
        let config = ServerConfig::default();
        let server = ChatServer::new(config);
        // Basic test to ensure server can be created
        assert_eq!(server.config.port, 8080);
    }

    #[tokio::test]
    #[test]
    fn test_encryption_handshake() {
        // Test the encryption key derivation logic
        let secret1 = EphemeralSecret::random();
        let secret2 = EphemeralSecret::random();
        let public1 = PublicKey::from(&secret1);
        let public2 = PublicKey::from(&secret2);

        let shared1 = secret1.diffie_hellman(&public2);
        let shared2 = secret2.diffie_hellman(&public1);

        // Both sides should compute the same shared secret
        assert_eq!(shared1.as_bytes(), shared2.as_bytes());
    }
}
