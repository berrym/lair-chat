//! Toast notification overlay component.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{Notification, NotificationLevel};

/// Configuration for toast rendering.
#[allow(dead_code)]
pub struct ToastConfig {
    /// Maximum width of a toast.
    pub max_width: u16,
    /// Position from right edge.
    pub margin_right: u16,
    /// Position from bottom edge.
    pub margin_bottom: u16,
    /// Spacing between toasts.
    pub spacing: u16,
}

impl Default for ToastConfig {
    fn default() -> Self {
        Self {
            max_width: 40,
            margin_right: 2,
            margin_bottom: 2,
            spacing: 0,
        }
    }
}

/// Get the icon/prefix for a notification level.
fn level_icon(level: NotificationLevel) -> &'static str {
    match level {
        NotificationLevel::Info => "ℹ",
        NotificationLevel::Success => "✓",
        NotificationLevel::Warning => "⚠",
        NotificationLevel::Error => "✗",
    }
}

/// Get the color for a notification level.
fn level_color(level: NotificationLevel) -> Color {
    match level {
        NotificationLevel::Info => Color::Cyan,
        NotificationLevel::Success => Color::Green,
        NotificationLevel::Warning => Color::Yellow,
        NotificationLevel::Error => Color::Red,
    }
}

/// Get the border color for a notification level.
fn level_border_color(level: NotificationLevel) -> Color {
    match level {
        NotificationLevel::Info => Color::DarkGray,
        NotificationLevel::Success => Color::Green,
        NotificationLevel::Warning => Color::Yellow,
        NotificationLevel::Error => Color::Red,
    }
}

/// Render toast notifications as overlays.
///
/// Toasts are rendered in the bottom-right corner, stacking upward.
pub fn render_toasts(
    frame: &mut Frame,
    area: Rect,
    notifications: &[Notification],
    config: &ToastConfig,
) {
    if notifications.is_empty() {
        return;
    }

    // Calculate positions for each toast, starting from bottom
    let mut y_offset = area.height.saturating_sub(config.margin_bottom);

    for notification in notifications.iter().rev() {
        // Calculate toast dimensions
        let icon = level_icon(notification.level);
        let content = format!(" {} {} ", icon, notification.message);
        let width = (content.len() as u16 + 2).min(config.max_width).max(10);
        let height = 3; // Border + content + border

        // Check if we have room for this toast
        if y_offset < height {
            break;
        }

        y_offset = y_offset.saturating_sub(height + config.spacing);

        // Calculate position (bottom-right corner)
        let x = area.width.saturating_sub(width + config.margin_right);
        let toast_area = Rect::new(x, y_offset, width, height);

        // Clear the area behind the toast
        frame.render_widget(Clear, toast_area);

        // Create styled content
        let color = level_color(notification.level);
        let border_color = level_border_color(notification.level);

        let line = Line::from(vec![
            Span::styled(
                format!(" {} ", icon),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                truncate_message(&notification.message, width.saturating_sub(6) as usize),
                Style::default().fg(Color::White),
            ),
            Span::raw(" "),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(Color::Black));

        let paragraph = Paragraph::new(line).block(block);

        frame.render_widget(paragraph, toast_area);
    }
}

/// Truncate a message to fit within a given width.
fn truncate_message(message: &str, max_len: usize) -> String {
    if message.len() <= max_len {
        message.to_string()
    } else if max_len > 3 {
        format!("{}...", &message[..max_len - 3])
    } else {
        message[..max_len].to_string()
    }
}

/// Render toasts with default configuration.
pub fn render_toasts_default(frame: &mut Frame, area: Rect, notifications: &[Notification]) {
    render_toasts(frame, area, notifications, &ToastConfig::default());
}
