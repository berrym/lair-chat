use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, prelude::*};
use futures::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap, env, error::Error, io, net::SocketAddr, sync::Arc, time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    signal,
    sync::{mpsc, Mutex},
};

use tokio_util::codec::{Framed, LinesCodec};
use tracing_subscriber::fmt::format::FmtSpan;
use x25519_dalek::{EphemeralSecret, PublicKey};

use lair_chat::auth::AuthRequest;

use lair_chat::server::{
    api::{start_api_server, ApiState},
    auth::{Role, UserStatus},
    config::ServerConfig,
    storage::{
        current_timestamp, generate_id, DatabaseConfig, Message, MessageMetadata, MessageType,
        Pagination, Room, RoomPrivacy, RoomRole, RoomSettings, RoomType, StorageManager, User,
        UserProfile, UserRole, UserSettings,
    },
};

/// Shorthand for the transmit half of the message channel.
pub type Tx<T> = mpsc::UnboundedSender<T>;
/// Shorthand for the receive half of the message channel.
pub type Rx<T> = mpsc::UnboundedReceiver<T>;
pub type WriteData = (Vec<u8>, Tx<String>);

/// TCP server statistics for monitoring
#[derive(Debug, Clone)]
pub struct TcpServerStats {
    pub connected_peers: usize,
    pub authenticated_users: usize,
    pub active_rooms: usize,
    pub pending_invitations: usize,
}

/// Data that is shared between all peers in the chat server.
pub struct SharedState {
    peers: HashMap<SocketAddr, WriteData>,
    storage: Arc<StorageManager>,
    connected_users: HashMap<String, ConnectedUser>,
    start_time: std::time::Instant,
}

// PendingInvitation struct removed - using database invitation system

#[derive(Debug, Clone)]
struct ConnectedUser {
    user_id: String,                 // Database user ID
    username: String,                // Cache for performance
    address: SocketAddr,             // Connection-specific
    connected_at: u64,               // Connection-specific
    current_room_id: Option<String>, // Database room ID
}

// Room struct removed - using database Room model from storage::models

impl SharedState {
    fn new(storage: Arc<StorageManager>) -> Self {
        Self {
            peers: HashMap::new(),
            storage,
            connected_users: HashMap::new(),
            start_time: std::time::Instant::now(),
        }
    }

    // Phase 2: Database helper functions
    async fn get_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<User>, lair_chat::server::storage::StorageError> {
        self.storage.users().get_user_by_username(username).await
    }

    async fn get_room_by_name(
        &self,
        room_name: &str,
    ) -> Result<Option<Room>, lair_chat::server::storage::StorageError> {
        self.storage.rooms().get_room_by_name(room_name).await
    }

    async fn get_user_rooms(
        &self,
        user_id: &str,
    ) -> Result<
        Vec<lair_chat::server::storage::RoomMembership>,
        lair_chat::server::storage::StorageError,
    > {
        self.storage
            .rooms()
            .list_user_memberships(user_id, Pagination::default())
            .await
    }

    async fn create_room_in_db(
        &self,
        creator_user_id: &str,
        room_name: &str,
    ) -> Result<Room, lair_chat::server::storage::StorageError> {
        // Check if room exists
        if self.storage.rooms().room_name_exists(room_name).await? {
            return Err(lair_chat::server::storage::StorageError::DuplicateError {
                entity: "Room".to_string(),
                message: format!("Room '{}' already exists", room_name),
            });
        }

        // Create room
        let room = Room {
            id: generate_id(),
            name: room_name.to_string(),
            display_name: room_name.to_string(),
            description: None,
            topic: None,
            room_type: RoomType::Channel,
            privacy: RoomPrivacy::Public,
            settings: RoomSettings::default(),
            created_by: creator_user_id.to_string(),
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            is_active: true,
        };

        let created_room = self.storage.rooms().create_room(room).await?;

        // Add creator as owner
        use lair_chat::server::storage::{current_timestamp, RoomMembership};
        let membership = RoomMembership {
            id: generate_id(),
            room_id: created_room.id.clone(),
            user_id: creator_user_id.to_string(),
            role: RoomRole::Owner,
            joined_at: current_timestamp(),
            last_activity: Some(current_timestamp()),
            is_active: true,
            settings: lair_chat::server::storage::RoomMemberSettings::default(),
        };
        self.storage.rooms().add_room_member(membership).await?;

        Ok(created_room)
    }

    async fn join_room_in_db(
        &self,
        user_id: &str,
        room_id: &str,
    ) -> Result<(), lair_chat::server::storage::StorageError> {
        // Check if user is already a member
        if self
            .storage
            .rooms()
            .is_room_member(room_id, user_id)
            .await?
        {
            return Ok(()); // Already a member
        }

        // Add user as member
        use lair_chat::server::storage::{current_timestamp, RoomMembership};
        let membership = RoomMembership {
            id: generate_id(),
            room_id: room_id.to_string(),
            user_id: user_id.to_string(),
            role: RoomRole::Member,
            joined_at: current_timestamp(),
            last_activity: Some(current_timestamp()),
            is_active: true,
            settings: lair_chat::server::storage::RoomMemberSettings::default(),
        };
        self.storage.rooms().add_room_member(membership).await?;

        Ok(())
    }

    async fn leave_room_in_db(
        &self,
        user_id: &str,
        room_id: &str,
    ) -> Result<(), lair_chat::server::storage::StorageError> {
        self.storage
            .rooms()
            .remove_room_member(room_id, user_id)
            .await
    }

    async fn store_message_in_db(
        &self,
        room_id: &str,
        user_id: &str,
        content: &str,
    ) -> Result<Message, lair_chat::server::storage::StorageError> {
        let message = Message {
            id: generate_id(),
            room_id: room_id.to_string(),
            user_id: user_id.to_string(),
            content: content.to_string(),
            message_type: MessageType::Text,
            timestamp: current_timestamp(),
            edited_at: None,
            parent_message_id: None,
            metadata: MessageMetadata::default(),
            is_deleted: false,
            deleted_at: None,
        };

        self.storage.messages().store_message(message).await
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
        let current_room = if let Some(user) = self.connected_users.get(username) {
            user.current_room_id
                .clone()
                .unwrap_or_else(|| "Lobby".to_string())
        } else {
            "Lobby".to_string()
        };

        let room_status_msg = format!("ROOM_STATUS:{},{}", current_room, username);

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

            if let Some((_key, sender)) = self.peers.get(&user_addr) {
                match sender.send(message.to_string()) {
                    Ok(()) => true,
                    Err(e) => {
                        tracing::error!("Failed to send message to {}: {}", username, e);
                        false
                    }
                }
            } else {
                false
            }
        } else {
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

            // Phase 2: Database operations handled by helper functions
            tracing::info!("User {} disconnected from {}", username, addr);
            self.broadcast_user_list().await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::FULL)
        .init();

    println!("\nLair Chat Integrated Server Starting...");
    println!("==========================================");

    // Initialize TCP stats storage for admin dashboard
    lair_chat::server::api::models::admin::init_tcp_stats();

    // TCP Server Configuration
    let tcp_port = env::var("TCP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);
    let tcp_addr = format!("127.0.0.1:{}", tcp_port);

    // REST API Server Configuration
    let rest_port = env::var("REST_PORT")
        .unwrap_or_else(|_| "8082".to_string())
        .parse::<u16>()
        .unwrap_or(8082);
    let rest_addr = format!("127.0.0.1:{}", rest_port)
        .parse::<SocketAddr>()
        .expect("Invalid REST server address");

    println!("Starting services:");
    println!("   • TCP Chat Server:  {}", tcp_addr);
    println!("   • REST API Server:  http://{}", rest_addr);
    println!("   • Admin Dashboard:  http://{}/admin/", rest_addr);

    // Initialize SHARED database configuration with increased connection pool for dual server access
    let database_config = DatabaseConfig {
        url: env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/lair_chat.db".to_string()),
        max_connections: 20, // Increased for dual server access
        min_connections: 2,  // Minimum for both servers
        connection_timeout: Duration::from_secs(30),
        idle_timeout: Duration::from_secs(300),
        auto_migrate: true,
    };

    // Create SINGLE StorageManager instance shared by both servers
    let storage = Arc::new(
        StorageManager::new(database_config)
            .await
            .expect("Failed to initialize shared storage manager"),
    );

    println!("Shared database initialized successfully");

    // Initialize server configuration
    let config = ServerConfig::default();

    // Generate JWT secret
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let secret: [u8; 32] = rng.gen();
        general_purpose::STANDARD.encode(secret)
    });

    // Create TCP server state using shared storage
    let tcp_state = Arc::new(Mutex::new(SharedState::new(Arc::clone(&storage))));

    // Create REST API state using same shared storage
    let api_state = ApiState::new(Arc::clone(&storage), jwt_secret, Arc::new(config));

    // Create admin user if needed
    tokio::spawn({
        let storage = Arc::clone(&storage);
        async move {
            if let Err(e) = ensure_admin_user(&storage).await {
                tracing::warn!("Failed to create admin user: {}", e);
            }
        }
    });

    // Start REST API server with shared storage
    let rest_server_task = {
        let api_state = api_state.clone();
        tokio::spawn(async move {
            if let Err(e) = start_api_server(rest_addr, api_state).await {
                tracing::error!("REST API server error: {}", e);
            }
        })
    };

    // Start TCP Chat server with shared storage
    let tcp_server_task = {
        let state = Arc::clone(&tcp_state);
        tokio::spawn(async move {
            if let Err(e) = start_tcp_server(state, &tcp_addr).await {
                tracing::error!("TCP server error: {}", e);
            }
        })
    };

    println!("\nBoth servers started successfully with shared database!");
    println!("\nAccess your services:");
    println!(
        "   • TCP Chat (telnet):     telnet {} {}",
        "127.0.0.1", tcp_port
    );
    println!("   • Admin Dashboard:       http://{}/admin/", rest_addr);
    println!("   • REST API:             http://{}/api/v1", rest_addr);
    println!(
        "   • API Health:           http://{}/api/v1/health",
        rest_addr
    );
    println!("\nDefault admin credentials: admin / AdminPassword123!");
    println!("\nPress Ctrl+C to stop both servers");
    println!("==========================================\n");

    // Wait for shutdown signal or server completion
    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("\nShutdown signal received, stopping servers...");
        }
        result = rest_server_task => {
            match result {
                Ok(()) => tracing::info!("REST API server completed"),
                Err(e) => tracing::error!("REST API server task error: {}", e),
            }
        }
        result = tcp_server_task => {
            match result {
                Ok(()) => tracing::info!("TCP server completed"),
                Err(e) => tracing::error!("TCP server task error: {}", e),
            }
        }
    }

    println!("Servers stopped cleanly");
    Ok(())
}

async fn ensure_admin_user(storage: &StorageManager) -> Result<(), Box<dyn Error>> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    // Check if default admin user exists
    let admin_username = env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_password =
        env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "AdminPassword123!".to_string());

    match storage.users().get_user_by_username(&admin_username).await {
        Ok(_) => {
            // Admin user already exists
            tracing::info!("Admin user '{}' already exists", admin_username);
        }
        Err(_) => {
            // Create admin user
            let argon2 = Argon2::default();
            let salt = SaltString::generate(&mut OsRng);
            let password_hash = argon2
                .hash_password(admin_password.as_bytes(), &salt)
                .map_err(|e| format!("Failed to hash password: {}", e))?
                .to_string();

            // Create admin user with proper User struct
            use lair_chat::server::storage::models::{User, UserProfile, UserRole, UserSettings};
            use uuid::Uuid;

            let now_timestamp = chrono::Utc::now().timestamp() as u64;
            let admin_user = User {
                id: Uuid::new_v4().to_string(),
                username: admin_username.clone(),
                email: Some(format!("{}@lair-chat.local", admin_username)),
                password_hash,
                salt: salt.to_string(),
                created_at: now_timestamp,
                updated_at: now_timestamp,
                last_seen: None,
                is_active: true,
                role: UserRole::Admin,
                profile: UserProfile {
                    display_name: Some("Administrator".to_string()),
                    avatar: None,
                    status_message: Some("System Administrator".to_string()),
                    bio: Some("Default admin account".to_string()),
                    timezone: Some("UTC".to_string()),
                    language: Some("en".to_string()),
                    custom_fields: std::collections::HashMap::new(),
                },
                settings: UserSettings::default(),
            };

            storage
                .users()
                .create_user(admin_user)
                .await
                .map_err(|e| format!("Failed to create admin user: {}", e))?;

            tracing::info!("Created admin user: {}", admin_username);
        }
    }

    Ok(())
}

async fn start_tcp_server(
    state: Arc<Mutex<SharedState>>,
    addr: &str,
) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("TCP Chat server listening on: {}", addr);

    // Start stats update task
    let stats_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Ok(shared_state) = stats_state.try_lock() {
                let stats = shared_state.get_stats();

                // Create room user counts map
                // TODO: Phase 2 - Get room user counts from database
                let room_user_counts: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();

                // Convert to shared_types format
                let shared_stats = lair_chat::shared_types::TcpServerStats {
                    connected_peers: stats.connected_peers,
                    authenticated_users: stats.authenticated_users,
                    active_rooms: stats.active_rooms,
                    pending_invitations: stats.pending_invitations,
                    room_user_counts,
                    uptime_seconds: shared_state.start_time.elapsed().as_secs(),
                };
                lair_chat::server::api::models::update_tcp_stats(shared_stats).await;
            }
        }
    });

    loop {
        let (stream, addr) = listener.accept().await?;
        let state = Arc::clone(&state);

        tracing::info!("Accepting TCP connection from: {}", addr);

        tokio::spawn(async move {
            let server_secret_key = EphemeralSecret::random();
            let server_public_key = PublicKey::from(&server_secret_key);

            if let Err(e) = process(state, stream, addr, server_public_key, server_secret_key).await
            {
                tracing::error!("Error processing TCP connection {}: {}", addr, e);
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

                let (username, password, is_registration) = match &auth_request {
                    AuthRequest::Login {
                        username, password, ..
                    } => (username.clone(), password.clone(), false),
                    AuthRequest::Register {
                        username, password, ..
                    } => (username.clone(), password.clone(), true),
                };

                let state_guard = state.lock().await;

                // Convert to the format expected by auth service
                let legacy_auth_request = lair_chat::server::auth::AuthRequest {
                    username: username.clone(),
                    password: password.clone(),
                    fingerprint: match &auth_request {
                        AuthRequest::Login { fingerprint, .. } => fingerprint.clone(),
                        AuthRequest::Register { fingerprint, .. } => fingerprint.clone(),
                    },
                    is_registration,
                };

                let result = if is_registration {
                    // Phase 3: Database authentication
                    match handle_registration(&state_guard.storage, &username, &password).await {
                        Ok(_) => {
                            tracing::info!("User {} registered successfully", username);
                            // Phase 3: Database authentication
                            match handle_login(&state_guard.storage, &username, &password).await {
                                Ok(user) => {
                                    tracing::info!("Auto-login successful for {}", username);
                                    Ok(user)
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
                    // Phase 3: Database authentication
                    match handle_login(&state_guard.storage, &username, &password).await {
                        Ok(user) => {
                            tracing::info!("Login successful for {}", username);
                            Ok(user)
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
                                user_id: authenticated_user.id.to_string(),
                                username: authenticated_user.username.clone(),
                                address: addr,
                                connected_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs(),
                                current_room_id: None, // Start in no specific room, will join lobby via database
                            };
                            state_guard
                                .connected_users
                                .insert(authenticated_user.username.clone(), connected_user);

                            // Phase 3: User authenticated via database
                            tracing::info!(
                                "User {} authenticated via database",
                                authenticated_user.username
                            );

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

                    // Handle room commands
                    if decrypted_message.starts_with("CREATE_ROOM:") {
                        let room_name = decrypted_message
                            .strip_prefix("CREATE_ROOM:")
                            .unwrap()
                            .trim();

                        let mut state_guard = state.lock().await;

                        // Validate room name
                        if room_name.is_empty() {
                            let error_msg = "ROOM_ERROR:Room name cannot be empty";
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(error_msg.to_string());
                            }
                        } else if room_name.to_lowercase() == "lobby" {
                            let error_msg =
                                "ROOM_ERROR:Cannot create a room named 'Lobby' (reserved)";
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(error_msg.to_string());
                            }
                        } else {
                            // TODO: Phase 4 - Check if room exists in database
                            // For now, assume room doesn't exist and create it

                            // TODO: Phase 4 - Create room in database
                            tracing::info!(
                                "Creating room '{}' - TODO: implement database version",
                                room_name
                            );

                            // Move user to the new room (updates in-memory connection state)
                            if let Some(user) = state_guard
                                .connected_users
                                .get_mut(&authenticated_user.username)
                            {
                                user.current_room_id = Some(room_name.to_string());
                                tracing::info!(
                                    "User {} created and joined room '{}'",
                                    authenticated_user.username,
                                    room_name
                                );

                                // Send success responses
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(format!("ROOM_CREATED:{}", room_name));
                                    let _ = sender.send(format!("CURRENT_ROOM:{}", room_name));
                                }

                                // Broadcast room status update
                                state_guard
                                    .broadcast_room_status(&authenticated_user.username)
                                    .await;
                            } else {
                                let error_msg = "ROOM_ERROR:Failed to join created room";
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg.to_string());
                                }
                            }
                        }
                    } else if decrypted_message.starts_with("JOIN_ROOM:") {
                        let room_name =
                            decrypted_message.strip_prefix("JOIN_ROOM:").unwrap().trim();

                        let mut state_guard = state.lock().await;

                        // TODO: Phase 4 - Check if room exists in database
                        // For now, assume room exists
                        if let Some(user) = state_guard
                            .connected_users
                            .get_mut(&authenticated_user.username)
                        {
                            user.current_room_id = Some(room_name.to_string());
                            tracing::info!(
                                "User {} joined room '{}'",
                                authenticated_user.username,
                                room_name
                            );

                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(format!("ROOM_JOINED:{}", room_name));
                                let _ = sender.send(format!("CURRENT_ROOM:{}", room_name));
                            }

                            state_guard
                                .broadcast_room_status(&authenticated_user.username)
                                .await;
                        } else {
                            let error_msg =
                                format!("ROOM_ERROR:Failed to join room '{}'", room_name);
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(error_msg);
                            }
                        }
                    } else if decrypted_message == "LEAVE_ROOM" {
                        let mut state_guard = state.lock().await;

                        let current_room = if let Some(user) = state_guard
                            .connected_users
                            .get(&authenticated_user.username)
                        {
                            user.current_room_id
                                .clone()
                                .unwrap_or_else(|| "Lobby".to_string())
                        } else {
                            "Lobby".to_string()
                        };

                        if current_room != "Lobby" {
                            // TODO: Phase 4 - Move user to lobby in database
                            if let Some(user) = state_guard
                                .connected_users
                                .get_mut(&authenticated_user.username)
                            {
                                user.current_room_id = None; // No specific room (effectively lobby)
                                tracing::info!(
                                    "User {} left room '{}' and returned to Lobby",
                                    authenticated_user.username,
                                    current_room
                                );

                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(format!("ROOM_LEFT:{}", current_room));
                                    let _ = sender.send("CURRENT_ROOM:Lobby".to_string());
                                }

                                state_guard
                                    .broadcast_room_status(&authenticated_user.username)
                                    .await;
                            } else {
                                let error_msg = "ROOM_ERROR:Failed to return to Lobby";
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg.to_string());
                                }
                            }
                        }
                    } else if decrypted_message == "LIST_ROOMS" {
                        let state_guard = state.lock().await;
                        // TODO: Phase 4 - Get room list from database
                        let room_names: Vec<String> = Vec::new();

                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                            let _ = sender.send(format!("ROOM_LIST:{}", room_names.join(",")));
                        }
                    } else if decrypted_message.starts_with("DM:") {
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

                            let mut state_guard = state.lock().await;

                            // Send to recipient as PRIVATE_MESSAGE
                            let private_message = format!(
                                "PRIVATE_MESSAGE:{}:{}",
                                authenticated_user.username, dm_content
                            );
                            let sent = state_guard
                                .send_to_user(target_username, &private_message)
                                .await;

                            // Send confirmation back to sender
                            if sent {
                                let confirmation = format!(
                                    "SYSTEM_MESSAGE:DM sent to {}: {}",
                                    target_username, dm_content
                                );
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(confirmation);
                                }
                            } else {
                                let error_msg = format!(
                                    "SYSTEM_MESSAGE:ERROR: User {} is not online or not found",
                                    target_username
                                );
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                            }
                        }
                    } else if decrypted_message.starts_with("INVITE_USER:") {
                        let parts: Vec<&str> = decrypted_message
                            .strip_prefix("INVITE_USER:")
                            .unwrap()
                            .splitn(2, ':')
                            .collect();

                        if parts.len() == 2 {
                            let target_username = parts[0];
                            let room_name = parts[1];

                            tracing::info!(
                                "Processing invitation from {} to {} for room '{}'",
                                authenticated_user.username,
                                target_username,
                                room_name
                            );

                            let mut state_guard = state.lock().await;

                            // TODO: Phase 4 - Check if room exists in database
                            // For now, assume room exists and proceed
                            let error_msg = format!(
                                "SYSTEM_MESSAGE:TODO: Room invitation to '{}' - database implementation needed",
                                room_name
                            );
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(error_msg);
                            }

                            // TODO: Phase 6 - Add pending invitation to database
                            tracing::info!("Would add invitation from {} to {} for room {} - TODO: implement database version",
                                authenticated_user.username, target_username, room_name);

                            // Send invitation to target user
                            let invitation_message = format!(
                                "SYSTEM_MESSAGE:{} invited you to join room '{}'",
                                authenticated_user.username, room_name
                            );
                            let sent = state_guard
                                .send_to_user(target_username, &invitation_message)
                                .await;

                            // Send confirmation to inviter
                            if sent {
                                let confirmation = format!(
                                    "SYSTEM_MESSAGE:You invited {} to join room '{}'",
                                    target_username, room_name
                                );
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(confirmation);
                                }
                            } else {
                                let error_msg = format!(
                                    "SYSTEM_MESSAGE:ERROR: User {} is not online",
                                    target_username
                                );
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                            }
                        }
                    } else if decrypted_message.starts_with("ACCEPT_INVITATION:") {
                        let room_param = decrypted_message
                            .strip_prefix("ACCEPT_INVITATION:")
                            .unwrap()
                            .trim();

                        let mut state_guard = state.lock().await;

                        // TODO: Phase 6 - Get invitation from database
                        let room_name = if room_param == "LATEST" {
                            // Placeholder - would get most recent invitation from database
                            let error_msg =
                                "SYSTEM_MESSAGE:ERROR: Invitation acceptance not implemented yet - database needed";
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(error_msg.to_string());
                            }
                            continue; // Skip the rest of this handler
                        } else {
                            room_param.to_string()
                        };

                        // TODO: Phase 6 - Check and remove invitation from database
                        // For now, assume invitation exists and proceed

                        // Move user to the room (updates connection state)
                        if let Some(user) = state_guard
                            .connected_users
                            .get_mut(&authenticated_user.username)
                        {
                            user.current_room_id = Some(room_name.to_string());

                            tracing::info!(
                                "User {} accepted invitation and joined room '{}'",
                                authenticated_user.username,
                                room_name
                            );

                            // Send success messages
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(format!("ROOM_JOINED:{}", room_name));
                                let _ = sender.send(format!("CURRENT_ROOM:{}", room_name));
                            }

                            // Broadcast room status update
                            state_guard
                                .broadcast_room_status(&authenticated_user.username)
                                .await;

                            // Send confirmation message
                            let confirmation_msg =
                                format!("SYSTEM_MESSAGE:You joined room '{}'", room_name);
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(confirmation_msg);
                            }
                        } else {
                            let error_msg = format!(
                                "SYSTEM_MESSAGE:ERROR: Failed to join room '{}'",
                                room_name
                            );
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(error_msg);
                            }
                        }
                    } else if decrypted_message.starts_with("DECLINE_INVITATION:") {
                        let room_param = decrypted_message
                            .strip_prefix("DECLINE_INVITATION:")
                            .unwrap()
                            .trim();

                        let mut state_guard = state.lock().await;

                        // Determine the actual room name
                        // TODO: Phase 6 - Get invitation from database
                        let room_name = if room_param == "LATEST" {
                            // Placeholder - would get most recent invitation from database
                            let error_msg =
                                "SYSTEM_MESSAGE:ERROR: Invitation decline not implemented yet - database needed";
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(error_msg.to_string());
                            }
                            continue; // Skip the rest of this handler
                        } else {
                            room_param.to_string()
                        };

                        // TODO: Phase 6 - Remove invitation from database
                        let confirmation = format!(
                            "SYSTEM_MESSAGE:You declined the invitation to room '{}'",
                            room_name
                        );
                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                            let _ = sender.send(confirmation);
                        }
                    } else if decrypted_message == "LIST_INVITATIONS" {
                        let state_guard = state.lock().await;
                        // TODO: Phase 6 - Get pending invitations from database
                        let invitations: Vec<String> = Vec::new(); // Placeholder

                        if invitations.is_empty() {
                            let msg = "SYSTEM_MESSAGE:No pending invitations";
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(msg.to_string());
                            }
                        } else {
                            // TODO: Phase 6 - List invitations from database
                            let msg = "SYSTEM_MESSAGE:Invitation listing not implemented yet - database needed";
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(msg.to_string());
                            }
                        }
                    } else if decrypted_message == "ACCEPT_ALL_INVITATIONS" {
                        let state_guard = state.lock().await;

                        // TODO: Phase 6 - Accept all invitations from database
                        let msg = "SYSTEM_MESSAGE:Accept all invitations not implemented yet - database needed";
                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                            let _ = sender.send(msg.to_string());
                        }
                    } else if decrypted_message == "SHOW_HELP" {
                        let state_guard = state.lock().await;

                        let help_msg = "SYSTEM_MESSAGE:Available commands:\n\
                            • CREATE_ROOM:<name> - Create a new room\n\
                            • JOIN_ROOM:<name> - Join an existing room\n\
                            • LIST_INVITATIONS - Show pending invitations\n\
                            • ACCEPT_INVITATION:<room> - Accept invitation\n\
                            • DECLINE_INVITATION:<room> - Decline invitation";

                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                            let _ = sender.send(help_msg.to_string());
                        }
                    } else if decrypted_message == "REQUEST_USER_LIST" {
                        let state_guard = state.lock().await;
                        let user_list = state_guard.get_connected_users();
                        let user_list_msg = format!("USER_LIST:{}", user_list.join(","));

                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                            let _ = sender.send(user_list_msg);
                        }
                    } else {
                        // Regular chat message - broadcast to users in same room
                        let current_room = {
                            let state_guard = state.lock().await;
                            if let Some(user) = state_guard
                                .connected_users
                                .get(&authenticated_user.username)
                            {
                                user.current_room_id
                                    .clone()
                                    .unwrap_or_else(|| "Lobby".to_string())
                            } else {
                                "Lobby".to_string()
                            }
                        };

                        let formatted_message =
                            format!("{}: {}", authenticated_user.username, decrypted_message);

                        // Broadcast to users in the same room only
                        let mut state_guard = state.lock().await;
                        // TODO: Phase 4 - Get room users from database
                        let room_users: Vec<String> = Vec::new();

                        for username in &room_users {
                            if username != &authenticated_user.username {
                                let _ =
                                    state_guard.send_to_user(username, &formatted_message).await;
                            }
                        }
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

impl SharedState {
    fn get_stats(&self) -> TcpServerStats {
        TcpServerStats {
            connected_peers: self.peers.len(),
            authenticated_users: self.connected_users.len(),
            active_rooms: 0,        // TODO: Query from database
            pending_invitations: 0, // TODO: Query from database
        }
    }

    // TODO: Phase 2 - These methods will be replaced with database operations
    /*
    fn add_pending_invitation(&mut self, username: &str, invitation: PendingInvitation) {
        // Will be replaced with database storage
    }

    fn get_pending_invitations(&self, username: &str) -> Vec<&PendingInvitation> {
        // Will be replaced with database query
        Vec::new()
    }

    fn remove_pending_invitation(&mut self, username: &str, room_name: &str) -> bool {
        // Will be replaced with database operation
        false
    }

    fn get_latest_invitation(&self, username: &str) -> Option<&PendingInvitation> {
        // Will be replaced with database query
        None
    }


    */
}

// Phase 3: Database-backed authentication functions
async fn handle_registration(
    storage: &StorageManager,
    username: &str,
    password: &str,
) -> Result<(), lair_chat::server::auth::AuthError> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    // Check if username already exists
    if let Ok(Some(_)) = storage.users().get_user_by_username(username).await {
        return Err(lair_chat::server::auth::AuthError::UsernameTaken);
    }

    // Hash the password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| lair_chat::server::auth::AuthError::HashingError(e.to_string()))?
        .to_string();

    // Create new user
    let user = User {
        id: generate_id(),
        username: username.to_string(),
        email: None,
        password_hash,
        salt: salt.to_string(),
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
        last_seen: None,
        is_active: true,
        role: UserRole::User,
        profile: UserProfile::default(),
        settings: UserSettings::default(),
    };

    // Store user in database
    storage
        .users()
        .create_user(user)
        .await
        .map_err(|e| lair_chat::server::auth::AuthError::StorageError(e.to_string()))?;

    tracing::info!("User {} registered successfully", username);
    Ok(())
}

async fn handle_login(
    storage: &StorageManager,
    username: &str,
    password: &str,
) -> Result<lair_chat::server::auth::User, lair_chat::server::auth::AuthError> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    // Get user from database
    let storage_user = storage
        .users()
        .get_user_by_username(username)
        .await
        .map_err(|e| lair_chat::server::auth::AuthError::StorageError(e.to_string()))?
        .ok_or(lair_chat::server::auth::AuthError::UserNotFound)?;

    // Verify password
    let parsed_hash = PasswordHash::new(&storage_user.password_hash)
        .map_err(|e| lair_chat::server::auth::AuthError::HashingError(e.to_string()))?;

    let argon2 = Argon2::default();
    if !argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
    {
        return Err(lair_chat::server::auth::AuthError::InvalidCredentials);
    }

    // Update last seen timestamp
    storage
        .users()
        .update_last_seen(&storage_user.id, current_timestamp())
        .await
        .map_err(|e| lair_chat::server::auth::AuthError::StorageError(e.to_string()))?;

    // Convert storage User to auth User
    let auth_user = lair_chat::server::auth::User {
        id: uuid::Uuid::parse_str(&storage_user.id)
            .map_err(|e| lair_chat::server::auth::AuthError::InternalError(e.to_string()))?,
        username: storage_user.username.clone(),
        password_hash: storage_user.password_hash.clone(),
        roles: match storage_user.role {
            UserRole::Admin => vec![Role::Admin],
            UserRole::Moderator => vec![Role::Moderator],
            UserRole::User => vec![Role::User],
            UserRole::Guest => vec![Role::Guest],
        },
        created_at: storage_user.created_at,
        last_login: current_timestamp(),
        status: if storage_user.is_active {
            UserStatus::Active
        } else {
            UserStatus::Inactive
        },
    };

    tracing::info!("User {} logged in successfully", username);
    Ok(auth_user)
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
