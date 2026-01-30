//! Full-screen help overlay component.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

/// Help overlay state.
pub struct HelpOverlay {
    /// Whether the help overlay is visible.
    pub visible: bool,
    /// Scroll position.
    scroll: usize,
}

impl Default for HelpOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpOverlay {
    /// Create a new help overlay (hidden by default).
    pub fn new() -> Self {
        Self {
            visible: false,
            scroll: 0,
        }
    }

    /// Show the help overlay.
    pub fn show(&mut self) {
        self.visible = true;
        self.scroll = 0;
    }

    /// Hide the help overlay.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Handle a key event. Returns true if the overlay should close.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if !self.visible {
            return false;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char(' ') => {
                self.hide();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1);
                false
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
                false
            }
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(10);
                false
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
                false
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.scroll = 0;
                false
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.scroll = usize::MAX; // Will be clamped in render
                false
            }
            _ => false,
        }
    }

    /// Render the help overlay.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Clear the entire screen
        frame.render_widget(Clear, area);

        // Main block
        let block = Block::default()
            .title(" Help ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Build help content
        let help_lines = build_help_lines();
        let total_lines = help_lines.len();

        // Calculate visible area (leave room for footer)
        let content_area = Rect {
            x: inner.x + 1,
            y: inner.y,
            width: inner.width.saturating_sub(3), // Room for scrollbar
            height: inner.height.saturating_sub(2), // Room for footer
        };

        let visible_lines = content_area.height as usize;

        // Clamp scroll position
        let max_scroll = total_lines.saturating_sub(visible_lines);
        self.scroll = self.scroll.min(max_scroll);

        // Render visible lines
        let visible_content: Vec<Line> = help_lines
            .into_iter()
            .skip(self.scroll)
            .take(visible_lines)
            .collect();

        let paragraph = Paragraph::new(visible_content);
        frame.render_widget(paragraph, content_area);

        // Render scrollbar if needed
        if total_lines > visible_lines {
            let scrollbar_area = Rect {
                x: inner.x + inner.width - 1,
                y: inner.y,
                width: 1,
                height: inner.height.saturating_sub(2),
            };

            let mut scrollbar_state = ScrollbarState::new(total_lines).position(self.scroll);

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

        // Footer with hints
        let footer_area = Rect {
            x: inner.x,
            y: inner.y + inner.height - 1,
            width: inner.width,
            height: 1,
        };

        let footer = Paragraph::new(Line::from(vec![
            Span::styled(" j/k", Style::default().fg(Color::Yellow)),
            Span::styled(" scroll  ", Style::default().fg(Color::DarkGray)),
            Span::styled("g/G", Style::default().fg(Color::Yellow)),
            Span::styled(" top/bottom  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc/Enter/q", Style::default().fg(Color::Yellow)),
            Span::styled(" close", Style::default().fg(Color::DarkGray)),
        ]))
        .alignment(Alignment::Center);

        frame.render_widget(footer, footer_area);
    }
}

/// Add a section header to the lines vector.
fn add_header(lines: &mut Vec<Line<'static>>, text: &'static str) {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        text,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "─".repeat(text.len()),
        Style::default().fg(Color::DarkGray),
    )));
}

/// Add a key binding to the lines vector.
fn add_key(lines: &mut Vec<Line<'static>>, key: &'static str, desc: &'static str) {
    lines.push(Line::from(vec![
        Span::styled(format!("  {:<14}", key), Style::default().fg(Color::Yellow)),
        Span::styled(desc, Style::default().fg(Color::White)),
    ]));
}

/// Build the help content as styled lines.
fn build_help_lines() -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(
        "═══════════════════════════════════════",
        Style::default().fg(Color::Cyan),
    )));
    lines.push(Line::from(Span::styled(
        "           LAIR CHAT HELP",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "═══════════════════════════════════════",
        Style::default().fg(Color::Cyan),
    )));

    // Navigation
    add_header(&mut lines, "NAVIGATION");
    add_key(&mut lines, "i", "Enter insert mode to type messages");
    add_key(&mut lines, "Esc", "Exit insert mode / Close dialogs");
    add_key(&mut lines, "r", "Open room list");
    add_key(&mut lines, "Tab", "Switch between messages and users panel");
    add_key(&mut lines, "j / ↓", "Scroll down / Navigate down");
    add_key(&mut lines, "k / ↑", "Scroll up / Navigate up");
    add_key(&mut lines, "G", "Jump to bottom of messages");
    add_key(&mut lines, "g", "Jump to top of messages");
    add_key(&mut lines, "y", "Copy last message to clipboard");
    add_key(&mut lines, "I", "Show pending invitations");
    add_key(&mut lines, "m", "Show room members (in a room)");
    add_key(&mut lines, "q", "Quit application");
    add_key(&mut lines, "R", "Reconnect to server");
    add_key(&mut lines, "? / F1", "Show this help");
    add_key(&mut lines, "Ctrl+P", "Open command palette");

    // Input mode
    add_header(&mut lines, "INPUT MODE (when typing)");
    add_key(&mut lines, "Enter", "Send message");
    add_key(&mut lines, "Alt+Enter", "Insert newline");
    add_key(&mut lines, "Esc", "Exit insert mode");
    add_key(&mut lines, "Ctrl+V / Ctrl+Y", "Paste from clipboard");
    add_key(&mut lines, "Ctrl+A", "Move cursor to start of line");
    add_key(&mut lines, "Ctrl+E", "Move cursor to end of line");
    add_key(&mut lines, "Ctrl+W", "Delete word before cursor");
    add_key(&mut lines, "Ctrl+U", "Clear line to start");
    add_key(&mut lines, "Ctrl+K", "Clear line to end");

    // Commands
    add_header(&mut lines, "COMMANDS (type in insert mode)");
    add_key(&mut lines, "/help", "Show this help");
    add_key(&mut lines, "/rooms", "Open room list");
    add_key(&mut lines, "/create <name>", "Create a new room");
    add_key(
        &mut lines,
        "/dm <username>",
        "Start direct message with user",
    );
    add_key(&mut lines, "/quit", "Quit application");

    // Users panel
    add_header(&mut lines, "USERS PANEL (press Tab to switch)");
    add_key(&mut lines, "j / k", "Navigate user list");
    add_key(&mut lines, "Enter", "Start DM with selected user");
    add_key(&mut lines, "i", "Invite selected user to current room");
    add_key(&mut lines, "Tab / Esc", "Return to messages");

    // Room list
    add_header(&mut lines, "ROOM LIST (press r to open)");
    add_key(&mut lines, "j / k", "Navigate room list");
    add_key(&mut lines, "Enter", "Join selected room");
    add_key(&mut lines, "c", "Create new room");
    add_key(&mut lines, "Esc", "Return to chat");

    // Getting started
    add_header(&mut lines, "GETTING STARTED");
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  1. Press 'r' to open the room list",
        Style::default().fg(Color::White),
    )));
    lines.push(Line::from(Span::styled(
        "  2. Join an existing room or press 'c' to create one",
        Style::default().fg(Color::White),
    )));
    lines.push(Line::from(Span::styled(
        "  3. Press 'i' to enter insert mode and type a message",
        Style::default().fg(Color::White),
    )));
    lines.push(Line::from(Span::styled(
        "  4. Press Enter to send, or Esc to cancel",
        Style::default().fg(Color::White),
    )));
    lines.push(Line::from(Span::styled(
        "  5. Press Tab to switch to users panel for DMs",
        Style::default().fg(Color::White),
    )));

    lines.push(Line::from(""));

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn test_help_overlay_new() {
        let overlay = HelpOverlay::new();
        assert!(!overlay.visible);
        assert_eq!(overlay.scroll, 0);
    }

    #[test]
    fn test_help_overlay_show_hide() {
        let mut overlay = HelpOverlay::new();

        overlay.show();
        assert!(overlay.visible);
        assert_eq!(overlay.scroll, 0);

        overlay.hide();
        assert!(!overlay.visible);
    }

    #[test]
    fn test_help_overlay_scroll_reset_on_show() {
        let mut overlay = HelpOverlay::new();
        overlay.scroll = 10;

        overlay.show();
        assert_eq!(overlay.scroll, 0);
    }

    #[test]
    fn test_handle_key_close_esc() {
        let mut overlay = HelpOverlay::new();
        overlay.show();

        let closed = overlay.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(closed);
        assert!(!overlay.visible);
    }

    #[test]
    fn test_handle_key_close_enter() {
        let mut overlay = HelpOverlay::new();
        overlay.show();

        let closed = overlay.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(closed);
        assert!(!overlay.visible);
    }

    #[test]
    fn test_handle_key_close_q() {
        let mut overlay = HelpOverlay::new();
        overlay.show();

        let closed = overlay.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(closed);
        assert!(!overlay.visible);
    }

    #[test]
    fn test_handle_key_scroll_down() {
        let mut overlay = HelpOverlay::new();
        overlay.show();

        let closed = overlay.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert!(!closed);
        assert_eq!(overlay.scroll, 1);
    }

    #[test]
    fn test_handle_key_scroll_up() {
        let mut overlay = HelpOverlay::new();
        overlay.show();
        overlay.scroll = 5;

        let closed = overlay.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert!(!closed);
        assert_eq!(overlay.scroll, 4);
    }

    #[test]
    fn test_handle_key_scroll_up_at_top() {
        let mut overlay = HelpOverlay::new();
        overlay.show();
        overlay.scroll = 0;

        let closed = overlay.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert!(!closed);
        assert_eq!(overlay.scroll, 0);
    }

    #[test]
    fn test_handle_key_home() {
        let mut overlay = HelpOverlay::new();
        overlay.show();
        overlay.scroll = 50;

        let closed = overlay.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert!(!closed);
        assert_eq!(overlay.scroll, 0);
    }

    #[test]
    fn test_handle_key_when_not_visible() {
        let mut overlay = HelpOverlay::new();
        // Not visible

        let closed = overlay.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!closed);
    }

    #[test]
    fn test_build_help_lines_not_empty() {
        let lines = build_help_lines();
        assert!(!lines.is_empty());
        assert!(lines.len() > 20); // Should have substantial content
    }
}
