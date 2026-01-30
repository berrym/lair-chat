//! Full-screen room members overlay component.

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

use crate::protocol::RoomMember;

/// Actions that can be returned from the members overlay.
/// The actual user is resolved via selected_index() in the caller.
#[derive(Debug, Clone)]
pub enum MemberAction {
    /// Close the overlay.
    Close,
    /// Start DM with the currently selected user.
    StartDM,
    /// Kick the currently selected member (requires moderator or owner role).
    KickMember,
    /// Promote selected member to moderator (requires owner role).
    PromoteToMod,
    /// Demote selected member to regular member (requires owner role).
    DemoteToMember,
}

/// Members overlay state.
pub struct MembersOverlay {
    /// Whether the overlay is visible.
    pub visible: bool,
    /// Selection state for the list.
    list_state: ListState,
    /// Room name being displayed.
    room_name: String,
}

impl Default for MembersOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl MembersOverlay {
    /// Create a new members overlay (hidden by default).
    pub fn new() -> Self {
        Self {
            visible: false,
            list_state: ListState::default(),
            room_name: String::new(),
        }
    }

    /// Show the overlay.
    pub fn show(&mut self, room_name: &str, member_count: usize) {
        self.visible = true;
        self.room_name = room_name.to_string();
        // Select first member if there are any
        if member_count > 0 {
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
    pub fn handle_key(&mut self, key: KeyEvent, member_count: usize) -> Option<MemberAction> {
        if !self.visible {
            return None;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                Some(MemberAction::Close)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next(member_count);
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_prev(member_count);
                None
            }
            KeyCode::Enter => {
                // Start DM with selected user (actual user resolved via selected_index in main.rs)
                self.list_state.selected().map(|_| MemberAction::StartDM)
            }
            KeyCode::Char('x') => {
                // Kick selected member (requires moderator or owner role)
                self.list_state.selected().map(|_| MemberAction::KickMember)
            }
            KeyCode::Char('p') => {
                // Promote to moderator (requires owner role)
                self.list_state
                    .selected()
                    .map(|_| MemberAction::PromoteToMod)
            }
            KeyCode::Char('d') => {
                // Demote to member (requires owner role)
                self.list_state
                    .selected()
                    .map(|_| MemberAction::DemoteToMember)
            }
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

    /// Render the members overlay.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, members: &[RoomMember]) {
        if !self.visible {
            return;
        }

        // Clear the entire screen
        frame.render_widget(Clear, area);

        let count = members.len();
        let title = format!(" Members of #{} ({}) ", self.room_name, count);

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

        if members.is_empty() {
            // Show empty state
            let empty_msg = Paragraph::new("No members in this room")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            frame.render_widget(empty_msg, content_area);
        } else {
            // Group members by role
            let (owners, moderators, regular_members) = group_by_role(members);

            // Build list items
            let mut items: Vec<ListItem> = Vec::new();
            let mut selectable_indices: Vec<usize> = Vec::new();
            let mut current_idx = 0;

            // Owners section
            if !owners.is_empty() {
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    format!("─── Owners ({}) ───", owners.len()),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )])));

                for member in &owners {
                    let is_selected = self.list_state.selected() == Some(current_idx);
                    items.push(self.render_member(member, is_selected));
                    selectable_indices.push(current_idx);
                    current_idx += 1;
                }
            }

            // Moderators section
            if !moderators.is_empty() {
                items.push(ListItem::new(Line::from("")));
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    format!("─── Moderators ({}) ───", moderators.len()),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )])));

                for member in &moderators {
                    let is_selected = self.list_state.selected() == Some(current_idx);
                    items.push(self.render_member(member, is_selected));
                    selectable_indices.push(current_idx);
                    current_idx += 1;
                }
            }

            // Regular members section
            if !regular_members.is_empty() {
                items.push(ListItem::new(Line::from("")));
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    format!("─── Members ({}) ───", regular_members.len()),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )])));

                for member in &regular_members {
                    let is_selected = self.list_state.selected() == Some(current_idx);
                    items.push(self.render_member(member, is_selected));
                    selectable_indices.push(current_idx);
                    current_idx += 1;
                }
            }

            let total_lines = items.len();
            let visible_lines = content_area.height as usize;

            // Render the list
            let list = List::new(items);
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
            Span::styled(":nav ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::styled(":DM ", Style::default().fg(Color::DarkGray)),
            Span::styled("p", Style::default().fg(Color::Cyan)),
            Span::styled(":promote ", Style::default().fg(Color::DarkGray)),
            Span::styled("d", Style::default().fg(Color::Yellow)),
            Span::styled(":demote ", Style::default().fg(Color::DarkGray)),
            Span::styled("x", Style::default().fg(Color::Red)),
            Span::styled(":kick ", Style::default().fg(Color::DarkGray)),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::styled(":close", Style::default().fg(Color::DarkGray)),
        ]))
        .alignment(Alignment::Center);

        frame.render_widget(footer, footer_area);
    }

    /// Render a single member as a list item.
    fn render_member(&self, member: &RoomMember, is_selected: bool) -> ListItem<'static> {
        let (prefix, role_color) = match member.role.as_str() {
            "owner" => ("*", Color::Yellow),
            "moderator" | "mod" => ("+", Color::Cyan),
            _ => (" ", Color::White),
        };

        // Show online indicator
        let online_indicator = if member.is_online { "●" } else { "○" };
        let online_color = if member.is_online {
            Color::Green
        } else {
            Color::DarkGray
        };

        let base_style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(role_color)
        };

        let selector = if is_selected { ">> " } else { "   " };

        let line = Line::from(vec![
            Span::styled(selector.to_string(), base_style),
            Span::styled(
                online_indicator.to_string(),
                if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(online_color)
                },
            ),
            Span::styled(" ".to_string(), base_style),
            Span::styled(member.username.clone(), base_style),
            Span::styled(
                format!(" {}", prefix),
                if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(role_color).add_modifier(Modifier::BOLD)
                },
            ),
        ]);

        ListItem::new(line)
    }
}

/// Type alias for a list of member references grouped by role.
type MemberGroup<'a> = Vec<&'a RoomMember>;

/// Group members by role: (owners, moderators, regular members).
fn group_by_role(members: &[RoomMember]) -> (MemberGroup<'_>, MemberGroup<'_>, MemberGroup<'_>) {
    let mut owners = Vec::new();
    let mut moderators = Vec::new();
    let mut regular = Vec::new();

    for member in members {
        match member.role.as_str() {
            "owner" => owners.push(member),
            "moderator" | "mod" => moderators.push(member),
            _ => regular.push(member),
        }
    }

    (owners, moderators, regular)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crossterm::event::KeyModifiers;
    use uuid::Uuid;

    fn make_test_member(username: &str, role: &str, is_online: bool) -> RoomMember {
        RoomMember {
            user_id: Uuid::new_v4(),
            username: username.to_string(),
            role: role.to_string(),
            joined_at: Utc::now(),
            is_online,
        }
    }

    #[test]
    fn test_members_overlay_new() {
        let overlay = MembersOverlay::new();
        assert!(!overlay.visible);
        assert!(overlay.list_state.selected().is_none());
    }

    #[test]
    fn test_members_overlay_show_hide() {
        let mut overlay = MembersOverlay::new();

        overlay.show("general", 3);
        assert!(overlay.visible);
        assert_eq!(overlay.room_name, "general");
        assert_eq!(overlay.list_state.selected(), Some(0));

        overlay.hide();
        assert!(!overlay.visible);
    }

    #[test]
    fn test_members_overlay_show_empty() {
        let mut overlay = MembersOverlay::new();

        overlay.show("empty-room", 0);
        assert!(overlay.visible);
        assert!(overlay.list_state.selected().is_none());
    }

    #[test]
    fn test_handle_key_close_esc() {
        let mut overlay = MembersOverlay::new();
        overlay.show("test", 1);

        let result = overlay.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1);
        assert!(matches!(result, Some(MemberAction::Close)));
        assert!(!overlay.visible);
    }

    #[test]
    fn test_handle_key_close_q() {
        let mut overlay = MembersOverlay::new();
        overlay.show("test", 1);

        let result = overlay.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE), 1);
        assert!(matches!(result, Some(MemberAction::Close)));
    }

    #[test]
    fn test_handle_key_navigation() {
        let mut overlay = MembersOverlay::new();
        overlay.show("test", 3);
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
    fn test_handle_key_start_dm() {
        let mut overlay = MembersOverlay::new();
        overlay.show("test", 1);

        let result = overlay.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 1);
        assert!(matches!(result, Some(MemberAction::StartDM)));
    }

    #[test]
    fn test_handle_key_kick() {
        let mut overlay = MembersOverlay::new();
        overlay.show("test", 1);

        let result = overlay.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE), 1);
        assert!(matches!(result, Some(MemberAction::KickMember)));
    }

    #[test]
    fn test_handle_key_promote() {
        let mut overlay = MembersOverlay::new();
        overlay.show("test", 1);

        let result = overlay.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE), 1);
        assert!(matches!(result, Some(MemberAction::PromoteToMod)));
    }

    #[test]
    fn test_handle_key_demote() {
        let mut overlay = MembersOverlay::new();
        overlay.show("test", 1);

        let result = overlay.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), 1);
        assert!(matches!(result, Some(MemberAction::DemoteToMember)));
    }

    #[test]
    fn test_group_by_role() {
        let owner = make_test_member("alice", "owner", true);
        let mod1 = make_test_member("bob", "moderator", false);
        let member1 = make_test_member("charlie", "member", true);
        let member2 = make_test_member("dave", "member", false);

        let members = vec![owner, mod1, member1, member2];
        let (owners, moderators, regular) = group_by_role(&members);

        assert_eq!(owners.len(), 1);
        assert_eq!(moderators.len(), 1);
        assert_eq!(regular.len(), 2);
    }
}
