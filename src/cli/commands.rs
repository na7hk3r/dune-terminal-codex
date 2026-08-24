use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dune",
    about = "Dune Terminal Codex — A Codex of Arrakis in your terminal",
    version,
    long_about = "An offline encyclopedia and search tool for the Dune universe.\n\nIndex your personal Dune book collection, search full-text,\nexplore the codex, and receive wisdom from the Oracle."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Open the TUI interface
    Tui,

    /// Search your indexed books
    Search {
        /// Search query
        query: String,

        /// Filter by book name
        #[arg(long)]
        book: Option<String>,
    },

    /// List indexed books
    Books,

    /// Show information about a specific book
    Book {
        /// Book name (partial match)
        name: String,
    },

    /// Look up a character in the codex
    Character {
        /// Character name
        name: String,
    },

    /// Look up a Great House
    House {
        /// House name
        name: String,
    },

    /// Look up a planet
    Planet {
        /// Planet name
        name: String,
    },

    /// Browse or search the glossary
    Glossary {
        /// Optional term to look up
        term: Option<String>,
    },

    /// Get a random quote
    Quote,

    /// Receive wisdom from the Oracle
    Oracle,

    /// Show library statistics
    Stats,

    /// Index or reindex your book collection
    Index {
        /// Rebuild the entire index from scratch
        #[arg(long)]
        rebuild: bool,

        /// Show verbose output during indexing
        #[arg(long)]
        verbose: bool,

        /// Index specific file(s) instead of scanning library paths
        #[arg(long = "file", num_args = 1..)]
        files: Option<Vec<String>>,
    },

    /// Import codex data from external sources
    ImportCodex,

    /// Show or edit configuration
    Config,

    /// Open a PDF at a specific page, by path or by book id
    Open {
        /// Path to a PDF file, or the numeric id shown by `dune books`
        path: String,

        /// Page number to open at
        #[arg(long)]
        page: Option<u32>,
    },
}
