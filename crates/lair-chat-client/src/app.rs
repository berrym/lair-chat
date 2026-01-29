//! Application state and logic.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;

use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::protocol::{
    ClientMessage, Connection, HttpClient, HttpClientConfig, MessageTarget, Room, RoomListItem,
    ServerMessage, Session, TcpError, User,
};

/// Type alias for user IDs.
type UserId = Uuid;

/// Application state.
pub struct App {
    /// Current screen.
    pub screen: Screen,
    /// TCP connection to the server (for real-time messaging).
    pub connection: Option<Connection>,
    /// HTTP client for authentication and queries.
    pub http_client: HttpClient,
    /// Server address (TCP).
    pub server_addr: SocketAddr,
    /// HTTP server URL.
    #[allow(dead_code)]
    pub http_base_url: String,
    /// Current user (after login).
    pub user: Option<User>,
    /// Current session.
    pub session: Option<Session>,
    /// JWT token for authentication.
    pub token: Option<String>,
    /// Rooms the user is a member of (with member counts).
    pub rooms: Vec<RoomListItem>,
    /// Currently selected room (None if viewing DM).
    pub current_room: Option<Room>,
    /// Currently selected DM partner (None if viewing room).
    pub current_dm_user: Option<User>,
    /// Messages for the current view.
    pub messages: Vec<ChatMessage>,
    /// Status message.
    pub status: Option<String>,
    /// Error message.
    pub error: Option<String>,
    /// Should quit.
    pub should_quit: bool,
    /// All known users (excluding self).
    pub all_users: Vec<User>,
    /// Set of user IDs that are currently online.
    pub online_user_ids: HashSet<UserId>,
    /// User cache for username lookups (UserId -> username).
    pub user_cache: HashMap<UserId, String>,
}

/// Screens in the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    /// Login/register screen.
    Login,
    /// Main chat screen.
    Chat,
    /// Room list/selection screen.
    Rooms,
}

/// A chat message for display.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Message ID (if from server).
    #[allow(dead_code)]
    pub id: Option<uuid::Uuid>,
    /// Message author.
    pub author: String,
    /// Message content.
    pub content: String,
    /// When the message was created.
    pub timestamp: DateTime<Utc>,
    /// Whether this is a system message.
    pub is_system: bool,
}

impl ChatMessage {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: None,
            author: "System".to_string(),
            content: content.into(),
            timestamp: Utc::now(),
            is_system: true,
        }
    }

    /// Create a user message.
    pub fn user(author: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: None,
            author: author.into(),
            content: content.into(),
            timestamp: Utc::now(),
            is_system: false,
        }
    }
}

/// Actions that can be performed.
#[derive(Debug, Clone)]
pub enum Action {
    /// Quit the application.
    Quit,
    /// Login with credentials.
    Login { username: String, password: String },
    /// Register new account.
    Register {
        username: String,
        email: String,
        password: String,
    },
    /// Send a chat message.
    SendMessage(String),
    /// Switch to room list.
    ShowRooms,
    /// Join a room.
    JoinRoom(uuid::Uuid),
    /// Switch to a room we're already a member of.
    SwitchToRoom(crate::protocol::Room),
    /// Create a room.
    CreateRoom(String),
    /// Go back to chat.
    BackToChat,
    /// Reconnect to server.
    Reconnect,
    /// Start a DM with a user by username.
    StartDM(String),
    /// Start a DM with a user by index in online_users list.
    StartDMByIndex(usize),
    /// Show help.
    ShowHelp,
    /// Clear error message.
    ClearError,
}

impl App {
    /// Create a new application.
    ///
    /// # Arguments
    /// * `server_addr` - TCP server address for real-time messaging
    /// * `http_port` - HTTP server port for authentication (defaults to TCP port + 2)
    #[allow(dead_code)]
    pub fn new(server_addr: SocketAddr) -> Self {
        Self::with_http_port(server_addr, server_addr.port() + 2)
    }

    /// Create a new application with explicit HTTP port.
    #[allow(dead_code)]
    pub fn with_http_port(server_addr: SocketAddr, http_port: u16) -> Self {
        let http_base_url = format!("http://{}:{}", server_addr.ip(), http_port);
        Self::with_http_config(server_addr, http_base_url, false)
    }

    /// Create a new application with full HTTP configuration.
    ///
    /// # Arguments
    /// * `server_addr` - TCP server address for real-time messaging
    /// * `http_url` - Full HTTP API base URL (e.g., "http://localhost:8082" or "https://localhost:8082")
    /// * `skip_tls_verify` - Skip TLS certificate verification (for self-signed certs)
    pub fn with_http_config(
        server_addr: SocketAddr,
        http_url: String,
        skip_tls_verify: bool,
    ) -> Self {
        let http_config = HttpClientConfig {
            base_url: http_url.clone(),
            skip_tls_verify,
        };
        Self {
            screen: Screen::Login,
            connection: None,
            http_client: HttpClient::with_config(http_config),
            server_addr,
            http_base_url: http_url,
            user: None,
            session: None,
            token: None,
            rooms: Vec::new(),
            current_room: None,
            current_dm_user: None,
            messages: Vec::new(),
            status: None,
            error: None,
            should_quit: false,
            all_users: Vec::new(),
            online_user_ids: HashSet::new(),
            user_cache: HashMap::new(),
        }
    }

    /// Get lists of online and offline users (excluding self).
    pub fn get_user_lists(&self) -> (Vec<String>, Vec<String>) {
        let mut online = Vec::new();
        let mut offline = Vec::new();

        for user in &self.all_users {
            if self.online_user_ids.contains(&user.id) {
                online.push(user.username.clone());
            } else {
                offline.push(user.username.clone());
            }
        }

        (online, offline)
    }

    /// Look up a username by user ID, returning a display string.
    fn get_username(&self, user_id: UserId) -> String {
        // Check if it's the current user
        if let Some(user) = &self.user {
            if user.id == user_id {
                return user.username.clone();
            }
        }

        // Check the cache
        if let Some(username) = self.user_cache.get(&user_id) {
            return username.clone();
        }

        // Check all users list
        if let Some(user) = self.all_users.iter().find(|u| u.id == user_id) {
            return user.username.clone();
        }

        // Fall back to shortened UUID
        let id_str = user_id.to_string();
        if id_str.len() > 8 {
            format!("{}...", &id_str[..8])
        } else {
            id_str
        }
    }

    /// Add a user to the cache.
    fn cache_user(&mut self, user: &User) {
        self.user_cache.insert(user.id, user.username.clone());
    }

    /// Connect to the server.
    pub async fn connect(&mut self) -> Result<(), TcpError> {
        info!("Connecting to {}", self.server_addr);
        self.status = Some("Connecting...".to_string());
        self.error = None;

        let connection = Connection::connect(self.server_addr).await?;
        self.connection = Some(connection);
        self.status = Some("Connected".to_string());
        self.add_system_message("Connected to server");

        Ok(())
    }

    /// Attempt to reconnect to the server.
    pub async fn reconnect(&mut self) {
        // Clear old connection state
        self.connection = None;
        self.status = Some("Reconnecting...".to_string());

        // Attempt to reconnect TCP
        match Connection::connect(self.server_addr).await {
            Ok(connection) => {
                self.connection = Some(connection);
                self.error = None;

                // If we have a valid token, try to re-authenticate
                if let Some(token) = self.token.clone() {
                    self.add_system_message("Reconnected to server. Re-authenticating...");
                    self.authenticate_tcp_connection(token).await;
                } else {
                    self.status = Some("Reconnected".to_string());
                    self.add_system_message("Reconnected to server. Please log in again.");
                    // Reset to login screen since session is lost
                    self.user = None;
                    self.session = None;
                    self.screen = Screen::Login;
                }
            }
            Err(e) => {
                self.error = Some(format!("Reconnect failed: {}", e));
                self.status = Some("Disconnected".to_string());
            }
        }
    }

    /// Disconnect from the server.
    pub async fn disconnect(&mut self) {
        // Logout via HTTP if we have a token
        if self.token.is_some() {
            let _ = self.http_client.logout().await;
        }

        if let Some(conn) = self.connection.take() {
            conn.shutdown().await;
        }
        self.user = None;
        self.session = None;
        self.token = None;
        self.http_client.clear_token();
        self.rooms.clear();
        self.current_room = None;
        self.status = None;
        self.add_system_message("Disconnected from server");
    }

    /// Process an action.
    pub async fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.should_quit = true;
                self.disconnect().await;
            }
            Action::Login { username, password } => {
                self.handle_login(username, password).await;
            }
            Action::Register {
                username,
                email,
                password,
            } => {
                self.handle_register(username, email, password).await;
            }
            Action::SendMessage(content) => {
                self.handle_send_message(content).await;
            }
            Action::ShowRooms => {
                self.screen = Screen::Rooms;
                self.request_room_list().await;
            }
            Action::JoinRoom(room_id) => {
                self.handle_join_room(room_id).await;
            }
            Action::SwitchToRoom(room) => {
                self.status = Some(format!("In room: {}", room.name));
                self.add_system_message(format!("Switched to room: {}", room.name));
                self.current_room = Some(room);
                self.current_dm_user = None;
                self.messages.clear();
                self.screen = Screen::Chat;
                self.error = None;
                // Request message history
                self.request_message_history().await;
            }
            Action::CreateRoom(name) => {
                self.handle_create_room(name).await;
            }
            Action::BackToChat => {
                self.screen = Screen::Chat;
            }
            Action::Reconnect => {
                self.reconnect().await;
            }
            Action::StartDM(username) => {
                self.handle_start_dm(username).await;
            }
            Action::StartDMByIndex(idx) => {
                // Get combined user list (online first, then offline)
                let (online, offline) = self.get_user_lists();
                let combined: Vec<_> = online.iter().chain(offline.iter()).collect();
                if let Some(username) = combined.get(idx) {
                    self.handle_start_dm((*username).clone()).await;
                }
            }
            Action::ShowHelp => {
                self.show_help();
            }
            Action::ClearError => {
                self.error = None;
            }
        }
    }

    /// Show help message.
    fn show_help(&mut self) {
        self.messages.clear();
        self.add_system_message("=== Lair Chat Help ===");
        self.add_system_message("");
        self.add_system_message("NAVIGATION:");
        self.add_system_message("  i        - Enter insert mode to type messages");
        self.add_system_message("  Esc      - Exit insert mode");
        self.add_system_message("  r        - Open room list");
        self.add_system_message("  j/k      - Scroll messages down/up");
        self.add_system_message("  G/g      - Jump to bottom/top of messages");
        self.add_system_message("  q        - Quit application");
        self.add_system_message("  R        - Reconnect to server");
        self.add_system_message("  ?/F1     - Show this help");
        self.add_system_message("");
        self.add_system_message("COMMANDS (type in insert mode):");
        self.add_system_message("  /help              - Show this help");
        self.add_system_message("  /rooms             - Open room list");
        self.add_system_message("  /create <name>     - Create a new room");
        self.add_system_message("  /join <room>       - Join a room (use room list instead)");
        self.add_system_message("  /dm <username>     - Start direct message with user");
        self.add_system_message("  /quit              - Quit application");
        self.add_system_message("");
        self.add_system_message("GETTING STARTED:");
        self.add_system_message("  1. Press 'r' to open rooms and join or create one");
        self.add_system_message("  2. Press 'i' to start typing a message");
        self.add_system_message("  3. Press Enter to send, Esc to cancel");
        self.add_system_message("");
    }

    /// Handle starting a DM with a user.
    async fn handle_start_dm(&mut self, username: String) {
        // Don't DM yourself
        if let Some(user) = &self.user {
            if user.username.eq_ignore_ascii_case(&username) {
                self.error = Some("Cannot send DM to yourself".to_string());
                return;
            }
        }

        // Find the user in all_users
        let target_user = self
            .all_users
            .iter()
            .find(|u| u.username.eq_ignore_ascii_case(&username))
            .cloned();

        if let Some(user) = target_user {
            // Switch to DM view
            self.current_room = None;
            self.current_dm_user = Some(user.clone());
            self.messages.clear();
            self.add_system_message(format!("Direct message with {}", user.username));
            self.screen = Screen::Chat;

            // Request DM history
            self.request_dm_history(user.id).await;
        } else {
            self.error = Some(format!("User '{}' not found", username));
        }
    }

    /// Request DM history with a user.
    async fn request_dm_history(&mut self, recipient: UserId) {
        let Some(conn) = &self.connection else {
            return;
        };

        let msg = ClientMessage::GetMessages {
            request_id: Some(Uuid::new_v4().to_string()),
            target: MessageTarget::DirectMessage { recipient },
            limit: Some(50),
            before: None,
        };

        if let Err(e) = conn.send(msg).await {
            self.error = Some(format!("Failed to request DM history: {}", e));
        }
    }

    /// Handle login via HTTP, then authenticate TCP connection.
    async fn handle_login(&mut self, username: String, password: String) {
        self.status = Some("Logging in via HTTP...".to_string());
        self.error = None;

        // Step 1: HTTP login to get JWT token
        match self.http_client.login(&username, &password).await {
            Ok((user, session, token)) => {
                info!("HTTP login successful");

                // Store user and session info
                self.cache_user(&user);
                self.user = Some(user);
                self.session = Some(session);
                self.token = Some(token.clone());

                // Step 2: Authenticate TCP connection with the token
                self.authenticate_tcp_connection(token).await;
            }
            Err(e) => {
                self.error = Some(format!("Login failed: {}", e));
                self.status = None;
            }
        }
    }

    /// Handle registration via HTTP, then authenticate TCP connection.
    async fn handle_register(&mut self, username: String, email: String, password: String) {
        self.status = Some("Registering via HTTP...".to_string());
        self.error = None;

        // Step 1: HTTP register to get JWT token
        match self
            .http_client
            .register(&username, &email, &password)
            .await
        {
            Ok((user, session, token)) => {
                info!("HTTP registration successful");

                // Store user and session info
                self.cache_user(&user);
                self.user = Some(user);
                self.session = Some(session);
                self.token = Some(token.clone());

                // Step 2: Authenticate TCP connection with the token
                self.authenticate_tcp_connection(token).await;
            }
            Err(e) => {
                self.error = Some(format!("Registration failed: {}", e));
                self.status = None;
            }
        }
    }

    /// Authenticate the TCP connection using a JWT token.
    async fn authenticate_tcp_connection(&mut self, token: String) {
        let Some(conn) = &self.connection else {
            self.error = Some("Not connected to server".to_string());
            return;
        };

        self.status = Some("Authenticating TCP connection...".to_string());

        let msg = ClientMessage::authenticate(&token);

        if let Err(e) = conn.send(msg).await {
            self.error = Some(format!("Failed to authenticate TCP: {}", e));
            self.status = None;
        }

        // Response will be handled in handle_server_message
    }

    /// Handle sending a message.
    async fn handle_send_message(&mut self, content: String) {
        if content.is_empty() {
            return;
        }

        let Some(conn) = &self.connection else {
            self.error = Some("Not connected".to_string());
            return;
        };

        // Determine target: room or DM
        let target = if let Some(room) = &self.current_room {
            MessageTarget::Room { room_id: room.id }
        } else if let Some(dm_user) = &self.current_dm_user {
            MessageTarget::DirectMessage {
                recipient: dm_user.id,
            }
        } else {
            self.error = Some("No room or DM selected".to_string());
            return;
        };

        let msg = ClientMessage::send_message(target, &content);

        if let Err(e) = conn.send(msg).await {
            self.error = Some(format!("Failed to send message: {}", e));
            return;
        }

        // Add message optimistically
        if let Some(user) = &self.user {
            self.messages
                .push(ChatMessage::user(&user.username, &content));
        }
    }

    /// Request room list.
    async fn request_room_list(&mut self) {
        let Some(conn) = &self.connection else {
            return;
        };

        let msg = ClientMessage::ListRooms {
            request_id: Some(Uuid::new_v4().to_string()),
            filter: None,
            limit: Some(50),
            offset: None,
        };

        if let Err(e) = conn.send(msg).await {
            self.error = Some(format!("Failed to request rooms: {}", e));
        }
    }

    /// Request user list to populate cache.
    async fn request_user_list(&mut self) {
        let Some(conn) = &self.connection else {
            return;
        };

        let msg = ClientMessage::ListUsers {
            request_id: Some(Uuid::new_v4().to_string()),
            filter: None,
            limit: Some(100),
            offset: None,
        };

        if let Err(e) = conn.send(msg).await {
            debug!("Failed to request user list: {}", e);
        }
    }

    /// Handle joining a room.
    async fn handle_join_room(&mut self, room_id: uuid::Uuid) {
        let Some(conn) = &self.connection else {
            self.error = Some("Not connected to server".to_string());
            return;
        };

        self.status = Some(format!("Joining room {}...", room_id));

        let msg = ClientMessage::JoinRoom {
            request_id: Some(uuid::Uuid::new_v4().to_string()),
            room_id,
        };

        if let Err(e) = conn.send(msg).await {
            self.error = Some(format!("Failed to join room: {}", e));
        }
    }

    /// Request message history for the current room.
    async fn request_message_history(&mut self) {
        let Some(conn) = &self.connection else {
            return;
        };

        let Some(room) = &self.current_room else {
            return;
        };

        let msg = ClientMessage::GetMessages {
            request_id: Some(uuid::Uuid::new_v4().to_string()),
            target: MessageTarget::Room { room_id: room.id },
            limit: Some(50),
            before: None,
        };

        if let Err(e) = conn.send(msg).await {
            self.error = Some(format!("Failed to request message history: {}", e));
        }
    }

    /// Handle creating a room.
    async fn handle_create_room(&mut self, name: String) {
        let Some(conn) = &self.connection else {
            return;
        };

        let msg = ClientMessage::CreateRoom {
            request_id: Some(uuid::Uuid::new_v4().to_string()),
            name,
            description: None,
            settings: None,
        };

        if let Err(e) = conn.send(msg).await {
            self.error = Some(format!("Failed to create room: {}", e));
        }
    }

    /// Handle a server message.
    async fn handle_server_message(&mut self, msg: ServerMessage) {
        debug!("Handling server message: {:?}", msg);

        match msg {
            // New: Response to JWT-based authentication
            ServerMessage::AuthenticateResponse {
                success,
                user,
                session,
                error,
                ..
            } => {
                if success {
                    self.error = None;
                    // Update user/session if provided (may already be set from HTTP response)
                    if let Some(ref u) = user {
                        self.cache_user(u);
                        self.user = Some(u.clone());
                    }
                    if session.is_some() {
                        self.session = session;
                    }
                    self.screen = Screen::Chat;
                    self.status = Some(format!(
                        "Logged in as {}",
                        self.user
                            .as_ref()
                            .map(|u| &u.username)
                            .unwrap_or(&"?".to_string())
                    ));
                    self.add_system_message("Welcome to Lair Chat!");
                    self.add_system_message("");
                    self.add_system_message("Quick start:");
                    self.add_system_message("  r      - Open rooms list to join or create a room");
                    self.add_system_message(
                        "  Tab    - Switch to Users panel, select user + Enter to DM",
                    );
                    self.add_system_message("  i      - Start typing a message");
                    self.add_system_message("  ?/F1   - Show full help");
                    self.add_system_message("");
                    // Request room list and user list after authentication
                    self.request_room_list().await;
                    self.request_user_list().await;
                } else {
                    let err_msg = error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Authentication failed".to_string());
                    self.error = Some(err_msg);
                    self.status = None;
                    // Clear user/session since auth failed
                    self.user = None;
                    self.session = None;
                    self.token = None;
                }
            }

            // Legacy: Response to direct TCP login (deprecated)
            ServerMessage::LoginResponse {
                success,
                user,
                session,
                error,
                ..
            } => {
                warn!("Received deprecated LoginResponse - client should use HTTP + Authenticate");
                if success {
                    // Clear any previous errors
                    self.error = None;
                    // Cache the logged-in user
                    if let Some(ref u) = user {
                        self.cache_user(u);
                    }
                    self.user = user;
                    self.session = session;
                    self.screen = Screen::Chat;
                    self.status = Some(format!(
                        "Logged in as {}",
                        self.user
                            .as_ref()
                            .map(|u| &u.username)
                            .unwrap_or(&"?".to_string())
                    ));
                    self.add_system_message("Welcome to Lair Chat!");
                    self.add_system_message("");
                    self.add_system_message("Quick start:");
                    self.add_system_message("  r      - Open rooms list to join or create a room");
                    self.add_system_message(
                        "  Tab    - Switch to Users panel, select user + Enter to DM",
                    );
                    self.add_system_message("  i      - Start typing a message");
                    self.add_system_message("  ?/F1   - Show full help");
                    self.add_system_message("");
                    // Request room list and user list after login
                    self.request_room_list().await;
                    self.request_user_list().await;
                } else {
                    let err_msg = error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Login failed".to_string());
                    self.error = Some(err_msg);
                    self.status = None;
                }
            }

            // Legacy: Response to direct TCP register (deprecated)
            ServerMessage::RegisterResponse {
                success,
                user,
                session,
                error,
                ..
            } => {
                warn!(
                    "Received deprecated RegisterResponse - client should use HTTP + Authenticate"
                );
                if success {
                    // Clear any previous errors
                    self.error = None;
                    // Cache the registered user
                    if let Some(ref u) = user {
                        self.cache_user(u);
                    }
                    self.user = user;
                    self.session = session;
                    self.screen = Screen::Chat;
                    self.status = Some(format!(
                        "Registered as {}",
                        self.user
                            .as_ref()
                            .map(|u| &u.username)
                            .unwrap_or(&"?".to_string())
                    ));
                    self.add_system_message("Registration successful! Welcome to Lair Chat!");
                    self.add_system_message("");
                    self.add_system_message("Quick start:");
                    self.add_system_message("  r      - Open rooms list to join or create a room");
                    self.add_system_message(
                        "  Tab    - Switch to Users panel, select user + Enter to DM",
                    );
                    self.add_system_message("  i      - Start typing a message");
                    self.add_system_message("  ?/F1   - Show full help");
                    self.add_system_message("");
                    self.request_room_list().await;
                    self.request_user_list().await;
                } else {
                    let err_msg = error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Registration failed".to_string());
                    self.error = Some(err_msg);
                    self.status = None;
                }
            }

            ServerMessage::SendMessageResponse { success, error, .. } => {
                if !success {
                    let err_msg = error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Failed to send message".to_string());
                    self.error = Some(err_msg);
                }
            }

            ServerMessage::ListRoomsResponse {
                success,
                rooms,
                error,
                ..
            } => {
                if success {
                    self.rooms = rooms;
                    // Don't auto-join - let user choose from room list
                } else {
                    let err_msg = error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Failed to list rooms".to_string());
                    self.error = Some(err_msg);
                }
            }

            ServerMessage::JoinRoomResponse {
                success,
                room,
                error,
                ..
            } => {
                if success {
                    if let Some(room) = room {
                        self.status = Some(format!("In room: {}", room.name));
                        self.add_system_message(format!("Joined room: {}", room.name));
                        self.current_room = Some(room);
                        self.current_dm_user = None; // Clear DM if any
                        self.messages.clear();
                        self.screen = Screen::Chat;
                        // Request message history for the room
                        self.request_message_history().await;
                    } else {
                        self.error = Some("Join succeeded but no room data returned".to_string());
                    }
                } else {
                    let err_msg = error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Failed to join room".to_string());
                    self.error = Some(err_msg);
                }
            }

            ServerMessage::CreateRoomResponse {
                success,
                room,
                error,
                ..
            } => {
                if success {
                    if let Some(room) = room {
                        self.add_system_message(format!("Created room: {}", room.name));
                        // Add as RoomListItem with member_count of 1 (just the creator)
                        self.rooms.push(RoomListItem {
                            room: room.clone(),
                            member_count: 1,
                            is_member: true,
                        });
                        self.current_room = Some(room);
                        self.messages.clear();
                        self.screen = Screen::Chat;
                    }
                } else {
                    let err_msg = error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Failed to create room".to_string());
                    self.error = Some(err_msg);
                }
            }

            ServerMessage::GetMessagesResponse {
                success,
                messages,
                error,
                ..
            } => {
                if success {
                    // Messages come from server newest-first, reverse to display oldest-first
                    let mut messages = messages;
                    messages.reverse();

                    // Convert to ChatMessages
                    let history: Vec<ChatMessage> = messages
                        .into_iter()
                        .map(|msg| {
                            let author = self.get_username(msg.author);

                            ChatMessage {
                                id: Some(msg.id),
                                author,
                                content: msg.content,
                                timestamp: msg.created_at,
                                is_system: false,
                            }
                        })
                        .collect();

                    // Prepend history to existing messages (system messages about joining)
                    let mut new_messages = history;
                    new_messages.append(&mut self.messages);
                    self.messages = new_messages;
                } else {
                    let err_msg = error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Failed to load message history".to_string());
                    self.error = Some(err_msg);
                }
            }

            ServerMessage::MessageReceived {
                message,
                author_username,
            } => {
                // Skip if this is our own message (already added optimistically)
                if self.user.as_ref().map(|u| u.id) == Some(message.author) {
                    return;
                }

                // Cache the author for future lookups
                self.user_cache
                    .insert(message.author, author_username.clone());

                let should_display = match &message.target {
                    // Room message: show if viewing that room
                    MessageTarget::Room { room_id } => {
                        self.current_room.as_ref().map(|r| &r.id) == Some(room_id)
                    }
                    // DM: show if viewing DM with sender or recipient
                    MessageTarget::DirectMessage { recipient } => {
                        if let Some(dm_user) = &self.current_dm_user {
                            // Show if DM is with the sender or recipient
                            dm_user.id == message.author || dm_user.id == *recipient
                        } else {
                            false
                        }
                    }
                };

                if should_display {
                    self.messages.push(ChatMessage {
                        id: Some(message.id),
                        author: author_username,
                        content: message.content,
                        timestamp: message.created_at,
                        is_system: false,
                    });
                } else if let MessageTarget::DirectMessage { .. } = &message.target {
                    // Got a DM while not viewing it - show notification with instructions
                    self.add_system_message(format!(
                        "New DM from {} (Tab to Users, select with j/k, Enter to reply)",
                        author_username
                    ));
                }
            }

            ServerMessage::UserJoinedRoom { room_id, user, .. } => {
                // Always cache the user info
                self.cache_user(&user);
                // Mark user as online
                self.online_user_ids.insert(user.id);
                // Add to all_users if not present
                if !self.all_users.iter().any(|u| u.id == user.id) {
                    self.all_users.push(user.clone());
                }

                if let Some(current) = &self.current_room {
                    if room_id == current.id {
                        self.add_system_message(format!("{} joined the room", user.username));
                    }
                }
            }

            ServerMessage::UserLeftRoom {
                room_id,
                user_id,
                reason,
            } => {
                if let Some(current) = &self.current_room {
                    if room_id == current.id {
                        let username = self.get_username(user_id);
                        self.add_system_message(format!("{} left the room ({})", username, reason));
                    }
                }
            }

            ServerMessage::UserOnline { user_id, username } => {
                // Cache this user and mark as online
                self.user_cache.insert(user_id, username.clone());
                self.online_user_ids.insert(user_id);
                self.add_system_message(format!("{} is now online", username));
            }

            ServerMessage::UserOffline { user_id, username } => {
                self.add_system_message(format!("{} went offline", username));
                self.online_user_ids.remove(&user_id);
            }

            ServerMessage::ServerNotice { message, severity } => {
                self.add_system_message(format!("[{}] {}", severity.to_uppercase(), message));
            }

            ServerMessage::Error { code, message, .. } => {
                self.error = Some(format!("{}: {}", code, message));
            }

            ServerMessage::ListUsersResponse {
                success,
                users,
                online_user_ids,
                ..
            } => {
                if success {
                    // Clear and repopulate all_users list and online status
                    self.all_users.clear();
                    self.online_user_ids.clear();

                    // Parse online user IDs from server
                    for id_str in online_user_ids {
                        if let Ok(id) = Uuid::parse_str(&id_str) {
                            self.online_user_ids.insert(id);
                        }
                    }

                    for user in users {
                        self.cache_user(&user);
                        // Add to all_users if not the current user
                        if self.user.as_ref().map(|u| u.id != user.id).unwrap_or(true) {
                            self.all_users.push(user);
                        }
                    }
                    debug!(
                        "Cached {} users, {} in all_users, {} online",
                        self.user_cache.len(),
                        self.all_users.len(),
                        self.online_user_ids.len()
                    );
                }
            }

            ServerMessage::Pong { .. } => {
                // Keepalive response, ignore
            }

            _ => {
                debug!("Unhandled server message");
            }
        }
    }

    /// Add a system message.
    fn add_system_message(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessage::system(content));
    }

    /// Poll for incoming messages.
    pub async fn poll_messages(&mut self) {
        // Collect messages first to avoid borrow issues
        let mut messages = Vec::new();
        let mut connection_lost = false;

        if let Some(conn) = &mut self.connection {
            // Try to receive messages with a short timeout
            loop {
                match tokio::time::timeout(std::time::Duration::from_millis(10), conn.rx.recv())
                    .await
                {
                    Ok(Some(msg)) => messages.push(msg),
                    Ok(None) => {
                        // Connection closed
                        connection_lost = true;
                        break;
                    }
                    Err(_) => break, // Timeout, no more messages
                }
            }
        }

        // Handle connection loss - attempt automatic reconnection
        if connection_lost {
            self.connection = None;
            self.add_system_message("Connection lost. Attempting to reconnect...");
            self.reconnect().await;
        }

        // Process collected messages
        for msg in messages {
            self.handle_server_message(msg).await;
        }
    }

    /// Send a ping to keep the connection alive.
    pub async fn send_ping(&mut self) {
        if let Some(conn) = &self.connection {
            let _ = conn.send(ClientMessage::Ping).await;
        }
    }
}
