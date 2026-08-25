use std::path::Path;

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn guess_book_title(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown");

    let cleaned = stem.replace(['_', '-'], " ");

    let title_case: Vec<String> = cleaned
        .split_whitespace()
        .enumerate()
        .map(|(i, word)| {
            let lower = word.to_lowercase();
            match lower.as_str() {
                "the" | "a" | "an" | "of" | "and" | "in" | "on" | "at" | "to" | "for"
                    if i > 0 =>
                {
                    lower
                }
                _ => capitalize(&lower),
            }
        })
        .collect();

    title_case.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn guess_title_from_filename() {
        let path = PathBuf::from("/books/Dune.pdf");
        assert_eq!(guess_book_title(&path), "Dune");
    }

    #[test]
    fn guess_title_with_subdirectory() {
        let path = PathBuf::from("/books/Dune Messiah.pdf");
        assert_eq!(guess_book_title(&path), "Dune Messiah");
    }

    #[test]
    fn guess_title_with_underscores() {
        let path = PathBuf::from("/books/Children_of_Dune.pdf");
        assert_eq!(guess_book_title(&path), "Children of Dune");
    }

    #[test]
    fn guess_title_preserves_articles() {
        let path = PathBuf::from("/books/The_Dune_Encyclopedia.pdf");
        assert_eq!(guess_book_title(&path), "The Dune Encyclopedia");
    }
}
