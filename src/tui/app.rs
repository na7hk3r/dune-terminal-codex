use crate::config::settings::Config;
use crate::database::repository::{Database, SearchResult};
use crate::oracle::engine::OracleEngine;
use crate::yazi::integration::open_pdf;
use std::cell::Cell;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Home,
    Library,
    Search,
    Codex,
    Oracle,
    Stats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexSubTab {
    Characters,
    Houses,
    Planets,
    Glossary,
}

impl ActiveTab {
    pub fn all() -> &'static [ActiveTab] {
        &[
            ActiveTab::Home,
            ActiveTab::Library,
            ActiveTab::Search,
            ActiveTab::Codex,
            ActiveTab::Oracle,
            ActiveTab::Stats,
        ]
    }

    pub fn index(&self) -> usize {
        match self {
            ActiveTab::Home => 0,
            ActiveTab::Library => 1,
            ActiveTab::Search => 2,
            ActiveTab::Codex => 3,
            ActiveTab::Oracle => 4,
            ActiveTab::Stats => 5,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0 => ActiveTab::Home,
            1 => ActiveTab::Library,
            2 => ActiveTab::Search,
            3 => ActiveTab::Codex,
            4 => ActiveTab::Oracle,
            5 => ActiveTab::Stats,
            _ => ActiveTab::Home,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            ActiveTab::Home => "Home",
            ActiveTab::Library => "Library",
            ActiveTab::Search => "Search",
            ActiveTab::Codex => "Codex",
            ActiveTab::Oracle => "Oracle",
            ActiveTab::Stats => "Stats",
        }
    }

    pub fn key_hint(&self) -> &str {
        match self {
            ActiveTab::Home => "1",
            ActiveTab::Library => "2",
            ActiveTab::Search => "3",
            ActiveTab::Codex => "4",
            ActiveTab::Oracle => "5",
            ActiveTab::Stats => "6",
        }
    }
}

impl CodexSubTab {
    pub fn all() -> &'static [CodexSubTab] {
        &[
            CodexSubTab::Characters,
            CodexSubTab::Houses,
            CodexSubTab::Planets,
            CodexSubTab::Glossary,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            CodexSubTab::Characters => "Characters",
            CodexSubTab::Houses => "Houses",
            CodexSubTab::Planets => "Planets",
            CodexSubTab::Glossary => "Glossary",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodexCounts {
    pub characters: u64,
    pub houses: u64,
    pub planets: u64,
    pub glossary: u64,
    pub quotes: u64,
}

pub struct App {
    pub running: bool,
    pub active_tab: ActiveTab,
    pub codex_sub_tab: CodexSubTab,
    /// PDF viewer command from `[reader]` in the config.
    pub reader_command: String,
    /// Folders where indexed PDFs live, from `[library] paths` in the config.
    pub library_paths: Vec<PathBuf>,

    pub books: Vec<(String, String, u32, u32)>,
    pub selected_book: usize,

    pub search_input: String,
    pub search_results: Vec<SearchResult>,
    pub selected_result: usize,

    pub codex_items: Vec<String>,
    pub selected_codex_item: usize,
    pub codex_detail: Option<String>,
    pub codex_counts: CodexCounts,
    /// Scroll offset of the codex detail view (display lines).
    pub codex_detail_scroll: Cell<usize>,
    /// Total wrapped display lines of the current detail (updated on render).
    pub codex_detail_total: Cell<usize>,
    /// Viewport height (inner rows) of the detail view (updated on render).
    pub codex_viewport_height: Cell<u16>,

    pub oracle_wisdom: Option<String>,
    pub oracle_source: Option<String>,
}

impl App {
    pub fn new(db: &Database, config: &Config) -> Self {
        let books = Self::load_books(db);
        let (oracle_wisdom, oracle_source) = Self::load_oracle(db);
        let codex_counts = Self::load_codex_counts(db);

        let mut app = Self {
            running: true,
            active_tab: ActiveTab::Home,
            codex_sub_tab: CodexSubTab::Characters,
            reader_command: config.reader.command.clone(),
            library_paths: config.resolve_library_paths(),
            books,
            selected_book: 0,
            search_input: String::new(),
            search_results: Vec::new(),
            selected_result: 0,
            codex_items: Vec::new(),
            selected_codex_item: 0,
            codex_detail: None,
            codex_counts,
            codex_detail_scroll: Cell::new(0),
            codex_detail_total: Cell::new(0),
            codex_viewport_height: Cell::new(1),
            oracle_wisdom,
            oracle_source,
        };

        app.load_codex_items(db);
        app
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn next_tab(&mut self) {
        let i = self.active_tab.index();
        let total = ActiveTab::all().len();
        self.active_tab = ActiveTab::from_index((i + 1) % total);
    }

    pub fn prev_tab(&mut self) {
        let i = self.active_tab.index();
        let total = ActiveTab::all().len();
        self.active_tab = ActiveTab::from_index((i + total - 1) % total);
    }

    pub fn set_tab(&mut self, tab: ActiveTab) {
        self.active_tab = tab;
    }

    pub fn next_codex_sub_tab(&mut self) {
        let tabs = CodexSubTab::all();
        let current = tabs
            .iter()
            .position(|t| *t == self.codex_sub_tab)
            .unwrap_or(0);
        self.codex_sub_tab = tabs[(current + 1) % tabs.len()];
    }

    pub fn select_next(&mut self) {
        match self.active_tab {
            ActiveTab::Library => {
                if !self.books.is_empty() {
                    self.selected_book = (self.selected_book + 1) % self.books.len();
                }
            }
            ActiveTab::Search => {
                if !self.search_results.is_empty() {
                    self.selected_result = (self.selected_result + 1) % self.search_results.len();
                }
            }
            ActiveTab::Codex
                if !self.codex_items.is_empty() => {
                    self.selected_codex_item =
                        (self.selected_codex_item + 1) % self.codex_items.len();
                }
            _ => {}
        }
    }

    pub fn select_prev(&mut self) {
        match self.active_tab {
            ActiveTab::Library => {
                if !self.books.is_empty() {
                    let len = self.books.len();
                    self.selected_book = (self.selected_book + len - 1) % len;
                }
            }
            ActiveTab::Search => {
                if !self.search_results.is_empty() {
                    let len = self.search_results.len();
                    self.selected_result = (self.selected_result + len - 1) % len;
                }
            }
            ActiveTab::Codex
                if !self.codex_items.is_empty() => {
                    let len = self.codex_items.len();
                    self.selected_codex_item = (self.selected_codex_item + len - 1) % len;
                }
            _ => {}
        }
    }

    pub fn go_to_top(&mut self) {
        match self.active_tab {
            ActiveTab::Library => self.selected_book = 0,
            ActiveTab::Search => self.selected_result = 0,
            ActiveTab::Codex => self.selected_codex_item = 0,
            _ => {}
        }
    }

    pub fn go_to_bottom(&mut self) {
        match self.active_tab {
            ActiveTab::Library => {
                if !self.books.is_empty() {
                    self.selected_book = self.books.len() - 1;
                }
            }
            ActiveTab::Search => {
                if !self.search_results.is_empty() {
                    self.selected_result = self.search_results.len() - 1;
                }
            }
            ActiveTab::Codex
                if !self.codex_items.is_empty() => {
                    self.selected_codex_item = self.codex_items.len() - 1;
                }
            _ => {}
        }
    }

    pub fn search(&mut self, query: &str, db: &Database) {
        self.search_input = query.to_string();
        self.search_results = db.search_fts(query, None, 20).unwrap_or_default();
        self.selected_result = 0;
    }

    pub fn append_search_char(&mut self, c: char) {
        self.search_input.push(c);
    }

    pub fn delete_search_char(&mut self) {
        self.search_input.pop();
    }

    pub fn refresh_oracle(&mut self, db: &Database) {
        let oracle = OracleEngine::new(db);
        match oracle.random_wisdom() {
            Ok(Some((wisdom, source))) => {
                self.oracle_wisdom = Some(wisdom);
                self.oracle_source = source;
            }
            _ => {
                self.oracle_wisdom = Some("The Oracle is silent.".to_string());
                self.oracle_source = None;
            }
        }
    }

    pub fn show_codex_detail(&mut self, db: &Database) {
        if self.codex_items.is_empty() {
            return;
        }
        let name = &self.codex_items[self.selected_codex_item];
        let detail = match self.codex_sub_tab {
            CodexSubTab::Characters => db
                .character_by_name(name)
                .ok()
                .flatten()
                .unwrap_or_else(|| "No details available.".to_string()),
            CodexSubTab::Houses => db
                .house_by_name(name)
                .ok()
                .flatten()
                .unwrap_or_else(|| "No details available.".to_string()),
            CodexSubTab::Planets => db
                .planet_by_name(name)
                .ok()
                .flatten()
                .unwrap_or_else(|| "No details available.".to_string()),
            CodexSubTab::Glossary => db
                .glossary_by_term(name)
                .ok()
                .flatten()
                .unwrap_or_else(|| "No details available.".to_string()),
        };
        self.codex_detail = Some(detail);
    }

    pub fn close_detail(&mut self) {
        self.codex_detail = None;
        self.codex_detail_scroll.set(0);
    }

    pub fn scroll_detail_down(&mut self, lines: usize) {
        let next = self
            .codex_detail_scroll
            .get()
            .saturating_add(lines)
            .min(self.max_detail_offset());
        self.codex_detail_scroll.set(next);
    }

    pub fn scroll_detail_up(&mut self, lines: usize) {
        self.codex_detail_scroll
            .set(self.codex_detail_scroll.get().saturating_sub(lines));
    }

    pub fn scroll_detail_top(&mut self) {
        self.codex_detail_scroll.set(0);
    }

    pub fn scroll_detail_bottom(&mut self) {
        self.codex_detail_scroll.set(self.max_detail_offset());
    }

    pub fn detail_page_size(&self) -> usize {
        (self.codex_viewport_height.get() as usize).max(1)
    }

    fn max_detail_offset(&self) -> usize {
        self.codex_detail_total
            .get()
            .saturating_sub(self.detail_page_size())
    }

    fn load_books(db: &Database) -> Vec<(String, String, u32, u32)> {
        let mut stmt = db
            .conn
            .prepare("SELECT title, file_path, page_count, word_count FROM books ORDER BY title")
            .unwrap();
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, u32>(3)?,
            ))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    fn load_codex_items(&mut self, db: &Database) {
        self.codex_items.clear();
        self.codex_detail = None;
        self.selected_codex_item = 0;

        match self.codex_sub_tab {
            CodexSubTab::Characters => {
                if let Ok(mut stmt) = db.conn.prepare("SELECT name FROM characters ORDER BY name") {
                    self.codex_items = stmt
                        .query_map([], |row| row.get(0))
                        .unwrap()
                        .filter_map(|r| r.ok())
                        .collect();
                }
            }
            CodexSubTab::Houses => {
                if let Ok(mut stmt) = db.conn.prepare("SELECT name FROM houses ORDER BY name") {
                    self.codex_items = stmt
                        .query_map([], |row| row.get(0))
                        .unwrap()
                        .filter_map(|r| r.ok())
                        .collect();
                }
            }
            CodexSubTab::Planets => {
                if let Ok(mut stmt) = db.conn.prepare("SELECT name FROM planets ORDER BY name") {
                    self.codex_items = stmt
                        .query_map([], |row| row.get(0))
                        .unwrap()
                        .filter_map(|r| r.ok())
                        .collect();
                }
            }
            CodexSubTab::Glossary => {
                if let Ok(mut stmt) = db.conn.prepare("SELECT term FROM glossary ORDER BY term") {
                    self.codex_items = stmt
                        .query_map([], |row| row.get(0))
                        .unwrap()
                        .filter_map(|r| r.ok())
                        .collect();
                }
            }
        }
    }

    pub fn switch_codex_sub_tab(&mut self, tab: CodexSubTab, db: &Database) {
        self.codex_sub_tab = tab;
        self.load_codex_items(db);
    }

    fn load_oracle(db: &Database) -> (Option<String>, Option<String>) {
        let oracle = OracleEngine::new(db);
        match oracle.random_wisdom() {
            Ok(Some((wisdom, source))) => (Some(wisdom), source),
            _ => (Some("The Oracle is silent.".to_string()), None),
        }
    }

    fn load_codex_counts(db: &Database) -> CodexCounts {
        fn count(db: &Database, table: &str) -> u64 {
            db.conn
                .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |r| {
                    r.get::<_, i64>(0)
                })
                .unwrap_or(0) as u64
        }

        CodexCounts {
            characters: count(db, "characters"),
            houses: count(db, "houses"),
            planets: count(db, "planets"),
            glossary: count(db, "glossary"),
            quotes: count(db, "quotes"),
        }
    }

    fn resolve_pdf(&self, stored: &str) -> anyhow::Result<PathBuf> {
        crate::library::resolver::resolve_file_path(stored, &self.library_paths)
            .ok_or_else(|| {
                anyhow::anyhow!("PDF not found. Run `dune index` from a configured library path.\n  stored: {stored}")
            })
    }

    pub fn open_selected_book(&self) -> anyhow::Result<()> {
        if self.active_tab != ActiveTab::Library || self.books.is_empty() {
            return Ok(());
        }
        let (_, path, page, _) = &self.books[self.selected_book];
        let resolved = self.resolve_pdf(path)?;
        let page_num = if *page > 0 { Some(*page) } else { None };
        open_pdf(&resolved.to_string_lossy(), page_num, &self.reader_command)
    }

    pub fn open_search_result(&self, db: &Database) -> anyhow::Result<()> {
        if self.active_tab != ActiveTab::Search || self.search_results.is_empty() {
            return Ok(());
        }
        let result = &self.search_results[self.selected_result];
        let page_num = if result.page_number > 0 {
            Some(result.page_number)
        } else {
            None
        };

        let file_path: String = db
            .conn
            .query_row(
                "SELECT file_path FROM books WHERE id = ?1",
                [result.book_id],
                |row| row.get(0),
            )
            .map_err(|e| anyhow::anyhow!("book not found: {}", e))?;

        let resolved = self.resolve_pdf(&file_path)?;
        open_pdf(&resolved.to_string_lossy(), page_num, &self.reader_command)
    }
}
