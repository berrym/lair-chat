//! Login screen component.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::Action;

/// Login screen state.
pub struct LoginScreen {
    /// Current input mode.
    pub mode: LoginMode,
    /// Username input.
    pub username: String,
    /// Password input.
    pub password: String,
    /// Email input (for registration).
    pub email: String,
    /// Whether in registration mode.
    pub registering: bool,
    /// Current field index.
    field_index: usize,
}

/// Input mode for the login screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginMode {
    Normal,
    EditingUsername,
    EditingPassword,
    EditingEmail,
}

impl Default for LoginScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl LoginScreen {
    /// Create a new login screen.
    pub fn new() -> Self {
        Self {
            mode: LoginMode::Normal,
            username: String::new(),
            password: String::new(),
            email: String::new(),
            registering: false,
            field_index: 0,
        }
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match self.mode {
            LoginMode::Normal => self.handle_normal_key(key),
            LoginMode::EditingUsername => self.handle_editing_key(key, EditField::Username),
            LoginMode::EditingPassword => self.handle_editing_key(key, EditField::Password),
            LoginMode::EditingEmail => self.handle_editing_key(key, EditField::Email),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Tab | KeyCode::Down => {
                self.next_field();
                None
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.prev_field();
                None
            }
            KeyCode::Enter => {
                if self.field_index == self.max_field_index() {
                    // Submit
                    self.submit()
                } else {
                    // Edit current field
                    self.start_editing();
                    None
                }
            }
            KeyCode::Char('r') => {
                self.registering = !self.registering;
                self.field_index = 0;
                None
            }
            KeyCode::Char('i') | KeyCode::Char('e') => {
                self.start_editing();
                None
            }
            _ => None,
        }
    }

    fn handle_editing_key(&mut self, key: KeyEvent, field: EditField) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.mode = LoginMode::Normal;
                None
            }
            KeyCode::Enter => {
                self.mode = LoginMode::Normal;
                self.next_field();
                None
            }
            KeyCode::Backspace => {
                self.get_field_mut(field).pop();
                None
            }
            KeyCode::Char(c) => {
                self.get_field_mut(field).push(c);
                None
            }
            _ => None,
        }
    }

    fn get_field_mut(&mut self, field: EditField) -> &mut String {
        match field {
            EditField::Username => &mut self.username,
            EditField::Password => &mut self.password,
            EditField::Email => &mut self.email,
        }
    }

    fn start_editing(&mut self) {
        self.mode = match self.field_index {
            0 => LoginMode::EditingUsername,
            1 if self.registering => LoginMode::EditingEmail,
            1 => LoginMode::EditingPassword,
            2 if self.registering => LoginMode::EditingPassword,
            _ => LoginMode::Normal,
        };
    }

    fn next_field(&mut self) {
        self.field_index = (self.field_index + 1).min(self.max_field_index());
    }

    fn prev_field(&mut self) {
        self.field_index = self.field_index.saturating_sub(1);
    }

    fn max_field_index(&self) -> usize {
        if self.registering {
            3
        } else {
            2
        }
    }

    fn submit(&mut self) -> Option<Action> {
        if self.username.is_empty() || self.password.is_empty() {
            return None;
        }

        if self.registering {
            if self.email.is_empty() {
                return None;
            }
            Some(Action::Register {
                username: self.username.clone(),
                email: self.email.clone(),
                password: self.password.clone(),
            })
        } else {
            Some(Action::Login {
                username: self.username.clone(),
                password: self.password.clone(),
            })
        }
    }

    /// Render the login screen.
    pub fn render(&self, frame: &mut Frame, area: Rect, error: Option<&str>) {
        // Center the login box
        let popup_area = centered_rect(60, 50, area);

        // Clear background
        frame.render_widget(Clear, popup_area);

        let title = if self.registering {
            "Register"
        } else {
            "Login"
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        frame.render_widget(block, popup_area);

        let inner = inner_rect(popup_area);

        // Layout for fields
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(if self.registering {
                vec![
                    Constraint::Length(3), // Username
                    Constraint::Length(3), // Email
                    Constraint::Length(3), // Password
                    Constraint::Length(3), // Submit button
                    Constraint::Length(2), // Toggle hint
                    Constraint::Length(2), // Error
                    Constraint::Min(0),    // Space
                ]
            } else {
                vec![
                    Constraint::Length(3), // Username
                    Constraint::Length(3), // Password
                    Constraint::Length(3), // Submit button
                    Constraint::Length(2), // Toggle hint
                    Constraint::Length(2), // Error
                    Constraint::Min(0),    // Space
                ]
            })
            .split(inner);

        // Username field
        let username_style = self.field_style(0);
        let username_block = Block::default()
            .title("Username")
            .borders(Borders::ALL)
            .style(username_style);
        let username_text = if self.mode == LoginMode::EditingUsername {
            format!("{}|", self.username)
        } else {
            self.username.clone()
        };
        let username_para = Paragraph::new(username_text).block(username_block);
        frame.render_widget(username_para, chunks[0]);

        let mut chunk_idx = 1;

        // Email field (registration only)
        if self.registering {
            let email_style = self.field_style(1);
            let email_block = Block::default()
                .title("Email")
                .borders(Borders::ALL)
                .style(email_style);
            let email_text = if self.mode == LoginMode::EditingEmail {
                format!("{}|", self.email)
            } else {
                self.email.clone()
            };
            let email_para = Paragraph::new(email_text).block(email_block);
            frame.render_widget(email_para, chunks[chunk_idx]);
            chunk_idx += 1;
        }

        // Password field
        let password_idx = if self.registering { 2 } else { 1 };
        let password_style = self.field_style(password_idx);
        let password_block = Block::default()
            .title("Password")
            .borders(Borders::ALL)
            .style(password_style);
        let password_text = if self.mode == LoginMode::EditingPassword {
            format!("{}|", "*".repeat(self.password.len()))
        } else {
            "*".repeat(self.password.len())
        };
        let password_para = Paragraph::new(password_text).block(password_block);
        frame.render_widget(password_para, chunks[chunk_idx]);
        chunk_idx += 1;

        // Submit button
        let submit_idx = if self.registering { 3 } else { 2 };
        let submit_style = if self.field_index == submit_idx {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::Cyan)
        };
        let submit_text = if self.registering {
            "[ Register ]"
        } else {
            "[ Login ]"
        };
        let submit_para = Paragraph::new(submit_text)
            .style(submit_style)
            .alignment(Alignment::Center);
        frame.render_widget(submit_para, chunks[chunk_idx]);
        chunk_idx += 1;

        // Toggle hint
        let toggle_hint = if self.registering {
            "Press 'r' to switch to Login"
        } else {
            "Press 'r' to switch to Register"
        };
        let hint_para = Paragraph::new(toggle_hint)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(hint_para, chunks[chunk_idx]);
        chunk_idx += 1;

        // Error message
        if let Some(err) = error {
            let error_para = Paragraph::new(err)
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            frame.render_widget(error_para, chunks[chunk_idx]);
        }
    }

    fn field_style(&self, index: usize) -> Style {
        if self.field_index == index {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        }
    }
}

#[derive(Clone, Copy)]
enum EditField {
    Username,
    Password,
    Email,
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
