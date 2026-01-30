//! Full-screen invitations overlay component.

use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};

use crate::protocol::Invitation;

/// Actions that can be returned from the invitations overlay.
/// The actual invitation ID is resolved via selected_index() in the caller.
#[derive(Debug, Clone)]
pub enum InvitationAction {
    /// Accept the currently selected invitation.
    Accept,
    /// Decline the currently selected invitation.
    Decline,
    /// Close the overlay.
    Close,
    /// Refresh the invitations list.
    Refresh,
}

/// Invitations overlay state.
pub struct InvitationsOverlay {
    /// Whether the overlay is visible.
    pub visible: bool,
    /// Selection state for the list.
    list_state: ListState,
}

impl Default for InvitationsOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl InvitationsOverlay {
    /// Create a new invitations overlay (hidden by default).
    pub fn new() -> Self {
        Self {
            visible: false,
            list_state: ListState::default(),
        }
    }

    /// Show the overlay.
    pub fn show(&mut self, invitation_count: usize) {
        self.visible = true;
        // Select first item if there are invitations
        if invitation_count > 0 {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    /// Hide the overlay.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Handle a key event. Returns Some(action) if an action should be taken.
    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        invitation_count: usize,
    ) -> Option<InvitationAction> {
        if !self.visible {
            return None;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                Some(InvitationAction::Close)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next(invitation_count);
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_prev(invitation_count);
                None
            }
            KeyCode::Enter | KeyCode::Char('a') => {
                // Accept selected invitation (actual ID resolved via selected_index in main.rs)
                if self.list_state.selected().is_some() {
                    Some(InvitationAction::Accept)
                } else {
                    None
                }
            }
            KeyCode::Char('d') | KeyCode::Char('x') => {
                // Decline selected invitation (actual ID resolved via selected_index in main.rs)
                if self.list_state.selected().is_some() {
                    Some(InvitationAction::Decline)
                } else {
                    None
                }
            }
            KeyCode::Char('r') => Some(InvitationAction::Refresh),
            _ => None,
        }
    }

    /// Get the currently selected index.
    pub fn selected_index(&self) -> Option<usize> {
        self.list_state.selected()
    }

    /// Select the next item.
    fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let next = (current + 1).min(len.saturating_sub(1));
        self.list_state.select(Some(next));
    }

    /// Select the previous item.
    fn select_prev(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.list_state.select(Some(prev));
    }

    /// Render the invitations overlay.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, invitations: &[Invitation]) {
        if !self.visible {
            return;
        }

        // Clear the entire screen
        frame.render_widget(Clear, area);

        let count = invitations.len();
        let title = format!(" Invitations ({}) ", count);

        // Main block
        let block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Content area (leave room for footer)
        let content_area = Rect {
            x: inner.x + 1,
            y: inner.y,
            width: inner.width.saturating_sub(3), // Room for scrollbar
            height: inner.height.saturating_sub(2), // Room for footer
        };

        if invitations.is_empty() {
            // Show empty state
            let empty_msg = Paragraph::new("No pending invitations")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            frame.render_widget(empty_msg, content_area);
        } else {
            // Build invitation items
            let items: Vec<ListItem> = invitations
                .iter()
                .enumerate()
                .flat_map(|(idx, inv)| {
                    self.render_invitation(idx, inv, content_area.width as usize)
                })
                .collect();

            let total_lines = items.len();
            let visible_lines = content_area.height as usize;

            // Render the list
            let list = List::new(items).highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_stateful_widget(list, content_area, &mut self.list_state);

            // Render scrollbar if needed
            if total_lines > visible_lines {
                let scrollbar_area = Rect {
                    x: inner.x + inner.width - 1,
                    y: inner.y,
                    width: 1,
                    height: inner.height.saturating_sub(2),
                };

                let position = self.list_state.selected().unwrap_or(0);
                let mut scrollbar_state = ScrollbarState::new(count).position(position);

                frame.render_stateful_widget(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some("▲"))
                        .end_symbol(Some("▼"))
                        .track_symbol(Some("│"))
                        .thumb_symbol("█"),
                    scrollbar_area,
                    &mut scrollbar_state,
                );
            }
        }

        // Footer with hints
        let footer_area = Rect {
            x: inner.x,
            y: inner.y + inner.height - 1,
            width: inner.width,
            height: 1,
        };

        let footer = Paragraph::new(Line::from(vec![
            Span::styled(" j/k", Style::default().fg(Color::Yellow)),
            Span::styled(":nav  ", Style::default().fg(Color::DarkGray)),
            Span::styled("a/Enter", Style::default().fg(Color::Green)),
            Span::styled(":accept  ", Style::default().fg(Color::DarkGray)),
            Span::styled("d/x", Style::default().fg(Color::Red)),
            Span::styled(":decline  ", Style::default().fg(Color::DarkGray)),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::styled(":refresh  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc/q", Style::default().fg(Color::Yellow)),
            Span::styled(":close", Style::default().fg(Color::DarkGray)),
        ]))
        .alignment(Alignment::Center);

        frame.render_widget(footer, footer_area);
    }

    /// Render a single invitation as list items.
    fn render_invitation(
        &self,
        idx: usize,
        inv: &Invitation,
        _width: usize,
    ) -> Vec<ListItem<'static>> {
        let is_selected = self.list_state.selected() == Some(idx);
        let prefix = if is_selected { ">> " } else { "   " };

        // Calculate relative time
        let age = format_relative_time(inv.created_at);

        // Room name line
        let room_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let room_line = Line::from(vec![
            Span::styled(prefix.to_string(), room_style),
            Span::styled(format!("#{}", inv.room_name), room_style),
        ]);

        // From line
        let inviter_display = &inv.inviter_name;
        let from_line = Line::from(vec![
            Span::raw("     "),
            Span::styled("From: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                inviter_display.to_string(),
                Style::default().fg(Color::Green),
            ),
            Span::styled(" · ", Style::default().fg(Color::DarkGray)),
            Span::styled(age, Style::default().fg(Color::DarkGray)),
        ]);

        let mut items = vec![ListItem::new(room_line), ListItem::new(from_line)];

        // Message line (if present)
        if let Some(ref msg) = inv.message {
            let msg_line = Line::from(vec![
                Span::raw("     "),
                Span::styled(
                    format!("\"{}\"", msg),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]);
            items.push(ListItem::new(msg_line));
        }

        // Empty line for spacing
        items.push(ListItem::new(Line::from("")));

        items
    }
}

/// Format a timestamp as relative time (e.g., "2h ago", "1d ago").
fn format_relative_time(timestamp: chrono::DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);

    if duration.num_days() > 0 {
        format!("{}d ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{}h ago", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{}m ago", duration.num_minutes())
    } else {
        "just now".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use crossterm::event::KeyModifiers;

    #[test]
    fn test_invitations_overlay_new() {
        let overlay = InvitationsOverlay::new();
        assert!(!overlay.visible);
        assert!(overlay.list_state.selected().is_none());
    }

    #[test]
    fn test_invitations_overlay_show_hide() {
        let mut overlay = InvitationsOverlay::new();

        overlay.show(3);
        assert!(overlay.visible);
        assert_eq!(overlay.list_state.selected(), Some(0));

        overlay.hide();
        assert!(!overlay.visible);
    }

    #[test]
    fn test_invitations_overlay_show_empty() {
        let mut overlay = InvitationsOverlay::new();

        overlay.show(0);
        assert!(overlay.visible);
        assert!(overlay.list_state.selected().is_none());
    }

    #[test]
    fn test_handle_key_close_esc() {
        let mut overlay = InvitationsOverlay::new();
        overlay.show(1);

        let result = overlay.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1);
        assert!(matches!(result, Some(InvitationAction::Close)));
        assert!(!overlay.visible);
    }

    #[test]
    fn test_handle_key_close_q() {
        let mut overlay = InvitationsOverlay::new();
        overlay.show(1);

        let result = overlay.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE), 1);
        assert!(matches!(result, Some(InvitationAction::Close)));
    }

    #[test]
    fn test_handle_key_navigation() {
        let mut overlay = InvitationsOverlay::new();
        overlay.show(3);
        assert_eq!(overlay.selected_index(), Some(0));

        // Move down
        overlay.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE), 3);
        assert_eq!(overlay.selected_index(), Some(1));

        // Move down again
        overlay.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), 3);
        assert_eq!(overlay.selected_index(), Some(2));

        // Move up
        overlay.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE), 3);
        assert_eq!(overlay.selected_index(), Some(1));

        // Move up
        overlay.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), 3);
        assert_eq!(overlay.selected_index(), Some(0));
    }

    #[test]
    fn test_handle_key_accept() {
        let mut overlay = InvitationsOverlay::new();
        overlay.show(1);

        let result = overlay.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), 1);
        assert!(matches!(result, Some(InvitationAction::Accept)));
    }

    #[test]
    fn test_handle_key_decline() {
        let mut overlay = InvitationsOverlay::new();
        overlay.show(1);

        let result = overlay.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), 1);
        assert!(matches!(result, Some(InvitationAction::Decline)));
    }

    #[test]
    fn test_handle_key_refresh() {
        let mut overlay = InvitationsOverlay::new();
        overlay.show(1);

        let result = overlay.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE), 1);
        assert!(matches!(result, Some(InvitationAction::Refresh)));
    }

    #[test]
    fn test_format_relative_time() {
        let now = Utc::now();

        assert_eq!(format_relative_time(now), "just now");
        assert_eq!(format_relative_time(now - Duration::minutes(5)), "5m ago");
        assert_eq!(format_relative_time(now - Duration::hours(2)), "2h ago");
        assert_eq!(format_relative_time(now - Duration::days(1)), "1d ago");
        assert_eq!(format_relative_time(now - Duration::days(7)), "7d ago");
    }
}
