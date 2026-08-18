use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::tui::app::{App, CodexSubTab};
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(ref detail) = app.codex_detail {
        render_detail(frame, area, app, detail);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let tab_names: Vec<String> = CodexSubTab::all()
        .iter()
        .map(|t| {
            let marker = if *t == app.codex_sub_tab {
                "●"
            } else {
                "○"
            };
            format!(" {} {} ", marker, t.label())
        })
        .collect();

    let tab_bar = Paragraph::new(Line::from(Span::styled(
        tab_names.join(" "),
        Theme::text_style(),
    )))
    .style(Theme::text_style());
    frame.render_widget(tab_bar, chunks[0]);

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(
            format!(" {} ", app.codex_sub_tab.label()),
            Theme::title_style(),
        ));

    if app.codex_items.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No data. Run: dune import-codex",
            Theme::dim_style(),
        )))
        .block(list_block)
        .style(Theme::text_style());
        frame.render_widget(empty, chunks[1]);
        return;
    }

    let items: Vec<ListItem> = app
        .codex_items
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let marker = if i == app.selected_codex_item {
                "▸ "
            } else {
                "  "
            };
            let style = if i == app.selected_codex_item {
                Theme::highlight_style()
            } else {
                Theme::text_style()
            };
            ListItem::new(Span::styled(format!("{}{}", marker, name), style))
        })
        .collect();

    let list = List::new(items)
        .block(list_block)
        .highlight_style(Theme::highlight_style());

    frame.render_widget(list, chunks[1]);
}

fn render_detail(frame: &mut Frame, area: Rect, app: &App, detail: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(
            format!(
                " {} ",
                app.codex_items
                    .get(app.selected_codex_item)
                    .unwrap_or(&String::new())
            ),
            Theme::title_style(),
        ));

    let lines: Vec<Line> = detail
        .lines()
        .take((area.height as usize).saturating_sub(4))
        .map(|l| Line::from(Span::styled(l, Theme::text_style())))
        .chain(std::iter::once(Line::from("")))
        .chain(std::iter::once(Line::from(Span::styled(
            "  [Esc] back   [j/k] navigate",
            Theme::dim_style(),
        ))))
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Theme::text_style());

    frame.render_widget(paragraph, area);
}
