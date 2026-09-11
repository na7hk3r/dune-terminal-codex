//! Markdown and JSON export of the library and codex.
//!
//! Typed accessors live on [`Database`] so the renderers stay pure and
//! unit-testable; `collect_export_data` assembles the shared [`ExportData`]
//! shape that both renderers consume.

use serde::Serialize;

use crate::database::repository::Database;

/// A serializable snapshot of the whole library + codex, one section per
/// table. This is the JSON schema of `dune export --format json`.
#[derive(Debug, Serialize)]
pub struct ExportData {
    pub books: Vec<BookExport>,
    pub characters: Vec<EntityExport>,
    pub houses: Vec<EntityExport>,
    pub planets: Vec<EntityExport>,
    pub glossary: Vec<GlossaryExport>,
    pub quotes: Vec<QuoteExport>,
}

#[derive(Debug, Serialize)]
pub struct BookExport {
    pub title: String,
    pub file_path: String,
    pub page_count: u32,
    pub word_count: u32,
}

/// `name`/`description`/`source`/`source_url` shape shared by characters,
/// houses and planets.
#[derive(Debug, Serialize)]
pub struct EntityExport {
    pub name: String,
    pub description: Option<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
}

/// Glossary entries use `term`/`definition` in the JSON schema.
#[derive(Debug, Serialize)]
pub struct GlossaryExport {
    pub term: String,
    pub definition: Option<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QuoteExport {
    pub text: String,
    pub attribution: Option<String>,
    pub book_title: Option<String>,
    pub source: Option<String>,
}

impl Database {
    /// Every indexed book, sorted by title.
    pub fn all_books(&self) -> anyhow::Result<Vec<BookExport>> {
        let mut stmt = self
            .conn
            .prepare("SELECT title, file_path, page_count, word_count FROM books ORDER BY title")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(BookExport {
                    title: row.get(0)?,
                    file_path: row.get(1)?,
                    page_count: row.get(2)?,
                    word_count: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn all_entity(&self, table: &str, name_col: &str) -> anyhow::Result<Vec<EntityExport>> {
        let sql = format!(
            "SELECT {name_col}, description, source, source_url FROM {table} ORDER BY {name_col}"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(EntityExport {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    source: row.get(2)?,
                    source_url: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Every character, sorted by name.
    pub fn all_characters(&self) -> anyhow::Result<Vec<EntityExport>> {
        self.all_entity("characters", "name")
    }

    /// Every Great House, sorted by name.
    pub fn all_houses(&self) -> anyhow::Result<Vec<EntityExport>> {
        self.all_entity("houses", "name")
    }

    /// Every planet, sorted by name.
    pub fn all_planets(&self) -> anyhow::Result<Vec<EntityExport>> {
        self.all_entity("planets", "name")
    }

    /// Every glossary term, sorted by term.
    pub fn all_glossary(&self) -> anyhow::Result<Vec<GlossaryExport>> {
        let mut stmt = self.conn.prepare(
            "SELECT term, definition, source, source_url FROM glossary ORDER BY term",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(GlossaryExport {
                    term: row.get(0)?,
                    definition: row.get(1)?,
                    source: row.get(2)?,
                    source_url: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Every quote, sorted by text.
    pub fn all_quotes(&self) -> anyhow::Result<Vec<QuoteExport>> {
        let mut stmt = self.conn.prepare(
            "SELECT text, attribution, book_title, source FROM quotes ORDER BY text",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(QuoteExport {
                    text: row.get(0)?,
                    attribution: row.get(1)?,
                    book_title: row.get(2)?,
                    source: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

/// Queries every section in one pass.
pub fn collect_export_data(db: &Database) -> anyhow::Result<ExportData> {
    Ok(ExportData {
        books: db.all_books()?,
        characters: db.all_characters()?,
        houses: db.all_houses()?,
        planets: db.all_planets()?,
        glossary: db.all_glossary()?,
        quotes: db.all_quotes()?,
    })
}

/// Renders the export as a markdown document: a books table plus one sorted
/// `- name — description` list per codex section.
pub fn render_markdown(data: &ExportData) -> String {
    let mut out = String::new();

    out.push_str("## Books\n");
    out.push_str("| Title | Pages | Words |\n");
    out.push_str("|---|---|---|\n");
    for book in &data.books {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            book.title, book.page_count, book.word_count
        ));
    }

    let mut render_entities = |heading: &str, rows: &[EntityExport]| {
        out.push_str(&format!("\n## {}\n", heading));
        for row in rows {
            let desc = row.description.as_deref().unwrap_or_default();
            out.push_str(&format!("- {} — {}\n", row.name, desc));
        }
    };
    render_entities("Characters", &data.characters);
    render_entities("Houses", &data.houses);
    render_entities("Planets", &data.planets);

    out.push_str("\n## Glossary\n");
    for row in &data.glossary {
        let def = row.definition.as_deref().unwrap_or_default();
        out.push_str(&format!("- {} — {}\n", row.term, def));
    }

    out.push_str("\n## Quotes\n");
    for row in &data.quotes {
        let attribution = row.attribution.as_deref().unwrap_or_default();
        match &row.book_title {
            Some(book) => out.push_str(&format!(
                "- \"{}\" — {} ({})\n",
                row.text, attribution, book
            )),
            None => out.push_str(&format!("- \"{}\" — {}\n", row.text, attribution)),
        }
    }

    out
}

/// Renders the export as pretty-printed JSON.
pub fn render_json(data: &ExportData) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(data)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn empty_data() -> ExportData {
        ExportData {
            books: vec![],
            characters: vec![],
            houses: vec![],
            planets: vec![],
            glossary: vec![],
            quotes: vec![],
        }
    }

    fn populated_data() -> ExportData {
        ExportData {
            books: vec![BookExport {
                title: "Dune".to_string(),
                file_path: "/tmp/dune.pdf".to_string(),
                page_count: 412,
                word_count: 200000,
            }],
            characters: vec![EntityExport {
                name: "Paul Atreides".to_string(),
                description: Some("Duke of Arrakis".to_string()),
                source: Some("builtin".to_string()),
                source_url: None,
            }],
            houses: vec![],
            planets: vec![],
            glossary: vec![],
            quotes: vec![QuoteExport {
                text: "Fear is the mind-killer.".to_string(),
                attribution: Some("Paul Atreides".to_string()),
                book_title: Some("Dune".to_string()),
                source: Some("builtin".to_string()),
            }],
        }
    }

    #[test]
    fn render_markdown_empty_data_produces_empty_sections() {
        let md = render_markdown(&empty_data());
        assert!(md.contains("## Books"));
        assert!(md.contains("| Title | Pages | Words |"));
        assert!(md.contains("## Characters"));
        assert!(md.contains("## Houses"));
        assert!(md.contains("## Planets"));
        assert!(md.contains("## Glossary"));
        assert!(md.contains("## Quotes"));
        // Empty data must not leak any row content.
        assert!(!md.contains("| Dune |"));
        assert!(!md.contains("- Paul Atreides"));
        assert!(!md.contains("Fear is the mind-killer."));
    }

    #[test]
    fn render_markdown_populated_data_renders_table_and_lists() {
        let md = render_markdown(&populated_data());
        assert!(md.contains("| Dune | 412 | 200000 |"));
        assert!(md.contains("- Paul Atreides — Duke of Arrakis"));
        assert!(md.contains(
            "- \"Fear is the mind-killer.\" — Paul Atreides (Dune)"
        ));
    }

    #[test]
    fn render_json_empty_data_renders_all_empty_arrays() {
        let json = render_json(&empty_data()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let obj = value.as_object().expect("export json must be an object");
        for key in [
            "books",
            "characters",
            "houses",
            "planets",
            "glossary",
            "quotes",
        ] {
            assert_eq!(
                obj[key].as_array().map(Vec::len),
                Some(0),
                "section '{key}' must be an empty array"
            );
        }
    }

    #[test]
    fn render_json_populated_data_round_trips_struct_fields() {
        let data = populated_data();
        let json = render_json(&data).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["books"][0]["title"], "Dune");
        assert_eq!(value["books"][0]["page_count"], 412);
        assert_eq!(value["books"][0]["word_count"], 200000);
        assert_eq!(value["characters"][0]["name"], "Paul Atreides");
        assert_eq!(value["characters"][0]["description"], "Duke of Arrakis");
        assert_eq!(value["quotes"][0]["text"], "Fear is the mind-killer.");
        assert_eq!(value["quotes"][0]["book_title"], "Dune");
    }

    #[test]
    fn accessors_return_typed_rows_for_each_table() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        db.conn
            .execute_batch(
                "INSERT INTO books (title, file_path, page_count, word_count) VALUES
                    ('Dune', '/tmp/dune.pdf', 412, 200000);
                 INSERT INTO characters (name, description, source, source_url) VALUES
                    ('Paul Atreides', 'Duke of Arrakis', 'wiki',
                     'https://dune.fandom.com/wiki/Paul_Atreides');
                 INSERT INTO houses (name, description, source) VALUES
                    ('Atreides', 'Noble house of Caladan', 'builtin');
                 INSERT INTO planets (name, description, source) VALUES
                    ('Arrakis', 'Desert planet', 'builtin');
                 INSERT INTO glossary (term, definition, source) VALUES
                    ('Melange', 'The spice', 'builtin');
                 INSERT INTO quotes (text, attribution, book_title, source) VALUES
                    ('Fear is the mind-killer.', 'Paul Atreides', 'Dune', 'builtin');",
            )
            .unwrap();

        let books = db.all_books().unwrap();
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "Dune");
        assert_eq!(books[0].file_path, "/tmp/dune.pdf");
        assert_eq!(books[0].page_count, 412);
        assert_eq!(books[0].word_count, 200000);

        let characters = db.all_characters().unwrap();
        assert_eq!(characters.len(), 1);
        assert_eq!(characters[0].name, "Paul Atreides");
        assert_eq!(
            characters[0].source_url.as_deref(),
            Some("https://dune.fandom.com/wiki/Paul_Atreides")
        );

        assert_eq!(db.all_houses().unwrap()[0].name, "Atreides");
        assert_eq!(db.all_planets().unwrap()[0].name, "Arrakis");
        let glossary = db.all_glossary().unwrap();
        assert_eq!(glossary[0].term, "Melange");
        assert_eq!(glossary[0].definition.as_deref(), Some("The spice"));

        let quotes = db.all_quotes().unwrap();
        assert_eq!(quotes.len(), 1);
        assert_eq!(quotes[0].text, "Fear is the mind-killer.");
        assert_eq!(quotes[0].attribution.as_deref(), Some("Paul Atreides"));
        assert_eq!(quotes[0].book_title.as_deref(), Some("Dune"));
    }
}