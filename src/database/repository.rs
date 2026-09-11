use rusqlite::{Connection, params};
use std::path::Path;

use crate::database::migrations::run_migrations;

/// Describes one of the four codex lookup tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupKind {
    Character,
    House,
    Planet,
    Glossary,
}

impl LookupKind {
    /// Returns `(table_name, name_column, display_sql)` where `display_sql` is
    /// an SQL fragment that produces `"name\n\ndescription"` from the row.
    fn table_info(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            LookupKind::Character => (
                "characters",
                "name",
                "name || '\n\n' || COALESCE(description, 'No description available.')",
            ),
            LookupKind::House => (
                "houses",
                "name",
                "name || '\n\n' || COALESCE(description, 'No description available.')",
            ),
            LookupKind::Planet => (
                "planets",
                "name",
                "name || '\n\n' || COALESCE(description, 'No description available.')",
            ),
            LookupKind::Glossary => (
                "glossary",
                "term",
                "term || '\n\n' || definition",
            ),
        }
    }

    /// The column that holds the display name (either `name` or `term`).
    fn name_column(&self) -> &'static str {
        self.table_info().1
    }
}

pub struct Database {
    pub conn: Connection,
}

/// Keeps only characters the FTS indexes can represent: alphanumerics, spaces
/// and underscores (the same contract `search_fts` has always used).
fn sanitize_query(query: &str) -> String {
    query
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '_')
        .collect()
}

/// Breaks a token into its overlapping 3-character windows, e.g.
/// `"atelides"` -> `["ate", "tel", "eli", "lid", "ide", "des"]`.
/// Tokens shorter than 3 chars yield no trigrams.
fn trigram_terms(token: &str) -> Vec<String> {
    let chars: Vec<char> = token.chars().collect();
    if chars.len() < 3 {
        return Vec::new();
    }
    (0..=chars.len() - 3)
        .map(|i| chars[i..i + 3].iter().collect())
        .collect()
}

impl Database {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_default() -> anyhow::Result<Self> {
        Self::open(&crate::config::settings::Config::db_path()?)
    }

    pub fn book_count(&self) -> anyhow::Result<u64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM books", [], |r| {
                let v: i64 = r.get(0)?;
                Ok(v as u64)
            })
            .unwrap_or(0u64))
    }

    pub fn total_pages(&self) -> anyhow::Result<u64> {
        Ok(self
            .conn
            .query_row("SELECT COALESCE(SUM(page_count), 0) FROM books", [], |r| {
                let v: i64 = r.get(0)?;
                Ok(v as u64)
            })
            .unwrap_or(0u64))
    }

    pub fn total_words(&self) -> anyhow::Result<u64> {
        Ok(self
            .conn
            .query_row("SELECT COALESCE(SUM(word_count), 0) FROM books", [], |r| {
                let v: i64 = r.get(0)?;
                Ok(v as u64)
            })
            .unwrap_or(0u64))
    }

    pub fn search_fts(
        &self,
        query: &str,
        book: Option<&str>,
        limit: u32,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let sanitized = sanitize_query(query);
        let terms: Vec<&str> = sanitized.split_whitespace().collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let fts_query: String = terms
            .iter()
            .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" AND ");

        let mut sql = String::from(
            "SELECT p.book_id, b.title, p.page_number, p.content,
                    highlight(pages_fts, 0, '>>>', '<<<') as highlighted
             FROM pages_fts
             JOIN pages p ON p.id = pages_fts.rowid
             JOIN books b ON b.id = p.book_id
             WHERE pages_fts MATCH ?1",
        );
        if book.is_some() {
            sql.push_str(" AND LOWER(b.title) LIKE ?3");
        }
        sql.push_str(" ORDER BY rank LIMIT ?2");

        let map_row = |row: &rusqlite::Row| {
            Ok(SearchResult {
                book_id: row.get(0)?,
                book_title: row.get(1)?,
                page_number: row.get(2)?,
                content: row.get(3)?,
                highlighted: row.get(4)?,
            })
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let results = match book {
            Some(filter) => {
                let pattern = format!("%{}%", filter.to_lowercase());
                stmt.query_map(params![fts_query, limit, pattern], map_row)?
                    .collect::<Result<Vec<_>, _>>()?
            }
            None => stmt
                .query_map(params![fts_query, limit], map_row)?
                .collect::<Result<Vec<_>, _>>()?,
        };

        Ok(results)
    }

    /// Fuzzy full-text search over the trigram index (migration 13).
    ///
    /// - Queries shorter than 3 characters (or empty after sanitizing) fall
    ///   back to the exact `search_fts` path, since trigrams need 3+ chars.
    /// - Punctuation-only queries sanitize to nothing and return an empty
    ///   vector without error.
    /// - 3+ char queries match the OR of each token's trigrams, so pages that
    ///   share only part of the query (typos, interior substrings) still rank
    ///   via bm25; multi-token queries AND the per-token trigram groups.
    /// - Results merge trigram matches first, then append exact `search_fts`
    ///   hits not already present, deduplicated by `(book_id, page_number)`.
    pub fn fuzzy_search(
        &self,
        query: &str,
        book: Option<&str>,
        limit: u32,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let sanitized = sanitize_query(query);
        let non_space_chars: usize = sanitized.chars().filter(|c| !c.is_whitespace()).count();
        if non_space_chars < 3 {
            return self.search_fts(query, book, limit);
        }

        // Per token: OR its trigrams so partial/typo'd matches still hit.
        // Tokens shorter than 3 chars contribute nothing; if every token is
        // too short the query degenerates to the exact path.
        let mut groups: Vec<String> = Vec::new();
        for token in sanitized.split_whitespace() {
            let trigrams = trigram_terms(token);
            if trigrams.is_empty() {
                continue;
            }
            let group = trigrams
                .iter()
                .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(" OR ");
            groups.push(format!("({group})"));
        }
        if groups.is_empty() {
            return self.search_fts(query, book, limit);
        }
        let fts_query = groups.join(" AND ");

        let mut sql = String::from(
            "SELECT p.book_id, b.title, p.page_number, p.content,
                    highlight(pages_trigrams, 0, '>>>', '<<<') as highlighted
             FROM pages_trigrams
             JOIN pages p ON p.id = pages_trigrams.rowid
             JOIN books b ON b.id = p.book_id
             WHERE pages_trigrams MATCH ?1",
        );
        if book.is_some() {
            sql.push_str(" AND LOWER(b.title) LIKE ?3");
        }
        sql.push_str(" ORDER BY rank LIMIT ?2");

        let map_row = |row: &rusqlite::Row| {
            Ok(SearchResult {
                book_id: row.get(0)?,
                book_title: row.get(1)?,
                page_number: row.get(2)?,
                content: row.get(3)?,
                highlighted: row.get(4)?,
            })
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let trigram_results: Vec<SearchResult> = match book {
            Some(filter) => {
                let pattern = format!("%{}%", filter.to_lowercase());
                stmt.query_map(params![fts_query, limit, pattern], map_row)?
                    .collect::<Result<Vec<_>, _>>()?
            }
            None => stmt
                .query_map(params![fts_query, limit], map_row)?
                .collect::<Result<Vec<_>, _>>()?,
        };

        // Append exact matches not already surfaced by the trigram index,
        // deduplicated by the pages natural key.
        let mut merged = trigram_results;
        let exact = self.search_fts(query, book, limit)?;
        for result in exact {
            let duplicate = merged
                .iter()
                .any(|r| r.book_id == result.book_id && r.page_number == result.page_number);
            if !duplicate {
                merged.push(result);
            }
        }
        merged.truncate(limit as usize);
        Ok(merged)
    }

    /// Resolves a book's file path from its numeric id.
    pub fn book_path_by_id(&self, id: i64) -> anyhow::Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT file_path FROM books WHERE id = ?1",
            params![id],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(path) => Ok(Some(path)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn insert_book(
        &self,
        title: &str,
        path: &str,
        page_count: u32,
        word_count: u32,
        file_size: i64,
    ) -> anyhow::Result<i64> {
        let existing: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM books WHERE file_path = ?1",
                params![path],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            self.conn
                .execute("DELETE FROM pages WHERE book_id = ?1", params![id])?;
            self.conn.execute(
                "UPDATE books SET title = ?1, page_count = ?2, word_count = ?3,
                 file_size = ?4, indexed_at = datetime('now')
                 WHERE id = ?5",
                params![title, page_count, word_count, file_size, id],
            )?;
            Ok(id)
        } else {
            self.conn.execute(
                "INSERT INTO books (title, file_path, page_count, word_count, file_size, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
                params![title, path, page_count, word_count, file_size],
            )?;
            Ok(self.conn.last_insert_rowid())
        }
    }

    pub fn insert_page(&self, book_id: i64, page_number: u32, content: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO pages (book_id, page_number, content) VALUES (?1, ?2, ?3)",
            params![book_id, page_number, content],
        )?;
        Ok(())
    }

    /// Looks up an entity across the four codex tables by `kind`, returning
    /// the `"name\n\ndescription"` display string (or `None` when missing).
    pub fn entity_by_name(
        &self,
        kind: LookupKind,
        name: &str,
    ) -> anyhow::Result<Option<String>> {
        let (table, _name_col, display) = kind.table_info();
        let pattern = format!("%{}%", name.to_lowercase());
        let sql = format!(
            "SELECT {} FROM {} WHERE LOWER({}) LIKE ?1 LIMIT 1",
            display,
            table,
            kind.name_column()
        );
        let result = self.conn.query_row(&sql, params![pattern], |row| {
            row.get::<_, String>(0)
        });
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Lists all display names for a kind, sorted alphabetically.
    pub fn list_names(&self, kind: LookupKind) -> anyhow::Result<Vec<String>> {
        let (table, _name_col, _display) = kind.table_info();
        let sql = format!(
            "SELECT {} FROM {} ORDER BY {}",
            kind.name_column(),
            table,
            kind.name_column()
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let names = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(names)
    }

    pub fn character_by_name(&self, name: &str) -> anyhow::Result<Option<String>> {
        self.entity_by_name(LookupKind::Character, name)
    }

    pub fn house_by_name(&self, name: &str) -> anyhow::Result<Option<String>> {
        self.entity_by_name(LookupKind::House, name)
    }

    pub fn planet_by_name(&self, name: &str) -> anyhow::Result<Option<String>> {
        self.entity_by_name(LookupKind::Planet, name)
    }

    pub fn glossary_by_term(&self, term: &str) -> anyhow::Result<Option<String>> {
        self.entity_by_name(LookupKind::Glossary, term)
    }

    pub fn glossary_list(&self, limit: u32) -> anyhow::Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT term, definition FROM glossary ORDER BY term LIMIT ?1")?;
        let results = stmt
            .query_map(params![limit], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }

    pub fn random_quote(&self) -> anyhow::Result<Option<QuoteRow>> {
        let result = self.conn.query_row(
            "SELECT text, attribution, book_title FROM quotes ORDER BY RANDOM() LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn random_glossary_entry(&self) -> anyhow::Result<Option<(String, String)>> {
        let result = self.conn.query_row(
            "SELECT term, definition FROM glossary ORDER BY RANDOM() LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Debug)]
pub struct SearchResult {
    pub book_id: i64,
    pub book_title: String,
    pub page_number: u32,
    #[allow(dead_code)]
    pub content: String,
    pub highlighted: String,
}

/// `(text, attribution, book_title)` as returned by `random_quote`.
pub type QuoteRow = (String, Option<String>, Option<String>);

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:")).unwrap()
    }

    #[test]
    fn insert_and_count_book() {
        let db = test_db();
        db.insert_book("Dune", "/tmp/dune.pdf", 412, 200000, 1024000)
            .unwrap();
        assert_eq!(db.book_count().unwrap(), 1);
    }

    #[test]
    fn search_fts_works() {
        let db = test_db();
        let book_id = db
            .insert_book("Dune", "/tmp/dune.pdf", 412, 200000, 1024000)
            .unwrap();
        db.insert_page(book_id, 1, "The spice must flow").unwrap();
        db.insert_page(book_id, 2, "Fear is the mind-killer")
            .unwrap();
        let results = db.search_fts("spice", None, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("spice"));
    }

    #[test]
    fn search_fts_filters_by_book() {
        let db = test_db();
        let dune = db.insert_book("Dune", "/tmp/dune.pdf", 10, 100, 1).unwrap();
        let messiah = db
            .insert_book("Dune Messiah", "/tmp/messiah.pdf", 10, 100, 1)
            .unwrap();
        db.insert_page(dune, 1, "the spice must flow").unwrap();
        db.insert_page(messiah, 1, "the spice must flow").unwrap();

        // No filter: both books match.
        assert_eq!(db.search_fts("spice", None, 10).unwrap().len(), 2);

        // Partial, case-insensitive filter keeps only the matching book.
        let filtered = db.search_fts("spice", Some("messiah"), 10).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].book_title, "Dune Messiah");

        // A filter that matches nothing returns no results.
        assert!(
            db.search_fts("spice", Some("god emperor"), 10)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn book_path_by_id_resolves_and_misses_gracefully() {
        let db = test_db();
        let id = db
            .insert_book("Dune", "/tmp/dune.pdf", 412, 200000, 1024000)
            .unwrap();
        assert_eq!(
            db.book_path_by_id(id).unwrap(),
            Some("/tmp/dune.pdf".to_string())
        );
        assert_eq!(db.book_path_by_id(9999).unwrap(), None);
    }

    #[test]
    fn search_fts_ranks_by_bm25_relevance() {
        let db = test_db();
        // Insert sparse book first so ids would put it first if sorted by id.
        let sparse = db.insert_book("A Sparse Book", "/tmp/sparse.pdf", 10, 100, 1)
            .unwrap();
        let dense = db.insert_book("B Dense Book", "/tmp/dense.pdf", 10, 100, 1)
            .unwrap();

        db.insert_page(sparse, 1, "the spice must flow once").unwrap();
        db.insert_page(
            dense,
            1,
            "spice spice spice the spice melange is the spice of arrakis",
        )
        .unwrap();

        let results = db.search_fts("spice", None, 10).unwrap();
        assert!(results.len() >= 2);
        // bm25: denser match ranks first regardless of insertion order.
        assert_eq!(
            results[0].book_title, "B Dense Book",
            "expected dense match first, got order: {:?}",
            results.iter().map(|r| &r.book_title).collect::<Vec<_>>()
        );
        // Highlight markers stay intact.
        assert!(results[0].highlighted.contains(">>>spice<<<"));
    }

    // --- T7: LookupKind dedup tests ---

    fn seed_codex_data(db: &Database) {
        db.conn
            .execute_batch(
                "INSERT INTO characters (name, description) VALUES
                    ('Paul Atreides', 'Duke of Arrakis'),
                    ('Leto Atreides', 'Father of Paul');
                 INSERT INTO houses (name, description) VALUES
                    ('Atreides', 'Noble house of Caladan');
                 INSERT INTO planets (name, description) VALUES
                    ('Arrakis', 'Desert planet, source of spice');
                 INSERT INTO glossary (term, definition) VALUES
                    ('Melange', 'The spice of Arrakis');",
            )
            .unwrap();
    }

    #[test]
    fn entity_by_name_returns_description_for_each_kind() {
        let db = test_db();
        seed_codex_data(&db);

        let result = db
            .entity_by_name(LookupKind::Character, "Paul")
            .unwrap();
        assert_eq!(result, Some("Paul Atreides\n\nDuke of Arrakis".into()));

        let result = db
            .entity_by_name(LookupKind::House, "Atreides")
            .unwrap();
        assert_eq!(
            result,
            Some("Atreides\n\nNoble house of Caladan".into())
        );

        let result = db
            .entity_by_name(LookupKind::Planet, "Arrakis")
            .unwrap();
        assert_eq!(result, Some("Arrakis\n\nDesert planet, source of spice".into()));

        let result = db
            .entity_by_name(LookupKind::Glossary, "Melange")
            .unwrap();
        assert_eq!(result, Some("Melange\n\nThe spice of Arrakis".into()));
    }

    #[test]
    fn entity_by_name_returns_none_for_missing() {
        let db = test_db();
        seed_codex_data(&db);

        assert!(db
            .entity_by_name(LookupKind::Character, "Nonexistent")
            .unwrap()
            .is_none());
        assert!(db
            .entity_by_name(LookupKind::House, "Nonexistent")
            .unwrap()
            .is_none());
        assert!(db
            .entity_by_name(LookupKind::Planet, "Nonexistent")
            .unwrap()
            .is_none());
        assert!(db
            .entity_by_name(LookupKind::Glossary, "Nonexistent")
            .unwrap()
            .is_none());
    }

    #[test]
    fn list_names_returns_sorted_names_per_kind() {
        let db = test_db();
        seed_codex_data(&db);

        let chars = db.list_names(LookupKind::Character).unwrap();
        assert_eq!(chars, vec!["Leto Atreides", "Paul Atreides"]);

        let houses = db.list_names(LookupKind::House).unwrap();
        assert_eq!(houses, vec!["Atreides"]);

        let planets = db.list_names(LookupKind::Planet).unwrap();
        assert_eq!(planets, vec!["Arrakis"]);

        let glossary = db.list_names(LookupKind::Glossary).unwrap();
        assert_eq!(glossary, vec!["Melange"]);
    }

    #[test]
    fn list_names_empty_table_returns_empty_vec() {
        let db = test_db();
        // No codex data inserted.
        let names = db.list_names(LookupKind::Character).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn legacy_delegates_match_generic_entity_by_name() {
        let db = test_db();
        seed_codex_data(&db);

        // character_by_name delegates to entity_by_name(Character, …)
        assert_eq!(
            db.character_by_name("Paul").unwrap(),
            db.entity_by_name(LookupKind::Character, "Paul").unwrap()
        );
        assert_eq!(
            db.character_by_name("Nonexistent").unwrap(),
            None
        );
        // house_by_name
        assert_eq!(
            db.house_by_name("Atreides").unwrap(),
            db.entity_by_name(LookupKind::House, "Atreides").unwrap()
        );
        // planet_by_name
        assert_eq!(
            db.planet_by_name("Arrakis").unwrap(),
            db.entity_by_name(LookupKind::Planet, "Arrakis").unwrap()
        );
        // glossary_by_term
        assert_eq!(
            db.glossary_by_term("Melange").unwrap(),
            db.entity_by_name(LookupKind::Glossary, "Melange").unwrap()
        );
    }

    #[test]
    fn source_url_round_trips_through_each_codex_table() {
        let db = test_db();

        db.conn
            .execute(
                "INSERT INTO characters (name, description, source, source_url)
                 VALUES (?1, ?2, 'wiki', ?3)",
                rusqlite::params![
                    "Paul Atreides",
                    "Duke of Arrakis",
                    "https://dune.fandom.com/wiki/Paul_Atreides"
                ],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO houses (name, description, source, source_url)
                 VALUES (?1, ?2, 'wiki', ?3)",
                rusqlite::params![
                    "Atreides",
                    "Noble house",
                    "https://dune.fandom.com/wiki/House_Atreides"
                ],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO planets (name, description, source, source_url)
                 VALUES (?1, ?2, 'wiki', ?3)",
                rusqlite::params![
                    "Arrakis",
                    "Desert planet",
                    "https://dune.fandom.com/wiki/Arrakis"
                ],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO glossary (term, definition, source, source_url)
                 VALUES (?1, ?2, 'wiki', ?3)",
                rusqlite::params![
                    "Melange",
                    "The spice",
                    "https://dune.fandom.com/wiki/Melange"
                ],
            )
            .unwrap();

        let roundtrip = |sql: &str, name: &str| -> Option<String> {
            db.conn
                .query_row(sql, [name], |row| row.get::<_, String>(0))
                .ok()
        };

        assert_eq!(
            roundtrip(
                "SELECT source_url FROM characters WHERE name = ?1",
                "Paul Atreides"
            )
            .as_deref(),
            Some("https://dune.fandom.com/wiki/Paul_Atreides")
        );
        assert_eq!(
            roundtrip("SELECT source_url FROM houses WHERE name = ?1", "Atreides")
                .as_deref(),
            Some("https://dune.fandom.com/wiki/House_Atreides")
        );
        assert_eq!(
            roundtrip("SELECT source_url FROM planets WHERE name = ?1", "Arrakis")
                .as_deref(),
            Some("https://dune.fandom.com/wiki/Arrakis")
        );
        assert_eq!(
            roundtrip("SELECT source_url FROM glossary WHERE term = ?1", "Melange")
                .as_deref(),
            Some("https://dune.fandom.com/wiki/Melange")
        );

        // The URL also survives a re-import (INSERT OR REPLACE upsert).
        db.conn
            .execute(
                "INSERT OR REPLACE INTO characters (name, description, source, source_url)
                 VALUES (?1, ?2, 'wiki', ?3)",
                rusqlite::params![
                    "Paul Atreides",
                    "Duke of Arrakis (updated)",
                    "https://dune.fandom.com/wiki/Paul_Atreides"
                ],
            )
            .unwrap();
        let (desc, url): (String, String) = db
            .conn
            .query_row(
                "SELECT description, source_url FROM characters WHERE name = ?1",
                ["Paul Atreides"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(desc, "Duke of Arrakis (updated)");
        assert_eq!(url, "https://dune.fandom.com/wiki/Paul_Atreides");
    }

    // --- T12: trigram fuzzy search tests ---

    fn seed_fuzzy_pages(db: &Database) {
        let dune = db
            .insert_book("Dune", "/tmp/dune.pdf", 412, 200000, 1024000)
            .unwrap();
        let desert = db
            .insert_book("Desert Lore", "/tmp/desert.pdf", 100, 50000, 512000)
            .unwrap();
        let other = db
            .insert_book("Unrelated", "/tmp/other.pdf", 50, 25000, 256000)
            .unwrap();
        // Atreides shares "ide"+"des" with a typo'd query "atelides";
        // desert shares only "des"; unrelated shares none.
        db.insert_page(
            dune,
            1,
            "The Atreides family rules Arrakis and the spice must flow",
        )
        .unwrap();
        db.insert_page(desert, 1, "desert dwellers of the deep desert")
            .unwrap();
        db.insert_page(other, 1, "quiet library shelves under starlight")
            .unwrap();
    }

    #[test]
    fn fuzzy_search_tolerates_typos_via_trigram_overlap() {
        let db = test_db();
        seed_fuzzy_pages(&db);

        let results = db.fuzzy_search("atelides", None, 10).unwrap();
        assert!(!results.is_empty(), "typo'd query must still find pages");

        let atreides_idx = results
            .iter()
            .position(|r| r.content.contains("Atreides"))
            .expect("fuzzy search must surface the Atreides page");
        let desert_idx = results
            .iter()
            .position(|r| r.content.contains("desert"))
            .expect("fuzzy search must also surface the partial trigram page");
        assert!(
            atreides_idx < desert_idx,
            "page sharing more trigrams must rank first, got order: {:?}",
            results.iter().map(|r| &r.book_title).collect::<Vec<_>>()
        );
    }

    #[test]
    fn fuzzy_search_falls_back_to_exact_search_for_short_queries() {
        let db = test_db();
        seed_fuzzy_pages(&db);

        // A query shorter than 3 chars cannot use trigrams: the result must be
        // bit-identical to the existing exact search path.
        assert_eq!(
            db.fuzzy_search("sp", None, 10).unwrap()
                .iter()
                .map(|r| (r.book_title.clone(), r.page_number))
                .collect::<Vec<_>>(),
            db.search_fts("sp", None, 10).unwrap()
                .iter()
                .map(|r| (r.book_title.clone(), r.page_number))
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn fuzzy_search_punctuation_only_returns_empty_without_error() {
        let db = test_db();
        seed_fuzzy_pages(&db);

        let results = db.fuzzy_search("!!!  ???", None, 10).unwrap();
        assert!(results.is_empty(), "punctuation-only query must be empty");
    }

    #[test]
    fn fuzzy_search_respects_book_filter() {
        let db = test_db();
        let dune = db
            .insert_book("Dune", "/tmp/dune.pdf", 412, 200000, 1024000)
            .unwrap();
        let messiah = db
            .insert_book("Dune Messiah", "/tmp/messiah.pdf", 300, 150000, 768000)
            .unwrap();
        db.insert_page(dune, 1, "The Atreides family rules Arrakis")
            .unwrap();
        db.insert_page(messiah, 1, "Paul Atreides arrives on Dune")
            .unwrap();

        let filtered = db
            .fuzzy_search("atreides", Some("messiah"), 10)
            .unwrap();
        assert!(!filtered.is_empty());
        assert!(
            filtered.iter().all(|r| r.book_title == "Dune Messiah"),
            "book filter must keep only the matching book: {:?}",
            filtered.iter().map(|r| &r.book_title).collect::<Vec<_>>()
        );
    }

    #[test]
    fn fuzzy_search_merges_and_dedupes_against_exact_results() {
        let db = test_db();
        let dune = db
            .insert_book("Dune", "/tmp/dune.pdf", 412, 200000, 1024000)
            .unwrap();
        db.insert_page(
            dune,
            1,
            "the spice must flow, the spice extends consciousness",
        )
        .unwrap();
        db.insert_page(dune, 2, "spice melange is rare and precious")
            .unwrap();

        // "spice" is an exact unicode61 token AND a trigram substring: both
        // paths match, so the merge must not duplicate any page.
        let results = db.fuzzy_search("spice", None, 10).unwrap();
        assert!(!results.is_empty());
        let key = |r: &SearchResult| (r.book_id, r.page_number);
        let mut keys: Vec<(i64, u32)> = results.iter().map(key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(
            keys.len(),
            results.len(),
            "merged results must not contain duplicate pages"
        );
        assert_eq!(keys.len(), 2, "both spice pages must be found once each");
    }

    #[test]
    fn fuzzy_search_requires_all_tokens_of_multi_token_query() {
        let db = test_db();
        let dune = db
            .insert_book("Dune", "/tmp/dune.pdf", 412, 200000, 1024000)
            .unwrap();
        let sand = db
            .insert_book("Sand Chronicles", "/tmp/sand.pdf", 80, 40000, 409600)
            .unwrap();
        let spicebook = db
            .insert_book("Spice Tales", "/tmp/spicetal.pdf", 60, 30000, 307200)
            .unwrap();
        db.insert_page(
            dune,
            1,
            "The Atreides family and the spice must flow",
        )
        .unwrap();
        db.insert_page(sand, 1, "Atreides honor and duty").unwrap();
        db.insert_page(spicebook, 1, "spice melange is life").unwrap();

        let results = db.fuzzy_search("atreides spice", None, 10).unwrap();
        assert_eq!(results.len(), 1, "both tokens must appear in the match");
        assert!(results[0].content.contains("Atreides"));
        assert!(results[0].content.contains("spice"));
    }

    #[test]
    fn fuzzy_search_finds_partial_substrings() {
        let db = test_db();
        let dune = db
            .insert_book("Dune", "/tmp/dune.pdf", 412, 200000, 1024000)
            .unwrap();
        db.insert_page(dune, 1, "The Atreides family rules Arrakis")
            .unwrap();

        // "treid" is a non-prefix interior substring of "Atreides".
        let results = db.fuzzy_search("treid", None, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Atreides"));
    }
}
