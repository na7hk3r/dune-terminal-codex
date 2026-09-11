use rusqlite::Connection;
use tracing::info;

use crate::database::schema::{ALL_MIGRATIONS, CREATE_MIGRATIONS};

pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(CREATE_MIGRATIONS)?;

    let current: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total = ALL_MIGRATIONS.len() as u32;

    if current >= total {
        return Ok(());
    }

    for (version, sql) in ALL_MIGRATIONS.iter() {
        let v: u32 = version.parse()?;
        if v > current {
            info!("applying migration {}", v);
            conn.execute_batch(sql)?;
            conn.execute("INSERT INTO migrations (version) VALUES (?1)", [v])?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrations_run_on_empty_database() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        let version: u32 = conn
            .query_row("SELECT MAX(version) FROM migrations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, ALL_MIGRATIONS.len() as u32);
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM migrations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, ALL_MIGRATIONS.len() as u32);
    }

    #[test]
    fn migration_11_adds_source_url_and_backfills_character_aliases() {
        let conn = Connection::open_in_memory().unwrap();

        // Simulate a pre-migration-11 database: characters exists without
        // source_url and holds legacy wiki URLs in the aliases column.
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
            INSERT INTO characters (name, aliases) VALUES
                ('Paul Atreides', 'https://dune.fandom.com/wiki/Paul_Atreides'),
                ('Vladimir Harkonnen', 'The Baron');",
        )
        .unwrap();

        run_migrations(&conn).unwrap();

        let has_column = |table: &str| -> bool {
            conn.prepare(&format!("PRAGMA table_info({})", table))
                .unwrap()
                .query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .filter_map(Result::ok)
                .any(|name| name == "source_url")
        };
        assert!(has_column("characters"), "characters lacks source_url");
        assert!(has_column("houses"), "houses lacks source_url");
        assert!(has_column("planets"), "planets lacks source_url");
        assert!(has_column("glossary"), "glossary lacks source_url");

        let (url, aliases): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT source_url, aliases FROM characters WHERE name = ?1",
                ["Paul Atreides"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            url.as_deref(),
            Some("https://dune.fandom.com/wiki/Paul_Atreides"),
            "backfill must move the wiki URL from aliases into source_url"
        );
        assert_eq!(aliases, None, "backfill must clear the legacy aliases column");

        let (url, aliases): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT source_url, aliases FROM characters WHERE name = ?1",
                ["Vladimir Harkonnen"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(url, None, "non-URL aliases must not be backfilled");
        assert_eq!(aliases.as_deref(), Some("The Baron"));
    }

    fn table_columns(conn: &Connection, table: &str) -> Vec<String> {
        conn.prepare(&format!("PRAGMA table_info({})", table))
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    }

    #[test]
    fn migration_12_drops_dead_columns_and_chapters_table() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // chapters table is gone entirely.
        let chapters_exist: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='chapters'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            > 0;
        assert!(!chapters_exist, "chapters table must be dropped by migration 12");

        // Dead columns are gone from each codex table.
        let character_cols = table_columns(&conn, "characters");
        for dead in ["house", "related", "books"] {
            assert!(
                !character_cols.iter().any(|c| c == dead),
                "characters.{dead} must be dropped by migration 12, got columns {character_cols:?}"
            );
        }
        let house_cols = table_columns(&conn, "houses");
        for dead in ["homeworld", "notable_members"] {
            assert!(!house_cols.iter().any(|c| c == dead));
        }
        let planet_cols = table_columns(&conn, "planets");
        for dead in ["system", "notable_features"] {
            assert!(!planet_cols.iter().any(|c| c == dead));
        }
        let glossary_cols = table_columns(&conn, "glossary");
        for dead in ["category", "origin"] {
            assert!(!glossary_cols.iter().any(|c| c == dead));
        }
        let quote_cols = table_columns(&conn, "quotes");
        for dead in ["chapter", "page_number"] {
            assert!(!quote_cols.iter().any(|c| c == dead));
        }

        // Living columns survive: source_url (migration 11) and the display/source columns.
        for col in ["name", "description", "source", "source_url"] {
            assert!(character_cols.iter().any(|c| c == col));
        }
        for col in ["name", "description", "source", "source_url"] {
            assert!(house_cols.iter().any(|c| c == col));
        }
        for col in ["term", "definition", "source", "source_url"] {
            assert!(glossary_cols.iter().any(|c| c == col));
        }
        for col in ["text", "attribution", "book_title", "source"] {
            assert!(quote_cols.iter().any(|c| c == col));
        }
    }

    #[test]
    fn migration_13_creates_trigram_index_and_backfills_existing_pages() {
        let conn = Connection::open_in_memory().unwrap();

        // Simulate a pre-migration-13 database: pages exist before the trigram
        // index is created, so the migration's backfill must populate it.
        conn.execute_batch(
            "CREATE TABLE pages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                book_id INTEGER NOT NULL,
                page_number INTEGER NOT NULL,
                content TEXT NOT NULL,
                UNIQUE(book_id, page_number)
            );
            INSERT INTO pages (book_id, page_number, content) VALUES
                (1, 1, 'The Atreides family rules Arrakis');",
        )
        .unwrap();

        run_migrations(&conn).unwrap();

        let trigram_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='pages_trigrams'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            > 0;
        assert!(trigram_exists, "migration 13 must create pages_trigrams");

        let indexed: i64 = conn
            .query_row("SELECT COUNT(*) FROM pages_trigrams", [], |row| row.get(0))
            .unwrap();
        let pages: i64 = conn
            .query_row("SELECT COUNT(*) FROM pages", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            indexed, pages,
            "backfill must index every pre-existing page"
        );

        // Substring semantics: an interior substring of the content matches.
        let hit: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pages_trigrams WHERE pages_trigrams MATCH '\"treid\"'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(hit, 1, "trigram index must support substring matching");
    }

    #[test]
    fn migration_13_triggers_keep_trigram_rows_in_sync() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // pages_trigrams_ai indexes new pages.
        conn.execute_batch(
            "INSERT INTO books (title, file_path, page_count, word_count)
             VALUES ('Dune', '/tmp/dune.pdf', 10, 100);
             INSERT INTO pages (book_id, page_number, content) VALUES (1, 1, 'the spice must flow')",
        )
        .unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pages_trigrams", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "pages_trigrams_ai must index inserts");

        // pages_trigrams_au replaces stale trigrams with the updated content.
        conn.execute(
            "UPDATE pages SET content = 'The Atreides family' WHERE id = 1",
            [],
        )
        .unwrap();
        let new_hit: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pages_trigrams WHERE pages_trigrams MATCH '\"treid\"'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            new_hit, 1,
            "pages_trigrams_au must index the updated content"
        );
        let old_hit: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pages_trigrams WHERE pages_trigrams MATCH '\"spice\"'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            old_hit, 0,
            "pages_trigrams_au must drop the stale trigrams"
        );

        // pages_trigrams_ad removes deleted pages entirely.
        conn.execute("DELETE FROM pages WHERE id = 1", []).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pages_trigrams", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "pages_trigrams_ad must remove deleted pages");
    }

    #[test]
    fn migration_12_columns_drop_idempotently_on_rerun() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        // Second run must not re-apply or fail; version count stays at the total.
        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM migrations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, ALL_MIGRATIONS.len() as u32);

        // The dropped schema state is stable across both runs.
        let chapters_exist: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='chapters'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            > 0;
        assert!(!chapters_exist);
    }
}
