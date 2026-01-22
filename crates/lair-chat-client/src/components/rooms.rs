//! Room list screen component.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

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

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent, rooms: &[RoomListItem]) -> Option<Action> {
        if self.creating {
            return self.handle_create_key(key);
        }

        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Esc | KeyCode::Char('b') => Some(Action::BackToChat),
            KeyCode::Char('j') | KeyCode::Down => {
                self.next(rooms.len());
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.prev(rooms.len());
                None
            }
            KeyCode::Enter => {
                if let Some(idx) = self.state.selected() {
                    if let Some(item) = rooms.get(idx) {
                        return Some(Action::JoinRoom(item.room.id));
                    }
                }
                None
            }
            KeyCode::Char('c') | KeyCode::Char('n') => {
                self.creating = true;
                self.new_room_name.clear();
                None
            }
            _ => None,
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
            .title(" Rooms ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

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

        // Help line
        let help_spans = vec![Span::styled(
            " j/k:navigate Enter:join c:create b/Esc:back q:quit ",
            Style::default().fg(Color::DarkGray),
        )];
        let help_line = Line::from(help_spans);
        let help_para = Paragraph::new(help_line);
        frame.render_widget(help_para, chunks[2]);
    }
}
