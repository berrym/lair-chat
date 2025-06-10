use color_eyre::Result;
use crossterm::event::KeyEvent;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::{
    action::Action,
    auth::{AuthState, Credentials},
    connection_manager::ConnectionManager,
    components::{
        auth::{AuthStatusBar, LoginScreen},
        home::Home,
        fps::FpsCounter,
        StatusBar,
        Component
    },
    config::Config,
    transport::{ConnectionConfig, ConnectionObserver},
    tui::{Event, Tui},
    tcp_transport::TcpTransport,
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
    connection_manager: ConnectionManager,
    
    // Authentication components
    auth_state: AuthState,
    login_screen: LoginScreen,
    auth_status: AuthStatusBar,
    
    // Main application components
    home_component: Home,
    status_bar: StatusBar,
    fps_counter: FpsCounter,
}

/// Observer for handling ConnectionManager messages and events
pub struct ChatMessageObserver {
    action_sender: mpsc::UnboundedSender<Action>,
}

impl ChatMessageObserver {
    pub fn new(action_sender: mpsc::UnboundedSender<Action>) -> Self {
        Self { action_sender }
    }
}

impl ConnectionObserver for ChatMessageObserver {
    fn on_message(&self, message: String) {
        // Send received message to UI via action system
        let _ = self.action_sender.send(Action::ReceiveMessage(message));
    }
    
    fn on_error(&self, error: String) {
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
        
        // For now, just log the status change
        tracing::info!("Connection status changed: connected={}", connected);
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
    pub fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        
        // Create modern ConnectionManager with transport
        let connection_config = ConnectionConfig {
            address: "127.0.0.1:8080".parse().unwrap(),
            timeout_ms: 5000,
        };
        
        let mut connection_manager = ConnectionManager::new(connection_config.clone());
        let transport = Box::new(TcpTransport::new(connection_config));
        connection_manager.with_transport(transport);
        
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
            home_component: Home::new(),
            status_bar: StatusBar::new(),
            fps_counter: FpsCounter::default(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        // Initialize components
        let size = tui.size()?;
        self.init_components(size.into())?;

        // Set up action sender for transport layer to update status bar (legacy compatibility)
        #[allow(deprecated)]
        crate::transport::set_action_sender(self.action_tx.clone());

        // Set up ConnectionManager observer for modern message handling
        self.setup_connection_observer()?;

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
        self.home_component.register_action_handler(self.action_tx.clone())?;
        self.home_component.register_config_handler(self.config.clone())?;
        self.home_component.init(size)?;
        
        self.fps_counter.register_action_handler(self.action_tx.clone())?;
        self.fps_counter.register_config_handler(self.config.clone())?;
        self.fps_counter.init(size)?;
        
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
                    crossterm::event::KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
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
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        match self.auth_state {
            AuthState::Unauthenticated | AuthState::Failed { .. } => {
                // In auth mode, send events to login screen
                self.login_screen.handle_key(key)
            }
            AuthState::Authenticated { .. } => {
                // In authenticated mode, send to home component
                self.home_component.handle_key(key)
            }
            AuthState::Authenticating => {
                // During authentication, ignore key events
                None
            }
        }
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
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::Suspend => {
                self.should_suspend = true;
            }
            Action::Resume => {
                self.should_suspend = false;
            }
            Action::Resize(w, h) => {
                // Handle resize
                let size = ratatui::prelude::Size::new(*w, *h);
                self.init_components(size)?;
            }
            Action::Tick => {
                // Update components that need tick events
                self.last_tick_key_events.drain(..);
                
                // Update FPS counter
                self.fps_counter.update(action.clone())?;
            }
            
            // Authentication actions - TEMPORARY: Use legacy transport for actual server connection
            // The modern handle_modern_login/register methods create mock sessions but don't connect
            // to the actual server transport layer, so messages aren't sent/received.
            // We use legacy methods here to maintain functionality while ConnectionManager
            // integration is completed. This will be replaced in Step 7-9.
            Action::Login(credentials) => {
                // Use legacy method that actually connects to server and enables message flow
                self.handle_login_with_server(credentials.clone(), "127.0.0.1:8080".to_string());
            }
            
            Action::Register(credentials) => {
                // Use legacy method that actually connects to server and enables message flow
                self.handle_register_with_server(credentials.clone(), "127.0.0.1:8080".to_string());
            }
            
            Action::LoginWithServer(credentials, server_address) => {
                // For backward compatibility, still support legacy method for now
                #[allow(deprecated)]
                self.handle_login_with_server(credentials.clone(), server_address.clone());
            }
            
            Action::RegisterWithServer(credentials, server_address) => {
                // For backward compatibility, still support legacy method for now
                #[allow(deprecated)]
                self.handle_register_with_server(credentials.clone(), server_address.clone());
            }
            
            Action::Logout => {
                info!("User logging out - cleaning up authentication state");
                
                // Clean up authentication state
                self.auth_state = AuthState::Unauthenticated;
                self.auth_status.update_state(self.auth_state.clone());
                self.mode = Mode::Authentication;
                
                // Add logout message to UI
                self.home_component.add_message_to_room(
                    "Logged out successfully".to_string(),
                    false
                );
                
                // Future: Add ConnectionManager.disconnect() call here
                // when full ConnectionManager integration is complete
            }

            // New action for handling registration success
            Action::RegistrationSuccess(username) => {
                info!("User {} registered successfully", username);
                // Add success message to UI
                self.home_component.add_message_to_room(
                    format!("Registration successful for user: {}", username),
                    false
                );
                // Keep in authenticating state, will transition when auth completes
            }
            
            // New action for handling auth failure
            Action::AuthenticationFailure(error) => {
                error!("Authentication failed: {}", error);
                self.auth_state = AuthState::Failed { reason: error.clone() };
                self.auth_status.update_state(self.auth_state.clone());
                self.login_screen.handle_error(crate::auth::AuthError::InternalError(error.clone()));
                self.mode = Mode::Authentication;
                
                // Add error message to UI for better user feedback
                self.home_component.add_message_to_room(
                    format!("Authentication failed: {}", error),
                    false
                );
            }
            
            // New action for handling auth success
            Action::AuthenticationSuccess(auth_state) => {
                self.auth_state = auth_state.clone();
                self.auth_status.update_state(auth_state.clone());
                self.mode = Mode::Home;
                
                if let AuthState::Authenticated { ref profile, .. } = auth_state {
                    info!("User {} authenticated successfully - transitioning to home mode", profile.username);
                    
                    // Update status bar with authentication info
                    self.status_bar.set_auth_state(auth_state.clone());
                    
                    // Use modern ConnectionManager to get status
                    let connection_status = self.get_connection_status();
                    self.status_bar.set_connection_status(connection_status);
                    
                    // Initialize chat system for authenticated user
                    if let Err(e) = self.home_component.initialize_chat(profile.username.clone()) {
                        error!("Failed to initialize chat system for {}: {}", profile.username, e);
                    } else {
                        info!("Chat system initialized successfully for {}", profile.username);
                    }
                    
                    // Connection is already established during authentication
                    // Server will send welcome message, so we don't add duplicate client messages
                    
                    info!("User {} authenticated and ready for chat", profile.username);
                }
            }
            
            Action::EnterInsert => {
                // Switch home component to insert mode
                self.home_component.update(action.clone())?;
            }
            Action::EnterNormal => {
                // Switch home component to normal mode
                self.home_component.update(action.clone())?;
            }
            Action::ExitProcessing => {
                // Exit processing mode
                self.home_component.update(action.clone())?;
            }
            Action::EnterProcessing => {
                // Enter processing mode
                self.home_component.update(action.clone())?;
            }
            Action::ToggleFps => {
                // Toggle FPS counter
                self.fps_counter.update(action.clone())?;
            }
            Action::ToggleShowHelp => {
                // Toggle help display
                self.home_component.update(action.clone())?;
            }
            
            Action::SendMessage(message) => {
                self.handle_modern_send_message(message.clone());
            }
            
            Action::ReceiveMessage(message) => {
                // Modern message handling through observer pattern
                // Observer has already received the message, now handle UI updates
                info!("ACTION: ReceiveMessage handler called with: '{}'", message);
                
                // Check if this is an authentication response from server
                if message.contains("Welcome back") || 
                   message.contains("has joined the chat") ||
                   message.contains("Registration successful") {
                    info!("Authentication success message detected: '{}' (current auth_state: {:?})", message, self.auth_state);
                    
                    // Only process if we're currently authenticating to avoid interference
                    if self.auth_state == AuthState::Authenticating {
                        info!("Processing authentication response for authenticating user");
                        
                        // Create successful auth state - server has confirmed authentication
                        let username = if message.contains("Welcome back") {
                            // Extract from "Welcome back, username" format
                            message.split("Welcome back, ").nth(1)
                                .and_then(|s| s.split(',').next())
                                .unwrap_or("User")
                        } else if message.contains("has joined") {
                            // Extract from "username has joined the chat" format
                            message.split(" has joined").next().unwrap_or("User")
                        } else {
                            "User"
                        };
                        
                        info!("Extracted username from server response: '{}'", username);
                        
                        let auth_state = AuthState::Authenticated {
                            profile: crate::auth::UserProfile {
                                id: uuid::Uuid::new_v4(),
                                username: username.to_string(),
                                roles: vec!["user".to_string()],
                            },
                            session: crate::auth::Session {
                                id: uuid::Uuid::new_v4(),
                                token: format!("server_token_{}", username),
                                created_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                expires_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() + 3600,
                            },
                        };
                        
                        // Send authentication success action
                        info!("Sending AuthenticationSuccess action for user: {}", username);
                        let _ = self.action_tx.send(Action::AuthenticationSuccess(auth_state));
                    } else {
                        info!("Ignoring authentication message - not currently authenticating (state: {:?})", self.auth_state);
                    }
                } else if message.contains("Authentication failed") || 
                          message.contains("Login failed") ||
                          message.contains("Invalid credentials") {
                    warn!("Authentication failure message detected: '{}' (current auth_state: {:?})", message, self.auth_state);
                    
                    if self.auth_state == AuthState::Authenticating {
                        let _ = self.action_tx.send(Action::AuthenticationFailure(message.clone()));
                    }
                }
                
                // Ensure chat is initialized
                if !self.home_component.is_chat_initialized() {
                    warn!("Chat not initialized, attempting to initialize with default user");
                    if let Err(e) = self.home_component.initialize_chat("DefaultUser".to_string()) {
                        error!("Failed to initialize chat: {}", e);
                    }
                }
                
                self.home_component.add_message_to_room(message.to_string(), false);
                
                // Update status bar message count
                self.status_bar.record_received_message();
                info!("Message added to room and status updated: {}", message);
            }
            
            Action::Error(error) => {
                // Handle errors from observer pattern and other sources
                self.home_component.add_message_to_room(
                    format!("Error: {}", error), 
                    false
                );
                warn!("Error received via action system: {}", error);
            }
            
            // Pass other actions to appropriate components
            _ => {
                match self.mode {
                    Mode::Home => {
                        self.home_component.update(action.clone())?;
                    }
                    _ => {}
                }
            }
        }
        
        Ok(None)
    }

    /// Modern authentication flow using ConnectionManager
    fn handle_modern_login(&mut self, credentials: Credentials) {
        let action_tx = self.action_tx.clone();
        
        // Set state to authenticating immediately
        self.auth_state = AuthState::Authenticating;
        
        // Get authentication manager if available
        let auth_manager = if let Some(auth_state) = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.connection_manager.get_auth_state().await
            })
        }) {
            // Authentication manager is available
            true
        } else {
            // Enable authentication on connection manager if not already enabled
            self.connection_manager.with_auth();
            false
        };
        
        let creds = credentials.clone();
        
        // Spawn async task to handle real authentication
    tokio::spawn(async move {
        // For now, still use mock authentication but with realistic flow
        // This will be replaced with real ConnectionManager auth in future iterations
            
        // Step 1: Validate credentials
        if creds.username.is_empty() || creds.password.is_empty() {
            let _ = action_tx.send(Action::AuthenticationFailure("Username and password are required".to_string()));
            return;
        }
            
        if creds.username.len() < 3 {
            let _ = action_tx.send(Action::AuthenticationFailure("Username must be at least 3 characters".to_string()));
            return;
        }
            
        // Step 2: Simulate connection process
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            
        // Step 3: Simulate authentication process
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            
        // Step 4: Simulate success - in real implementation this would be:
        // 1. connection_manager.connect().await
        // 2. auth_manager.login(credentials).await
        // 3. Handle actual auth response
            
        let result = Action::AuthenticationSuccess(AuthState::Authenticated {
            profile: crate::auth::UserProfile {
                id: uuid::Uuid::new_v4(),
                username: creds.username.clone(),
                roles: vec!["user".to_string()],
            },
            session: crate::auth::Session {
                id: uuid::Uuid::new_v4(),
                token: format!("auth_token_{}", creds.username),
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                expires_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() + 3600, // 1 hour expiration
            },
        });
            
            let _ = action_tx.send(result);
        });
    }
    
    /// Modern registration flow using ConnectionManager
    fn handle_modern_register(&mut self, credentials: Credentials) {
        let action_tx = self.action_tx.clone();
        
        // Set state to authenticating immediately
        self.auth_state = AuthState::Authenticating;
        
        // Enable authentication on connection manager if not already enabled
        self.connection_manager.with_auth();
        
        let creds = credentials.clone();
        
        // Spawn async task to handle real registration
        tokio::spawn(async move {
            // For now, still use mock registration but with realistic flow
            // This will be replaced with real ConnectionManager registration in future iterations
            
            // Step 1: Validate registration requirements
            if creds.username.is_empty() || creds.password.is_empty() {
                let _ = action_tx.send(Action::AuthenticationFailure("Username and password are required".to_string()));
                return;
            }
            
            if creds.username.len() < 3 {
                let _ = action_tx.send(Action::AuthenticationFailure("Username must be at least 3 characters".to_string()));
                return;
            }
            
            if creds.password.len() < 6 {
                let _ = action_tx.send(Action::AuthenticationFailure("Password must be at least 6 characters".to_string()));
                return;
            }
            
            // Step 2: Simulate connection process
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            
            // Step 3: Simulate registration process
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            
            // Step 4: Simulate successful registration
            // In real implementation this would be:
            // 1. connection_manager.connect().await
            // 2. connection_manager.register(credentials).await
            // 3. Auto-login after registration
            
            // Send registration success notification
            let _ = action_tx.send(Action::RegistrationSuccess(creds.username.clone()));
            
            // Step 5: Small delay before auto-login
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            
            // Step 6: Auto-login after successful registration
            let auth_result = Action::AuthenticationSuccess(AuthState::Authenticated {
                profile: crate::auth::UserProfile {
                    id: uuid::Uuid::new_v4(),
                    username: creds.username.clone(),
                    roles: vec!["user".to_string()],
                },
                session: crate::auth::Session {
                    id: uuid::Uuid::new_v4(),
                    token: format!("reg_token_{}", creds.username),
                    created_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    expires_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() + 3600, // 1 hour expiration
                },
            });
            
            let _ = action_tx.send(auth_result);
        });
    }
    
    /// Modern message sending using ConnectionManager
    fn handle_modern_send_message(&mut self, message: String) {
        // Use modern ConnectionManager to get status
        let connection_status = self.get_connection_status();
        
        if connection_status == crate::transport::ConnectionStatus::CONNECTED {
            info!("Sending message via legacy transport (connection_status: {:?}): '{}'", connection_status, message);
            
            // Use legacy transport directly since it's what's actually connected
            // TODO: Replace with ConnectionManager when fully integrated
            #[allow(deprecated)]
            use crate::transport::add_outgoing_message;
            #[allow(deprecated)]
            add_outgoing_message(message.clone());
            
            // Update status bar message count
            self.status_bar.record_sent_message();
            
            // Note: Legacy transport will add "You: {message}" to display automatically
            info!("Message queued successfully via legacy transport: {}", message);
        } else {
            warn!("Cannot send message - client not connected (status: {:?}): {}", connection_status, message);
            // Use modern error handling for connection failures
            self.home_component.add_message_to_room(
                format!("Error: Cannot send message - not connected (status: {:?})", connection_status),
                false
            );
        }
    }

    /// Set up message observer for ConnectionManager
    fn setup_connection_observer(&mut self) -> Result<()> {
        let observer = Arc::new(ChatMessageObserver::new(self.action_tx.clone()));
        self.connection_manager.register_observer(observer);
        tracing::info!("ConnectionManager message observer registered");
        Ok(())
    }

    /// Get current connection status from ConnectionManager
    /// Helper method to reduce code duplication
    fn get_connection_status(&self) -> crate::transport::ConnectionStatus {
        let cm_status = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.connection_manager.get_status().await
            })
        });
        
        // Also check legacy status for comparison
        #[allow(deprecated)]
        let legacy_status = crate::transport::CLIENT_STATUS.lock().unwrap().status.clone();
        
        debug!("Connection status check - ConnectionManager: {:?}, Legacy: {:?}", cm_status, legacy_status);
        
        // For now, use legacy status since that's what's actually connected
        // TODO: Remove this when ConnectionManager is fully integrated
        legacy_status
    }

    fn draw(&mut self, tui: &mut Tui) -> Result<()> {
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
                    use ratatui::{widgets::{Block, Borders, Paragraph}, style::{Color, Style}};
                    let message = match self.login_screen.mode {
                        crate::components::auth::LoginMode::Register => "Creating account and connecting...",
                        crate::components::auth::LoginMode::Login => "Authenticating and connecting...",
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
                            Constraint::Length(1),  // Status bar
                            Constraint::Min(0),     // Main content
                            Constraint::Length(1),  // FPS counter
                        ])
                        .split(area);

                    // Update status bar with current state
                    self.status_bar.set_auth_state(self.auth_state.clone());
                    
                    // Use modern ConnectionManager to get status
                    let connection_status = self.get_connection_status();
                    self.status_bar.set_connection_status(connection_status);
                    
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

    fn handle_login_with_server(&mut self, credentials: crate::auth::Credentials, server_address: String) {
        self.auth_state = AuthState::Authenticating;
        self.auth_status.update_state(self.auth_state.clone());
        
        tokio::spawn({
            let tx = self.action_tx.clone();
            let creds = credentials.clone();
            let server_addr = server_address.clone();
            async move {
                // DEPRECATED: Using legacy transport and compatibility layer
                // TODO: Replace with direct ConnectionManager usage in v0.6.0
                #[allow(deprecated)]
                use crate::transport::{CLIENT_STATUS, ConnectionStatus, add_text_message};
                #[allow(deprecated)]
                use crate::compatibility_layer::connect_client_compat;
                
                // Try to connect to the server first
                let address: Result<std::net::SocketAddr, _> = server_addr.parse();
                match address {
                    Ok(addr) => {
                        let input = tui_input::Input::default();
                        #[allow(deprecated)]
                        match connect_client_compat(input, addr).await {
                            Ok(()) => {
                                #[allow(deprecated)]
                                {
                                    use crate::transport::{CLIENT_STATUS, ConnectionStatus};
                                    CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::CONNECTED;
                                }
                                info!("Successfully connected to server at {}", server_addr);
                                
                                // Add small delay to ensure connection is stable
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                
                                // Send authentication request to server
                                #[allow(deprecated)]
                                use crate::compatibility_layer::authenticate_compat;
                                
                                // Send authentication request
                                #[allow(deprecated)]
                                match authenticate_compat(creds.username.clone(), creds.password.clone()).await {
                                    Ok(()) => {
                                        info!("Authentication request sent for user: {}", creds.username);
                                        // Start authentication response monitoring
                                        let tx_clone = tx.clone();
                                        let username = creds.username.clone();
                                        
                                        tokio::spawn(async move {
                                            match wait_for_auth_response(username.clone()).await {
                                                Ok(auth_state) => {
                                                    info!("Authentication successful for user: {}", username);
                                                    let _ = tx_clone.send(Action::AuthenticationSuccess(auth_state));
                                                }
                                                Err(error_msg) => {
                                                    warn!("Authentication failed for user {}: {}", username, error_msg);
                                                    let _ = tx_clone.send(Action::AuthenticationFailure(error_msg));
                                                }
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        error!("Failed to send authentication request for {}: {}", creds.username, e);
                                        #[allow(deprecated)]
                                        add_text_message(format!("Authentication failed: {}", e));
                                        let _ = tx.send(Action::AuthenticationFailure("Failed to send authentication request".to_string()));
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Connection failed to {}: {}", server_addr, e);
                                add_text_message(format!("Failed to connect to {}: {}", server_addr, e));
                                add_text_message("Authentication failed - could not connect to server".to_string());
                                add_text_message("Retrying connection may help if server is starting up...".to_string());
                                add_text_message("Start the server with: cargo run --bin lair-chat-server".to_string());
                                let detailed_error = format!("Connection to {} failed: {}. This could be due to: (1) Server not running - start with 'cargo run --bin lair-chat-server', (2) Server starting up - wait a moment and retry, (3) Port already in use, (4) Firewall blocking connection, (5) Server crashed or not listening properly.", server_addr, e);
                                let _ = tx.send(Action::AuthenticationFailure(detailed_error));
                            }
                        }
                    }
                    Err(_) => {
                        add_text_message(format!("Invalid server address: {}", server_addr));
                        let _ = tx.send(Action::AuthenticationFailure("Invalid server address format".to_string()));
                    }
                }
            }
        });
    }

    fn handle_register_with_server(&mut self, credentials: crate::auth::Credentials, server_address: String) {
        self.auth_state = AuthState::Authenticating;
        self.auth_status.update_state(self.auth_state.clone());
        
        tokio::spawn({
            let tx = self.action_tx.clone();
            let creds = credentials.clone();
            let server_addr = server_address.clone();
            async move {
                use crate::transport::{CLIENT_STATUS, ConnectionStatus, add_text_message};
                use crate::compatibility_layer::connect_client_compat;
                
                // Try to connect to the server first
                let address: Result<std::net::SocketAddr, _> = server_addr.parse();
                match address {
                    Ok(addr) => {
                        let input = tui_input::Input::default();
                        match connect_client_compat(input, addr).await {
                            Ok(()) => {
                                CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::CONNECTED;
                                info!("Successfully connected to server at {} for registration", server_addr);
                                
                                // Add small delay to ensure connection is stable
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                
                                // Send registration request
                                match register_compat(creds.username.clone(), creds.password.clone()).await {
                                    Ok(()) => {
                                        info!("Registration request sent for user: {}", creds.username);
                                        // Start registration response monitoring
                                        let tx_clone = tx.clone();
                                        let username = creds.username.clone();
                                        
                                        tokio::spawn(async move {
                                            match wait_for_auth_response(username.clone()).await {
                                                Ok(auth_state) => {
                                                    info!("Registration and authentication successful for user: {}", username);
                                                    let _ = tx_clone.send(Action::AuthenticationSuccess(auth_state));
                                                }
                                                Err(error_msg) => {
                                                    warn!("Registration/authentication failed for user {}: {}", username, error_msg);
                                                    let _ = tx_clone.send(Action::AuthenticationFailure(error_msg));
                                                }
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        error!("Failed to send registration request for {}: {}", creds.username, e);
                                        add_text_message(format!("Registration failed: {}", e));
                                        let _ = tx.send(Action::AuthenticationFailure("Failed to send registration request".to_string()));
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Registration connection failed to {}: {}", server_addr, e);
                                add_text_message(format!("Failed to connect to {}: {}", server_addr, e));
                                add_text_message("Registration failed - could not connect to server".to_string());
                                add_text_message("Retrying connection may help if server is starting up...".to_string());
                                add_text_message("Start the server with: cargo run --bin lair-chat-server".to_string());
                                let detailed_error = format!("Connection to {} failed: {}. This could be due to: (1) Server not running - start with 'cargo run --bin lair-chat-server', (2) Server starting up - wait a moment and retry, (3) Port already in use, (4) Firewall blocking connection, (5) Server crashed or not listening properly.", server_addr, e);
                                let _ = tx.send(Action::AuthenticationFailure(detailed_error));
                            }
                        }
                    }
                    Err(_) => {
                        add_text_message(format!("Invalid server address: {}", server_addr));
                        let _ = tx.send(Action::AuthenticationFailure("Invalid server address format".to_string()));
                    }
                }
            }
        });
    }
}

/// Wait for and parse authentication response from server
async fn wait_for_auth_response(username: String) -> Result<crate::auth::AuthState, String> {
    use crate::transport::MESSAGES;
    use crate::auth::{UserProfile, Session};
    use uuid::Uuid;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    info!("Starting authentication response wait for user: {}", username);
    
    // Monitor incoming messages for authentication response
    for attempt in 0..200 { // Wait up to 20 seconds (200 * 100ms)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Log progress every 5 seconds
        if attempt % 50 == 0 && attempt > 0 {
            info!("Authentication wait progress: {} seconds elapsed for user {}", attempt / 10, username);
        }
        
        // Check both legacy MESSAGES and modern message system
        let mut recent_messages: Vec<String> = Vec::new();
        
        // Get from legacy system
        let legacy_messages = MESSAGES.lock().unwrap();
        recent_messages.extend(legacy_messages.text.iter().rev().take(5).cloned());
        drop(legacy_messages);
        
        // Also check if there's a global way to access recent messages from modern system
        // For now, we'll primarily rely on the action system and legacy fallback
        
        let mut success_found = false;
        let _failure_found = false;
        
        for message in &recent_messages {
            debug!("Checking auth message for {}: {}", username, message);
            
            // Check for authentication success indicators - handle both login and registration
            if message.contains("Welcome back") || 
               message.contains(&format!("Welcome back, {}", username)) ||
               message.contains(&format!("{} has joined", username)) ||
               message.contains("Registration successful") {
                info!("Authentication success detected for user: {}", username);
                success_found = true;
            }
            
            // Check for authentication failure indicators
            if message.contains("Authentication failed") || 
               message.contains("Login failed") ||
               message.contains("Registration failed") ||
               message.contains("Internal error") ||
               message.contains("Error:") {
                error!("Authentication failure detected for user {}: {}", username, message);
            }
        }
        
        // Success indicators from server always take priority over client-side errors
        if success_found {
            info!("Server authentication SUCCESS detected for user {}", username);
            
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            let profile = UserProfile {
                id: Uuid::new_v4(),
                username: username.clone(),
                roles: vec!["user".to_string()],
            };
            
            let session = Session {
                id: Uuid::new_v4(),
                token: format!("session_{}", username),
                created_at: now,
                expires_at: now + 3600,
            };
            
            return Ok(crate::auth::AuthState::Authenticated { profile, session });
        } 
        
        // Only treat as failure if we see server-side failure messages (not client errors)
        let server_failure = recent_messages.iter().any(|msg| {
            msg.contains("Authentication failed:") ||  // Server sends this format
            msg.contains("Login failed") ||
            msg.contains("Registration failed")
            // Exclude "Internal error" and generic "Error:" as these are often client-side
        });
        
        if server_failure {
            error!("Server authentication FAILURE detected for user {}", username);
            let failure_message = recent_messages.iter()
                .find(|msg| msg.contains("Authentication failed:") || 
                           msg.contains("Login failed") ||
                           msg.contains("Registration failed"))
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "Server rejected authentication".to_string());
            return Err(failure_message);
        }
    }
    
    error!("Authentication timeout after 20 seconds for user: {}", username);
    Err("Authentication timeout - no response from server after 20 seconds".to_string())
}

/// Send registration request to server
async fn register_compat(username: String, password: String) -> Result<(), crate::transport::TransportError> {
    use crate::transport::{add_silent_outgoing_message, add_text_message};
    
    let auth_request = serde_json::json!({
        "username": username,
        "password": password,
        "fingerprint": "client_device_fingerprint",
        "is_registration": true
    });
    
    add_text_message("Sending registration request...".to_string());
    add_silent_outgoing_message(auth_request.to_string());
    
    Ok(())
}