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
                self.auth_state = AuthState::Authenticating;
                self.auth_status.update_state(self.auth_state.clone());
                
                // TODO: Actually authenticate with server
                // For now, simulate successful login
                tokio::spawn({
                    let tx = self.action_tx.clone();
                    let creds = credentials.clone();
                    async move {
                        // Simulate async authentication
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        
                        // Mock successful authentication
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
                            expires_at: now + 3600, // 1 hour
                        };
                        
                        let auth_state = AuthState::Authenticated { profile, session };
                        let _ = tx.send(Action::AuthenticationSuccess(auth_state));
                    }
                });
            }
            
            Action::Register(credentials) => {
                self.auth_state = AuthState::Authenticating;
                self.auth_status.update_state(self.auth_state.clone());
                
                // TODO: Actually register with server
                // For now, simulate successful registration and login
                tokio::spawn({
                    let tx = self.action_tx.clone();
                    let creds = credentials.clone();
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                        
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
                });
            }
            
            Action::Logout => {
                self.auth_state = AuthState::Unauthenticated;
                self.auth_status.update_state(self.auth_state.clone());
                self.mode = Mode::Authentication;
            }

            // New action for handling auth success
            Action::AuthenticationSuccess(auth_state) => {
                self.auth_state = auth_state.clone();
                self.auth_status.update_state(auth_state.clone());
                self.mode = Mode::Home;
                
                if let AuthState::Authenticated { ref profile, .. } = auth_state {
                    info!("User {} authenticated successfully", profile.username);
                }
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
                    // Show loading screen
                    use ratatui::{widgets::{Block, Borders, Paragraph}, style::{Color, Style}};
                    let loading = Paragraph::new("Authenticating...")
                        .style(Style::default().fg(Color::Yellow))
                        .block(Block::default().borders(Borders::ALL));
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
}