use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Deserialize;
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

use super::clean::clean_entity_text;

#[derive(Deserialize)]
pub struct WikiResponse {
    pub description: String,
    pub wiki: String,
}

/// The network strategy for fetching a single wiki term, injectable so the
/// importer can be tested without HTTP.
pub type Fetcher = fn(base_url: &str, term: &str) -> Result<WikiResponse>;

/// Describes one entity type for the shared wiki import path (D8).
#[derive(Debug, Clone, Copy)]
pub struct WikiKind {
    pub table: &'static str,
    pub name_col: &'static str,
    pub desc_col: &'static str,
}

const CHARACTERS: &[&str] = &[
    "paul_atreides",
    "leto_atreides_i",
    "jessica_atreides",
    "alia_atreides",
    "leto_atreides_ii",
    "ghanima_atreides",
    "vladimir_harkonnen",
    "feyd-rautha_harkonnen",
    "glossu_rabban",
    "duncan_idaho",
    "thufir_hawat",
    "gurney_halleck",
    "chani",
    "stilgar",
    "liet_kynes",
    "irulan_corrino",
    "shaddam_corrino_iv",
    "gaius_helen_mohiam",
    "count_fenring",
    "hayt",
    "scytale",
    "siona_atreides",
    "moneo_atreides",
    "lucilla",
    "hwi_oreilly",
    "princess_wensicia",
    "edric",
    "malky",
    "dancing_bear",
    "wellington_yueh",
];

const HOUSES: &[&str] = &[
    "house_atreides",
    "house_harkonnen",
    "house_corrino",
    "house_ix",
    "house_richese",
    "house_ginaz",
    "house_canopus",
    "house_ecaz",
    "house_vermillion",
];

const PLANETS: &[&str] = &[
    "arrakis",
    "caladan",
    "giedi_prime",
    "kaitain",
    "salusa_secundus",
    "ix",
    "richese",
    "calmull",
    "poritrin",
    "bela_teguse",
    "lankiveil",
    "earth",
];

const GLOSSARY_TERMS: &[&str] = &[
    "spice_melange",
    "krisatz_haderach",
    "bene_gesserit",
    "mentat",
    "guild_navigator",
    "sandworm",
    "shai-hulud",
    "stillsuit",
    "crysknife",
    "gom_jabbar",
    "golden_path",
    "sardaukar",
    "imperium",
    "landsraad",
    "CHOAM",
    "spacing_guild",
    "melange",
    "spice",
    "nothink",
    "other_memory",
    "prana-bindu",
    "weirding_way",
    "kanly",
    "faufreluches",
    "sietch",
    "fremen",
    "ornithopter",
    "thumpers",
    "water_of_life",
    "butlerian_jihad",
];

pub fn fetch_term(base_url: &str, term: &str) -> Result<WikiResponse> {
    let url = format!("{}/dune/{}", base_url, term);
    let response: WikiResponse = ureq::get(&url)
        .call()
        .with_context(|| format!("HTTP request failed for term: {}", term))?
        .body_mut()
        .read_json()
        .with_context(|| format!("Failed to parse JSON for term: {}", term))?;
    Ok(response)
}

/// Shared import loop for one entity type (D8).
/// Returns `(imported_count, errors)`.
fn import_kind(
    conn: &Connection,
    base_url: &str,
    spec: WikiKind,
    terms: &[&str],
    fetch: Fetcher,
) -> (u32, Vec<String>) {
    let mut imported = 0u32;
    let mut errors = Vec::new();
    let kind_label = spec.table;

    for term in terms {
        let clean_term = term.trim();
        match fetch(base_url, clean_term) {
            Ok(resp) => {
                if resp.description.len() > 10 {
                    let description =
                        clean_entity_text(&clean_term.replace('_', " "), &resp.description);
                    let sql = format!(
                        "INSERT OR REPLACE INTO {} ({}, {}, source, source_url)
                         VALUES (?1, ?2, 'wiki', ?3)",
                        spec.table, spec.name_col, spec.desc_col,
                    );
                    conn.execute(
                        &sql,
                        rusqlite::params![
                            clean_term.replace('_', " "),
                            description.as_str(),
                            resp.wiki
                        ],
                    )
                    .unwrap_or_else(|e| {
                        warn!("  insert failed for {} {}: {}", kind_label, clean_term, e);
                        0
                    });
                    imported += 1;
                    info!("  {}: {}", kind_label, clean_term);
                }
            }
            Err(e) => {
                warn!("  failed: {} - {}", clean_term, e);
                errors.push(format!("{} {}: {}", kind_label, clean_term, e));
            }
        }
        thread::sleep(Duration::from_millis(150));
    }
    (imported, errors)
}

const CHARACTERS_SPEC: WikiKind = WikiKind {
    table: "characters",
    name_col: "name",
    desc_col: "description",
};
const HOUSES_SPEC: WikiKind = WikiKind {
    table: "houses",
    name_col: "name",
    desc_col: "description",
};
const PLANETS_SPEC: WikiKind = WikiKind {
    table: "planets",
    name_col: "name",
    desc_col: "description",
};
const GLOSSARY_SPEC: WikiKind = WikiKind {
    table: "glossary",
    name_col: "term",
    desc_col: "definition",
};

pub fn import_from_wiki(conn: &Connection, base_url: &str, fetch: Fetcher) -> Result<()> {
    info!("Starting codex import from {}", base_url);

    let mut total_imported = 0u32;
    let mut all_errors = Vec::new();

    let specs: &[(WikiKind, &[&str])] = &[
        (CHARACTERS_SPEC, CHARACTERS),
        (HOUSES_SPEC, HOUSES),
        (PLANETS_SPEC, PLANETS),
        (GLOSSARY_SPEC, GLOSSARY_TERMS),
    ];

    for (spec, terms) in specs {
        info!("Importing {}...", spec.table);
        let (count, errs) = import_kind(conn, base_url, *spec, terms, fetch);
        total_imported += count;
        all_errors.extend(errs);
    }

    add_builtin_quotes(conn)?;

    info!(
        "Import complete: {} items imported, {} errors",
        total_imported,
        all_errors.len()
    );

    Ok(())
}

fn add_builtin_quotes(conn: &Connection) -> Result<()> {
    let quotes: Vec<(&str, &str, &str)> = vec![
        (
            "Fear is the mind-killer. Fear is the little-death that brings total obliteration. I will face my fear. I will permit it to pass over me and through me.",
            "Paul Atreides",
            "Dune",
        ),
        (
            "He who controls the spice controls the universe.",
            "Unknown",
            "Dune",
        ),
        (
            "The mystery of life isn't a problem to solve, but a reality to experience.",
            "Paul Atreides",
            "Dune",
        ),
        (
            "A beginning is the time for taking the most delicate care.",
            "Unknown",
            "Dune",
        ),
        ("Tell me of your homeworld Usul.", "Stilgar", "Dune"),
        (
            "The spice melange is the key to interstellar travel.",
            "Unknown",
            "Dune",
        ),
        ("Only I will remain.", "Paul Atreides", "Dune"),
        (
            "He who can destroy a thing, controls a thing.",
            "Paul Atreides",
            "Dune",
        ),
        ("The sleeper must awaken.", "Paul Atreides", "Dune"),
        (
            "Walk without rhythm, and you won't attract the worm.",
            "Stilgar",
            "Dune",
        ),
        (
            "There is no escape, we pay for the violence of our ancestors.",
            "Unknown",
            "Dune",
        ),
        (
            "God created Arrakis to train the faithful.",
            "Unknown",
            "Dune",
        ),
        ("Hope clouds observation.", "Unknown", "Dune"),
        (
            "Surprise is the most dangerous weapon in the universe.",
            "Leto Atreides I",
            "Dune",
        ),
        ("Train hard, fight easy.", "Gurney Halleck", "Dune"),
        (
            "The beginning of knowledge is the discovery of something we do not understand.",
            "Unknown",
            "Dune",
        ),
        (
            "You don't get something for nothing. You can't have freedom without responsibility.",
            "Unknown",
            "Dune",
        ),
    ];

    for (text, attribution, book_title) in &quotes {
        conn.execute(
            "INSERT OR IGNORE INTO quotes (text, attribution, book_title, source)
             VALUES (?1, ?2, ?3, 'builtin')",
            rusqlite::params![text, attribution, book_title],
        )?;
    }

    info!("Added {} builtin quotes", quotes.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_fetcher(base_url: &str, term: &str) -> Result<WikiResponse> {
        Ok(WikiResponse {
            description: format!("A detailed entry about the term {}.", term),
            wiki: format!("{}/dune/{}", base_url, term),
        })
    }

    #[test]
    fn import_writes_source_url_and_self_heals_legacy_alias_rows() {
        let conn = Connection::open_in_memory().unwrap();
        crate::database::migrations::run_migrations(&conn).unwrap();

        // A legacy row from the pre-migration-11 schema: the wiki URL lives in
        // aliases and source_url does not exist at insert time.
        conn.execute(
            "INSERT INTO characters (name, aliases) VALUES (?1, ?2)",
            rusqlite::params![
                "paul atreides",
                "https://dune.fandom.com/wiki/paul_atreides"
            ],
        )
        .unwrap();

        import_from_wiki(&conn, "https://stub.local", stub_fetcher).unwrap();

        let (url, aliases): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT source_url, aliases FROM characters WHERE name = ?1",
                ["paul atreides"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            url.as_deref(),
            Some("https://stub.local/dune/paul_atreides"),
            "re-import must write the wiki URL into source_url"
        );
        assert_eq!(
            aliases, None,
            "INSERT OR REPLACE must clear the legacy aliases column (self-heal)"
        );

        let url: Option<String> = conn
            .query_row(
                "SELECT source_url FROM houses WHERE name = ?1",
                ["house atreides"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            url.as_deref(),
            Some("https://stub.local/dune/house_atreides"),
            "houses must receive source_url too"
        );
    }

    /// All four entity types go through the shared `import_kind` path and
    /// land correct rows with `source_url` set.
    #[test]
    fn import_kind_all_tables_landing() {
        let conn = Connection::open_in_memory().unwrap();
        crate::database::migrations::run_migrations(&conn).unwrap();

        let terms = &["test_term_one", "test_term_two"];
        let (_, errs) = import_kind(&conn, "https://stub.local", CHARACTERS_SPEC, terms, stub_fetcher);
        assert!(errs.is_empty(), "stub should not fail");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM characters WHERE source='wiki'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2, "both terms should land in characters");

        let url: Option<String> = conn
            .query_row(
                "SELECT source_url FROM characters WHERE name = ?1",
                ["test term one"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            url.as_deref(),
            Some("https://stub.local/dune/test_term_one"),
            "source_url must be set"
        );

        // houses via shared path
        let (_, errs) = import_kind(&conn, "https://stub.local", HOUSES_SPEC, terms, stub_fetcher);
        assert!(errs.is_empty());
        let hcount: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM houses WHERE source='wiki'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hcount, 2);

        // planets via shared path
        let (_, errs) = import_kind(&conn, "https://stub.local", PLANETS_SPEC, terms, stub_fetcher);
        assert!(errs.is_empty());
        let pcount: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM planets WHERE source='wiki'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pcount, 2);

        // glossary via shared path
        let (_, errs) = import_kind(&conn, "https://stub.local", GLOSSARY_SPEC, terms, stub_fetcher);
        assert!(errs.is_empty());
        let gcount: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM glossary WHERE source='wiki'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(gcount, 2);

        // glossary uses term/definition columns
        let (term_val, def_val): (String, String) = conn
            .query_row(
                "SELECT term, definition FROM glossary WHERE term = ?1",
                ["test term one"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(term_val, "test term one");
        assert!(def_val.contains("test_term_one"));
    }

    /// When the fetcher fails for ONE term, that error is recorded and the
    /// remaining terms still import successfully.
    #[test]
    fn import_kind_partial_failure_records_error() {
        let conn = Connection::open_in_memory().unwrap();
        crate::database::migrations::run_migrations(&conn).unwrap();

        fn fail_first(base_url: &str, term: &str) -> Result<WikiResponse> {
            if term == "bad_term" {
                anyhow::bail!("network timeout");
            }
            stub_fetcher(base_url, term)
        }

        let terms = &["bad_term", "good_term"];
        let (imported, errs) =
            import_kind(&conn, "https://stub.local", CHARACTERS_SPEC, terms, fail_first);

        assert_eq!(imported, 1, "only good_term should import");
        assert_eq!(errs.len(), 1, "one error should be recorded");
        assert!(
            errs[0].contains("bad_term"),
            "error message should name the failing term"
        );

        let row: Option<String> = conn
            .query_row(
                "SELECT name FROM characters WHERE name = ?1",
                ["good term"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(row.as_deref(), Some("good term"));
    }

    /// A description shorter than 10 characters causes the row to be skipped.
    #[test]
    fn import_kind_short_description_skipped() {
        let conn = Connection::open_in_memory().unwrap();
        crate::database::migrations::run_migrations(&conn).unwrap();

        fn short_desc(_base_url: &str, term: &str) -> Result<WikiResponse> {
            Ok(WikiResponse {
                description: "short".to_string(),
                wiki: format!("https://stub.local/dune/{}", term),
            })
        }

        let terms = &["tiny"];
        let (imported, _) =
            import_kind(&conn, "https://stub.local", CHARACTERS_SPEC, terms, short_desc);

        assert_eq!(imported, 0, "short description must be skipped");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM characters WHERE source='wiki'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
}
