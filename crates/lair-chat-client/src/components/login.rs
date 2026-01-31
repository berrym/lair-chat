//! Login screen component.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::Action;

/// Which field is currently focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedField {
    Server,
    Username,
    Email,
    Password,
    Submit,
}

/// Login screen state.
pub struct LoginScreen {
    /// Server address input (TCP).
    pub server: String,
    /// Username input.
    pub username: String,
    /// Password input.
    pub password: String,
    /// Email input (for registration).
    pub email: String,
    /// Whether in registration mode.
    pub registering: bool,
    /// Currently focused field.
    pub focused: FocusedField,
}

impl Default for LoginScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl LoginScreen {
    /// Create a new login screen with default server address.
    pub fn new() -> Self {
        Self::with_server("127.0.0.1:8080".to_string())
    }

    /// Create a new login screen with a specific server address.
    pub fn with_server(server: String) -> Self {
        Self {
            server,
            username: String::new(),
            password: String::new(),
            email: String::new(),
            registering: false,
            focused: FocusedField::Server,
        }
    }

    /// Get the current server address.
    #[allow(dead_code)]
    pub fn server_addr(&self) -> &str {
        &self.server
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Global shortcuts
        match key.code {
            KeyCode::Esc => return Some(Action::Quit),
            KeyCode::F(1) => {
                // Toggle registration mode
                self.registering = !self.registering;
                self.focused = FocusedField::Username;
                return None;
            }
            _ => {}
        }

        // Ctrl+C to quit
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(Action::Quit);
        }

        // Tab to move to next field
        if key.code == KeyCode::Tab {
            self.focus_next();
            return None;
        }

        // Shift+Tab or Up to move to previous field
        if key.code == KeyCode::BackTab
            || (key.code == KeyCode::Up && self.focused != FocusedField::Username)
        {
            self.focus_prev();
            return None;
        }

        // Down arrow to move to next field
        if key.code == KeyCode::Down && self.focused != FocusedField::Submit {
            self.focus_next();
            return None;
        }

        // Handle input based on focused field
        match self.focused {
            FocusedField::Server => self.handle_text_input(key, &mut self.server.clone()),
            FocusedField::Username => self.handle_text_input(key, &mut self.username.clone()),
            FocusedField::Email => self.handle_text_input(key, &mut self.email.clone()),
            FocusedField::Password => self.handle_text_input(key, &mut self.password.clone()),
            FocusedField::Submit => self.handle_submit_key(key),
        }
    }

    fn handle_text_input(&mut self, key: KeyEvent, _field: &mut String) -> Option<Action> {
        match key.code {
            KeyCode::Char(c) => {
                // Type character into the focused field
                match self.focused {
                    FocusedField::Server => self.server.push(c),
                    FocusedField::Username => self.username.push(c),
                    FocusedField::Email => self.email.push(c),
                    FocusedField::Password => self.password.push(c),
                    FocusedField::Submit => {}
                }
                None
            }
            KeyCode::Backspace => {
                match self.focused {
                    FocusedField::Server => {
                        self.server.pop();
                    }
                    FocusedField::Username => {
                        self.username.pop();
                    }
                    FocusedField::Email => {
                        self.email.pop();
                    }
                    FocusedField::Password => {
                        self.password.pop();
                    }
                    FocusedField::Submit => {}
                }
                None
            }
            KeyCode::Enter => {
                // Move to next field, or submit if on last input field
                if self.focused == FocusedField::Password
                    || (!self.registering && self.focused == FocusedField::Username)
                {
                    // Try to submit if we have enough data
                    if self.can_submit() {
                        return self.submit();
                    }
                }
                self.focus_next();
                None
            }
            _ => None,
        }
    }

    fn handle_submit_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => self.submit(),
            _ => None,
        }
    }

    fn focus_next(&mut self) {
        self.focused = match self.focused {
            FocusedField::Server => FocusedField::Username,
            FocusedField::Username => {
                if self.registering {
                    FocusedField::Email
                } else {
                    FocusedField::Password
                }
            }
            FocusedField::Email => FocusedField::Password,
            FocusedField::Password => FocusedField::Submit,
            FocusedField::Submit => FocusedField::Server,
        };
    }

    fn focus_prev(&mut self) {
        self.focused = match self.focused {
            FocusedField::Server => FocusedField::Submit,
            FocusedField::Username => FocusedField::Server,
            FocusedField::Email => FocusedField::Username,
            FocusedField::Password => {
                if self.registering {
                    FocusedField::Email
                } else {
                    FocusedField::Username
                }
            }
            FocusedField::Submit => FocusedField::Password,
        };
    }

    fn can_submit(&self) -> bool {
        if self.server.is_empty() || self.username.is_empty() || self.password.is_empty() {
            return false;
        }
        if self.registering && self.email.is_empty() {
            return false;
        }
        true
    }

    fn submit(&mut self) -> Option<Action> {
        if !self.can_submit() {
            return None;
        }

        if self.registering {
            Some(Action::Register {
                server: self.server.clone(),
                username: self.username.clone(),
                email: self.email.clone(),
                password: self.password.clone(),
            })
        } else {
            Some(Action::Login {
                server: self.server.clone(),
                username: self.username.clone(),
                password: self.password.clone(),
            })
        }
    }

    /// Render the login screen.
    pub fn render(&self, frame: &mut Frame, area: Rect, error: Option<&str>) {
        // Center the login box - registration needs more height for the extra email field
        let height_percent = if self.registering { 70 } else { 60 };
        let popup_area = centered_rect(60, height_percent, area);

        // Clear background
        frame.render_widget(Clear, popup_area);

        let title = if self.registering {
            " Register - New Account "
        } else {
            " Login "
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        frame.render_widget(block, popup_area);

        let inner = inner_rect(popup_area);

        // Layout for fields
        let constraints = if self.registering {
            vec![
                Constraint::Length(3), // Server
                Constraint::Length(3), // Username
                Constraint::Length(3), // Email
                Constraint::Length(3), // Password
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Submit button
                Constraint::Length(1), // Spacer
                Constraint::Length(2), // Error
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Help text
                Constraint::Min(0),    // Space
            ]
        } else {
            vec![
                Constraint::Length(3), // Server
                Constraint::Length(3), // Username
                Constraint::Length(3), // Password
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Submit button
                Constraint::Length(1), // Spacer
                Constraint::Length(2), // Error
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Help text
                Constraint::Min(0),    // Space
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(constraints)
            .split(inner);

        let mut chunk_idx = 0;

        // Server field
        let server_focused = self.focused == FocusedField::Server;
        let server_style = if server_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let server_block = Block::default()
            .title(if server_focused {
                " Server (typing) "
            } else {
                " Server "
            })
            .borders(Borders::ALL)
            .style(server_style);
        let server_text = if server_focused {
            format!("{}|", self.server)
        } else if self.server.is_empty() {
            "e.g., 127.0.0.1:8080".to_string()
        } else {
            self.server.clone()
        };
        let server_text_style = if self.server.is_empty() && !server_focused {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };
        let server_para = Paragraph::new(server_text)
            .style(server_text_style)
            .block(server_block);
        frame.render_widget(server_para, chunks[chunk_idx]);
        chunk_idx += 1;

        // Username field
        let username_focused = self.focused == FocusedField::Username;
        let username_style = if username_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let username_block = Block::default()
            .title(if username_focused {
                " Username (typing) "
            } else {
                " Username "
            })
            .borders(Borders::ALL)
            .style(username_style);
        let username_text = if username_focused {
            format!("{}|", self.username)
        } else if self.username.is_empty() {
            "Enter your username...".to_string()
        } else {
            self.username.clone()
        };
        let username_text_style = if self.username.is_empty() && !username_focused {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };
        let username_para = Paragraph::new(username_text)
            .style(username_text_style)
            .block(username_block);
        frame.render_widget(username_para, chunks[chunk_idx]);
        chunk_idx += 1;

        // Email field (registration only)
        if self.registering {
            let email_focused = self.focused == FocusedField::Email;
            let email_style = if email_focused {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let email_block = Block::default()
                .title(if email_focused {
                    " Email (typing) "
                } else {
                    " Email "
                })
                .borders(Borders::ALL)
                .style(email_style);
            let email_text = if email_focused {
                format!("{}|", self.email)
            } else if self.email.is_empty() {
                "Enter your email...".to_string()
            } else {
                self.email.clone()
            };
            let email_text_style = if self.email.is_empty() && !email_focused {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            let email_para = Paragraph::new(email_text)
                .style(email_text_style)
                .block(email_block);
            frame.render_widget(email_para, chunks[chunk_idx]);
            chunk_idx += 1;
        }

        // Password field
        let password_focused = self.focused == FocusedField::Password;
        let password_style = if password_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let password_block = Block::default()
            .title(if password_focused {
                " Password (typing) "
            } else {
                " Password "
            })
            .borders(Borders::ALL)
            .style(password_style);
        let password_text = if password_focused {
            format!("{}|", "*".repeat(self.password.len()))
        } else if self.password.is_empty() {
            "Enter your password...".to_string()
        } else {
            "*".repeat(self.password.len())
        };
        let password_text_style = if self.password.is_empty() && !password_focused {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };
        let password_para = Paragraph::new(password_text)
            .style(password_text_style)
            .block(password_block);
        frame.render_widget(password_para, chunks[chunk_idx]);
        chunk_idx += 1;

        // Spacer
        chunk_idx += 1;

        // Submit button
        let submit_focused = self.focused == FocusedField::Submit;
        let can_submit = self.can_submit();
        let submit_style = if submit_focused {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if can_submit {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let submit_text = if self.registering {
            if submit_focused {
                ">>> Create Account <<<"
            } else {
                "[ Create Account ]"
            }
        } else if submit_focused {
            ">>> Login <<<"
        } else {
            "[ Login ]"
        };
        let submit_para = Paragraph::new(submit_text)
            .style(submit_style)
            .alignment(Alignment::Center);
        frame.render_widget(submit_para, chunks[chunk_idx]);
        chunk_idx += 1;

        // Spacer
        chunk_idx += 1;

        // Error message
        if let Some(err) = error {
            let error_para = Paragraph::new(err)
                .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            frame.render_widget(error_para, chunks[chunk_idx]);
        }
        chunk_idx += 1;

        // Spacer
        chunk_idx += 1;

        // Help text
        let help_text = if self.registering {
            "Tab/Enter: Next field | F1: Switch to Login | Esc: Quit"
        } else {
            "Tab/Enter: Next field | F1: Switch to Register | Esc: Quit"
        };
        let help_para = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(help_para, chunks[chunk_idx]);
    }
}

/// Create a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Get inner rectangle (remove border).
fn inner_rect(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}
