//! DM navigation and conversation management UI component for Lair-Chat
//! Provides conversation list, navigation between DMs, and overall DM management interface.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::chat::{ConversationId, ConversationSummary, UserId, UserPresence};

/// Events that can be emitted by the DM navigation component
#[derive(Debug, Clone)]
pub enum NavigationEvent {
    /// Open conversation with user
    OpenConversation(ConversationId),
    /// Start new DM with user
    StartNewDM(UserId),
    /// Show user list for starting new DM
    ShowUserList,
    /// Archive conversation
    ArchiveConversation(ConversationId),
    /// Unarchive conversation
    UnarchiveConversation(ConversationId),
    /// Mute conversation
    MuteConversation(ConversationId),
    /// Unmute conversation
    UnmuteConversation(ConversationId),
    /// Delete conversation
    DeleteConversation(ConversationId),
    /// Mark conversation as read
    MarkConversationRead(ConversationId),
    /// Mark all conversations as read
    MarkAllRead,
    /// Search conversations
    SearchConversations(String),
    /// Refresh conversation list
    RefreshConversations,
    /// Toggle archived view
    ToggleArchivedView,
    /// Navigation panel closed
    NavigationClosed,
}

/// Navigation view mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavigationViewMode {
    /// Show active conversations
    Active,
    /// Show archived conversations
    Archived,
    /// Show all conversations
    All,
}

/// DM navigation state
#[derive(Debug, Clone)]
pub struct NavigationState {
    /// List of conversations
    pub conversations: Vec<ConversationSummary>,
    /// Currently selected conversation index
    pub selected_index: Option<usize>,
    /// List state for conversation selection
    pub list_state: ListState,
    /// Search query
    pub search_query: String,
    /// Whether search is active
    pub search_active: bool,
    /// Current view mode
    pub view_mode: NavigationViewMode,
    /// Whether the panel is visible
    pub visible: bool,
    /// Whether the panel has focus
    pub focused: bool,
    /// Current user ID
    pub current_user_id: Option<UserId>,
    /// User presence cache for display names
    pub user_cache: HashMap<UserId, UserPresence>,
    /// Show unread count in title
    pub show_unread_counts: bool,
    /// Show last message preview
    pub show_message_preview: bool,
    /// Show timestamps
    pub show_timestamps: bool,
    /// Sort order (newest first or alphabetical)
    pub sort_by_activity: bool,
}

impl Default for NavigationState {
    fn default() -> Self {
        Self {
            conversations: Vec::new(),
            selected_index: None,
            list_state: ListState::default(),
            search_query: String::new(),
            search_active: false,
            view_mode: NavigationViewMode::Active,
            visible: false,
            focused: false,
            current_user_id: None,
            user_cache: HashMap::new(),
            show_unread_counts: true,
            show_message_preview: true,
            show_timestamps: true,
            sort_by_activity: true,
        }
    }
}

impl NavigationState {
    /// Create new navigation state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set current user
    pub fn set_current_user(&mut self, user_id: UserId) {
        self.current_user_id = Some(user_id);
    }

    /// Update conversation list
    pub fn update_conversations(&mut self, conversations: Vec<ConversationSummary>) {
        // Apply filters based on view mode
        self.conversations = match self.view_mode {
            NavigationViewMode::Active => conversations
                .into_iter()
                .filter(|conv| !conv.is_archived)
                .collect(),
            NavigationViewMode::Archived => conversations
                .into_iter()
                .filter(|conv| conv.is_archived)
                .collect(),
            NavigationViewMode::All => conversations,
        };

        // Apply search filter if active
        if self.search_active && !self.search_query.is_empty() {
            self.conversations = self.filter_by_search();
        }

        // Sort conversations
        if self.sort_by_activity {
            self.conversations
                .sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        } else {
            self.conversations
                .sort_by(|a, b| a.other_username.cmp(&b.other_username));
        }

        // Reset selection if it's out of bounds
        if let Some(index) = self.selected_index {
            if index >= self.conversations.len() {
                self.selected_index = if self.conversations.is_empty() {
                    None
                } else {
                    Some(0)
                };
            }
        } else if !self.conversations.is_empty() {
            self.selected_index = Some(0);
        }

        self.list_state.select(self.selected_index);
    }

    /// Filter conversations by search query
    fn filter_by_search(&self) -> Vec<ConversationSummary> {
        let query = self.search_query.to_lowercase();
        self.conversations
            .iter()
            .filter(|conv| {
                conv.other_username.to_lowercase().contains(&query)
                    || conv
                        .last_message
                        .as_ref()
                        .map(|msg| msg.to_lowercase().contains(&query))
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Set search query
    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        if self.search_active {
            // Reapply filters
            let all_conversations = self.conversations.clone();
            self.update_conversations(all_conversations);
        }
    }

    /// Toggle search mode
    pub fn toggle_search(&mut self) {
        self.search_active = !self.search_active;
        if !self.search_active {
            self.search_query.clear();
        }
    }

    /// Set view mode
    pub fn set_view_mode(&mut self, mode: NavigationViewMode) {
        self.view_mode = mode;
        // Reapply filters
        let all_conversations = self.conversations.clone();
        self.update_conversations(all_conversations);
    }

    /// Select next conversation
    pub fn select_next(&mut self) {
        if self.conversations.is_empty() {
            return;
        }

        let next_index = match self.selected_index {
            Some(index) => {
                if index >= self.conversations.len() - 1 {
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

    /// Select previous conversation
    pub fn select_previous(&mut self) {
        if self.conversations.is_empty() {
            return;
        }

        let prev_index = match self.selected_index {
            Some(index) => {
                if index == 0 {
                    self.conversations.len() - 1
                } else {
                    index - 1
                }
            }
            None => self.conversations.len() - 1,
        };

        self.selected_index = Some(prev_index);
        self.list_state.select(self.selected_index);
    }

    /// Get currently selected conversation
    pub fn selected_conversation(&self) -> Option<&ConversationSummary> {
        self.selected_index
            .and_then(|index| self.conversations.get(index))
    }

    /// Get total unread count
    pub fn total_unread_count(&self) -> u32 {
        self.conversations
            .iter()
            .map(|conv| conv.unread_count)
            .sum()
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
        self.search_active = false;
    }

    /// Update user cache
    pub fn update_user_cache(&mut self, users: Vec<UserPresence>) {
        self.user_cache.clear();
        for user in users {
            self.user_cache.insert(user.user_id, user);
        }
    }

    /// Get display name for user
    pub fn get_user_display_name(&self, user_id: UserId) -> String {
        self.user_cache
            .get(&user_id)
            .map(|user| user.display_name().to_string())
            .unwrap_or_else(|| format!("User {}", user_id))
    }
}

/// DM navigation panel widget
pub struct NavigationPanel {
    /// Panel state
    state: NavigationState,
    /// Event sender
    event_sender: Option<mpsc::UnboundedSender<NavigationEvent>>,
    /// Panel title
    title: String,
    /// Show conversation actions (archive, mute, etc.)
    show_actions: bool,
    /// Compact display mode
    compact_mode: bool,
}

impl NavigationPanel {
    /// Create new navigation panel
    pub fn new() -> Self {
        Self {
            state: NavigationState::new(),
            event_sender: None,
            title: "Direct Messages".to_string(),
            show_actions: true,
            compact_mode: false,
        }
    }

    /// Create navigation panel with event sender
    pub fn with_event_sender(event_sender: mpsc::UnboundedSender<NavigationEvent>) -> Self {
        Self {
            state: NavigationState::new(),
            event_sender: Some(event_sender),
            title: "Direct Messages".to_string(),
            show_actions: true,
            compact_mode: false,
        }
    }

    /// Get mutable reference to state
    pub fn state_mut(&mut self) -> &mut NavigationState {
        &mut self.state
    }

    /// Get reference to state
    pub fn state(&self) -> &NavigationState {
        &self.state
    }

    /// Send event if sender is available
    fn send_event(&self, event: NavigationEvent) {
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
                if self.state.search_active {
                    self.state.toggle_search();
                } else {
                    self.state.hide();
                    self.send_event(NavigationEvent::NavigationClosed);
                }
                true
            }
            KeyCode::Enter => {
                if let Some(conversation) = self.state.selected_conversation() {
                    self.send_event(NavigationEvent::OpenConversation(conversation.id.clone()));
                }
                true
            }
            KeyCode::Up | KeyCode::Char('k') if !self.state.search_active => {
                self.state.select_previous();
                true
            }
            KeyCode::Down | KeyCode::Char('j') if !self.state.search_active => {
                self.state.select_next();
                true
            }
            KeyCode::Char('n') if !self.state.search_active => {
                self.send_event(NavigationEvent::ShowUserList);
                true
            }
            KeyCode::Char('/') if !self.state.search_active => {
                self.state.toggle_search();
                true
            }
            KeyCode::Char('a') if !self.state.search_active => {
                if let Some(conversation) = self.state.selected_conversation() {
                    if conversation.is_archived {
                        self.send_event(NavigationEvent::UnarchiveConversation(
                            conversation.id.clone(),
                        ));
                    } else {
                        self.send_event(NavigationEvent::ArchiveConversation(
                            conversation.id.clone(),
                        ));
                    }
                }
                true
            }
            KeyCode::Char('m') if !self.state.search_active => {
                if let Some(conversation) = self.state.selected_conversation() {
                    if conversation.is_muted {
                        self.send_event(NavigationEvent::UnmuteConversation(
                            conversation.id.clone(),
                        ));
                    } else {
                        self.send_event(NavigationEvent::MuteConversation(conversation.id.clone()));
                    }
                }
                true
            }
            KeyCode::Char('r') if !self.state.search_active => {
                if let Some(conversation) = self.state.selected_conversation() {
                    self.send_event(NavigationEvent::MarkConversationRead(
                        conversation.id.clone(),
                    ));
                }
                true
            }
            KeyCode::Char('R') if !self.state.search_active => {
                self.send_event(NavigationEvent::MarkAllRead);
                true
            }
            KeyCode::F(5) if !self.state.search_active => {
                self.send_event(NavigationEvent::RefreshConversations);
                true
            }
            KeyCode::Tab if !self.state.search_active => {
                let next_mode = match self.state.view_mode {
                    NavigationViewMode::Active => NavigationViewMode::Archived,
                    NavigationViewMode::Archived => NavigationViewMode::All,
                    NavigationViewMode::All => NavigationViewMode::Active,
                };
                self.state.set_view_mode(next_mode);
                true
            }
            KeyCode::Backspace if self.state.search_active => {
                self.state.search_query.pop();
                let query = self.state.search_query.clone();
                self.send_event(NavigationEvent::SearchConversations(query));
                true
            }
            KeyCode::Char(c) if self.state.search_active => {
                self.state.search_query.push(c);
                let query = self.state.search_query.clone();
                self.send_event(NavigationEvent::SearchConversations(query));
                true
            }
            _ => false,
        }
    }

    /// Render the navigation panel
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if !self.state.visible {
            return;
        }

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),                                            // Header
                Constraint::Length(if self.state.search_active { 3 } else { 0 }), // Search
                Constraint::Min(5),    // Conversation list
                Constraint::Length(2), // Status bar
            ])
            .split(area);

        // Render header
        self.render_header(f, chunks[0]);

        // Render search box if active
        if self.state.search_active {
            self.render_search(f, chunks[1]);
        }

        // Render conversation list
        let list_area = if self.state.search_active {
            chunks[2]
        } else {
            chunks[1]
        };
        self.render_conversation_list(f, list_area);

        // Render status bar
        let status_area = if self.state.search_active {
            chunks[3]
        } else {
            chunks[2]
        };
        self.render_status_bar(f, status_area);
    }

    /// Render navigation header
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let mut title_spans = vec![Span::styled(
            self.title.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )];

        // Add unread count
        let unread_count = self.state.total_unread_count();
        if unread_count > 0 {
            title_spans.push(Span::styled(
                format!(" ({})", unread_count),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        }

        // Add view mode indicator
        let mode_text = match self.state.view_mode {
            NavigationViewMode::Active => "",
            NavigationViewMode::Archived => " [Archived]",
            NavigationViewMode::All => " [All]",
        };

        if !mode_text.is_empty() {
            title_spans.push(Span::styled(mode_text, Style::default().fg(Color::Yellow)));
        }

        let header_block = Block::default()
            .title_top(Line::from(title_spans).alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_style(if self.state.focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Gray)
            });

        f.render_widget(header_block, area);
    }

    /// Render search box
    fn render_search(&self, f: &mut Frame, area: Rect) {
        let search_area = area.inner(Margin {
            vertical: 0,
            horizontal: 1,
        });

        let search_text = if self.state.search_query.is_empty() {
            "Type to search conversations..."
        } else {
            &self.state.search_query
        };

        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title("Search");

        let search_widget = Paragraph::new(search_text).block(search_block).style(
            if self.state.search_query.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            },
        );

        f.render_widget(search_widget, search_area);

        // Show cursor
        if self.state.search_active {
            let cursor_x = search_area.x + self.state.search_query.len() as u16 + 1;
            let cursor_y = search_area.y + 1;
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }

    /// Render conversation list
    fn render_conversation_list(&self, f: &mut Frame, area: Rect) {
        let list_area = area.inner(Margin {
            vertical: 0,
            horizontal: 1,
        });

        if self.state.conversations.is_empty() {
            let empty_text = match self.state.view_mode {
                NavigationViewMode::Active => {
                    "No active conversations. Press 'n' to start a new DM."
                }
                NavigationViewMode::Archived => "No archived conversations.",
                NavigationViewMode::All => "No conversations yet. Press 'n' to start a new DM.",
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
            .conversations
            .iter()
            .map(|conv| self.create_conversation_item(conv))
            .collect();

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

    /// Create a list item for a conversation
    fn create_conversation_item(&self, conversation: &ConversationSummary) -> ListItem {
        let mut lines = Vec::new();

        // First line: Username and status
        let mut first_line = Vec::new();

        // Unread indicator
        if conversation.unread_count > 0 {
            first_line.push(Span::styled(
                "● ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            first_line.push(Span::styled("  ", Style::default()));
        }

        // Username
        first_line.push(Span::styled(
            conversation.other_username.clone(),
            if conversation.unread_count > 0 {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            },
        ));

        // Unread count
        if conversation.unread_count > 1 {
            first_line.push(Span::styled(
                format!(" ({})", conversation.unread_count),
                Style::default().fg(Color::Red),
            ));
        }

        // Muted indicator
        if conversation.is_muted {
            first_line.push(Span::styled(" 🔇", Style::default().fg(Color::Yellow)));
        }

        // Timestamp
        if self.state.show_timestamps {
            let time_text = self.format_timestamp(conversation.last_activity);
            first_line.push(Span::styled(
                format!(" - {}", time_text),
                Style::default().fg(Color::DarkGray),
            ));
        }

        lines.push(Line::from(first_line));

        // Second line: Last message preview
        if self.state.show_message_preview {
            if let Some(preview) = &conversation.last_message {
                let preview_text = if preview.len() > 60 {
                    format!("  {}...", &preview[..57])
                } else {
                    format!("  {}", preview)
                };

                lines.push(Line::from(vec![Span::styled(
                    preview_text,
                    Style::default().fg(Color::DarkGray),
                )]));
            } else {
                lines.push(Line::from(vec![Span::styled(
                    "  No messages yet",
                    Style::default().fg(Color::DarkGray),
                )]));
            }
        }

        // Add spacing if not in compact mode
        if !self.compact_mode {
            lines.push(Line::from(""));
        }

        ListItem::new(lines)
    }

    /// Format timestamp for display
    fn format_timestamp(&self, timestamp: u64) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let age = now.saturating_sub(timestamp);

        if age < 60 {
            "now".to_string()
        } else if age < 3600 {
            format!("{}m", age / 60)
        } else if age < 86400 {
            format!("{}h", age / 3600)
        } else if age < 604800 {
            format!("{}d", age / 86400)
        } else {
            format!("{}w", age / 604800)
        }
    }

    /// Render status bar
    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let mut status_spans = Vec::new();

        // Conversation count
        let count_text = format!("{} conversations", self.state.conversations.len());
        status_spans.push(Span::styled(count_text, Style::default().fg(Color::Cyan)));

        // View mode
        let mode_text = match self.state.view_mode {
            NavigationViewMode::Active => "Active",
            NavigationViewMode::Archived => "Archived",
            NavigationViewMode::All => "All",
        };
        status_spans.push(Span::styled(
            format!(" | {}", mode_text),
            Style::default().fg(Color::Green),
        ));

        // Help text
        if !self.state.search_active {
            status_spans.push(Span::styled(
                " | ↑↓:navigate n:new /:search a:archive m:mute r:read Tab:mode",
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            status_spans.push(Span::styled(
                " | Type to search, Esc:cancel",
                Style::default().fg(Color::DarkGray),
            ));
        }

        let status_widget = Paragraph::new(Line::from(status_spans))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        f.render_widget(status_widget, area);
    }
}

impl Default for NavigationPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_state_creation() {
        let state = NavigationState::new();
        assert!(!state.visible);
        assert!(!state.focused);
        assert!(state.conversations.is_empty());
        assert_eq!(state.view_mode, NavigationViewMode::Active);
    }

    #[test]
    fn test_conversation_filtering() {
        let mut state = NavigationState::new();

        let conversations = vec![
            ConversationSummary {
                id: ConversationId::from("conv1"),
                other_user_id: uuid::Uuid::new_v4(),
                other_username: "alice".to_string(),
                last_message: Some("Hello".to_string()),
                last_activity: 100,
                unread_count: 1,
                is_archived: false,
                is_muted: false,
            },
            ConversationSummary {
                id: ConversationId::from("conv2"),
                other_user_id: uuid::Uuid::new_v4(),
                other_username: "bob".to_string(),
                last_message: Some("Hi there".to_string()),
                last_activity: 200,
                unread_count: 0,
                is_archived: true,
                is_muted: false,
            },
        ];

        // Test active view
        state.set_view_mode(NavigationViewMode::Active);
        state.update_conversations(conversations.clone());
        assert_eq!(state.conversations.len(), 1);
        assert_eq!(state.conversations[0].other_username, "alice");

        // Test archived view
        state.set_view_mode(NavigationViewMode::Archived);
        state.update_conversations(conversations.clone());
        assert_eq!(state.conversations.len(), 1);
        assert_eq!(state.conversations[0].other_username, "bob");

        // Test all view
        state.set_view_mode(NavigationViewMode::All);
        state.update_conversations(conversations);
        assert_eq!(state.conversations.len(), 2);
    }

    #[test]
    fn test_search_functionality() {
        let mut state = NavigationState::new();

        let conversations = vec![
            ConversationSummary {
                id: ConversationId::from("conv1"),
                other_user_id: uuid::Uuid::new_v4(),
                other_username: "alice".to_string(),
                last_message: Some("Hello world".to_string()),
                last_activity: 100,
                unread_count: 0,
                is_archived: false,
                is_muted: false,
            },
            ConversationSummary {
                id: ConversationId::from("conv2"),
                other_user_id: uuid::Uuid::new_v4(),
                other_username: "bob".to_string(),
                last_message: Some("Hi there".to_string()),
                last_activity: 200,
                unread_count: 0,
                is_archived: false,
                is_muted: false,
            },
        ];

        state.update_conversations(conversations);
        assert_eq!(state.conversations.len(), 2);

        // Test username search
        state.toggle_search();
        state.set_search_query("ali".to_string());
        assert_eq!(state.conversations.len(), 1);
        assert_eq!(state.conversations[0].other_username, "alice");

        // Test message content search
        state.set_search_query("world".to_string());
        assert_eq!(state.conversations.len(), 1);
        assert_eq!(state.conversations[0].other_username, "alice");
    }

    #[test]
    fn test_navigation() {
        let mut state = NavigationState::new();

        let conversations = vec![
            ConversationSummary {
                id: ConversationId::from("conv1"),
                other_user_id: uuid::Uuid::new_v4(),
                other_username: "user1".to_string(),
                last_message: None,
                last_activity: 100,
                unread_count: 0,
                is_archived: false,
                is_muted: false,
            },
            ConversationSummary {
                id: ConversationId::from("conv2"),
                other_user_id: uuid::Uuid::new_v4(),
                other_username: "user2".to_string(),
                last_message: None,
                last_activity: 200,
                unread_count: 0,
                is_archived: false,
                is_muted: false,
            },
        ];

        state.update_conversations(conversations);

        // Test navigation
        assert_eq!(state.selected_index, Some(0));

        state.select_next();
        assert_eq!(state.selected_index, Some(1));

        // Wrap around
        state.select_next();
        assert_eq!(state.selected_index, Some(0));

        // Go back
        state.select_previous();
        assert_eq!(state.selected_index, Some(1));
    }
}
