//! User list panel UI component for Lair-Chat
//! Displays online users with presence indicators, search functionality, and DM initiation.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{block::Title, Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::chat::{UserId, UserPresence, UserStatus};

/// Events that can be emitted by the user list panel
#[derive(Debug, Clone)]
pub enum UserListEvent {
    /// User selected for starting a DM
    UserSelected(UserId),
    /// Search query changed
    SearchChanged(String),
    /// Request to refresh user list
    RefreshRequested,
    /// Panel focus requested
    FocusRequested,
    /// Panel dismissed/closed
    Dismissed,
}

/// User list panel state
#[derive(Debug, Clone)]
pub struct UserListState {
    /// List of users to display
    pub users: Vec<UserPresence>,
    /// Currently selected user index
    pub selected_index: Option<usize>,
    /// Search query
    pub search_query: String,
    /// Whether the search input is focused
    pub search_focused: bool,
    /// Scroll state for the user list
    pub list_state: ListState,
    /// Whether the panel is visible
    pub visible: bool,
    /// Whether the panel has focus
    pub focused: bool,
    /// Filter settings
    pub filter: UserFilter,
    /// User typing indicators
    pub typing_indicators: HashMap<UserId, String>,
    /// Last refresh timestamp
    pub last_refresh: Option<u64>,
}

/// User filtering options
#[derive(Debug, Clone)]
pub struct UserFilter {
    /// Show only online users
    pub online_only: bool,
    /// Show only available users (online/idle)
    pub available_only: bool,
    /// Exclude current user
    pub exclude_current: bool,
    /// Current user ID for exclusion
    pub current_user_id: Option<UserId>,
}

impl Default for UserFilter {
    fn default() -> Self {
        Self {
            online_only: false,
            available_only: true, // Default to available users
            exclude_current: true,
            current_user_id: None,
        }
    }
}

impl Default for UserListState {
    fn default() -> Self {
        Self {
            users: Vec::new(),
            selected_index: None,
            search_query: String::new(),
            search_focused: false,
            list_state: ListState::default(),
            visible: false,
            focused: false,
            filter: UserFilter::default(),
            typing_indicators: HashMap::new(),
            last_refresh: None,
        }
    }
}

impl UserListState {
    /// Create new user list state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current user ID for filtering
    pub fn set_current_user(&mut self, user_id: UserId) {
        self.filter.current_user_id = Some(user_id);
    }

    /// Update user list
    pub fn update_users(&mut self, users: Vec<UserPresence>) {
        // Apply filters
        let filtered_users = self.apply_filters(users);

        // Apply search if there's a query
        self.users = if self.search_query.is_empty() {
            filtered_users
        } else {
            self.filter_by_search(filtered_users)
        };

        // Reset selection if it's out of bounds or users list is empty
        if self.users.is_empty() {
            self.selected_index = None;
        } else if let Some(index) = self.selected_index {
            if index >= self.users.len() {
                self.selected_index = Some(0);
            }
        } else {
            // If no selection and we have users, select the first one
            self.selected_index = Some(0);
        }

        // Update list state - ensure valid state
        self.list_state.select(self.selected_index);
        self.last_refresh = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    /// Apply user filters
    fn apply_filters(&self, users: Vec<UserPresence>) -> Vec<UserPresence> {
        users
            .into_iter()
            .filter(|user| {
                // Exclude current user if enabled
                if self.filter.exclude_current {
                    if let Some(current_id) = self.filter.current_user_id {
                        if user.user_id == current_id {
                            return false;
                        }
                    }
                }

                // Filter by online status
                if self.filter.online_only && !user.is_online() {
                    return false;
                }

                // Filter by availability
                if self.filter.available_only && !user.is_available() {
                    return false;
                }

                true
            })
            .collect()
    }

    /// Filter users by search query
    fn filter_by_search(&self, users: Vec<UserPresence>) -> Vec<UserPresence> {
        let query = self.search_query.to_lowercase();
        users
            .into_iter()
            .filter(|user| {
                user.username.to_lowercase().contains(&query)
                    || user
                        .display_name
                        .as_ref()
                        .map(|name| name.to_lowercase().contains(&query))
                        .unwrap_or(false)
            })
            .collect()
    }

    /// Update search query
    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        // Reapply filters when search changes
        let current_users = self.users.clone();
        self.update_users(current_users);
    }

    /// Clear search query
    pub fn clear_search(&mut self) {
        self.set_search_query(String::new());
    }

    /// Select next user
    pub fn select_next(&mut self) {
        if self.users.is_empty() {
            self.selected_index = None;
            self.list_state.select(None);
            return;
        }

        let next_index = match self.selected_index {
            Some(index) => {
                if index >= self.users.len().saturating_sub(1) {
                    0
                } else {
                    index + 1
                }
            }
            None => 0,
        };

        self.selected_index = Some(next_index);
        self.list_state.select(self.selected_index);
    }

    /// Select previous user
    pub fn select_previous(&mut self) {
        if self.users.is_empty() {
            self.selected_index = None;
            self.list_state.select(None);
            return;
        }

        let prev_index = match self.selected_index {
            Some(index) => {
                if index == 0 {
                    self.users.len().saturating_sub(1)
                } else {
                    index.saturating_sub(1)
                }
            }
            None => self.users.len().saturating_sub(1),
        };

        self.selected_index = Some(prev_index);
        self.list_state.select(self.selected_index);
    }

    /// Get currently selected user
    pub fn selected_user(&self) -> Option<&UserPresence> {
        if self.users.is_empty() {
            return None;
        }
        self.selected_index.and_then(|index| self.users.get(index))
    }

    /// Show the panel
    pub fn show(&mut self) {
        self.visible = true;
        self.focused = true;
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.visible = false;
        self.focused = false;
        self.search_focused = false;
    }

    /// Toggle search focus
    pub fn toggle_search_focus(&mut self) {
        self.search_focused = !self.search_focused;
    }

    /// Set search focus
    pub fn set_search_focus(&mut self, focused: bool) {
        self.search_focused = focused;
    }

    /// Update typing indicator for a user
    pub fn set_user_typing(&mut self, user_id: UserId, typing_to: Option<UserId>) {
        if let Some(typing_to) = typing_to {
            self.typing_indicators
                .insert(user_id, format!("typing to {}", typing_to));
        } else {
            self.typing_indicators.remove(&user_id);
        }
    }

    /// Get user count
    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    /// Check if panel is empty
    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }
}

/// User list panel widget
pub struct UserListPanel {
    /// Panel state
    state: UserListState,
    /// Event sender
    event_sender: Option<mpsc::UnboundedSender<UserListEvent>>,
    /// Panel title
    title: String,
    /// Show presence details
    show_presence_details: bool,
    /// Show typing indicators
    show_typing_indicators: bool,
}

impl UserListPanel {
    /// Create new user list panel
    pub fn new() -> Self {
        Self {
            state: UserListState::new(),
            event_sender: None,
            title: "Online Users".to_string(),
            show_presence_details: true,
            show_typing_indicators: true,
        }
    }

    /// Create user list panel with event sender
    pub fn with_event_sender(event_sender: mpsc::UnboundedSender<UserListEvent>) -> Self {
        Self {
            state: UserListState::new(),
            event_sender: Some(event_sender),
            title: "Online Users".to_string(),
            show_presence_details: true,
            show_typing_indicators: true,
        }
    }

    /// Set panel title
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    /// Get mutable reference to state
    pub fn state_mut(&mut self) -> &mut UserListState {
        &mut self.state
    }

    /// Get reference to state
    pub fn state(&self) -> &UserListState {
        &self.state
    }

    /// Send event if sender is available
    fn send_event(&self, event: UserListEvent) {
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event);
        }
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, event: KeyEvent) -> bool {
        if !self.state.visible || !self.state.focused {
            return false;
        }

        match event.code {
            KeyCode::Esc => {
                if self.state.search_focused {
                    self.state.set_search_focus(false);
                } else {
                    self.state.hide();
                    self.send_event(UserListEvent::Dismissed);
                }
                true
            }
            KeyCode::Enter => {
                if self.state.search_focused {
                    self.state.set_search_focus(false);
                } else if !self.state.users.is_empty() {
                    if let Some(user) = self.state.selected_user() {
                        self.send_event(UserListEvent::UserSelected(user.user_id));
                        self.state.hide();
                    }
                }
                true
            }
            KeyCode::Up | KeyCode::Char('k') if !self.state.search_focused => {
                if !self.state.users.is_empty() {
                    self.state.select_previous();
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') if !self.state.search_focused => {
                if !self.state.users.is_empty() {
                    self.state.select_next();
                }
                true
            }
            KeyCode::Char('/') if !self.state.search_focused => {
                self.state.set_search_focus(true);
                true
            }
            KeyCode::Char('r') if !self.state.search_focused => {
                self.send_event(UserListEvent::RefreshRequested);
                true
            }
            KeyCode::Char('c') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.clear_search();
                self.send_event(UserListEvent::SearchChanged(String::new()));
                true
            }
            KeyCode::Backspace if self.state.search_focused => {
                self.state.search_query.pop();
                let query = self.state.search_query.clone();
                self.send_event(UserListEvent::SearchChanged(query));
                true
            }
            KeyCode::Char(c) if self.state.search_focused => {
                self.state.search_query.push(c);
                let query = self.state.search_query.clone();
                self.send_event(UserListEvent::SearchChanged(query));
                true
            }
            _ => false,
        }
    }

    /// Render the user list panel
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if !self.state.visible {
            return;
        }

        // Create popup area (centered)
        let popup_area = self.centered_rect(60, 80, area);

        // Clear the background
        f.render_widget(Clear, popup_area);

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search box
                Constraint::Min(5),    // User list
                Constraint::Length(2), // Status/help
            ])
            .split(popup_area);

        // Render main border
        let main_block = Block::default()
            .title_top(Line::from(self.title.as_str()).alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_style(if self.state.focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Gray)
            });

        f.render_widget(main_block, popup_area);

        // Render search box
        self.render_search_box(f, chunks[0]);

        // Render user list
        self.render_user_list(f, chunks[1]);

        // Render status bar
        self.render_status_bar(f, chunks[2]);
    }

    /// Render search box
    fn render_search_box(&self, f: &mut Frame, area: Rect) {
        let search_area = area.inner(Margin {
            vertical: 0,
            horizontal: 1,
        });

        let search_text = if self.state.search_query.is_empty() {
            "Search users... (press '/' to search)"
        } else {
            &self.state.search_query
        };

        let search_block =
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(if self.state.search_focused {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Gray)
                });

        let search_widget =
            Paragraph::new(search_text)
                .block(search_block)
                .style(if self.state.search_focused {
                    Style::default().fg(Color::Yellow)
                } else if self.state.search_query.is_empty() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                });

        f.render_widget(search_widget, search_area);

        // Show cursor if search is focused
        if self.state.search_focused {
            let cursor_x = search_area.x + self.state.search_query.len() as u16;
            let cursor_y = search_area.y;
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }

    /// Render user list
    fn render_user_list(&self, f: &mut Frame, area: Rect) {
        let list_area = area.inner(Margin {
            vertical: 0,
            horizontal: 1,
        });

        if self.state.users.is_empty() {
            let empty_text = if self.state.search_query.is_empty() {
                "No users online"
            } else {
                "No users match your search"
            };

            let empty_widget = Paragraph::new(empty_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

            f.render_widget(empty_widget, list_area);
            return;
        }

        // Create list items
        let items: Vec<ListItem> = self
            .state
            .users
            .iter()
            .map(|user| self.create_user_item(user))
            .collect();

        // Create list widget
        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("► ");

        // Render with selection state
        let mut list_state = self.state.list_state.clone();
        f.render_stateful_widget(list, list_area, &mut list_state);
    }

    /// Create a list item for a user
    fn create_user_item(&self, user: &UserPresence) -> ListItem {
        let mut spans = Vec::new();

        // Status indicator
        let (status_symbol, status_color) = match user.status {
            UserStatus::Online => ("●", Color::Green),
            UserStatus::Idle => ("◐", Color::Yellow),
            UserStatus::Away => ("○", Color::Yellow),
            UserStatus::Offline => ("○", Color::Red),
            UserStatus::Banned => ("✖", Color::Red),
            UserStatus::Left => ("←", Color::Gray),
        };

        spans.push(Span::styled(
            format!("{} ", status_symbol),
            Style::default().fg(status_color),
        ));

        // Check for unread DM indicator in status message
        let has_unread = user
            .status_message
            .as_ref()
            .map(|msg| msg.contains("unread"))
            .unwrap_or(false);

        if has_unread {
            spans.push(Span::styled("🔔 ", Style::default().fg(Color::Yellow)));
        }

        // Username/display name
        let display_name = user.display_name();
        spans.push(Span::styled(
            display_name.to_string(),
            Style::default().fg(Color::White),
        ));

        // Show unread DM count if present
        if has_unread {
            if let Some(status_msg) = &user.status_message {
                spans.push(Span::styled(
                    format!(" ({})", status_msg),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
            }
        }

        // Show typing indicator if enabled
        if self.show_typing_indicators {
            if let Some(typing_status) = self.state.typing_indicators.get(&user.user_id) {
                spans.push(Span::styled(
                    format!(" ({})", typing_status),
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
        }

        // Show presence details if enabled (but not if showing unread count)
        if self.show_presence_details && user.status != UserStatus::Online && !has_unread {
            let last_seen = user.human_last_seen();
            spans.push(Span::styled(
                format!(" - {}", last_seen),
                Style::default().fg(Color::DarkGray),
            ));
        }

        ListItem::new(Line::from(spans))
    }

    /// Render status bar
    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let status_area = area.inner(Margin {
            vertical: 0,
            horizontal: 1,
        });

        let mut status_text = Vec::new();

        // User count
        let count_text = format!("{} users", self.state.users.len());
        status_text.push(Span::styled(count_text, Style::default().fg(Color::Cyan)));

        // Filter status
        if self.state.filter.available_only {
            status_text.push(Span::styled(
                " | Available only",
                Style::default().fg(Color::Green),
            ));
        }
        if self.state.filter.online_only {
            status_text.push(Span::styled(
                " | Online only",
                Style::default().fg(Color::Green),
            ));
        }

        // Help text
        if !self.state.search_focused {
            status_text.push(Span::styled(
                " | ↑↓:navigate /:search r:refresh Enter:select Esc:close",
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            status_text.push(Span::styled(
                " | Type to search, Enter:done, Esc:cancel",
                Style::default().fg(Color::DarkGray),
            ));
        }

        let status_widget = Paragraph::new(Line::from(status_text))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        f.render_widget(status_widget, status_area);
    }

    /// Helper function to create a centered rectangle
    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
}

impl Default for UserListPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_list_state_creation() {
        let state = UserListState::new();
        assert!(!state.visible);
        assert!(!state.focused);
        assert!(state.users.is_empty());
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn test_user_filtering() {
        let mut state = UserListState::new();
        let current_user = uuid::Uuid::new_v4();
        state.set_current_user(current_user);

        let users = vec![
            UserPresence::new(current_user, "current".to_string()),
            UserPresence::new(uuid::Uuid::new_v4(), "online".to_string()),
            {
                let mut user = UserPresence::new(uuid::Uuid::new_v4(), "away".to_string());
                user.set_status(UserStatus::Away);
                user
            },
        ];

        state.update_users(users);

        // Should exclude current user and away user (available_only is true by default)
        assert_eq!(state.users.len(), 1);
        assert_eq!(state.users[0].username, "online");
    }

    #[test]
    fn test_search_functionality() {
        let mut state = UserListState::new();

        let users = vec![
            UserPresence::new(uuid::Uuid::new_v4(), "alice".to_string()),
            UserPresence::new(uuid::Uuid::new_v4(), "bob".to_string()),
            UserPresence::new(uuid::Uuid::new_v4(), "charlie".to_string()),
        ];

        state.update_users(users.clone());
        assert_eq!(state.users.len(), 3);

        // Test search - need to update with original users after setting search query
        state.set_search_query("ali".to_string());
        state.update_users(users.clone()); // Re-apply with search filter
        assert_eq!(state.users.len(), 1);
        assert_eq!(state.users[0].username, "alice");

        // Clear search
        state.clear_search();
        state.update_users(users); // Re-apply without search filter
        assert_eq!(state.users.len(), 3);
    }

    #[test]
    fn test_navigation() {
        let mut state = UserListState::new();

        let users = vec![
            UserPresence::new(uuid::Uuid::new_v4(), "user1".to_string()),
            UserPresence::new(uuid::Uuid::new_v4(), "user2".to_string()),
            UserPresence::new(uuid::Uuid::new_v4(), "user3".to_string()),
        ];

        state.update_users(users);

        // Test navigation
        assert_eq!(state.selected_index, Some(0));

        state.select_next();
        assert_eq!(state.selected_index, Some(1));

        state.select_next();
        assert_eq!(state.selected_index, Some(2));

        // Wrap around
        state.select_next();
        assert_eq!(state.selected_index, Some(0));

        // Go back
        state.select_previous();
        assert_eq!(state.selected_index, Some(2));
    }

    #[test]
    fn test_typing_indicators() {
        let mut state = UserListState::new();
        let user1 = uuid::Uuid::new_v4();
        let user2 = uuid::Uuid::new_v4();

        state.set_user_typing(user1, Some(user2));
        assert!(state.typing_indicators.contains_key(&user1));

        state.set_user_typing(user1, None);
        assert!(!state.typing_indicators.contains_key(&user1));
    }
}
