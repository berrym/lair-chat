//! Room list screen component.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use tracing::debug;

use crate::app::Action;
use crate::protocol::{Room, RoomListItem};

/// Room list screen state.
pub struct RoomsScreen {
    /// List selection state.
    pub state: ListState,
    /// Creating new room.
    pub creating: bool,
    /// New room name input.
    pub new_room_name: String,
}

impl Default for RoomsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl RoomsScreen {
    /// Create a new rooms screen.
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            state,
            creating: false,
            new_room_name: String::new(),
        }
    }

    /// Handle a key event. Returns (action, debug_message).
    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        rooms: &[RoomListItem],
    ) -> (Option<Action>, Option<String>) {
        if self.creating {
            return (self.handle_create_key(key), None);
        }

        match key.code {
            KeyCode::Char('q') => (Some(Action::Quit), None),
            KeyCode::Esc | KeyCode::Char('b') => (Some(Action::BackToChat), None),
            KeyCode::Char('j') | KeyCode::Down => {
                self.next(rooms.len());
                (None, None)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.prev(rooms.len());
                (None, None)
            }
            KeyCode::Enter => {
                debug!(
                    "Enter pressed: selected={:?}, rooms_len={}",
                    self.state.selected(),
                    rooms.len()
                );
                if let Some(idx) = self.state.selected() {
                    if let Some(item) = rooms.get(idx) {
                        if item.is_member {
                            // Already a member - just switch to this room
                            debug!("Switching to room: {} ({})", item.room.name, item.room.id);
                            (Some(Action::SwitchToRoom(item.room.clone())), None)
                        } else {
                            // Not a member - need to join
                            debug!("Joining room: {} ({})", item.room.name, item.room.id);
                            (
                                Some(Action::JoinRoom(item.room.id)),
                                Some(format!("Joining {}", item.room.name)),
                            )
                        }
                    } else {
                        (
                            None,
                            Some(format!("No room at index {} (len={})", idx, rooms.len())),
                        )
                    }
                } else {
                    (None, Some("No room selected".to_string()))
                }
            }
            KeyCode::Char('c') | KeyCode::Char('n') => {
                self.creating = true;
                self.new_room_name.clear();
                (None, None)
            }
            _ => (None, Some(format!("Unhandled key: {:?}", key.code))),
        }
    }

    fn handle_create_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.creating = false;
                self.new_room_name.clear();
                None
            }
            KeyCode::Enter => {
                if !self.new_room_name.is_empty() {
                    let name = std::mem::take(&mut self.new_room_name);
                    self.creating = false;
                    return Some(Action::CreateRoom(name));
                }
                None
            }
            KeyCode::Backspace => {
                self.new_room_name.pop();
                None
            }
            KeyCode::Char(c) => {
                self.new_room_name.push(c);
                None
            }
            _ => None,
        }
    }

    fn next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => (i + 1) % len,
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn prev(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Render the rooms screen.
    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        rooms: &[RoomListItem],
        current_room: Option<&Room>,
        status: Option<&str>,
        error: Option<&str>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),    // Room list
                Constraint::Length(3), // Create room input
                Constraint::Length(1), // Help
            ])
            .split(area);

        // Room list
        let rooms_block = Block::default()
            .title(format!(" Rooms ({}) ", rooms.len()))
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        if rooms.is_empty() {
            // Show empty state message
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No rooms available",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Press 'c' to create the first room!",
                    Style::default().fg(Color::Yellow),
                )),
            ];
            let empty_para = Paragraph::new(empty_text)
                .block(rooms_block)
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(empty_para, chunks[0]);
        } else {
            let items: Vec<ListItem> = rooms
                .iter()
                .map(|item| {
                    let is_current = current_room.map(|c| c.id == item.room.id).unwrap_or(false);
                    let marker = if is_current { "* " } else { "  " };
                    let style = if is_current {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    // Show room name with member count
                    let member_text = if item.member_count == 1 {
                        "1 member".to_string()
                    } else {
                        format!("{} members", item.member_count)
                    };
                    ListItem::new(format!("{}{} ({})", marker, item.room.name, member_text))
                        .style(style)
                })
                .collect();

            let list = List::new(items)
                .block(rooms_block)
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            frame.render_stateful_widget(list, chunks[0], &mut self.state);
        }

        // Create room input
        let create_title = if self.creating {
            " Enter room name (Esc to cancel) "
        } else {
            " Press 'c' to create a new room "
        };
        let create_style = if self.creating {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let create_block = Block::default()
            .title(create_title)
            .borders(Borders::ALL)
            .style(create_style);

        let create_text = if self.creating {
            format!("{}|", self.new_room_name)
        } else {
            String::new()
        };
        let create_para = Paragraph::new(create_text).block(create_block);
        frame.render_widget(create_para, chunks[1]);

        // Help/status line
        let mut help_spans = Vec::new();

        if let Some(err) = error {
            help_spans.push(Span::styled(
                format!(" ERROR: {} ", err),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
            help_spans.push(Span::raw(" | "));
        } else if let Some(stat) = status {
            help_spans.push(Span::styled(
                format!(" {} ", stat),
                Style::default().fg(Color::Yellow),
            ));
            help_spans.push(Span::raw(" | "));
        }

        help_spans.push(Span::styled(
            "j/k:navigate Enter:join c:create b/Esc:back q:quit ",
            Style::default().fg(Color::DarkGray),
        ));
        let help_line = Line::from(help_spans);
        let help_para = Paragraph::new(help_line);
        frame.render_widget(help_para, chunks[2]);
    }
}
