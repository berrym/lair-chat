//! Application state and logic.

use std::net::SocketAddr;

use chrono::{DateTime, Utc};
use tracing::{debug, info};

use crate::protocol::{
    ClientMessage, Connection, MessageTarget, Room, ServerMessage, Session, TcpError, User,
};

/// Application state.
pub struct App {
    /// Current screen.
    pub screen: Screen,
    /// Connection to the server.
    pub connection: Option<Connection>,
    /// Server address.
    pub server_addr: SocketAddr,
    /// Current user (after login).
    pub user: Option<User>,
    /// Current session.
    pub session: Option<Session>,
    /// Rooms the user is a member of.
    pub rooms: Vec<Room>,
    /// Currently selected room.
    pub current_room: Option<Room>,
    /// Messages for the current view.
    pub messages: Vec<ChatMessage>,
    /// Status message.
    pub status: Option<String>,
    /// Error message.
    pub error: Option<String>,
    /// Should quit.
    pub should_quit: bool,
    /// Online users.
    pub online_users: Vec<User>,
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
    /// Create a room.
    CreateRoom(String),
    /// Go back to chat.
    BackToChat,
}

impl App {
    /// Create a new application.
    pub fn new(server_addr: SocketAddr) -> Self {
        Self {
            screen: Screen::Login,
            connection: None,
            server_addr,
            user: None,
            session: None,
            rooms: Vec::new(),
            current_room: None,
            messages: Vec::new(),
            status: None,
            error: None,
            should_quit: false,
            online_users: Vec::new(),
        }
    }

    /// Connect to the server.
    pub async fn connect(&mut self) -> Result<(), TcpError> {
        info!("Connecting to {}", self.server_addr);
        self.status = Some("Connecting...".to_string());

        let connection = Connection::connect(self.server_addr).await?;
        self.connection = Some(connection);
        self.status = Some("Connected".to_string());
        self.add_system_message("Connected to server");

        Ok(())
    }

    /// Disconnect from the server.
    pub async fn disconnect(&mut self) {
        if let Some(conn) = self.connection.take() {
            conn.shutdown().await;
        }
        self.user = None;
        self.session = None;
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
            Action::CreateRoom(name) => {
                self.handle_create_room(name).await;
            }
            Action::BackToChat => {
                self.screen = Screen::Chat;
            }
        }
    }

    /// Handle login.
    async fn handle_login(&mut self, username: String, password: String) {
        let Some(conn) = &self.connection else {
            self.error = Some("Not connected".to_string());
            return;
        };

        let msg = ClientMessage::login(&username, &password);

        if let Err(e) = conn.send(msg).await {
            self.error = Some(format!("Failed to send login: {}", e));
            return;
        }

        self.status = Some("Logging in...".to_string());
    }

    /// Handle registration.
    async fn handle_register(&mut self, username: String, email: String, password: String) {
        let Some(conn) = &self.connection else {
            self.error = Some("Not connected".to_string());
            return;
        };

        let msg = ClientMessage::register(&username, &email, &password);

        if let Err(e) = conn.send(msg).await {
            self.error = Some(format!("Failed to send register: {}", e));
            return;
        }

        self.status = Some("Registering...".to_string());
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

        // For now, we need a room to send to
        let Some(room) = &self.current_room else {
            self.error = Some("No room selected".to_string());
            return;
        };

        let target = MessageTarget::Room { room_id: room.id };

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
            request_id: Some(uuid::Uuid::new_v4().to_string()),
            filter: None,
            limit: Some(50),
            offset: None,
        };

        if let Err(e) = conn.send(msg).await {
            self.error = Some(format!("Failed to request rooms: {}", e));
        }
    }

    /// Handle joining a room.
    async fn handle_join_room(&mut self, room_id: uuid::Uuid) {
        let Some(conn) = &self.connection else {
            return;
        };

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
            ServerMessage::LoginResponse {
                success,
                user,
                session,
                error,
                ..
            } => {
                if success {
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
                    self.add_system_message("Login successful! Welcome to Lair Chat.");
                    // Request room list after login
                    self.request_room_list().await;
                } else {
                    let err_msg = error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Login failed".to_string());
                    self.error = Some(err_msg);
                    self.status = None;
                }
            }

            ServerMessage::RegisterResponse {
                success,
                user,
                session,
                error,
                ..
            } => {
                if success {
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
                    self.add_system_message("Registration successful! Welcome to Lair Chat.");
                    self.request_room_list().await;
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
                    self.rooms = rooms.into_iter().map(|r| r.room).collect();
                    // Auto-join first room if none selected
                    if self.current_room.is_none() && !self.rooms.is_empty() {
                        let first_room = self.rooms[0].clone();
                        self.handle_join_room(first_room.id).await;
                    }
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
                        self.add_system_message(format!("Joined room: {}", room.name));
                        self.current_room = Some(room);
                        self.messages.clear();
                        self.screen = Screen::Chat;
                        // Request message history for the room
                        self.request_message_history().await;
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
                        self.rooms.push(room.clone());
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
                    // Insert historical messages at the beginning (they're older)
                    let history: Vec<ChatMessage> = messages
                        .into_iter()
                        .map(|msg| {
                            let author = self
                                .online_users
                                .iter()
                                .find(|u| u.id == msg.author)
                                .map(|u| u.username.clone())
                                .unwrap_or_else(|| msg.author.to_string());

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

            ServerMessage::MessageReceived { message } => {
                // Only show if it's for our current room
                if let Some(current) = &self.current_room {
                    if let MessageTarget::Room { room_id } = &message.target {
                        if room_id == &current.id {
                            // Find author name
                            let author = self
                                .online_users
                                .iter()
                                .find(|u| u.id == message.author)
                                .map(|u| u.username.clone())
                                .unwrap_or_else(|| message.author.to_string());

                            self.messages.push(ChatMessage {
                                id: Some(message.id),
                                author,
                                content: message.content,
                                timestamp: message.created_at,
                                is_system: false,
                            });
                        }
                    }
                }
            }

            ServerMessage::UserJoinedRoom { room_id, user, .. } => {
                if let Some(current) = &self.current_room {
                    if room_id == current.id {
                        self.add_system_message(format!("{} joined the room", user.username));
                        if !self.online_users.iter().any(|u| u.id == user.id) {
                            self.online_users.push(user);
                        }
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
                        if let Some(user) = self.online_users.iter().find(|u| u.id == user_id) {
                            self.add_system_message(format!(
                                "{} left the room ({})",
                                user.username, reason
                            ));
                        }
                        self.online_users.retain(|u| u.id != user_id);
                    }
                }
            }

            ServerMessage::UserOnline { username, .. } => {
                self.add_system_message(format!("{} is now online", username));
            }

            ServerMessage::UserOffline { user_id, username } => {
                self.add_system_message(format!("{} went offline", username));
                self.online_users.retain(|u| u.id != user_id);
            }

            ServerMessage::ServerNotice { message, severity } => {
                self.add_system_message(format!("[{}] {}", severity.to_uppercase(), message));
            }

            ServerMessage::Error { code, message, .. } => {
                self.error = Some(format!("{}: {}", code, message));
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

        // Handle connection loss
        if connection_lost {
            self.connection = None;
            self.error = Some("Connection lost".to_string());
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
