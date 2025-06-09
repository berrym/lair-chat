//! Chat view component for Lair-Chat
//! Provides message display and input functionality.


use std::sync::{Arc, Mutex};
use std::time::Duration;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use tui_input::Input;

use crate::action::Action;
use crate::components::{Component, Frame};
use crate::transport::{Message, MessageType};
use super::status::StatusBar;

pub struct ChatView {
    /// Input buffer for new messages
    input: Input,
    /// List of messages in the chat
    messages: Vec<Message>,
    /// Username of the current user
    username: Option<String>,
    /// Whether to show the help popup
    show_help: bool,
    /// Reference to status bar
    status_bar: Option<Arc<Mutex<StatusBar>>>,
}

impl ChatView {
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            messages: Vec::new(),
            username: None,
            show_help: false,
            status_bar: None,
        }
    }

    /// Set the status bar reference
    pub fn with_status_bar(&mut self, status_bar: Arc<Mutex<StatusBar>>) {
        self.status_bar = Some(status_bar);
    }

    pub fn set_username(&mut self, username: String) {
        self.username = Some(username);
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message.clone());
        
        // Update status bar
        if let Some(status_bar) = &self.status_bar {
            if let Ok(mut bar) = status_bar.try_lock() {
                match message.message_type {
                    MessageType::UserMessage => bar.record_sent_message(),
                    MessageType::ReceivedMessage => bar.record_received_message(),
                    MessageType::ErrorMessage => {
                        bar.show_error(message.content.clone(), Duration::from_secs(5));
                    }
                    _ => {}
                }
            }
        }
    }

    fn submit_message(&mut self) -> Option<Action> {
        let content = self.input.value().trim();
        if content.is_empty() {
            return None;
        }

        let message = content.to_string();
        self.input.reset();
        Some(Action::SendMessage(message))
    }

    fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
}

impl Component for ChatView {
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<Action> {
        match key.code {
            crossterm::event::KeyCode::Enter => self.submit_message(),
            crossterm::event::KeyCode::Char('h')
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.toggle_help();
                None
            }
            crossterm::event::KeyCode::Up => {
                // TODO: Implement history navigation
                None
            }
            crossterm::event::KeyCode::Down => {
                // TODO: Implement history navigation
                None
            }
            _ => {
                // Handle input character
                if let crossterm::event::KeyCode::Char(c) = key.code {
                    let current = self.input.value();
                    self.input = self.input.clone().with_value(format!("{}{}", current, c));
                } else if key.code == crossterm::event::KeyCode::Backspace {
                    let current = self.input.value();
                    if !current.is_empty() {
                        self.input = self.input.clone().with_value(current[..current.len()-1].to_string());
                    }
                }
                None
            }
        }
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),     // Messages
                Constraint::Length(3),  // Input
            ])
            .split(area);

        // Draw messages
        let messages: Vec<ListItem> = self
            .messages
            .iter()
            .map(|m| {
                let style = match m.message_type {
                    MessageType::SystemMessage => Style::default().fg(Color::Yellow),
                    MessageType::ErrorMessage => Style::default().fg(Color::Red),
                    _ => Style::default(),
                };
                ListItem::new(Line::from(vec![Span::styled(m.content.clone(), style)]))
            })
            .collect();

        let messages = List::new(messages).block(Block::default().borders(Borders::ALL).title("Chat"));
        f.render_widget(messages, chunks[0]);

        // Draw input box
        let input = Paragraph::new(self.input.value())
            .style(Style::default())
            .block(Block::default().borders(Borders::ALL).title("Message"));
        f.render_widget(input, chunks[1]);

        // Draw help popup if enabled
        if self.show_help {
            let help_text = vec![
                "Chat Commands:",
                "",
                "Enter    - Send message",
                "Ctrl+h   - Toggle help",
                "Ctrl+c   - Quit",
                "",
                "Press any key to close",
            ];

            let help_area = centered_rect(60, 40, area);
            let help = Paragraph::new(help_text.join("\n"))
                .block(Block::default().borders(Borders::ALL).title("Help"));
            f.render_widget(help, help_area);
        }

        Ok(())
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect
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
    use crate::transport::{Message, MessageType};
    // use crossterm::event::KeyEvent;

    #[test]
    fn test_chat_view_creation() {
        let view = ChatView::new();
        assert!(view.messages.is_empty());
        assert!(view.username.is_none());
        assert!(!view.show_help);
    }

    #[test]
    fn test_message_submission() {
        let mut view = ChatView::new();
        
        // Empty message should not create action
        assert!(view.submit_message().is_none());
        
        // Add text to input
        view.input = Input::new("Hello, world!".into());
        
        // Submit should create SendMessage action
        match view.submit_message() {
            Some(Action::SendMessage(msg)) => {
                assert_eq!(msg, "Hello, world!");
            }
            _ => panic!("Expected SendMessage action"),
        }
        
        // Input should be cleared
        assert!(view.input.value().is_empty());
    }

    #[test]
    fn test_help_toggle() {
        let mut view = ChatView::new();
        assert!(!view.show_help);
        
        // Toggle help on
        view.toggle_help();
        assert!(view.show_help);
        
        // Toggle help off
        view.toggle_help();
        assert!(!view.show_help);
    }

    #[test]
    fn test_message_display() {
        let mut view = ChatView::new();
        
        // Add different types of messages
        view.add_message(Message::system_message("System message".into()));
        view.add_message(Message::user_message("User message".into()));
        view.add_message(Message::error_message("Error message".into()));
        
        assert_eq!(view.messages.len(), 3);
        
        // Verify message types
        assert_eq!(view.messages[0].message_type, MessageType::SystemMessage);
        assert_eq!(view.messages[1].message_type, MessageType::UserMessage);
        assert_eq!(view.messages[2].message_type, MessageType::ErrorMessage);
    }
}