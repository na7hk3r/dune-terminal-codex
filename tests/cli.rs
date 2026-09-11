//! End-to-end CLI tests. The binary is launched against a hermetic
//! `XDG_DATA_HOME`/`HOME` so it never touches the developer's real database.

use std::path::Path;
use std::process::Command;

/// Seeds a scratch database at `$XDG_DATA_HOME/dune/dune.db` with one book
/// whose page contains many "spice" matches, using the same schema the
/// binary creates through its own migrations (books, pages, FTS5 index and
/// sync triggers). The triggers must exist before inserting so the pages
/// are indexed.
fn seed_search_db(root: &Path) {
    let db_path = root.join("dune").join("dune.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute_batch(
        "CREATE TABLE books (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            file_path TEXT NOT NULL UNIQUE,
            file_hash TEXT,
            page_count INTEGER DEFAULT 0,
            word_count INTEGER DEFAULT 0,
            file_size INTEGER DEFAULT 0,
            indexed_at TEXT,
            created_at TEXT DEFAULT (datetime('now'))
        );
        CREATE TABLE pages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
            page_number INTEGER NOT NULL,
            content TEXT NOT NULL,
            UNIQUE(book_id, page_number)
        );
        CREATE VIRTUAL TABLE pages_fts USING fts5(
            content, content='pages', content_rowid='id', tokenize='unicode61'
        );
        CREATE TRIGGER pages_ai AFTER INSERT ON pages BEGIN
            INSERT INTO pages_fts(rowid, content) VALUES (new.id, new.content);
        END;
        CREATE TRIGGER pages_ad AFTER DELETE ON pages BEGIN
            INSERT INTO pages_fts(pages_fts, rowid, content) VALUES('delete', old.id, old.content);
        END;
        CREATE TRIGGER pages_au AFTER UPDATE ON pages BEGIN
            INSERT INTO pages_fts(pages_fts, rowid, content) VALUES('delete', old.id, old.content);
            INSERT INTO pages_fts(rowid, content) VALUES (new.id, new.content);
        END;
        INSERT INTO books (title, file_path, page_count, word_count)
            VALUES ('Dune', '/tmp/dune.pdf', 10, 1000);
        INSERT INTO pages (book_id, page_number, content) VALUES (
            1, 1,
            'spice spice spice the spice melange is the spice of arrakis and the spice must
             flow across a very long horizon indeed for the spice is life itself and the spice
             extends consciousness and the spice is central to the entire story of dune'
        );",
    )
    .unwrap();
}

#[test]
fn cli_search_output_has_no_highlight_markers() {
    let tmp = tempfile::TempDir::new().unwrap();
    seed_search_db(tmp.path());

    let output = Command::new(env!("CARGO_BIN_EXE_dune"))
        .args(["search", "spice"])
        .env("XDG_DATA_HOME", tmp.path())
        .env("HOME", tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "search failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("spice"), "expected match text in output: {stdout:?}");
    assert!(
        !stdout.contains('>'),
        "CLI search output leaks '>' marker chars: {stdout:?}"
    );
    assert!(
        !stdout.contains('<'),
        "CLI search output leaks '<' marker chars: {stdout:?}"
    );
}

/// Seeds the codex tables (characters/houses/planets/glossary/quotes) into the
/// scratch database created by `seed_search_db`, in their PRE-migration-11/12
/// state (dead columns present, no source_url) so the binary's own migrations
/// perform the column adds/drops exactly like a legacy database would.
fn seed_codex_tables(root: &Path) {
    let db_path = root.join("dune").join("dune.db");
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute_batch(
        "CREATE TABLE characters (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            aliases TEXT,
            house TEXT,
            description TEXT,
            related TEXT,
            books TEXT,
            source TEXT DEFAULT 'builtin'
        );
        INSERT INTO characters (name, description) VALUES
            ('Paul Atreides', 'Duke of Arrakis'),
            ('Leto Atreides', 'Father of Paul');
        CREATE TABLE houses (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            homeworld TEXT,
            notable_members TEXT,
            source TEXT DEFAULT 'builtin'
        );
        INSERT INTO houses (name, description) VALUES
            ('Atreides', 'Noble house of Caladan');
        CREATE TABLE planets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            system TEXT,
            notable_features TEXT,
            source TEXT DEFAULT 'builtin'
        );
        INSERT INTO planets (name, description) VALUES
            ('Arrakis', 'Desert planet, source of spice');
        CREATE TABLE glossary (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            term TEXT NOT NULL UNIQUE,
            definition TEXT NOT NULL,
            category TEXT,
            origin TEXT,
            source TEXT DEFAULT 'builtin'
        );
        INSERT INTO glossary (term, definition) VALUES
            ('Melange', 'The spice of Arrakis');
        CREATE TABLE quotes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            attribution TEXT,
            book_title TEXT,
            chapter TEXT,
            page_number INTEGER,
            source TEXT DEFAULT 'builtin'
        );
        INSERT INTO quotes (text, attribution, book_title) VALUES
            ('Fear is the mind-killer.', 'Paul Atreides', 'Dune');",
    )
    .unwrap();
}

/// Runs `dune <args...>` against a hermetic database.
///
/// `db_root` holds the seeded database at `db_root/dune/dune.db` and is passed
/// via `DUNE_DB_PATH`; `xdg_root` is a decoy XDG_DATA_HOME that must NOT be
/// used. Returns the captured output.
fn run_hermetic(db_root: &Path, xdg_root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_dune"))
        .args(args)
        .env("DUNE_DB_PATH", db_root.join("dune").join("dune.db"))
        .env("XDG_DATA_HOME", xdg_root)
        .env("HOME", db_root)
        .output()
        .unwrap()
}

fn run_ok(args: &[&str]) -> String {
    let tmp = tempfile::TempDir::new().unwrap();
    let xdg = tempfile::TempDir::new().unwrap();
    seed_search_db(tmp.path());
    seed_codex_tables(tmp.path());
    let output = run_hermetic(tmp.path(), xdg.path(), args);
    assert!(
        output.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn cli_books_lists_indexed_books_from_dune_db_path() {
    // The decoy XDG_DATA_HOME is empty: only DUNE_DB_PATH can provide data.
    let tmp = tempfile::TempDir::new().unwrap();
    let xdg = tempfile::TempDir::new().unwrap();
    seed_search_db(tmp.path());

    let output = run_hermetic(tmp.path(), xdg.path(), &["books"]);
    assert!(
        output.status.success(),
        "books failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Dune"),
        "books must list the seeded title via DUNE_DB_PATH: {stdout:?}"
    );
    assert!(stdout.contains("1 books indexed"), "books count: {stdout:?}");
}

#[test]
fn cli_codex_commands_print_entries() {
    let stdout = run_ok(&["character", "Paul"]);
    assert!(stdout.contains("Paul Atreides"), "{stdout:?}");
    assert!(stdout.contains("Duke of Arrakis"), "{stdout:?}");

    let stdout = run_ok(&["house", "Atreides"]);
    assert!(stdout.contains("Noble house of Caladan"), "{stdout:?}");

    let stdout = run_ok(&["planet", "Arrakis"]);
    assert!(stdout.contains("Desert planet"), "{stdout:?}");

    let stdout = run_ok(&["glossary", "Melange"]);
    assert!(stdout.contains("The spice of Arrakis"), "{stdout:?}");
}

#[test]
fn cli_codex_missing_entries_exit_zero_with_hint() {
    let stdout = run_ok(&["character", "Nonexistent"]);
    assert!(stdout.contains("No character found"), "{stdout:?}");
    assert!(stdout.contains("import-codex"), "{stdout:?}");
}

#[test]
fn cli_quote_and_oracle_have_output() {
    let stdout = run_ok(&["quote"]);
    assert!(stdout.contains("Fear is the mind-killer."), "{stdout:?}");

    let oracle = run_ok(&["oracle"]);
    assert!(
        oracle.contains("Fear is the mind-killer.") || oracle.contains("Melange"),
        "oracle must quote the seeded quote or glossary entry: {oracle:?}"
    );
}

#[test]
fn cli_empty_database_commands_exit_zero_gracefully() {
    // DUNE_DB_PATH points at a path that does not exist yet: the binary
    // creates a fresh empty database through its own migrations.
    let tmp = tempfile::TempDir::new().unwrap();
    let xdg = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("fresh").join("dune.db");

    for args in [
        &["books"][..],
        &["character", "Paul"][..],
        &["quote"][..],
        &["oracle"][..],
        &["glossary"][..],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_dune"))
            .args(args)
            .env("DUNE_DB_PATH", &db_path)
            .env("XDG_DATA_HOME", xdg.path())
            .env("HOME", tmp.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?} must exit 0 on an empty database: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.trim().is_empty(),
            "{args:?} must print a graceful message on an empty database"
        );
    }

    // The schema was created by migrations on the fresh path: the four codex
    // lookups and the quotes table all exist.
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let table: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='characters'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(table, 1, "fresh DUNE_DB_PATH must be migrated to current schema");
}