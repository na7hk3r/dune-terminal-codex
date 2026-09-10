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