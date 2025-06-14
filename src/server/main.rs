use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::prelude::*;
use futures::SinkExt;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, env, error::Error, io, net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};
use tracing_subscriber::fmt::format::FmtSpan;
use x25519_dalek::{EphemeralSecret, PublicKey};

mod auth;
use auth::{AuthRequest, AuthService, MemorySessionStorage, MemoryUserStorage, User, UserStorage};

/// Shorthand for the transmit half of the message channel.
pub type Tx<T> = mpsc::UnboundedSender<T>;
/// Shorthand for the receive half of the message channel.
pub type Rx<T> = mpsc::UnboundedReceiver<T>;
pub type WriteData = (Vec<u8>, Tx<String>);

/// Data that is shared between all peers in the chat server.
///
/// This is the set of `Tx` handles for all connected clients. Whenever a
/// message is received from a client, it is broadcasted to all peers by
/// iterating over the `peers` entries and sending a copy of the message on each
/// `Tx`.
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
    /// Create a new, empty, instance of `SharedState`.
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
            if let Ok(user4) = User::new("bob".to_string(), "password456") {
                let _ = user_storage.create_user(user4).await;
            }
            tracing::info!("Default test users created");
        });

        let mut rooms = HashMap::new();
        // Create the default Lobby room
        rooms.insert("Lobby".to_string(), Room::new("Lobby".to_string(), true));

        SharedState {
            peers: HashMap::new(),
            auth_service,
            connected_users: HashMap::new(),
            rooms,
        }
    }

    /// Add a connected user to the server tracking and join them to the Lobby
    fn add_connected_user(&mut self, username: String, address: SocketAddr) {
        let user = ConnectedUser {
            username: username.clone(),
            address,
            connected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            current_room: "Lobby".to_string(),
        };

        self.connected_users.insert(username.clone(), user);

        // Add user to the Lobby room
        if let Some(lobby) = self.rooms.get_mut("Lobby") {
            lobby.add_user(username.clone());
        }

        tracing::info!("User {} joined the Lobby", username);
    }

    /// Remove a connected user from server tracking and all rooms
    fn remove_connected_user(&mut self, address: SocketAddr) {
        // Find the user by address
        let username = self
            .connected_users
            .iter()
            .find(|(_, user)| user.address == address)
            .map(|(username, _)| username.clone());

        if let Some(username) = username {
            // Remove from all rooms
            for room in self.rooms.values_mut() {
                room.remove_user(&username);
            }

            // Remove from connected users
            self.connected_users.remove(&username);
            tracing::info!("User {} left the server", username);
        }
    }

    /// Get list of all connected users (for DM user discovery)
    fn get_connected_users(&self) -> Vec<String> {
        self.connected_users.keys().cloned().collect()
    }

    /// Get users in a specific room
    fn get_room_users(&self, room_name: &str) -> Vec<String> {
        self.rooms
            .get(room_name)
            .map(|room| room.users.clone())
            .unwrap_or_default()
    }

    /// Broadcast user list to all connected clients (encrypted)
    async fn broadcast_user_list(&mut self) {
        let user_list = self.get_connected_users();
        let user_list_msg = format!("USER_LIST:{}", user_list.join(","));

        // Broadcast to all peers with encryption
        for (_, peer_data) in self.peers.iter_mut() {
            let encrypted_msg = encrypt(peer_data.0.clone(), user_list_msg.clone());
            let _ = peer_data.1.send(encrypted_msg);
        }
    }

    /// Broadcast a room status message to all connected clients
    async fn broadcast_room_status(&mut self, username: &str) {
        let room_status_msg = format!("ROOM_STATUS:Lobby,{}", username);

        // Broadcast to all peers with encryption
        for (_, peer_data) in self.peers.iter_mut() {
            let encrypted_msg = encrypt(peer_data.0.clone(), room_status_msg.clone());
            let _ = peer_data.1.send(encrypted_msg);
        }
    }

    /// Send a `LineCodec` encoded message to every peer, except
    /// for the sender.
    async fn broadcast(&mut self, sender: SocketAddr, message: &str) {
        for (addr, peer_data) in self.peers.iter_mut() {
            if *addr != sender {
                let encrypted_msg = encrypt(peer_data.0.clone(), message.to_string());
                let _ = peer_data.1.send(encrypted_msg);
            }
        }
    }

    /// Send a message to a specific user
    async fn send_to_user(&mut self, username: &str, message: &str) -> bool {
        tracing::info!(
            "DEBUG: send_to_user called - target: '{}', message: '{}'",
            username,
            message
        );

        if let Some(user) = self.connected_users.get(username) {
            tracing::info!(
                "DEBUG: Found user '{}' at address {}",
                username,
                user.address
            );
            if let Some(peer_data) = self.peers.get_mut(&user.address) {
                let encrypted_msg = encrypt(peer_data.0.clone(), message.to_string());
                tracing::info!("DEBUG: Sending encrypted message to {}", username);
                let send_result = peer_data.1.send(encrypted_msg);
                match send_result {
                    Ok(_) => {
                        tracing::info!("DEBUG: Successfully sent message to {}", username);
                        return true;
                    }
                    Err(e) => {
                        tracing::error!("DEBUG: Failed to send message to {}: {:?}", username, e);
                        return false;
                    }
                }
            } else {
                tracing::warn!(
                    "DEBUG: User '{}' found but no peer data at address {}",
                    username,
                    user.address
                );
            }
        } else {
            tracing::warn!("DEBUG: User '{}' not found in connected_users", username);
            tracing::info!(
                "DEBUG: Current connected users: {:?}",
                self.connected_users.keys().collect::<Vec<_>>()
            );
        }
        false
    }
}

/// The state for each connected client.
struct Peer {
    /// The TCP socket wrapped with the `Lines` codec, defined below.
    ///
    /// This handles sending and receiving data on the socket. When using
    /// `Lines`, we can work at the line level instead of having to manage the
    /// raw byte operations.
    transport: Framed<TcpStream, LinesCodec>,

    /// Receive half of the message channel.
    ///
    /// This is used to receive messages from peers. When a message is received
    /// off of this `Rx`, it will be written to the socket.
    rx: Rx<String>,
}

impl Peer {
    /// Create a new instance of `Peer`.
    async fn new(
        state: Arc<Mutex<SharedState>>,
        transport: Framed<TcpStream, LinesCodec>,
        shared_aes_key: Vec<u8>,
    ) -> io::Result<Peer> {
        let mut state_guard = state.lock().await;

        // Get the client socket address
        let addr = transport.get_ref().peer_addr()?;

        // Create a channel for this peer
        let (tx, rx) = mpsc::unbounded_channel();

        // Create a shared key/transport writer tuple
        let write_data = (shared_aes_key, tx);

        // Add an entry for this `Peer` in the shared state map.
        state_guard.peers.insert(addr, write_data);

        Ok(Peer { transport, rx })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Configure a `tracing` subscriber that logs traces emitted by the chat
    // server.
    tracing_subscriber::fmt()
        // Filter what traces are displayed based on the RUST_LOG environment
        // variable.
        //
        // Traces emitted by the example code will always be displayed. You
        // can set `RUST_LOG=tokio=trace` to enable additional traces emitted by
        // Tokio itself.
        // .with_env_filter(EnvFilter::from_default_env().add_directive("chat=info".parse()?))
        // Log events when `tracing` spans are created, entered, exited, or
        // closed. When Tokio's internal tracing support is enabled (as
        // described above), this can be used to track the lifecycle of spawned
        // tasks on the Tokio runtime.
        .with_span_events(FmtSpan::FULL)
        // Set this subscriber as the default, to collect all traces emitted by
        // the program.
        .init();

    // Create the shared state. This is how all the peers communicate.
    //
    // The server task will hold a handle to this. For every new client, the
    // `state` handle is cloned and passed into the task that processes the
    // client connection.
    let state = Arc::new(Mutex::new(SharedState::new()));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    // Bind a TCP listener to the socket address.
    //
    // Note that this is the Tokio TcpListener, which is fully async.
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("server running on {}", addr);
    tracing::info!("Lobby room created and ready for connections");

    loop {
        let server_secret_key = EphemeralSecret::random();
        let server_public_key = PublicKey::from(&server_secret_key);

        // Asynchronously wait for an inbound TcpStream.
        let (stream, addr) = listener.accept().await?;

        // Clone a handle to the `SharedState` for the new connection.
        let state = Arc::clone(&state);

        // Spawn our handler to be run asynchronously.
        tokio::spawn(async move {
            tracing::debug!("accepted connection");
            if let Err(e) = process(state, stream, addr, server_public_key, server_secret_key).await
            {
                tracing::info!("an error occurred; error = {:?}", e);
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
    // start the handshake by sending public key to peer
    let mut transport = Framed::new(stream, LinesCodec::new());
    transport
        .send(BASE64_STANDARD.encode(server_public_key))
        .await?;
    // recieve peer's public key
    let peer_public_key_string = match transport.next().await {
        Some(key_string) => {
            println!("Got public key from: {}", addr.to_string());
            key_string
        }
        None => {
            println!("Failed to get public key from peer!");
            return Ok(());
        }
    };
    // keep converting until key is a 32 byte u8 array
    let peer_public_key_vec = match peer_public_key_string {
        Ok(key_vec) => match BASE64_STANDARD.decode(key_vec) {
            Ok(decoded) => decoded,
            Err(e) => {
                tracing::error!("Failed to decode base64 public key from {}: {}", addr, e);
                return Ok(());
            }
        },
        Err(e) => {
            tracing::error!("Failed to receive public key from {}: {}", addr, e);
            return Ok(());
        }
    };
    let peer_public_key_slice: &[u8] = match peer_public_key_vec.as_slice().try_into() {
        Ok(key_slice) => key_slice,
        _ => {
            println!("Failed to convert peer public key byte vec to slice!");
            return Ok(());
        }
    };
    let peer_public_key_array: [u8; 32] = match peer_public_key_slice.try_into() {
        Ok(key_array) => key_array,
        _ => {
            println!("Failed to convert public key slice to byte array!");
            return Ok(());
        }
    };
    // create shared keys using secure SHA-256 with domain separation
    let shared_secret = server_secret_key.diffie_hellman(&PublicKey::from(peer_public_key_array));
    let mut hasher = Sha256::new();
    hasher.update(shared_secret.as_bytes());
    hasher.update(b"LAIR_CHAT_AES_KEY"); // Domain separation
    let result = hasher.finalize();
    let shared_aes256_key = result.to_vec();

    // Send a welcome prompt to the client
    transport
        .send(encrypt(
            shared_aes256_key.clone(),
            "Welcome to The Lair! Please login or register.".to_string(),
        ))
        .await?;

    // Handle authentication
    let mut user = None;
    let mut session = None;

    while user.is_none() {
        match transport.next().await {
            Some(Ok(message)) => {
                let auth_request: AuthRequest =
                    serde_json::from_str(&decrypt(shared_aes256_key.clone(), message)).map_err(
                        |_| io::Error::new(io::ErrorKind::InvalidData, "Invalid auth request"),
                    )?;

                // Store username before auth_request is moved
                let username = auth_request.username.clone();

                let state_guard = state.lock().await;
                let result = if auth_request.is_registration {
                    // Register the user first
                    match state_guard
                        .auth_service
                        .register(auth_request.username.clone(), &auth_request.password)
                        .await
                    {
                        Ok(_registered_user) => {
                            tracing::info!(
                                "User {} registered successfully, attempting auto-login",
                                username
                            );
                            // After successful registration, automatically log them in
                            match state_guard.auth_service.login(auth_request).await {
                                Ok(response) => {
                                    session = Some(response.session.clone());
                                    tracing::info!(
                                        "Auto-login successful for {}, session: {}",
                                        response.user.username,
                                        response.session.token
                                    );
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
                            session = Some(response.session.clone());
                            tracing::info!(
                                "Login successful for {}, session: {}",
                                response.user.username,
                                response.session.token
                            );
                            Ok(response.user)
                        }
                        Err(e) => {
                            tracing::error!("Login failed for {}: {}", username, e);
                            Err(e)
                        }
                    }
                };
                drop(state_guard); // Release the lock

                match result {
                    Ok(authenticated_user) => {
                        user = Some(authenticated_user.clone());

                        // Add user to server tracking and broadcast updated user list
                        {
                            let mut state_guard = state.lock().await;
                            state_guard
                                .add_connected_user(authenticated_user.username.clone(), addr);
                            state_guard.broadcast_user_list().await;
                            state_guard
                                .broadcast_room_status(&authenticated_user.username)
                                .await;
                        }

                        transport
                            .send(encrypt(
                                shared_aes256_key.clone(),
                                format!("Welcome to the Lobby, {}!", authenticated_user.username),
                            ))
                            .await?;
                        break;
                    }
                    Err(e) => {
                        transport
                            .send(encrypt(
                                shared_aes256_key.clone(),
                                format!("Authentication failed: {}", e),
                            ))
                            .await?;
                    }
                }
            }
            _ => {
                tracing::error!("Client disconnected during authentication: {}", addr);
                return Ok(());
            }
        }
    }

    let user = user.unwrap();
    let session = session.unwrap();

    // Register our peer with state which internally sets up some channels.
    let mut peer = Peer::new(state.clone(), transport, shared_aes256_key.clone()).await?;

    // A client has connected, let's let everyone know.
    {
        let mut state_guard = state.lock().await;
        let message = format!("{} has joined the Lobby", user.username);
        tracing::info!("{}", message);
        state_guard.broadcast(addr, &message).await;
    }

    // Process incoming messages until our stream is exhausted by a disconnect.
    loop {
        tokio::select! {
            // A message was received from a peer. Send it to the current user.
            Some(message) = peer.rx.recv() => {
                peer.transport.send(message).await?;
            }
            result = peer.transport.next() => match result {
                // A message was received from the current user, we should
                // broadcast this message to the other users.
                Some(Ok(message)) => {
                    let decrypted_message = decrypt(shared_aes256_key.clone(), message);
                    tracing::info!("DEBUG: Received message from {}: '{}'", user.username, decrypted_message);

                    // Handle special protocol messages
                    if decrypted_message.starts_with("DM:") {
                        tracing::info!("DEBUG: Processing DM message from {}: '{}'", user.username, decrypted_message);
                        // Handle direct message: DM:target_user:message_content
                        let parts: Vec<&str> = decrypted_message.splitn(3, ':').collect();
                        if parts.len() == 3 {
                            let target_user = parts[1];
                            let dm_content = parts[2];
                            let dm_message = format!("DM_FROM:{}:{}", user.username, dm_content);

                            tracing::info!(
                                "DEBUG: Parsed DM - from: '{}', to: '{}', content: '{}', formatted: '{}'",
                                user.username, target_user, dm_content, dm_message
                            );

                            let mut state_guard = state.lock().await;
                            if state_guard.send_to_user(target_user, &dm_message).await {
                                tracing::info!("DM sent from {} to {}: {}", user.username, target_user, dm_content);
                            } else {
                                tracing::warn!("Failed to send DM from {} to {}: user not found", user.username, target_user);
                            }
                        } else {
                            tracing::error!("DEBUG: Invalid DM format from {}: parts={:?}", user.username, parts);
                        }
                    } else if decrypted_message == "REQUEST_USER_LIST" {
                        // Handle user list request silently - no broadcast needed
                        let mut state_guard = state.lock().await;
                        state_guard.broadcast_user_list().await;
                    } else if decrypted_message.starts_with("SYSTEM:") ||
                             decrypted_message.starts_with("ERROR:") ||
                             decrypted_message.contains("has joined") ||
                             decrypted_message.contains("has left") ||
                             decrypted_message.contains("Welcome") ||
                             decrypted_message.starts_with("Connected to") ||
                             decrypted_message.starts_with("Disconnected from") {
                        // System messages - broadcast without username prefix
                        let mut state_guard = state.lock().await;
                        state_guard.broadcast(addr, &decrypted_message).await;
                    } else {
                        // Regular chat message - broadcast to all users in the Lobby
                        let mut state_guard = state.lock().await;
                        let message = format!("{}: {}", user.username, decrypted_message);
                        state_guard.broadcast(addr, &message).await;
                    }
                }
                // An error occurred.
                Some(Err(e)) => {
                    tracing::error!(
                        "an error occurred while processing messages for {}; error = {:?}",
                        user.username,
                        e
                    );
                }
                // The stream has been exhausted.
                None => break,
            },
        }
    }

    // If this section is reached it means that the client was disconnected!
    // Let's let everyone still connected know about it.
    {
        let mut state_guard = state.lock().await;
        state_guard.peers.remove(&addr);

        // Remove user from server tracking and broadcast updated user list
        state_guard.remove_connected_user(addr);
        state_guard.broadcast_user_list().await;

        // Cleanup user session
        tracing::info!(
            "Cleaning up session for {} (token: {})",
            user.username,
            session.token
        );
        if let Err(e) = state_guard.auth_service.logout(&session.token).await {
            tracing::error!("Failed to cleanup session for {}: {}", user.username, e);
        } else {
            tracing::info!("Session cleanup successful for {}", user.username);
        }

        let message = format!("{} has left the Lobby", user.username);
        tracing::info!("{}", message);
        state_guard.broadcast(addr, &message).await;
    }

    Ok(())
}

fn encrypt(key: Vec<u8>, data: String) -> String {
    let aes_key = Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher.encrypt(&nonce, data.as_bytes()).unwrap();
    let mut encrypted_data = nonce.to_vec();
    encrypted_data.extend_from_slice(&ciphertext);
    BASE64_STANDARD.encode(&encrypted_data)
}

fn decrypt(key: Vec<u8>, data: String) -> String {
    let encrypted_data = BASE64_STANDARD.decode(data).unwrap();
    let (nonce, ciphertext) = encrypted_data.split_at(12);
    let aes_key = Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Nonce::from_slice(nonce);
    let plaintext = cipher.decrypt(nonce, ciphertext).unwrap();
    String::from_utf8(plaintext).unwrap()
}
