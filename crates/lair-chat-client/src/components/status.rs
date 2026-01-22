//! Status bar component.

#![allow(dead_code)]

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Render a status bar.
pub fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    username: Option<&str>,
    room: Option<&str>,
    status: Option<&str>,
    error: Option<&str>,
) {
    let mut spans = vec![Span::styled(" ", Style::default())];

    // User info
    if let Some(user) = username {
        spans.push(Span::styled(user, Style::default().fg(Color::Green)));
    } else {
        spans.push(Span::styled(
            "Not logged in",
            Style::default().fg(Color::DarkGray),
        ));
    }

    spans.push(Span::raw(" | "));

    // Room info
    if let Some(room) = room {
        spans.push(Span::styled(room, Style::default().fg(Color::Cyan)));
    } else {
        spans.push(Span::styled(
            "No room",
            Style::default().fg(Color::DarkGray),
        ));
    }

    spans.push(Span::raw(" | "));

    // Status
    if let Some(status) = status {
        spans.push(Span::styled(status, Style::default().fg(Color::Blue)));
    }

    // Error (if any)
    if let Some(err) = error {
        spans.push(Span::raw(" | "));
        spans.push(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        ));
    }

    let line = Line::from(spans);
    let para = Paragraph::new(line);
    frame.render_widget(para, area);
}
