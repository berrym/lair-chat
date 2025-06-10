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
    transport::ConnectionConfig,
    tui::{Event, Tui},
    tcp_transport::TcpTransport,
};

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
            
            // Authentication actions - Modern implementation
            Action::Login(credentials) => {
                self.handle_modern_login(credentials.clone());
            }
            
            Action::Register(credentials) => {
                self.handle_modern_register(credentials.clone());
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
                self.auth_state = AuthState::Unauthenticated;
                self.auth_status.update_state(self.auth_state.clone());
                self.mode = Mode::Authentication;
            }

            // New action for handling registration success
            Action::RegistrationSuccess(username) => {
                info!("User {} registered successfully", username);
                // Keep in authenticating state, will transition when auth completes
            }
            
            // New action for handling auth failure
            Action::AuthenticationFailure(error) => {
                self.auth_state = AuthState::Failed { reason: error.clone() };
                self.auth_status.update_state(self.auth_state.clone());
                self.login_screen.handle_error(crate::auth::AuthError::InternalError(error.clone()));
                self.mode = Mode::Authentication;
                info!("Authentication failed");
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
                    
                    // Use legacy status for now until ConnectionManager is fully integrated
                    #[allow(deprecated)]
                    self.status_bar.set_connection_status(crate::transport::CLIENT_STATUS.lock().unwrap().status.clone());
                    
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
                // Modern message handling - add to chat system
                self.home_component.add_message_to_room(message.to_string(), false);
                
                // Also add to legacy system for compatibility
                #[allow(deprecated)]
                crate::transport::add_text_message(message.to_string());
                
                // Update status bar message count
                self.status_bar.record_received_message();
                debug!("Received message: {}", message);
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
        
        // Enable authentication on connection manager
        self.connection_manager.with_auth();
        
        // Set state to authenticating immediately
        self.auth_state = AuthState::Authenticating;
        
        // Create a clone of the connection manager for the async task
        // to avoid borrowing issues in the main app loop
        let server_addr = "127.0.0.1:8080".to_string();
        let creds = credentials.clone();
        
        // Spawn async task to handle authentication without blocking
        tokio::spawn(async move {
            // For now, use a simplified approach that doesn't require mutable access
            // TODO: Implement full ConnectionManager async integration
            
            // Simulate authentication delay
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            // Send result back via action system
            let result = Action::AuthenticationSuccess(AuthState::Authenticated {
                profile: crate::auth::UserProfile {
                    id: uuid::Uuid::new_v4(),
                    username: creds.username.clone(),
                    roles: vec!["user".to_string()],
                },
                session: crate::auth::Session {
                    id: uuid::Uuid::new_v4(),
                    token: "mock_token".to_string(),
                    created_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    expires_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() + 3600,
                },
            });
            
            let _ = action_tx.send(result);
        });
    }
    
    /// Modern registration flow using ConnectionManager
    fn handle_modern_register(&mut self, credentials: Credentials) {
        let action_tx = self.action_tx.clone();
        
        // Enable authentication on connection manager
        self.connection_manager.with_auth();
        
        // Set state to authenticating immediately
        self.auth_state = AuthState::Authenticating;
        
        // Create clones for the async task to avoid borrowing issues
        let server_addr = "127.0.0.1:8080".to_string();
        let creds = credentials.clone();
        
        // Spawn async task to handle registration without blocking
        tokio::spawn(async move {
            // For now, use a simplified approach that doesn't require mutable access
            // TODO: Implement full ConnectionManager async integration
            
            // Simulate registration delay
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            
            // Send registration success followed by authentication
            let _ = action_tx.send(Action::RegistrationSuccess(creds.username.clone()));
            
            // Auto-login after successful registration
            let auth_result = Action::AuthenticationSuccess(AuthState::Authenticated {
                profile: crate::auth::UserProfile {
                    id: uuid::Uuid::new_v4(),
                    username: creds.username.clone(),
                    roles: vec!["user".to_string()],
                },
                session: crate::auth::Session {
                    id: uuid::Uuid::new_v4(),
                    token: "mock_token".to_string(),
                    created_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    expires_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() + 3600,
                },
            });
            
            let _ = action_tx.send(auth_result);
        });
    }
    
    /// Modern message sending using ConnectionManager
    fn handle_modern_send_message(&mut self, message: String) {
        // For now, use legacy transport system since it's proven to work
        // TODO: Complete ConnectionManager message sending integration
        #[allow(deprecated)]
        use crate::transport::{CLIENT_STATUS, ConnectionStatus, add_text_message, add_outgoing_message};
        
        #[allow(deprecated)]
        let client_status = CLIENT_STATUS.lock().unwrap();
        
        if client_status.status == ConnectionStatus::CONNECTED {
            // Add message to outgoing queue for server transmission
            #[allow(deprecated)]
            add_outgoing_message(message.clone());
            
            // Update status bar message count
            self.status_bar.record_sent_message();
            
            debug!("Message queued for sending: {}", message);
        } else {
            warn!("Cannot send message - client not connected: {}", message);
            #[allow(deprecated)]
            add_text_message("Cannot send message: Not connected to server".to_string());
        }
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
                    #[allow(deprecated)]
                    self.status_bar.set_connection_status(crate::transport::CLIENT_STATUS.lock().unwrap().status.clone());
                    
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
        
        let messages = MESSAGES.lock().unwrap();
        let recent_messages: Vec<String> = messages.text.iter().rev().take(10).cloned().collect();
        drop(messages);
        
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