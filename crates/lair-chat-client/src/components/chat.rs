//! Chat screen component.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
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
    /// Online users (usernames).
    pub online_users: &'a [String],
    /// Offline users (usernames).
    pub offline_users: &'a [String],
}

/// Which panel is focused in the chat screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatFocus {
    /// Messages/input panel is focused.
    #[default]
    Messages,
    /// Users panel is focused.
    Users,
}

/// Chat screen state.
pub struct ChatScreen {
    /// Current input mode.
    pub mode: ChatMode,
    /// Message input buffer.
    pub input: String,
    /// Cursor position within input.
    pub cursor: usize,
    /// Scroll position.
    pub scroll: usize,
    /// Which panel is focused.
    pub focus: ChatFocus,
    /// Selection state for the users list.
    pub user_list_state: ListState,
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
            cursor: 0,
            scroll: 0,
            focus: ChatFocus::Messages,
            user_list_state: ListState::default(),
        }
    }

    /// Handle a key event.
    /// `user_count` is the number of users in the online users list (for navigation).
    pub fn handle_key(&mut self, key: KeyEvent, user_count: usize) -> Option<Action> {
        match self.mode {
            ChatMode::Normal => self.handle_normal_key(key, user_count),
            ChatMode::Insert => self.handle_insert_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent, user_count: usize) -> Option<Action> {
        // Handle Tab key to switch focus (available in both focuses)
        if key.code == KeyCode::Tab {
            self.focus = match self.focus {
                ChatFocus::Messages => {
                    // When switching to Users, select first user if none selected
                    if user_count > 0 && self.user_list_state.selected().is_none() {
                        self.user_list_state.select(Some(0));
                    }
                    ChatFocus::Users
                }
                ChatFocus::Users => ChatFocus::Messages,
            };
            return None;
        }

        match self.focus {
            ChatFocus::Messages => self.handle_messages_key(key),
            ChatFocus::Users => self.handle_users_key(key, user_count),
        }
    }

    fn handle_messages_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('i') | KeyCode::Enter => {
                self.mode = ChatMode::Insert;
                None
            }
            KeyCode::Char('r') => Some(Action::ShowRooms),
            KeyCode::Char('R') => Some(Action::Reconnect),
            KeyCode::Char('?') | KeyCode::F(1) => Some(Action::ShowHelp),
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
            KeyCode::Esc => Some(Action::ClearError),
            _ => None,
        }
    }

    fn handle_users_key(&mut self, key: KeyEvent, user_count: usize) -> Option<Action> {
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('r') => Some(Action::ShowRooms),
            KeyCode::Char('R') => Some(Action::Reconnect),
            KeyCode::Char('?') | KeyCode::F(1) => Some(Action::ShowHelp),
            KeyCode::Char('j') | KeyCode::Down => {
                if user_count > 0 {
                    let current = self.user_list_state.selected().unwrap_or(0);
                    let next = (current + 1).min(user_count.saturating_sub(1));
                    self.user_list_state.select(Some(next));
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if user_count > 0 {
                    let current = self.user_list_state.selected().unwrap_or(0);
                    let next = current.saturating_sub(1);
                    self.user_list_state.select(Some(next));
                }
                None
            }
            KeyCode::Enter => {
                // Start DM with selected user
                if let Some(idx) = self.user_list_state.selected() {
                    return Some(Action::StartDMByIndex(idx));
                }
                None
            }
            KeyCode::Esc => {
                // Switch back to messages focus
                self.focus = ChatFocus::Messages;
                None
            }
            _ => None,
        }
    }

    fn handle_insert_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Handle Ctrl modifiers first
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                // Ctrl+C - exit insert mode (like Esc)
                KeyCode::Char('c') => {
                    self.mode = ChatMode::Normal;
                    return None;
                }
                // Ctrl+U - clear line (delete from cursor to start)
                KeyCode::Char('u') => {
                    self.input.drain(..self.cursor);
                    self.cursor = 0;
                    return None;
                }
                // Ctrl+K - delete from cursor to end
                KeyCode::Char('k') => {
                    self.input.truncate(self.cursor);
                    return None;
                }
                // Ctrl+W - delete word before cursor
                KeyCode::Char('w') => {
                    self.delete_word_before_cursor();
                    return None;
                }
                // Ctrl+A - move cursor to start
                KeyCode::Char('a') => {
                    self.cursor = 0;
                    return None;
                }
                // Ctrl+E - move cursor to end
                KeyCode::Char('e') => {
                    self.cursor = self.input.len();
                    return None;
                }
                // Ctrl+B - move cursor back (like Left arrow)
                KeyCode::Char('b') => {
                    self.cursor = self.cursor.saturating_sub(1);
                    return None;
                }
                // Ctrl+F - move cursor forward (like Right arrow)
                KeyCode::Char('f') => {
                    self.cursor = (self.cursor + 1).min(self.input.len());
                    return None;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Esc => {
                self.mode = ChatMode::Normal;
                None
            }
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    let content = std::mem::take(&mut self.input);
                    self.cursor = 0;

                    // Check for commands
                    if content.starts_with('/') {
                        return self.handle_command(&content);
                    }

                    return Some(Action::SendMessage(content));
                }
                None
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                }
                None
            }
            KeyCode::Delete => {
                if self.cursor < self.input.len() {
                    self.input.remove(self.cursor);
                }
                None
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                None
            }
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.input.len());
                None
            }
            KeyCode::Home => {
                self.cursor = 0;
                None
            }
            KeyCode::End => {
                self.cursor = self.input.len();
                None
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor, c);
                self.cursor += 1;
                None
            }
            _ => None,
        }
    }

    /// Delete the word before the cursor (Ctrl+W behavior).
    fn delete_word_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }

        // Find start of word (skip trailing spaces, then find word start)
        let mut end = self.cursor;

        // Skip any spaces before cursor
        while end > 0 && self.input.chars().nth(end - 1) == Some(' ') {
            end -= 1;
        }

        // Find start of word
        let mut start = end;
        while start > 0 && self.input.chars().nth(start - 1) != Some(' ') {
            start -= 1;
        }

        // Delete from start to cursor
        if start < self.cursor {
            self.input.drain(start..self.cursor);
            self.cursor = start;
        }
    }

    fn handle_command(&mut self, input: &str) -> Option<Action> {
        let parts: Vec<&str> = input[1..].splitn(2, ' ').collect();
        let cmd = parts.first().unwrap_or(&"");
        let args = parts.get(1).unwrap_or(&"");

        match *cmd {
            "quit" | "q" => Some(Action::Quit),
            "rooms" | "r" => Some(Action::ShowRooms),
            "help" | "h" | "?" => Some(Action::ShowHelp),
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
    pub fn render(&mut self, frame: &mut Frame, area: Rect, ctx: &ChatRenderContext<'_>) {
        let ChatRenderContext {
            messages,
            room_name,
            dm_user,
            username,
            status,
            error,
            online_users,
            offline_users,
        } = ctx;

        // Main layout: chat area + users panel (always show users panel)
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(40),    // Chat area (flexible)
                Constraint::Length(22), // Users panel (fixed width)
            ])
            .split(area);

        let chat_area = main_chunks[0];
        let users_area = main_chunks[1];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Messages
                Constraint::Length(3), // Input
                Constraint::Length(1), // Status
            ])
            .split(chat_area);

        // Determine border colors based on focus
        let (messages_border_color, users_border_color) = match self.focus {
            ChatFocus::Messages => (Color::Cyan, Color::DarkGray),
            ChatFocus::Users => (Color::DarkGray, Color::Yellow),
        };

        // Messages area
        let (title, title_color) = if let Some(dm) = dm_user {
            (format!(" DM: {} ", dm), Color::Magenta)
        } else if let Some(r) = room_name {
            (format!(" #{} ", r), Color::Cyan)
        } else {
            (" No Room - Press 'r' for rooms ".to_string(), Color::Yellow)
        };
        let messages_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(messages_border_color))
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
        let has_target = room_name.is_some() || dm_user.is_some();
        let input_title = match (self.mode, has_target) {
            (ChatMode::Normal, true) => " Press 'i' to type ",
            (ChatMode::Normal, false) => " Press 'r' to join a room ",
            (ChatMode::Insert, true) => " Enter:send Esc:cancel C-w:del-word C-u:clear ",
            (ChatMode::Insert, false) => " No room! Esc then 'r' ",
        };
        let input_style = match self.mode {
            ChatMode::Normal => Style::default().fg(Color::DarkGray),
            ChatMode::Insert => Style::default().fg(Color::Yellow),
        };
        let input_block = Block::default()
            .title(input_title)
            .borders(Borders::ALL)
            .style(input_style);

        // Render input with cursor
        let input_text = if self.mode == ChatMode::Insert {
            // Show cursor position
            let (before, after) = self.input.split_at(self.cursor);
            format!("{}│{}", before, after)
        } else {
            self.input.clone()
        };
        let input_para = Paragraph::new(input_text).block(input_block);
        frame.render_widget(input_para, chunks[1]);

        // Status bar with context-aware hints
        let online_count = online_users.len();
        let total_count = online_users.len() + offline_users.len();
        let mut status_spans = vec![
            Span::styled(
                format!(" {} ", username.unwrap_or("Not logged in")),
                Style::default().fg(Color::Green),
            ),
            Span::raw(" | "),
            Span::styled(
                format!("{} ", status.unwrap_or("Disconnected")),
                Style::default().fg(Color::Cyan),
            ),
        ];

        status_spans.push(Span::styled(
            format!(" | {}/{} online ", online_count, total_count),
            Style::default().fg(Color::Magenta),
        ));

        if let Some(err) = error {
            status_spans.push(Span::styled(
                format!(" | {} ", err),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        }

        // Context-aware keybind hints
        let hints = match (self.mode, self.focus) {
            (ChatMode::Insert, _) => " Esc:normal Enter:send ",
            (ChatMode::Normal, ChatFocus::Messages) => " Tab:users i:type r:rooms ?:help q:quit ",
            (ChatMode::Normal, ChatFocus::Users) => " Tab:chat j/k:nav Enter:DM Esc:back ",
        };
        status_spans.push(Span::styled(hints, Style::default().fg(Color::DarkGray)));

        let status_line = Line::from(status_spans);
        let status_para = Paragraph::new(status_line);
        frame.render_widget(status_para, chunks[2]);

        // Users panel (always visible)
        let users_title = if self.focus == ChatFocus::Users {
            format!(" Users ({}) [Tab:back] ", total_count)
        } else {
            format!(" Users ({}) [Tab] ", total_count)
        };
        let users_block = Block::default()
            .title(users_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(users_border_color))
            .style(Style::default().fg(Color::Magenta));

        if online_users.is_empty() && offline_users.is_empty() {
            // Show "no users" message
            let empty_msg = Paragraph::new("  No other users")
                .style(Style::default().fg(Color::DarkGray))
                .block(users_block);
            frame.render_widget(empty_msg, users_area);
        } else {
            // Build user list with sections
            let mut user_items: Vec<ListItem> = Vec::new();
            let mut list_idx = 0usize;

            // Online section header
            if !online_users.is_empty() {
                user_items.push(
                    ListItem::new(format!("─ Online ({}) ─", online_users.len())).style(
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                );
            }

            // Online users
            for name in online_users.iter() {
                let is_selected = self.user_list_state.selected() == Some(list_idx)
                    && self.focus == ChatFocus::Users;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                };
                let prefix = if is_selected { "> " } else { "  " };
                user_items.push(ListItem::new(format!("{}{}", prefix, name)).style(style));
                list_idx += 1;
            }

            // Offline section header
            if !offline_users.is_empty() {
                user_items.push(
                    ListItem::new(format!("─ Offline ({}) ─", offline_users.len())).style(
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    ),
                );
            }

            // Offline users
            for name in offline_users.iter() {
                let is_selected = self.user_list_state.selected() == Some(list_idx)
                    && self.focus == ChatFocus::Users;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let prefix = if is_selected { "> " } else { "  " };
                user_items.push(ListItem::new(format!("{}{}", prefix, name)).style(style));
                list_idx += 1;
            }

            let users_list = List::new(user_items).block(users_block).highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_stateful_widget(users_list, users_area, &mut self.user_list_state);
        }
    }
}
