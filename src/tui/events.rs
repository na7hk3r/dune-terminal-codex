use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::tui::app::{ActiveTab, App};

pub enum AppEvent {
    Tick,
    Key(KeyEvent),
}

pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    pub fn next(&self) -> anyhow::Result<AppEvent> {
        if event::poll(self.tick_rate)? {
            if let Event::Key(key) = event::read()? {
                return Ok(AppEvent::Key(key));
            }
        }
        Ok(AppEvent::Tick)
    }
}

pub fn handle_key_event(app: &mut App, key: KeyEvent, db: &crate::database::repository::Database) {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => app.quit(),
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            if app.codex_detail.is_some() {
                app.close_detail();
            } else if app.active_tab == ActiveTab::Search && !app.search_input.is_empty() {
                app.search_input.clear();
                app.search_results.clear();
            } else {
                app.quit();
            }
        }
        KeyCode::Char('1') => app.set_tab(ActiveTab::Home),
        KeyCode::Char('2') => app.set_tab(ActiveTab::Library),
        KeyCode::Char('3') => app.set_tab(ActiveTab::Search),
        KeyCode::Char('4') => app.set_tab(ActiveTab::Codex),
        KeyCode::Char('5') => app.set_tab(ActiveTab::Oracle),
        KeyCode::Char('6') => app.set_tab(ActiveTab::Stats),
        KeyCode::Tab => app.next_tab(),
        KeyCode::BackTab => app.prev_tab(),
        KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Enter => {
            if app.active_tab == ActiveTab::Codex && app.codex_detail.is_none() {
                app.show_codex_detail(db);
            } else if app.active_tab == ActiveTab::Library {
                let _ = app.open_selected_book();
            } else if app.active_tab == ActiveTab::Search && !app.search_results.is_empty() {
                let _ = app.open_search_result(db);
            }
        }
        KeyCode::Char('r') => {
            if app.active_tab == ActiveTab::Oracle {
                app.refresh_oracle(db);
            }
        }
        KeyCode::Char('c') => {
            if app.active_tab == ActiveTab::Codex && app.codex_detail.is_none() {
                app.next_codex_sub_tab();
                app.switch_codex_sub_tab(app.codex_sub_tab, db);
            }
        }
        KeyCode::Char(c) if app.active_tab == ActiveTab::Search => {
            if app.codex_detail.is_some() {
                return;
            }
            app.append_search_char(c);
            if !app.search_input.is_empty() {
                app.search(&app.search_input.clone(), db);
            }
        }
        KeyCode::Backspace if app.active_tab == ActiveTab::Search => {
            app.delete_search_char();
            if app.search_input.is_empty() {
                app.search_results.clear();
            } else {
                app.search(&app.search_input.clone(), db);
            }
        }
        _ => {}
    }
}
