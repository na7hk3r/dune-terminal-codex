use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(" Library ", Theme::title_style()));

    if app.books.is_empty() {
        let empty = vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(Span::styled("  No books indexed yet.", Theme::dim_style())),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(Span::styled("  Run: dune index", Theme::accent_style())),
        ];
        let paragraph = ratatui::widgets::Paragraph::new(empty)
            .block(block)
            .style(Theme::text_style());
        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .books
        .iter()
        .enumerate()
        .map(|(i, (title, path, pages, words))| {
            let marker = if i == app.selected_book { "▸ " } else { "  " };
            let style = if i == app.selected_book {
                Theme::highlight_style()
            } else {
                Theme::text_style()
            };
            let filename = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            ListItem::new(Span::styled(
                format!(
                    "{}{} — {} ({}p, {}w)",
                    marker, title, filename, pages, words
                ),
                style,
            ))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::highlight_style());

    frame.render_widget(list, area);
}
