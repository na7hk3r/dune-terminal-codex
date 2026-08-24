//! Shared helpers used by both the CLI and the TUI.

/// Formats a number with thousands separators: `1174969 -> "1,174,969"`.
pub fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Greedy word-wrap: splits `text` into display lines of at most `width`
/// characters. Words longer than `width` are hard-split. Blank input lines
/// are preserved as blank output lines.
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();

    for raw in text.split('\n') {
        let mut current = String::new();
        let mut current_len = 0usize;

        let words: Vec<&str> = raw.split_whitespace().collect();
        if words.is_empty() {
            out.push(String::new());
            continue;
        }

        for word in words {
            let mut word = word;
            loop {
                let word_len = word.chars().count();
                let sep = usize::from(!current.is_empty());
                if current_len + sep + word_len <= width {
                    if sep == 1 {
                        current.push(' ');
                    }
                    current.push_str(word);
                    current_len += sep + word_len;
                    break;
                }
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                    current_len = 0;
                    continue;
                }
                // Single word longer than a full line: hard-split it.
                let split_at = word
                    .char_indices()
                    .nth(width)
                    .map(|(i, _)| i)
                    .unwrap_or(word.len());
                out.push(word[..split_at].to_string());
                word = &word[split_at..];
                if word.is_empty() {
                    break;
                }
            }
        }
        out.push(current);
    }

    out
}

/// Truncates `s` to at most `max_chars` visible characters, ending with an
/// ellipsis (`…`) when anything was cut.
pub fn truncate_ellipsis(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut result: String = s.chars().take(max_chars - 1).collect();
    result.push('…');
    result
}

/// Reduces a string to lowercase ASCII alphanumerics, so titles can be
/// compared against filename stems regardless of punctuation/spacing.
pub fn slugify(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// True when `title` and `filename_stem` carry the same information
/// (i.e., showing the filename next to the title adds nothing).
pub fn filename_derivable_from_title(title: &str, filename_stem: &str) -> bool {
    let t = slugify(title);
    !t.is_empty() && t == slugify(filename_stem)
}

/// Renders the CLI Oracle box: quote + author wrapped and padded so every
/// border line has exactly the same display width.
pub fn oracle_box(wisdom: &str, source: Option<&str>) -> Vec<String> {
    const MAX_INNER: usize = 60;
    const INDENT: &str = "  ";

    let mut content: Vec<String> = vec!["ORACLE".to_string(), String::new()];
    for line in wrap_text(wisdom, MAX_INNER - 2) {
        content.push(format!("\u{201C}{}\u{201D}", line));
    }
    if let Some(src) = source {
        content.push(String::new());
        content.push(format!("— {}", src));
    }

    let inner_width = content
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(6);

    let horizontal = "─".repeat(inner_width + 2);
    let mut out = vec![format!("{}╭{}╮", INDENT, horizontal)];
    for line in &content {
        out.push(format!(
            "{}│ {}{} │",
            INDENT,
            line,
            " ".repeat(inner_width - line.chars().count())
        ));
    }
    out.push(format!("{}╰{}╯", INDENT, horizontal));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_basic() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(1), "1");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn wrap_text_respects_width() {
        assert_eq!(wrap_text("hola mundo cruel", 11), vec!["hola mundo", "cruel"]);
    }

    #[test]
    fn wrap_text_preserves_blank_lines() {
        assert_eq!(wrap_text("a\n\nb", 10), vec!["a", "", "b"]);
    }

    #[test]
    fn wrap_text_hard_splits_long_words() {
        assert_eq!(wrap_text("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn wrap_text_never_exceeds_width() {
        let text = "The spice must flow through the desert and the worms will come";
        for line in wrap_text(text, 12) {
            assert!(line.chars().count() <= 12, "line too long: {:?}", line);
        }
    }

    #[test]
    fn truncate_ellipsis_short_strings_pass_through() {
        assert_eq!(truncate_ellipsis("short", 10), "short");
        assert_eq!(truncate_ellipsis("exact", 5), "exact");
    }

    #[test]
    fn truncate_ellipsis_cuts_with_marker() {
        assert_eq!(truncate_ellipsis("abcdefghij", 6), "abcde…");
        assert_eq!(truncate_ellipsis("abc", 1), "…");
        assert_eq!(truncate_ellipsis("abc", 0), "");
    }

    #[test]
    fn truncate_ellipsis_counts_unicode_chars() {
        assert_eq!(truncate_ellipsis("áéíóúñ", 4), "áéí…");
    }

    #[test]
    fn filename_matches_slugified_title() {
        assert!(filename_derivable_from_title(
            "1 Dune Frank Herbert",
            "1_Dune_-_Frank_Herbert"
        ));
        assert!(!filename_derivable_from_title(
            "Dune",
            "dune_reader_edition_v2"
        ));
        // Empty titles never count as derivable.
        assert!(!filename_derivable_from_title("", "anything"));
    }

    #[test]
    fn oracle_box_lines_are_aligned() {
        let lines = oracle_box("Fear is the mind-killer.", Some("Paul Atreides — Dune"));
        let widths: Vec<usize> = lines.iter().map(|l| l.chars().count()).collect();
        assert!(widths.windows(2).all(|w| w[0] == w[1]));
        // The quote must live inside the borders, never on its own line.
        assert!(lines.iter().any(|l| l.contains('“') && l.contains('"') == false));
        assert!(lines
            .iter()
            .all(|l| !l.contains("Fear") || (l.starts_with("  │") && l.ends_with('│'))));
    }

    #[test]
    fn oracle_box_wraps_long_quotes() {
        let wisdom = "The mystery of life isn't a problem to solve, but a reality to \
                      experience, and the spice must flow across a very long horizon";
        for line in oracle_box(wisdom, None) {
            assert!(line.chars().count() <= 70, "box too wide: {:?}", line);
        }
    }
}
