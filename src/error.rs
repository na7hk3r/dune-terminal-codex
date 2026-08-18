use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum DuneError {
    #[error("configuration not found at {path}")]
    ConfigNotFound { path: PathBuf },

    #[error("invalid configuration: {reason}")]
    ConfigInvalid { reason: String },

    #[error("library path does not exist: {path}")]
    LibraryPathMissing { path: PathBuf },

    #[error("no library paths configured")]
    NoLibraryPaths,

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("PDF extraction failed for {path}: {reason}")]
    PdfExtractionFailed { path: PathBuf, reason: String },

    #[error("PDF not found: {path}")]
    PdfNotFound { path: PathBuf },

    #[error("no search results for '{query}'")]
    NoSearchResults { query: String },

    #[error("entity not found: {name}")]
    EntityNotFound { name: String },

    #[error("failed to open PDF reader: {0}")]
    ReaderFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, DuneError>;
