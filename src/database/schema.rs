pub const CREATE_BOOKS: &str = "
CREATE TABLE IF NOT EXISTS books (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    file_path TEXT NOT NULL UNIQUE,
    file_hash TEXT,
    page_count INTEGER DEFAULT 0,
    word_count INTEGER DEFAULT 0,
    file_size INTEGER DEFAULT 0,
    indexed_at TEXT,
    created_at TEXT DEFAULT (datetime('now'))
)";

pub const CREATE_PAGES: &str = "
CREATE TABLE IF NOT EXISTS pages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    page_number INTEGER NOT NULL,
    content TEXT NOT NULL,
    UNIQUE(book_id, page_number)
)";

pub const CREATE_CHAPTERS: &str = "
CREATE TABLE IF NOT EXISTS chapters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    start_page INTEGER NOT NULL,
    end_page INTEGER
)";

pub const CREATE_CHARACTERS: &str = "
CREATE TABLE IF NOT EXISTS characters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    aliases TEXT,
    house TEXT,
    description TEXT,
    related TEXT,
    books TEXT,
    source TEXT DEFAULT 'builtin'
)";

pub const CREATE_HOUSES: &str = "
CREATE TABLE IF NOT EXISTS houses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    homeworld TEXT,
    notable_members TEXT,
    source TEXT DEFAULT 'builtin'
)";

pub const CREATE_PLANETS: &str = "
CREATE TABLE IF NOT EXISTS planets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    system TEXT,
    notable_features TEXT,
    source TEXT DEFAULT 'builtin'
)";

pub const CREATE_GLOSSARY: &str = "
CREATE TABLE IF NOT EXISTS glossary (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    term TEXT NOT NULL UNIQUE,
    definition TEXT NOT NULL,
    category TEXT,
    origin TEXT,
    source TEXT DEFAULT 'builtin'
)";

pub const CREATE_QUOTES: &str = "
CREATE TABLE IF NOT EXISTS quotes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    text TEXT NOT NULL,
    attribution TEXT,
    book_title TEXT,
    chapter TEXT,
    page_number INTEGER,
    source TEXT DEFAULT 'builtin'
)";

pub const CREATE_SEARCH_INDEX: &str = "
CREATE VIRTUAL TABLE IF NOT EXISTS pages_fts USING fts5(
    content,
    content='pages',
    content_rowid='id',
    tokenize='unicode61'
)";

pub const CREATE_FTS_SYNC: &str = "
CREATE TRIGGER IF NOT EXISTS pages_ai AFTER INSERT ON pages BEGIN
    INSERT INTO pages_fts(rowid, content) VALUES (new.id, new.content);
END;

CREATE TRIGGER IF NOT EXISTS pages_ad AFTER DELETE ON pages BEGIN
    INSERT INTO pages_fts(pages_fts, rowid, content) VALUES('delete', old.id, old.content);
END;

CREATE TRIGGER IF NOT EXISTS pages_au AFTER UPDATE ON pages BEGIN
    INSERT INTO pages_fts(pages_fts, rowid, content) VALUES('delete', old.id, old.content);
    INSERT INTO pages_fts(rowid, content) VALUES (new.id, new.content);
END;
";

pub const MIGRATION_11_SOURCE_URL: &str = "
ALTER TABLE characters ADD COLUMN source_url TEXT;
ALTER TABLE houses ADD COLUMN source_url TEXT;
ALTER TABLE planets ADD COLUMN source_url TEXT;
ALTER TABLE glossary ADD COLUMN source_url TEXT;

UPDATE characters
SET source_url = aliases, aliases = NULL
WHERE source_url IS NULL AND aliases LIKE 'http%';
";

pub const MIGRATION_12_DROP_DEAD: &str = "
DROP TABLE chapters;

ALTER TABLE characters DROP COLUMN house;
ALTER TABLE characters DROP COLUMN related;
ALTER TABLE characters DROP COLUMN books;

ALTER TABLE houses DROP COLUMN homeworld;
ALTER TABLE houses DROP COLUMN notable_members;

ALTER TABLE planets DROP COLUMN system;
ALTER TABLE planets DROP COLUMN notable_features;

ALTER TABLE glossary DROP COLUMN category;
ALTER TABLE glossary DROP COLUMN origin;

ALTER TABLE quotes DROP COLUMN chapter;
ALTER TABLE quotes DROP COLUMN page_number;
";

pub const MIGRATION_13_TRIGRAMS: &str = "
CREATE VIRTUAL TABLE pages_trigrams USING fts5(
    content,
    tokenize='trigram case_sensitive 0',
    content='pages',
    content_rowid='id'
);

CREATE TRIGGER pages_trigrams_ai AFTER INSERT ON pages BEGIN
    INSERT INTO pages_trigrams(rowid, content) VALUES (new.id, new.content);
END;

CREATE TRIGGER pages_trigrams_ad AFTER DELETE ON pages BEGIN
    INSERT INTO pages_trigrams(pages_trigrams, rowid, content) VALUES('delete', old.id, old.content);
END;

CREATE TRIGGER pages_trigrams_au AFTER UPDATE ON pages BEGIN
    INSERT INTO pages_trigrams(pages_trigrams, rowid, content) VALUES('delete', old.id, old.content);
    INSERT INTO pages_trigrams(rowid, content) VALUES (new.id, new.content);
END;

INSERT INTO pages_trigrams(rowid, content) SELECT id, content FROM pages;
";

pub const CREATE_MIGRATIONS: &str = "
CREATE TABLE IF NOT EXISTS migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT DEFAULT (datetime('now'))
)";

pub const ALL_MIGRATIONS: &[(&str, &str)] = &[
    ("1", CREATE_BOOKS),
    ("2", CREATE_PAGES),
    ("3", CREATE_CHAPTERS),
    ("4", CREATE_CHARACTERS),
    ("5", CREATE_HOUSES),
    ("6", CREATE_PLANETS),
    ("7", CREATE_GLOSSARY),
    ("8", CREATE_QUOTES),
    ("9", CREATE_SEARCH_INDEX),
    ("10", CREATE_FTS_SYNC),
    ("11", MIGRATION_11_SOURCE_URL),
    ("12", MIGRATION_12_DROP_DEAD),
    ("13", MIGRATION_13_TRIGRAMS),
];
