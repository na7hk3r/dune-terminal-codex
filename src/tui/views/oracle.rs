use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(" 🌙 Oracle ", Theme::title_style()));

    let wisdom = app
        .oracle_wisdom
        .as_deref()
        .unwrap_or("The Oracle is silent.");

    let source = app.oracle_source.as_deref();

    let mut lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            format!("    \"{}\"", wisdom),
            Style::default()
                .add_modifier(Modifier::ITALIC)
                .fg(Theme::sand()),
        )),
        Line::from(""),
    ];

    if let Some(s) = source {
        lines.push(Line::from(Span::styled(
            format!("    — {}", s),
            Theme::dim_style(),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Theme::text_style());

    frame.render_widget(paragraph, chunks[0]);

    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(" Actions ", Theme::title_style()));

    let status_line = Line::from(vec![
        Span::styled("  [r] ", Theme::accent_style()),
        Span::styled("New wisdom", Theme::text_style()),
        Span::styled("  │  ", Theme::dim_style()),
        Span::styled("  [Tab] ", Theme::accent_style()),
        Span::styled("Next tab", Theme::text_style()),
        Span::styled("  │  ", Theme::dim_style()),
        Span::styled("  [q] ", Theme::accent_style()),
        Span::styled("Home", Theme::text_style()),
    ]);

    let status = Paragraph::new(status_line)
        .block(status_block)
        .style(Theme::text_style());

    frame.render_widget(status, chunks[1]);
}
