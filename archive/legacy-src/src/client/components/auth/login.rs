use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use tui_input::Input;

use color_eyre::Result;

use crate::action::Action;
use crate::auth::{AuthError, AuthState, Credentials};
use crate::components::Component;
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
    password_confirm: Input,
    server: Input,
    port: Input,
    error_message: Option<String>,
    pub mode: LoginMode,
    focused_field: usize,
    processing: bool,
    show_help: bool,
    help_scroll: usize,
    show_error: bool,
    error_scroll: usize,
}

impl LoginScreen {
    pub fn new() -> Self {
        Self {
            username: Input::default(),
            password: Input::default(),
            password_confirm: Input::default(),
            server: Input::new("127.0.0.1".into()),
            port: Input::new("8080".into()),
            error_message: None,
            mode: LoginMode::Login,
            focused_field: 0,
            processing: false,
            show_help: false,
            help_scroll: 0,
            show_error: false,
            error_scroll: 0,
        }
    }

    fn get_field_indexes(&self) -> (usize, usize, usize, usize, Option<usize>) {
        // Returns (username, password, password_confirm/server, server/port, port/none)
        match self.mode {
            LoginMode::Login => (0, 1, 2, 3, None),
            LoginMode::Register => (0, 1, 2, 3, Some(4)),
        }
    }

    fn get_max_field(&self) -> usize {
        match self.mode {
            LoginMode::Login => 3,
            LoginMode::Register => 4,
        }
    }

    fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            LoginMode::Login => LoginMode::Register,
            LoginMode::Register => LoginMode::Login,
        };
        self.error_message = None;
        // Clear password confirmation when switching modes
        self.password_confirm = Input::default();
        // Reset focus to username when switching modes
        self.focused_field = 0;
    }

    fn submit(&mut self) -> Option<Action> {
        if self.processing {
            return None;
        }

        // Clear any previous error
        self.error_message = None;

        // Validate username and password
        if self.username.value().trim().is_empty() {
            self.error_message = Some("Username cannot be empty".to_string());

            return None;
        }

        if self.password.value().trim().is_empty() {
            self.error_message = Some("Password cannot be empty".to_string());

            return None;
        }

        // For registration, validate password confirmation
        if matches!(self.mode, LoginMode::Register) {
            if self.password_confirm.value().trim().is_empty() {
                self.error_message = Some("Password confirmation cannot be empty".to_string());
                return None;
            }

            if self.password.value() != self.password_confirm.value() {
                self.error_message = Some("Passwords do not match".to_string());
                return None;
            }
        }

        // Validate server and port
        if self.server.value().trim().is_empty() {
            self.error_message = Some("Server address cannot be empty".to_string());

            return None;
        }

        if self.port.value().trim().is_empty() {
            self.error_message = Some("Port cannot be empty".to_string());

            return None;
        }

        // Validate port is a number in valid range
        match self.port.value().trim().parse::<u16>() {
            Ok(port) => {
                if port == 0 {
                    self.error_message = Some("Port must be greater than 0".to_string());

                    return None;
                }
            }
            Err(_) => {
                self.error_message = Some("Port must be a valid number (1-65535)".to_string());

                return None;
            }
        }

        let credentials = Credentials {
            username: self.username.value().trim().to_string(),
            password: self.password.value().trim().to_string(),
        };

        let server_address = format!(
            "{}:{}",
            self.server.value().trim(),
            self.port.value().trim()
        );

        // Validate server address format
        if server_address.parse::<std::net::SocketAddr>().is_err() {
            self.error_message = Some("Invalid server address format".to_string());

            return None;
        }

        self.processing = true;
        self.error_message = None;

        let action = match self.mode {
            LoginMode::Login => Action::Login(credentials),
            LoginMode::Register => Action::Register(credentials),
        };

        Some(action)
    }

    pub fn handle_error(&mut self, error: AuthError) {
        self.processing = false;
        self.error_message = Some(error.to_string());
        self.show_error = true;
        self.error_scroll = 0;
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
            AuthState::Unauthenticated => {
                // Reset login screen to clean state
                self.processing = false;
                self.error_message = None;
                self.show_help = false;
                self.show_error = false;
                self.help_scroll = 0;
                self.error_scroll = 0;
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
                if !self.show_help && !self.show_error {
                    let max_field = self.get_max_field();
                    self.focused_field = (self.focused_field + 1) % (max_field + 1);
                }
                None
            }
            crossterm::event::KeyCode::BackTab => {
                if !self.show_help && !self.show_error {
                    let max_field = self.get_max_field();
                    self.focused_field = if self.focused_field == 0 {
                        max_field
                    } else {
                        self.focused_field - 1
                    };
                }
                None
            }
            crossterm::event::KeyCode::Enter => {
                if self.show_error {
                    self.show_error = false;
                    self.error_scroll = 0;
                    None
                } else if self.show_help {
                    // If help is open, close it instead of submitting
                    self.show_help = false;
                    self.help_scroll = 0;
                    None
                } else {
                    self.submit()
                }
            }
            crossterm::event::KeyCode::Char('t')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.toggle_mode();
                None
            }
            crossterm::event::KeyCode::Char('?') => {
                if !self.show_error {
                    self.show_help = !self.show_help;
                    self.help_scroll = 0; // Reset scroll when opening help
                }
                None
            }
            crossterm::event::KeyCode::Esc => {
                if self.show_error {
                    self.show_error = false;
                    self.error_scroll = 0;
                    None
                } else if self.show_help {
                    self.show_help = false;
                    self.help_scroll = 0;
                    None
                } else {
                    None
                }
            }
            crossterm::event::KeyCode::Up if self.show_help || self.show_error => {
                if self.show_help && self.help_scroll > 0 {
                    self.help_scroll -= 1;
                } else if self.show_error && self.error_scroll > 0 {
                    self.error_scroll -= 1;
                }
                None
            }
            crossterm::event::KeyCode::Down if self.show_help || self.show_error => {
                if self.show_help {
                    self.help_scroll += 1;
                } else if self.show_error {
                    self.error_scroll += 1;
                }
                None
            }
            crossterm::event::KeyCode::PageUp if self.show_help || self.show_error => {
                if self.show_help {
                    self.help_scroll = self.help_scroll.saturating_sub(5);
                } else if self.show_error {
                    self.error_scroll = self.error_scroll.saturating_sub(5);
                }
                None
            }
            crossterm::event::KeyCode::PageDown if self.show_help || self.show_error => {
                if self.show_help {
                    self.help_scroll += 5;
                } else if self.show_error {
                    self.error_scroll += 5;
                }
                None
            }
            crossterm::event::KeyCode::Char(c) => {
                if !self.show_help && !self.show_error {
                    match self.focused_field {
                        0 => {
                            self.username = self.username.clone().with_value(format!(
                                "{}{}",
                                self.username.value(),
                                c
                            ));
                        }
                        1 => {
                            self.password = self.password.clone().with_value(format!(
                                "{}{}",
                                self.password.value(),
                                c
                            ));
                        }
                        2 => {
                            if matches!(self.mode, LoginMode::Register) {
                                self.password_confirm = self
                                    .password_confirm
                                    .clone()
                                    .with_value(format!("{}{}", self.password_confirm.value(), c));
                            } else {
                                self.server = self.server.clone().with_value(format!(
                                    "{}{}",
                                    self.server.value(),
                                    c
                                ));
                            }
                        }
                        3 => {
                            if matches!(self.mode, LoginMode::Register) {
                                self.server = self.server.clone().with_value(format!(
                                    "{}{}",
                                    self.server.value(),
                                    c
                                ));
                            } else {
                                self.port = self.port.clone().with_value(format!(
                                    "{}{}",
                                    self.port.value(),
                                    c
                                ));
                            }
                        }
                        4 => {
                            if matches!(self.mode, LoginMode::Register) {
                                self.port = self.port.clone().with_value(format!(
                                    "{}{}",
                                    self.port.value(),
                                    c
                                ));
                            }
                        }
                        _ => {}
                    }
                }
                None
            }
            crossterm::event::KeyCode::Backspace => {
                if !self.show_help && !self.show_error {
                    match self.focused_field {
                        0 => {
                            let value = self.username.value();
                            if !value.is_empty() {
                                self.username = self
                                    .username
                                    .clone()
                                    .with_value(value[..value.len() - 1].to_string());
                            }
                        }
                        1 => {
                            let value = self.password.value();
                            if !value.is_empty() {
                                self.password = self
                                    .password
                                    .clone()
                                    .with_value(value[..value.len() - 1].to_string());
                            }
                        }
                        2 => {
                            if matches!(self.mode, LoginMode::Register) {
                                let value = self.password_confirm.value();
                                if !value.is_empty() {
                                    self.password_confirm = self
                                        .password_confirm
                                        .clone()
                                        .with_value(value[..value.len() - 1].to_string());
                                }
                            } else {
                                let value = self.server.value();
                                if !value.is_empty() {
                                    self.server = self
                                        .server
                                        .clone()
                                        .with_value(value[..value.len() - 1].to_string());
                                }
                            }
                        }
                        3 => {
                            if matches!(self.mode, LoginMode::Register) {
                                let value = self.server.value();
                                if !value.is_empty() {
                                    self.server = self
                                        .server
                                        .clone()
                                        .with_value(value[..value.len() - 1].to_string());
                                }
                            } else {
                                let value = self.port.value();
                                if !value.is_empty() {
                                    self.port = self
                                        .port
                                        .clone()
                                        .with_value(value[..value.len() - 1].to_string());
                                }
                            }
                        }
                        4 => {
                            if matches!(self.mode, LoginMode::Register) {
                                let value = self.port.value();
                                if !value.is_empty() {
                                    self.port = self
                                        .port
                                        .clone()
                                        .with_value(value[..value.len() - 1].to_string());
                                }
                            }
                        }

                        _ => {}
                    }
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
                Constraint::Percentage(5), // Top padding
                Constraint::Length(35),    // Login form (much taller)
                Constraint::Percentage(5), // Bottom padding
            ])
            .split(area);

        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(15), // Left padding (reduced)
                Constraint::Percentage(70), // Login form (increased width)
                Constraint::Percentage(15), // Right padding (reduced)
            ])
            .split(vertical_chunks[1]);

        let form_area = horizontal_chunks[1];

        // Draw a clear background
        f.render_widget(Clear, form_area);

        // Create the main box with title
        let main_title = match self.mode {
            LoginMode::Login => "Lair Chat - Login",
            LoginMode::Register => "Lair Chat - Register",
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
            .constraints(if matches!(self.mode, LoginMode::Register) {
                vec![
                    Constraint::Length(4), // Mode selection and instructions
                    Constraint::Length(1), // Spacer
                    Constraint::Length(3), // Username input
                    Constraint::Length(3), // Password input
                    Constraint::Length(3), // Password confirm
                    Constraint::Length(3), // Server input
                    Constraint::Length(3), // Port input
                    Constraint::Length(1), // Spacer
                    Constraint::Length(2), // Help label (taller for errors)
                ]
            } else {
                vec![
                    Constraint::Length(4), // Mode selection and instructions
                    Constraint::Length(1), // Spacer
                    Constraint::Length(3), // Username input
                    Constraint::Length(3), // Password input
                    Constraint::Length(3), // Server input
                    Constraint::Length(3), // Port input
                    Constraint::Length(1), // Spacer
                    Constraint::Length(2), // Help label (taller for errors)
                ]
            })
            .split(form_area);

        // Draw mode selection and instructions
        let mode_instruction = match self.mode {
            LoginMode::Login => "Login Mode - Press Ctrl+T to switch to Register Mode",
            LoginMode::Register => "Register Mode - Press Ctrl+T to switch to Login Mode",
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
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                mode_instruction,
                Style::default().fg(Color::Yellow),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Authentication Mode"),
        );
        f.render_widget(mode_display, form_chunks[0]);

        // Draw username field with better styling
        let username_title = if self.focused_field == 0 {
            "Username (FOCUSED - Type here)"
        } else {
            "Username"
        };

        let username_style = if self.focused_field == 0 {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
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

        let username_display = if self.focused_field == 0 {
            format!("{}|", self.username.value())
        } else {
            self.username.value().to_string()
        };

        let username_input = Paragraph::new(username_display)
            .style(username_style)
            .block(username_block)
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(username_input, form_chunks[2]);

        // Draw password field with better styling
        let password_title = if self.focused_field == 1 {
            "Password (FOCUSED - Type here)"
        } else {
            "Password"
        };

        let password_style = if self.focused_field == 1 {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
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

        let masked_password = if self.focused_field == 1 {
            format!("{}|", "•".repeat(self.password.value().len()))
        } else {
            "•".repeat(self.password.value().len())
        };
        let password_input = Paragraph::new(masked_password)
            .style(password_style)
            .block(password_block)
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(password_input, form_chunks[3]);

        // Draw password confirmation field (only in register mode)
        let mut current_chunk = 4;
        if matches!(self.mode, LoginMode::Register) {
            let confirm_title = if self.focused_field == 2 {
                "Confirm Password (FOCUSED - Type here)"
            } else {
                "Confirm Password"
            };

            let confirm_style = if self.focused_field == 2 {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let confirm_block = Block::default()
                .borders(Borders::ALL)
                .title(confirm_title)
                .border_style(if self.focused_field == 2 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Gray)
                });

            let masked_confirm = if self.focused_field == 2 {
                format!("{}|", "•".repeat(self.password_confirm.value().len()))
            } else {
                "•".repeat(self.password_confirm.value().len())
            };
            let confirm_input = Paragraph::new(masked_confirm)
                .style(confirm_style)
                .block(confirm_block)
                .wrap(ratatui::widgets::Wrap { trim: false });
            f.render_widget(confirm_input, form_chunks[4]);
            current_chunk = 5;
        }

        // Draw server field with better styling
        let (_, _, _, server_idx, port_idx) = self.get_field_indexes();
        let server_focused = self.focused_field == server_idx;
        let server_title = if server_focused {
            "Server (FOCUSED - Type here)"
        } else {
            "Server"
        };

        let server_style = if server_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let server_block = Block::default()
            .borders(Borders::ALL)
            .title(server_title)
            .border_style(if server_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            });

        let server_display = if server_focused {
            format!("{}|", self.server.value())
        } else {
            self.server.value().to_string()
        };

        let server_input = Paragraph::new(server_display)
            .style(server_style)
            .block(server_block)
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(server_input, form_chunks[current_chunk]);

        // Draw port field with better styling
        let port_idx = port_idx.unwrap_or(server_idx + 1);
        let port_focused = self.focused_field == port_idx;
        let port_title = if port_focused {
            "Port (FOCUSED - Type here)"
        } else {
            "Port"
        };

        let port_style = if port_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let port_block = Block::default()
            .borders(Borders::ALL)
            .title(port_title)
            .border_style(if port_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            });

        let port_display = if port_focused {
            format!("{}|", self.port.value())
        } else {
            self.port.value().to_string()
        };

        let port_input = Paragraph::new(port_display)
            .style(port_style)
            .block(port_block)
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(port_input, form_chunks[current_chunk + 1]);

        // Draw simple help label
        let help_text = if self.processing {
            Paragraph::new("Processing... | Press ? for help")
                .style(Style::default().fg(Color::Yellow))
        } else if self.error_message.is_some() {
            Paragraph::new("Error occurred | Press Esc to view details | Press ? for help")
                .style(Style::default().fg(Color::Red))
        } else {
            Paragraph::new("Press ? for help").style(Style::default().fg(Color::Blue))
        };

        f.render_widget(help_text, form_chunks[6]);

        // Draw help popup if visible
        if self.show_help {
            self.draw_help_popup(f, area)?;
        }

        // Draw error popup if visible
        if self.show_error {
            self.draw_error_popup(f, area)?;
        }

        Ok(())
    }
}

impl LoginScreen {
    fn draw_help_popup(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        // Create centered popup
        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ])
            .split(area)[1];

        let popup_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ])
            .split(popup_area)[1];

        // Clear background
        f.render_widget(Clear, popup_area);

        let all_help_text = vec![
            Line::from(vec![Span::styled(
                "Lair Chat - Login Help",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Getting Started:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("1. Enter your username and password"),
            Line::from("2. For registration: confirm your password"),
            Line::from("3. Specify server address (e.g., 127.0.0.1)"),
            Line::from("4. Enter port number (e.g., 8080)"),
            Line::from("5. Press Enter to connect and authenticate"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Navigation:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("Tab/Shift+Tab - Move between fields"),
            Line::from("Enter - Submit login/registration"),
            Line::from("Ctrl+T - Toggle Login/Register mode"),
            Line::from("Backspace - Delete characters"),
            Line::from("Type - Enter text in focused field"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Modes:",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("Login - Sign in with existing account"),
            Line::from("  • Username and password only"),
            Line::from("Register - Create new account"),
            Line::from("  • Username, password, and password confirmation"),
            Line::from("  • Passwords must match to proceed"),
            Line::from("(Use Ctrl+T to switch between modes)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Server Setup:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]),
            Line::from("IMPORTANT: Start the server first in another terminal:"),
            Line::from("  cargo run --bin lair-chat-server"),
            Line::from(""),
            Line::from("The server will listen on 127.0.0.1:8080 by default"),
            Line::from("Make sure this matches your login form settings"),
            Line::from(""),
            Line::from("Common connection issues:"),
            Line::from("- Server not running (start it first!)"),
            Line::from("- Wrong server address or port"),
            Line::from("- Firewall blocking connection"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Help Navigation:",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("Up/Down - Scroll help text"),
            Line::from("PageUp/PageDown - Scroll faster"),
            Line::from("? or Esc - Close help popup"),
            Line::from("Ctrl+C - Quit application"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Troubleshooting:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]),
            Line::from("- Make sure server is running first"),
            Line::from("- Check server address and port are correct"),
            Line::from("- Username and password cannot be empty"),
            Line::from("- Port must be a number between 1-65535"),
            Line::from(""),
            Line::from("Press Esc or ? to close this help"),
        ];

        // Calculate available height for content
        let content_height = popup_area.height.saturating_sub(2) as usize; // -2 for borders
        let max_scroll = all_help_text.len().saturating_sub(content_height);

        // Limit scroll to prevent scrolling past content
        if self.help_scroll > max_scroll {
            self.help_scroll = max_scroll;
        }

        // Get visible portion of help text
        let visible_text: Vec<Line> = all_help_text
            .iter()
            .skip(self.help_scroll)
            .take(content_height)
            .cloned()
            .collect();

        let title = if max_scroll > 0 {
            format!(
                "Help - Press Esc to close ({}/{})",
                self.help_scroll + 1,
                all_help_text.len()
            )
        } else {
            "Help - Press Esc to close".to_string()
        };

        let help_popup = Paragraph::new(visible_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(ratatui::widgets::Wrap { trim: false });

        f.render_widget(help_popup, popup_area);

        // Draw scrollbar if needed
        if max_scroll > 0 {
            let scrollbar_area = Rect {
                x: popup_area.x + popup_area.width - 1,
                y: popup_area.y + 1,
                width: 1,
                height: popup_area.height - 2,
            };

            let scrollbar_pos = if max_scroll > 0 {
                (self.help_scroll * (scrollbar_area.height as usize - 1)) / max_scroll
            } else {
                0
            };

            for y in 0..scrollbar_area.height {
                let style = if y as usize == scrollbar_pos {
                    Style::default().bg(Color::Yellow)
                } else {
                    Style::default().bg(Color::DarkGray)
                };
                f.render_widget(
                    Paragraph::new(" ").style(style),
                    Rect {
                        x: scrollbar_area.x,
                        y: scrollbar_area.y + y,
                        width: 1,
                        height: 1,
                    },
                );
            }
        }

        Ok(())
    }

    fn draw_error_popup(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        // Create centered popup for error display
        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(area)[1];

        let popup_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ])
            .split(popup_area)[1];

        // Clear background
        f.render_widget(Clear, popup_area);

        if let Some(error) = &self.error_message {
            let error_lines = vec![
                Line::from(vec![Span::styled(
                    "Connection Error",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "Error: ",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(error.as_str(), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Troubleshooting:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from("1. Make sure the server is running:"),
                Line::from("   cargo run --bin lair-chat-server"),
                Line::from(""),
                Line::from("2. Check server address and port are correct"),
                Line::from("3. Verify no firewall is blocking the connection"),
                Line::from("4. Try restarting both client and server"),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Navigation:",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from("Up/Down - Scroll | Esc/Enter - Close"),
            ];

            // Calculate scrolling
            let content_height = popup_area.height.saturating_sub(2) as usize;
            let max_scroll = error_lines.len().saturating_sub(content_height);

            if self.error_scroll > max_scroll {
                self.error_scroll = max_scroll;
            }

            let visible_lines: Vec<Line> = error_lines
                .iter()
                .skip(self.error_scroll)
                .take(content_height)
                .cloned()
                .collect();

            let title = if max_scroll > 0 {
                format!(
                    "Error Details - Press Esc to close ({}/{})",
                    self.error_scroll + 1,
                    error_lines.len()
                )
            } else {
                "Error Details - Press Esc to close".to_string()
            };

            let error_popup = Paragraph::new(visible_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(Style::default().fg(Color::Red)),
                )
                .wrap(ratatui::widgets::Wrap { trim: false });

            f.render_widget(error_popup, popup_area);
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
