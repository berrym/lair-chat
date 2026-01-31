//! Application state and logic.

use std::collections::{HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Notification severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    /// Informational message.
    Info,
    /// Success message.
    Success,
    /// Warning message.
    #[allow(dead_code)]
    Warning,
    /// Error message.
    Error,
}

impl NotificationLevel {
    /// Get the auto-dismiss duration for this level.
    pub fn auto_dismiss_duration(&self) -> Duration {
        match self {
            NotificationLevel::Info => Duration::from_secs(3),
            NotificationLevel::Success => Duration::from_secs(4),
            NotificationLevel::Warning => Duration::from_secs(6),
            NotificationLevel::Error => Duration::from_secs(8),
        }
    }
}

/// A notification message to display to the user.
#[derive(Debug, Clone)]
pub struct Notification {
    /// The message to display.
    pub message: String,
    /// Severity level.
    pub level: NotificationLevel,
    /// When this notification was created.
    pub created_at: Instant,
}

use crate::protocol::{
    ClientMessage, Connection, HttpClient, HttpClientConfig, Invitation, MessageTarget, Room,
    RoomListItem, RoomMember, ServerMessage, Session, TcpError, User,
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
    /// Notification queue (auto-dismissing messages).
    pub notifications: VecDeque<Notification>,
    /// Should quit.
    pub should_quit: bool,
    /// All known users (excluding self).
    pub all_users: Vec<User>,
    /// Set of user IDs that are currently online.
    pub online_user_ids: HashSet<UserId>,
    /// User cache for username lookups (UserId -> username).
    pub user_cache: HashMap<UserId, String>,
    /// Unread DM counts per user (UserId -> unread count).
    pub unread_dms: HashMap<UserId, u32>,
    /// Pending invitations for the current user.
    pub pending_invitations: Vec<Invitation>,
    /// Current room members (when viewing members overlay).
    pub current_room_members: Vec<RoomMember>,
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
    /// Login with credentials (includes server address for connection).
    Login {
        server: String,
        username: String,
        password: String,
    },
    /// Register new account (includes server address for connection).
    Register {
        server: String,
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
    /// Copy last message to clipboard (handled by main loop with ChatScreen).
    CopyLastMessage,
    /// Show invitations overlay.
    ShowInvitations,
    /// Accept an invitation.
    AcceptInvitation(uuid::Uuid),
    /// Decline an invitation.
    DeclineInvitation(uuid::Uuid),
    /// Invite a user to the current room (for programmatic use).
    #[allow(dead_code)]
    InviteUser {
        user_id: uuid::Uuid,
        room_id: uuid::Uuid,
        message: Option<String>,
    },
    /// Invite a user by index in the users list.
    InviteUserByIndex(usize),
    /// Show room members overlay.
    ShowMembers,
    /// Refresh invitations list.
    RefreshInvitations,
    /// Kick a member from the current room.
    KickMember { user_id: uuid::Uuid },
    /// Change a member's role in the current room.
    ChangeMemberRole { user_id: uuid::Uuid, role: String },
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
            notifications: VecDeque::new(),
            should_quit: false,
            all_users: Vec::new(),
            online_user_ids: HashSet::new(),
            user_cache: HashMap::new(),
            unread_dms: HashMap::new(),
            pending_invitations: Vec::new(),
            current_room_members: Vec::new(),
        }
    }

    /// Add an error notification.
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.notifications.push_back(Notification {
            message: message.into(),
            level: NotificationLevel::Error,
            created_at: Instant::now(),
        });
    }

    /// Add an info notification.
    #[allow(dead_code)]
    pub fn set_info(&mut self, message: impl Into<String>) {
        self.notifications.push_back(Notification {
            message: message.into(),
            level: NotificationLevel::Info,
            created_at: Instant::now(),
        });
    }

    /// Add a success notification.
    #[allow(dead_code)]
    pub fn set_success(&mut self, message: impl Into<String>) {
        self.notifications.push_back(Notification {
            message: message.into(),
            level: NotificationLevel::Success,
            created_at: Instant::now(),
        });
    }

    /// Add a warning notification (more prominent, longer display).
    pub fn set_warning(&mut self, message: impl Into<String>) {
        self.notifications.push_back(Notification {
            message: message.into(),
            level: NotificationLevel::Warning,
            created_at: Instant::now(),
        });
    }

    /// Clear all notifications.
    pub fn clear_notifications(&mut self) {
        self.notifications.clear();
    }

    /// Remove expired notifications (call periodically).
    pub fn tick_notifications(&mut self) {
        let now = Instant::now();
        self.notifications
            .retain(|n| now.duration_since(n.created_at) < n.level.auto_dismiss_duration());
    }

    /// Get the current error message (for backward compatibility).
    pub fn error(&self) -> Option<&str> {
        self.notifications
            .iter()
            .find(|n| n.level == NotificationLevel::Error)
            .map(|n| n.message.as_str())
    }

    /// Get all notifications for rendering toasts.
    pub fn notifications(&self) -> Vec<Notification> {
        self.notifications.iter().cloned().collect()
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

    /// Get the count of pending invitations.
    #[allow(dead_code)]
    pub fn pending_invitation_count(&self) -> usize {
        self.pending_invitations.len()
    }

    /// Get unread DM counts by username.
    pub fn get_unread_dms(&self) -> HashMap<String, u32> {
        let mut result = HashMap::new();
        for (user_id, count) in &self.unread_dms {
            if let Some(user) = self.all_users.iter().find(|u| &u.id == user_id) {
                result.insert(user.username.clone(), *count);
            }
        }
        result
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
        self.clear_notifications();

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
                self.clear_notifications();

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
                self.set_error(format!("Reconnect failed: {}", e));
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
    ///
    /// Returns `true` if the chat context changed (room/DM switch) and the
    /// chat view should scroll to the bottom.
    pub async fn handle_action(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => {
                self.should_quit = true;
                self.disconnect().await;
                false
            }
            Action::Login {
                server,
                username,
                password,
            } => {
                self.handle_login(server, username, password).await;
                false
            }
            Action::Register {
                server,
                username,
                email,
                password,
            } => {
                self.handle_register(server, username, email, password)
                    .await;
                false
            }
            Action::SendMessage(content) => {
                self.handle_send_message(content).await;
                false
            }
            Action::ShowRooms => {
                self.screen = Screen::Rooms;
                self.request_room_list().await;
                false
            }
            Action::JoinRoom(room_id) => {
                self.handle_join_room(room_id).await;
                // JoinRoom triggers SwitchToRoom via server response, so don't scroll here
                false
            }
            Action::SwitchToRoom(room) => {
                self.status = Some(format!("In room: {}", room.name));
                self.add_system_message(format!("Switched to room: {}", room.name));
                self.current_room = Some(room);
                self.current_dm_user = None;
                self.messages.clear();
                self.screen = Screen::Chat;
                self.clear_notifications();
                // Request message history
                self.request_message_history().await;
                true // Context changed - scroll to bottom
            }
            Action::CreateRoom(name) => {
                self.handle_create_room(name).await;
                false
            }
            Action::BackToChat => {
                self.screen = Screen::Chat;
                false
            }
            Action::Reconnect => {
                self.reconnect().await;
                false
            }
            Action::StartDM(username) => {
                self.handle_start_dm(username).await;
                true // Context changed - scroll to bottom
            }
            Action::StartDMByIndex(idx) => {
                // Get combined user list (online first, then offline)
                let (online, offline) = self.get_user_lists();
                let combined: Vec<_> = online.iter().chain(offline.iter()).collect();
                if let Some(username) = combined.get(idx) {
                    self.handle_start_dm((*username).clone()).await;
                    true // Context changed - scroll to bottom
                } else {
                    false
                }
            }
            Action::ShowHelp => {
                // Handled in main.rs via dialog popup
                false
            }
            Action::ClearError => {
                self.clear_notifications();
                false
            }
            Action::CopyLastMessage => {
                // Handled by main loop (requires clipboard from ChatScreen)
                false
            }
            Action::ShowInvitations => {
                // Handled in main.rs - just trigger refresh
                self.request_invitations().await;
                false
            }
            Action::RefreshInvitations => {
                self.request_invitations().await;
                false
            }
            Action::AcceptInvitation(invitation_id) => {
                self.handle_accept_invitation(invitation_id).await;
                true // Context may have changed if we joined a room
            }
            Action::DeclineInvitation(invitation_id) => {
                self.handle_decline_invitation(invitation_id).await;
                false
            }
            Action::InviteUser {
                user_id,
                room_id,
                message,
            } => {
                self.handle_invite_user(user_id, room_id, message).await;
                false
            }
            Action::InviteUserByIndex(idx) => {
                // Get combined user list (online first, then offline)
                let (online, offline) = self.get_user_lists();
                let combined: Vec<_> = online.iter().chain(offline.iter()).collect();
                if let Some(username) = combined.get(idx) {
                    // Find the user ID
                    if let Some(user) = self.all_users.iter().find(|u| &u.username == *username) {
                        if let Some(room) = &self.current_room {
                            let user_id = user.id;
                            let room_id = room.id;
                            self.handle_invite_user(user_id, room_id, None).await;
                        } else {
                            self.set_error("Must be in a room to invite users");
                        }
                    } else {
                        self.set_error("User not found");
                    }
                }
                false
            }
            Action::ShowMembers => {
                // Fetch room members and handled in main.rs
                self.request_room_members().await;
                false
            }
            Action::KickMember { user_id } => {
                self.handle_kick_member(user_id).await;
                false
            }
            Action::ChangeMemberRole { user_id, role } => {
                self.handle_change_member_role(user_id, role).await;
                false
            }
        }
    }

    /// Get the content of the last message (for clipboard copy).
    pub fn last_message_content(&self) -> Option<&str> {
        self.messages.last().map(|m| m.content.as_str())
    }
    /// Handle starting a DM with a user.
    async fn handle_start_dm(&mut self, username: String) {
        // Don't DM yourself
        if let Some(user) = &self.user {
            if user.username.eq_ignore_ascii_case(&username) {
                self.set_error("Cannot send DM to yourself");
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
            // Clear unread count for this user
            self.unread_dms.remove(&user.id);
            self.add_system_message(format!("Direct message with {}", user.username));
            self.screen = Screen::Chat;

            // Request DM history
            self.request_dm_history(user.id).await;
        } else {
            self.set_error(format!("User '{}' not found", username));
        }
    }

    /// Request DM history with a user via HTTP.
    async fn request_dm_history(&mut self, recipient: UserId) {
        let recipient_id = recipient.to_string();
        match self
            .http_client
            .get_messages("direct_message", &recipient_id, Some(50))
            .await
        {
            Ok(response) => {
                // Messages come from server newest-first, reverse to display oldest-first
                let mut messages = response.messages;
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

                // Prepend history to existing messages
                let mut new_messages = history;
                new_messages.append(&mut self.messages);
                self.messages = new_messages;
            }
            Err(e) => {
                self.set_error(format!("Failed to load DM history: {}", e));
            }
        }
    }

    /// Handle login via HTTP, then authenticate TCP connection.
    async fn handle_login(&mut self, server: String, username: String, password: String) {
        self.clear_notifications();

        // Step 1: Parse and update server configuration
        if let Err(e) = self.update_server_config(&server) {
            self.set_error(format!("Invalid server address: {}", e));
            return;
        }

        // Step 2: Connect to the server (TCP)
        self.status = Some("Connecting to server...".to_string());
        if let Err(e) = self.connect().await {
            self.set_error(format!("Connection failed: {}", e));
            self.status = None;
            return;
        }

        // Step 3: HTTP login to get JWT token
        self.status = Some("Logging in...".to_string());
        match self.http_client.login(&username, &password).await {
            Ok((user, session, token)) => {
                info!("HTTP login successful");

                // Store user and session info
                self.cache_user(&user);
                self.user = Some(user);
                self.session = Some(session);
                self.token = Some(token.clone());

                // Step 4: Authenticate TCP connection with the token
                self.authenticate_tcp_connection(token).await;
            }
            Err(e) => {
                self.set_error(format!("Login failed: {}", e));
                self.status = None;
            }
        }
    }

    /// Handle registration via HTTP, then authenticate TCP connection.
    async fn handle_register(
        &mut self,
        server: String,
        username: String,
        email: String,
        password: String,
    ) {
        self.clear_notifications();

        // Step 1: Parse and update server configuration
        if let Err(e) = self.update_server_config(&server) {
            self.set_error(format!("Invalid server address: {}", e));
            return;
        }

        // Step 2: Connect to the server (TCP)
        self.status = Some("Connecting to server...".to_string());
        if let Err(e) = self.connect().await {
            self.set_error(format!("Connection failed: {}", e));
            self.status = None;
            return;
        }

        // Step 3: HTTP register to get JWT token
        self.status = Some("Registering...".to_string());
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

                // Step 4: Authenticate TCP connection with the token
                self.authenticate_tcp_connection(token).await;
            }
            Err(e) => {
                self.set_error(format!("Registration failed: {}", e));
                self.status = None;
            }
        }
    }

    /// Update the server configuration (TCP address and HTTP URL).
    /// Returns an error if the address cannot be parsed.
    fn update_server_config(&mut self, server: &str) -> Result<(), std::net::AddrParseError> {
        let server_addr: SocketAddr = server.parse()?;
        self.server_addr = server_addr;

        // Derive HTTP URL from TCP address (port + 2)
        let http_url = format!("http://{}:{}", server_addr.ip(), server_addr.port() + 2);
        self.http_base_url = http_url.clone();

        // Update HTTP client with new base URL
        let http_config = HttpClientConfig {
            base_url: http_url,
            skip_tls_verify: self.http_client.skip_tls_verify(),
        };
        self.http_client = HttpClient::with_config(http_config);

        Ok(())
    }

    /// Authenticate the TCP connection using a JWT token.
    async fn authenticate_tcp_connection(&mut self, token: String) {
        let Some(conn) = &self.connection else {
            self.set_error("Not connected to server");
            return;
        };

        self.status = Some("Authenticating TCP connection...".to_string());

        let msg = ClientMessage::authenticate(&token);

        if let Err(e) = conn.send(msg).await {
            self.set_error(format!("Failed to authenticate TCP: {}", e));
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
            self.set_error("Not connected");
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
            self.set_error("No room or DM selected");
            return;
        };

        let msg = ClientMessage::send_message(target, &content);

        if let Err(e) = conn.send(msg).await {
            self.set_error(format!("Failed to send message: {}", e));
            return;
        }

        // Add message optimistically
        if let Some(user) = &self.user {
            self.messages
                .push(ChatMessage::user(&user.username, &content));
        }
    }

    /// Request room list via HTTP.
    async fn request_room_list(&mut self) {
        match self.http_client.list_rooms().await {
            Ok(response) => {
                // Convert HTTP response format to our internal format
                self.rooms = response
                    .rooms
                    .into_iter()
                    .map(|item| RoomListItem {
                        room: item.room,
                        member_count: item.member_count,
                        is_member: item.is_member,
                    })
                    .collect();
            }
            Err(e) => {
                self.set_error(format!("Failed to list rooms: {}", e));
            }
        }
    }

    /// Request user list via HTTP to populate cache.
    async fn request_user_list(&mut self) {
        match self.http_client.list_users().await {
            Ok(response) => {
                // Clear and repopulate all_users list and online status
                self.all_users.clear();
                self.online_user_ids.clear();

                for user_with_status in response.users {
                    let user = user_with_status.user;
                    let online = user_with_status.online;

                    // Track online status
                    if online {
                        self.online_user_ids.insert(user.id);
                    }

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
            Err(e) => {
                debug!("Failed to request user list: {}", e);
            }
        }
    }

    /// Handle joining a room via HTTP.
    async fn handle_join_room(&mut self, room_id: uuid::Uuid) {
        self.status = Some(format!("Joining room {}...", room_id));

        match self.http_client.join_room(&room_id.to_string()).await {
            Ok(room) => {
                self.status = Some(format!("In room: {}", room.name));
                self.add_system_message(format!("Joined room: {}", room.name));
                self.current_room = Some(room);
                self.current_dm_user = None;
                self.messages.clear();
                self.screen = Screen::Chat;
                // Request message history for the room
                self.request_message_history().await;
            }
            Err(e) => {
                self.set_error(format!("Failed to join room: {}", e));
            }
        }
    }

    /// Request message history for the current room via HTTP.
    async fn request_message_history(&mut self) {
        let Some(room) = &self.current_room else {
            return;
        };

        let room_id = room.id.to_string();
        match self
            .http_client
            .get_messages("room", &room_id, Some(50))
            .await
        {
            Ok(response) => {
                // Messages come from server newest-first, reverse to display oldest-first
                let mut messages = response.messages;
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
            }
            Err(e) => {
                self.set_error(format!("Failed to load message history: {}", e));
            }
        }
    }

    /// Handle creating a room via HTTP.
    async fn handle_create_room(&mut self, name: String) {
        match self.http_client.create_room(&name, None).await {
            Ok(room) => {
                self.add_system_message(format!("Created room: {}", room.name));
                // Add as RoomListItem with member_count of 1 (just the creator)
                self.rooms.push(RoomListItem {
                    room: room.clone(),
                    member_count: 1,
                    is_member: true,
                });
                self.current_room = Some(room);
                self.current_dm_user = None;
                self.messages.clear();
                self.screen = Screen::Chat;
            }
            Err(e) => {
                self.set_error(format!("Failed to create room: {}", e));
            }
        }
    }

    /// Request pending invitations via HTTP.
    pub async fn request_invitations(&mut self) {
        debug!("Requesting pending invitations via HTTP...");
        match self.http_client.list_invitations().await {
            Ok(response) => {
                self.pending_invitations = response.invitations;
                debug!(
                    "Loaded {} pending invitations via HTTP",
                    self.pending_invitations.len()
                );
            }
            Err(e) => {
                debug!("Failed to load invitations: {}", e);
            }
        }
    }

    /// Handle accepting an invitation.
    async fn handle_accept_invitation(&mut self, invitation_id: Uuid) {
        match self
            .http_client
            .accept_invitation(&invitation_id.to_string())
            .await
        {
            Ok(response) => {
                // Remove from pending invitations
                self.pending_invitations
                    .retain(|inv| inv.id != invitation_id);

                // Use the room returned directly from the response
                self.set_success(format!("Joined room: {}", response.room.name));

                self.current_room = Some(response.room);
                self.current_dm_user = None;
                self.messages.clear();
                self.screen = Screen::Chat;
                self.request_message_history().await;
            }
            Err(e) => {
                self.set_error(format!("Failed to accept invitation: {}", e));
            }
        }
    }

    /// Handle declining an invitation.
    async fn handle_decline_invitation(&mut self, invitation_id: Uuid) {
        match self
            .http_client
            .decline_invitation(&invitation_id.to_string())
            .await
        {
            Ok(_) => {
                // Remove from pending invitations
                self.pending_invitations
                    .retain(|inv| inv.id != invitation_id);
                self.set_info("Invitation declined");
            }
            Err(e) => {
                self.set_error(format!("Failed to decline invitation: {}", e));
            }
        }
    }

    /// Handle inviting a user to a room.
    async fn handle_invite_user(&mut self, user_id: Uuid, room_id: Uuid, message: Option<String>) {
        match self
            .http_client
            .create_invitation(
                &room_id.to_string(),
                &user_id.to_string(),
                message.as_deref(),
            )
            .await
        {
            Ok(_) => {
                // Find the username for a nice message
                let username = self.get_username(user_id);
                self.set_success(format!("Invitation sent to {}", username));
            }
            Err(e) => {
                self.set_error(format!("Failed to send invitation: {}", e));
            }
        }
    }

    /// Request room members via HTTP.
    pub async fn request_room_members(&mut self) {
        let Some(room) = &self.current_room else {
            self.set_error("No room selected");
            return;
        };

        let room_id = room.id.to_string();
        match self.http_client.get_room_members(&room_id).await {
            Ok(response) => {
                self.current_room_members = response.members;
                debug!("Loaded {} room members", self.current_room_members.len());
            }
            Err(e) => {
                self.set_error(format!("Failed to load room members: {}", e));
            }
        }
    }

    /// Handle kicking a member from the current room.
    async fn handle_kick_member(&mut self, user_id: Uuid) {
        let Some(room) = &self.current_room else {
            self.set_error("No room selected");
            return;
        };

        let room_id = room.id.to_string();
        let user_id_str = user_id.to_string();
        let username = self.get_username(user_id);

        match self.http_client.kick_member(&room_id, &user_id_str).await {
            Ok(()) => {
                // Remove the member from local list
                self.current_room_members.retain(|m| m.user_id != user_id);
                self.set_success(format!("Kicked {} from the room", username));
            }
            Err(e) => {
                self.set_error(format!("Failed to kick member: {}", e));
            }
        }
    }

    /// Handle changing a member's role in the current room.
    async fn handle_change_member_role(&mut self, user_id: Uuid, role: String) {
        let Some(room) = &self.current_room else {
            self.set_error("No room selected");
            return;
        };

        let room_id = room.id.to_string();
        let user_id_str = user_id.to_string();
        let username = self.get_username(user_id);

        match self
            .http_client
            .change_member_role(&room_id, &user_id_str, &role)
            .await
        {
            Ok(response) => {
                // Update the member in local list
                if let Some(member) = self
                    .current_room_members
                    .iter_mut()
                    .find(|m| m.user_id == user_id)
                {
                    *member = response.member;
                }
                self.set_success(format!("Changed {}'s role to {}", username, role));
            }
            Err(e) => {
                self.set_error(format!("Failed to change role: {}", e));
            }
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
                    self.clear_notifications();
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
                    // Request room list, user list, and pending invitations after authentication
                    self.request_room_list().await;
                    self.request_user_list().await;
                    self.request_invitations().await;
                    // Show invitation notification if there are any pending
                    if !self.pending_invitations.is_empty() {
                        let count = self.pending_invitations.len();
                        self.set_warning(format!(
                            "You have {} pending invitation{} (I to view)",
                            count,
                            if count == 1 { "" } else { "s" }
                        ));
                    }
                } else {
                    let err_msg = error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Authentication failed".to_string());
                    self.set_error(err_msg);
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
                    self.clear_notifications();
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
                    self.set_error(err_msg);
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
                    self.clear_notifications();
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
                    self.set_error(err_msg);
                    self.status = None;
                }
            }

            ServerMessage::SendMessageResponse { success, error, .. } => {
                if !success {
                    let err_msg = error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Failed to send message".to_string());
                    self.set_error(err_msg);
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
                    self.set_error(err_msg);
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
                        self.set_error("Join succeeded but no room data returned");
                    }
                } else {
                    let err_msg = error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Failed to join room".to_string());
                    self.set_error(err_msg);
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
                    self.set_error(err_msg);
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
                    self.set_error(err_msg);
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
                    // Got a DM while not viewing it - increment unread count
                    *self.unread_dms.entry(message.author).or_insert(0) += 1;
                    let unread = self.unread_dms.get(&message.author).copied().unwrap_or(0);
                    // Show notification with unread count
                    self.add_system_message(format!(
                        "New DM from {} ({} unread) - Tab to Users, j/k to select, Enter to reply",
                        author_username, unread
                    ));
                }
            }

            ServerMessage::UserJoinedRoom { room_id, user, .. } => {
                // Always cache the user info
                self.cache_user(&user);
                // Mark user as online
                self.online_user_ids.insert(user.id);
                // Add to all_users if not present and not the current user
                let is_self = self.user.as_ref().map(|u| u.id == user.id).unwrap_or(false);
                if !is_self && !self.all_users.iter().any(|u| u.id == user.id) {
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
                debug!(
                    "Received UserLeftRoom event: room={}, user={}, reason={}",
                    room_id, user_id, reason
                );
                // Check if the current user was kicked/banned from the room
                let is_current_user = self.user.as_ref().map(|u| u.id == user_id).unwrap_or(false);

                if is_current_user {
                    // User was removed from a room
                    let was_in_this_room = self
                        .current_room
                        .as_ref()
                        .map(|r| r.id == room_id)
                        .unwrap_or(false);

                    if was_in_this_room {
                        // Exit the room view
                        let room_name = self
                            .current_room
                            .as_ref()
                            .map(|r| r.name.clone())
                            .unwrap_or_default();
                        self.current_room = None;
                        self.current_room_members.clear();
                        self.messages.clear();

                        // Show notification based on reason
                        if reason == "kicked" {
                            self.set_error(format!("You were kicked from #{}", room_name));
                        } else if reason == "banned" {
                            self.set_error(format!("You were banned from #{}", room_name));
                        } else {
                            self.set_info(format!("You left #{}", room_name));
                        }
                    }
                } else if let Some(current) = &self.current_room {
                    // Another user left the room we're viewing
                    if room_id == current.id {
                        let username = self.get_username(user_id);
                        self.add_system_message(format!("{} left the room ({})", username, reason));
                        // Remove from local members list
                        self.current_room_members.retain(|m| m.user_id != user_id);
                    }
                }
            }

            ServerMessage::MemberRoleChanged {
                room_id,
                username,
                old_role,
                new_role,
                ..
            } => {
                if let Some(current) = &self.current_room {
                    if room_id == current.id {
                        self.add_system_message(format!(
                            "{}'s role changed from {} to {}",
                            username, old_role, new_role
                        ));
                        // Refresh members list if we have the overlay open
                        // The caller can handle refreshing the members list
                    }
                }
            }

            ServerMessage::UserOnline { user_id, username } => {
                // Cache this user and mark as online
                self.user_cache.insert(user_id, username.clone());
                self.online_user_ids.insert(user_id);

                // Check if we have unread DMs from this user
                if let Some(&unread) = self.unread_dms.get(&user_id) {
                    self.add_system_message(format!(
                        "{} is now online ({} unread DM{})",
                        username,
                        unread,
                        if unread == 1 { "" } else { "s" }
                    ));
                } else {
                    self.add_system_message(format!("{} is now online", username));
                }
            }

            ServerMessage::UserOffline { user_id, username } => {
                // Check if this is our current DM partner
                let is_dm_partner = self
                    .current_dm_user
                    .as_ref()
                    .is_some_and(|dm| dm.id == user_id);

                if is_dm_partner {
                    // More prominent notification for DM partner
                    self.add_system_message(format!(
                        "⚠ {} went offline - they may not see your messages",
                        username
                    ));
                } else {
                    self.add_system_message(format!("{} went offline", username));
                }
                self.online_user_ids.remove(&user_id);
            }

            ServerMessage::ServerNotice { message, severity } => {
                self.add_system_message(format!("[{}] {}", severity.to_uppercase(), message));
            }

            ServerMessage::Error { code, message, .. } => {
                self.set_error(format!("{}: {}", code, message));
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

            ServerMessage::InvitationReceived { invitation } => {
                debug!(
                    "Received InvitationReceived event: room={}, from={}",
                    invitation.room_name, invitation.inviter_name
                );
                // Add to pending invitations
                self.pending_invitations.push(invitation.clone());

                // Show prominent notification (names are always populated by server)
                let inviter = &invitation.inviter_name;
                let room = &invitation.room_name;
                self.set_warning(format!(
                    "📨 {} invited you to #{} (I to view)",
                    inviter, room
                ));
                self.add_system_message(format!(
                    "📨 {} invited you to join #{} - Press 'I' to view invitations",
                    inviter, room
                ));
            }

            ServerMessage::ListInvitationsResponse {
                success,
                invitations,
                ..
            } => {
                if success {
                    self.pending_invitations = invitations;
                    debug!(
                        "Received {} pending invitations",
                        self.pending_invitations.len()
                    );
                }
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
    /// Poll for new messages from the server.
    ///
    /// Returns `true` if any new messages were received (for smart scroll).
    pub async fn poll_messages(&mut self) -> bool {
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

        let had_messages = !messages.is_empty();

        // Handle connection loss - attempt automatic reconnection
        if connection_lost {
            self.connection = None;
            self.add_system_message("Connection lost. Attempting to reconnect...");
            self.reconnect().await;
        }

        // Process collected messages
        for msg in messages {
            debug!(
                "Processing server message: {:?}",
                std::mem::discriminant(&msg)
            );
            self.handle_server_message(msg).await;
        }

        had_messages
    }

    /// Send a ping to keep the connection alive.
    pub async fn send_ping(&mut self) {
        if let Some(conn) = &self.connection {
            let _ = conn.send(ClientMessage::Ping).await;
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn test_server_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)
    }

    // ========================================================================
    // NotificationLevel Tests
    // ========================================================================

    #[test]
    fn test_notification_level_auto_dismiss_durations() {
        assert_eq!(
            NotificationLevel::Info.auto_dismiss_duration(),
            Duration::from_secs(3)
        );
        assert_eq!(
            NotificationLevel::Success.auto_dismiss_duration(),
            Duration::from_secs(4)
        );
        assert_eq!(
            NotificationLevel::Warning.auto_dismiss_duration(),
            Duration::from_secs(6)
        );
        assert_eq!(
            NotificationLevel::Error.auto_dismiss_duration(),
            Duration::from_secs(8)
        );
    }

    // ========================================================================
    // ChatMessage Tests
    // ========================================================================

    #[test]
    fn test_chat_message_system() {
        let msg = ChatMessage::system("Welcome to the server");

        assert!(msg.is_system);
        assert_eq!(msg.author, "System");
        assert_eq!(msg.content, "Welcome to the server");
        assert!(msg.id.is_none());
    }

    #[test]
    fn test_chat_message_user() {
        let msg = ChatMessage::user("alice", "Hello, world!");

        assert!(!msg.is_system);
        assert_eq!(msg.author, "alice");
        assert_eq!(msg.content, "Hello, world!");
        assert!(msg.id.is_none());
    }

    // ========================================================================
    // App Initialization Tests
    // ========================================================================

    #[test]
    fn test_app_with_http_config() {
        let addr = test_server_addr();
        let app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        assert_eq!(app.screen, Screen::Login);
        assert!(app.connection.is_none());
        assert!(app.user.is_none());
        assert!(app.session.is_none());
        assert!(app.token.is_none());
        assert!(app.rooms.is_empty());
        assert!(app.current_room.is_none());
        assert!(app.current_dm_user.is_none());
        assert!(app.messages.is_empty());
        assert!(app.notifications.is_empty());
        assert!(!app.should_quit);
    }

    // ========================================================================
    // Notification Tests
    // ========================================================================

    #[test]
    fn test_set_error() {
        let addr = test_server_addr();
        let mut app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        app.set_error("Test error");

        assert!(!app.notifications.is_empty());
        assert_eq!(app.notifications[0].level, NotificationLevel::Error);
        assert_eq!(app.notifications[0].message, "Test error");
    }

    #[test]
    fn test_set_info() {
        let addr = test_server_addr();
        let mut app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        app.set_info("Test info");

        assert!(!app.notifications.is_empty());
        assert_eq!(app.notifications[0].level, NotificationLevel::Info);
        assert_eq!(app.notifications[0].message, "Test info");
    }

    #[test]
    fn test_set_success() {
        let addr = test_server_addr();
        let mut app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        app.set_success("Test success");

        assert!(!app.notifications.is_empty());
        assert_eq!(app.notifications[0].level, NotificationLevel::Success);
        assert_eq!(app.notifications[0].message, "Test success");
    }

    #[test]
    fn test_clear_notifications() {
        let addr = test_server_addr();
        let mut app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        app.set_error("Error 1");
        app.set_info("Info 1");
        assert_eq!(app.notifications.len(), 2);

        app.clear_notifications();
        assert!(app.notifications.is_empty());
    }

    #[test]
    fn test_error_getter() {
        let addr = test_server_addr();
        let mut app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        assert!(app.error().is_none());

        app.set_error("Test error");
        assert_eq!(app.error(), Some("Test error"));

        // Info notifications shouldn't appear in error()
        app.clear_notifications();
        app.set_info("Test info");
        assert!(app.error().is_none());
    }

    #[test]
    fn test_notifications_getter() {
        let addr = test_server_addr();
        let mut app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        app.set_error("Error");
        app.set_info("Info");

        let notifs = app.notifications();
        assert_eq!(notifs.len(), 2);
    }

    // ========================================================================
    // User List Tests
    // ========================================================================

    #[test]
    fn test_get_user_lists_empty() {
        let addr = test_server_addr();
        let app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        let (online, offline) = app.get_user_lists();
        assert!(online.is_empty());
        assert!(offline.is_empty());
    }

    #[test]
    fn test_get_user_lists_with_users() {
        let addr = test_server_addr();
        let mut app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        // Add some users
        let alice_id = Uuid::new_v4();
        let bob_id = Uuid::new_v4();

        app.all_users.push(User {
            id: alice_id,
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            role: "user".to_string(),
            created_at: Utc::now(),
        });
        app.all_users.push(User {
            id: bob_id,
            username: "bob".to_string(),
            email: "bob@example.com".to_string(),
            role: "user".to_string(),
            created_at: Utc::now(),
        });

        // Mark Alice as online
        app.online_user_ids.insert(alice_id);

        let (online, offline) = app.get_user_lists();
        assert!(online.contains(&"alice".to_string()));
        assert!(offline.contains(&"bob".to_string()));
    }

    // ========================================================================
    // Unread DM Tests
    // ========================================================================

    #[test]
    fn test_get_unread_dms_empty() {
        let addr = test_server_addr();
        let app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        let unread = app.get_unread_dms();
        assert!(unread.is_empty());
    }

    #[test]
    fn test_get_unread_dms_with_counts() {
        let addr = test_server_addr();
        let mut app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        let alice_id = Uuid::new_v4();
        app.all_users.push(User {
            id: alice_id,
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            role: "user".to_string(),
            created_at: Utc::now(),
        });

        app.unread_dms.insert(alice_id, 5);

        let unread = app.get_unread_dms();
        assert_eq!(unread.get("alice"), Some(&5));
    }

    // ========================================================================
    // System Message Tests
    // ========================================================================

    #[test]
    fn test_add_system_message() {
        let addr = test_server_addr();
        let mut app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        app.add_system_message("Test system message");

        assert_eq!(app.messages.len(), 1);
        assert!(app.messages[0].is_system);
        assert_eq!(app.messages[0].content, "Test system message");
    }

    // ========================================================================
    // Last Message Content Tests
    // ========================================================================

    #[test]
    fn test_last_message_content_empty() {
        let addr = test_server_addr();
        let app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        assert!(app.last_message_content().is_none());
    }

    #[test]
    fn test_last_message_content() {
        let addr = test_server_addr();
        let mut app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        app.messages
            .push(ChatMessage::user("alice", "First message"));
        app.messages.push(ChatMessage::user("bob", "Last message"));

        assert_eq!(app.last_message_content(), Some("Last message"));
    }

    // ========================================================================
    // Screen Tests
    // ========================================================================

    #[test]
    fn test_screen_enum() {
        assert_eq!(Screen::Login, Screen::Login);
        assert_eq!(Screen::Chat, Screen::Chat);
        assert_eq!(Screen::Rooms, Screen::Rooms);
        assert_ne!(Screen::Login, Screen::Chat);
    }

    // ========================================================================
    // Action Tests
    // ========================================================================

    #[test]
    fn test_action_variants() {
        // Test that all action variants can be constructed
        let _ = Action::Quit;
        let _ = Action::Login {
            server: "127.0.0.1:8080".to_string(),
            username: "test".to_string(),
            password: "pass".to_string(),
        };
        let _ = Action::Register {
            server: "127.0.0.1:8080".to_string(),
            username: "test".to_string(),
            email: "test@test.com".to_string(),
            password: "pass".to_string(),
        };
        let _ = Action::SendMessage("hello".to_string());
        let _ = Action::ShowRooms;
        let _ = Action::JoinRoom(Uuid::new_v4());
        let _ = Action::CreateRoom("room".to_string());
        let _ = Action::BackToChat;
        let _ = Action::Reconnect;
        let _ = Action::StartDM("user".to_string());
        let _ = Action::StartDMByIndex(0);
        let _ = Action::ShowHelp;
        let _ = Action::ClearError;
        let _ = Action::CopyLastMessage;
    }

    // ========================================================================
    // Tick Notifications Test
    // ========================================================================

    #[test]
    fn test_tick_notifications_keeps_recent() {
        let addr = test_server_addr();
        let mut app = App::with_http_config(addr, "http://localhost:8082".to_string(), false);

        app.set_error("Recent error");

        // Tick immediately - notification should still be there
        app.tick_notifications();
        assert!(!app.notifications.is_empty());
    }
}
