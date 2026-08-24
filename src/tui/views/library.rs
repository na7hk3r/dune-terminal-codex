use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::app::App;
use crate::tui::theme::Theme;
use crate::util::{filename_derivable_from_title, format_number, truncate_ellipsis};

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

    // Usable columns inside the borders.
    let max_cols = (area.width.saturating_sub(2) as usize).max(8);

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

            let mut row = format!("{}{}", marker, title);
            let filename_stem = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.rsplit_once('.').map(|(stem, _)| stem))
                .unwrap_or("");
            if !filename_derivable_from_title(title, filename_stem) {
                row.push_str(&format!(" — {}", filename_stem));
            }
            row.push_str(&format!(
                " ({}p, {}w)",
                pages,
                format_number(*words as u64)
            ));

            ListItem::new(Span::styled(
                truncate_ellipsis(&row, max_cols),
                style,
            ))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::highlight_style());

    frame.render_widget(list, area);
}
