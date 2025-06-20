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
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

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
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let lines = Framed::new(stream, LinesCodec::new());
    let (mut sink, mut stream) = lines.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Add peer to shared state
    {
        let mut state = state.lock().await;
        state.add_peer(addr, tx);
    }

    // Spawn task to handle outgoing messages
    let send_task = {
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
                            let error_msg = format!("{}", e);
                            error!("Auth error for {}: {}", addr, error_msg);
                            send_message_to_peer(&state, &addr, &format!("ERROR: {}", error_msg))
                                .await;
                        }
                    }
                } else if authenticated {
                    // Handle regular chat messages
                    if let Some(ref user) = username {
                        // Check message type BEFORE calling handle_chat_message
                        // to prevent special messages from being formatted with username prefix
                        if msg.starts_with("DM:") {
                            handle_dm_message(&msg, user, addr, &state).await;
                        } else if msg.starts_with("INVITE_USER:")
                            || msg.starts_with("ACCEPT_INVITATION:")
                            || msg.starts_with("DECLINE_INVITATION:")
                            || msg == "LIST_INVITATIONS"
                            || msg == "ACCEPT_ALL_INVITATIONS"
                            || msg.starts_with("CREATE_ROOM:")
                            || msg.starts_with("JOIN_ROOM:")
                            || msg == "LEAVE_ROOM"
                            || msg == "LIST_ROOMS"
                            || msg == "REQUEST_USER_LIST"
                        {
                            handle_room_and_invitation_commands(&msg, user, addr, &state).await;
                        } else {
                            handle_chat_message(&msg, user, addr, &state).await;
                        }
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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

/// Handle DM messages separately to ensure they never get broadcasted
async fn handle_dm_message(
    msg: &str,
    username: &str,
    addr: SocketAddr,
    state: &Arc<Mutex<SharedState>>,
) {
    let parts: Vec<&str> = msg.splitn(3, ':').collect();
    if parts.len() == 3 {
        let recipient = parts[1];
        let dm_content = parts[2];

        debug!(
            "Processing DM from {} to {}: {}",
            username, recipient, dm_content
        );

        let state_guard = state.lock().await;

        // Find the recipient's connection
        if let Some(recipient_user) = state_guard.get_user(recipient) {
            let recipient_addr = recipient_user.address;

            // Send DM only to the recipient with DM_FROM prefix
            let dm_message = format!("DM_FROM:{}:{}", username, dm_content);
            state_guard.send_to_peer(&recipient_addr, &dm_message);

            debug!(
                "DM sent from {} to {} at {}",
                username, recipient, recipient_addr
            );
        } else {
            // Recipient not found, send error back to sender
            let error_msg = format!("User '{}' not found or not online", recipient);
            state_guard.send_to_peer(&addr, &error_msg);
            warn!(
                "DM failed - recipient {} not found for sender {}",
                recipient, username
            );
        }

        drop(state_guard);
    } else {
        // Invalid DM format, send error back to sender
        let state_guard = state.lock().await;
        state_guard.send_to_peer(&addr, "Invalid DM format. Use: DM:username:message");
        drop(state_guard);
    }
}

/// Handle room and invitation commands (private, not broadcasted)
async fn handle_room_and_invitation_commands(
    msg: &str,
    username: &str,
    addr: SocketAddr,
    state: &Arc<Mutex<SharedState>>,
) {
    // Handle room commands
    if msg.starts_with("CREATE_ROOM:") {
        let room_name = msg.strip_prefix("CREATE_ROOM:").unwrap_or("").trim();
        if !room_name.is_empty() {
            let mut state_guard = state.lock().await;
            if state_guard.create_room(room_name.to_string()) {
                // Room created successfully, move user to it
                state_guard.move_user_to_room(username, room_name);
                state_guard.send_to_peer(&addr, &format!("ROOM_CREATED:{}", room_name));
                state_guard.send_to_peer(&addr, &format!("ROOM_JOINED:{}", room_name));
                debug!("Room '{}' created and joined by {}", room_name, username);
            } else {
                state_guard.send_to_peer(
                    &addr,
                    &format!("ROOM_ERROR:Room '{}' already exists", room_name),
                );
            }
            drop(state_guard);
        }
        return;
    }

    if msg.starts_with("JOIN_ROOM:") {
        let room_name = msg.strip_prefix("JOIN_ROOM:").unwrap_or("").trim();
        if !room_name.is_empty() {
            let mut state_guard = state.lock().await;
            if state_guard.get_room(room_name).is_some() {
                state_guard.move_user_to_room(username, room_name);
                state_guard.send_to_peer(&addr, &format!("ROOM_JOINED:{}", room_name));
                debug!("User {} joined room '{}'", username, room_name);
            } else {
                state_guard.send_to_peer(
                    &addr,
                    &format!("ROOM_ERROR:Room '{}' does not exist", room_name),
                );
            }
            drop(state_guard);
        }
        return;
    }

    if msg == "LEAVE_ROOM" {
        let mut state_guard = state.lock().await;
        state_guard.move_user_to_room(username, "lobby");
        state_guard.send_to_peer(&addr, "ROOM_LEFT:left room");
        state_guard.send_to_peer(&addr, "ROOM_JOINED:lobby");
        debug!("User {} left room and returned to lobby", username);
        drop(state_guard);
        return;
    }

    if msg == "LIST_ROOMS" {
        let state_guard = state.lock().await;
        let rooms: Vec<String> = state_guard
            .get_all_rooms()
            .iter()
            .filter(|room| !room.is_lobby)
            .map(|room| room.name.clone())
            .collect();
        let room_list = if rooms.is_empty() {
            "ROOM_LIST:".to_string()
        } else {
            format!("ROOM_LIST:{}", rooms.join(","))
        };
        state_guard.send_to_peer(&addr, &room_list);
        drop(state_guard);
        return;
    }

    if msg == "REQUEST_USER_LIST" {
        let state_guard = state.lock().await;
        let users: Vec<String> = state_guard
            .get_all_users()
            .iter()
            .map(|user| user.username.clone())
            .collect();
        let user_list = if users.is_empty() {
            "USER_LIST:".to_string()
        } else {
            format!("USER_LIST:{}", users.join(","))
        };
        state_guard.send_to_peer(&addr, &user_list);
        drop(state_guard);
        return;
    }

    if msg.starts_with("INVITE_USER:") {
        let invite_data = msg.strip_prefix("INVITE_USER:").unwrap_or("");
        let parts: Vec<&str> = invite_data.splitn(2, ':').collect();
        if parts.len() == 2 {
            let target_username = parts[0];
            let room_name = parts[1];

            debug!(
                "Processing invite from {} to invite {} to room '{}'",
                username, target_username, room_name
            );

            let mut state_guard = state.lock().await;

            // Check if the target user exists and is online
            if let Some(target_user) = state_guard.get_user(target_username) {
                let target_addr = target_user.address;

                // Check if the room exists
                if state_guard.get_room(room_name).is_some() {
                    // Add pending invitation
                    let invitation = crate::server::app::state::PendingInvitation {
                        inviter: username.to_string(),
                        room_name: room_name.to_string(),
                        invited_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    };
                    state_guard.add_pending_invitation(target_username, invitation);

                    // Get updated count for better UX
                    let pending_count = state_guard.get_pending_invitations(target_username).len();

                    // Send invitation to the target user with position info
                    let invite_message = format!(
                        "INVITATION:{}:{}:{} invited you to join the chat room '{}' (#{} of {} pending)",
                        username, room_name, username, room_name, pending_count, pending_count
                    );
                    state_guard.send_to_peer(&target_addr, &invite_message);

                    // Send confirmation to the inviter
                    state_guard.send_to_peer(
                        &addr,
                        &format!(
                            "Invitation sent to {} for room '{}'",
                            target_username, room_name
                        ),
                    );

                    debug!(
                        "Invitation sent from {} to {} for room '{}'",
                        username, target_username, room_name
                    );
                } else {
                    // Room doesn't exist
                    state_guard.send_to_peer(
                        &addr,
                        &format!("INVITE_ERROR:Room '{}' does not exist", room_name),
                    );
                }
            } else {
                // Target user not found or not online
                state_guard.send_to_peer(
                    &addr,
                    &format!(
                        "INVITE_ERROR:User '{}' not found or not online",
                        target_username
                    ),
                );
            }

            drop(state_guard);
        } else {
            // Invalid invite format
            let state_guard = state.lock().await;
            state_guard.send_to_peer(&addr, "INVITE_ERROR:Invalid invite format");
            drop(state_guard);
        }
        return;
    }

    // Handle ACCEPT_INVITATION
    if msg.starts_with("ACCEPT_INVITATION:") {
        let room_name = &msg[18..]; // Remove "ACCEPT_INVITATION:" prefix
        let mut state_guard = state.lock().await;

        if room_name == "LATEST" {
            // Accept the latest invitation
            if let Some(latest_invite) = state_guard.get_latest_invitation(username) {
                let room_to_join = latest_invite.room_name.clone();

                // Remove the invitation
                state_guard.remove_pending_invitation(username, &room_to_join);

                // Join the room using existing method
                if state_guard.move_user_to_room(username, &room_to_join) {
                    state_guard.send_to_peer(&addr, &format!("Joined room '{}'", room_to_join));
                    state_guard
                        .broadcast_to_room(&room_to_join, &format!("{} joined the room", username));
                } else {
                    state_guard.send_to_peer(&addr, "ACCEPT_ERROR:Failed to join room");
                }
            } else {
                state_guard.send_to_peer(&addr, "ACCEPT_ERROR:No pending invitations");
            }
        } else {
            // Accept specific room invitation
            if state_guard.remove_pending_invitation(username, room_name) {
                // Join the room using existing method
                if state_guard.move_user_to_room(username, room_name) {
                    state_guard.send_to_peer(&addr, &format!("Joined room '{}'", room_name));
                    state_guard
                        .broadcast_to_room(room_name, &format!("{} joined the room", username));
                } else {
                    state_guard.send_to_peer(&addr, "ACCEPT_ERROR:Failed to join room");
                }
            } else {
                state_guard.send_to_peer(
                    &addr,
                    &format!("ACCEPT_ERROR:No invitation found for room '{}'", room_name),
                );
            }
        }

        drop(state_guard);
        return;
    }

    // Handle DECLINE_INVITATION
    if msg.starts_with("DECLINE_INVITATION:") {
        let room_name = &msg[19..]; // Remove "DECLINE_INVITATION:" prefix
        let mut state_guard = state.lock().await;

        if room_name == "LATEST" {
            // Decline the latest invitation
            if let Some(latest_invite) = state_guard.get_latest_invitation(username) {
                let room_to_decline = latest_invite.room_name.clone();
                state_guard.remove_pending_invitation(username, &room_to_decline);
                state_guard.send_to_peer(
                    &addr,
                    &format!("Declined invitation to room '{}'", room_to_decline),
                );
            } else {
                state_guard.send_to_peer(&addr, "DECLINE_ERROR:No pending invitations");
            }
        } else {
            // Decline specific room invitation
            if state_guard.remove_pending_invitation(username, room_name) {
                state_guard.send_to_peer(
                    &addr,
                    &format!("Declined invitation to room '{}'", room_name),
                );
            } else {
                state_guard.send_to_peer(
                    &addr,
                    &format!("DECLINE_ERROR:No invitation found for room '{}'", room_name),
                );
            }
        }

        drop(state_guard);
        return;
    }

    // Handle LIST_INVITATIONS
    if msg == "LIST_INVITATIONS" {
        let state_guard = state.lock().await;
        let invitations = state_guard.get_pending_invitations(username);

        if invitations.is_empty() {
            state_guard.send_to_peer(&addr, "No pending invitations");
        } else {
            let mut invite_list = String::from("Pending invitations:\n");
            for (i, invite) in invitations.iter().enumerate() {
                invite_list.push_str(&format!(
                    "{}. Room: '{}' (from {})\n",
                    i + 1,
                    invite.room_name,
                    invite.inviter
                ));
            }
            invite_list.push_str("Use '/accept <room>' or '/accept' for latest");
            state_guard.send_to_peer(&addr, &invite_list);
        }

        drop(state_guard);
        return;
    }

    // Handle ACCEPT_ALL_INVITATIONS
    if msg == "ACCEPT_ALL_INVITATIONS" {
        let mut state_guard = state.lock().await;
        let invitations = state_guard
            .get_pending_invitations(username)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        if invitations.is_empty() {
            state_guard.send_to_peer(&addr, "No pending invitations to accept");
        } else {
            let count = invitations.len();
            let mut last_room = None;

            // Accept all invitations (user will end up in the last room)
            for invite in invitations {
                state_guard.remove_pending_invitation(username, &invite.room_name);
                last_room = Some(invite.room_name);
            }

            // Join the last room
            if let Some(room_name) = last_room {
                if state_guard.move_user_to_room(username, &room_name) {
                    state_guard.send_to_peer(
                        &addr,
                        &format!(
                            "Accepted {} invitations and joined room '{}'",
                            count, room_name
                        ),
                    );
                    state_guard
                        .broadcast_to_room(&room_name, &format!("{} joined the room", username));
                } else {
                    state_guard.send_to_peer(
                        &addr,
                        &format!("Accepted {} invitations but failed to join room", count),
                    );
                }
            }
        }

        drop(state_guard);
        return;
    }

    // Unknown command
    let state_guard = state.lock().await;
    state_guard.send_to_peer(&addr, &format!("Unknown command: {}", msg));
    drop(state_guard);
}

/// Handle chat messages (broadcasted to room only)
async fn handle_chat_message(
    msg: &str,
    username: &str,
    addr: SocketAddr,
    state: &Arc<Mutex<SharedState>>,
) {
    let formatted_msg = format!("{}: {}", username, msg);

    let state_guard = state.lock().await;

    // Get user's current room and broadcast only to that room
    if let Some(user) = state_guard.get_user(username) {
        let current_room = user.current_room.clone();
        state_guard.broadcast_to_room(&current_room, &formatted_msg);
        debug!(
            "Broadcasted message from {} to room '{}': {}",
            username, current_room, msg
        );
    } else {
        // Fallback to old behavior if user not found
        state_guard.broadcast_message(&addr, &formatted_msg);
        debug!("Broadcasted message from {} (fallback): {}", username, msg);
    }

    drop(state_guard);
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
fn decrypt_message(
    encrypted_msg: &str,
    key: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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
    async fn test_encryption_handshake() {
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
