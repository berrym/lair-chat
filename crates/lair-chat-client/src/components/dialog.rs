//! Custom dialog/popup component.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Result from a dialog interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogResult {
    /// Dialog is still open, no result yet.
    Pending,
    /// User confirmed (Enter on confirm, or submitted input).
    Confirmed(Option<String>),
    /// User cancelled (Esc).
    Cancelled,
}

/// The type of dialog to display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogKind {
    /// Information display with OK button.
    Info,
    /// Yes/No confirmation.
    Confirm,
    /// Text input with submit.
    #[allow(dead_code)]
    Input,
}

/// Button selection for confirm dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmSelection {
    Yes,
    No,
}

impl ConfirmSelection {
    fn toggle(&mut self) {
        *self = match self {
            ConfirmSelection::Yes => ConfirmSelection::No,
            ConfirmSelection::No => ConfirmSelection::Yes,
        };
    }
}

/// Dialog state and content.
pub struct Dialog {
    /// Dialog title.
    pub title: String,
    /// Dialog message/content.
    pub message: String,
    /// Type of dialog.
    pub kind: DialogKind,
    /// Whether the dialog is visible.
    pub visible: bool,
    /// Input buffer (for Input dialogs).
    input: String,
    /// Cursor position in input.
    cursor: usize,
    /// Selected button (for Confirm dialogs).
    selection: ConfirmSelection,
}

impl Default for Dialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Dialog {
    /// Create a new dialog (hidden by default).
    pub fn new() -> Self {
        Self {
            title: String::new(),
            message: String::new(),
            kind: DialogKind::Info,
            visible: false,
            input: String::new(),
            cursor: 0,
            selection: ConfirmSelection::Yes,
        }
    }

    /// Show an info dialog.
    #[allow(dead_code)]
    pub fn show_info(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.title = title.into();
        self.message = message.into();
        self.kind = DialogKind::Info;
        self.visible = true;
    }

    /// Show a confirmation dialog.
    pub fn show_confirm(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.title = title.into();
        self.message = message.into();
        self.kind = DialogKind::Confirm;
        self.selection = ConfirmSelection::Yes;
        self.visible = true;
    }

    /// Show an input dialog.
    #[allow(dead_code)]
    pub fn show_input(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.title = title.into();
        self.message = message.into();
        self.kind = DialogKind::Input;
        self.input.clear();
        self.cursor = 0;
        self.visible = true;
    }

    /// Show an input dialog with a default value.
    #[allow(dead_code)]
    pub fn show_input_with_default(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
        default: impl Into<String>,
    ) {
        self.title = title.into();
        self.message = message.into();
        self.kind = DialogKind::Input;
        self.input = default.into();
        self.cursor = self.input.len();
        self.visible = true;
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.visible = false;
        self.input.clear();
        self.cursor = 0;
    }

    /// Handle a key event. Returns the dialog result.
    pub fn handle_key(&mut self, key: KeyEvent) -> DialogResult {
        if !self.visible {
            return DialogResult::Pending;
        }

        match self.kind {
            DialogKind::Info => self.handle_info_key(key),
            DialogKind::Confirm => self.handle_confirm_key(key),
            DialogKind::Input => self.handle_input_key(key),
        }
    }

    fn handle_info_key(&mut self, key: KeyEvent) -> DialogResult {
        match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
                self.close();
                DialogResult::Confirmed(None)
            }
            _ => DialogResult::Pending,
        }
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) -> DialogResult {
        match key.code {
            KeyCode::Esc => {
                self.close();
                DialogResult::Cancelled
            }
            KeyCode::Enter => {
                self.close();
                match self.selection {
                    ConfirmSelection::Yes => DialogResult::Confirmed(None),
                    ConfirmSelection::No => DialogResult::Cancelled,
                }
            }
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Tab
            | KeyCode::Char('h')
            | KeyCode::Char('l') => {
                self.selection.toggle();
                DialogResult::Pending
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.close();
                DialogResult::Confirmed(None)
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.close();
                DialogResult::Cancelled
            }
            _ => DialogResult::Pending,
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) -> DialogResult {
        // Handle Ctrl modifiers
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('u') => {
                    self.input.drain(..self.cursor);
                    self.cursor = 0;
                    return DialogResult::Pending;
                }
                KeyCode::Char('k') => {
                    self.input.truncate(self.cursor);
                    return DialogResult::Pending;
                }
                KeyCode::Char('a') => {
                    self.cursor = 0;
                    return DialogResult::Pending;
                }
                KeyCode::Char('e') => {
                    self.cursor = self.input.len();
                    return DialogResult::Pending;
                }
                KeyCode::Char('w') => {
                    // Delete word before cursor
                    if self.cursor > 0 {
                        let before = &self.input[..self.cursor];
                        let trimmed = before.trim_end();
                        let new_end = trimmed
                            .rfind(|c: char| c.is_whitespace())
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        self.input.drain(new_end..self.cursor);
                        self.cursor = new_end;
                    }
                    return DialogResult::Pending;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Esc => {
                self.close();
                DialogResult::Cancelled
            }
            KeyCode::Enter => {
                let value = std::mem::take(&mut self.input);
                self.close();
                DialogResult::Confirmed(Some(value))
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                }
                DialogResult::Pending
            }
            KeyCode::Delete => {
                if self.cursor < self.input.len() {
                    self.input.remove(self.cursor);
                }
                DialogResult::Pending
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                DialogResult::Pending
            }
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.input.len());
                DialogResult::Pending
            }
            KeyCode::Home => {
                self.cursor = 0;
                DialogResult::Pending
            }
            KeyCode::End => {
                self.cursor = self.input.len();
                DialogResult::Pending
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor, c);
                self.cursor += 1;
                DialogResult::Pending
            }
            _ => DialogResult::Pending,
        }
    }

    /// Render the dialog as an overlay.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate dialog dimensions
        let width = (area.width * 50 / 100).clamp(30, 60);
        let height = match self.kind {
            DialogKind::Info => 7,
            DialogKind::Confirm => 7,
            DialogKind::Input => 8,
        };

        // Center the dialog
        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;
        let dialog_area = Rect::new(x, y, width, height);

        // Clear the area behind the dialog
        frame.render_widget(Clear, dialog_area);

        // Border color based on dialog type
        let border_color = match self.kind {
            DialogKind::Info => Color::Cyan,
            DialogKind::Confirm => Color::Yellow,
            DialogKind::Input => Color::Green,
        };

        // Main block
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(Color::Black));

        frame.render_widget(block, dialog_area);

        // Inner area
        let inner = Rect {
            x: dialog_area.x + 2,
            y: dialog_area.y + 1,
            width: dialog_area.width.saturating_sub(4),
            height: dialog_area.height.saturating_sub(2),
        };

        // Layout for content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(match self.kind {
                DialogKind::Info => vec![Constraint::Min(1), Constraint::Length(2)],
                DialogKind::Confirm => vec![Constraint::Min(1), Constraint::Length(2)],
                DialogKind::Input => vec![
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(2),
                ],
            })
            .split(inner);

        // Message
        let message = Paragraph::new(self.message.as_str())
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);
        frame.render_widget(message, chunks[0]);

        // Render based on dialog type
        match self.kind {
            DialogKind::Info => {
                let hint = Paragraph::new("[Enter/Esc] OK")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center);
                frame.render_widget(hint, chunks[1]);
            }
            DialogKind::Confirm => {
                let yes_style = if self.selection == ConfirmSelection::Yes {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                };
                let no_style = if self.selection == ConfirmSelection::No {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Red)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Red)
                };

                let buttons = Line::from(vec![
                    Span::raw("    "),
                    Span::styled(" [Y]es ", yes_style),
                    Span::raw("   "),
                    Span::styled(" [N]o ", no_style),
                    Span::raw("    "),
                ]);
                let buttons_para = Paragraph::new(buttons).alignment(Alignment::Center);
                frame.render_widget(buttons_para, chunks[1]);
            }
            DialogKind::Input => {
                // Input field
                let input_display = if self.input.is_empty() {
                    Line::from(vec![
                        Span::styled("> ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            "_",
                            Style::default()
                                .fg(Color::Gray)
                                .add_modifier(Modifier::SLOW_BLINK),
                        ),
                    ])
                } else {
                    let before_cursor = &self.input[..self.cursor];
                    let at_cursor = self.input.chars().nth(self.cursor).unwrap_or(' ');
                    let after_cursor = if self.cursor < self.input.len() {
                        &self.input[self.cursor + 1..]
                    } else {
                        ""
                    };

                    Line::from(vec![
                        Span::styled("> ", Style::default().fg(Color::Yellow)),
                        Span::raw(before_cursor),
                        Span::styled(
                            at_cursor.to_string(),
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::White)
                                .add_modifier(Modifier::SLOW_BLINK),
                        ),
                        Span::raw(after_cursor),
                    ])
                };
                let input_para = Paragraph::new(input_display);
                frame.render_widget(input_para, chunks[1]);

                // Hint
                let hint = Paragraph::new("[Enter] Submit  [Esc] Cancel")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center);
                frame.render_widget(hint, chunks[2]);
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // ========================================================================
    // Dialog State Tests
    // ========================================================================

    #[test]
    fn test_dialog_new() {
        let dialog = Dialog::new();
        assert!(!dialog.visible);
        assert!(dialog.title.is_empty());
        assert!(dialog.message.is_empty());
        assert_eq!(dialog.kind, DialogKind::Info);
    }

    #[test]
    fn test_dialog_default() {
        let dialog = Dialog::default();
        assert!(!dialog.visible);
    }

    #[test]
    fn test_dialog_show_info() {
        let mut dialog = Dialog::new();
        dialog.show_info("Title", "Message");

        assert!(dialog.visible);
        assert_eq!(dialog.title, "Title");
        assert_eq!(dialog.message, "Message");
        assert_eq!(dialog.kind, DialogKind::Info);
    }

    #[test]
    fn test_dialog_show_confirm() {
        let mut dialog = Dialog::new();
        dialog.show_confirm("Confirm", "Are you sure?");

        assert!(dialog.visible);
        assert_eq!(dialog.title, "Confirm");
        assert_eq!(dialog.message, "Are you sure?");
        assert_eq!(dialog.kind, DialogKind::Confirm);
        assert_eq!(dialog.selection, ConfirmSelection::Yes);
    }

    #[test]
    fn test_dialog_show_input() {
        let mut dialog = Dialog::new();
        dialog.show_input("Input", "Enter value:");

        assert!(dialog.visible);
        assert_eq!(dialog.title, "Input");
        assert_eq!(dialog.message, "Enter value:");
        assert_eq!(dialog.kind, DialogKind::Input);
        assert!(dialog.input.is_empty());
        assert_eq!(dialog.cursor, 0);
    }

    #[test]
    fn test_dialog_show_input_with_default() {
        let mut dialog = Dialog::new();
        dialog.show_input_with_default("Input", "Enter value:", "default");

        assert!(dialog.visible);
        assert_eq!(dialog.input, "default");
        assert_eq!(dialog.cursor, 7); // At end of "default"
    }

    #[test]
    fn test_dialog_close() {
        let mut dialog = Dialog::new();
        dialog.show_info("Title", "Message");
        dialog.close();

        assert!(!dialog.visible);
        assert!(dialog.input.is_empty());
        assert_eq!(dialog.cursor, 0);
    }

    // ========================================================================
    // ConfirmSelection Tests
    // ========================================================================

    #[test]
    fn test_confirm_selection_toggle() {
        let mut selection = ConfirmSelection::Yes;
        selection.toggle();
        assert_eq!(selection, ConfirmSelection::No);
        selection.toggle();
        assert_eq!(selection, ConfirmSelection::Yes);
    }

    // ========================================================================
    // Info Dialog Key Handling Tests
    // ========================================================================

    #[test]
    fn test_info_dialog_enter_closes() {
        let mut dialog = Dialog::new();
        dialog.show_info("Title", "Message");

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Confirmed(None));
        assert!(!dialog.visible);
    }

    #[test]
    fn test_info_dialog_esc_closes() {
        let mut dialog = Dialog::new();
        dialog.show_info("Title", "Message");

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Confirmed(None));
        assert!(!dialog.visible);
    }

    #[test]
    fn test_info_dialog_space_closes() {
        let mut dialog = Dialog::new();
        dialog.show_info("Title", "Message");

        let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Confirmed(None));
    }

    #[test]
    fn test_info_dialog_other_key_pending() {
        let mut dialog = Dialog::new();
        dialog.show_info("Title", "Message");

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Pending);
        assert!(dialog.visible);
    }

    // ========================================================================
    // Confirm Dialog Key Handling Tests
    // ========================================================================

    #[test]
    fn test_confirm_dialog_enter_yes() {
        let mut dialog = Dialog::new();
        dialog.show_confirm("Confirm", "Sure?");

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        // Default selection is Yes
        assert_eq!(result, DialogResult::Confirmed(None));
    }

    #[test]
    fn test_confirm_dialog_enter_no() {
        let mut dialog = Dialog::new();
        dialog.show_confirm("Confirm", "Sure?");
        dialog.selection = ConfirmSelection::No;

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Cancelled);
    }

    #[test]
    fn test_confirm_dialog_esc_cancels() {
        let mut dialog = Dialog::new();
        dialog.show_confirm("Confirm", "Sure?");

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Cancelled);
    }

    #[test]
    fn test_confirm_dialog_y_key_confirms() {
        let mut dialog = Dialog::new();
        dialog.show_confirm("Confirm", "Sure?");

        let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Confirmed(None));
    }

    #[test]
    fn test_confirm_dialog_n_key_cancels() {
        let mut dialog = Dialog::new();
        dialog.show_confirm("Confirm", "Sure?");

        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Cancelled);
    }

    #[test]
    fn test_confirm_dialog_arrows_toggle() {
        let mut dialog = Dialog::new();
        dialog.show_confirm("Confirm", "Sure?");
        assert_eq!(dialog.selection, ConfirmSelection::Yes);

        let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Pending);
        assert_eq!(dialog.selection, ConfirmSelection::No);

        let key = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        let _ = dialog.handle_key(key);
        assert_eq!(dialog.selection, ConfirmSelection::Yes);
    }

    #[test]
    fn test_confirm_dialog_tab_toggles() {
        let mut dialog = Dialog::new();
        dialog.show_confirm("Confirm", "Sure?");

        let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let _ = dialog.handle_key(key);

        assert_eq!(dialog.selection, ConfirmSelection::No);
    }

    // ========================================================================
    // Input Dialog Key Handling Tests
    // ========================================================================

    #[test]
    fn test_input_dialog_char_input() {
        let mut dialog = Dialog::new();
        dialog.show_input("Input", "Enter:");

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Pending);
        assert_eq!(dialog.input, "a");
        assert_eq!(dialog.cursor, 1);
    }

    #[test]
    fn test_input_dialog_enter_submits() {
        let mut dialog = Dialog::new();
        dialog.show_input("Input", "Enter:");
        dialog.input = "hello".to_string();
        dialog.cursor = 5;

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Confirmed(Some("hello".to_string())));
        assert!(!dialog.visible);
    }

    #[test]
    fn test_input_dialog_esc_cancels() {
        let mut dialog = Dialog::new();
        dialog.show_input("Input", "Enter:");
        dialog.input = "hello".to_string();

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Cancelled);
        assert!(!dialog.visible);
    }

    #[test]
    fn test_input_dialog_backspace() {
        let mut dialog = Dialog::new();
        dialog.show_input("Input", "Enter:");
        dialog.input = "hello".to_string();
        dialog.cursor = 5;

        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        let _ = dialog.handle_key(key);

        assert_eq!(dialog.input, "hell");
        assert_eq!(dialog.cursor, 4);
    }

    #[test]
    fn test_input_dialog_delete() {
        let mut dialog = Dialog::new();
        dialog.show_input("Input", "Enter:");
        dialog.input = "hello".to_string();
        dialog.cursor = 2;

        let key = KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE);
        let _ = dialog.handle_key(key);

        assert_eq!(dialog.input, "helo");
    }

    #[test]
    fn test_input_dialog_left_right() {
        let mut dialog = Dialog::new();
        dialog.show_input("Input", "Enter:");
        dialog.input = "hello".to_string();
        dialog.cursor = 3;

        let key = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        let _ = dialog.handle_key(key);
        assert_eq!(dialog.cursor, 2);

        let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        let _ = dialog.handle_key(key);
        assert_eq!(dialog.cursor, 3);
    }

    #[test]
    fn test_input_dialog_home_end() {
        let mut dialog = Dialog::new();
        dialog.show_input("Input", "Enter:");
        dialog.input = "hello".to_string();
        dialog.cursor = 3;

        let key = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
        let _ = dialog.handle_key(key);
        assert_eq!(dialog.cursor, 0);

        let key = KeyEvent::new(KeyCode::End, KeyModifiers::NONE);
        let _ = dialog.handle_key(key);
        assert_eq!(dialog.cursor, 5);
    }

    #[test]
    fn test_input_dialog_ctrl_a_e() {
        let mut dialog = Dialog::new();
        dialog.show_input("Input", "Enter:");
        dialog.input = "hello".to_string();
        dialog.cursor = 3;

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        let _ = dialog.handle_key(key);
        assert_eq!(dialog.cursor, 0);

        let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
        let _ = dialog.handle_key(key);
        assert_eq!(dialog.cursor, 5);
    }

    #[test]
    fn test_input_dialog_ctrl_u() {
        let mut dialog = Dialog::new();
        dialog.show_input("Input", "Enter:");
        dialog.input = "hello world".to_string();
        dialog.cursor = 6;

        let key = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL);
        let _ = dialog.handle_key(key);

        assert_eq!(dialog.input, "world");
        assert_eq!(dialog.cursor, 0);
    }

    #[test]
    fn test_input_dialog_ctrl_k() {
        let mut dialog = Dialog::new();
        dialog.show_input("Input", "Enter:");
        dialog.input = "hello world".to_string();
        dialog.cursor = 5;

        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL);
        let _ = dialog.handle_key(key);

        assert_eq!(dialog.input, "hello");
    }

    #[test]
    fn test_input_dialog_ctrl_w() {
        let mut dialog = Dialog::new();
        dialog.show_input("Input", "Enter:");
        dialog.input = "hello world".to_string();
        dialog.cursor = 11;

        let key = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL);
        let _ = dialog.handle_key(key);

        // Should delete "world"
        assert!(dialog.input.len() < 11);
    }

    // ========================================================================
    // Hidden Dialog Tests
    // ========================================================================

    #[test]
    fn test_hidden_dialog_returns_pending() {
        let mut dialog = Dialog::new();
        // Dialog is hidden by default

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let result = dialog.handle_key(key);

        assert_eq!(result, DialogResult::Pending);
    }
}
