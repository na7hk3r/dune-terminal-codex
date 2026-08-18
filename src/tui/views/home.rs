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
        .title(Span::styled(" ᑐ ᑌ ᑎ ᑕ ", Theme::title_style()));

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  T E R M I N A L   C O D E X",
            Theme::dim_style(),
        )),
        Line::from(""),
    ];

    let tabs = [
        ("1", "LIBRARY", "Browse indexed books"),
        ("2", "CODEX", "Characters, houses, planets"),
        ("3", "SEARCH", "Full-text search"),
        ("4", "ORACLE", "Receive wisdom"),
        ("5", "STATS", "Library statistics"),
    ];

    for (key, name, desc) in &tabs {
        lines.push(Line::from(vec![
            Span::styled(format!("  [{}] ", key), Theme::accent_style()),
            Span::styled(*name, Theme::text_style()),
            Span::raw(format!("   {}", desc)),
        ]));
    }

    lines.push(Line::from(""));

    if let Some(ref wisdom) = app.oracle_wisdom {
        let truncated: String = wisdom.chars().take(80).collect();
        lines.push(Line::from(Span::styled(
            format!("  \"{}\"", truncated),
            Style::default()
                .add_modifier(Modifier::ITALIC)
                .fg(Theme::earth()),
        )));
        if let Some(ref source) = app.oracle_source {
            lines.push(Line::from(Span::styled(
                format!("  — {}", source),
                Theme::dim_style(),
            )));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Theme::text_style());

    frame.render_widget(paragraph, chunks[0]);

    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(" Status ", Theme::title_style()));

    let status_line = Line::from(vec![
        Span::styled(format!(" {} books", app.books.len()), Theme::text_style()),
        Span::styled(" │ ", Theme::dim_style()),
        Span::styled(
            format!(" {} codex items", app.codex_items.len()),
            Theme::text_style(),
        ),
        Span::styled(" │ ", Theme::dim_style()),
        Span::styled(
            format!(" {} search results", app.search_results.len()),
            Theme::text_style(),
        ),
    ]);

    let status = Paragraph::new(status_line)
        .block(status_block)
        .style(Theme::text_style());

    frame.render_widget(status, chunks[1]);
}
