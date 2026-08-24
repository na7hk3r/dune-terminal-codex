use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use crate::tui::app::{App, CodexSubTab};
use crate::tui::theme::Theme;
use crate::util::wrap_text;

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
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let title = app
        .codex_items
        .get(app.selected_codex_item)
        .cloned()
        .unwrap_or_default();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::accent_style())
        .title(Span::styled(
            format!(" {} ", title),
            Theme::title_style(),
        ));

    let inner = block.inner(rows[0]);
    let width = (inner.width as usize).max(8);
    let viewport = (inner.height as usize).max(1);

    // Wrap once per frame so scroll math matches exactly what is displayed.
    let wrapped = wrap_text(detail, width);
    let total = wrapped.len();
    let max_offset = total.saturating_sub(viewport);
    let offset = app.codex_detail_scroll.get().min(max_offset);
    app.codex_detail_scroll.set(offset);
    app.codex_detail_total.set(total);
    app.codex_viewport_height.set(inner.height.max(1));

    let visible_end = (offset + viewport).min(total);
    let lines: Vec<Line> = wrapped[offset..visible_end]
        .iter()
        .map(|l| detail_line(l))
        .collect();

    frame.render_widget(Paragraph::new(lines).block(block), rows[0]);

    let mut scrollbar_state = ScrollbarState::new(max_offset).position(offset);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Theme::dim_style())
            .thumb_style(Theme::accent_style()),
        inner,
        &mut scrollbar_state,
    );

    let current = if total == 0 { 0 } else { offset + 1 };
    let indicator = Line::from(Span::styled(
        format!(
            " línea {}/{} · [j/k] líneas · [PgUp/PgDn] página · [g/G] inicio/fin · [Esc] volver",
            current, total
        ),
        Theme::dim_style(),
    ));
    frame.render_widget(Paragraph::new(indicator), rows[1]);
}

fn detail_line(line: &str) -> Line<'static> {
    if let Some(header) = line.strip_prefix("## ") {
        return Line::from(Span::styled(
            header.to_uppercase(),
            Theme::accent_style().add_modifier(Modifier::BOLD),
        ));
    }
    // "Label: value" rows inside the Profile block.
    if let Some((label, rest)) = split_profile_field(line) {
        return Line::from(vec![
            Span::styled(
                format!("{}: ", label),
                Theme::accent_style().add_modifier(Modifier::BOLD),
            ),
            Span::styled(rest.to_string(), Theme::text_style()),
        ]);
    }
    Line::from(Span::styled(line.to_string(), Theme::text_style()))
}

fn split_profile_field(line: &str) -> Option<(&str, &str)> {
    let idx = line.find(": ")?;
    if idx == 0 || idx > 24 {
        return None;
    }
    let label = &line[..idx];
    let first = label.chars().next()?;
    if !first.is_uppercase() {
        return None;
    }
    if !label
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '\'')
    {
        return None;
    }
    Some((label, &line[idx + 2..]))
}
