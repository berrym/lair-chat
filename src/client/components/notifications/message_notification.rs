//! Message notification overlay component for cross-conversation notifications

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use crate::{action::Action, components::Component};

/// Represents a single message notification
#[derive(Debug, Clone)]
pub struct MessageNotification {
    /// Unique identifier for this notification
    pub id: Uuid,
    /// Name of the message sender
    pub sender_name: String,
    /// Preview of the message content
    pub message_preview: String,
    /// ID of the conversation this message belongs to
    pub conversation_id: String,
    /// When this notification was created
    pub created_at: SystemTime,
    /// When this notification should auto-dismiss
    pub auto_dismiss_time: SystemTime,
}

impl MessageNotification {
    /// Create a new message notification
    pub fn new(
        sender_name: String,
        message_preview: String,
        conversation_id: String,
        auto_dismiss_duration: Duration,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4(),
            sender_name,
            message_preview,
            conversation_id,
            created_at: now,
            auto_dismiss_time: now + auto_dismiss_duration,
        }
    }

    /// Check if this notification should be dismissed
    pub fn should_dismiss(&self) -> bool {
        SystemTime::now() >= self.auto_dismiss_time
    }

    /// Get how much time is left before auto-dismiss
    pub fn time_remaining(&self) -> Option<Duration> {
        self.auto_dismiss_time
            .duration_since(SystemTime::now())
            .ok()
    }
}

/// Notification overlay component that shows temporary message notifications
pub struct NotificationOverlay {
    /// Queue of active notifications
    notifications: Vec<MessageNotification>,
    /// Maximum number of notifications to show at once
    max_notifications: usize,
    /// Whether the overlay is visible
    visible: bool,
    /// Whether notifications are enabled
    enabled: bool,
}

impl NotificationOverlay {
    /// Create a new notification overlay
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
            max_notifications: 3,
            visible: false,
            enabled: true,
        }
    }

    /// Add a new notification to the queue
    pub fn add_notification(&mut self, notification: MessageNotification) {
        if !self.enabled {
            return;
        }

        // Remove any existing notification from the same sender to avoid spam
        self.notifications
            .retain(|n| n.sender_name != notification.sender_name);

        // Add the new notification
        self.notifications.push(notification);

        // Keep only the most recent notifications
        if self.notifications.len() > self.max_notifications {
            self.notifications
                .drain(0..self.notifications.len() - self.max_notifications);
        }

        // Show the overlay when notifications are added
        self.visible = true;

        tracing::info!(
            "Added notification from {} - total notifications: {}",
            self.notifications.last().unwrap().sender_name,
            self.notifications.len()
        );
    }

    /// Remove expired notifications
    pub fn cleanup_expired(&mut self) {
        let before_count = self.notifications.len();
        self.notifications.retain(|n| !n.should_dismiss());

        if self.notifications.len() != before_count {
            tracing::debug!(
                "Cleaned up {} expired notifications",
                before_count - self.notifications.len()
            );
        }

        // Hide overlay if no notifications remain
        if self.notifications.is_empty() {
            self.visible = false;
        }
    }

    /// Dismiss all notifications
    pub fn dismiss_all(&mut self) {
        self.notifications.clear();
        self.visible = false;
    }

    /// Dismiss a specific notification by ID
    pub fn dismiss_notification(&mut self, id: Uuid) {
        self.notifications.retain(|n| n.id != id);
        if self.notifications.is_empty() {
            self.visible = false;
        }
    }

    /// Enable or disable notifications
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.dismiss_all();
        }
    }

    /// Check if the overlay should be visible
    pub fn is_visible(&self) -> bool {
        self.visible && !self.notifications.is_empty()
    }

    /// Get the current notification count
    pub fn notification_count(&self) -> usize {
        self.notifications.len()
    }

    /// Handle input events
    pub fn handle_input(&mut self, key: KeyEvent) -> bool {
        if !self.is_visible() {
            return false;
        }

        match key.code {
            KeyCode::Esc => {
                self.dismiss_all();
                true
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                self.dismiss_all();
                true
            }
            _ => false,
        }
    }

    /// Render the notification overlay
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if !self.is_visible() {
            return;
        }

        // Clean up expired notifications before rendering
        self.cleanup_expired();

        if self.notifications.is_empty() {
            return;
        }

        // Calculate overlay position (top-right corner)
        let overlay_width = 50.min(area.width.saturating_sub(4));
        let overlay_height = (self.notifications.len() * 3 + 2).min(15) as u16;

        let overlay_area = Rect {
            x: area.width.saturating_sub(overlay_width + 2),
            y: 1,
            width: overlay_width,
            height: overlay_height,
        };

        // Clear the area
        f.render_widget(Clear, overlay_area);

        // Create main block
        let block = Block::default()
            .title("📬 New Messages")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));

        f.render_widget(block, overlay_area);

        // Render individual notifications
        let inner_area = overlay_area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });

        let notification_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                self.notifications
                    .iter()
                    .map(|_| Constraint::Length(3))
                    .collect::<Vec<_>>(),
            )
            .split(inner_area);

        for (i, notification) in self.notifications.iter().enumerate() {
            if i >= notification_areas.len() {
                break;
            }

            self.render_notification(f, notification_areas[i], notification);
        }

        // Render dismiss hint at the bottom if there's space
        if overlay_area.height > 4 {
            let hint_area = Rect {
                x: overlay_area.x + 1,
                y: overlay_area.y + overlay_area.height - 2,
                width: overlay_area.width.saturating_sub(2),
                height: 1,
            };

            let hint = Paragraph::new("Press ESC or 'd' to dismiss")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);

            f.render_widget(hint, hint_area);
        }
    }

    /// Render a single notification
    fn render_notification(&self, f: &mut Frame, area: Rect, notification: &MessageNotification) {
        // Create notification layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(2)])
            .split(area);

        // Render sender line
        let sender_line = Line::from(vec![
            Span::styled("From: ", Style::default().fg(Color::Gray)),
            Span::styled(
                &notification.sender_name,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            // Add time remaining indicator
            if let Some(remaining) = notification.time_remaining() {
                Span::styled(
                    format!(" ({}s)", remaining.as_secs()),
                    Style::default().fg(Color::DarkGray),
                )
            } else {
                Span::styled("", Style::default())
            },
        ]);

        let sender_paragraph = Paragraph::new(sender_line);
        f.render_widget(sender_paragraph, chunks[0]);

        // Render message preview (truncate if too long)
        let max_preview_len = (area.width as usize).saturating_sub(4);
        let preview_text = if notification.message_preview.len() > max_preview_len {
            format!(
                "{}...",
                &notification.message_preview[..max_preview_len.saturating_sub(3)]
            )
        } else {
            notification.message_preview.clone()
        };

        let message_paragraph = Paragraph::new(preview_text)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });

        f.render_widget(message_paragraph, chunks[1]);
    }
}

impl Default for NotificationOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for NotificationOverlay {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if self.handle_input(key) {
            Ok(Some(Action::Render))
        } else {
            Ok(None)
        }
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
        self.render(f, area);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let notification = MessageNotification::new(
            "Alice".to_string(),
            "Hello there!".to_string(),
            "conv_123".to_string(),
            Duration::from_secs(5),
        );

        assert_eq!(notification.sender_name, "Alice");
        assert_eq!(notification.message_preview, "Hello there!");
        assert_eq!(notification.conversation_id, "conv_123");
        assert!(!notification.should_dismiss());
    }

    #[test]
    fn test_notification_overlay() {
        let mut overlay = NotificationOverlay::new();
        assert!(!overlay.is_visible());
        assert_eq!(overlay.notification_count(), 0);

        let notification = MessageNotification::new(
            "Bob".to_string(),
            "Test message".to_string(),
            "conv_456".to_string(),
            Duration::from_secs(5),
        );

        overlay.add_notification(notification);
        assert!(overlay.is_visible());
        assert_eq!(overlay.notification_count(), 1);

        overlay.dismiss_all();
        assert!(!overlay.is_visible());
        assert_eq!(overlay.notification_count(), 0);
    }

    #[test]
    fn test_notification_expiry() {
        let mut overlay = NotificationOverlay::new();

        // Create a notification that expires immediately
        let expired_notification = MessageNotification {
            id: Uuid::new_v4(),
            sender_name: "Test".to_string(),
            message_preview: "Test".to_string(),
            conversation_id: "test".to_string(),
            created_at: SystemTime::now(),
            auto_dismiss_time: SystemTime::now() - Duration::from_secs(1),
        };

        overlay.notifications.push(expired_notification);
        assert_eq!(overlay.notification_count(), 1);

        overlay.cleanup_expired();
        assert_eq!(overlay.notification_count(), 0);
        assert!(!overlay.is_visible());
    }

    #[test]
    fn test_max_notifications_limit() {
        let mut overlay = NotificationOverlay::new();
        overlay.max_notifications = 2;

        // Add 3 notifications
        for i in 0..3 {
            let notification = MessageNotification::new(
                format!("User{}", i),
                format!("Message {}", i),
                format!("conv_{}", i),
                Duration::from_secs(5),
            );
            overlay.add_notification(notification);
        }

        // Should only keep the last 2
        assert_eq!(overlay.notification_count(), 2);
        assert_eq!(overlay.notifications[0].sender_name, "User1");
        assert_eq!(overlay.notifications[1].sender_name, "User2");
    }
}
