use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

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

        let base = result.highlighted.trim();
        let raw: String = base.chars().take(120).collect();
        let was_truncated = base.chars().count() > 120;
        let mut spans: Vec<Span> = crate::util::marker_segments(&raw)
            .into_iter()
            .map(|seg| {
                let style = if seg.matched {
                    Theme::highlight_style()
                } else {
                    Theme::dim_style()
                };
                Span::styled(seg.text, style)
            })
            .collect();
        if was_truncated {
            spans.push(Span::styled("…", Theme::dim_style()));
        }
        lines.push(Line::from(spans));
        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines)
        .block(result_block)
        .wrap(Wrap { trim: true })
        .style(Theme::text_style());

    frame.render_widget(paragraph, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repository::SearchResult;
    use crate::tui::app::ActiveTab;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    fn app_with_one_result(highlighted: &str) -> App {
        let db = crate::database::repository::Database::open(std::path::Path::new(":memory:"))
            .unwrap();
        let config = crate::config::settings::Config::default();
        let mut app = App::new(&db, &config);
        app.active_tab = ActiveTab::Search;
        app.search_input = "spice".to_string();
        app.search_results = vec![SearchResult {
            book_id: 1,
            book_title: "Dune".to_string(),
            page_number: 7,
            content: String::new(),
            highlighted: highlighted.to_string(),
        }];
        app.selected_result = 0;
        app
    }

    fn render_app(app: &App) -> Buffer {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render(frame, frame.area(), app);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn row_text(buffer: &Buffer, y: u16) -> String {
        // Skip the two border columns so the text is the paragraph content.
        (1..buffer.area.width - 1)
            .filter_map(|x| buffer.cell((x, y)).map(|c| c.symbol().to_string()))
            .collect()
    }

    fn find_row(buffer: &Buffer, needle: &str) -> Option<u16> {
        (0..buffer.area.height).find(|y| row_text(buffer, *y).contains(needle))
    }

    #[test]
    fn matched_span_renders_highlighted_without_markers() {
        let app = app_with_one_result("the >>>spice<<< must flow");
        let buffer = render_app(&app);

        // Raw FTS5 markers must never reach the terminal.
        assert!(
            !buffer
                .content()
                .iter()
                .any(|c| c.symbol() == ">" || c.symbol() == "<"),
            "search view leaks marker chars"
        );

        let y = find_row(&buffer, "must flow").expect("snippet row not rendered");
        let text = row_text(&buffer, y);
        // Count cells, not bytes; the content starts one cell past the border.
        let content_col = text.chars().take_while(|c| *c != 's').count();
        let x_s = content_col as u16 + 1;

        // The matched word carries the highlight style…
        let matched = buffer.cell((x_s, y)).unwrap();
        assert_eq!(matched.symbol(), "s");
        assert_eq!(matched.fg, Theme::highlight_style().fg.unwrap());
        assert_eq!(matched.bg, Theme::highlight_style().bg.unwrap());

        // …and the surrounding plain text keeps the dim style.
        let plain = buffer.cell((x_s - 2, y)).unwrap();
        assert_eq!(plain.symbol(), "e");
        assert_eq!(plain.fg, Theme::dim_style().fg.unwrap());
    }

    #[test]
    fn truncated_snippet_renders_ellipsis_without_dangling_marker() {
        // The 120-char cut lands inside the opening `>>>` marker, leaving a
        // dangling `>` that must not become visible text.
        let highlighted = format!("{}>>>spice<<<", "a".repeat(119));
        let app = app_with_one_result(&highlighted);
        let buffer = render_app(&app);

        assert!(
            !buffer
                .content()
                .iter()
                .any(|c| c.symbol() == ">" || c.symbol() == "<"),
            "truncated snippet leaks a dangling marker"
        );

        let y = find_row(&buffer, "…").expect("truncated snippet lacks the ellipsis");
        assert!(row_text(&buffer, y).trim_end().ends_with('…'));
    }
}
