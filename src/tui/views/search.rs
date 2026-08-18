use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(" Search ", Theme::title_style()));

    let input_text = format!("  {}_", app.search_input);
    let input = Paragraph::new(Line::from(Span::styled(input_text, Theme::text_style())))
        .block(input_block);
    frame.render_widget(input, chunks[0]);

    let result_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(
            format!(" Results ({}) ", app.search_results.len()),
            Theme::title_style(),
        ));

    if app.search_results.is_empty() {
        let msg = if app.search_input.is_empty() {
            "  Type to search your indexed books..."
        } else {
            "  No results found."
        };
        let empty = Paragraph::new(Line::from(Span::styled(msg, Theme::dim_style())))
            .block(result_block)
            .style(Theme::text_style());
        frame.render_widget(empty, chunks[1]);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    for (i, result) in app.search_results.iter().enumerate() {
        let marker = if i == app.selected_result { "▸" } else { " " };
        let style = if i == app.selected_result {
            Theme::highlight_style()
        } else {
            Theme::text_style()
        };

        lines.push(Line::from(Span::styled(
            format!(
                " {} {} — Page {}",
                marker, result.book_title, result.page_number
            ),
            style,
        )));

        let snippet: String = result.highlighted.chars().take(120).collect();
        lines.push(Line::from(Span::styled(
            format!("   {}...", snippet.trim()),
            Theme::dim_style(),
        )));
        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines)
        .block(result_block)
        .style(Theme::text_style());

    frame.render_widget(paragraph, chunks[1]);
}
