//! Error display component for Lair-Chat
//! Provides user-friendly error message display in the UI.

use std::time::{Duration, Instant};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{
    client::{
        components::{Component, Frame},
        errors::{ErrorCategory, FriendlyError},
    },
    errors::connection_error,
};

/// Component for displaying user-friendly error messages
pub struct ErrorDisplay {
    /// Current error message to display
    error: Option<FriendlyError>,
    /// When to auto-dismiss the error
    timeout: Option<Instant>,
    /// Whether to show detailed technical information
    show_details: bool,
    /// Default timeout duration
    default_timeout: Duration,
}

impl ErrorDisplay {
    /// Create a new error display component
    pub fn new() -> Self {
        Self {
            error: None,
            timeout: None,
            show_details: false,
            default_timeout: Duration::from_secs(5),
        }
    }

    /// Set the default timeout duration
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.default_timeout = duration;
        self
    }

    /// Check if an error is currently being displayed
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    /// Set an error to display
    pub fn set_error(&mut self, error: FriendlyError) {
        self.error = Some(error);
        self.timeout = Some(Instant::now() + self.default_timeout);
    }

    /// Set an error to display with a custom timeout
    pub fn set_error_with_timeout(&mut self, error: FriendlyError, timeout: Duration) {
        self.error = Some(error);
        self.timeout = Some(Instant::now() + timeout);
    }

    /// Clear the current error
    pub fn clear(&mut self) {
        self.error = None;
        self.timeout = None;
    }

    /// Toggle display of technical details
    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    /// Check if the error should be auto-dismissed
    fn check_timeout(&mut self) {
        if let Some(timeout) = self.timeout {
            if Instant::now() >= timeout {
                self.clear();
            }
        }
    }

    /// Get the style for the current error category
    fn error_style(&self) -> Style {
        match self.error.as_ref().map(|e| e.category) {
            Some(ErrorCategory::Connection) => Style::default().fg(Color::Yellow),
            Some(ErrorCategory::Authentication) => Style::default().fg(Color::Magenta),
            Some(ErrorCategory::Messaging) => Style::default().fg(Color::Cyan),
            Some(ErrorCategory::Data) => Style::default().fg(Color::Blue),
            Some(ErrorCategory::System) => Style::default().fg(Color::Red),
            _ => Style::default().fg(Color::Red),
        }
    }
}

impl Component for ErrorDisplay {
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<crate::action::Action> {
        if self.has_error() {
            match key.code {
                crossterm::event::KeyCode::Esc | 
                crossterm::event::KeyCode::Enter |
                crossterm::event::KeyCode::Char(' ') => {
                    self.clear();
                    None
                }
                crossterm::event::KeyCode::Tab => {
                    self.toggle_details();
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> std::io::Result<()> {
        self.check_timeout();
        
        if let Some(error) = &self.error {
            // Create a centered popup for the error
            let popup_area = centered_rect(60, 50, area);
            
            // Clear the background
            f.render_widget(Clear, popup_area);
            
            // Create the error message
            let title = format!("{} Error", error.category.name());
            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .style(self.error_style());
            
            let inner_area = block.inner(popup_area);
            f.render_widget(block, popup_area);
            
            // Split the inner area for message and details
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Min(3),   // Message
                    Constraint::Length(1), // Spacer
                    Constraint::Length(if self.show_details { 5 } else { 2 }),  // Details/controls
                ])
                .split(inner_area);
            
            // Render the message
            let message_text = if self.show_details {
                error.format_detailed()
            } else {
                error.format()
            };
            
            let message = Paragraph::new(message_text)
                .style(Style::default());
            f.render_widget(message, chunks[0]);
            
            // Render controls
            let controls = Paragraph::new(Line::from(vec![
                Span::styled("Press ", Style::default()),
                Span::styled("ESC", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(" to dismiss, ", Style::default()),
                Span::styled("Tab", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(" to toggle details", Style::default()),
            ]));
            f.render_widget(controls, chunks[2]);
        }
        
        Ok(())
    }
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let mut display = ErrorDisplay::new();
        assert!(!display.has_error());
        
        // Set an error
        let error = connection_error("Test connection error");
        display.set_error(error);
        assert!(display.has_error());
        
        // Test timeout check
        display.set_error_with_timeout(
            connection_error("Timeout test"),
            Duration::from_nanos(1),
        );
        assert!(display.has_error());
        
        // Force timeout check
        display.check_timeout();
        assert!(!display.has_error());
    }
    
    #[test]
    fn test_error_styling() {
        let mut display = ErrorDisplay::new();
        display.set_error(connection_error("Connection error"));
        
        let style = display.error_style();
        assert_eq!(style.fg, Some(Color::Yellow));
    }
    
    #[test]
    fn test_toggle_details() {
        let mut display = ErrorDisplay::new();
        assert!(!display.show_details);
        
        display.toggle_details();
        assert!(display.show_details);
        
        display.toggle_details();
        assert!(!display.show_details);
    }
}