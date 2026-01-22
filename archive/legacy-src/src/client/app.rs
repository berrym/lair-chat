use color_eyre::Result;
use crossterm::event::KeyEvent;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::{
    action::Action,
    aes_gcm_encryption::create_aes_gcm_encryption_with_random_key,
    auth::{AuthState, Credentials},
    components::{
        auth::{AuthStatusBar, LoginScreen},
        fps::FpsCounter,
        home::Home,
        Component, StatusBar,
    },
    config::Config,
    connection_manager::ConnectionManager,
    message_router::ClientMessageRouter,
    tcp_transport::TcpTransport,
    transport::{ConnectionConfig, ConnectionObserver, Message, MessageStore},
    tui::{Event, Tui},
};

use std::sync::Arc;

pub struct App {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,

    // Modern connection management
    connection_manager: Arc<tokio::sync::Mutex<ConnectionManager>>,

    // Authentication components
    auth_state: AuthState,
    login_screen: LoginScreen,
    auth_status: AuthStatusBar,

    // Main application components
    home_component: Home,
    status_bar: StatusBar,
    fps_counter: FpsCounter,

    // Server-provided user list for DM discovery
    connected_users: Vec<String>,

    // Unified message routing system
    message_router: ClientMessageRouter,
}

/// Observer for handling ConnectionManager messages and events
pub struct ChatMessageObserver {
    action_sender: mpsc::UnboundedSender<Action>,
    message_store: Arc<std::sync::Mutex<MessageStore>>,
}

impl ChatMessageObserver {
    pub fn new(action_sender: mpsc::UnboundedSender<Action>) -> Self {
        Self {
            action_sender,
            message_store: Arc::new(std::sync::Mutex::new(MessageStore::new())),
        }
    }

    /// Get a reference to the message store
    pub fn get_message_store(&self) -> Arc<std::sync::Mutex<MessageStore>> {
        Arc::clone(&self.message_store)
    }
}

impl ConnectionObserver for ChatMessageObserver {
    fn on_message(&self, message: String) {
        debug!("Connection received message: {}", message);

        // *** COMPREHENSIVE MESSAGE SOURCE TRACKING ***
        if message.contains("USER_LIST")
            || message.contains("ROOM_LIST")
            || message.contains("CURRENT_ROOM")
            || message.contains("ROOM_STATUS")
            || message.contains("Reconnected User")
            || message.contains(": true")
            || message == "true"
        {
            tracing::error!(
                "🔍 CONNECTION OBSERVER: Protocol message received from server: '{}'",
                message
            );
        } else {
            tracing::info!(
                "🔍 CONNECTION OBSERVER: Regular message received: '{}'",
                message
            );
        }

        // Store message in local store
        if let Ok(mut store) = self.message_store.lock() {
            store.add_message(Message::received_message(message.clone()));
        }

        // Forward to the action sender for processing
        let _ = self.action_sender.send(Action::RouteMessage(message));
    }

    fn on_error(&self, error: String) {
        // Store error as system message
        if let Ok(mut store) = self.message_store.lock() {
            store.add_message(Message::error_message(error.clone()));
        }

        // Send error to UI via Error action
        let _ = self.action_sender.send(Action::Error(error));
    }

    fn on_status_change(&self, connected: bool) {
        // Handle connection status changes
        let status = if connected {
            crate::transport::ConnectionStatus::CONNECTED
        } else {
            crate::transport::ConnectionStatus::DISCONNECTED
        };

        // Store system message about connection status
        if let Ok(mut store) = self.message_store.lock() {
            let status_msg = if connected {
                "Connected to server."
            } else {
                "Disconnected from server."
            };
            store.add_message(Message::system_message(status_msg.to_string()));
        }

        // Send status change to UI via action system
        let _ = self
            .action_sender
            .send(Action::ConnectionStatusChanged(status));

        // Log for debugging
        tracing::info!("Connection status changed: {:?}", status);
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Processing,
    Home,
    Authentication,
}

impl App {
    pub fn new(tick_rate: f64, frame_rate: f64, text_only: bool) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        // Create modern ConnectionManager with transport
        let connection_config = ConnectionConfig {
            address: "127.0.0.1:8080".parse().unwrap(),
            timeout_ms: 5000,
        };

        let mut connection_manager = ConnectionManager::new(connection_config.clone());
        let transport = Box::new(TcpTransport::new(connection_config));
        connection_manager.with_transport(transport);

        // Configure secure AES-GCM encryption with proper handshake
        let encryption = create_aes_gcm_encryption_with_random_key();
        connection_manager.with_encryption(encryption);

        // Authentication will be automatically set up during connect()
        // after the encryption handshake is complete

        let connection_manager = Arc::new(tokio::sync::Mutex::new(connection_manager));

        // Clone action_tx for message router before moving it into the struct
        let action_tx_clone = action_tx.clone();

        Ok(Self {
            tick_rate,
            frame_rate,
            should_quit: false,
            should_suspend: false,
            config: Config::new()?,
            mode: Mode::Authentication, // Start in auth mode
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,

            // Modern connection management
            connection_manager,

            // Authentication components
            auth_state: AuthState::Unauthenticated,
            login_screen: LoginScreen::new(),
            auth_status: AuthStatusBar::new(),

            // Main components
            home_component: Home::new_with_options(text_only),
            status_bar: StatusBar::new(),
            fps_counter: FpsCounter::default(),

            // Server-provided user list
            connected_users: Vec::new(),

            // Unified message routing system
            message_router: ClientMessageRouter::new(action_tx_clone),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate)
            .mouse(true);
        tui.enter()?;

        // Initialize components
        let size = tui.size()?;
        self.init_components(size.into())?;

        // Set up action sender for transport layer to update status bar (legacy compatibility)
        // This is needed because authentication still uses legacy transport
        // TODO: Remove this once legacy transport is fully eliminated
        // crate::transport::set_action_sender(self.action_tx.clone());

        // Register observer with ConnectionManager for message handling
        {
            let mut manager = self.connection_manager.lock().await;
            let observer = Arc::new(ChatMessageObserver::new(self.action_tx.clone()));
            manager.register_observer(observer);
        }

        // Set up legacy transport bridge for backward compatibility

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;

            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    fn init_components(&mut self, size: ratatui::prelude::Size) -> Result<()> {
        // Initialize components
        self.home_component
            .register_action_handler(self.action_tx.clone())?;
        self.home_component
            .register_config_handler(self.config.clone())?;
        self.home_component.init(size)?;

        self.fps_counter
            .register_action_handler(self.action_tx.clone())?;
        self.fps_counter
            .register_config_handler(self.config.clone())?;
        self.fps_counter.init(size)?;

        // Register status bar for mouse events
        self.status_bar
            .register_action_handler(self.action_tx.clone())?;

        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };

        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => {
                self.last_tick_key_events.push(key);

                // Handle global keys first
                match key.code {
                    crossterm::event::KeyCode::Char('c')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        action_tx.send(Action::Quit)?;
                    }
                    _ => {
                        // Route to appropriate component based on auth state
                        if let Some(action) = self.handle_key_event(key) {
                            action_tx.send(action)?;
                        }
                    }
                }
            }
            Event::Mouse(mouse) => {
                // Handle mouse events based on auth state
                if let Some(action) = self.handle_mouse_event(mouse) {
                    action_tx.send(action)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        match self.auth_state {
            AuthState::Unauthenticated | AuthState::Failed { .. } => {
                // In unauthenticated mode, send to login screen
                self.login_screen.handle_key(key)
            }
            AuthState::Authenticated { .. } => {
                // In authenticated mode, send to home component
                self.home_component.handle_key(key)
            }
            AuthState::Authenticating => None,
        }
    }

    fn handle_mouse_event(&mut self, mouse: crossterm::event::MouseEvent) -> Option<Action> {
        match self.auth_state {
            AuthState::Authenticated { .. } => {
                // In authenticated mode, check status bar first for mouse events
                if let Ok(Some(action)) = self.status_bar.handle_mouse_event(mouse) {
                    return Some(action);
                }
                // Then try home component
                if let Ok(Some(action)) = self.home_component.handle_mouse_event(mouse) {
                    return Some(action);
                }
            }
            _ => {}
        }
        None
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        // Process all pending actions
        while let Ok(action) = self.action_rx.try_recv() {
            if let Some(action) = self.update(&action)? {
                self.action_tx.send(action)?;
            }
        }

        // Always render
        self.draw(tui)?;
        Ok(())
    }

    fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        // Log action for debugging
        if !matches!(action, Action::Tick | Action::Render | Action::Update) {
            tracing::debug!("Handling action: {:?}", action);
        }

        match action {
            Action::RecordReceivedMessage => {
                // Update the status bar message count for received messages
                self.status_bar.record_received_message();
                tracing::info!(
                    "DEBUG: App processed RecordReceivedMessage action - count now: {}",
                    self.status_bar.get_received_count()
                );
                Ok(None)
            }
            Action::RecordSentMessage => {
                // Update the status bar message count for sent messages
                self.status_bar.record_sent_message();
                tracing::info!(
                    "DEBUG: App processed RecordSentMessage action - count now: {}",
                    self.status_bar.get_sent_count()
                );
                Ok(None)
            }
            Action::UpdateUnreadDMCount(count) => {
                // Update the status bar with unread DM count
                self.status_bar.set_unread_dm_count(*count);
                tracing::info!(
                    "DEBUG: App processed UpdateUnreadDMCount action - count now: {}",
                    count
                );
                Ok(None)
            }
            Action::Quit => {
                self.should_quit = true;
                Ok(None)
            }
            Action::Suspend => {
                self.should_suspend = true;
                Ok(None)
            }
            Action::Resume => {
                self.should_suspend = false;
                Ok(None)
            }
            Action::Resize(w, h) => {
                // Handle resize
                let size = ratatui::prelude::Size::new(*w, *h);
                self.init_components(size)?;
                Ok(None)
            }
            Action::Tick => {
                // Update components that need tick events
                self.last_tick_key_events.drain(..);

                // Update FPS counter
                self.fps_counter.update(action.clone())?;

                Ok(None)
            }

            // Authentication actions - TEMPORARY: Use legacy transport for actual server connection
            // The modern handle_modern_login/register methods create mock sessions but don't connect
            // to the actual server transport layer, so messages aren't sent/received.
            // We use legacy methods here to maintain functionality while ConnectionManager
            // integration is completed. This will be replaced in Step 7-9.
            Action::Login(credentials) => {
                // Use ConnectionManager for login
                self.handle_connection_manager_login(credentials.clone());
                Ok(None)
            }
            Action::Register(credentials) => {
                // Use ConnectionManager for registration
                self.handle_connection_manager_register(credentials.clone());
                Ok(None)
            }

            Action::LoginWithServer(credentials, server_address) => {
                // Use ConnectionManager for login with specific server
                self.handle_connection_manager_login_with_server(
                    credentials.clone(),
                    server_address.clone(),
                );
                Ok(None)
            }

            Action::RegisterWithServer(credentials, server_address) => {
                // Use ConnectionManager for registration with specific server
                self.handle_connection_manager_register_with_server(
                    credentials.clone(),
                    server_address.clone(),
                );
                Ok(None)
            }

            Action::Logout => {
                info!("User logging out - cleaning up authentication state");

                // Clean up authentication state
                self.auth_state = AuthState::Unauthenticated;
                self.auth_status.update_state(self.auth_state.clone());
                self.mode = Mode::Authentication;

                // Add logout message to UI
                self.home_component
                    .add_message_to_room("Logged out successfully".to_string(), true);

                Ok(None)
            }

            Action::Reconnect => {
                info!("User requested reconnection - transitioning to authentication");

                // Use modern ConnectionManager to disconnect
                let connection_manager = Arc::clone(&self.connection_manager);
                tokio::spawn(async move {
                    let mut manager = connection_manager.lock().await;
                    let disconnect_result = manager.disconnect().await;
                    if let Err(e) = disconnect_result {
                        tracing::error!("Error during disconnect: {}", e);
                    } else {
                        tracing::info!("Successfully disconnected");
                    }
                });

                // Clean up authentication state and return to login
                self.auth_state = AuthState::Unauthenticated;
                self.auth_status.update_state(self.auth_state.clone());
                self.mode = Mode::Authentication;

                // Add informational message
                self.home_component
                    .add_message_to_room("Disconnected. Please log in again.".to_string(), true);

                Ok(None)
            }

            // New action for handling registration success
            Action::RegistrationSuccess(username) => {
                info!("User {} registered successfully", username);
                // Add success message to UI
                self.home_component.add_message_to_room(
                    format!("Registration successful for user: {}", username),
                    true,
                );
                // Keep in authenticating state, will transition when auth completes
                Ok(None)
            }

            // New action for handling auth failure
            Action::AuthenticationFailure(error) => {
                error!("Authentication failed: {}", error);

                // Immediately reset auth state first
                self.auth_state = AuthState::Failed {
                    reason: error.clone(),
                };
                self.auth_status.update_state(self.auth_state.clone());
                self.login_screen
                    .handle_error(crate::auth::AuthError::InternalError(error.clone()));
                self.mode = Mode::Authentication;

                // Reset connection manager to prevent hanging on next attempt
                let connection_manager = Arc::clone(&self.connection_manager);
                tokio::spawn(async move {
                    info!("Cleaning up connection after authentication failure");
                    // Force disconnect with longer wait
                    let mut manager = connection_manager.lock().await;
                    let _ = manager.disconnect().await;
                    drop(manager);
                    // Longer delay to ensure complete cleanup
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    info!("Connection cleanup completed");
                });

                // Add error message to UI for better user feedback
                self.home_component
                    .add_message_to_room(format!("Authentication failed: {}", error), true);
                Ok(None)
            }

            // New action for handling auth success
            Action::AuthenticationSuccess(auth_state) => {
                self.auth_state = auth_state.clone();
                self.auth_status.update_state(auth_state.clone());
                self.mode = Mode::Home;

                if let AuthState::Authenticated { ref profile, .. } = auth_state {
                    info!(
                        "User {} authenticated successfully - transitioning to home mode",
                        profile.username
                    );

                    // Update status bar with authentication info
                    self.status_bar.set_auth_state(auth_state.clone());

                    // Use modern ConnectionManager to get status
                    let connection_status = self.get_connection_status();
                    self.status_bar.set_connection_status(connection_status);
                    self.home_component.set_connection_status(connection_status);

                    // Initialize chat system for authenticated user
                    if let Err(e) = self
                        .home_component
                        .initialize_chat(profile.username.clone())
                    {
                        error!(
                            "Failed to initialize chat system for {}: {}",
                            profile.username, e
                        );
                    } else {
                        info!(
                            "Chat system initialized successfully for {}",
                            profile.username
                        );
                        // Update status bar to show Lobby room
                        self.status_bar.set_current_room(Some("Lobby".to_string()));
                    }

                    // Connection is already established during authentication
                    // Server will send welcome message, so we don't add duplicate client messages

                    info!("User {} authenticated and ready for chat", profile.username);
                }

                Ok(None)
            }

            Action::StartDMConversation(username) => {
                // Handle starting a DM conversation - update status bar
                self.status_bar
                    .set_current_room(Some(format!("DM with {}", username)));
                info!("Started DM conversation with {}", username);

                // Pass the action to home component for handling
                self.home_component.update(action.clone())?;
                Ok(None)
            }

            Action::ReturnToLobby => {
                // Handle returning to Lobby from DM mode - update status bar
                self.status_bar.set_current_room(Some("Lobby".to_string()));
                info!("Returned to Lobby from DM mode");

                // Pass the action to home component for handling
                self.home_component.update(action.clone())?;
                Ok(None)
            }

            Action::EnterInsert => {
                // Switch home component to insert mode
                self.home_component.update(action.clone())?;
                Ok(None)
            }
            Action::EnterNormal => {
                // Switch home component to normal mode
                self.home_component.update(action.clone())?;
                Ok(None)
            }
            Action::ExitProcessing => {
                // Exit processing mode
                self.home_component.update(action.clone())?;
                Ok(None)
            }
            Action::EnterProcessing => {
                // Enter processing mode
                self.home_component.update(action.clone())?;
                Ok(None)
            }
            Action::ToggleFps => {
                // Toggle FPS counter
                self.fps_counter.update(action.clone())?;
                Ok(None)
            }
            Action::ToggleShowHelp => {
                // Toggle help display
                self.home_component.update(action.clone())?;
                Ok(None)
            }
            Action::ToggleDM => {
                // Toggle DM navigation panel
                self.home_component.update(action.clone())?;
                Ok(None)
            }
            Action::OpenDMNavigation => {
                // Open DM navigation panel (from status bar click)
                self.home_component.update(Action::ToggleDM)?;
                Ok(None)
            }
            Action::MarkAllDMsRead => {
                // Mark all DM conversations as read
                self.home_component.update(action.clone())?;
                // Update status bar to show 0 unread
                self.status_bar.set_unread_dm_count(0);
                tracing::info!("Marked all DM conversations as read");
                Ok(None)
            }

            Action::SendMessage(message) => {
                info!("DEBUG: SendMessage action received: '{}'", message);
                // Handle message sending synchronously by using try_lock
                self.handle_modern_send_message_sync(message.clone());
                Ok(None)
            }

            // New unified message router actions
            Action::DisplayMessage { content, is_system } => {
                debug!(
                    "DisplayMessage action: content='{}', is_system={}",
                    content, is_system
                );

                // Check if this is a DM message that should be handled specially
                if content.starts_with("💬 ") && !*is_system {
                    // Extract sender and message from "💬 sender: message" format
                    if let Some(rest) = content.strip_prefix("💬 ") {
                        if let Some((sender, message)) = rest.split_once(": ") {
                            debug!("Processing DM message from {} to DM system", sender);

                            // Get current user to determine if this is sent or received
                            let current_user =
                                if let AuthState::Authenticated { ref profile, .. } =
                                    self.auth_state
                                {
                                    Some(profile.username.clone())
                                } else {
                                    None
                                };

                            if let Some(current_user) = current_user {
                                if sender == current_user {
                                    // This is a sent message - skip it since it was already added
                                    // when the user sent the message via handle_dm_message_send
                                    debug!(
                                        "Skipping sent DM message display - already added locally"
                                    );
                                } else {
                                    // This is a received message from sender
                                    debug!("Adding received DM message from {}", sender);
                                    self.home_component.add_dm_received_message(
                                        sender.to_string(),
                                        message.to_string(),
                                    );
                                }
                            } else {
                                // Fallback if no current user - treat as received
                                debug!("No current user - treating as received message");
                                self.home_component.add_dm_received_message(
                                    sender.to_string(),
                                    message.to_string(),
                                );
                            }
                        } else {
                            // Fallback to regular room message if parsing fails
                            self.home_component
                                .add_message_to_room(content.clone(), *is_system);
                        }
                    } else {
                        self.home_component
                            .add_message_to_room(content.clone(), *is_system);
                    }
                } else {
                    // Regular message or system message - display through room component
                    self.home_component
                        .add_message_to_room(content.clone(), *is_system);
                }

                debug!("DisplayMessage processed successfully");
                Ok(None)
            }

            Action::UpdateConnectedUsers(users) => {
                // Update connected users list
                self.connected_users = users.clone();
                self.home_component.update_connected_users(users.clone());
                Ok(None)
            }

            Action::UpdateCurrentRoom(room_name) => {
                // Update current room in status bar
                self.status_bar.set_current_room(Some(room_name.clone()));
                Ok(None)
            }

            Action::RouteMessage(message) => {
                debug!("RouteMessage action received: {}", message);

                // COMPREHENSIVE DEBUG: Log all protocol messages
                if message.contains("Reconnected User")
                    || message.contains("USER_LIST")
                    || message.contains("ROOM_LIST")
                    || message.contains("CURRENT_ROOM")
                    || message.contains("ROOM_STATUS")
                    || message.contains(": true")
                    || message == "true"
                {
                    tracing::warn!(
                        "🔍 APP DEBUG: RouteMessage received protocol message: '{}'",
                        message
                    );
                }

                let current_username =
                    if let AuthState::Authenticated { ref profile, .. } = self.auth_state {
                        Some(profile.username.clone())
                    } else {
                        None
                    };

                if let Some(current_username) = current_username {
                    debug!("Routing message for {}: '{}'", current_username, message);

                    // Update current user in message router
                    if let AuthState::Authenticated { ref profile, .. } = self.auth_state {
                        self.message_router
                            .set_current_user(Some(profile.username.clone()));
                    }

                    // Use unified message router as the ONLY message handler
                    match self
                        .message_router
                        .parse_and_route_protocol_message(&message)
                    {
                        Ok(()) => {
                            debug!(
                                "Message router successfully processed message for {}: '{}'",
                                current_username, message
                            );
                        }
                        Err(e) => {
                            debug!(
                                "Message router failed to process '{}' for {}: {}",
                                message, current_username, e
                            );

                            // SMART protocol message filtering - catch display spam but allow processing

                            // The message router failed, but we should only filter messages that
                            // would be harmful to display. Legitimate protocol messages like
                            // USER_LIST: and ROOM_STATUS: should have been processed by the router.

                            // If we get here with a protocol message, it means the router couldn't
                            // handle it properly, so we should filter it to prevent display spam.

                            // Filter Reconnected User protocol messages (these should never display)
                            if message.starts_with("Reconnected User: ")
                                && !message.contains(" joined the room")
                            {
                                debug!("Filtered out Reconnected User protocol: '{}'", message);
                                tracing::warn!(
                                    "🔍 APP FALLBACK: Filtered Reconnected User message that router couldn't handle: '{}'",
                                    message
                                );
                                return Ok(None);
                            }

                            // Filter any remaining protocol spam that slipped through
                            if message.starts_with("USER_LIST:")
                                || message.starts_with("ROOM_STATUS:")
                                || message.starts_with("ROOM_LIST:")
                                || message.starts_with("CURRENT_ROOM:")
                                || message == "true"
                                || message.trim() == "true"
                                || message.ends_with(": true")
                                || (message.contains(": ") && {
                                    let parts: Vec<&str> = message.splitn(2, ": ").collect();
                                    parts.len() == 2
                                        && (parts[1].starts_with("USER_LIST")
                                            || parts[1].starts_with("ROOM_STATUS")
                                            || parts[1].starts_with("CURRENT_ROOM")
                                            || parts[1] == "true"
                                            || parts[1].trim() == "true")
                                })
                            {
                                debug!(
                                    "Filtered out protocol spam that router couldn't handle: '{}'",
                                    message
                                );
                                tracing::warn!(
                                    "🔍 APP FALLBACK: Filtered protocol spam: '{}'",
                                    message
                                );
                                return Ok(None);
                            }

                            // *** EMERGENCY OVERRIDE: FINAL PROTOCOL BLOCK BEFORE UI ***
                            // This is the absolute last chance to block protocol messages
                            if message.contains("USER_LIST")
                                || message.contains("ROOM_LIST")
                                || message.contains("CURRENT_ROOM")
                                || message.contains("ROOM_STATUS")
                                || message.contains("Reconnected User")
                                || message.contains("REQUEST_USER_LIST")
                                || message.contains("ROOM_CREATED")
                                || message.contains("ROOM_JOINED")
                                || message.contains("ROOM_LEFT")
                                || message.contains(": true")
                                || message == "true"
                                || message.trim() == "true"
                                || (message.contains(": ") && {
                                    let parts: Vec<&str> = message.splitn(2, ": ").collect();
                                    parts.len() == 2
                                        && (parts[1].len() < 50
                                            && (parts[1].contains("_LIST")
                                                || parts[1].contains("_ROOM")
                                                || parts[1].contains("_STATUS")
                                                || parts[1].matches(',').count() > 2
                                                || parts[1].chars().all(|c| {
                                                    c.is_alphanumeric()
                                                        || c == '_'
                                                        || c == ':'
                                                        || c == ','
                                                })))
                                })
                            {
                                tracing::error!(
                                    "🚨🚨🚨 EMERGENCY OVERRIDE: Blocked protocol message at final stage: '{}'",
                                    message
                                );
                                return Ok(None);
                            }

                            // If we get here, log it as a genuine message that passed all filters
                            tracing::warn!(
                                "✅ APP: Displaying genuine unhandled message: '{}'",
                                message
                            );
                            self.home_component
                                .add_message_to_room(message.to_string(), false);
                        }
                    }
                } else {
                    debug!("Received message while not authenticated: '{}'", message);
                }

                Ok(None)
            }

            Action::MessageSent(message) => {
                // Handle sent messages from ConnectionManager
                info!("ACTION: MessageSent handler called with: '{}'", message);

                // Skip DM messages - they are handled by the message router system
                if message.starts_with("DM:") {
                    debug!(
                        "Skipping DM message from MessageSent action - handled by message router"
                    );
                    return Ok(None);
                }

                // Only add non-DM messages to room display
                self.home_component
                    .add_message_to_room(message.to_string(), false);
                info!("Sent message added to room: {}", message);

                // Record sent message for status bar
                self.status_bar.record_sent_message();
                info!(
                    "Sent message counted - Total now: {}",
                    self.status_bar.get_sent_count()
                );

                Ok(None)
            }

            Action::Error(error) => {
                // Handle errors from observer pattern and other sources
                self.home_component
                    .add_message_to_room(format!("Error: {}", error), true);
                warn!("Error received via action system: {}", error);
                Ok(None)
            }

            Action::DisconnectClient => {
                info!("User requested disconnect - using modern ConnectionManager");

                // Use modern ConnectionManager for disconnection
                let connection_manager = Arc::clone(&self.connection_manager);
                tokio::spawn(async move {
                    let mut manager = connection_manager.lock().await;
                    if let Err(e) = manager.disconnect().await {
                        error!("Failed to disconnect: {}", e);
                    } else {
                        info!("Successfully disconnected from server");
                    }
                });

                // Clean up authentication state and return to login
                self.auth_state = AuthState::Unauthenticated;
                self.auth_status.update_state(self.auth_state.clone());
                self.mode = Mode::Authentication;

                // Reset chat state in home component to ensure clean reconnection
                self.home_component.reset_chat_state();

                // Reset login screen state to ensure proper functionality
                self.login_screen.handle_auth_state(&self.auth_state);

                // Add informational message
                self.home_component
                    .add_message_to_room("Disconnected from server.".to_string(), true);

                Ok(None)
            }

            // Room actions
            Action::CreateRoom(room_name) => {
                info!("Creating room: {}", room_name);
                // Send room creation request to server using the new direct method
                let create_message = format!("CREATE_ROOM:{}", room_name);
                self.send_room_command_to_server(create_message);
                self.home_component
                    .add_message_to_room(format!("Creating room '{}'...", room_name), true);
                Ok(None)
            }

            Action::JoinRoom(room_name) => {
                info!("Joining room: {}", room_name);
                // Send room join request to server using the new direct method
                let join_message = format!("JOIN_ROOM:{}", room_name);
                self.send_room_command_to_server(join_message);
                self.home_component
                    .add_message_to_room(format!("Joining room '{}'...", room_name), true);
                Ok(None)
            }

            Action::LeaveRoom => {
                info!("Leaving current room");
                // Send leave room request to server using the new direct method
                self.send_room_command_to_server("LEAVE_ROOM".to_string());
                self.home_component
                    .add_message_to_room("Leaving current room...".to_string(), true);
                Ok(None)
            }

            Action::ListRooms => {
                info!("Requesting room list");
                // Send room list request to server using the new direct method
                self.send_room_command_to_server("LIST_ROOMS".to_string());
                self.home_component
                    .add_message_to_room("Requesting available rooms...".to_string(), true);
                Ok(None)
            }

            Action::RoomCreated(room_name) => {
                info!("Room created successfully: {}", room_name);
                self.home_component.add_message_to_room(
                    format!("Room '{}' created successfully!", room_name),
                    true,
                );
                // Pass the action to home component to update available rooms
                self.home_component.update(action.clone())?;
                // Automatically join the created room
                self.status_bar.set_current_room(Some(room_name.clone()));
                Ok(None)
            }

            Action::RoomJoined(room_name) => {
                info!("Successfully joined room: {}", room_name);
                self.home_component
                    .add_message_to_room(format!("✅ Joined room '{}'", room_name), true);
                self.status_bar.set_current_room(Some(room_name.clone()));
                Ok(None)
            }

            Action::RoomLeft(room_name) => {
                info!("Successfully left room: {}", room_name);
                self.home_component
                    .add_message_to_room(format!("✅ Left room '{}'", room_name), true);
                // Return to Lobby
                self.status_bar.set_current_room(Some("Lobby".to_string()));
                Ok(None)
            }

            Action::RoomError(error) => {
                warn!("Room operation error: {}", error);
                self.home_component
                    .add_message_to_room(format!("❌ Room error: {}", error), true);
                Ok(None)
            }

            Action::RoomListReceived(rooms) => {
                info!("Received room list: {:?}", rooms);
                if rooms.is_empty() {
                    self.home_component.add_message_to_room(
                        "📋 No rooms available besides Lobby".to_string(),
                        true,
                    );
                } else {
                    self.home_component
                        .add_message_to_room("📋 Available rooms:".to_string(), true);
                    for room in rooms {
                        self.home_component
                            .add_message_to_room(format!("  • {}", room), true);
                    }
                    self.home_component.add_message_to_room(
                        "Use /join <room_name> to join a room".to_string(),
                        true,
                    );
                }
                Ok(None)
            }

            Action::CurrentRoomChanged(room_name) => {
                info!("Current room changed to: {}", room_name);
                self.status_bar.set_current_room(Some(room_name.clone()));
                Ok(None)
            }

            Action::InvitationReceived(_inviter, room_name, invite_message) => {
                // Use DisplayMessage actions directly to ensure UI visibility
                let invitation_display = format!("🔔 INVITATION: {}", invite_message);
                if let Err(e) = self.action_tx.send(Action::DisplayMessage {
                    content: invitation_display,
                    is_system: true,
                }) {
                    warn!("Failed to send invitation display: {:?}", e);
                }

                // Send instructions via DisplayMessage
                let instructions = format!(
                    "💡 To respond: '/accept {}' or '/decline {}' or just '/accept' for latest",
                    room_name, room_name
                );
                if let Err(e) = self.action_tx.send(Action::DisplayMessage {
                    content: instructions,
                    is_system: true,
                }) {
                    warn!("Failed to send instructions: {:?}", e);
                }

                // Send alternatives via DisplayMessage
                let alternatives = format!(
                    "   You can also use '/join {}' to accept or '/invites' to see all pending",
                    room_name
                );
                if let Err(e) = self.action_tx.send(Action::DisplayMessage {
                    content: alternatives,
                    is_system: true,
                }) {
                    warn!("Failed to send alternatives: {:?}", e);
                }
                Ok(None)
            }

            Action::InviteError(error) => {
                warn!("Invitation error: {}", error);
                self.home_component
                    .add_message_to_room(format!("❌ Invitation error: {}", error), true);
                Ok(None)
            }

            Action::InvitationAccepted(room_name) => {
                info!("Invitation accepted for room: {}", room_name);
                // Automatically join the room
                let _ = self.action_tx.send(Action::JoinRoom(room_name.clone()));
                self.home_component.add_message_to_room(
                    format!("✅ Accepted invitation to join room '{}'", room_name),
                    true,
                );
                Ok(None)
            }

            Action::InvitationDeclined(room_name) => {
                info!("Invitation declined for room: {}", room_name);
                self.home_component.add_message_to_room(
                    format!("❌ Declined invitation to join room '{}'", room_name),
                    true,
                );
                Ok(None)
            }

            // Pass other actions to appropriate components
            _ => {
                match self.mode {
                    Mode::Home => {
                        self.home_component.update(action.clone())?;
                    }
                    _ => {}
                }
                Ok(None)
            }
        }
    }

    /// Reset connection manager to clean state
    async fn reset_connection(&self) {
        info!("Resetting connection manager to clean state");
        let connection_manager = Arc::clone(&self.connection_manager);
        let mut manager = connection_manager.lock().await;
        let _ = manager.disconnect().await;
        // Small delay to ensure clean disconnect
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    /// Modern authentication flow using ConnectionManager with server-compatible encryption
    fn handle_connection_manager_login(&mut self, credentials: Credentials) {
        let action_tx = self.action_tx.clone();
        let connection_manager = Arc::clone(&self.connection_manager);

        // Set state to authenticating immediately
        self.auth_state = AuthState::Authenticating;
        self.auth_status.update_state(self.auth_state.clone());

        tokio::spawn(async move {
            // Validate credentials
            if credentials.username.is_empty() || credentials.password.is_empty() {
                let _ = action_tx.send(Action::AuthenticationFailure(
                    "Username and password are required".to_string(),
                ));
                return;
            }

            if credentials.username.len() < 3 {
                let _ = action_tx.send(Action::AuthenticationFailure(
                    "Username must be at least 3 characters".to_string(),
                ));
                return;
            }

            // Ensure clean state by disconnecting first
            {
                info!("Ensuring clean connection state before login attempt");
                let mut manager = connection_manager.lock().await;
                let _ = manager.disconnect().await;
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            }

            // Connect using ConnectionManager with server-compatible encryption
            {
                let mut manager = connection_manager.lock().await;
                match manager.connect().await {
                    Ok(()) => {
                        info!("Successfully connected to server using ConnectionManager");
                    }
                    Err(e) => {
                        error!("ConnectionManager connection failed: {}", e);
                        let detailed_error = format!("Connection failed: {}. This could be due to: (1) Server not running - start with 'cargo run --bin lair-chat-server', (2) Server starting up - wait a moment and retry, (3) Port already in use, (4) Firewall blocking connection.", e);
                        let _ = action_tx.send(Action::AuthenticationFailure(detailed_error));
                        return;
                    }
                }
            }

            // Login using ConnectionManager
            {
                let manager = connection_manager.lock().await;
                match manager.login(credentials.clone()).await {
                    Ok(()) => {
                        info!("Login successful for user: {}", credentials.username);

                        // Create a successful auth state
                        let auth_state = AuthState::Authenticated {
                            profile: crate::auth::UserProfile {
                                id: uuid::Uuid::new_v4(),
                                username: credentials.username.clone(),
                                roles: vec!["user".to_string()],
                            },
                            session: crate::auth::Session {
                                id: uuid::Uuid::new_v4(),
                                token: format!("session_{}", credentials.username),
                                created_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                expires_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs()
                                    + 3600, // 1 hour expiration
                            },
                        };

                        // Add stabilization delay to ensure server-side login is complete
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        let _ = action_tx.send(Action::AuthenticationSuccess(auth_state));
                    }
                    Err(e) => {
                        error!("Login failed for {}: {}", credentials.username, e);

                        // Immediately send failure action for quick UI response
                        let _ = action_tx.send(Action::AuthenticationFailure(format!(
                            "Login failed: {}",
                            e
                        )));
                    }
                }
            }
        });
    }

    /// Modern registration flow using ConnectionManager with server-compatible encryption
    fn handle_connection_manager_register(&mut self, credentials: Credentials) {
        let action_tx = self.action_tx.clone();
        let connection_manager = Arc::clone(&self.connection_manager);

        // Set state to authenticating immediately
        self.auth_state = AuthState::Authenticating;
        self.auth_status.update_state(self.auth_state.clone());

        tokio::spawn(async move {
            // Validate credentials
            if credentials.username.is_empty() || credentials.password.is_empty() {
                let _ = action_tx.send(Action::AuthenticationFailure(
                    "Username and password are required".to_string(),
                ));
                return;
            }

            if credentials.username.len() < 3 {
                let _ = action_tx.send(Action::AuthenticationFailure(
                    "Username must be at least 3 characters".to_string(),
                ));
                return;
            }

            if credentials.password.len() < 6 {
                let _ = action_tx.send(Action::AuthenticationFailure(
                    "Password must be at least 6 characters".to_string(),
                ));
                return;
            }

            // Ensure clean state by disconnecting first and waiting
            {
                info!("Ensuring clean connection state before registration attempt");
                let mut manager = connection_manager.lock().await;
                let _ = manager.disconnect().await;
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }

            // Connect using ConnectionManager with server-compatible encryption
            {
                let mut manager = connection_manager.lock().await;
                match manager.connect().await {
                    Ok(()) => {
                        info!("Successfully connected to server using ConnectionManager");
                    }
                    Err(e) => {
                        error!("ConnectionManager connection failed: {}", e);
                        let detailed_error = format!("Connection failed: {}. This could be due to: (1) Server not running - start with 'cargo run --bin lair-chat-server', (2) Server starting up - wait a moment and retry, (3) Port already in use, (4) Firewall blocking connection.", e);
                        let _ = action_tx.send(Action::AuthenticationFailure(detailed_error));
                        return;
                    }
                }
            }

            // Register using ConnectionManager
            {
                let manager = connection_manager.lock().await;
                match manager.register(credentials.clone()).await {
                    Ok(()) => {
                        info!("Registration successful for user: {}", credentials.username);

                        // Send registration success notification
                        let _ = action_tx
                            .send(Action::RegistrationSuccess(credentials.username.clone()));

                        // Add stabilization delay to ensure server-side registration is complete
                        // This prevents the first message sending issue after registration
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                        // Create a successful auth state for auto-login after registration
                        let auth_state = AuthState::Authenticated {
                            profile: crate::auth::UserProfile {
                                id: uuid::Uuid::new_v4(),
                                username: credentials.username.clone(),
                                roles: vec!["user".to_string()],
                            },
                            session: crate::auth::Session {
                                id: uuid::Uuid::new_v4(),
                                token: format!("reg_session_{}", credentials.username),
                                created_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                expires_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs()
                                    + 3600, // 1 hour expiration
                            },
                        };

                        let _ = action_tx.send(Action::AuthenticationSuccess(auth_state));
                    }
                    Err(e) => {
                        error!("Registration failed for {}: {}", credentials.username, e);
                        let _ = action_tx.send(Action::AuthenticationFailure(format!(
                            "Registration failed: {}",
                            e
                        )));
                    }
                }
            }
        });
    }

    /// Modern authentication flow using ConnectionManager with specific server address
    fn handle_connection_manager_login_with_server(
        &mut self,
        credentials: Credentials,
        server_address: String,
    ) {
        let action_tx = self.action_tx.clone();
        let connection_manager = Arc::clone(&self.connection_manager);

        // Set state to authenticating immediately
        self.auth_state = AuthState::Authenticating;
        self.auth_status.update_state(self.auth_state.clone());

        tokio::spawn(async move {
            // Parse server address
            let addr: std::net::SocketAddr = match server_address.parse() {
                Ok(addr) => addr,
                Err(_) => {
                    let _ = action_tx.send(Action::AuthenticationFailure(format!(
                        "Invalid server address: {}",
                        server_address
                    )));
                    return;
                }
            };

            // Update connection manager config
            {
                let mut manager = connection_manager.lock().await;
                let config = crate::transport::ConnectionConfig::new(addr);
                manager.update_config(config);
            }

            // Validate credentials
            if credentials.username.is_empty() || credentials.password.is_empty() {
                let _ = action_tx.send(Action::AuthenticationFailure(
                    "Username and password are required".to_string(),
                ));
                return;
            }

            if credentials.username.len() < 3 {
                let _ = action_tx.send(Action::AuthenticationFailure(
                    "Username must be at least 3 characters".to_string(),
                ));
                return;
            }

            // Connect using ConnectionManager with server-compatible encryption
            {
                let mut manager = connection_manager.lock().await;
                match manager.connect().await {
                    Ok(()) => {
                        info!("Successfully connected to server using ConnectionManager");
                    }
                    Err(e) => {
                        error!("ConnectionManager connection failed: {}", e);
                        let detailed_error = format!("Connection failed: {}. This could be due to: (1) Server not running - start with 'cargo run --bin lair-chat-server', (2) Server starting up - wait a moment and retry, (3) Port already in use, (4) Firewall blocking connection.", e);
                        let _ = action_tx.send(Action::AuthenticationFailure(detailed_error));
                        return;
                    }
                }
            }

            // Login using ConnectionManager
            {
                let manager = connection_manager.lock().await;
                match manager.login(credentials.clone()).await {
                    Ok(()) => {
                        info!("Login successful for user: {}", credentials.username);

                        // Create a successful auth state
                        let auth_state = AuthState::Authenticated {
                            profile: crate::auth::UserProfile {
                                id: uuid::Uuid::new_v4(),
                                username: credentials.username.clone(),
                                roles: vec!["user".to_string()],
                            },
                            session: crate::auth::Session {
                                id: uuid::Uuid::new_v4(),
                                token: format!("session_{}", credentials.username),
                                created_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                expires_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs()
                                    + 3600, // 1 hour expiration
                            },
                        };

                        // Add stabilization delay to ensure server-side login is complete
                        // This prevents the first message sending issue after authentication
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        let _ = action_tx.send(Action::AuthenticationSuccess(auth_state));
                    }
                    Err(e) => {
                        error!("Login failed for {}: {}", credentials.username, e);
                        let _ = action_tx.send(Action::AuthenticationFailure(format!(
                            "Login failed: {}",
                            e
                        )));
                    }
                }
            }
        });
    }

    /// Modern registration flow using ConnectionManager with specific server address
    fn handle_connection_manager_register_with_server(
        &mut self,
        credentials: Credentials,
        server_address: String,
    ) {
        let action_tx = self.action_tx.clone();
        let connection_manager = Arc::clone(&self.connection_manager);

        // Set state to authenticating immediately
        self.auth_state = AuthState::Authenticating;
        self.auth_status.update_state(self.auth_state.clone());

        tokio::spawn(async move {
            // Parse server address
            let addr: std::net::SocketAddr = match server_address.parse() {
                Ok(addr) => addr,
                Err(_) => {
                    let _ = action_tx.send(Action::AuthenticationFailure(format!(
                        "Invalid server address: {}",
                        server_address
                    )));
                    return;
                }
            };

            // Update connection manager config
            {
                let mut manager = connection_manager.lock().await;
                let config = crate::transport::ConnectionConfig::new(addr);
                manager.update_config(config);
            }

            // Validate credentials
            if credentials.username.is_empty() || credentials.password.is_empty() {
                let _ = action_tx.send(Action::AuthenticationFailure(
                    "Username and password are required".to_string(),
                ));
                return;
            }

            if credentials.username.len() < 3 {
                let _ = action_tx.send(Action::AuthenticationFailure(
                    "Username must be at least 3 characters".to_string(),
                ));
                return;
            }

            if credentials.password.len() < 6 {
                let _ = action_tx.send(Action::AuthenticationFailure(
                    "Password must be at least 6 characters".to_string(),
                ));
                return;
            }

            // Connect using ConnectionManager with server-compatible encryption
            {
                let mut manager = connection_manager.lock().await;
                match manager.connect().await {
                    Ok(()) => {
                        info!("Successfully connected to server using ConnectionManager");
                    }
                    Err(e) => {
                        error!("ConnectionManager connection failed: {}", e);
                        let detailed_error = format!("Connection failed: {}. This could be due to: (1) Server not running - start with 'cargo run --bin lair-chat-server', (2) Server starting up - wait a moment and retry, (3) Port already in use, (4) Firewall blocking connection.", e);
                        let _ = action_tx.send(Action::AuthenticationFailure(detailed_error));
                        return;
                    }
                }
            }

            // Register using ConnectionManager
            {
                let manager = connection_manager.lock().await;
                match manager.register(credentials.clone()).await {
                    Ok(()) => {
                        info!("Registration successful for user: {}", credentials.username);

                        // Send registration success notification
                        let _ = action_tx
                            .send(Action::RegistrationSuccess(credentials.username.clone()));

                        // Create a successful auth state for auto-login after registration
                        let auth_state = AuthState::Authenticated {
                            profile: crate::auth::UserProfile {
                                id: uuid::Uuid::new_v4(),
                                username: credentials.username.clone(),
                                roles: vec!["user".to_string()],
                            },
                            session: crate::auth::Session {
                                id: uuid::Uuid::new_v4(),
                                token: format!("reg_session_{}", credentials.username),
                                created_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                expires_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs()
                                    + 3600, // 1 hour expiration
                            },
                        };

                        // Verify connection is ready for messaging before completing registration
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                        // Double-check connection status
                        if manager.get_status_sync()
                            == crate::transport::ConnectionStatus::CONNECTED
                        {
                            info!("Connection verified ready for messaging after registration");
                            let _ = action_tx.send(Action::AuthenticationSuccess(auth_state));
                        } else {
                            warn!("Connection not ready after registration, retrying...");
                            // Give it a bit more time and try once more
                            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                            let _ = action_tx.send(Action::AuthenticationSuccess(auth_state));
                        }
                    }
                    Err(e) => {
                        error!("Registration failed for {}: {}", credentials.username, e);
                        let _ = action_tx.send(Action::AuthenticationFailure(format!(
                            "Registration failed: {}",
                            e
                        )));
                    }
                }
            }
        });
    }

    /// Modern message sending using ConnectionManager only (synchronous version)
    fn handle_modern_send_message_sync(&mut self, message: String) {
        info!(
            "🔥 DEBUG: handle_modern_send_message_sync called with: '{}'",
            message
        );

        // Check if this is a DM message and handle it specially
        if message.starts_with("DM:") {
            info!("📨 DEBUG: Detected DM message, routing to DM handler");
            self.handle_dm_message_send(message);
            return;
        }

        // Check if this is a room command and send it directly without user prefix
        if message.starts_with("CREATE_ROOM:")
            || message.starts_with("JOIN_ROOM:")
            || message == "LEAVE_ROOM"
            || message == "LIST_ROOMS"
            || message == "REQUEST_USER_LIST"
            || message.starts_with("INVITE_USER:")
            || message.starts_with("ACCEPT_INVITATION:")
            || message.starts_with("DECLINE_INVITATION:")
            || message == "LIST_INVITATIONS"
            || message == "ACCEPT_ALL_INVITATIONS"
        {
            info!(
                "🏠 DEBUG: Detected room command, sending directly: '{}'",
                message
            );
            self.send_room_command_to_server(message);
            return;
        }

        // Send to server using extracted method
        self.send_message_to_server(message);
    }

    /// Send room commands directly to server without user prefix
    fn send_room_command_to_server(&mut self, command: String) {
        let command_to_send = command.clone();

        // Check connection status
        let connection_status = if let Ok(manager) = self.connection_manager.try_lock() {
            manager.get_status_sync()
        } else {
            crate::transport::ConnectionStatus::DISCONNECTED
        };

        // Check if user is authenticated
        let is_authenticated = match &self.auth_state {
            AuthState::Authenticated { .. } => true,
            _ => false,
        };

        if !is_authenticated {
            info!("Room command attempt while not authenticated");
            let _ = self.action_tx.send(Action::DisplayMessage {
                content: "Cannot execute room command: Not logged in. Please log in first."
                    .to_string(),
                is_system: true,
            });
            return;
        }

        if connection_status == crate::transport::ConnectionStatus::CONNECTED {
            info!("Sending room command via ConnectionManager: '{}'", command);

            // Queue the command sending as an async task
            let connection_manager = Arc::clone(&self.connection_manager);
            let action_tx = self.action_tx.clone();
            let command_clone = command.clone();

            tokio::spawn(async move {
                let send_result = {
                    let mut manager = connection_manager.lock().await;
                    manager.send_message(command_to_send).await
                };

                match send_result {
                    Ok(()) => {
                        info!("Room command sent successfully: {}", command_clone);
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to send room command: {}", e);
                        tracing::error!("{}", error_msg);
                        let _ = action_tx.send(Action::Error(error_msg));
                    }
                }
            });
        } else {
            let error_msg = format!(
                "Cannot send room command - not connected (status: {:?})",
                connection_status
            );
            warn!("{}: {}", error_msg, command);
            let _ = self.action_tx.send(Action::Error(error_msg));
        }
    }

    /// Extract server sending logic to avoid recursion
    fn send_message_to_server(&mut self, message: String) {
        info!(
            "🔄 DEBUG: send_message_to_server called with: '{}'",
            message
        );

        // Send raw message content - server will format it with username
        let message_to_send = message.clone();

        // Get connection status from ConnectionManager
        let connection_status = if let Ok(manager) = self.connection_manager.try_lock() {
            manager.get_status_sync()
        } else {
            crate::transport::ConnectionStatus::DISCONNECTED
        };

        // Check if user is authenticated
        let is_authenticated = match &self.auth_state {
            AuthState::Authenticated { .. } => true,
            _ => false,
        };

        if !is_authenticated {
            info!(
                "Attempted to send message while not authenticated: auth_state={:?}",
                self.auth_state
            );
            // Show error message to user
            let tx = &self.action_tx;
            let _ = tx.send(Action::DisplayMessage {
                content: "Cannot send message: Not logged in. Please log in first.".to_string(),
                is_system: true,
            });
            return;
        }

        if connection_status == crate::transport::ConnectionStatus::CONNECTED {
            info!(
                "Sending message via ConnectionManager (status: {:?}): '{}'",
                connection_status, message
            );
            debug!("DEBUG: Connection verified as CONNECTED before sending message");

            // Verify that home component has been initialized
            if !self.home_component.is_chat_initialized() {
                // Try to initialize with current username
                if let AuthState::Authenticated { profile, .. } = &self.auth_state {
                    if let Err(e) = self
                        .home_component
                        .initialize_chat(profile.username.clone())
                    {
                        error!("Failed to initialize chat: {}", e);
                        return;
                    } else {
                        // Update status bar to show Lobby room
                        self.status_bar.set_current_room(Some("Lobby".to_string()));
                    }
                }
            }

            // Queue the message sending as an async task
            let connection_manager = Arc::clone(&self.connection_manager);
            let action_tx = self.action_tx.clone();
            let message_clone = message.clone();
            let message_to_send_clone = message_to_send.clone();

            tokio::spawn(async move {
                // Check connection status before attempting send
                let _pre_lock_status = {
                    if let Ok(manager) = connection_manager.try_lock() {
                        manager.get_status_sync()
                    } else {
                        crate::transport::ConnectionStatus::DISCONNECTED
                    }
                };

                let send_result = {
                    let mut manager = connection_manager.lock().await;
                    let _pre_send_status = manager.get_status().await;
                    let result = manager.send_message(message_to_send_clone).await;
                    let _post_send_status = manager.get_status().await;

                    result
                };
                tracing::info!(
                    "DEBUG: ConnectionManager.send_message returned: {:?}",
                    send_result
                );

                // Add additional debugging for first message issues
                if send_result.is_err() {
                    tracing::error!(
                        "DEBUG: Message send failed - this might be the first message issue"
                    );
                }

                // Check final status after lock is released
                let _final_status = {
                    if let Ok(manager) = connection_manager.try_lock() {
                        manager.get_status_sync()
                    } else {
                        crate::transport::ConnectionStatus::DISCONNECTED
                    }
                };

                match send_result {
                    Ok(()) => {
                        info!(
                            "Message sent successfully via ConnectionManager: {}",
                            message_clone
                        );

                        // Display sent message to user - but only for non-DM messages
                        // DM messages are handled separately in handle_dm_message_send
                        if !message_clone.starts_with("DM:") {
                            let sent_message = format!("You: {}", message_clone);
                            let _ = action_tx.send(Action::DisplayMessage {
                                content: sent_message,
                                is_system: false,
                            });
                        }

                        // Record sent message for status bar
                        let _ = action_tx.send(Action::MessageSent(message_clone));
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to send message: {}", e);
                        tracing::error!("{}", error_msg);
                        let _ = action_tx.send(Action::Error(error_msg));
                    }
                }
            });
        } else {
            let error_msg = format!(
                "Cannot send message - not connected (status: {:?})",
                connection_status
            );
            warn!("{}: {}", error_msg, message);
            let _ = self.action_tx.send(Action::Error(error_msg));
        }
    }

    /// Get current connection status from ConnectionManager (sync wrapper)
    /// Helper method for use in sync contexts
    fn get_connection_status(&self) -> crate::transport::ConnectionStatus {
        // Use try_lock to avoid blocking, fall back to sync status check
        if let Ok(manager) = self.connection_manager.try_lock() {
            let status = manager.get_status_sync();
            tracing::info!(
                "DEBUG: get_connection_status (sync) returning: {:?}",
                status
            );
            status
        } else {
            crate::transport::ConnectionStatus::DISCONNECTED
        }
    }

    fn draw(&mut self, tui: &mut Tui) -> Result<()> {
        // Get connection status for UI display
        let connection_status = self.get_connection_status();
        self.status_bar.set_connection_status(connection_status);
        self.home_component.set_connection_status(connection_status);

        tui.draw(|frame| {
            let area = frame.area();

            match self.auth_state {
                AuthState::Unauthenticated | AuthState::Failed { .. } => {
                    // Show login screen
                    if let Err(e) = self.login_screen.draw(frame, area) {
                        debug!("Error drawing login screen: {}", e);
                    }
                }
                AuthState::Authenticating => {
                    // Show loading screen with appropriate message
                    use ratatui::{
                        style::{Color, Style},
                        widgets::{Block, Borders, Paragraph},
                    };
                    let message = match self.login_screen.mode {
                        crate::components::auth::LoginMode::Register => {
                            "Creating account and connecting..."
                        }
                        crate::components::auth::LoginMode::Login => {
                            "Authenticating and connecting..."
                        }
                    };
                    let loading = Paragraph::new(message)
                        .style(Style::default().fg(Color::Yellow))
                        .block(Block::default().borders(Borders::ALL).title("Please Wait"));
                    frame.render_widget(loading, area);
                }
                AuthState::Authenticated { .. } => {
                    // Show main application
                    use ratatui::layout::{Constraint, Direction, Layout};

                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(1), // Status bar
                            Constraint::Min(0),    // Main content
                            Constraint::Length(1), // FPS counter
                        ])
                        .split(area);

                    // Update status bar with current state
                    self.status_bar.set_auth_state(self.auth_state.clone());

                    // Use modern ConnectionManager to get status
                    let connection_status = self.get_connection_status();
                    self.status_bar.set_connection_status(connection_status);
                    self.home_component.set_connection_status(connection_status);

                    // Draw status bar
                    if let Err(e) = self.status_bar.draw(frame, chunks[0]) {
                        debug!("Error drawing status bar: {}", e);
                    }

                    // Draw main content
                    if let Err(e) = self.home_component.draw(frame, chunks[1]) {
                        debug!("Error drawing home component: {}", e);
                    }

                    // Draw FPS counter
                    if let Err(e) = self.fps_counter.draw(frame, chunks[2]) {
                        debug!("Error drawing FPS counter: {}", e);
                    }
                }
            }
        })?;
        Ok(())
    }

    /// Handle sending a DM message (format: "DM:partner:content")
    fn handle_dm_message_send(&mut self, message: String) {
        info!(
            "🔥 DEBUG: handle_dm_message_send called with: '{}'",
            message
        );

        // Parse DM message format: "DM:partner:content"
        let parts: Vec<&str> = message.splitn(3, ':').collect();
        if parts.len() != 3 || parts[0] != "DM" {
            error!("❌ Invalid DM message format: {}", message);
            return;
        }

        let partner = parts[1];
        let content = parts[2];

        // DM message parsed successfully

        // Start DM conversation if not already active
        // This ensures the conversation is visible and properly initialized
        let _ = self
            .action_tx
            .send(Action::StartDMConversation(partner.to_string()));

        // Add sent message to local DM conversation immediately
        info!(
            "💬 DEBUG: Adding sent message to local DM conversation with {}",
            partner
        );
        self.home_component
            .add_dm_sent_message(partner.to_string(), content.to_string());

        // Send to server
        info!("🚀 DEBUG: About to send DM to server: '{}'", message);
        self.send_dm_to_server(message);
        info!("📡 DEBUG: DM sent to server");
    }

    /// Send DM directly to server without local processing
    fn send_dm_to_server(&mut self, message: String) {
        let message_to_send = message.clone();

        // Check connection status
        let connection_status = if let Ok(manager) = self.connection_manager.try_lock() {
            manager.get_status_sync()
        } else {
            crate::transport::ConnectionStatus::DISCONNECTED
        };

        // Check if user is authenticated
        let is_authenticated = match &self.auth_state {
            AuthState::Authenticated { .. } => true,
            _ => false,
        };

        if !is_authenticated {
            info!("DM attempt while not authenticated");
            let _ = self.action_tx.send(Action::DisplayMessage {
                content: "Cannot send DM: Not logged in. Please log in first.".to_string(),
                is_system: true,
            });
            return;
        }

        if connection_status == crate::transport::ConnectionStatus::CONNECTED {
            info!("Sending DM via ConnectionManager: '{}'", message);

            // Queue the DM sending as an async task
            let connection_manager = Arc::clone(&self.connection_manager);
            let action_tx = self.action_tx.clone();
            let message_clone = message.clone();

            tokio::spawn(async move {
                let send_result = {
                    let mut manager = connection_manager.lock().await;
                    manager.send_message(message_to_send).await
                };

                match send_result {
                    Ok(()) => {
                        info!("DM sent successfully: {}", message_clone);
                        // Record sent message for status bar (but don't duplicate in chat)
                        let _ = action_tx.send(Action::MessageSent(message_clone));
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to send DM: {}", e);
                        tracing::error!("{}", error_msg);
                        let _ = action_tx.send(Action::Error(error_msg));
                    }
                }
            });
        } else {
            let error_msg = format!(
                "Cannot send DM - not connected (status: {:?})",
                connection_status
            );
            warn!("{}: {}", error_msg, message);
            let _ = self.action_tx.send(Action::Error(error_msg));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::transport::{ConnectionStatus, MessageType};
    use tokio::sync::mpsc;

    #[test]
    fn test_chat_message_observer_message_handling() {
        // Set up message channel
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();

        // Create observer
        let observer = ChatMessageObserver::new(tx);

        // Test on_message
        observer.on_message("Hello world".to_string());

        // Verify message was stored in MessageStore
        let message_store = observer.get_message_store();
        let store = message_store.lock().unwrap();
        assert_eq!(store.messages.len(), 1);
        assert_eq!(store.messages[0].content, "Hello world");
        assert_eq!(store.messages[0].message_type, MessageType::ReceivedMessage);

        // Verify action was sent
        if let Some(action) = rx.try_recv().ok() {
            match action {
                Action::RouteMessage(msg) => assert_eq!(msg, "Hello world"),
                _ => panic!("Wrong action type received"),
            }
        } else {
            panic!("No action received");
        }
    }

    #[test]
    fn test_chat_message_observer_error_handling() {
        // Set up message channel
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();

        // Create observer
        let observer = ChatMessageObserver::new(tx);

        // Test on_error
        observer.on_error("Connection lost".to_string());

        // Verify error was stored in MessageStore
        let message_store = observer.get_message_store();
        let store = message_store.lock().unwrap();
        assert_eq!(store.messages.len(), 1);
        assert_eq!(store.messages[0].content, "Connection lost");
        assert_eq!(store.messages[0].message_type, MessageType::ErrorMessage);

        // Verify action was sent
        if let Some(action) = rx.try_recv().ok() {
            match action {
                Action::Error(msg) => assert_eq!(msg, "Connection lost"),
                _ => panic!("Wrong action type received"),
            }
        } else {
            panic!("No action received");
        }
    }

    #[test]
    fn test_chat_message_observer_status_change() {
        // Set up message channel
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();

        // Create observer
        let observer = ChatMessageObserver::new(tx);

        // Test on_status_change - connected
        observer.on_status_change(true);

        // Verify status message was stored in MessageStore
        let message_store = observer.get_message_store();
        let store = message_store.lock().unwrap();
        assert_eq!(store.messages.len(), 1);
        assert_eq!(store.messages[0].content, "Connected to server.");
        assert_eq!(store.messages[0].message_type, MessageType::SystemMessage);

        // Verify connection status action was sent
        if let Some(action) = rx.try_recv().ok() {
            match action {
                Action::ConnectionStatusChanged(status) => {
                    assert_eq!(status, ConnectionStatus::CONNECTED)
                }
                _ => panic!("Wrong action type received"),
            }
        } else {
            panic!("No action received");
        }

        // Test on_status_change - disconnected
        observer.on_status_change(false);

        // Verify status message was stored
        let message_store = observer.get_message_store();
        let store = message_store.lock().unwrap();
        assert_eq!(store.messages.len(), 2);
        assert_eq!(store.messages[1].content, "Disconnected from server.");
        assert_eq!(store.messages[1].message_type, MessageType::SystemMessage);

        // Verify connection status action was sent
        if let Some(action) = rx.try_recv().ok() {
            match action {
                Action::ConnectionStatusChanged(status) => {
                    assert_eq!(status, ConnectionStatus::DISCONNECTED)
                }
                _ => panic!("Wrong action type received"),
            }
        } else {
            panic!("No action received");
        }
    }
}
