//! Chat screen component.

use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
    Frame,
};

use crate::app::{Action, ChatMessage};

/// Message style for bubble rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageStyle {
    /// Message sent by current user (right-aligned, blue).
    Sent,
    /// Message received from others (left-aligned, gray).
    Received,
    /// System message (centered, italic).
    System,
    /// System message (left-aligned, for help text).
    SystemLeft,
    /// DM sent by current user (right-aligned, purple).
    DmSent,
    /// DM received from others (left-aligned, green).
    DmReceived,
}

/// Wrap text to fit within a specified width.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || text.is_empty() {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        // If adding this word would exceed max width, start a new line
        if !current_line.is_empty() && current_line.len() + 1 + word.len() > max_width {
            lines.push(current_line.clone());
            current_line.clear();
        }

        // If the word itself is longer than max_width, break it
        if word.len() > max_width {
            if !current_line.is_empty() {
                lines.push(current_line.clone());
                current_line.clear();
            }
            // Break long word into chunks
            let mut chars: Vec<char> = word.chars().collect();
            while !chars.is_empty() {
                let chunk_size = max_width.min(chars.len());
                let chunk: String = chars.drain(..chunk_size).collect();
                lines.push(chunk);
            }
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(text.to_string());
    }

    lines
}

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
    /// Unread DM counts per username.
    pub unread_dms: &'a std::collections::HashMap<String, u32>,
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
    /// Message input buffer (supports multiple lines).
    pub input: String,
    /// Cursor position within input (byte offset).
    pub cursor: usize,
    /// Scroll position for messages.
    pub scroll: usize,
    /// Scroll position for input (when input has many lines).
    pub input_scroll: usize,
    /// Which panel is focused.
    pub focus: ChatFocus,
    /// Selection state for the users list.
    pub user_list_state: ListState,
    /// System clipboard (lazy-initialized).
    clipboard: Option<Clipboard>,
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
            input_scroll: 0,
            focus: ChatFocus::Messages,
            user_list_state: ListState::default(),
            clipboard: None,
        }
    }

    /// Get or initialize the clipboard.
    fn get_clipboard(&mut self) -> Option<&mut Clipboard> {
        if self.clipboard.is_none() {
            self.clipboard = Clipboard::new().ok();
        }
        self.clipboard.as_mut()
    }

    /// Paste text from clipboard at current cursor position.
    fn paste_from_clipboard(&mut self) {
        if let Some(clipboard) = self.get_clipboard() {
            if let Ok(text) = clipboard.get_text() {
                // Normalize line endings to \n but allow multiline
                let text: String = text.replace("\r\n", "\n").replace('\r', "\n");
                if !text.is_empty() {
                    self.input.insert_str(self.cursor, &text);
                    self.cursor += text.len();
                }
            }
        }
    }

    /// Get the line number and column position of the cursor.
    /// Returns (line_index, column_in_line).
    fn cursor_line_col(&self) -> (usize, usize) {
        let before_cursor = &self.input[..self.cursor];
        let line = before_cursor.matches('\n').count();
        let last_newline = before_cursor.rfind('\n');
        let col = match last_newline {
            Some(pos) => self.cursor - pos - 1,
            None => self.cursor,
        };
        (line, col)
    }

    /// Get the byte offset of the start of a given line.
    fn line_start_offset(&self, line_idx: usize) -> usize {
        if line_idx == 0 {
            return 0;
        }
        let mut count = 0;
        for (i, c) in self.input.char_indices() {
            if c == '\n' {
                count += 1;
                if count == line_idx {
                    return i + 1;
                }
            }
        }
        self.input.len()
    }

    /// Get the byte offset of the end of a given line (before newline or end of string).
    fn line_end_offset(&self, line_idx: usize) -> usize {
        let start = self.line_start_offset(line_idx);
        if let Some(rel_pos) = self.input[start..].find('\n') {
            start + rel_pos
        } else {
            self.input.len()
        }
    }

    /// Get the total number of lines in the input.
    fn input_line_count(&self) -> usize {
        self.input.matches('\n').count() + 1
    }

    /// Move cursor up one line, trying to preserve column position.
    fn cursor_up(&mut self) {
        let (line, col) = self.cursor_line_col();
        if line > 0 {
            let new_line_start = self.line_start_offset(line - 1);
            let new_line_end = self.line_end_offset(line - 1);
            let new_line_len = new_line_end - new_line_start;
            self.cursor = new_line_start + col.min(new_line_len);
        }
    }

    /// Move cursor down one line, trying to preserve column position.
    fn cursor_down(&mut self) {
        let (line, col) = self.cursor_line_col();
        let total_lines = self.input_line_count();
        if line + 1 < total_lines {
            let new_line_start = self.line_start_offset(line + 1);
            let new_line_end = self.line_end_offset(line + 1);
            let new_line_len = new_line_end - new_line_start;
            self.cursor = new_line_start + col.min(new_line_len);
        }
    }

    /// Move cursor to start of current line.
    fn cursor_line_start(&mut self) {
        let (line, _) = self.cursor_line_col();
        self.cursor = self.line_start_offset(line);
    }

    /// Move cursor to end of current line.
    fn cursor_line_end(&mut self) {
        let (line, _) = self.cursor_line_col();
        self.cursor = self.line_end_offset(line);
    }

    /// Copy text to clipboard.
    pub fn copy_to_clipboard(&mut self, text: &str) {
        if let Some(clipboard) = self.get_clipboard() {
            let _ = clipboard.set_text(text.to_string());
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
            KeyCode::Char('y') => Some(Action::CopyLastMessage),
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
        // Handle Alt+Enter to insert newline
        if key.modifiers.contains(KeyModifiers::ALT) && key.code == KeyCode::Enter {
            self.input.insert(self.cursor, '\n');
            self.cursor += 1;
            return None;
        }

        // Handle Ctrl modifiers
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                // Ctrl+C - exit insert mode (like Esc)
                KeyCode::Char('c') => {
                    self.mode = ChatMode::Normal;
                    return None;
                }
                // Ctrl+U - clear from cursor to start of line
                KeyCode::Char('u') => {
                    let (line, _) = self.cursor_line_col();
                    let line_start = self.line_start_offset(line);
                    self.input.drain(line_start..self.cursor);
                    self.cursor = line_start;
                    return None;
                }
                // Ctrl+K - delete from cursor to end of line
                KeyCode::Char('k') => {
                    let (line, _) = self.cursor_line_col();
                    let line_end = self.line_end_offset(line);
                    self.input.drain(self.cursor..line_end);
                    return None;
                }
                // Ctrl+W - delete word before cursor
                KeyCode::Char('w') => {
                    self.delete_word_before_cursor();
                    return None;
                }
                // Ctrl+A - move cursor to start of current line
                KeyCode::Char('a') => {
                    self.cursor_line_start();
                    return None;
                }
                // Ctrl+E - move cursor to end of current line
                KeyCode::Char('e') => {
                    self.cursor_line_end();
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
                // Ctrl+N - move cursor down (like Down arrow)
                KeyCode::Char('n') => {
                    self.cursor_down();
                    return None;
                }
                // Ctrl+P - move cursor up (like Up arrow)
                KeyCode::Char('p') => {
                    self.cursor_up();
                    return None;
                }
                // Ctrl+V - paste from clipboard
                KeyCode::Char('v') => {
                    self.paste_from_clipboard();
                    return None;
                }
                // Ctrl+Y - yank/paste (readline style)
                KeyCode::Char('y') => {
                    self.paste_from_clipboard();
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
                    self.input_scroll = 0;

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
            KeyCode::Up => {
                self.cursor_up();
                None
            }
            KeyCode::Down => {
                self.cursor_down();
                None
            }
            KeyCode::Home => {
                self.cursor_line_start();
                None
            }
            KeyCode::End => {
                self.cursor_line_end();
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
            unread_dms,
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

        // Calculate input height based on content (min 3, max 6 lines)
        let input_line_count = self.input_line_count();
        let input_height = (input_line_count as u16 + 2).clamp(3, 6); // +2 for borders

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),               // Messages
                Constraint::Length(input_height), // Input (dynamic height)
                Constraint::Length(1),            // Status
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
        let inner_width = chunks[0].width.saturating_sub(4) as usize; // Account for borders and padding
        let message_max_width = (inner_width * 75) / 100; // 75% of available width for bubbles

        // Determine if we're in DM mode
        let is_dm_mode = dm_user.is_some();

        // Build message lines with bubble styling
        let mut all_lines: Vec<Line> = Vec::new();

        for (idx, msg) in messages.iter().enumerate() {
            // Add spacing between messages (except first)
            if idx > 0 {
                all_lines.push(Line::from(""));
            }

            // Determine message style
            let msg_style = if msg.is_system {
                // Use left-aligned for help/welcome text (contains colons, bullets, or multiple lines)
                if msg.content.contains(':')
                    || msg.content.contains('•')
                    || msg.content.contains('-')
                    || msg.content.starts_with("Welcome")
                    || msg.content.starts_with("Commands")
                    || msg.content.starts_with("Keys")
                {
                    MessageStyle::SystemLeft
                } else {
                    MessageStyle::System
                }
            } else {
                let is_own_message = username.is_some_and(|u| u == msg.author);
                if is_dm_mode {
                    if is_own_message {
                        MessageStyle::DmSent
                    } else {
                        MessageStyle::DmReceived
                    }
                } else if is_own_message {
                    MessageStyle::Sent
                } else {
                    MessageStyle::Received
                }
            };

            // Format message content with timestamp and author
            let time = msg.timestamp.format("%H:%M");
            let display_content = if msg.is_system {
                format!("[{}] {}", time, msg.content)
            } else {
                format!("[{}] {}: {}", time, msg.author, msg.content)
            };

            // Get styling based on message type
            let (text_style, _bubble_style, right_align) = match msg_style {
                MessageStyle::Sent => (
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(59, 130, 246)) // Blue
                        .add_modifier(Modifier::BOLD),
                    Some(Color::Rgb(59, 130, 246)),
                    true,
                ),
                MessageStyle::Received => (
                    Style::default()
                        .fg(Color::Rgb(55, 65, 81)) // Dark gray text
                        .bg(Color::Rgb(229, 231, 235)), // Light gray background
                    Some(Color::Rgb(229, 231, 235)),
                    false,
                ),
                MessageStyle::System => (
                    Style::default()
                        .fg(Color::Rgb(156, 163, 175))
                        .add_modifier(Modifier::ITALIC),
                    None,
                    false,
                ),
                MessageStyle::SystemLeft => (Style::default().fg(Color::Cyan), None, false),
                MessageStyle::DmSent => (
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(147, 51, 234)) // Purple
                        .add_modifier(Modifier::BOLD),
                    Some(Color::Rgb(147, 51, 234)),
                    true,
                ),
                MessageStyle::DmReceived => (
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Rgb(34, 197, 94)) // Green
                        .add_modifier(Modifier::BOLD),
                    Some(Color::Rgb(34, 197, 94)),
                    false,
                ),
            };

            // Wrap text to fit within bubble width
            let content_width = message_max_width.saturating_sub(4).max(10); // Account for padding
            let wrapped_lines = wrap_text(&display_content, content_width);

            for wrapped_line in wrapped_lines {
                let line = match msg_style {
                    MessageStyle::System => {
                        // Centered system message with bullet points
                        let system_content = format!("• {} •", wrapped_line);
                        let content_len = system_content.len();
                        let padding = inner_width.saturating_sub(content_len) / 2;
                        Line::from(vec![
                            Span::raw(" ".repeat(padding)),
                            Span::styled(system_content, text_style),
                        ])
                    }
                    MessageStyle::SystemLeft => {
                        // Left-aligned system message (for help text)
                        Line::from(vec![
                            Span::raw("  "),
                            Span::styled(wrapped_line.clone(), text_style),
                        ])
                    }
                    _ => {
                        // Bubble message with padding
                        let bubble_content = format!("  {}  ", wrapped_line);
                        let content_len = bubble_content.len();

                        if right_align {
                            // Right-align sent messages
                            let padding = inner_width.saturating_sub(content_len);
                            Line::from(vec![
                                Span::raw(" ".repeat(padding)),
                                Span::styled(bubble_content, text_style),
                            ])
                        } else {
                            // Left-align received messages
                            Line::from(vec![Span::styled(bubble_content, text_style)])
                        }
                    }
                };

                all_lines.push(line);
            }
        }

        // Calculate scroll - now based on lines, not messages
        let total_lines = all_lines.len();
        let max_scroll = total_lines.saturating_sub(inner_height);
        let scroll = self.scroll.min(max_scroll);

        // Take visible lines
        let visible_lines: Vec<ListItem> = all_lines
            .into_iter()
            .skip(scroll)
            .take(inner_height)
            .map(ListItem::new)
            .collect();

        let messages_list = List::new(visible_lines).block(messages_block);
        frame.render_widget(messages_list, chunks[0]);

        // Render scrollbar if there are more lines than can fit
        if total_lines > inner_height {
            let mut scrollbar_state = ScrollbarState::new(total_lines).position(scroll);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .track_symbol(Some("│"))
                .thumb_symbol("█");
            // Render inside the messages area (offset by 1 for border)
            let scrollbar_area = Rect {
                x: chunks[0].x + chunks[0].width.saturating_sub(1),
                y: chunks[0].y + 1,
                width: 1,
                height: chunks[0].height.saturating_sub(2),
            };
            frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }

        // Input area
        let has_target = room_name.is_some() || dm_user.is_some();
        let input_title = match (self.mode, has_target) {
            (ChatMode::Normal, true) => " Press 'i' to type ",
            (ChatMode::Normal, false) => " Press 'r' to join a room ",
            (ChatMode::Insert, true) => " Enter:send Alt+Enter:newline Esc:cancel ",
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

        // Render multi-line input with cursor
        let input_inner_height = chunks[1].height.saturating_sub(2) as usize; // -2 for borders
        let (cursor_line, cursor_col) = self.cursor_line_col();

        // Adjust input scroll to keep cursor visible
        if cursor_line < self.input_scroll {
            self.input_scroll = cursor_line;
        } else if cursor_line >= self.input_scroll + input_inner_height {
            self.input_scroll = cursor_line - input_inner_height + 1;
        }

        // Build lines for display
        let input_lines: Vec<&str> = self.input.split('\n').collect();
        let mut display_lines: Vec<Line> = Vec::new();

        for (line_idx, line_text) in input_lines
            .iter()
            .enumerate()
            .skip(self.input_scroll)
            .take(input_inner_height)
        {
            if self.mode == ChatMode::Insert && line_idx == cursor_line {
                // This line has the cursor
                let col = cursor_col.min(line_text.len());
                let (before, after) = line_text.split_at(col);
                display_lines.push(Line::from(vec![
                    Span::raw(before.to_string()),
                    Span::styled(
                        "│",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(after.to_string()),
                ]));
            } else {
                display_lines.push(Line::from(line_text.to_string()));
            }
        }

        let input_para = Paragraph::new(display_lines).block(input_block);
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
            (ChatMode::Insert, _) => " Esc:normal Enter:send Alt+Enter:newline ",
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
                // Check for unread DMs
                let display = if let Some(&count) = unread_dms.get(name) {
                    Line::from(vec![
                        Span::styled(format!("{}{} ", prefix, name), style),
                        Span::styled(
                            format!("({})", count),
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                    ])
                } else {
                    Line::from(Span::styled(format!("{}{}", prefix, name), style))
                };
                user_items.push(ListItem::new(display));
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
                // Check for unread DMs
                let display = if let Some(&count) = unread_dms.get(name) {
                    Line::from(vec![
                        Span::styled(format!("{}{} ", prefix, name), style),
                        Span::styled(
                            format!("({})", count),
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                    ])
                } else {
                    Line::from(Span::styled(format!("{}{}", prefix, name), style))
                };
                user_items.push(ListItem::new(display));
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
