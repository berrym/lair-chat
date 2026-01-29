//! Command palette component with fuzzy search.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::Action;

/// A command that can be executed from the palette.
#[derive(Debug, Clone)]
pub struct PaletteCommand {
    /// Unique identifier for the command.
    #[allow(dead_code)]
    pub id: &'static str,
    /// Display name shown in the palette.
    pub name: &'static str,
    /// Description/hint shown next to the name.
    pub description: &'static str,
    /// The action to execute.
    pub action: Action,
}

impl PaletteCommand {
    /// Create a new palette command.
    pub const fn new(
        id: &'static str,
        name: &'static str,
        description: &'static str,
        action: Action,
    ) -> Self {
        Self {
            id,
            name,
            description,
            action,
        }
    }
}

/// Command palette state.
pub struct CommandPalette {
    /// Whether the palette is visible.
    pub visible: bool,
    /// Current search input.
    pub input: String,
    /// All available commands.
    commands: Vec<PaletteCommand>,
    /// Filtered command indices with match scores.
    filtered: Vec<(usize, i64)>,
    /// Selection state for the filtered list.
    list_state: ListState,
    /// Fuzzy matcher.
    matcher: SkimMatcherV2,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandPalette {
    /// Create a new command palette with default commands.
    pub fn new() -> Self {
        let commands = vec![
            PaletteCommand::new("rooms", "Show Rooms", "Open room list", Action::ShowRooms),
            PaletteCommand::new(
                "help",
                "Show Help",
                "Display help information",
                Action::ShowHelp,
            ),
            PaletteCommand::new(
                "reconnect",
                "Reconnect",
                "Reconnect to server",
                Action::Reconnect,
            ),
            PaletteCommand::new(
                "copy",
                "Copy Last Message",
                "Copy last message to clipboard",
                Action::CopyLastMessage,
            ),
            PaletteCommand::new("quit", "Quit", "Exit the application", Action::Quit),
        ];

        let mut palette = Self {
            visible: false,
            input: String::new(),
            commands,
            filtered: Vec::new(),
            list_state: ListState::default(),
            matcher: SkimMatcherV2::default(),
        };

        // Initialize with all commands
        palette.update_filter();
        palette
    }

    /// Toggle the palette visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.input.clear();
            self.update_filter();
        }
    }

    /// Open the palette.
    #[allow(dead_code)]
    pub fn open(&mut self) {
        self.visible = true;
        self.input.clear();
        self.update_filter();
    }

    /// Close the palette.
    pub fn close(&mut self) {
        self.visible = false;
        self.input.clear();
    }

    /// Update the filtered list based on current input.
    fn update_filter(&mut self) {
        self.filtered.clear();

        if self.input.is_empty() {
            // Show all commands when no input
            for (idx, _) in self.commands.iter().enumerate() {
                self.filtered.push((idx, 0));
            }
        } else {
            // Fuzzy match against command names and descriptions
            for (idx, cmd) in self.commands.iter().enumerate() {
                let search_text = format!("{} {}", cmd.name, cmd.description);
                if let Some(score) = self.matcher.fuzzy_match(&search_text, &self.input) {
                    self.filtered.push((idx, score));
                }
            }
            // Sort by score (highest first)
            self.filtered.sort_by(|a, b| b.1.cmp(&a.1));
        }

        // Select first item if available
        if !self.filtered.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    /// Handle a key event. Returns Some(Action) if a command was executed.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Handle Ctrl modifiers
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                // Ctrl+P - close or move up
                KeyCode::Char('p') => {
                    self.close();
                    return None;
                }
                // Ctrl+N - move down
                KeyCode::Char('n') => {
                    self.select_next();
                    return None;
                }
                // Ctrl+J - move down (vim style)
                KeyCode::Char('j') => {
                    self.select_next();
                    return None;
                }
                // Ctrl+K - move up (vim style)
                KeyCode::Char('k') => {
                    self.select_prev();
                    return None;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Esc => {
                self.close();
                None
            }
            KeyCode::Enter => {
                // Execute selected command
                if let Some(selected) = self.list_state.selected() {
                    if let Some(&(cmd_idx, _)) = self.filtered.get(selected) {
                        let action = self.commands[cmd_idx].action.clone();
                        self.close();
                        return Some(action);
                    }
                }
                None
            }
            KeyCode::Up => {
                self.select_prev();
                None
            }
            KeyCode::Down => {
                self.select_next();
                None
            }
            KeyCode::Backspace => {
                self.input.pop();
                self.update_filter();
                None
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                self.update_filter();
                None
            }
            _ => None,
        }
    }

    /// Select the previous item.
    fn select_prev(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let prev = if current == 0 {
            self.filtered.len() - 1
        } else {
            current - 1
        };
        self.list_state.select(Some(prev));
    }

    /// Select the next item.
    fn select_next(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let next = (current + 1) % self.filtered.len();
        self.list_state.select(Some(next));
    }

    /// Render the command palette as an overlay.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate centered popup area (60% width, up to 15 items tall)
        let popup_width = (area.width * 60 / 100).clamp(40, 80);
        let popup_height = (self.filtered.len() as u16 + 4).clamp(6, 17);

        let popup_area = centered_rect(popup_width, popup_height, area);

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Main block
        let block = Block::default()
            .title(" Command Palette ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        frame.render_widget(block, popup_area);

        // Inner area
        let inner = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width.saturating_sub(2),
            height: popup_area.height.saturating_sub(2),
        };

        // Split into input and list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(inner);

        // Input line with prompt
        let input_line = Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Yellow)),
            Span::raw(&self.input),
            Span::styled(
                "_",
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ]);
        let input_para = Paragraph::new(input_line);
        frame.render_widget(input_para, chunks[0]);

        // Separator
        let sep = Paragraph::new("─".repeat(chunks[1].width as usize))
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(sep, chunks[1]);

        // Command list
        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .map(|(i, &(cmd_idx, _score))| {
                let cmd = &self.commands[cmd_idx];
                let is_selected = self.list_state.selected() == Some(i);

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let line = Line::from(vec![
                    Span::styled(cmd.name, style),
                    Span::styled(
                        format!("  {}", cmd.description),
                        if is_selected {
                            Style::default().fg(Color::DarkGray).bg(Color::Cyan)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items);
        frame.render_stateful_widget(list, chunks[2], &mut self.list_state);
    }
}

/// Helper function to create a centered rect.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // ========================================================================
    // Palette State Tests
    // ========================================================================

    #[test]
    fn test_command_palette_new() {
        let palette = CommandPalette::new();
        assert!(!palette.visible);
        assert!(palette.input.is_empty());
        assert!(!palette.filtered.is_empty()); // Should have default commands
    }

    #[test]
    fn test_command_palette_default() {
        let palette = CommandPalette::default();
        assert!(!palette.visible);
    }

    #[test]
    fn test_palette_toggle() {
        let mut palette = CommandPalette::new();
        assert!(!palette.visible);

        palette.toggle();
        assert!(palette.visible);
        assert!(palette.input.is_empty());

        palette.toggle();
        assert!(!palette.visible);
    }

    #[test]
    fn test_palette_open() {
        let mut palette = CommandPalette::new();
        palette.input = "test".to_string();

        palette.open();

        assert!(palette.visible);
        assert!(palette.input.is_empty()); // Should be cleared
    }

    #[test]
    fn test_palette_close() {
        let mut palette = CommandPalette::new();
        palette.visible = true;
        palette.input = "test".to_string();

        palette.close();

        assert!(!palette.visible);
        assert!(palette.input.is_empty());
    }

    // ========================================================================
    // Filtering Tests
    // ========================================================================

    #[test]
    fn test_filter_empty_input_shows_all() {
        let mut palette = CommandPalette::new();
        palette.input.clear();
        palette.update_filter();

        // Should show all default commands (5 commands)
        assert_eq!(palette.filtered.len(), 5);
    }

    #[test]
    fn test_filter_narrows_results() {
        let mut palette = CommandPalette::new();
        palette.input = "quit".to_string();
        palette.update_filter();

        // Should find at least the "Quit" command
        assert!(palette.filtered.iter().any(|(idx, _)| {
            palette.commands[*idx].name.to_lowercase().contains("quit")
        }));
    }

    #[test]
    fn test_filter_no_match() {
        let mut palette = CommandPalette::new();
        palette.input = "zzzzzzzzzzz".to_string();
        palette.update_filter();

        // Should have no matches
        assert!(palette.filtered.is_empty());
    }

    #[test]
    fn test_filter_selects_first_when_has_results() {
        let mut palette = CommandPalette::new();
        palette.input = "room".to_string();
        palette.update_filter();

        if !palette.filtered.is_empty() {
            assert_eq!(palette.list_state.selected(), Some(0));
        }
    }

    #[test]
    fn test_filter_no_selection_when_empty() {
        let mut palette = CommandPalette::new();
        palette.input = "zzzzzzzzzzz".to_string();
        palette.update_filter();

        assert_eq!(palette.list_state.selected(), None);
    }

    // ========================================================================
    // Navigation Tests
    // ========================================================================

    #[test]
    fn test_select_next() {
        let mut palette = CommandPalette::new();
        palette.update_filter();
        palette.list_state.select(Some(0));

        palette.select_next();
        assert_eq!(palette.list_state.selected(), Some(1));
    }

    #[test]
    fn test_select_next_wraps() {
        let mut palette = CommandPalette::new();
        palette.update_filter();
        let last = palette.filtered.len() - 1;
        palette.list_state.select(Some(last));

        palette.select_next();
        assert_eq!(palette.list_state.selected(), Some(0)); // Wraps to start
    }

    #[test]
    fn test_select_prev() {
        let mut palette = CommandPalette::new();
        palette.update_filter();
        palette.list_state.select(Some(2));

        palette.select_prev();
        assert_eq!(palette.list_state.selected(), Some(1));
    }

    #[test]
    fn test_select_prev_wraps() {
        let mut palette = CommandPalette::new();
        palette.update_filter();
        palette.list_state.select(Some(0));

        palette.select_prev();
        let expected = palette.filtered.len() - 1;
        assert_eq!(palette.list_state.selected(), Some(expected)); // Wraps to end
    }

    #[test]
    fn test_select_empty_does_nothing() {
        let mut palette = CommandPalette::new();
        palette.input = "zzzzzzzzzzz".to_string();
        palette.update_filter();
        // filtered is now empty

        palette.select_next();
        palette.select_prev();
        // Should not panic
    }

    // ========================================================================
    // Key Handling Tests
    // ========================================================================

    #[test]
    fn test_key_esc_closes() {
        let mut palette = CommandPalette::new();
        palette.visible = true;

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let result = palette.handle_key(key);

        assert!(result.is_none());
        assert!(!palette.visible);
    }

    #[test]
    fn test_key_enter_executes_command() {
        let mut palette = CommandPalette::new();
        palette.visible = true;
        palette.update_filter();
        palette.list_state.select(Some(0));

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let result = palette.handle_key(key);

        // Should return an action
        assert!(result.is_some());
        assert!(!palette.visible);
    }

    #[test]
    fn test_key_down_moves_selection() {
        let mut palette = CommandPalette::new();
        palette.visible = true;
        palette.update_filter();
        palette.list_state.select(Some(0));

        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let result = palette.handle_key(key);

        assert!(result.is_none());
        assert_eq!(palette.list_state.selected(), Some(1));
    }

    #[test]
    fn test_key_up_moves_selection() {
        let mut palette = CommandPalette::new();
        palette.visible = true;
        palette.update_filter();
        palette.list_state.select(Some(2));

        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let result = palette.handle_key(key);

        assert!(result.is_none());
        assert_eq!(palette.list_state.selected(), Some(1));
    }

    #[test]
    fn test_key_char_updates_input() {
        let mut palette = CommandPalette::new();
        palette.visible = true;

        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let result = palette.handle_key(key);

        assert!(result.is_none());
        assert_eq!(palette.input, "q");
    }

    #[test]
    fn test_key_backspace_removes_char() {
        let mut palette = CommandPalette::new();
        palette.visible = true;
        palette.input = "quit".to_string();

        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        let result = palette.handle_key(key);

        assert!(result.is_none());
        assert_eq!(palette.input, "qui");
    }

    #[test]
    fn test_key_ctrl_p_closes() {
        let mut palette = CommandPalette::new();
        palette.visible = true;

        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let result = palette.handle_key(key);

        assert!(result.is_none());
        assert!(!palette.visible);
    }

    #[test]
    fn test_key_ctrl_n_moves_down() {
        let mut palette = CommandPalette::new();
        palette.visible = true;
        palette.update_filter();
        palette.list_state.select(Some(0));

        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
        let result = palette.handle_key(key);

        assert!(result.is_none());
        assert_eq!(palette.list_state.selected(), Some(1));
    }

    #[test]
    fn test_key_ctrl_j_moves_down() {
        let mut palette = CommandPalette::new();
        palette.visible = true;
        palette.update_filter();
        palette.list_state.select(Some(0));

        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL);
        let result = palette.handle_key(key);

        assert!(result.is_none());
        assert_eq!(palette.list_state.selected(), Some(1));
    }

    #[test]
    fn test_key_ctrl_k_moves_up() {
        let mut palette = CommandPalette::new();
        palette.visible = true;
        palette.update_filter();
        palette.list_state.select(Some(2));

        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL);
        let result = palette.handle_key(key);

        assert!(result.is_none());
        assert_eq!(palette.list_state.selected(), Some(1));
    }

    // ========================================================================
    // PaletteCommand Tests
    // ========================================================================

    #[test]
    fn test_palette_command_new() {
        let cmd = PaletteCommand::new(
            "test",
            "Test Command",
            "A test command",
            Action::Quit,
        );

        assert_eq!(cmd.id, "test");
        assert_eq!(cmd.name, "Test Command");
        assert_eq!(cmd.description, "A test command");
    }

    // ========================================================================
    // Helper Function Tests
    // ========================================================================

    #[test]
    fn test_centered_rect() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(40, 20, area);

        assert_eq!(centered.width, 40);
        assert_eq!(centered.height, 20);
        assert_eq!(centered.x, 30); // (100 - 40) / 2
        assert_eq!(centered.y, 15); // (50 - 20) / 2
    }

    #[test]
    fn test_centered_rect_larger_than_area() {
        let area = Rect::new(0, 0, 30, 20);
        let centered = centered_rect(50, 30, area);

        // Should be clamped to area size
        assert!(centered.width <= area.width);
        assert!(centered.height <= area.height);
    }
}
