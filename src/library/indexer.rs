use crate::config::settings::Config;
use crate::database::repository::Database;
use crate::library::metadata::guess_book_title;
use crate::library::pdf;
use crate::library::scanner;
use std::path::Path;
use tracing::{info, warn};

pub struct IndexResult {
    pub books_indexed: u32,
    pub pages_indexed: u32,
    pub errors: Vec<String>,
}

pub fn index_library(db: &Database, config: &Config, verbose: bool) -> anyhow::Result<IndexResult> {
    let mut result = IndexResult {
        books_indexed: 0,
        pages_indexed: 0,
        errors: Vec::new(),
    };

    let paths = config.resolve_library_paths();

    if paths.is_empty() {
        return Ok(result);
    }

    for dir in &paths {
        if !dir.is_dir() {
            warn!("library path does not exist: {}", dir.display());
            result
                .errors
                .push(format!("path not found: {}", dir.display()));
            continue;
        }

        let pdfs = scanner::scan_directory(dir);
        if verbose {
            info!("found {} PDFs in {}", pdfs.len(), dir.display());
        }

        for pdf_path in &pdfs {
            match index_pdf(db, pdf_path, verbose) {
                Ok((pages, _book_id)) => {
                    result.books_indexed += 1;
                    result.pages_indexed += pages;
                    if verbose {
                        info!("indexed: {} ({} pages)", pdf_path.display(), pages);
                    }
                }
                Err(e) => {
                    let msg = format!("{}: {}", pdf_path.display(), e);
                    warn!("{}", msg);
                    result.errors.push(msg);
                }
            }
        }
    }

    Ok(result)
}

pub fn index_pdf(db: &Database, path: &Path, verbose: bool) -> anyhow::Result<(u32, i64)> {
    let info = pdf::pdf_info(path)?;
    let title = guess_book_title(path);

    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len() as i64;

    let book_id = db.insert_book(
        &title,
        &path.to_string_lossy(),
        info.page_count,
        0,
        file_size,
    )?;

    let mut total_words: u32 = 0;

    for page_num in 1..=info.page_count {
        match pdf::extract_page(path, page_num) {
            Ok(content) => {
                total_words += pdf::word_count(&content);
                db.insert_page(book_id, page_num, &content)?;
            }
            Err(e) => {
                if verbose {
                    warn!("failed to extract page {} of {}: {}", page_num, title, e);
                }
            }
        }
    }

    db.conn.execute(
        "UPDATE books SET word_count = ?1 WHERE id = ?2",
        rusqlite::params![total_words, book_id],
    )?;

    Ok((info.page_count, book_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repository::Database;
    use std::path::Path;

    fn test_db() -> Database {
        Database::open(Path::new(":memory:")).unwrap()
    }

    #[test]
    fn index_result_defaults() {
        let result = IndexResult {
            books_indexed: 0,
            pages_indexed: 0,
            errors: Vec::new(),
        };
        assert_eq!(result.books_indexed, 0);
        assert!(result.errors.is_empty());
    }
}
