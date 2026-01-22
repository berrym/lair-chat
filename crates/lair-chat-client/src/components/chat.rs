//! Chat screen component.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{Action, ChatMessage};

/// Context for rendering the chat screen.
pub struct ChatRenderContext<'a> {
    /// Messages to display.
    pub messages: &'a [ChatMessage],
    /// Current room name (None if in DM mode).
    pub room_name: Option<&'a str>,
    /// Current DM partner username (None if in room mode).
    pub dm_user: Option<&'a str>,
    /// Current username.
    pub username: Option<&'a str>,
    /// Connection status.
    pub status: Option<&'a str>,
    /// Error message to display.
    pub error: Option<&'a str>,
    /// Online users in the current room.
    pub online_users: &'a [String],
}

/// Chat screen state.
pub struct ChatScreen {
    /// Current input mode.
    pub mode: ChatMode,
    /// Message input.
    pub input: String,
    /// Scroll position.
    pub scroll: usize,
}

/// Input mode for chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatMode {
    Normal,
    Insert,
}

impl Default for ChatScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatScreen {
    /// Create a new chat screen.
    pub fn new() -> Self {
        Self {
            mode: ChatMode::Normal,
            input: String::new(),
            scroll: 0,
        }
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match self.mode {
            ChatMode::Normal => self.handle_normal_key(key),
            ChatMode::Insert => self.handle_insert_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('i') => {
                self.mode = ChatMode::Insert;
                None
            }
            KeyCode::Char('r') => Some(Action::ShowRooms),
            KeyCode::Char('R') => Some(Action::Reconnect),
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                None
            }
            KeyCode::Char('G') => {
                self.scroll = usize::MAX; // Will be clamped
                None
            }
            KeyCode::Char('g') => {
                self.scroll = 0;
                None
            }
            _ => None,
        }
    }

    fn handle_insert_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.mode = ChatMode::Normal;
                None
            }
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    let content = std::mem::take(&mut self.input);

                    // Check for commands
                    if content.starts_with('/') {
                        return self.handle_command(&content);
                    }

                    return Some(Action::SendMessage(content));
                }
                None
            }
            KeyCode::Backspace => {
                self.input.pop();
                None
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                None
            }
            _ => None,
        }
    }

    fn handle_command(&mut self, input: &str) -> Option<Action> {
        let parts: Vec<&str> = input[1..].splitn(2, ' ').collect();
        let cmd = parts.first().unwrap_or(&"");
        let args = parts.get(1).unwrap_or(&"");

        match *cmd {
            "quit" | "q" => Some(Action::Quit),
            "rooms" | "r" => Some(Action::ShowRooms),
            "create" => {
                if !args.is_empty() {
                    Some(Action::CreateRoom(args.to_string()))
                } else {
                    None
                }
            }
            "dm" | "msg" | "whisper" | "w" => {
                if !args.is_empty() {
                    Some(Action::StartDM(args.to_string()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Render the chat screen.
    pub fn render(&self, frame: &mut Frame, area: Rect, ctx: &ChatRenderContext<'_>) {
        let ChatRenderContext {
            messages,
            room_name,
            dm_user,
            username,
            status,
            error,
            online_users,
        } = ctx;

        // Main layout: chat area + users panel (if we have users)
        let show_users_panel = !online_users.is_empty();
        let main_chunks = if show_users_panel {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(40),    // Chat area (flexible)
                    Constraint::Length(20), // Users panel (fixed width)
                ])
                .split(area)
        } else {
            // No users panel, use full width
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100)])
                .split(area)
        };

        let chat_area = main_chunks[0];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Messages
                Constraint::Length(3), // Input
                Constraint::Length(1), // Status
            ])
            .split(chat_area);

        // Messages area
        let (title, title_color) = if let Some(dm) = dm_user {
            (format!(" DM: {} ", dm), Color::Magenta)
        } else if let Some(r) = room_name {
            (format!(" {} ", r), Color::Cyan)
        } else {
            (" Chat ".to_string(), Color::Cyan)
        };
        let messages_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().fg(title_color));

        let inner_height = chunks[0].height.saturating_sub(2) as usize;
        let total_messages = messages.len();

        // Clamp scroll
        let max_scroll = total_messages.saturating_sub(inner_height);
        let scroll = self.scroll.min(max_scroll);

        let visible_messages: Vec<ListItem> = messages
            .iter()
            .skip(scroll)
            .take(inner_height)
            .map(|msg| {
                let style = if msg.is_system {
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC)
                } else {
                    Style::default()
                };

                let time = msg.timestamp.format("%H:%M");
                let line = if msg.is_system {
                    Line::from(vec![
                        Span::styled(format!("[{}] ", time), Style::default().fg(Color::DarkGray)),
                        Span::styled(&msg.content, style),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(format!("[{}] ", time), Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            format!("{}: ", msg.author),
                            Style::default().fg(Color::Green),
                        ),
                        Span::raw(&msg.content),
                    ])
                };

                ListItem::new(line)
            })
            .collect();

        let messages_list = List::new(visible_messages).block(messages_block);
        frame.render_widget(messages_list, chunks[0]);

        // Input area
        let input_title = match self.mode {
            ChatMode::Normal => " Press 'i' to type ",
            ChatMode::Insert => " Type your message (Esc to exit) ",
        };
        let input_style = match self.mode {
            ChatMode::Normal => Style::default().fg(Color::DarkGray),
            ChatMode::Insert => Style::default().fg(Color::Yellow),
        };
        let input_block = Block::default()
            .title(input_title)
            .borders(Borders::ALL)
            .style(input_style);

        let input_text = if self.mode == ChatMode::Insert {
            format!("{}|", self.input)
        } else {
            self.input.clone()
        };
        let input_para = Paragraph::new(input_text).block(input_block);
        frame.render_widget(input_para, chunks[1]);

        // Status bar
        let user_count = online_users.len();
        let status_spans = vec![
            Span::styled(
                format!(" {} ", username.unwrap_or("Not logged in")),
                Style::default().fg(Color::Green),
            ),
            Span::raw(" | "),
            Span::styled(
                format!("{} ", status.unwrap_or("Disconnected")),
                Style::default().fg(Color::Cyan),
            ),
            if user_count > 0 {
                Span::styled(
                    format!(" | {} online ", user_count),
                    Style::default().fg(Color::Magenta),
                )
            } else {
                Span::raw("")
            },
            if let Some(err) = error {
                Span::styled(
                    format!(" | Error: {}", err),
                    Style::default().fg(Color::Red),
                )
            } else {
                Span::raw("")
            },
            Span::styled(
                " | q:quit r:rooms i:input R:reconnect ",
                Style::default().fg(Color::DarkGray),
            ),
        ];
        let status_line = Line::from(status_spans);
        let status_para = Paragraph::new(status_line);
        frame.render_widget(status_para, chunks[2]);

        // Users panel (if showing)
        if show_users_panel {
            let users_area = main_chunks[1];
            let users_block = Block::default()
                .title(format!(" Users ({}) ", online_users.len()))
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Magenta));

            let user_items: Vec<ListItem> = online_users
                .iter()
                .map(|name| {
                    let is_self = username.map(|u| u == name).unwrap_or(false);
                    let style = if is_self {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let prefix = if is_self { "* " } else { "  " };
                    ListItem::new(format!("{}{}", prefix, name)).style(style)
                })
                .collect();

            let users_list = List::new(user_items).block(users_block);
            frame.render_widget(users_list, users_area);
        }
    }
}
