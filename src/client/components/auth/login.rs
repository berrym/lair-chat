use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs};
use tui_input::Input;

use crate::client::auth::{AuthError, AuthState, Credentials};
use crate::client::components::{Component, Frame};
use crate::client::action::Action;

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
    mode: LoginMode,
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
                    0 => self.username.handle_key_event(key),
                    1 => self.password.handle_key_event(key),
                    _ => {}
                }
                None
            }
            crossterm::event::KeyCode::Backspace => {
                match self.focused_field {
                    0 => self.username.handle_key_event(key),
                    1 => self.password.handle_key_event(key),
                    _ => {}
                }
                None
            }
            _ => None,
        }
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> std::io::Result<()> {
        // Create a centered box for the login form
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(35),
                Constraint::Length(7),
                Constraint::Percentage(35),
            ])
            .split(area)[1];

        // Draw a clear background
        f.render_widget(Clear, area);

        // Create the main box
        let block = Block::default()
            .title(match self.mode {
                LoginMode::Login => "Login",
                LoginMode::Register => "Register",
            })
            .borders(Borders::ALL)
            .style(Style::default());

        // Split the area into sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1),  // Mode tabs
                Constraint::Length(1),  // Username
                Constraint::Length(1),  // Password
                Constraint::Length(1),  // Status/Error
            ])
            .split(area);

        // Draw the main box
        f.render_widget(block, area);

        // Draw mode tabs
        let mode_titles = vec!["Login", "Register"];
        let mode_spans: Vec<Line> = mode_titles
            .iter()
            .map(|t| {
                Line::from(vec![
                    Span::styled(
                        *t,
                        Style::default().add_modifier(
                            if (self.mode == LoginMode::Login && *t == "Login")
                                || (self.mode == LoginMode::Register && *t == "Register")
                            {
                                Modifier::REVERSED
                            } else {
                                Modifier::empty()
                            },
                        ),
                    )
                ])
            })
            .collect();

        let tabs = Tabs::new(mode_spans)
            .block(Block::default())
            .style(Style::default());
        f.render_widget(tabs, chunks[0]);

        // Draw username field
        let username_style = if self.focused_field == 0 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        
        let username_input = Paragraph::new(self.username.value())
            .style(username_style)
            .block(Block::default().borders(Borders::ALL).title("Username"));
        f.render_widget(username_input, chunks[1]);

        // Draw password field (masked)
        let password_style = if self.focused_field == 1 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        
        let masked_password = "*".repeat(self.password.value().len());
        let password_input = Paragraph::new(masked_password)
            .style(password_style)
            .block(Block::default().borders(Borders::ALL).title("Password"));
        f.render_widget(password_input, chunks[2]);

        // Draw status/error message
        if let Some(error) = &self.error_message {
            let error_msg = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .block(Block::default());
            f.render_widget(error_msg, chunks[3]);
        } else if self.processing {
            let status_msg = Paragraph::new("Processing...")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default());
            f.render_widget(status_msg, chunks[3]);
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