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
}
