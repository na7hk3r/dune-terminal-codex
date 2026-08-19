mod cli;
mod codex;
mod codex_import;
mod config;
mod database;
mod error;
mod library;
mod oracle;
mod search;
mod tui;
mod yazi;

use clap::Parser;
use cli::commands::{Cli, Commands};
use config::settings::Config;
use database::repository::Database;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Tui) => run_tui(),
        Some(Commands::Search { query, book }) => cmd_search(&query, book.as_deref()),
        Some(Commands::Books) => cmd_books(),
        Some(Commands::Book { name }) => cmd_book(&name),
        Some(Commands::Character { name }) => cmd_character(&name),
        Some(Commands::House { name }) => cmd_house(&name),
        Some(Commands::Planet { name }) => cmd_planet(&name),
        Some(Commands::Glossary { term }) => cmd_glossary(term.as_deref()),
        Some(Commands::Quote) => cmd_quote(),
        Some(Commands::Oracle) => cmd_oracle(),
        Some(Commands::Stats) => cmd_stats(),
        Some(Commands::Index {
            rebuild,
            verbose,
            files,
        }) => cmd_index(rebuild, verbose, files),
        Some(Commands::ImportCodex) => cmd_import_codex(),
        Some(Commands::Config) => cmd_config(),
        Some(Commands::Open { path, page }) => cmd_open(&path, page),
    }
}

fn open_database() -> anyhow::Result<Database> {
    let config = Config::load_or_default();
    Database::open_default(&config)
}

fn cmd_search(query: &str, book: Option<&str>) -> anyhow::Result<()> {
    let db = open_database()?;
    let results = db.search_fts(query, 20)?;

    if results.is_empty() {
        println!("No results for \"{}\"", query);
        if let Some(book_filter) = book {
            println!("  (filtered by book: {})", book_filter);
        }
        return Ok(());
    }

    println!("SEARCH: \"{}\"", query);
    println!("{}", "─".repeat(50));

    for (i, result) in results.iter().enumerate() {
        println!("\n{}", result.book_title.to_uppercase());
        println!("Page {}", result.page_number);
        println!();
        let snippet: String = result.highlighted.chars().take(200).collect();
        println!("  {}...", snippet.trim());
        if i < results.len() - 1 {
            println!();
        }
    }

    println!("\n{}", "─".repeat(50));
    println!("{} results found", results.len());

    Ok(())
}

fn cmd_books() -> anyhow::Result<()> {
    let db = open_database()?;
    let count = db.book_count()?;

    if count == 0 {
        println!("No books indexed.");
        println!("\nRun: dune index");
        return Ok(());
    }

    let mut stmt = db
        .conn
        .prepare("SELECT title, page_count, word_count FROM books ORDER BY title")?;
    let books: Vec<(String, u32, u32)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    println!("DUNE LIBRARY");
    println!("{}", "─".repeat(40));
    println!("{:<30} {:>8} {:>12}", "Title", "Pages", "Words");
    println!("{}", "─".repeat(40));

    for (title, pages, words) in &books {
        println!(
            "{:<30} {:>8} {:>12}",
            title,
            pages,
            format_number(*words as u64)
        );
    }

    println!("{}", "─".repeat(40));
    println!("{} books indexed", books.len());

    Ok(())
}

fn cmd_book(name: &str) -> anyhow::Result<()> {
    let db = open_database()?;
    let pattern = format!("%{}%", name.to_lowercase());
    let result = db.conn.query_row(
        "SELECT title, file_path, page_count, word_count, file_size
         FROM books WHERE LOWER(title) LIKE ?1 LIMIT 1",
        rusqlite::params![pattern],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, u32>(3)?,
                row.get::<_, i64>(4)?,
            ))
        },
    );

    match result {
        Ok((title, path, pages, words, size)) => {
            println!("{}", title.to_uppercase());
            println!("{}", "─".repeat(40));
            println!("Path:   {}", path);
            println!("Pages:  {}", pages);
            println!("Words:  {}", words);
            println!("Size:   {} KB", size / 1024);
        }
        Err(_) => {
            println!("No book found matching \"{}\"", name);
        }
    }

    Ok(())
}

fn cmd_character(name: &str) -> anyhow::Result<()> {
    let db = open_database()?;
    match db.character_by_name(name)? {
        Some(info) => {
            println!("{}", info);
        }
        None => {
            println!("No character found matching \"{}\"", name);
            println!("\nTip: dune import-codex  (to populate the codex)");
        }
    }
    Ok(())
}

fn cmd_house(name: &str) -> anyhow::Result<()> {
    let db = open_database()?;
    match db.house_by_name(name)? {
        Some(info) => {
            println!("{}", info);
        }
        None => {
            println!("No house found matching \"{}\"", name);
            println!("\nTip: dune import-codex  (to populate the codex)");
        }
    }
    Ok(())
}

fn cmd_planet(name: &str) -> anyhow::Result<()> {
    let db = open_database()?;
    match db.planet_by_name(name)? {
        Some(info) => {
            println!("{}", info);
        }
        None => {
            println!("No planet found matching \"{}\"", name);
            println!("\nTip: dune import-codex  (to populate the codex)");
        }
    }
    Ok(())
}

fn cmd_glossary(term: Option<&str>) -> anyhow::Result<()> {
    let db = open_database()?;

    match term {
        None => {
            let entries = db.glossary_list(50)?;
            if entries.is_empty() {
                println!("Glossary is empty.");
                println!("\nRun: dune import-codex");
                return Ok(());
            }
            println!("GLOSSARY");
            println!("{}", "─".repeat(50));
            for (t, _) in &entries {
                println!("  {}", t);
            }
            println!("{}", "─".repeat(50));
            println!("{} terms", entries.len());
        }
        Some(t) => match db.glossary_by_term(t)? {
            Some(info) => println!("{}", info),
            None => {
                println!("No term found matching \"{}\"", t);
                println!("\nRun: dune glossary  (to browse all terms)");
            }
        },
    }

    Ok(())
}

fn cmd_quote() -> anyhow::Result<()> {
    let db = open_database()?;
    match db.random_quote()? {
        Some((text, attribution, book_title)) => {
            println!();
            println!("  \"{}\"", text);
            if let Some(attr) = attribution {
                println!();
                let mut source = attr;
                if let Some(book) = book_title {
                    source = format!("{} — {}", source, book);
                }
                println!("  — {}", source);
            }
            println!();
        }
        None => {
            println!("No quotes available.");
            println!("\nRun: dune import-codex");
        }
    }
    Ok(())
}

fn cmd_oracle() -> anyhow::Result<()> {
    let db = open_database()?;
    let oracle = oracle::engine::OracleEngine::new(&db);

    match oracle.random_wisdom()? {
        Some((wisdom, source)) => {
            println!();
            println!("  ╭──────────────────────────────────────────╮");
            println!("  │               ORACLE                      │");
            println!("  │                                           │");
            println!("  \"{}\"", wisdom);
            if let Some(s) = source {
                println!("  — {}", s);
            }
            println!("  │                                           │");
            println!("  ╰──────────────────────────────────────────╯");
            println!();
        }
        None => {
            println!("The Oracle is silent.");
            println!("\nRun: dune import-codex");
        }
    }
    Ok(())
}

fn cmd_stats() -> anyhow::Result<()> {
    let db = open_database()?;
    let books = db.book_count()?;
    let pages = db.total_pages()?;
    let words = db.total_words()?;

    println!("DUNE LIBRARY");
    println!("{}", "─".repeat(40));
    println!("  Books:     {:>10}", books);
    println!("  Pages:     {:>10}", pages);
    println!("  Words:     {:>10}", format_number(words));
    println!("{}", "─".repeat(40));

    Ok(())
}

fn cmd_index(rebuild: bool, verbose: bool, files: Option<Vec<String>>) -> anyhow::Result<()> {
    let config = Config::load_or_default();
    config.ensure_dirs()?;

    let db_path = Config::db_path()?;

    if rebuild && db_path.exists() {
        std::fs::remove_file(&db_path)?;
        if verbose {
            println!("Removed existing database.");
        }
    }

    let db = Database::open(&db_path)?;

    if let Some(file_paths) = files {
        for file_path in &file_paths {
            let path = std::path::Path::new(file_path);
            if !path.exists() {
                println!("File not found: {}", file_path);
                continue;
            }
            match library::indexer::index_pdf(&db, path, verbose) {
                Ok((pages, _book_id)) => {
                    println!("Indexed: {} ({} pages)", file_path, pages);
                }
                Err(e) => {
                    println!("Error indexing {}: {}", file_path, e);
                }
            }
        }
    } else {
        let result = library::indexer::index_library(&db, &config, verbose)?;

        println!("Indexing complete.");
        println!("  Books:  {}", result.books_indexed);
        println!("  Pages:  {}", result.pages_indexed);

        if !result.errors.is_empty() {
            println!("  Errors: {}", result.errors.len());
            for err in &result.errors {
                println!("    - {}", err);
            }
        }
    }

    Ok(())
}

fn cmd_import_codex() -> anyhow::Result<()> {
    let db = open_database()?;
    println!("Importing codex data...");

    codex_import::wiki::import_from_wiki(&db.conn, &Config::load_or_default().codex.wiki_url)?;

    println!("Codex import complete.");
    Ok(())
}

fn cmd_config() -> anyhow::Result<()> {
    let config = Config::load_or_default();
    let path = Config::config_path()?;

    if !path.exists() {
        let saved = config.save()?;
        println!("Created default configuration at: {}", saved.display());
    } else {
        println!("Configuration: {}", path.display());
    }

    println!();
    println!("[library]");
    for p in &config.library.paths {
        println!("  paths = [\"{}\"]", p);
    }
    println!();
    println!("[reader]");
    println!("  command = \"{}\"", config.reader.command);

    Ok(())
}

fn cmd_open(path: &str, page: Option<u32>) -> anyhow::Result<()> {
    let config = Config::load_or_default();
    yazi::integration::open_pdf(path, page, &config.reader.command)?;
    Ok(())
}

fn run_tui() -> anyhow::Result<()> {
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use ratatui::Terminal;
    use ratatui::backend::CrosstermBackend;
    use std::io;

    let config = Config::load_or_default();
    config.ensure_dirs()?;
    let db = open_database()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = tui::app::App::new(&db, &config);
    let events = tui::events::EventHandler::new(std::time::Duration::from_millis(250));

    let result = run_app(&mut terminal, app, &events, &db);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

fn run_app(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    mut app: tui::app::App,
    events: &tui::events::EventHandler,
    db: &Database,
) -> anyhow::Result<()> {
    use tui::app::ActiveTab;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            use tui::layout::main_layout;
            let (header, content, footer) = main_layout(area);

            render_tab_bar(frame, header, &app);

            match app.active_tab {
                ActiveTab::Home => tui::views::home::render(frame, content, &app),
                ActiveTab::Library => tui::views::library::render(frame, content, &app),
                ActiveTab::Search => tui::views::search::render(frame, content, &app),
                ActiveTab::Codex => tui::views::codex::render(frame, content, &app),
                ActiveTab::Oracle => tui::views::oracle::render(frame, content, &app),
                ActiveTab::Stats => tui::views::stats::render(frame, content, &app),
            }

            render_footer(frame, footer, &app);
        })?;

        match events.next()? {
            tui::events::AppEvent::Key(key) => {
                tui::events::handle_key_event(&mut app, key, db);
            }
            tui::events::AppEvent::Tick => {}
        }

        if !app.running {
            break;
        }
    }

    Ok(())
}

fn render_tab_bar(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &tui::app::App) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Paragraph};

    let tabs = [
        tui::app::ActiveTab::Home,
        tui::app::ActiveTab::Library,
        tui::app::ActiveTab::Search,
        tui::app::ActiveTab::Codex,
        tui::app::ActiveTab::Oracle,
        tui::app::ActiveTab::Stats,
    ];

    let spans: Vec<Span> = tabs
        .iter()
        .enumerate()
        .flat_map(|(i, tab)| {
            let is_active = *tab == app.active_tab;
            let style = if is_active {
                tui::theme::Theme::highlight_style()
            } else {
                tui::theme::Theme::dim_style()
            };
            let label = format!(" {}:{} ", tab.key_hint(), tab.label());
            let mut result = vec![Span::styled(label, style)];
            if i < tabs.len() - 1 {
                result.push(Span::styled("│", tui::theme::Theme::dim_style()));
            }
            result
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(tui::theme::Theme::accent_style())
        .title(Span::styled(
            " ᑐ ᑌ ᑎ ᑕ TERMINAL CODEX ",
            tui::theme::Theme::title_style(),
        ));

    let paragraph = Paragraph::new(Line::from(spans))
        .block(block)
        .style(tui::theme::Theme::text_style());

    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &tui::app::App) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let hint = match app.active_tab {
        tui::app::ActiveTab::Home => "  [1-6] tabs  [Enter] next  [q] quit",
        tui::app::ActiveTab::Library => {
            "  [j/k] nav  [PgUp/PgDn] scroll  [Enter] open  [Tab] next  [q] home"
        }
        tui::app::ActiveTab::Search => "  type to search  [Enter] open  [Esc] clear  [q] home",
        tui::app::ActiveTab::Codex => {
            if app.codex_detail.is_some() {
                "  [Esc/q] back  [j/k] scroll"
            } else {
                "  [j/k] nav  [c] category  [Enter] details  [Tab] next  [q] home"
            }
        }
        tui::app::ActiveTab::Oracle => "  [r/Enter] new wisdom  [Tab] next  [q] home",
        tui::app::ActiveTab::Stats => "  [Tab] next  [q] home",
    };

    let line = Line::from(Span::styled(hint, tui::theme::Theme::dim_style()));

    let paragraph = Paragraph::new(line).style(tui::theme::Theme::text_style());
    frame.render_widget(paragraph, area);
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_basic() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(1), "1");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
    }
}
