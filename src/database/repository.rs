use rusqlite::{Connection, params};
use std::path::Path;

use crate::database::migrations::run_migrations;

pub struct Database {
    pub conn: Connection,
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
        let sanitized: String = query
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '_')
            .collect();
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

    pub fn character_by_name(&self, name: &str) -> anyhow::Result<Option<String>> {
        let pattern = format!("%{}%", name.to_lowercase());
        let result = self.conn.query_row(
            "SELECT name || '\n\n' || COALESCE(description, 'No description available.')
             FROM characters WHERE LOWER(name) LIKE ?1 LIMIT 1",
            params![pattern],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn house_by_name(&self, name: &str) -> anyhow::Result<Option<String>> {
        let pattern = format!("%{}%", name.to_lowercase());
        let result = self.conn.query_row(
            "SELECT name || '\n\n' || COALESCE(description, 'No description available.')
             FROM houses WHERE LOWER(name) LIKE ?1 LIMIT 1",
            params![pattern],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn planet_by_name(&self, name: &str) -> anyhow::Result<Option<String>> {
        let pattern = format!("%{}%", name.to_lowercase());
        let result = self.conn.query_row(
            "SELECT name || '\n\n' || COALESCE(description, 'No description available.')
             FROM planets WHERE LOWER(name) LIKE ?1 LIMIT 1",
            params![pattern],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn glossary_by_term(&self, term: &str) -> anyhow::Result<Option<String>> {
        let pattern = format!("%{}%", term.to_lowercase());
        let result = self.conn.query_row(
            "SELECT term || '\n\n' || definition
             FROM glossary WHERE LOWER(term) LIKE ?1 LIMIT 1",
            params![pattern],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
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
}
