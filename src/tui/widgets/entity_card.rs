use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::theme::Theme;

#[allow(dead_code)]
pub fn render_entity_card(frame: &mut Frame, area: Rect, name: &str, details: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(format!(" {} ", name), Theme::title_style()));

    let mut lines: Vec<Line> = details
        .lines()
        .map(|l| Line::from(Span::styled(l, Theme::text_style())))
        .collect();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  [Esc] back", Theme::dim_style())));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Theme::text_style());

    frame.render_widget(paragraph, area);
}
