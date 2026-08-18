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
