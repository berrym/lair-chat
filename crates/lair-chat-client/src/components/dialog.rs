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
