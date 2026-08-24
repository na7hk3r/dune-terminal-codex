use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::app::App;
use crate::tui::theme::Theme;
use crate::util::format_number;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let lib_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(" 📚 Library ", Theme::title_style()));

    let books = app.books.len() as u64;
    let total_pages: u64 = app.books.iter().map(|(_, _, p, _)| *p as u64).sum();
    let total_words: u64 = app.books.iter().map(|(_, _, _, w)| *w as u64).sum();

    let lib_lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Books:     ", Theme::dim_style())),
        Line::from(Span::styled(
            format!("    {}", format_number(books)),
            Theme::text_style(),
        )),
        Line::from(""),
        Line::from(Span::styled("  Pages:     ", Theme::dim_style())),
        Line::from(Span::styled(
            format!("    {}", format_number(total_pages)),
            Theme::text_style(),
        )),
        Line::from(""),
        Line::from(Span::styled("  Words:     ", Theme::dim_style())),
        Line::from(Span::styled(
            format!("    {}", format_number(total_words)),
            Theme::text_style(),
        )),
    ];

    let lib = Paragraph::new(lib_lines)
        .block(lib_block)
        .style(Theme::text_style());

    frame.render_widget(lib, chunks[0]);

    let codex_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(" 📜 Codex ", Theme::title_style()));

    let codex_lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Characters: ", Theme::dim_style())),
        Line::from(Span::styled("    27", Theme::text_style())),
        Line::from(""),
        Line::from(Span::styled("  Houses:     ", Theme::dim_style())),
        Line::from(Span::styled("    9", Theme::text_style())),
        Line::from(""),
        Line::from(Span::styled("  Planets:    ", Theme::dim_style())),
        Line::from(Span::styled("    12", Theme::text_style())),
        Line::from(""),
        Line::from(Span::styled("  Glossary:   ", Theme::dim_style())),
        Line::from(Span::styled("    30", Theme::text_style())),
        Line::from(""),
        Line::from(Span::styled("  Quotes:     ", Theme::dim_style())),
        Line::from(Span::styled("    17", Theme::text_style())),
    ];

    let codex = Paragraph::new(codex_lines)
        .block(codex_block)
        .style(Theme::text_style());

    frame.render_widget(codex, chunks[1]);
}
