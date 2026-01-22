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
    monitoring::{get_performance_monitor, init_performance_monitor},
    security::{get_security_middleware, init_security_middleware, SecurityMiddleware},
    storage::{
        current_timestamp, generate_id, DatabaseConfig, Invitation, InvitationMetadata,
        InvitationStatus, InvitationType, Message, MessageMetadata, MessageReaction, MessageType,
        Pagination, Room, RoomMemberSettings, RoomMembership, RoomPrivacy, RoomRole, RoomSettings,
        RoomType, StorageManager, User, UserProfile, UserRole, UserSettings,
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

    // Phase 4: Additional room database helpers
    async fn get_room_members(
        &self,
        room_id: &str,
    ) -> Result<Vec<String>, lair_chat::server::storage::StorageError> {
        let memberships = self
            .storage
            .rooms()
            .list_room_members(room_id, Pagination::default())
            .await?;

        let mut usernames = Vec::new();
        for membership in memberships {
            if let Some(user) = self
                .storage
                .users()
                .get_user_by_id(&membership.user_id)
                .await?
            {
                usernames.push(user.username);
            }
        }
        Ok(usernames)
    }

    async fn list_all_rooms(&self) -> Result<Vec<Room>, lair_chat::server::storage::StorageError> {
        self.storage
            .rooms()
            .list_rooms(Pagination::default(), None)
            .await
    }

    async fn user_can_join_room(
        &self,
        user_id: &str,
        room_id: &str,
    ) -> Result<bool, lair_chat::server::storage::StorageError> {
        // Check if room exists
        if self
            .storage
            .rooms()
            .get_room_by_id(room_id)
            .await?
            .is_none()
        {
            return Ok(false);
        }

        // Check if user is already a member
        if self
            .storage
            .rooms()
            .is_room_member(room_id, user_id)
            .await?
        {
            return Ok(true); // Already a member
        }

        // TODO: Add permission checks based on room privacy settings
        // For now, allow joining any existing room
        Ok(true)
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

    async fn get_room_list_from_db(
        &self,
    ) -> Result<Vec<String>, lair_chat::server::storage::StorageError> {
        let rooms = self
            .storage
            .rooms()
            .list_rooms(Pagination::default(), None)
            .await?;
        let room_names: Vec<String> = rooms.into_iter().map(|room| room.name).collect();
        Ok(room_names)
    }

    async fn get_room_members_from_db(
        &self,
        room_id: &str,
    ) -> Result<Vec<String>, lair_chat::server::storage::StorageError> {
        let memberships = self
            .storage
            .rooms()
            .list_room_members(room_id, Pagination::default())
            .await?;
        let mut usernames = Vec::new();
        for membership in memberships {
            if let Some(user) = self
                .storage
                .users()
                .get_user_by_id(&membership.user_id)
                .await?
            {
                usernames.push(user.username);
            }
        }
        Ok(usernames)
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

    // Phase 5: Enhanced Message Storage Helper Functions

    /// Edit an existing message
    async fn edit_message_in_db(
        &self,
        message_id: &str,
        new_content: &str,
        editor_user_id: &str,
    ) -> Result<Message, lair_chat::server::storage::StorageError> {
        // Get the original message to verify permissions
        if let Some(mut message) = self
            .storage
            .messages()
            .get_message_by_id(message_id)
            .await?
        {
            // Check if user can edit this message (only author can edit)
            if message.user_id != editor_user_id {
                return Err(lair_chat::server::storage::StorageError::ValidationError {
                    field: "user_id".to_string(),
                    message: "Permission denied: only message author can edit".to_string(),
                });
            }

            // Update message content and timestamp
            message.content = new_content.to_string();
            message.edited_at = Some(current_timestamp());

            // Save updated message
            self.storage.messages().update_message(message).await
        } else {
            Err(lair_chat::server::storage::StorageError::NotFound {
                entity: "Message".to_string(),
                id: message_id.to_string(),
            })
        }
    }

    /// Delete a message (soft delete)
    async fn delete_message_in_db(
        &self,
        message_id: &str,
        deleter_user_id: &str,
    ) -> Result<(), lair_chat::server::storage::StorageError> {
        // Get the original message to verify permissions
        if let Some(message) = self
            .storage
            .messages()
            .get_message_by_id(message_id)
            .await?
        {
            // Check if user can delete this message (author or room moderator)
            if message.user_id != deleter_user_id {
                // TODO: Add moderator permission check
                return Err(lair_chat::server::storage::StorageError::ValidationError {
                    field: "user_id".to_string(),
                    message: "Permission denied: only message author can delete".to_string(),
                });
            }

            // Soft delete the message
            self.storage
                .messages()
                .delete_message(message_id, current_timestamp())
                .await
        } else {
            Err(lair_chat::server::storage::StorageError::NotFound {
                entity: "Message".to_string(),
                id: message_id.to_string(),
            })
        }
    }

    /// Add a reaction to a message
    async fn add_reaction_to_message(
        &self,
        message_id: &str,
        user_id: &str,
        reaction: &str,
    ) -> Result<(), lair_chat::server::storage::StorageError> {
        let message_reaction = MessageReaction {
            user_id: user_id.to_string(),
            reaction: reaction.to_string(),
            timestamp: current_timestamp(),
        };

        self.storage
            .messages()
            .add_reaction(message_id, message_reaction)
            .await
    }

    /// Remove a reaction from a message
    async fn remove_reaction_from_message(
        &self,
        message_id: &str,
        user_id: &str,
        reaction: &str,
    ) -> Result<(), lair_chat::server::storage::StorageError> {
        self.storage
            .messages()
            .remove_reaction(message_id, user_id, reaction)
            .await
    }

    /// Get message history for a room with pagination
    async fn get_room_message_history(
        &self,
        room_id: &str,
        limit: u64,
        before_message_id: Option<&str>,
    ) -> Result<Vec<Message>, lair_chat::server::storage::StorageError> {
        if let Some(before_id) = before_message_id {
            self.storage
                .messages()
                .get_messages_before(room_id, before_id, limit)
                .await
        } else {
            let pagination = Pagination { limit, offset: 0 };
            self.storage
                .messages()
                .get_room_messages(room_id, pagination, None)
                .await
        }
    }

    /// Search messages in a room
    async fn search_messages_in_room(
        &self,
        room_id: &str,
        query: &str,
        limit: u64,
    ) -> Result<Vec<Message>, lair_chat::server::storage::StorageError> {
        let search_query = lair_chat::server::storage::SearchQuery {
            query: query.to_string(),
            room_id: Some(room_id.to_string()),
            user_id: None,
            message_type: None,
            date_from: None,
            date_to: None,
            limit: Some(limit),
            offset: Some(0),
        };

        let search_result = self
            .storage
            .messages()
            .search_messages(search_query)
            .await?;
        Ok(search_result.messages)
    }

    /// Store a direct message in the database
    async fn store_dm_in_db(
        &self,
        sender_user_id: &str,
        recipient_username: &str,
        content: &str,
    ) -> Result<Message, lair_chat::server::storage::StorageError> {
        // Get recipient user ID from username
        let recipient_user = self.get_user_by_username(recipient_username).await?;
        let recipient_user_id = match recipient_user {
            Some(user) => user.id,
            None => {
                return Err(lair_chat::server::storage::StorageError::NotFound {
                    entity: "User".to_string(),
                    id: recipient_username.to_string(),
                })
            }
        };

        // Create a DM room identifier (consistent ordering for same participants)
        let dm_room_id = if sender_user_id < recipient_user_id.as_str() {
            format!("dm_{}_{}", sender_user_id, recipient_user_id)
        } else {
            format!("dm_{}_{}", recipient_user_id, sender_user_id)
        };

        let message = Message {
            id: generate_id(),
            room_id: dm_room_id,
            user_id: sender_user_id.to_string(),
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

    /// Get DM history between two users
    async fn get_dm_history(
        &self,
        user1_id: &str,
        user2_id: &str,
        limit: u64,
    ) -> Result<Vec<Message>, lair_chat::server::storage::StorageError> {
        // Create consistent DM room identifier
        let dm_room_id = if user1_id < user2_id {
            format!("dm_{}_{}", user1_id, user2_id)
        } else {
            format!("dm_{}_{}", user2_id, user1_id)
        };

        let pagination = Pagination { limit, offset: 0 };
        self.storage
            .messages()
            .get_room_messages(&dm_room_id, pagination, None)
            .await
    }

    /// Mark messages as read for a user
    async fn mark_messages_read(
        &self,
        user_id: &str,
        room_id: &str,
        up_to_message_id: &str,
    ) -> Result<(), lair_chat::server::storage::StorageError> {
        let timestamp = current_timestamp();
        self.storage
            .messages()
            .mark_messages_read(user_id, room_id, up_to_message_id, timestamp)
            .await
    }

    /// Get unread message count for a user in a room
    async fn get_unread_message_count(
        &self,
        user_id: &str,
        room_id: &str,
        since_timestamp: u64,
    ) -> Result<u64, lair_chat::server::storage::StorageError> {
        let unread_messages = self
            .storage
            .messages()
            .get_unread_messages(user_id, room_id, since_timestamp)
            .await?;
        Ok(unread_messages.len() as u64)
    }

    /// Create a threaded reply to a message
    async fn create_threaded_reply(
        &self,
        parent_message_id: &str,
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
            parent_message_id: Some(parent_message_id.to_string()),
            metadata: MessageMetadata::default(),
            is_deleted: false,
            deleted_at: None,
        };

        self.storage.messages().store_message(message).await
    }

    /// Get message thread (replies to a specific message)
    async fn get_message_thread(
        &self,
        parent_message_id: &str,
        limit: u64,
    ) -> Result<Vec<Message>, lair_chat::server::storage::StorageError> {
        let pagination = Pagination { limit, offset: 0 };
        self.storage
            .messages()
            .get_message_thread(parent_message_id, pagination)
            .await
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

    // Initialize security and monitoring middleware
    init_security_middleware().await;
    init_performance_monitor().await;

    println!("Security middleware initialized");
    println!("Performance monitoring initialized");

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

    // Start stats update task with performance monitoring integration
    let stats_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Ok(shared_state) = stats_state.try_lock() {
                let stats = shared_state.get_stats().await;

                // Update performance monitoring system metrics
                let performance_monitor = get_performance_monitor().await;
                let _ = performance_monitor
                    .update_system_metrics(
                        stats.connected_peers as u32,
                        stats.active_rooms as u32,
                        stats.authenticated_users as u32,
                    )
                    .await;

                // Check performance thresholds and raise alerts
                let alerts = performance_monitor.check_thresholds().await;
                if !alerts.is_empty() {
                    tracing::warn!("Performance alerts: {} active alerts", alerts.len());
                    for alert in &alerts {
                        tracing::warn!(
                            "Performance alert: {:?} - {} ({})",
                            alert.alert_type,
                            alert.message,
                            alert.level
                        );
                    }
                }

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

    // Get security middleware for connection monitoring
    let security_middleware = get_security_middleware().await;

    // Log new connection attempt
    security_middleware
        .log_security_event(
            addr.ip(),
            None,
            "connection_attempt",
            format!("New connection from {}", addr),
        )
        .await;

    // Send server public key for handshake
    transport
        .send(BASE64_STANDARD.encode(server_public_key))
        .await?;

    // Receive client public key
    let peer_public_key_string = match transport.next().await {
        Some(Ok(key_string)) => {
            tracing::info!("Got public key from: {}", addr);
            security_middleware
                .log_security_event(
                    addr.ip(),
                    None,
                    "handshake_received",
                    "Client public key received".to_string(),
                )
                .await;
            key_string
        }
        Some(Err(e)) => {
            tracing::error!("Error receiving public key from {}: {}", addr, e);
            security_middleware
                .log_security_event(
                    addr.ip(),
                    None,
                    "handshake_error",
                    format!("Error receiving public key: {}", e),
                )
                .await;
            return Ok(());
        }
        None => {
            tracing::info!("Client {} disconnected during handshake", addr);
            security_middleware
                .log_security_event(
                    addr.ip(),
                    None,
                    "handshake_disconnect",
                    "Client disconnected during handshake".to_string(),
                )
                .await;
            return Ok(());
        }
    };

    // Decode and validate client public key
    let peer_public_key_vec = match BASE64_STANDARD.decode(peer_public_key_string) {
        Ok(decoded) => decoded,
        Err(e) => {
            tracing::error!("Failed to decode base64 public key from {}: {}", addr, e);
            security_middleware
                .record_suspicious_activity(
                    addr.ip(),
                    None,
                    "invalid_key_format",
                    "Failed to decode client public key",
                )
                .await
                .ok();
            return Ok(());
        }
    };

    if peer_public_key_vec.len() != 32 {
        tracing::error!("Invalid public key length from {}", addr);
        security_middleware
            .record_suspicious_activity(
                addr.ip(),
                None,
                "invalid_key_length",
                "Client sent invalid public key length",
            )
            .await
            .ok();
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
                // Check security before processing auth request
                let security_middleware = get_security_middleware().await;

                // Validate request security
                match security_middleware
                    .validate_request(addr.ip(), None, "auth")
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!("Security check failed for {}: {}", addr, e);
                        let _ = transport
                            .send(encrypt(
                                shared_aes256_key.clone(),
                                "Request blocked for security reasons".to_string(),
                            ))
                            .await;
                        continue;
                    }
                }

                let auth_request: AuthRequest =
                    match serde_json::from_str(&decrypt(shared_aes256_key.clone(), message)) {
                        Ok(req) => req,
                        Err(e) => {
                            tracing::error!("Invalid auth request from {}: {}", addr, e);

                            // Record failed authentication attempt for security
                            let _ = security_middleware
                                .record_failed_login(addr.ip(), None)
                                .await;

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
                let _legacy_auth_request = lair_chat::server::auth::AuthRequest {
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

                            // Record failed registration attempt for security
                            let _ = security_middleware
                                .record_failed_login(addr.ip(), Some(username.clone()))
                                .await;

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

                            // Record failed login attempt for security
                            let _ = security_middleware
                                .record_failed_login(addr.ip(), Some(username.clone()))
                                .await;

                            Err(e)
                        }
                    }
                };
                drop(state_guard);

                match result {
                    Ok(authenticated_user) => {
                        // Log successful authentication
                        security_middleware
                            .log_security_event(
                                addr.ip(),
                                Some(authenticated_user.username.clone()),
                                "authentication_success",
                                format!(
                                    "User {} successfully authenticated",
                                    authenticated_user.username
                                ),
                            )
                            .await;

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
                        // Log authentication failure
                        security_middleware
                            .log_security_event(
                                addr.ip(),
                                None,
                                "authentication_failure",
                                format!("Authentication failed: {}", e),
                            )
                            .await;

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
                security_middleware
                    .log_security_event(
                        addr.ip(),
                        None,
                        "auth_message_error",
                        format!("Error reading auth message: {}", e),
                    )
                    .await;
                return Err(e.into());
            }
            None => {
                tracing::info!("Client {} disconnected during authentication", addr);
                security_middleware
                    .log_security_event(
                        addr.ip(),
                        None,
                        "auth_disconnect",
                        "Client disconnected during authentication".to_string(),
                    )
                    .await;
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

                    // Security validation for each command
                    let security_middleware = get_security_middleware().await;
                    let performance_monitor = get_performance_monitor().await;

                    // Check if IP should be blocked
                    if security_middleware.should_block_user(addr.ip()).await {
                        tracing::warn!("Blocking command from suspicious IP: {}", addr.ip());
                        break;
                    }

                    // Advanced threat detection for suspicious patterns
                    let is_suspicious = decrypted_message.len() > 10000 // Extremely long messages
                        || decrypted_message.contains("<script>") // XSS attempts
                        || decrypted_message.contains("javascript:") // JS injection
                        || decrypted_message.contains("SELECT") && decrypted_message.contains("FROM") // SQL injection
                        || decrypted_message.contains("UNION") && decrypted_message.contains("SELECT") // SQL injection
                        || decrypted_message.contains("../") && decrypted_message.contains("..") // Path traversal
                        || decrypted_message.matches("INVITE_USER:").count() > 1 // Command injection
                        || decrypted_message.matches("CREATE_ROOM:").count() > 1 // Command injection
                        || decrypted_message.contains("\x00") // Null byte injection
                        || decrypted_message.contains("<?php") // PHP injection
                        || decrypted_message.contains("exec(") // Code execution
                        || decrypted_message.contains("eval(") // Code execution
                        || decrypted_message.contains("system(") // System command execution
                        || decrypted_message.contains("rm -rf") // Dangerous commands
                        || decrypted_message.contains("DROP TABLE") // Database destruction
                        || decrypted_message.contains("DELETE FROM") && decrypted_message.contains("*"); // Mass deletion

                    if is_suspicious {
                        tracing::warn!(
                            "Suspicious message detected from {}: {}",
                            addr,
                            decrypted_message.chars().take(100).collect::<String>()
                        );

                        // Record suspicious activity
                        let _ = security_middleware
                            .record_suspicious_activity(
                                addr.ip(),
                                Some(authenticated_user.username.clone()),
                                "suspicious_message_pattern",
                                "Detected potentially malicious message content",
                            )
                            .await;

                        // Block the message and potentially the user
                        let _ = transport
                            .send(encrypt(
                                aes_key.clone(),
                                "Message blocked: Suspicious content detected".to_string(),
                            ))
                            .await;
                        continue;
                    }

                    // Validate command request
                    match security_middleware
                        .validate_request(
                            addr.ip(),
                            Some(authenticated_user.username.clone()),
                            "command",
                        )
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => {
                            tracing::warn!(
                                "Security check failed for command from {}: {}",
                                addr,
                                e
                            );
                            continue;
                        }
                    }

                    // Record operation start time for performance monitoring
                    let operation_start = std::time::Instant::now();

                    // Record security event for command processing
                    let command_type = if decrypted_message.starts_with("CREATE_ROOM:") {
                        "create_room"
                    } else if decrypted_message.starts_with("JOIN_ROOM:") {
                        "join_room"
                    } else if decrypted_message == "LEAVE_ROOM" {
                        "leave_room"
                    } else if decrypted_message == "LIST_ROOMS" {
                        "list_rooms"
                    } else if decrypted_message.starts_with("DM:") {
                        "direct_message"
                    } else if decrypted_message.starts_with("INVITE_USER:") {
                        "invite_user"
                    } else {
                        "message_send"
                    };

                    let _ = performance_monitor
                        .record_security_event(
                            "command_processed",
                            &format!(
                                "User {} executed command: {}",
                                authenticated_user.username, command_type
                            ),
                        )
                        .await;

                    // Handle room commands
                    if decrypted_message.starts_with("CREATE_ROOM:") {
                        let room_name = decrypted_message
                            .strip_prefix("CREATE_ROOM:")
                            .unwrap()
                            .trim();

                        // Log security event for room creation
                        let _ = security_middleware
                            .log_security_event(
                                addr.ip(),
                                Some(authenticated_user.username.clone()),
                                "room_creation_attempt",
                                format!("User attempting to create room: {}", room_name),
                            )
                            .await;

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
                            // Phase 4: Create room in database
                            match state_guard
                                .create_room_in_db(&authenticated_user.id.to_string(), room_name)
                                .await
                            {
                                Ok(created_room) => {
                                    // Record successful operation
                                    let _ = performance_monitor
                                        .record_operation("create_room", operation_start.elapsed())
                                        .await;

                                    let _ = security_middleware
                                        .log_security_event(
                                            addr.ip(),
                                            Some(authenticated_user.username.clone()),
                                            "room_created",
                                            format!("Room '{}' created successfully", room_name),
                                        )
                                        .await;
                                    // Move user to the new room (updates in-memory connection state)
                                    if let Some(user) = state_guard
                                        .connected_users
                                        .get_mut(&authenticated_user.username)
                                    {
                                        user.current_room_id = Some(created_room.id.clone());
                                        tracing::info!(
                                            "User {} created and joined room '{}' (ID: {})",
                                            authenticated_user.username,
                                            room_name,
                                            created_room.id
                                        );

                                        // Send success responses
                                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                            let _ =
                                                sender.send(format!("ROOM_CREATED:{}", room_name));
                                            let _ =
                                                sender.send(format!("CURRENT_ROOM:{}", room_name));
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
                                Err(e) => {
                                    // Record error
                                    let _ = performance_monitor
                                        .record_operation_error("create_room", e.to_string())
                                        .await;

                                    let _ = security_middleware
                                        .log_security_event(
                                            addr.ip(),
                                            Some(authenticated_user.username.clone()),
                                            "room_creation_failed",
                                            format!("Failed to create room '{}': {}", room_name, e),
                                        )
                                        .await;

                                    let error_msg = if e.to_string().contains("already exists") {
                                        format!("ROOM_ERROR:Room '{}' already exists", room_name)
                                    } else {
                                        format!("ROOM_ERROR:Failed to create room: {}", e)
                                    };
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    tracing::error!("Failed to create room '{}': {}", room_name, e);
                                }
                            }
                        }
                    } else if decrypted_message.starts_with("JOIN_ROOM:") {
                        let room_name =
                            decrypted_message.strip_prefix("JOIN_ROOM:").unwrap().trim();

                        // Log security event for room join attempt
                        let _ = security_middleware
                            .log_security_event(
                                addr.ip(),
                                Some(authenticated_user.username.clone()),
                                "room_join_attempt",
                                format!("User attempting to join room: {}", room_name),
                            )
                            .await;

                        let mut state_guard = state.lock().await;

                        // Phase 4: Check if room exists in database
                        match state_guard.get_room_by_name(room_name).await {
                            Ok(Some(room)) => {
                                // Room exists, add user to room in database
                                match state_guard
                                    .join_room_in_db(&authenticated_user.id.to_string(), &room.id)
                                    .await
                                {
                                    Ok(()) => {
                                        // Record successful operation
                                        let _ = performance_monitor
                                            .record_operation(
                                                "join_room",
                                                operation_start.elapsed(),
                                            )
                                            .await;

                                        let _ = security_middleware
                                            .log_security_event(
                                                addr.ip(),
                                                Some(authenticated_user.username.clone()),
                                                "room_joined",
                                                format!("Successfully joined room: {}", room_name),
                                            )
                                            .await;

                                        // Update in-memory connection state
                                        if let Some(user) = state_guard
                                            .connected_users
                                            .get_mut(&authenticated_user.username)
                                        {
                                            user.current_room_id = Some(room.id.clone());
                                            tracing::info!(
                                                "User {} joined room '{}' (ID: {})",
                                                authenticated_user.username,
                                                room_name,
                                                room.id
                                            );

                                            if let Some((_key, sender)) =
                                                state_guard.peers.get(&addr)
                                            {
                                                let _ = sender
                                                    .send(format!("ROOM_JOINED:{}", room_name));
                                                let _ = sender
                                                    .send(format!("CURRENT_ROOM:{}", room_name));
                                            }

                                            state_guard
                                                .broadcast_room_status(&authenticated_user.username)
                                                .await;
                                        } else {
                                            let error_msg =
                                                "ROOM_ERROR:Failed to update connection state";
                                            if let Some((_key, sender)) =
                                                state_guard.peers.get(&addr)
                                            {
                                                let _ = sender.send(error_msg.to_string());
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        // Record error
                                        let _ = performance_monitor
                                            .record_operation_error("join_room", e.to_string())
                                            .await;

                                        let _ = security_middleware
                                            .log_security_event(
                                                addr.ip(),
                                                Some(authenticated_user.username.clone()),
                                                "room_join_failed",
                                                format!(
                                                    "Failed to join room '{}': {}",
                                                    room_name, e
                                                ),
                                            )
                                            .await;

                                        let error_msg =
                                            format!("ROOM_ERROR:Failed to join room: {}", e);
                                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                            let _ = sender.send(error_msg);
                                        }
                                        tracing::error!(
                                            "Failed to join room '{}': {}",
                                            room_name,
                                            e
                                        );
                                    }
                                }
                            }
                            Ok(None) => {
                                let error_msg =
                                    format!("ROOM_ERROR:Room '{}' does not exist", room_name);
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                            }
                            Err(e) => {
                                let error_msg =
                                    format!("ROOM_ERROR:Failed to check room existence: {}", e);
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                                tracing::error!(
                                    "Database error checking room '{}': {}",
                                    room_name,
                                    e
                                );
                            }
                        }
                    } else if decrypted_message == "LEAVE_ROOM" {
                        // Log security event for room leave attempt
                        let _ = security_middleware
                            .log_security_event(
                                addr.ip(),
                                Some(authenticated_user.username.clone()),
                                "room_leave_attempt",
                                "User attempting to leave room".to_string(),
                            )
                            .await;

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
                            // Phase 4: Remove user from room in database
                            match state_guard
                                .leave_room_in_db(&authenticated_user.id.to_string(), &current_room)
                                .await
                            {
                                Ok(()) => {
                                    // Record successful operation
                                    let _ = performance_monitor
                                        .record_operation("leave_room", operation_start.elapsed())
                                        .await;

                                    let _ = security_middleware
                                        .log_security_event(
                                            addr.ip(),
                                            Some(authenticated_user.username.clone()),
                                            "room_left",
                                            format!("Successfully left room: {}", current_room),
                                        )
                                        .await;

                                    // Update in-memory connection state
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
                                            let _ =
                                                sender.send(format!("ROOM_LEFT:{}", current_room));
                                            let _ = sender.send("CURRENT_ROOM:Lobby".to_string());
                                        }

                                        state_guard
                                            .broadcast_room_status(&authenticated_user.username)
                                            .await;
                                    } else {
                                        let error_msg =
                                            "ROOM_ERROR:Failed to update connection state";
                                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                            let _ = sender.send(error_msg.to_string());
                                        }
                                    }
                                }
                                Err(e) => {
                                    // Record error
                                    let _ = performance_monitor
                                        .record_operation_error("leave_room", e.to_string())
                                        .await;

                                    let _ = security_middleware
                                        .log_security_event(
                                            addr.ip(),
                                            Some(authenticated_user.username.clone()),
                                            "room_leave_failed",
                                            format!(
                                                "Failed to leave room '{}': {}",
                                                current_room, e
                                            ),
                                        )
                                        .await;

                                    let error_msg =
                                        format!("ROOM_ERROR:Failed to leave room: {}", e);
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    tracing::error!(
                                        "Failed to leave room '{}': {}",
                                        current_room,
                                        e
                                    );
                                }
                            }
                        }
                    } else if decrypted_message == "LIST_ROOMS" {
                        // Log security event for room list request
                        let _ = security_middleware
                            .log_security_event(
                                addr.ip(),
                                Some(authenticated_user.username.clone()),
                                "room_list_request",
                                "User requesting room list".to_string(),
                            )
                            .await;

                        let state_guard = state.lock().await;
                        // Phase 4: Get room list from database
                        match state_guard.get_room_list_from_db().await {
                            Ok(room_names) => {
                                // Record successful operation
                                let _ = performance_monitor
                                    .record_operation("list_rooms", operation_start.elapsed())
                                    .await;

                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ =
                                        sender.send(format!("ROOM_LIST:{}", room_names.join(",")));
                                }
                            }
                            Err(e) => {
                                // Record error
                                let _ = performance_monitor
                                    .record_operation_error("list_rooms", e.to_string())
                                    .await;

                                let error_msg =
                                    format!("ROOM_ERROR:Failed to get room list: {}", e);
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                                tracing::error!("Failed to get room list from database: {}", e);
                            }
                        }
                    } else if decrypted_message.starts_with("DM:") {
                        let dm_start = std::time::Instant::now();
                        let parts: Vec<&str> = decrypted_message[3..].splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let target_username = parts[0];
                            let dm_content = parts[1];

                            // Log security event for DM attempt
                            let _ = security_middleware
                                .log_security_event(
                                    addr.ip(),
                                    Some(authenticated_user.username.clone()),
                                    "dm_attempt",
                                    format!("User sending DM to: {}", target_username),
                                )
                                .await;

                            tracing::info!(
                                "Processing DM from {} to {}: '{}'",
                                authenticated_user.username,
                                target_username,
                                dm_content
                            );

                            // Send to recipient as PRIVATE_MESSAGE
                            // Phase 5: Store DM in database
                            let mut state_guard = state.lock().await;
                            match state_guard
                                .store_dm_in_db(
                                    &authenticated_user.id.to_string(),
                                    target_username,
                                    dm_content,
                                )
                                .await
                            {
                                Ok(_) => {
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

                                        // Record successful operation
                                        let _ = performance_monitor
                                            .record_operation("direct_message", dm_start.elapsed())
                                            .await;

                                        let _ = security_middleware
                                            .log_security_event(
                                                addr.ip(),
                                                Some(authenticated_user.username.clone()),
                                                "dm_sent",
                                                format!("DM sent to {}", target_username),
                                            )
                                            .await;
                                    } else {
                                        let error_msg = format!(
                                            "SYSTEM_MESSAGE:ERROR: User {} is not online or not found",
                                            target_username
                                        );
                                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                            let _ = sender.send(error_msg);
                                        }

                                        // Record error
                                        let _ = performance_monitor
                                            .record_operation_error(
                                                "direct_message",
                                                format!("User {} not online", target_username),
                                            )
                                            .await;
                                    }
                                }
                                Err(e) => {
                                    let error_msg =
                                        format!("SYSTEM_MESSAGE:ERROR: Failed to store DM: {}", e);
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }

                                    // Record error
                                    let _ = performance_monitor
                                        .record_operation_error("direct_message", e.to_string())
                                        .await;
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

                            // Log security event for invitation attempt
                            let _ = security_middleware
                                .log_security_event(
                                    addr.ip(),
                                    Some(authenticated_user.username.clone()),
                                    "invitation_attempt",
                                    format!(
                                        "User inviting {} to room: {}",
                                        target_username, room_name
                                    ),
                                )
                                .await;

                            tracing::info!(
                                "Processing invitation from {} to {} for room '{}'",
                                authenticated_user.username,
                                target_username,
                                room_name
                            );

                            let mut state_guard = state.lock().await;

                            // Check if room exists in database
                            let room_result = state_guard
                                .storage
                                .rooms()
                                .get_room_by_name(room_name)
                                .await;
                            let room = match room_result {
                                Ok(Some(room)) => room,
                                Ok(None) => {
                                    let error_msg = format!(
                                        "SYSTEM_MESSAGE:ERROR: Room '{}' does not exist",
                                        room_name
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                                Err(e) => {
                                    let error_msg = format!(
                                        "SYSTEM_MESSAGE:ERROR: Failed to check room: {}",
                                        e
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                            };

                            // Check if target user exists
                            let target_user = match state_guard
                                .storage
                                .users()
                                .get_user_by_username(target_username)
                                .await
                            {
                                Ok(Some(user)) => user,
                                Ok(None) => {
                                    let error_msg = format!(
                                        "SYSTEM_MESSAGE:ERROR: User '{}' does not exist",
                                        target_username
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                                Err(e) => {
                                    let error_msg = format!(
                                        "SYSTEM_MESSAGE:ERROR: Failed to check user: {}",
                                        e
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                            };

                            // Check if user is already a member of the room
                            let is_member = match state_guard
                                .storage
                                .rooms()
                                .is_room_member(&room.id, &target_user.id)
                                .await
                            {
                                Ok(is_member) => is_member,
                                Err(e) => {
                                    let error_msg = format!(
                                        "SYSTEM_MESSAGE:ERROR: Failed to check membership: {}",
                                        e
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                            };

                            if is_member {
                                let error_msg = format!(
                                    "SYSTEM_MESSAGE:ERROR: User '{}' is already a member of room '{}'",
                                    target_username, room_name
                                );
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                                continue;
                            }

                            // Check if there's already a pending invitation
                            let existing_invitation = match state_guard
                                .storage
                                .invitations()
                                .get_invitation_by_recipient_and_room(
                                    &target_user.id,
                                    &room.id,
                                    Some(InvitationStatus::Pending),
                                )
                                .await
                            {
                                Ok(invitation) => invitation,
                                Err(e) => {
                                    let error_msg = format!(
                                        "SYSTEM_MESSAGE:ERROR: Failed to check existing invitations: {}",
                                        e
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                            };

                            if existing_invitation.is_some() {
                                let error_msg = format!(
                                    "SYSTEM_MESSAGE:ERROR: User '{}' already has a pending invitation to room '{}'",
                                    target_username, room_name
                                );
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                                continue;
                            }

                            // Create invitation in database
                            let invitation = Invitation {
                                id: generate_id(),
                                sender_user_id: authenticated_user.id.to_string(),
                                recipient_user_id: target_user.id.clone(),
                                room_id: room.id.clone(),
                                invitation_type: InvitationType::RoomInvitation,
                                status: InvitationStatus::Pending,
                                message: Some(format!("Join room '{}'", room_name)),
                                created_at: current_timestamp(),
                                expires_at: Some(current_timestamp() + 86400 * 7), // 7 days
                                responded_at: None,
                                metadata: InvitationMetadata::default(),
                            };

                            match state_guard
                                .storage
                                .invitations()
                                .create_invitation(invitation)
                                .await
                            {
                                Ok(_) => {
                                    tracing::info!(
                                        "Created invitation from {} to {} for room {}",
                                        authenticated_user.username,
                                        target_username,
                                        room_name
                                    );
                                }
                                Err(e) => {
                                    let error_msg = format!(
                                        "SYSTEM_MESSAGE:ERROR: Failed to create invitation: {}",
                                        e
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                            }

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
                                // Record successful operation
                                let _ = performance_monitor
                                    .record_operation("send_invitation", operation_start.elapsed())
                                    .await;

                                let _ = security_middleware
                                    .log_security_event(
                                        addr.ip(),
                                        Some(authenticated_user.username.clone()),
                                        "invitation_sent",
                                        format!(
                                            "Invitation sent to {} for room: {}",
                                            target_username, room_name
                                        ),
                                    )
                                    .await;

                                let confirmation = format!(
                                    "SYSTEM_MESSAGE:You invited {} to join room '{}'",
                                    target_username, room_name
                                );
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(confirmation);
                                }
                            } else {
                                // Record error
                                let _ = performance_monitor
                                    .record_operation_error(
                                        "send_invitation",
                                        "User not online".to_string(),
                                    )
                                    .await;

                                let _ = security_middleware
                                    .log_security_event(
                                        addr.ip(),
                                        Some(authenticated_user.username.clone()),
                                        "invitation_failed",
                                        format!(
                                            "Failed to invite {} (not online) to room: {}",
                                            target_username, room_name
                                        ),
                                    )
                                    .await;

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
                        let accept_start = std::time::Instant::now();
                        let room_param = decrypted_message
                            .strip_prefix("ACCEPT_INVITATION:")
                            .unwrap()
                            .trim();

                        // Log security event for invitation acceptance
                        let _ = security_middleware
                            .log_security_event(
                                addr.ip(),
                                Some(authenticated_user.username.clone()),
                                "invitation_accept_attempt",
                                format!("User attempting to accept invitation: {}", room_param),
                            )
                            .await;

                        let mut state_guard = state.lock().await;

                        // Get invitation from database
                        let invitation = if room_param == "LATEST" {
                            // Get most recent pending invitation for this user
                            match state_guard
                                .storage
                                .invitations()
                                .list_user_invitations(
                                    &authenticated_user.id.to_string(),
                                    Some(InvitationStatus::Pending),
                                )
                                .await
                            {
                                Ok(invitations) => {
                                    if invitations.is_empty() {
                                        let error_msg =
                                            "SYSTEM_MESSAGE:ERROR: No pending invitations found";
                                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                            let _ = sender.send(error_msg.to_string());
                                        }
                                        continue;
                                    }
                                    invitations[0].clone()
                                }
                                Err(e) => {
                                    let error_msg = format!(
                                        "SYSTEM_MESSAGE:ERROR: Failed to get invitations: {}",
                                        e
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                            }
                        } else {
                            // Get invitation for specific room
                            match state_guard
                                .storage
                                .rooms()
                                .get_room_by_name(room_param)
                                .await
                            {
                                Ok(Some(room)) => {
                                    match state_guard
                                        .storage
                                        .invitations()
                                        .get_invitation_by_recipient_and_room(
                                            &authenticated_user.id.to_string(),
                                            &room.id,
                                            Some(InvitationStatus::Pending),
                                        )
                                        .await
                                    {
                                        Ok(Some(invitation)) => invitation,
                                        Ok(None) => {
                                            let error_msg = format!(
                                                "SYSTEM_MESSAGE:ERROR: No pending invitation found for room '{}'",
                                                room_param
                                            );
                                            if let Some((_key, sender)) =
                                                state_guard.peers.get(&addr)
                                            {
                                                let _ = sender.send(error_msg);
                                            }
                                            continue;
                                        }
                                        Err(e) => {
                                            let error_msg = format!(
                                                "SYSTEM_MESSAGE:ERROR: Failed to get invitation: {}",
                                                e
                                            );
                                            if let Some((_key, sender)) =
                                                state_guard.peers.get(&addr)
                                            {
                                                let _ = sender.send(error_msg);
                                            }
                                            continue;
                                        }
                                    }
                                }
                                Ok(None) => {
                                    let error_msg = format!(
                                        "SYSTEM_MESSAGE:ERROR: Room '{}' does not exist",
                                        room_param
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                                Err(e) => {
                                    let error_msg =
                                        format!("SYSTEM_MESSAGE:ERROR: Failed to get room: {}", e);
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                            }
                        };

                        // Get room details
                        let room = match state_guard
                            .storage
                            .rooms()
                            .get_room_by_id(&invitation.room_id)
                            .await
                        {
                            Ok(Some(room)) => room,
                            Ok(None) => {
                                let error_msg = "SYSTEM_MESSAGE:ERROR: Room no longer exists";
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg.to_string());
                                }
                                continue;
                            }
                            Err(e) => {
                                let error_msg = format!(
                                    "SYSTEM_MESSAGE:ERROR: Failed to get room details: {}",
                                    e
                                );
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                                continue;
                            }
                        };

                        let room_name = room.name.clone();

                        // Update invitation status to accepted
                        if let Err(e) = state_guard
                            .storage
                            .invitations()
                            .update_invitation_status(
                                &invitation.id,
                                InvitationStatus::Accepted,
                                current_timestamp(),
                            )
                            .await
                        {
                            let error_msg =
                                format!("SYSTEM_MESSAGE:ERROR: Failed to update invitation: {}", e);
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(error_msg);
                            }
                            continue;
                        }

                        // Add user to room membership
                        let membership = RoomMembership {
                            id: generate_id(),
                            room_id: room.id.clone(),
                            user_id: authenticated_user.id.to_string(),
                            role: RoomRole::Member,
                            joined_at: current_timestamp(),
                            last_activity: Some(current_timestamp()),
                            is_active: true,
                            settings: RoomMemberSettings::default(),
                        };
                        if let Err(e) = state_guard
                            .storage
                            .rooms()
                            .add_room_member(membership)
                            .await
                        {
                            let error_msg =
                                format!("SYSTEM_MESSAGE:ERROR: Failed to add user to room: {}", e);
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(error_msg);
                            }
                            continue;
                        }

                        // User state already updated above

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

                            // Record successful operation
                            let _ = performance_monitor
                                .record_operation("accept_invitation", accept_start.elapsed())
                                .await;

                            let _ = security_middleware
                                .log_security_event(
                                    addr.ip(),
                                    Some(authenticated_user.username.clone()),
                                    "invitation_accepted",
                                    format!("User accepted invitation to room '{}'", room_name),
                                )
                                .await;
                        } else {
                            let error_msg = format!(
                                "SYSTEM_MESSAGE:ERROR: Failed to join room '{}'",
                                room_name
                            );
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(error_msg);
                            }

                            // Record error
                            let _ = performance_monitor
                                .record_operation_error(
                                    "accept_invitation",
                                    "Failed to join room after accepting invitation".to_string(),
                                )
                                .await;
                        }
                    } else if decrypted_message.starts_with("DECLINE_INVITATION:") {
                        let decline_start = std::time::Instant::now();
                        let room_param = decrypted_message
                            .strip_prefix("DECLINE_INVITATION:")
                            .unwrap()
                            .trim();

                        let state_guard = state.lock().await;

                        // Get invitation from database
                        let invitation = if room_param == "LATEST" {
                            // Get most recent pending invitation for this user
                            match state_guard
                                .storage
                                .invitations()
                                .list_user_invitations(
                                    &authenticated_user.id.to_string(),
                                    Some(InvitationStatus::Pending),
                                )
                                .await
                            {
                                Ok(invitations) => {
                                    if invitations.is_empty() {
                                        let error_msg =
                                            "SYSTEM_MESSAGE:ERROR: No pending invitations found";
                                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                            let _ = sender.send(error_msg.to_string());
                                        }
                                        continue;
                                    }
                                    invitations[0].clone()
                                }
                                Err(e) => {
                                    let error_msg = format!(
                                        "SYSTEM_MESSAGE:ERROR: Failed to get invitations: {}",
                                        e
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                            }
                        } else {
                            // Get invitation for specific room
                            match state_guard
                                .storage
                                .rooms()
                                .get_room_by_name(room_param)
                                .await
                            {
                                Ok(Some(room)) => {
                                    match state_guard
                                        .storage
                                        .invitations()
                                        .get_invitation_by_recipient_and_room(
                                            &authenticated_user.id.to_string(),
                                            &room.id,
                                            Some(InvitationStatus::Pending),
                                        )
                                        .await
                                    {
                                        Ok(Some(invitation)) => invitation,
                                        Ok(None) => {
                                            let error_msg = format!(
                                                "SYSTEM_MESSAGE:ERROR: No pending invitation found for room '{}'",
                                                room_param
                                            );
                                            if let Some((_key, sender)) =
                                                state_guard.peers.get(&addr)
                                            {
                                                let _ = sender.send(error_msg);
                                            }
                                            continue;
                                        }
                                        Err(e) => {
                                            let error_msg = format!(
                                                "SYSTEM_MESSAGE:ERROR: Failed to get invitation: {}",
                                                e
                                            );
                                            if let Some((_key, sender)) =
                                                state_guard.peers.get(&addr)
                                            {
                                                let _ = sender.send(error_msg);
                                            }
                                            continue;
                                        }
                                    }
                                }
                                Ok(None) => {
                                    let error_msg = format!(
                                        "SYSTEM_MESSAGE:ERROR: Room '{}' does not exist",
                                        room_param
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                                Err(e) => {
                                    let error_msg =
                                        format!("SYSTEM_MESSAGE:ERROR: Failed to get room: {}", e);
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                    continue;
                                }
                            }
                        };

                        // Get room details for the confirmation message
                        let room = match state_guard
                            .storage
                            .rooms()
                            .get_room_by_id(&invitation.room_id)
                            .await
                        {
                            Ok(Some(room)) => room,
                            Ok(None) => {
                                let error_msg = "SYSTEM_MESSAGE:ERROR: Room no longer exists";
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg.to_string());
                                }
                                continue;
                            }
                            Err(e) => {
                                let error_msg = format!(
                                    "SYSTEM_MESSAGE:ERROR: Failed to get room details: {}",
                                    e
                                );
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                                continue;
                            }
                        };

                        // Update invitation status to declined
                        if let Err(e) = state_guard
                            .storage
                            .invitations()
                            .update_invitation_status(
                                &invitation.id,
                                InvitationStatus::Declined,
                                current_timestamp(),
                            )
                            .await
                        {
                            let error_msg =
                                format!("SYSTEM_MESSAGE:ERROR: Failed to update invitation: {}", e);
                            if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                let _ = sender.send(error_msg);
                            }
                            continue;
                        }

                        // Send confirmation message
                        let confirmation = format!(
                            "SYSTEM_MESSAGE:You declined the invitation to room '{}'",
                            room.name
                        );
                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                            let _ = sender.send(confirmation);
                        }

                        // Record successful operation
                        let _ = performance_monitor
                            .record_operation("decline_invitation", decline_start.elapsed())
                            .await;

                        let _ = security_middleware
                            .log_security_event(
                                addr.ip(),
                                Some(authenticated_user.username.clone()),
                                "invitation_declined",
                                format!("User declined invitation to room '{}'", room.name),
                            )
                            .await;
                    } else if decrypted_message == "LIST_INVITATIONS" {
                        let list_invitations_start = std::time::Instant::now();
                        // Log security event for invitation list request
                        let _ = security_middleware
                            .log_security_event(
                                addr.ip(),
                                Some(authenticated_user.username.clone()),
                                "invitation_list_request",
                                "User requesting invitation list".to_string(),
                            )
                            .await;

                        let state_guard = state.lock().await;

                        // Get pending invitations from database
                        match state_guard
                            .storage
                            .invitations()
                            .list_user_invitations(
                                &authenticated_user.id.to_string(),
                                Some(InvitationStatus::Pending),
                            )
                            .await
                        {
                            Ok(invitations) => {
                                if invitations.is_empty() {
                                    let msg = "SYSTEM_MESSAGE:No pending invitations";
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(msg.to_string());
                                    }

                                    // Record successful operation (empty list)
                                    let _ = performance_monitor
                                        .record_operation(
                                            "list_invitations",
                                            list_invitations_start.elapsed(),
                                        )
                                        .await;
                                } else {
                                    // Build invitation list with details
                                    let mut invitation_list = Vec::new();

                                    for invitation in invitations {
                                        // Get room details
                                        match state_guard
                                            .storage
                                            .rooms()
                                            .get_room_by_id(&invitation.room_id)
                                            .await
                                        {
                                            Ok(Some(room)) => {
                                                // Get sender details
                                                match state_guard
                                                    .storage
                                                    .users()
                                                    .get_user_by_id(&invitation.sender_user_id)
                                                    .await
                                                {
                                                    Ok(Some(sender)) => {
                                                        let invitation_info = format!(
                                                            "• Room: '{}' from {} (ID: {})",
                                                            room.name,
                                                            sender.username,
                                                            invitation.id
                                                        );
                                                        invitation_list.push(invitation_info);
                                                    }
                                                    Ok(None) => {
                                                        let invitation_info = format!(
                                                            "• Room: '{}' from unknown user (ID: {})",
                                                            room.name, invitation.id
                                                        );
                                                        invitation_list.push(invitation_info);
                                                    }
                                                    Err(_) => {
                                                        let invitation_info = format!(
                                                            "• Room: '{}' from unknown user (ID: {})",
                                                            room.name, invitation.id
                                                        );
                                                        invitation_list.push(invitation_info);
                                                    }
                                                }
                                            }
                                            Ok(None) => {
                                                let invitation_info = format!(
                                                    "• Room: (deleted) (ID: {})",
                                                    invitation.id
                                                );
                                                invitation_list.push(invitation_info);
                                            }
                                            Err(_) => {
                                                let invitation_info = format!(
                                                    "• Room: (unknown) (ID: {})",
                                                    invitation.id
                                                );
                                                invitation_list.push(invitation_info);
                                            }
                                        }
                                    }

                                    let msg = format!(
                                        "SYSTEM_MESSAGE:Pending invitations:\n{}",
                                        invitation_list.join("\n")
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(msg);
                                    }

                                    // Record successful operation (with invitations)
                                    let _ = performance_monitor
                                        .record_operation(
                                            "list_invitations",
                                            list_invitations_start.elapsed(),
                                        )
                                        .await;
                                }
                            }
                            Err(e) => {
                                let error_msg = format!(
                                    "SYSTEM_MESSAGE:ERROR: Failed to get invitations: {}",
                                    e
                                );
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }

                                // Record error
                                let _ = performance_monitor
                                    .record_operation_error("list_invitations", e.to_string())
                                    .await;
                            }
                        }
                    } else if decrypted_message == "ACCEPT_ALL_INVITATIONS" {
                        let mut state_guard = state.lock().await;

                        // Get all pending invitations for this user
                        match state_guard
                            .storage
                            .invitations()
                            .list_user_invitations(
                                &authenticated_user.id.to_string(),
                                Some(InvitationStatus::Pending),
                            )
                            .await
                        {
                            Ok(invitations) => {
                                if invitations.is_empty() {
                                    let msg = "SYSTEM_MESSAGE:No pending invitations to accept";
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(msg.to_string());
                                    }
                                } else {
                                    let mut accepted_count = 0;
                                    let mut failed_count = 0;
                                    let mut accepted_rooms = Vec::new();

                                    for invitation in invitations {
                                        // Get room details
                                        match state_guard
                                            .storage
                                            .rooms()
                                            .get_room_by_id(&invitation.room_id)
                                            .await
                                        {
                                            Ok(Some(room)) => {
                                                // Check if user is already a member
                                                match state_guard
                                                    .storage
                                                    .rooms()
                                                    .is_room_member(
                                                        &room.id,
                                                        &authenticated_user.id.to_string(),
                                                    )
                                                    .await
                                                {
                                                    Ok(true) => {
                                                        // User is already a member, just update invitation status
                                                        let _ = state_guard
                                                            .storage
                                                            .invitations()
                                                            .update_invitation_status(
                                                                &invitation.id,
                                                                InvitationStatus::Accepted,
                                                                current_timestamp(),
                                                            )
                                                            .await;
                                                        accepted_count += 1;
                                                        accepted_rooms.push(room.name.clone());
                                                    }
                                                    Ok(false) => {
                                                        // Accept invitation and add to room
                                                        let update_result = state_guard
                                                            .storage
                                                            .invitations()
                                                            .update_invitation_status(
                                                                &invitation.id,
                                                                InvitationStatus::Accepted,
                                                                current_timestamp(),
                                                            )
                                                            .await;

                                                        let membership = RoomMembership {
                                                            id: generate_id(),
                                                            room_id: room.id.clone(),
                                                            user_id: authenticated_user
                                                                .id
                                                                .to_string(),
                                                            role: RoomRole::Member,
                                                            joined_at: current_timestamp(),
                                                            last_activity: Some(current_timestamp()),
                                                            is_active: true,
                                                            settings: RoomMemberSettings::default(),
                                                        };
                                                        let member_result = state_guard
                                                            .storage
                                                            .rooms()
                                                            .add_room_member(membership)
                                                            .await;

                                                        if update_result.is_ok()
                                                            && member_result.is_ok()
                                                        {
                                                            accepted_count += 1;
                                                            accepted_rooms.push(room.name.clone());
                                                        } else {
                                                            failed_count += 1;
                                                        }
                                                    }
                                                    Err(_) => {
                                                        failed_count += 1;
                                                    }
                                                }
                                            }
                                            Ok(None) => {
                                                // Room doesn't exist, mark invitation as expired
                                                let _ = state_guard
                                                    .storage
                                                    .invitations()
                                                    .update_invitation_status(
                                                        &invitation.id,
                                                        InvitationStatus::Expired,
                                                        current_timestamp(),
                                                    )
                                                    .await;
                                                failed_count += 1;
                                            }
                                            Err(_) => {
                                                failed_count += 1;
                                            }
                                        }
                                    }

                                    let msg = if failed_count == 0 {
                                        format!(
                                            "SYSTEM_MESSAGE:Accepted {} invitations. Joined rooms: {}",
                                            accepted_count,
                                            accepted_rooms.join(", ")
                                        )
                                    } else {
                                        format!(
                                            "SYSTEM_MESSAGE:Accepted {} invitations, {} failed. Joined rooms: {}",
                                            accepted_count,
                                            failed_count,
                                            accepted_rooms.join(", ")
                                        )
                                    };

                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(msg);
                                    }
                                }
                            }
                            Err(e) => {
                                let error_msg = format!(
                                    "SYSTEM_MESSAGE:ERROR: Failed to get invitations: {}",
                                    e
                                );
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                            }
                        }
                    } else if decrypted_message == "SHOW_HELP" {
                        let state_guard = state.lock().await;

                        let help_msg = "SYSTEM_MESSAGE:Available commands:\n\
                            • CREATE_ROOM:<name> - Create a new room\n\
                            • JOIN_ROOM:<name> - Join an existing room\n\
                            • EDIT_MESSAGE:<id>:<new_content> - Edit your message\n\
                            • DELETE_MESSAGE:<id> - Delete your message\n\
                            • REACT_MESSAGE:<id>:<emoji> - Add reaction to message\n\
                            • UNREACT_MESSAGE:<id>:<emoji> - Remove reaction\n\
                            • SEARCH_MESSAGES:<query> - Search messages in current room\n\
                            • GET_HISTORY:<limit> - Get message history\n\
                            • REPLY_MESSAGE:<id>:<content> - Reply to a message\n\
                            • MARK_READ:<id> - Mark messages as read\n\
                            • LIST_INVITATIONS - Show pending invitations\n\
                            • ACCEPT_INVITATION:<room> - Accept invitation\n\
                            • DECLINE_INVITATION:<room> - Decline invitation";

                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                            let _ = sender.send(help_msg.to_string());
                        }
                    } else if decrypted_message == "REQUEST_USER_LIST" {
                        let user_list_start = std::time::Instant::now();
                        let state_guard = state.lock().await;
                        let user_list = state_guard.get_connected_users();
                        let user_list_msg = format!("USER_LIST:{}", user_list.join(","));

                        if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                            let _ = sender.send(user_list_msg);
                        }

                        // Record successful operation
                        let _ = performance_monitor
                            .record_operation("request_user_list", user_list_start.elapsed())
                            .await;
                    } else if decrypted_message.starts_with("EDIT_MESSAGE:") {
                        // Phase 5: Message editing
                        let parts: Vec<&str> = decrypted_message[13..].splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let message_id = parts[0];
                            let new_content = parts[1];

                            let mut state_guard = state.lock().await;
                            match state_guard
                                .edit_message_in_db(
                                    message_id,
                                    new_content,
                                    &authenticated_user.id.to_string(),
                                )
                                .await
                            {
                                Ok(edited_message) => {
                                    let success_msg =
                                        format!("MESSAGE_EDITED:{}:{}", message_id, new_content);
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(success_msg);
                                    }

                                    // Broadcast edit to room members
                                    let room_id = &edited_message.room_id;
                                    if let Ok(room_members) =
                                        state_guard.get_room_members_from_db(room_id).await
                                    {
                                        let edit_notification = format!(
                                            "MESSAGE_EDITED:{}:{}:{}",
                                            authenticated_user.username, message_id, new_content
                                        );
                                        for username in &room_members {
                                            if username != &authenticated_user.username {
                                                let _ = state_guard
                                                    .send_to_user(username, &edit_notification)
                                                    .await;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let error_msg =
                                        format!("MESSAGE_ERROR:Failed to edit message: {}", e);
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                }
                            }
                        }
                    } else if decrypted_message.starts_with("DELETE_MESSAGE:") {
                        // Phase 5: Message deletion
                        let message_id = decrypted_message
                            .strip_prefix("DELETE_MESSAGE:")
                            .unwrap()
                            .trim();

                        let state_guard = state.lock().await;
                        match state_guard
                            .delete_message_in_db(message_id, &authenticated_user.id.to_string())
                            .await
                        {
                            Ok(_) => {
                                let success_msg = format!("MESSAGE_DELETED:{}", message_id);
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(success_msg);
                                }

                                // TODO: Broadcast deletion to room members
                                tracing::info!(
                                    "Message {} deleted by user {}",
                                    message_id,
                                    authenticated_user.username
                                );
                            }
                            Err(e) => {
                                let error_msg =
                                    format!("MESSAGE_ERROR:Failed to delete message: {}", e);
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                            }
                        }
                    } else if decrypted_message.starts_with("REACT_MESSAGE:") {
                        // Phase 5: Message reactions
                        let parts: Vec<&str> = decrypted_message[14..].splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let message_id = parts[0];
                            let reaction = parts[1];

                            let state_guard = state.lock().await;
                            match state_guard
                                .add_reaction_to_message(
                                    message_id,
                                    &authenticated_user.id.to_string(),
                                    reaction,
                                )
                                .await
                            {
                                Ok(_) => {
                                    let success_msg =
                                        format!("REACTION_ADDED:{}:{}", message_id, reaction);
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(success_msg);
                                    }
                                }
                                Err(e) => {
                                    let error_msg =
                                        format!("MESSAGE_ERROR:Failed to add reaction: {}", e);
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                }
                            }
                        }
                    } else if decrypted_message.starts_with("UNREACT_MESSAGE:") {
                        // Phase 5: Remove message reactions
                        let parts: Vec<&str> = decrypted_message[16..].splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let message_id = parts[0];
                            let reaction = parts[1];

                            let state_guard = state.lock().await;
                            match state_guard
                                .remove_reaction_from_message(
                                    message_id,
                                    &authenticated_user.id.to_string(),
                                    reaction,
                                )
                                .await
                            {
                                Ok(_) => {
                                    let success_msg =
                                        format!("REACTION_REMOVED:{}:{}", message_id, reaction);
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(success_msg);
                                    }
                                }
                                Err(e) => {
                                    let error_msg =
                                        format!("MESSAGE_ERROR:Failed to remove reaction: {}", e);
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                }
                            }
                        }
                    } else if decrypted_message.starts_with("SEARCH_MESSAGES:") {
                        // Phase 5: Message search
                        let search_query = decrypted_message
                            .strip_prefix("SEARCH_MESSAGES:")
                            .unwrap()
                            .trim();

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

                        let state_guard = state.lock().await;
                        match state_guard
                            .search_messages_in_room(&current_room, search_query, 20)
                            .await
                        {
                            Ok(messages) => {
                                let results: Vec<String> = messages
                                    .iter()
                                    .map(|m| format!("{}:{}:{}", m.id, m.user_id, m.content))
                                    .collect();
                                let search_results =
                                    format!("SEARCH_RESULTS:{}", results.join("|"));
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(search_results);
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("MESSAGE_ERROR:Search failed: {}", e);
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                            }
                        }
                    } else if decrypted_message.starts_with("GET_HISTORY:") {
                        // Phase 5: Message history
                        let limit_str = decrypted_message
                            .strip_prefix("GET_HISTORY:")
                            .unwrap()
                            .trim();
                        let limit = limit_str.parse::<u64>().unwrap_or(20);

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

                        let state_guard = state.lock().await;
                        match state_guard
                            .get_room_message_history(&current_room, limit, None)
                            .await
                        {
                            Ok(messages) => {
                                for message in messages {
                                    let history_msg =
                                        format!("HISTORY:{}:{}", message.user_id, message.content);
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(history_msg);
                                    }
                                }
                            }
                            Err(e) => {
                                let error_msg =
                                    format!("MESSAGE_ERROR:Failed to get history: {}", e);
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                            }
                        }
                    } else if decrypted_message.starts_with("REPLY_MESSAGE:") {
                        // Phase 5: Threaded replies
                        let parts: Vec<&str> = decrypted_message[14..].splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let parent_message_id = parts[0];
                            let reply_content = parts[1];

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

                            let mut state_guard = state.lock().await;
                            match state_guard
                                .create_threaded_reply(
                                    parent_message_id,
                                    &current_room,
                                    &authenticated_user.id.to_string(),
                                    reply_content,
                                )
                                .await
                            {
                                Ok(reply_message) => {
                                    let success_msg = format!(
                                        "REPLY_CREATED:{}:{}",
                                        parent_message_id, reply_message.id
                                    );
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(success_msg);
                                    }

                                    // Broadcast reply to room members
                                    let formatted_reply = format!(
                                        "REPLY:{}:{}:{}",
                                        authenticated_user.username,
                                        parent_message_id,
                                        reply_content
                                    );
                                    if let Ok(room_members) =
                                        state_guard.get_room_members_from_db(&current_room).await
                                    {
                                        for username in &room_members {
                                            if username != &authenticated_user.username {
                                                let _ = state_guard
                                                    .send_to_user(username, &formatted_reply)
                                                    .await;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let error_msg =
                                        format!("MESSAGE_ERROR:Failed to create reply: {}", e);
                                    if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                        let _ = sender.send(error_msg);
                                    }
                                }
                            }
                        }
                    } else if decrypted_message.starts_with("MARK_READ:") {
                        // Phase 5: Read receipts
                        let message_id =
                            decrypted_message.strip_prefix("MARK_READ:").unwrap().trim();

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

                        let state_guard = state.lock().await;
                        match state_guard
                            .mark_messages_read(
                                &authenticated_user.id.to_string(),
                                &current_room,
                                message_id,
                            )
                            .await
                        {
                            Ok(_) => {
                                let success_msg = format!("MESSAGES_READ:{}", message_id);
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(success_msg);
                                }
                            }
                            Err(e) => {
                                let error_msg =
                                    format!("MESSAGE_ERROR:Failed to mark as read: {}", e);
                                if let Some((_key, sender)) = state_guard.peers.get(&addr) {
                                    let _ = sender.send(error_msg);
                                }
                            }
                        }
                    } else {
                        // Regular chat message - broadcast to users in same room

                        // Log security event for message sending
                        let _ = security_middleware
                            .log_security_event(
                                addr.ip(),
                                Some(authenticated_user.username.clone()),
                                "message_send",
                                format!(
                                    "User sending message: {}",
                                    decrypted_message.chars().take(50).collect::<String>()
                                ),
                            )
                            .await;

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

                        // Phase 4: Get room users from database and store message
                        if current_room != "Lobby" {
                            // Store message in database for non-lobby rooms
                            match state_guard
                                .store_message_in_db(
                                    &current_room,
                                    &authenticated_user.id.to_string(),
                                    &decrypted_message,
                                )
                                .await
                            {
                                Ok(_) => {
                                    // Record successful operation
                                    let _ = performance_monitor
                                        .record_operation("send_message", operation_start.elapsed())
                                        .await;

                                    let _ = security_middleware
                                        .log_security_event(
                                            addr.ip(),
                                            Some(authenticated_user.username.clone()),
                                            "message_sent",
                                            format!("Message sent to room: {}", current_room),
                                        )
                                        .await;

                                    tracing::info!(
                                        "Message stored in database for room '{}'",
                                        current_room
                                    );
                                }
                                Err(e) => {
                                    // Record error
                                    let _ = performance_monitor
                                        .record_operation_error("send_message", e.to_string())
                                        .await;

                                    let _ = security_middleware
                                        .log_security_event(
                                            addr.ip(),
                                            Some(authenticated_user.username.clone()),
                                            "message_send_failed",
                                            format!("Failed to send message: {}", e),
                                        )
                                        .await;

                                    tracing::error!("Failed to store message in database: {}", e);
                                }
                            }

                            // Get room members from database
                            match state_guard.get_room_members_from_db(&current_room).await {
                                Ok(room_users) => {
                                    for username in &room_users {
                                        if username != &authenticated_user.username {
                                            let _ = state_guard
                                                .send_to_user(username, &formatted_message)
                                                .await;
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to get room members from database: {}",
                                        e
                                    );
                                }
                            }
                        } else {
                            // For lobby, broadcast to all connected users
                            let connected_users = state_guard.get_connected_users();
                            for username in &connected_users {
                                if username != &authenticated_user.username {
                                    let _ = state_guard
                                        .send_to_user(username, &formatted_message)
                                        .await;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error reading message from {}: {}", addr, e);

                    // Log connection error for security monitoring
                    let security_middleware = get_security_middleware().await;
                    security_middleware
                        .log_security_event(
                            addr.ip(),
                            Some(authenticated_user.username.clone()),
                            "connection_error",
                            format!(
                                "Connection error for user {}: {}",
                                authenticated_user.username, e
                            ),
                        )
                        .await;
                    break;
                }
            }
        }

        // Log user disconnection for security monitoring
        let security_middleware = get_security_middleware().await;
        security_middleware
            .log_security_event(
                addr.ip(),
                Some(authenticated_user.username.clone()),
                "user_disconnect",
                format!(
                    "User {} disconnected from {}",
                    authenticated_user.username, addr
                ),
            )
            .await;

        send_task.abort();
        let mut state_guard = state.lock().await;
        state_guard.remove_peer(addr).await;

        // Log cleanup completion
        security_middleware
            .log_security_event(
                addr.ip(),
                Some(authenticated_user.username.clone()),
                "cleanup_completed",
                format!(
                    "Connection cleanup completed for user {}",
                    authenticated_user.username
                ),
            )
            .await;
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
    async fn get_stats(&self) -> TcpServerStats {
        // Get pending invitations count from database
        let pending_invitations = self
            .storage
            .invitations()
            .get_invitation_stats(None)
            .await
            .map(|stats| stats.pending_invitations)
            .unwrap_or(0);

        TcpServerStats {
            connected_peers: self.peers.len(),
            authenticated_users: self.connected_users.len(),
            active_rooms: 0, // TODO: Query from database
            pending_invitations: pending_invitations as usize,
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
