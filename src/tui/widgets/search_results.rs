use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::theme::Theme;

#[allow(dead_code)]
pub fn render_search_result(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    page: u32,
    snippet: &str,
    selected: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(
            format!(" {} — Page {} ", title, page),
            Theme::title_style(),
        ));

    let marker = if selected { "▸ " } else { "  " };
    let style = if selected {
        Theme::highlight_style()
    } else {
        Theme::text_style()
    };

    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        format!("{}{}", marker, snippet),
        style,
    ))];

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [Enter] open   [Esc] back",
        Theme::dim_style(),
    )));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Theme::text_style());

    frame.render_widget(paragraph, area);
}
