use std::io;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::client::action::Action;
use crate::client::auth::{AuthState, Credentials};
use crate::client::components::{Component, Frame};
use crate::client::components::auth::{AuthStatusBar, LoginScreen};
use crate::client::components::chat::ChatView;
use crate::client::components::status::StatusBar;
use crate::client::transport::ConnectionStatus;
use crossterm::event::KeyEvent;

/// Main application state and UI component
pub struct App {
    /// Current authentication state
    auth_state: AuthState,
    /// Login screen component
    login_screen: LoginScreen,
    /// Authentication status bar
    auth_status: AuthStatusBar,
    /// Chat view component
    chat_view: ChatView,
    /// Status bar component
    status_bar: StatusBar,
    /// Whether the application should exit
    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            auth_state: AuthState::Unauthenticated,
            login_screen: LoginScreen::new(),
            auth_status: AuthStatusBar::new(),
            chat_view: ChatView::new(),
            status_bar: StatusBar::new(),
            should_quit: false,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Update the authentication state
    pub fn update_auth_state(&mut self, state: AuthState) {
        self.auth_state = state.clone();
        self.auth_status.update_state(state.clone());
        self.status_bar.set_auth_state(state);
    }

    /// Handle successful authentication
    pub fn handle_auth_success(&mut self) {
        match &self.auth_state {
            AuthState::Authenticated { profile, .. } => {
                self.chat_view.set_username(profile.username.clone());
            }
            _ => {}
        }
    }

    /// Handle authentication error
    pub fn handle_auth_error(&mut self, error: String) {
        self.login_screen.handle_error(error.into());
    }
}

impl Component for App {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Global key handlers
        match key.code {
            crossterm::event::KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return Some(Action::Quit);
            }
            _ => {}
        }

        // Route key events based on authentication state
        match self.auth_state {
            AuthState::Unauthenticated | AuthState::Failed { .. } => {
                self.login_screen.handle_key(key)
            }
            AuthState::Authenticated { .. } => {
                self.chat_view.handle_key(key)
            }
            AuthState::Authenticating => None,
        }
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> io::Result<()> {
        // Create the layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Status bar
                Constraint::Min(0),     // Main content
                Constraint::Length(1),  // Bottom status bar
            ])
            .split(area);

        // Draw the auth status
        self.auth_status.draw(f, chunks[0])?;

        // Draw the main content based on authentication state
        // Draw main content
        match self.auth_state {
            AuthState::Unauthenticated | AuthState::Failed { .. } => {
                self.login_screen.draw(f, chunks[1])?;
            }
            AuthState::Authenticated { .. } => {
                self.chat_view.draw(f, chunks[1])?;
            }
            AuthState::Authenticating => {
                // Show loading screen
                let loading = Paragraph::new("Authenticating...")
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(loading, chunks[1]);
            }
        }

        // Draw the status bar
        self.status_bar.draw(f, chunks[2])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use uuid::Uuid;

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert!(!app.should_quit());
        assert!(matches!(app.auth_state, AuthState::Unauthenticated));
    }

    #[test]
    fn test_app_quit() {
        let mut app = App::new();
        assert!(!app.should_quit());
        
        app.quit();
        assert!(app.should_quit());
    }

    #[test]
    fn test_auth_state_update() {
        let mut app = App::new();
        
        let profile = crate::client::auth::UserProfile {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            roles: vec!["user".to_string()],
        };
        
        let session = crate::client::auth::Session {
            id: Uuid::new_v4(),
            token: "test_token".to_string(),
            created_at: 0,
            expires_at: u64::MAX,
        };
        
        app.update_auth_state(AuthState::Authenticated {
            profile,
            session,
        });
        
        match app.auth_state {
            AuthState::Authenticated { ref profile, .. } => {
                assert_eq!(profile.username, "testuser");
            }
            _ => panic!("Expected authenticated state"),
        }
    }

    #[test]
    fn test_ctrl_c_handling() {
        let mut app = App::new();
        
        let action = app.handle_key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        ));
        
        assert!(matches!(action, Some(Action::Quit)));
        assert!(app.should_quit());
    }
}