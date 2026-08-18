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
}
