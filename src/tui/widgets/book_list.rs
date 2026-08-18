use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, ListItem};

use crate::tui::theme::Theme;

#[allow(dead_code)]
pub fn render_book_item(frame: &mut Frame, area: Rect, title: &str, selected: bool) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(" Book ", Theme::title_style()));

    let marker = if selected { "▸ " } else { "  " };
    let style = if selected {
        Theme::highlight_style()
    } else {
        Theme::text_style()
    };

    let item = ListItem::new(Span::styled(format!("{}{}", marker, title), style));

    let list = ratatui::widgets::List::new(vec![item]).block(block);
    frame.render_widget(list, area);
}
