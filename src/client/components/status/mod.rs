//! Status bar component for Lair-Chat
//! Provides comprehensive status information display.

use std::time::{Duration, Instant};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use ratatui::Frame;

use crate::auth::AuthState;
#[cfg(test)]
use crate::auth::{UserProfile, Session};
#[cfg(test)]
use uuid::Uuid;
use crate::{
    components::Component,
    transport::ConnectionStatus,
};

/// Network statistics
#[derive(Debug, Default, Clone)]
pub struct NetworkStats {
    /// Messages sent in current session
    pub messages_sent: u64,
    /// Messages received in current session
    pub messages_received: u64,
    /// Last message timestamp
    pub last_message_time: Option<Instant>,
    /// Connection uptime
    pub connected_since: Option<Instant>,
}

impl NetworkStats {
    /// Get connection duration if connected
    pub fn uptime(&self) -> Option<Duration> {
        self.connected_since.map(|time| time.elapsed())
    }

    /// Format uptime as human-readable string
    pub fn format_uptime(&self) -> String {
        if let Some(duration) = self.uptime() {
            let secs = duration.as_secs();
            let hours = secs / 3600;
            let minutes = (secs % 3600) / 60;
            let seconds = secs % 60;
            format!("{}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            "Not connected".to_string()
        }
    }

    /// Get time since last message
    pub fn time_since_last_message(&self) -> Option<Duration> {
        self.last_message_time.map(|time| time.elapsed())
    }
}

/// Status bar component
pub struct StatusBar {
    /// Current connection status
    connection_status: ConnectionStatus,
    /// Authentication state
    auth_state: AuthState,
    /// Current room/channel name
    current_room: Option<String>,
    /// Network statistics
    network_stats: NetworkStats,
    /// Current error message (if any)
    error_message: Option<String>,
    /// Error message timeout
    error_timeout: Option<Instant>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            connection_status: ConnectionStatus::DISCONNECTED,
            auth_state: AuthState::Unauthenticated,
            current_room: None,
            network_stats: NetworkStats::default(),
            error_message: None,
            error_timeout: None,
        }
    }

    /// Update connection status
    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
        match status {
            ConnectionStatus::CONNECTED => {
                if self.network_stats.connected_since.is_none() {
                    self.network_stats.connected_since = Some(Instant::now());
                }
            }
            ConnectionStatus::DISCONNECTED => {
                self.network_stats.connected_since = None;
            }
        }
    }

    /// Update authentication state
    pub fn set_auth_state(&mut self, state: AuthState) {
        self.auth_state = state;
    }

    /// Set current room/channel
    pub fn set_current_room(&mut self, room: Option<String>) {
        self.current_room = room;
    }

    /// Record a sent message
    pub fn record_sent_message(&mut self) {
        self.network_stats.messages_sent += 1;
        self.network_stats.last_message_time = Some(Instant::now());
    }

    /// Record a received message
    pub fn record_received_message(&mut self) {
        self.network_stats.messages_received += 1;
        self.network_stats.last_message_time = Some(Instant::now());
    }

    /// Show an error message for a duration
    pub fn show_error(&mut self, message: String, duration: Duration) {
        self.error_message = Some(message);
        self.error_timeout = Some(Instant::now() + duration);
    }

    /// Clear current error message
    fn check_error_timeout(&mut self) {
        if let Some(timeout) = self.error_timeout {
            if Instant::now() > timeout {
                self.error_message = None;
                self.error_timeout = None;
            }
        }
    }

    /// Get the connection status style and text
    fn connection_status_style(&self) -> (Style, &'static str) {
        match self.connection_status {
            ConnectionStatus::CONNECTED => (
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
                "Connected",
            ),
            ConnectionStatus::DISCONNECTED => (
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
                "Disconnected",
            ),
        }
    }

    /// Get the authentication status style and text
    fn auth_status_style(&self) -> (Style, String) {
        match &self.auth_state {
            AuthState::Unauthenticated => (
                Style::default().fg(Color::Yellow),
                "Not logged in".to_string(),
            ),
            AuthState::Authenticating => (
                Style::default().fg(Color::Yellow),
                "Logging in...".to_string(),
            ),
            AuthState::Authenticated { profile, .. } => (
                Style::default().fg(Color::Green),
                format!("Logged in as {}", profile.username),
            ),
            AuthState::Failed { reason } => (
                Style::default().fg(Color::Red),
                format!("Auth failed: {}", reason),
            ),
        }
    }
}

impl Component for StatusBar {
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
        self.check_error_timeout();

        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(15),  // Connection status
                Constraint::Length(25),  // Auth status
                Constraint::Length(15),  // Room
                Constraint::Length(25),  // Stats
                Constraint::Min(20),     // Error/message area
            ])
            .split(area);

        // Draw connection status
        let (conn_style, conn_text) = self.connection_status_style();
        let connection = Paragraph::new(Line::from(vec![
            Span::styled(conn_text, conn_style),
        ]));
        f.render_widget(connection, chunks[0]);

        // Draw auth status
        let (auth_style, auth_text) = self.auth_status_style();
        let auth = Paragraph::new(Line::from(vec![
            Span::styled(&auth_text, auth_style),
        ]));
        f.render_widget(auth, chunks[1]);

        // Draw room info
        let room_text = self.current_room
            .as_ref()
            .map(|r| format!("Room: {}", r))
            .unwrap_or_else(|| "No room".to_string());
        let room = Paragraph::new(room_text);
        f.render_widget(room, chunks[2]);

        // Draw stats
        let stats_text = format!(
            "Sent: {} | Recv: {} | Up: {}",
            self.network_stats.messages_sent,
            self.network_stats.messages_received,
            self.network_stats.format_uptime(),
        );
        let stats = Paragraph::new(stats_text);
        f.render_widget(stats, chunks[3]);

        // Draw error message if any
        if let Some(error) = &self.error_message {
            let error_msg = Paragraph::new(error.clone())
                .style(Style::default().fg(Color::Red));
            f.render_widget(error_msg, chunks[4]);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_network_stats() {
        let mut stats = NetworkStats::default();
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert!(stats.connected_since.is_none());
        
        // Test uptime formatting
        assert_eq!(stats.format_uptime(), "Not connected");
        stats.connected_since = Some(Instant::now());
        assert!(stats.format_uptime().contains(":"));
    }

    #[test]
    fn test_status_bar_updates() {
        let mut status_bar = StatusBar::new();
        
        // Test connection status
        status_bar.set_connection_status(ConnectionStatus::CONNECTED);
        assert_eq!(status_bar.connection_status, ConnectionStatus::CONNECTED);
        assert!(status_bar.network_stats.connected_since.is_some());
        
        // Test message recording
        status_bar.record_sent_message();
        status_bar.record_received_message();
        assert_eq!(status_bar.network_stats.messages_sent, 1);
        assert_eq!(status_bar.network_stats.messages_received, 1);
        
        // Test error handling
        status_bar.show_error("Test error".to_string(), Duration::from_secs(1));
        assert!(status_bar.error_message.is_some());
        assert!(status_bar.error_timeout.is_some());
    }

    #[test]
    fn test_auth_state_display() {
        let mut status_bar = StatusBar::new();
        
        // Test unauthenticated
        let (_style, text) = status_bar.auth_status_style();
        assert_eq!(text, "Not logged in");
        
        // Test authenticated
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
        status_bar.set_auth_state(AuthState::Authenticated {
            profile,
            session,
        });
        
        let (_, text) = status_bar.auth_status_style();
        assert!(text.contains("testuser"));
    }
}