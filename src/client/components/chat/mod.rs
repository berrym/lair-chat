//! Chat view component for Lair-Chat
//! Provides message display and input functionality.

use std::collections::VecDeque;
use std::path::PathBuf;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use tui_input::Input;

use crate::client::action::Action;
use crate::client::components::{Component, Frame};
use crate::client::transport::Message;
use crate::client::history::CommandHistory;

pub struct ChatView {
    /// Input buffer for new messages
    input: Input,
    /// List of messages in the chat
    messages: Vec<Message>,
    /// Username of the current user
    username: Option<String>,
    /// Whether to show the help popup
    show_help: bool,
    /// Command history manager
    history: CommandHistory,
}

impl ChatView {
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            messages: Vec::new(),
            username: None,
            show_help: false,
            history: CommandHistory::new().unwrap_or_else(|_| CommandHistory {
                entries: VecDeque::new(),
                position: None,
                history_file: PathBuf::from("history.json"),
            }),
        }
    }

    pub fn set_username(&mut self, username: String) {
        self.username = Some(username);
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    fn submit_message(&mut self) -> Option<Action> {
        let content = self.input.value().trim();
        if content.is_empty() {
            return None;
        }

        let message = content.to_string();
        self.history.add(message.clone(), None);
        let _ = self.history.save();
        self.input.reset();
        self.history.reset_position();
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
                if let Some(previous) = self.history.previous() {
                    self.input = Input::new(previous.into());
                }
                None
            }
            crossterm::event::KeyCode::Down => {
                if let Some(next) = self.history.next() {
                    self.input = Input::new(next.into());
                } else {
                    self.input.reset();
                }
                None
            }
            _ => {
                self.input.handle_key_event(key);
                None
            }
        }
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> std::io::Result<()> {
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
                    crate::client::transport::MessageType::System => Style::default().fg(Color::Yellow),
                    crate::client::transport::MessageType::Error => Style::default().fg(Color::Red),
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
    use crate::client::transport::{Message, MessageType};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
        assert_eq!(view.messages[0].message_type, MessageType::System);
        assert_eq!(view.messages[1].message_type, MessageType::User);
        assert_eq!(view.messages[2].message_type, MessageType::Error);
    }
}