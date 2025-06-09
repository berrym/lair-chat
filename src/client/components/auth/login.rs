use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use tui_input::Input;

use color_eyre::Result;

use crate::auth::{AuthError, AuthState, Credentials};
use crate::components::Component;
use crate::action::Action;
use ratatui::Frame;

#[derive(Debug, Clone, PartialEq)]
pub enum LoginMode {
    Login,
    Register,
}

#[derive(Debug)]
pub struct LoginScreen {
    username: Input,
    password: Input,
    error_message: Option<String>,
    pub mode: LoginMode,
    focused_field: usize,
    processing: bool,
}

impl LoginScreen {
    pub fn new() -> Self {
        Self {
            username: Input::default(),
            password: Input::default(),
            error_message: None,
            mode: LoginMode::Login,
            focused_field: 0,
            processing: false,
        }
    }

    fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            LoginMode::Login => LoginMode::Register,
            LoginMode::Register => LoginMode::Login,
        };
        self.error_message = None;
    }

    fn submit(&mut self) -> Option<Action> {
        if self.username.value().is_empty() || self.password.value().is_empty() {
            self.error_message = Some("Username and password are required".to_string());
            return None;
        }

        let credentials = Credentials {
            username: self.username.value().to_string(),
            password: self.password.value().to_string(),
        };

        self.processing = true;
        match self.mode {
            LoginMode::Login => Some(Action::Login(credentials)),
            LoginMode::Register => Some(Action::Register(credentials)),
        }
    }

    pub fn handle_error(&mut self, error: AuthError) {
        self.processing = false;
        self.error_message = Some(error.to_string());
    }

    pub fn handle_auth_state(&mut self, state: &AuthState) {
        match state {
            AuthState::Failed { reason } => {
                self.processing = false;
                self.error_message = Some(reason.clone());
            }
            AuthState::Authenticating => {
                self.processing = true;
                self.error_message = None;
            }
            _ => {}
        }
    }
}

impl Component for LoginScreen {
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<Action> {
        if self.processing {
            return None;
        }

        match key.code {
            crossterm::event::KeyCode::Tab => {
                self.focused_field = (self.focused_field + 1) % 2;
                None
            }
            crossterm::event::KeyCode::BackTab => {
                self.focused_field = if self.focused_field == 0 { 1 } else { 0 };
                None
            }
            crossterm::event::KeyCode::Enter => {
                if self.focused_field == 1 {
                    self.submit()
                } else {
                    self.focused_field += 1;
                    None
                }
            }
            crossterm::event::KeyCode::Char('t') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.toggle_mode();
                None
            }
            crossterm::event::KeyCode::Char(c) => {
                match self.focused_field {
                    0 => { self.username = self.username.clone().with_value(format!("{}{}", self.username.value(), c)); },
                    1 => { self.password = self.password.clone().with_value(format!("{}{}", self.password.value(), c)); },
                    _ => {}
                }
                None
            }
            crossterm::event::KeyCode::Backspace => {
                match self.focused_field {
                    0 => { 
                        let value = self.username.value();
                        if !value.is_empty() {
                            self.username = self.username.clone().with_value(value[..value.len()-1].to_string());
                        }
                    },
                    1 => { 
                        let value = self.password.value();
                        if !value.is_empty() {
                            self.password = self.password.clone().with_value(value[..value.len()-1].to_string());
                        }
                    },
                    _ => {}
                }
                None
            }
            _ => None,
        }
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        // Create a larger centered box for the login form
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),  // Top padding
                Constraint::Length(18),      // Login form
                Constraint::Percentage(20),  // Bottom padding
            ])
            .split(area);

        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),  // Left padding
                Constraint::Percentage(50),  // Login form
                Constraint::Percentage(25),  // Right padding
            ])
            .split(vertical_chunks[1]);

        let form_area = horizontal_chunks[1];

        // Draw a clear background
        f.render_widget(Clear, form_area);

        // Create the main box with title
        let main_title = match self.mode {
            LoginMode::Login => "🔐 Lair Chat - Login",
            LoginMode::Register => "📝 Lair Chat - Register",
        };

        let main_block = Block::default()
            .title(main_title)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(main_block, form_area);

        // Split the form area into sections
        let form_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),   // Mode selection and instructions
                Constraint::Length(1),   // Spacer
                Constraint::Length(3),   // Username input
                Constraint::Length(3),   // Password input
                Constraint::Length(1),   // Spacer
                Constraint::Length(2),   // Instructions
                Constraint::Length(2),   // Status/Error
            ])
            .split(form_area);

        // Draw mode selection and instructions
        let mode_instruction = match self.mode {
            LoginMode::Login => "👤 Login Mode - Press Ctrl+T to switch to Register Mode",
            LoginMode::Register => "🆕 Register Mode - Press Ctrl+T to switch to Login Mode",
        };

        let mode_display = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Current Mode: ", Style::default().fg(Color::White)),
                Span::styled(
                    match self.mode {
                        LoginMode::Login => "LOGIN",
                        LoginMode::Register => "REGISTER",
                    },
                    Style::default()
                        .fg(match self.mode {
                            LoginMode::Login => Color::Cyan,
                            LoginMode::Register => Color::Green,
                        })
                        .add_modifier(Modifier::BOLD)
                ),
            ]),
            Line::from(Span::styled(
                mode_instruction,
                Style::default().fg(Color::Yellow)
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title("Authentication Mode"));
        f.render_widget(mode_display, form_chunks[0]);

        // Draw username field with better styling
        let username_title = if self.focused_field == 0 {
            "Username (FOCUSED - Type here)"
        } else {
            "Username"
        };

        let username_style = if self.focused_field == 0 {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let username_block = Block::default()
            .borders(Borders::ALL)
            .title(username_title)
            .border_style(if self.focused_field == 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            });

        let username_display = if self.username.value().is_empty() && self.focused_field == 0 {
            format!("{}_", self.username.value())
        } else {
            self.username.value().to_string()
        };

        let username_input = Paragraph::new(username_display)
            .style(username_style)
            .block(username_block);
        f.render_widget(username_input, form_chunks[2]);

        // Draw password field with better styling
        let password_title = if self.focused_field == 1 {
            "Password (FOCUSED - Type here)"
        } else {
            "Password"
        };

        let password_style = if self.focused_field == 1 {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let password_block = Block::default()
            .borders(Borders::ALL)
            .title(password_title)
            .border_style(if self.focused_field == 1 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            });

        let masked_password = if self.password.value().is_empty() && self.focused_field == 1 {
            "_".to_string()
        } else {
            "•".repeat(self.password.value().len())
        };
        let password_input = Paragraph::new(masked_password)
            .style(password_style)
            .block(password_block);
        f.render_widget(password_input, form_chunks[3]);

        // Draw navigation instructions
        let instructions = Paragraph::new(vec![
            Line::from("⌨️  Tab/Shift+Tab - Switch fields | Enter - Submit | Ctrl+T - Switch mode | Ctrl+C - Quit"),
        ])
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL).title("Controls"));
        f.render_widget(instructions, form_chunks[5]);

        // Draw status/error message
        if let Some(error) = &self.error_message {
            let error_msg = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("❌ Error: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::styled(error.as_str(), Style::default().fg(Color::Red)),
                ]),
            ])
            .block(Block::default().borders(Borders::ALL).title("Status"));
            f.render_widget(error_msg, form_chunks[6]);
        } else if self.processing {
            let status_msg = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                    Span::styled("Processing...", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]),
            ])
            .block(Block::default().borders(Borders::ALL).title("Status"));
            f.render_widget(status_msg, form_chunks[6]);
        } else {
            let ready_msg = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(Color::Green)),
                    Span::styled("Ready to authenticate", Style::default().fg(Color::Green)),
                ]),
            ])
            .block(Block::default().borders(Borders::ALL).title("Status"));
            f.render_widget(ready_msg, form_chunks[6]);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_login_screen_creation() {
        let screen = LoginScreen::new();
        assert_eq!(screen.mode, LoginMode::Login);
        assert_eq!(screen.focused_field, 0);
        assert!(!screen.processing);
    }

    #[test]
    fn test_mode_toggle() {
        let mut screen = LoginScreen::new();
        assert_eq!(screen.mode, LoginMode::Login);
        
        screen.toggle_mode();
        assert_eq!(screen.mode, LoginMode::Register);
        
        screen.toggle_mode();
        assert_eq!(screen.mode, LoginMode::Login);
    }

    #[test]
    fn test_field_focus() {
        let mut screen = LoginScreen::new();
        assert_eq!(screen.focused_field, 0);

        screen.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
        assert_eq!(screen.focused_field, 1);

        screen.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty()));
        assert_eq!(screen.focused_field, 0);
    }

    #[test]
    fn test_input_validation() {
        let mut screen = LoginScreen::new();
        
        // Empty fields should show error
        let action = screen.submit();
        assert!(action.is_none());
        assert!(screen.error_message.is_some());

        // Add username and password
        screen.username = Input::new("testuser".into());
        screen.password = Input::new("password123".into());
        
        // Should now return login action
        let action = screen.submit();
        assert!(action.is_some());
        match action.unwrap() {
            Action::Login(creds) => {
                assert_eq!(creds.username, "testuser");
                assert_eq!(creds.password, "password123");
            }
            _ => panic!("Expected Login action"),
        }
    }
}