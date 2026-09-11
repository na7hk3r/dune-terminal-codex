use crate::database::repository::Database;

pub struct OracleEngine<'a> {
    db: &'a Database,
}

impl<'a> OracleEngine<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn random_wisdom(&self) -> anyhow::Result<Option<(String, Option<String>)>> {
        if let Some((text, attribution, _book)) = self.db.random_quote()? {
            return Ok(Some((text, attribution)));
        }

        if let Some((term, definition)) = self.db.random_glossary_entry()? {
            return Ok(Some((format!("{}: {}", term, definition), None)));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:")).unwrap()
    }

    #[test]
    fn quote_only_database_returns_the_quote() {
        let db = test_db();
        db.conn
            .execute(
                "INSERT INTO quotes (text, attribution, book_title) VALUES (?1, ?2, ?3)",
                rusqlite::params!["Fear is the mind-killer.", "Paul Atreides", "Dune"],
            )
            .unwrap();

        let oracle = OracleEngine::new(&db);
        let (wisdom, source) = oracle
            .random_wisdom()
            .unwrap()
            .expect("single quote must produce wisdom");
        assert_eq!(wisdom, "Fear is the mind-killer.");
        assert_eq!(source.as_deref(), Some("Paul Atreides"));
    }

    #[test]
    fn glossary_only_database_returns_term_with_no_source() {
        let db = test_db();
        db.conn
            .execute(
                "INSERT INTO glossary (term, definition) VALUES (?1, ?2)",
                rusqlite::params!["Melange", "The spice of Arrakis"],
            )
            .unwrap();

        let oracle = OracleEngine::new(&db);
        let (wisdom, source) = oracle
            .random_wisdom()
            .unwrap()
            .expect("single glossary entry must produce wisdom");
        assert_eq!(wisdom, "Melange: The spice of Arrakis");
        assert_eq!(source, None);
    }

    #[test]
    fn empty_database_returns_none() {
        let db = test_db();
        let oracle = OracleEngine::new(&db);
        // Prefer the quote source, fall back to glossary, then silence.
        assert!(oracle.random_wisdom().unwrap().is_none());
    }
}
