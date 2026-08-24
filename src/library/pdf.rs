use std::path::Path;
use std::process::Command;

#[allow(dead_code)]
pub struct PdfInfo {
    pub page_count: u32,
    pub title: Option<String>,
}

pub fn pdf_info(path: &Path) -> anyhow::Result<PdfInfo> {
    let output = Command::new("pdfinfo")
        .arg(path)
        .output()
        .map_err(|e| anyhow::anyhow!("pdfinfo not found: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("pdfinfo failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let page_count = stdout
        .lines()
        .find(|l| l.starts_with("Pages:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let title = stdout
        .lines()
        .find(|l| l.starts_with("Title:"))
        .and_then(|l| l.splitn(2, ':').nth(1))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    Ok(PdfInfo { page_count, title })
}

pub fn extract_page(path: &Path, page: u32) -> anyhow::Result<String> {
    let output = Command::new("pdftotext")
        .args(["-f", &page.to_string(), "-l", &page.to_string()])
        .arg(path)
        .arg("-")
        .output()
        .map_err(|e| anyhow::anyhow!("pdftotext not found: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("pdftotext failed: {}", stderr));
    }

    Ok(fix_extracted_text(&String::from_utf8_lossy(&output.stdout)))
}

#[allow(dead_code)]
pub fn extract_all(path: &Path) -> anyhow::Result<String> {
    let output = Command::new("pdftotext")
        .arg(path)
        .arg("-")
        .output()
        .map_err(|e| anyhow::anyhow!("pdftotext not found: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("pdftotext failed: {}", stderr));
    }

    Ok(fix_extracted_text(&String::from_utf8_lossy(&output.stdout)))
}

/// Repairs artifacts introduced when page/line text is concatenated:
///
/// * de-hyphenation — a line ending in `-` whose next line starts with a
///   lowercase letter is joined without the hyphen (`naviga-` + `tion` →
///   `navigation`);
/// * soft hyphens (`U+00AD`) left behind by layout engines are dropped.
///
/// Running it on already-fixed text is a no-op (idempotent), so re-indexing
/// an existing library is safe.
pub fn fix_extracted_text(text: &str) -> String {
    let mut out_lines: Vec<String> = Vec::with_capacity(text.lines().count());

    for raw in text.lines() {
        let line = raw.trim_end().replace('\u{AD}', "");
        let joined = match out_lines.last() {
            Some(prev) if prev.ends_with('-') && starts_lowercase(&line) => {
                let mut merged = out_lines.pop().unwrap();
                merged.pop(); // drop the trailing hyphen
                merged.push_str(&line);
                merged
            }
            _ => line,
        };
        out_lines.push(joined);
    }

    out_lines.join("\n")
}

fn starts_lowercase(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_lowercase())
}

pub fn word_count(text: &str) -> u32 {
    text.split_whitespace().count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_hyphenated_line_breaks() {
        assert_eq!(
            fix_extracted_text("the navigation-\nmachines of the Guild"),
            "the navigationmachines of the Guild"
        );
    }

    #[test]
    fn merges_chained_hyphen_breaks() {
        assert_eq!(fix_extracted_text("axlotl-\ntanks"), "axlotltanks");
    }

    #[test]
    fn keeps_hyphen_when_next_line_starts_uppercase() {
        assert_eq!(
            fix_extracted_text("Dune-\nMessiah"),
            "Dune-\nMessiah"
        );
    }

    #[test]
    fn keeps_intra_line_hyphens_untouched() {
        assert_eq!(fix_extracted_text("mind-killer"), "mind-killer");
    }

    #[test]
    fn removes_soft_hyphens() {
        assert_eq!(fix_extracted_text("naviga\u{AD}-\ntion"), "navigation");
    }

    #[test]
    fn fix_is_idempotent() {
        let raw = "naviga-\ntion\nplain text stays-\nalone here";
        let once = fix_extracted_text(raw);
        assert_eq!(once, fix_extracted_text(&once));
    }
}
