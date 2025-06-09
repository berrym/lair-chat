use color_eyre::Result;
use crossterm::event::KeyEvent;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::{
    action::Action,
    auth::{AuthManager, AuthState, storage::FileTokenStorage},
    components::{
        auth::{AuthStatusBar, LoginScreen},
        home::Home,
        fps::FpsCounter,
        Component
    },
    config::Config,
    tui::{Event, Tui},
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
    
    // Authentication components
    auth_manager: AuthManager,
    auth_state: AuthState,
    login_screen: LoginScreen,
    auth_status: AuthStatusBar,
    
    // Main application components
    home_component: Home,
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
        // Create a mock transport and token storage for now
        let config = crate::transport::ConnectionConfig {
            address: "127.0.0.1:8080".parse().unwrap(),
            timeout_ms: 5000,
        };
        let mock_transport = std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(crate::tcp_transport::TcpTransport::new(config)) as Box<dyn crate::transport::Transport + Send + Sync>));
        let token_storage = Box::new(FileTokenStorage::new()?) as Box<dyn crate::auth::storage::TokenStorage>;
        let auth_manager = AuthManager::new(mock_transport, token_storage);
        
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
            
            // Authentication components
            auth_manager,
            auth_state: AuthState::Unauthenticated,
            login_screen: LoginScreen::new(),
            auth_status: AuthStatusBar::new(),
            
            // Main components
            home_component: Home::new(),
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
            
            // Authentication actions
            Action::Login(credentials) => {
                // Keep old login for backward compatibility
                self.handle_login_with_server(credentials.clone(), "127.0.0.1:8080".to_string());
            }
            
            Action::Register(credentials) => {
                // Keep old register for backward compatibility  
                self.handle_register_with_server(credentials.clone(), "127.0.0.1:8080".to_string());
            }
            
            Action::LoginWithServer(credentials, server_address) => {
                self.handle_login_with_server(credentials.clone(), server_address.clone());
            }
            
            Action::RegisterWithServer(credentials, server_address) => {
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
                    info!("User {} authenticated successfully", profile.username);
                    
                    // Connection is already established during authentication
                    use crate::transport::add_text_message;
                    
                    // Add welcome message to chat
                    add_text_message(" ".to_string());
                    add_text_message(format!("Welcome to Lair Chat, {}!", profile.username));
                    add_text_message("You are now connected and ready to chat!".to_string());
                    add_text_message("Press '/' to start typing your first message.".to_string());
                    add_text_message(" ".to_string());
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
                // Handle message sending
                use crate::transport::{CLIENT_STATUS, ConnectionStatus, add_text_message, add_outgoing_message};
                let client_status = CLIENT_STATUS.lock().unwrap();
                
                if client_status.status == ConnectionStatus::CONNECTED {
                    // Add message to outgoing queue for server transmission
                    add_outgoing_message(message.clone());
                    
                    // Add message to local display with username
                    if let AuthState::Authenticated { ref profile, .. } = self.auth_state {
                        add_text_message(format!("{}: {}", profile.username, message));
                    } else {
                        add_text_message(format!("You: {}", message));
                    }
                    
                    info!("Message sent: {}", message);
                } else {
                    add_text_message("Cannot send message: Not connected to server".to_string());
                }
            }
            
            Action::ReceiveMessage(message) => {
                // Handle received messages
                use crate::transport::add_text_message;
                add_text_message(message.to_string());
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
                            Constraint::Length(1),  // Auth status bar
                            Constraint::Min(0),     // Main content
                            Constraint::Length(1),  // FPS counter
                        ])
                        .split(area);

                    // Draw auth status
                    if let Err(e) = self.auth_status.draw(frame, chunks[0]) {
                        debug!("Error drawing auth status: {}", e);
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
                                add_text_message(format!("Connected to server at {}", server_addr));
                                
                                // Mock successful authentication after connection
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                
                                use crate::auth::{UserProfile, Session};
                                use uuid::Uuid;
                                use std::time::{SystemTime, UNIX_EPOCH};
                                
                                let now = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();
                                
                                let profile = UserProfile {
                                    id: Uuid::new_v4(),
                                    username: creds.username,
                                    roles: vec!["user".to_string()],
                                };
                                
                                let session = Session {
                                    id: Uuid::new_v4(),
                                    token: "mock_session_token".to_string(),
                                    created_at: now,
                                    expires_at: now + 3600,
                                };
                                
                                let auth_state = AuthState::Authenticated { profile, session };
                                let _ = tx.send(Action::AuthenticationSuccess(auth_state));
                            }
                            Err(e) => {
                                add_text_message(format!("Failed to connect to {}: {}", server_addr, e));
                                add_text_message("Authentication failed - could not connect to server".to_string());
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
                                add_text_message(format!("Connected to server at {}", server_addr));
                                
                                // Mock successful registration after connection (longer delay)
                                let _ = tx.send(Action::RegistrationSuccess(creds.username.clone()));
                                tokio::time::sleep(tokio::time::Duration::from_millis(1200)).await;
                                
                                use crate::auth::{UserProfile, Session};
                                use uuid::Uuid;
                                use std::time::{SystemTime, UNIX_EPOCH};
                                
                                let now = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();
                                
                                let profile = UserProfile {
                                    id: Uuid::new_v4(),
                                    username: creds.username,
                                    roles: vec!["user".to_string()],
                                };
                                
                                let session = Session {
                                    id: Uuid::new_v4(),
                                    token: "mock_session_token".to_string(),
                                    created_at: now,
                                    expires_at: now + 3600,
                                };
                                
                                let auth_state = AuthState::Authenticated { profile, session };
                                let _ = tx.send(Action::AuthenticationSuccess(auth_state));
                            }
                            Err(e) => {
                                add_text_message(format!("Failed to connect to {}: {}", server_addr, e));
                                add_text_message("Registration failed - could not connect to server".to_string());
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