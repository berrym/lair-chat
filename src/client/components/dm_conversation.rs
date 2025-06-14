//! DM conversation interface UI component for Lair-Chat
//! Displays direct message conversations with real-time features, message editing, and file attachments.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Frame,
};

use tokio::sync::mpsc;

use crate::chat::{
    ConversationId, DirectMessage, MessageDeliveryStatus, MessageId, UserId, UserPresence,
};

/// Events that can be emitted by the DM conversation interface
#[derive(Debug, Clone)]
pub enum ConversationEvent {
    /// Send a new message
    SendMessage(String),
    /// Edit an existing message
    EditMessage(MessageId, String),
    /// Delete a message
    DeleteMessage(MessageId),
    /// Mark messages as read
    MarkAsRead(Option<MessageId>),
    /// Start typing indicator
    StartTyping,
    /// Stop typing indicator
    StopTyping,
    /// Search within conversation
    SearchConversation(String),
    /// Request to load more messages
    LoadMoreMessages,
    /// Request to send file attachment
    SendFile(String),
    /// Conversation closed
    ConversationClosed,
    /// Focus input field
    FocusInput,
}

/// Message display mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageDisplayMode {
    /// Normal message display
    Normal,
    /// Compact display (less spacing)
    Compact,
    /// Detailed display (show timestamps, status)
    Detailed,
}

/// DM conversation state
#[derive(Debug, Clone)]
pub struct ConversationState {
    /// Conversation ID
    pub conversation_id: ConversationId,
    /// Messages in the conversation
    pub messages: Vec<DirectMessage>,
    /// Current user ID
    pub current_user_id: Option<UserId>,
    /// Other participant in the conversation
    pub other_user: Option<UserPresence>,
    /// Message being composed
    pub input_message: String,
    /// Whether input field is focused
    pub input_focused: bool,
    /// Currently selected message (for editing)
    pub selected_message_index: Option<usize>,
    /// Message being edited
    pub editing_message: Option<(MessageId, String)>,
    /// Scroll position
    pub scroll_position: usize,
    /// Maximum scroll position
    pub max_scroll: usize,
    /// List state for message selection
    pub list_state: ListState,
    /// Search query
    pub search_query: String,
    /// Search mode active
    pub search_mode: bool,
    /// Display mode
    pub display_mode: MessageDisplayMode,
    /// Show typing indicators
    pub show_typing_indicators: bool,
    /// Typing users
    pub typing_users: Vec<UserId>,
    /// Unread message count
    pub unread_count: u32,
    /// Whether conversation is muted
    pub is_muted: bool,
    /// Last activity timestamp
    pub last_activity: Option<u64>,
    /// Message display limit (for pagination)
    pub message_limit: usize,
    /// Whether we can load more messages
    pub can_load_more: bool,
}

impl ConversationState {
    /// Create new conversation state
    pub fn new(conversation_id: ConversationId) -> Self {
        Self {
            conversation_id,
            messages: Vec::new(),
            current_user_id: None,
            other_user: None,
            input_message: String::new(),
            input_focused: true,
            selected_message_index: None,
            editing_message: None,
            scroll_position: 0,
            max_scroll: 0,
            list_state: ListState::default(),
            search_query: String::new(),
            search_mode: false,
            display_mode: MessageDisplayMode::Normal,
            show_typing_indicators: true,
            typing_users: Vec::new(),
            unread_count: 0,
            is_muted: false,
            last_activity: None,
            message_limit: 50,
            can_load_more: false,
        }
    }

    /// Set current user
    pub fn set_current_user(&mut self, user_id: UserId) {
        self.current_user_id = Some(user_id);
    }

    /// Set other participant
    pub fn set_other_user(&mut self, user: UserPresence) {
        self.other_user = Some(user);
    }

    /// Update messages
    pub fn update_messages(&mut self, messages: Vec<DirectMessage>) {
        self.messages = messages;
        self.update_scroll_bounds();

        // Auto-scroll to bottom for new messages if we were at the bottom
        if self.scroll_position >= self.max_scroll.saturating_sub(5) {
            self.scroll_to_bottom();
        }

        self.update_last_activity();
    }

    /// Add a new message
    pub fn add_message(&mut self, message: DirectMessage) {
        let was_at_bottom = self.is_at_bottom();
        self.messages.push(message);
        self.update_scroll_bounds();

        // Auto-scroll to bottom for new messages if we were at the bottom
        if was_at_bottom {
            self.scroll_to_bottom();
        }

        self.update_last_activity();
    }

    /// Update last activity timestamp
    fn update_last_activity(&mut self) {
        if let Some(last_message) = self.messages.last() {
            self.last_activity = Some(last_message.created_at);
        }
    }

    /// Check if scrolled to bottom
    fn is_at_bottom(&self) -> bool {
        self.scroll_position >= self.max_scroll.saturating_sub(2)
    }

    /// Update scroll bounds based on message count
    fn update_scroll_bounds(&mut self) {
        self.max_scroll = self.messages.len().saturating_sub(1);
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_position = self.max_scroll;
    }

    /// Scroll up
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_position = self.scroll_position.saturating_sub(lines);
    }

    /// Scroll down
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_position = (self.scroll_position + lines).min(self.max_scroll);
    }

    /// Get visible messages based on scroll position
    pub fn visible_messages(&self, display_height: usize) -> &[DirectMessage] {
        let start = self.scroll_position;
        let end = (start + display_height).min(self.messages.len());
        &self.messages[start..end]
    }

    /// Start editing a message
    pub fn start_editing(&mut self, message_id: MessageId) {
        if let Some(message) = self.messages.iter().find(|m| m.id == message_id) {
            // Only allow editing own messages
            if Some(message.sender_id) == self.current_user_id && !message.is_deleted() {
                self.editing_message = Some((message_id, message.content.clone()));
                self.input_focused = false;
            }
        }
    }

    /// Cancel editing
    pub fn cancel_editing(&mut self) {
        self.editing_message = None;
        self.input_focused = true;
    }

    /// Get message being edited
    pub fn editing_content(&self) -> Option<&str> {
        self.editing_message
            .as_ref()
            .map(|(_, content)| content.as_str())
    }

    /// Update editing content
    pub fn update_editing_content(&mut self, content: String) {
        if let Some((message_id, _)) = self.editing_message {
            self.editing_message = Some((message_id, content));
        }
    }

    /// Toggle search mode
    pub fn toggle_search_mode(&mut self) {
        self.search_mode = !self.search_mode;
        if !self.search_mode {
            self.search_query.clear();
        }
    }

    /// Set search query
    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
    }

    /// Get filtered messages based on search
    pub fn filtered_messages(&self) -> Vec<&DirectMessage> {
        if self.search_query.is_empty() {
            self.messages.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.messages
                .iter()
                .filter(|msg| !msg.is_deleted() && msg.content.to_lowercase().contains(&query))
                .collect()
        }
    }

    /// Add typing user
    pub fn add_typing_user(&mut self, user_id: UserId) {
        if !self.typing_users.contains(&user_id) {
            self.typing_users.push(user_id);
        }
    }

    /// Remove typing user
    pub fn remove_typing_user(&mut self, user_id: UserId) {
        self.typing_users.retain(|&id| id != user_id);
    }

    /// Get conversation title
    pub fn title(&self) -> String {
        if let Some(user) = &self.other_user {
            format!("DM with {}", user.display_name())
        } else {
            "Direct Message".to_string()
        }
    }

    /// Check if user can edit message
    pub fn can_edit_message(&self, message: &DirectMessage) -> bool {
        Some(message.sender_id) == self.current_user_id
            && !message.is_deleted()
            && message.age_seconds() < 300 // 5 minutes
    }

    /// Check if user can delete message
    pub fn can_delete_message(&self, message: &DirectMessage) -> bool {
        Some(message.sender_id) == self.current_user_id && !message.is_deleted()
    }
}

/// DM conversation panel widget
pub struct ConversationPanel {
    /// Panel state
    state: ConversationState,
    /// Event sender
    event_sender: Option<mpsc::UnboundedSender<ConversationEvent>>,
    /// Show message timestamps
    show_timestamps: bool,
    /// Show delivery status
    show_delivery_status: bool,
    /// Show avatars (when available)
    show_avatars: bool,
    /// Max message display width
    max_message_width: usize,
}

impl ConversationPanel {
    /// Create new conversation panel
    pub fn new(conversation_id: ConversationId) -> Self {
        Self {
            state: ConversationState::new(conversation_id),
            event_sender: None,
            show_timestamps: true,
            show_delivery_status: true,
            show_avatars: false,
            max_message_width: 80,
        }
    }

    /// Create conversation panel with event sender
    pub fn with_event_sender(
        conversation_id: ConversationId,
        event_sender: mpsc::UnboundedSender<ConversationEvent>,
    ) -> Self {
        Self {
            state: ConversationState::new(conversation_id),
            event_sender: Some(event_sender),
            show_timestamps: true,
            show_delivery_status: true,
            show_avatars: false,
            max_message_width: 80,
        }
    }

    /// Get mutable reference to state
    pub fn state_mut(&mut self) -> &mut ConversationState {
        &mut self.state
    }

    /// Get reference to state
    pub fn state(&self) -> &ConversationState {
        &self.state
    }

    /// Send event if sender is available
    fn send_event(&self, event: ConversationEvent) {
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event);
        }
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, event: KeyEvent) -> bool {
        match event.code {
            KeyCode::Esc => {
                if self.state.editing_message.is_some() {
                    self.state.cancel_editing();
                } else if self.state.search_mode {
                    self.state.toggle_search_mode();
                } else {
                    self.send_event(ConversationEvent::ConversationClosed);
                }
                true
            }
            KeyCode::Enter => {
                if let Some((message_id, content)) = self.state.editing_message.clone() {
                    self.send_event(ConversationEvent::EditMessage(message_id, content));
                    self.state.cancel_editing();
                } else if !self.state.input_message.trim().is_empty() {
                    let message = self.state.input_message.clone();
                    self.state.input_message.clear();
                    self.send_event(ConversationEvent::SendMessage(message));
                    self.send_event(ConversationEvent::StopTyping);
                }
                true
            }
            KeyCode::Up if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.scroll_up(1);
                true
            }
            KeyCode::Down if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.scroll_down(1);
                true
            }
            KeyCode::PageUp => {
                self.state.scroll_up(10);
                if self.state.scroll_position == 0 && self.state.can_load_more {
                    self.send_event(ConversationEvent::LoadMoreMessages);
                }
                true
            }
            KeyCode::PageDown => {
                self.state.scroll_down(10);
                true
            }
            KeyCode::Home if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.scroll_position = 0;
                if self.state.can_load_more {
                    self.send_event(ConversationEvent::LoadMoreMessages);
                }
                true
            }
            KeyCode::End if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.scroll_to_bottom();
                true
            }
            KeyCode::Char('/') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.toggle_search_mode();
                true
            }
            KeyCode::Char('r') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.send_event(ConversationEvent::MarkAsRead(None));
                true
            }
            KeyCode::Char('f') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                // File attachment (would open file dialog in real implementation)
                self.send_event(ConversationEvent::SendFile("".to_string()));
                true
            }
            KeyCode::Backspace => {
                if self.state.editing_message.is_some() {
                    if let Some((message_id, mut content)) = self.state.editing_message.clone() {
                        content.pop();
                        self.state.editing_message = Some((message_id, content));
                    }
                } else if self.state.search_mode {
                    self.state.search_query.pop();
                } else {
                    self.state.input_message.pop();
                    if !self.state.input_message.is_empty() {
                        self.send_event(ConversationEvent::StartTyping);
                    } else {
                        self.send_event(ConversationEvent::StopTyping);
                    }
                }
                true
            }
            KeyCode::Char(c) => {
                if self.state.editing_message.is_some() {
                    if let Some((message_id, mut content)) = self.state.editing_message.clone() {
                        content.push(c);
                        self.state.editing_message = Some((message_id, content));
                    }
                } else if self.state.search_mode {
                    self.state.search_query.push(c);
                    self.send_event(ConversationEvent::SearchConversation(
                        self.state.search_query.clone(),
                    ));
                } else {
                    self.state.input_message.push(c);
                    self.send_event(ConversationEvent::StartTyping);
                }
                true
            }
            _ => false,
        }
    }

    /// Render the conversation panel
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(5),    // Messages
                Constraint::Length(3), // Input
                Constraint::Length(2), // Status bar
            ])
            .split(area);

        // Render header
        self.render_header(f, chunks[0]);

        // Render messages
        self.render_messages(f, chunks[1]);

        // Render input
        self.render_input(f, chunks[2]);

        // Render status bar
        self.render_status_bar(f, chunks[3]);
    }

    /// Render conversation header
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let title = if self.state.search_mode {
            format!("Search: {}", self.state.search_query)
        } else {
            self.state.title()
        };

        let mut title_spans = vec![Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )];

        // Add unread count if any
        if self.state.unread_count > 0 {
            title_spans.push(Span::styled(
                format!(" ({})", self.state.unread_count),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        }

        // Add muted indicator
        if self.state.is_muted {
            title_spans.push(Span::styled(" 🔇", Style::default().fg(Color::Yellow)));
        }

        // Add typing indicators
        if self.state.show_typing_indicators && !self.state.typing_users.is_empty() {
            title_spans.push(Span::styled(
                " (typing...)",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::ITALIC),
            ));
        }

        let header_block = Block::default()
            .title_top(Line::from(title_spans).alignment(Alignment::Center))
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Cyan));

        f.render_widget(header_block, area);
    }

    /// Render messages area
    fn render_messages(&self, f: &mut Frame, area: Rect) {
        let messages_area = area.inner(Margin {
            vertical: 0,
            horizontal: 1,
        });

        if self.state.messages.is_empty() {
            let empty_text = "No messages yet. Start typing to send a message!";
            let empty_widget = Paragraph::new(empty_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

            f.render_widget(empty_widget, messages_area);
            return;
        }

        let display_height = messages_area.height as usize;
        let messages_to_show = if self.state.search_mode {
            self.state.filtered_messages()
        } else {
            self.state.visible_messages(display_height).iter().collect()
        };

        // Create list items
        let items: Vec<ListItem> = messages_to_show
            .iter()
            .map(|message| self.create_message_item(message, messages_area.width))
            .collect();

        let messages_list = List::new(items);

        f.render_widget(messages_list, messages_area);

        // Render scrollbar if needed
        if self.state.messages.len() > display_height {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let mut scrollbar_state =
                ScrollbarState::new(self.state.max_scroll).position(self.state.scroll_position);

            f.render_stateful_widget(
                scrollbar,
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }
    }

    /// Create a list item for a message
    fn create_message_item(&self, message: &DirectMessage, width: u16) -> ListItem {
        let is_own_message = Some(message.sender_id) == self.state.current_user_id;

        let mut lines = Vec::new();

        // Message header (username, timestamp, status)
        let mut header_spans = Vec::new();

        // Username
        let username = if is_own_message {
            "You".to_string()
        } else if let Some(user) = &self.state.other_user {
            user.display_name().to_string()
        } else {
            format!("User {}", message.sender_id)
        };

        header_spans.push(Span::styled(
            username,
            Style::default()
                .fg(if is_own_message {
                    Color::Cyan
                } else {
                    Color::Green
                })
                .add_modifier(Modifier::BOLD),
        ));

        // Timestamp
        if self.show_timestamps {
            header_spans.push(Span::styled(
                format!(" at {}", message.human_age()),
                Style::default().fg(Color::DarkGray),
            ));
        }

        // Delivery status for own messages
        if is_own_message && self.show_delivery_status {
            let (status_symbol, status_color) = match message.delivery_status {
                MessageDeliveryStatus::Sending => ("⏳", Color::Yellow),
                MessageDeliveryStatus::Sent => ("✓", Color::Gray),
                MessageDeliveryStatus::Delivered => ("✓✓", Color::Blue),
                MessageDeliveryStatus::Read => ("✓✓", Color::Green),
                MessageDeliveryStatus::Failed(_) => ("✗", Color::Red),
                MessageDeliveryStatus::Edited => ("✏", Color::Yellow),
                MessageDeliveryStatus::Deleted => ("🗑", Color::Red),
            };

            header_spans.push(Span::styled(
                format!(" {}", status_symbol),
                Style::default().fg(status_color),
            ));
        }

        lines.push(Line::from(header_spans));

        // Message content
        let content = if message.is_deleted() {
            Span::styled(
                "This message was deleted",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )
        } else {
            let content_text = if message.content.len() > self.max_message_width {
                format!("{}...", &message.content[..self.max_message_width])
            } else {
                message.content.clone()
            };

            Span::styled(
                format!("  {}", content_text),
                Style::default().fg(Color::White),
            )
        };

        lines.push(Line::from(vec![content]));

        // File attachments
        if !message.attachments.is_empty() {
            for attachment in &message.attachments {
                let attachment_line = Line::from(vec![Span::styled(
                    format!("  📎 {} ({})", attachment.filename, attachment.human_size()),
                    Style::default().fg(Color::Blue),
                )]);
                lines.push(attachment_line);
            }
        }

        // Edited indicator
        if message.is_edited() {
            lines.push(Line::from(vec![Span::styled(
                "  (edited)",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )]));
        }

        // Add spacing between messages
        lines.push(Line::from(""));

        ListItem::new(lines)
    }

    /// Render input area
    fn render_input(&self, f: &mut Frame, area: Rect) {
        let input_text = if let Some((_, editing_content)) = &self.state.editing_message {
            editing_content.as_str()
        } else {
            self.state.input_message.as_str()
        };

        let input_style = if self.state.input_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };

        let title = if self.state.editing_message.is_some() {
            "Editing message (Enter: save, Esc: cancel)"
        } else {
            "Type your message (Enter: send, Ctrl+F: attach file)"
        };

        let input_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(input_style);

        let input_widget = Paragraph::new(input_text)
            .block(input_block)
            .style(input_style)
            .wrap(Wrap { trim: false });

        f.render_widget(input_widget, area);

        // Show cursor if input is focused
        if self.state.input_focused || self.state.editing_message.is_some() {
            let cursor_x = area.x + input_text.len() as u16 + 1;
            let cursor_y = area.y + 1;
            if cursor_x < area.x + area.width - 1 {
                f.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    /// Render status bar
    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let mut status_spans = Vec::new();

        // Message count
        let message_count = if self.state.search_mode {
            format!("{} found", self.state.filtered_messages().len())
        } else {
            format!("{} messages", self.state.messages.len())
        };

        status_spans.push(Span::styled(
            message_count,
            Style::default().fg(Color::Cyan),
        ));

        // Scroll position indicator
        if self.state.messages.len() > 10 {
            let scroll_percent = if self.state.max_scroll > 0 {
                (self.state.scroll_position * 100) / self.state.max_scroll
            } else {
                100
            };

            status_spans.push(Span::styled(
                format!(" | {}%", scroll_percent),
                Style::default().fg(Color::Blue),
            ));
        }

        // Load more indicator
        if self.state.can_load_more {
            status_spans.push(Span::styled(
                " | PageUp: load more",
                Style::default().fg(Color::Green),
            ));
        }

        // Help text
        if !self.state.search_mode {
            status_spans.push(Span::styled(
                " | Ctrl+/: search | Ctrl+R: mark read | Esc: close",
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            status_spans.push(Span::styled(
                " | Type to search | Esc: exit search",
                Style::default().fg(Color::DarkGray),
            ));
        }

        let status_widget = Paragraph::new(Line::from(status_spans))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        f.render_widget(status_widget, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::MessageType;

    #[test]
    fn test_conversation_state_creation() {
        let conv_id = ConversationId::from("test".to_string());
        let state = ConversationState::new(conv_id.clone());

        assert_eq!(state.conversation_id, conv_id);
        assert!(state.messages.is_empty());
        assert!(state.input_focused);
        assert!(!state.search_mode);
    }

    #[test]
    fn test_message_addition() {
        let conv_id = ConversationId::from("test".to_string());
        let mut state = ConversationState::new(conv_id);

        let message = DirectMessage::new_text(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            "Test message".to_string(),
        );

        state.add_message(message);
        assert_eq!(state.messages.len(), 1);
        assert!(state.last_activity.is_some());
    }

    #[test]
    fn test_scrolling() {
        let conv_id = ConversationId::from("test".to_string());
        let mut state = ConversationState::new(conv_id);

        // Add many messages
        for i in 0..20 {
            let message = DirectMessage::new_text(
                uuid::Uuid::new_v4(),
                uuid::Uuid::new_v4(),
                format!("Message {}", i),
            );
            state.add_message(message);
        }

        // Test scrolling
        assert_eq!(state.scroll_position, state.max_scroll);

        state.scroll_up(5);
        assert_eq!(state.scroll_position, state.max_scroll - 5);

        state.scroll_down(3);
        assert_eq!(state.scroll_position, state.max_scroll - 2);

        state.scroll_to_bottom();
        assert_eq!(state.scroll_position, state.max_scroll);
    }

    #[test]
    fn test_editing() {
        let conv_id = ConversationId::from("test".to_string());
        let mut state = ConversationState::new(conv_id);
        let user_id = uuid::Uuid::new_v4();
        state.set_current_user(user_id);

        let message = DirectMessage::new_text(
            user_id,
            uuid::Uuid::new_v4(),
            "Original message".to_string(),
        );
        let message_id = message.id;
        state.add_message(message);

        // Start editing
        state.start_editing(message_id);
        assert!(state.editing_message.is_some());
        assert!(!state.input_focused);

        // Update content
        state.update_editing_content("Edited content".to_string());
        assert_eq!(state.editing_content(), Some("Edited content"));

        // Cancel editing
        state.cancel_editing();
        assert!(state.editing_message.is_none());
        assert!(state.input_focused);
    }
}
