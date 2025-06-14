//! Status bar component for Lair-Chat
//! Provides comprehensive status information display.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::Frame;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::time::{Duration, Instant};

use crate::auth::AuthState;
#[cfg(test)]
use crate::auth::{Session, UserProfile};
use crate::{action::Action, components::Component, transport::ConnectionStatus};
use tokio::sync::mpsc::UnboundedSender;

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
    /// Current DM notification (if any)
    dm_notification: Option<String>,
    /// DM notification timeout
    dm_notification_timeout: Option<Instant>,
    /// Total unread DM count
    unread_dm_count: u32,
    /// Action sender for handling interactions
    action_tx: Option<UnboundedSender<Action>>,
    /// Area where DM count is rendered (for click detection)
    dm_count_area: Option<Rect>,
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
            dm_notification: None,
            dm_notification_timeout: None,
            unread_dm_count: 0,
            action_tx: None,
            dm_count_area: None,
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
        tracing::info!(
            "DEBUG: Status bar received message count updated to: {} at {:?}",
            self.network_stats.messages_received,
            Instant::now()
        );
    }

    /// Get the received message count
    pub fn get_received_count(&self) -> u64 {
        self.network_stats.messages_received
    }

    /// Get the sent message count
    pub fn get_sent_count(&self) -> u64 {
        self.network_stats.messages_sent
    }

    /// Show an error message for a duration
    pub fn show_error(&mut self, message: String, duration: Duration) {
        self.error_message = Some(message);
        self.error_timeout = Some(Instant::now() + duration);
    }

    /// Show a DM notification for a duration
    pub fn show_dm_notification(&mut self, sender: String, duration: Duration) {
        self.dm_notification = Some(format!("💬 New DM from {}", sender));
        self.dm_notification_timeout = Some(Instant::now() + duration);
    }

    /// Update the unread DM count
    pub fn set_unread_dm_count(&mut self, count: u32) {
        self.unread_dm_count = count;
    }

    /// Get the current unread DM count
    pub fn get_unread_dm_count(&self) -> u32 {
        self.unread_dm_count
    }

    /// Clear current error message
    /// Check if error message has timed out
    fn check_error_timeout(&mut self) {
        if let Some(timeout) = self.error_timeout {
            if Instant::now() >= timeout {
                self.error_message = None;
                self.error_timeout = None;
            }
        }
    }

    /// Check if DM notification has timed out
    fn check_dm_notification_timeout(&mut self) {
        if let Some(timeout) = self.dm_notification_timeout {
            if Instant::now() >= timeout {
                self.dm_notification = None;
                self.dm_notification_timeout = None;
            }
        }
    }

    /// Get the connection indicator style and symbol
    fn connection_indicator_style(&self) -> (Style, &'static str) {
        match self.connection_status {
            ConnectionStatus::CONNECTED => (
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
                "●ONLINE",
            ),
            ConnectionStatus::DISCONNECTED => (
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                "●OFFLINE",
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
                format!("👤 {}", profile.username),
            ),
            AuthState::Failed { reason } => (
                Style::default().fg(Color::Red),
                format!("❌ Auth failed: {}", reason),
            ),
        }
    }
}

impl Component for StatusBar {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> color_eyre::Result<Option<Action>> {
        if let MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            ..
        } = mouse
        {
            // Check if click is within DM count area
            if let Some(dm_area) = self.dm_count_area {
                if column >= dm_area.x
                    && column < dm_area.x + dm_area.width
                    && row >= dm_area.y
                    && row < dm_area.y + dm_area.height
                    && self.unread_dm_count > 0
                {
                    return Ok(Some(Action::OpenDMNavigation));
                }
            }
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
        self.check_error_timeout();
        self.check_dm_notification_timeout();

        // Create layout with better spacing to prevent overlapping
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(10),     // Connection indicator (more compact)
                Constraint::Length(20),     // Auth status (reduced width)
                Constraint::Percentage(12), // Room (percentage-based)
                Constraint::Length(15),     // DM count (fixed width)
                Constraint::Percentage(30), // Stats (percentage-based, reduced)
                Constraint::Min(10),        // Error/message area (flexible)
            ])
            .split(area);

        // Draw connection status with clear indicator
        let (conn_style, conn_indicator) = self.connection_indicator_style();
        let connection = Paragraph::new(Line::from(vec![Span::styled(conn_indicator, conn_style)]));
        f.render_widget(connection, chunks[0]);

        // Draw auth status with explicit truncation if needed
        let (auth_style, auth_text) = self.auth_status_style();
        let auth_display = if auth_text.len() > 23 {
            format!("{}…", &auth_text[..22])
        } else {
            auth_text.clone()
        };
        let auth = Paragraph::new(Line::from(vec![Span::styled(&auth_display, auth_style)]));
        f.render_widget(auth, chunks[1]);

        // Draw room info with truncation to prevent overflow
        let room_text = self
            .current_room
            .as_ref()
            .map(|r| {
                let max_len = chunks[2].width.saturating_sub(2) as usize;
                if r.len() > max_len.saturating_sub(2) {
                    format!("{}…", &r[..max_len.saturating_sub(3)])
                } else {
                    r.clone()
                }
            })
            .unwrap_or_else(|| "No room".to_string());
        let room = Paragraph::new(room_text).style(Style::default().fg(Color::Cyan));
        f.render_widget(room, chunks[2]);

        // Draw DM unread count
        if self.unread_dm_count > 0 {
            let dm_text = format!("💬 {} (click)", self.unread_dm_count);
            let dm_widget = Paragraph::new(dm_text).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED), // Add underline to indicate clickable
            );
            f.render_widget(dm_widget, chunks[3]);
            // Store the area for click detection
            self.dm_count_area = Some(chunks[3]);
        } else {
            // Show empty space when no unread messages
            let dm_widget = Paragraph::new("").style(Style::default());
            f.render_widget(dm_widget, chunks[3]);
            self.dm_count_area = None;
        }

        // Draw stats with clear message counts and bold numbers
        let stats_text = Line::from(vec![
            Span::raw("Sent: "),
            Span::styled(
                self.network_stats.messages_sent.to_string(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | Recv: "),
            Span::styled(
                self.network_stats.messages_received.to_string(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | Up: "),
            Span::raw(self.network_stats.format_uptime()),
        ]);

        tracing::info!(
            "Status bar stats - Sent: {}, Recv: {}, Uptime: {}",
            self.network_stats.messages_sent,
            self.network_stats.messages_received,
            self.network_stats.format_uptime()
        );

        let stats = Paragraph::new(stats_text).alignment(Alignment::Left);
        f.render_widget(stats, chunks[4]);

        // Draw DM notification with priority over error messages
        if let Some(dm_notification) = &self.dm_notification {
            let dm_msg = Paragraph::new(dm_notification.clone()).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
            f.render_widget(dm_msg, chunks[5]);
        } else if let Some(error) = &self.error_message {
            let error_msg = Paragraph::new(error.clone()).style(Style::default().fg(Color::Red));
            f.render_widget(error_msg, chunks[5]);
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
        status_bar.set_auth_state(AuthState::Authenticated { profile, session });

        let (_, text) = status_bar.auth_status_style();
        assert!(text.contains("testuser"));
    }
}
