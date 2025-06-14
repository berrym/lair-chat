use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::prelude::*;
use futures::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, env, error::Error, io, net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
};

use tokio_util::codec::{Framed, LinesCodec};
use tracing_subscriber::fmt::format::FmtSpan;
use x25519_dalek::{EphemeralSecret, PublicKey};

use lair_chat::server::auth::{
    AuthRequest, AuthService, MemorySessionStorage, MemoryUserStorage, User, UserStorage,
};

/// Shorthand for the transmit half of the message channel.
pub type Tx<T> = mpsc::UnboundedSender<T>;
/// Shorthand for the receive half of the message channel.
pub type Rx<T> = mpsc::UnboundedReceiver<T>;
pub type WriteData = (Vec<u8>, Tx<String>);

/// Data that is shared between all peers in the chat server.
struct SharedState {
    peers: HashMap<SocketAddr, WriteData>,
    auth_service: Arc<AuthService>,
    connected_users: HashMap<String, ConnectedUser>,
    rooms: HashMap<String, Room>,
}

#[derive(Debug, Clone)]
struct ConnectedUser {
    username: String,
    address: SocketAddr,
    connected_at: u64,
    current_room: String,
}

#[derive(Debug, Clone)]
struct Room {
    name: String,
    users: Vec<String>,
    created_at: u64,
    is_lobby: bool,
}

impl Room {
    fn new(name: String, is_lobby: bool) -> Self {
        Self {
            name,
            users: Vec::new(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            is_lobby,
        }
    }

    fn add_user(&mut self, username: String) {
        if !self.users.contains(&username) {
            self.users.push(username);
        }
    }

    fn remove_user(&mut self, username: &str) {
        self.users.retain(|u| u != username);
    }
}

impl SharedState {
    fn new() -> Self {
        let user_storage = Arc::new(MemoryUserStorage::new());
        let session_storage = Arc::new(MemorySessionStorage::new());
        let auth_service = Arc::new(AuthService::new(
            user_storage.clone(),
            session_storage,
            None,
        ));

        // Add default test users for easier testing
        tokio::spawn(async move {
            if let Ok(user1) = User::new("lusus".to_string(), "c2nt3ach") {
                let _ = user_storage.create_user(user1).await;
            }
            if let Ok(user2) = User::new("mberry".to_string(), "c2nt3ach") {
                let _ = user_storage.create_user(user2).await;
            }
            if let Ok(user3) = User::new("alice".to_string(), "password123") {
                let _ = user_storage.create_user(user3).await;
            }
            if let Ok(user4) = User::new("bob".to_string(), "password123") {
                let _ = user_storage.create_user(user4).await;
            }
            if let Ok(user5) = User::new("charlie".to_string(), "password123") {
                let _ = user_storage.create_user(user5).await;
            }
            tracing::info!("Default test users created: lusus, mberry, alice, bob, charlie");
        });

        let mut state = Self {
            peers: HashMap::new(),
            auth_service,
            connected_users: HashMap::new(),
            rooms: HashMap::new(),
        };

        // Create default Lobby room
        state
            .rooms
            .insert("Lobby".to_string(), Room::new("Lobby".to_string(), true));

        state
    }

    fn get_connected_users(&self) -> Vec<String> {
        self.connected_users.keys().cloned().collect()
    }

    async fn broadcast_user_list(&mut self) {
        let user_list = self.get_connected_users();
        let user_list_msg = format!("USER_LIST:{}", user_list.join(","));

        for (_, (_key, sender)) in self.peers.iter() {
            let _ = sender.send(user_list_msg.clone());
        }
    }

    async fn broadcast_room_status(&mut self, username: &str) {
        let room_status_msg = format!("ROOM_STATUS:Lobby,{}", username);

        for (_, (_key, sender)) in self.peers.iter() {
            let _ = sender.send(room_status_msg.clone());
        }
    }

    async fn broadcast(&mut self, sender: SocketAddr, message: &str) {
        for (addr, peer_data) in self.peers.iter_mut() {
            if *addr != sender {
                let _ = peer_data.1.send(message.to_string());
            }
        }
    }

    async fn send_to_user(&mut self, username: &str, message: &str) -> bool {
        tracing::info!(
            "DEBUG: send_to_user called - target: '{}', message: '{}'",
            username,
            message
        );

        if let Some(user) = self.connected_users.get(username) {
            let user_addr = user.address;
            tracing::info!("DEBUG: Found user {} at address {}", username, user_addr);

            if let Some((_key, sender)) = self.peers.get(&user_addr) {
                match sender.send(message.to_string()) {
                    Ok(()) => {
                        tracing::info!("DEBUG: Message sent successfully to {}", username);
                        true
                    }
                    Err(e) => {
                        tracing::error!("DEBUG: Failed to send message to {}: {}", username, e);
                        false
                    }
                }
            } else {
                tracing::warn!("DEBUG: No active connection found for user {}", username);
                false
            }
        } else {
            tracing::warn!("DEBUG: User {} not found in connected users", username);
            false
        }
    }

    async fn remove_peer(&mut self, addr: SocketAddr) {
        self.peers.remove(&addr);

        let username_to_remove = self
            .connected_users
            .iter()
            .find(|(_, user)| user.address == addr)
            .map(|(username, _)| username.clone());

        if let Some(username) = username_to_remove {
            self.connected_users.remove(&username);

            for room in self.rooms.values_mut() {
                room.remove_user(&username);
            }

            tracing::info!("User {} disconnected from {}", username, addr);
            self.broadcast_user_list().await;
        }
    }
}

struct Peer {
    messages: Rx<String>,
    state: Arc<Mutex<SharedState>>,
    addr: SocketAddr,
    aes_key: Vec<u8>,
}

impl Peer {
    async fn new(
        state: Arc<Mutex<SharedState>>,
        addr: SocketAddr,
        shared_aes_key: Vec<u8>,
    ) -> io::Result<Peer> {
        let (tx, rx) = mpsc::unbounded_channel();

        {
            let mut state_guard = state.lock().await;
            state_guard.peers.insert(addr, (shared_aes_key.clone(), tx));
        }

        Ok(Peer {
            messages: rx,
            state,
            addr,
            aes_key: shared_aes_key,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::FULL)
        .init();

    let state = Arc::new(Mutex::new(SharedState::new()));

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("Lair-Chat server listening on: {}", addr);

    loop {
        let (stream, addr) = listener.accept().await?;
        let state = Arc::clone(&state);

        tracing::info!("Accepting connection from: {}", addr);

        tokio::spawn(async move {
            let server_secret_key = EphemeralSecret::random();
            let server_public_key = PublicKey::from(&server_secret_key);

            if let Err(e) = process(state, stream, addr, server_public_key, server_secret_key).await
            {
                tracing::error!("Error processing connection {}: {}", addr, e);
            }
        });
    }
}

async fn process(
    state: Arc<Mutex<SharedState>>,
    stream: TcpStream,
    addr: SocketAddr,
    server_public_key: PublicKey,
    server_secret_key: EphemeralSecret,
) -> Result<(), Box<dyn Error>> {
    let mut transport = Framed::new(stream, LinesCodec::new());

    // Send server public key for handshake
    transport
        .send(BASE64_STANDARD.encode(server_public_key))
        .await?;

    // Receive client public key
    let peer_public_key_string = match transport.next().await {
        Some(Ok(key_string)) => {
            tracing::info!("Got public key from: {}", addr);
            key_string
        }
        Some(Err(e)) => {
            tracing::error!("Error receiving public key from {}: {}", addr, e);
            return Ok(());
        }
        None => {
            tracing::info!("Client {} disconnected during handshake", addr);
            return Ok(());
        }
    };

    // Decode and validate client public key
    let peer_public_key_vec = match BASE64_STANDARD.decode(peer_public_key_string) {
        Ok(decoded) => decoded,
        Err(e) => {
            tracing::error!("Failed to decode base64 public key from {}: {}", addr, e);
            return Ok(());
        }
    };

    if peer_public_key_vec.len() != 32 {
        tracing::error!("Invalid public key length from {}", addr);
        return Ok(());
    }

    let mut peer_public_key_array = [0u8; 32];
    peer_public_key_array.copy_from_slice(&peer_public_key_vec);
    let peer_public_key = PublicKey::from(peer_public_key_array);

    // Create shared secret and derive AES key
    let shared_secret = server_secret_key.diffie_hellman(&peer_public_key);
    let mut hasher = Sha256::new();
    hasher.update(shared_secret.as_bytes());
    hasher.update(b"LAIR_CHAT_AES_KEY");
    let result = hasher.finalize();
    let shared_aes256_key = result.to_vec();

    // Send welcome message
    transport
        .send(encrypt(
            shared_aes256_key.clone(),
            "Welcome to The Lair! Please login or register.".to_string(),
        ))
        .await?;

    // Handle authentication
    let mut user = None;

    while user.is_none() {
        match transport.next().await {
            Some(Ok(message)) => {
                let auth_request: AuthRequest =
                    match serde_json::from_str(&decrypt(shared_aes256_key.clone(), message)) {
                        Ok(req) => req,
                        Err(e) => {
                            tracing::error!("Invalid auth request from {}: {}", addr, e);
                            let _ = transport
                                .send(encrypt(
                                    shared_aes256_key.clone(),
                                    "Invalid authentication request".to_string(),
                                ))
                                .await;
                            continue;
                        }
                    };

                let username = auth_request.username.clone();
                let state_guard = state.lock().await;

                let result = if auth_request.is_registration {
                    match state_guard
                        .auth_service
                        .register(auth_request.username.clone(), &auth_request.password)
                        .await
                    {
                        Ok(_) => {
                            tracing::info!("User {} registered successfully", username);
                            match state_guard.auth_service.login(auth_request).await {
                                Ok(response) => {
                                    tracing::info!("Auto-login successful for {}", username);
                                    Ok(response.user)
                                }
                                Err(e) => {
                                    tracing::error!("Auto-login failed for {}: {}", username, e);
                                    Err(e)
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Registration failed for {}: {}", username, e);
                            Err(e)
                        }
                    }
                } else {
                    match state_guard.auth_service.login(auth_request).await {
                        Ok(response) => {
                            tracing::info!("Login successful for {}", username);
                            Ok(response.user)
                        }
                        Err(e) => {
                            tracing::error!("Login failed for {}: {}", username, e);
                            Err(e)
                        }
                    }
                };
                drop(state_guard);

                match result {
                    Ok(authenticated_user) => {
                        user = Some(authenticated_user.clone());

                        {
                            let mut state_guard = state.lock().await;
                            let connected_user = ConnectedUser {
                                username: authenticated_user.username.clone(),
                                address: addr,
                                connected_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs(),
                                current_room: "Lobby".to_string(),
                            };
                            state_guard
                                .connected_users
                                .insert(authenticated_user.username.clone(), connected_user);

                            if let Some(lobby) = state_guard.rooms.get_mut("Lobby") {
                                lobby.add_user(authenticated_user.username.clone());
                            }

                            let _ = transport
                                .send(encrypt(
                                    shared_aes256_key.clone(),
                                    "Authentication successful! Welcome to The Lair!".to_string(),
                                ))
                                .await;

                            state_guard.broadcast_user_list().await;
                            state_guard
                                .broadcast_room_status(&authenticated_user.username)
                                .await;
                        }

                        tracing::info!(
                            "User {} successfully authenticated and added to Lobby",
                            authenticated_user.username
                        );
                    }
                    Err(e) => {
                        let error_msg = format!("Authentication failed: {}", e);
                        let _ = transport
                            .send(encrypt(shared_aes256_key.clone(), error_msg))
                            .await;
                        tracing::warn!("Authentication failed for connection {}: {}", addr, e);
                    }
                }
            }
            Some(Err(e)) => {
                tracing::error!("Error reading auth message from {}: {}", addr, e);
                return Err(e.into());
            }
            None => {
                tracing::info!("Client {} disconnected during authentication", addr);
                return Ok(());
            }
        }
    }

    // Authentication successful, start message processing
    if let Some(authenticated_user) = user {
        let peer = Peer::new(state.clone(), addr, shared_aes256_key.clone()).await?;
        let Peer {
            mut messages,
            state,
            addr,
            aes_key,
        } = peer;

        let (mut sink, mut stream) = transport.split();

        // Handle outgoing messages
        let send_task = {
            let aes_key = aes_key.clone();
            tokio::spawn(async move {
                while let Some(msg) = messages.recv().await {
                    let encrypted_msg = encrypt(aes_key.clone(), msg);
                    if sink.send(encrypted_msg).await.is_err() {
                        break;
                    }
                }
            })
        };

        // Handle incoming messages
        while let Some(result) = stream.next().await {
            match result {
                Ok(message) => {
                    let decrypted_message = decrypt(aes_key.clone(), message);

                    if decrypted_message.starts_with("DM:") {
                        let parts: Vec<&str> = decrypted_message[3..].splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let target_username = parts[0];
                            let dm_content = parts[1];

                            tracing::info!(
                                "Processing DM from {} to {}: '{}'",
                                authenticated_user.username,
                                target_username,
                                dm_content
                            );

                            let dm_message =
                                format!("DM_FROM:{}:{}", authenticated_user.username, dm_content);

                            let mut state_guard = state.lock().await;
                            let sent = state_guard.send_to_user(target_username, &dm_message).await;

                            if !sent {
                                let error_msg = format!(
                                    "ERROR: User {} is not online or not found",
                                    target_username
                                );
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                            }
                        }
                    } else if decrypted_message == "REQUEST_USER_LIST" {
                        let state_guard = state.lock().await;
                        let user_list = state_guard.get_connected_users();
                        let user_list_msg = format!("USER_LIST:{}", user_list.join(","));
                        drop(state_guard);

                        if let Some((_key, sender)) = {
                            let state_guard = state.lock().await;
                            state_guard.peers.get(&addr).cloned()
                        } {
                            let _ = sender.send(user_list_msg);
                        }
                    } else {
                        let formatted_message =
                            format!("{}: {}", authenticated_user.username, decrypted_message);
                        let mut state_guard = state.lock().await;
                        state_guard.broadcast(addr, &formatted_message).await;
                    }
                }
                Err(e) => {
                    tracing::error!("Error reading message from {}: {}", addr, e);
                    break;
                }
            }
        }

        send_task.abort();
        let mut state_guard = state.lock().await;
        state_guard.remove_peer(addr).await;
    }

    Ok(())
}

fn encrypt(shared_aes256_key: Vec<u8>, message: String) -> String {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&shared_aes256_key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let encrypted_message = cipher.encrypt(&nonce, message.as_bytes()).unwrap();
    let combined = [nonce.as_slice(), &encrypted_message].concat();
    BASE64_STANDARD.encode(combined)
}

fn decrypt(shared_aes256_key: Vec<u8>, encrypted_string: String) -> String {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&shared_aes256_key));
    let combined = BASE64_STANDARD.decode(encrypted_string).unwrap();
    let (nonce_bytes, encrypted_message) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let decrypted_message = cipher.decrypt(nonce, encrypted_message).unwrap();
    String::from_utf8(decrypted_message).unwrap()
}
