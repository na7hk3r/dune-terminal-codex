use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Deserialize;
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

use super::clean::clean_entity_text;

#[derive(Deserialize)]
struct WikiResponse {
    description: String,
    wiki: String,
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

fn fetch_term(base_url: &str, term: &str) -> Result<WikiResponse> {
    let url = format!("{}/dune/{}", base_url, term);
    let response: WikiResponse = ureq::get(&url)
        .call()
        .with_context(|| format!("HTTP request failed for term: {}", term))?
        .body_mut()
        .read_json()
        .with_context(|| format!("Failed to parse JSON for term: {}", term))?;
    Ok(response)
}

pub fn import_from_wiki(conn: &Connection, base_url: &str) -> Result<()> {
    info!("Starting codex import from {}", base_url);

    let mut imported = 0u32;
    let mut errors = Vec::new();

    info!("Importing characters...");
    for term in CHARACTERS {
        match fetch_term(base_url, term) {
            Ok(resp) => {
                if resp.description.len() > 10 {
                    let description = clean_entity_text(&term.replace('_', " "), &resp.description);
                    conn.execute(
                        "INSERT OR REPLACE INTO characters (name, description, source, aliases)
                         VALUES (?1, ?2, 'wiki', ?3)",
                        rusqlite::params![term.replace('_', " "), description.as_str(), resp.wiki],
                    )?;
                    imported += 1;
                    info!("  character: {}", term);
                }
            }
            Err(e) => {
                warn!("  failed: {} - {}", term, e);
                errors.push(format!("character {}: {}", term, e));
            }
        }
        thread::sleep(Duration::from_millis(150));
    }

    info!("Importing houses...");
    for term in HOUSES {
        match fetch_term(base_url, term) {
            Ok(resp) => {
                if resp.description.len() > 10 {
                    let description = clean_entity_text(&term.replace('_', " "), &resp.description);
                    conn.execute(
                        "INSERT OR REPLACE INTO houses (name, description, source)
                         VALUES (?1, ?2, 'wiki')",
                        rusqlite::params![term.replace('_', " "), description.as_str()],
                    )?;
                    imported += 1;
                    info!("  house: {}", term);
                }
            }
            Err(e) => {
                warn!("  failed: {} - {}", term, e);
                errors.push(format!("house {}: {}", term, e));
            }
        }
        thread::sleep(Duration::from_millis(150));
    }

    info!("Importing planets...");
    for term in PLANETS {
        match fetch_term(base_url, term) {
            Ok(resp) => {
                if resp.description.len() > 10 {
                    let description = clean_entity_text(&term.replace('_', " "), &resp.description);
                    conn.execute(
                        "INSERT OR REPLACE INTO planets (name, description, source)
                         VALUES (?1, ?2, 'wiki')",
                        rusqlite::params![term.replace('_', " "), description.as_str()],
                    )?;
                    imported += 1;
                    info!("  planet: {}", term);
                }
            }
            Err(e) => {
                warn!("  failed: {} - {}", term, e);
                errors.push(format!("planet {}: {}", term, e));
            }
        }
        thread::sleep(Duration::from_millis(150));
    }

    info!("Importing glossary...");
    for term in GLOSSARY_TERMS {
        let clean_term = term.trim();
        match fetch_term(base_url, clean_term) {
            Ok(resp) => {
                if resp.description.len() > 10 {
                    let description =
                        clean_entity_text(&clean_term.replace('_', " "), &resp.description);
                    conn.execute(
                        "INSERT OR REPLACE INTO glossary (term, definition, source)
                         VALUES (?1, ?2, 'wiki')",
                        rusqlite::params![clean_term.replace('_', " "), description.as_str()],
                    )?;
                    imported += 1;
                    info!("  glossary: {}", clean_term);
                }
            }
            Err(e) => {
                warn!("  failed: {} - {}", clean_term, e);
                errors.push(format!("glossary {}: {}", clean_term, e));
            }
        }
        thread::sleep(Duration::from_millis(150));
    }

    add_builtin_quotes(conn)?;

    info!(
        "Import complete: {} items imported, {} errors",
        imported,
        errors.len()
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
