//! Authentication UI components for Lair-Chat
//! Provides login, registration, and authentication status components.

mod login;
pub use login::{LoginMode, LoginScreen};

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

use crate::client::auth::AuthState;
use crate::client::components::{Component, Frame};

/// Displays current authentication status in the UI
pub struct AuthStatusBar {
    state: AuthState,
}

impl AuthStatusBar {
    pub fn new() -> Self {
        Self {
            state: AuthState::Unauthenticated,
        }
    }

    pub fn update_state(&mut self, state: AuthState) {
        self.state = state;
    }
}

impl Component for AuthStatusBar {
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> std::io::Result<()> {
        let (status_text, style) = match &self.state {
            AuthState::Unauthenticated => (
                "Not logged in",
                Style::default().fg(Color::Red),
            ),
            AuthState::Authenticating => (
                "Authenticating...",
                Style::default().fg(Color::Yellow),
            ),
            AuthState::Authenticated { profile, .. } => (
                profile.username.as_str(),
                Style::default().fg(Color::Green),
            ),
            AuthState::Failed { reason } => (
                reason.as_str(),
                Style::default().fg(Color::Red),
            ),
        };

        let status = Paragraph::new(Span::styled(status_text, style));
        f.render_widget(status, area);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::auth::{Session, UserProfile};
    use uuid::Uuid;

    #[test]
    fn test_auth_status_bar() {
        let mut status_bar = AuthStatusBar::new();
        
        // Test initial state
        assert!(matches!(status_bar.state, AuthState::Unauthenticated));

        // Test authenticated state
        let profile = UserProfile {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            roles: vec!["user".to_string()],
        };
        
        let session = Session {
            id: Uuid::new_v4(),
            token: "test_token".to_string(),
            created_at: 0,
            expires_at: u64::MAX,
        };

        status_bar.update_state(AuthState::Authenticated {
            profile,
            session,
        });

        match status_bar.state {
            AuthState::Authenticated { ref profile, .. } => {
                assert_eq!(profile.username, "testuser");
            }
            _ => panic!("Expected authenticated state"),
        }
    }
}