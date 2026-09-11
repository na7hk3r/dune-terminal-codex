use std::path::{Path, PathBuf};

pub fn scan_directory(dir: &Path) -> Vec<PathBuf> {
    let mut pdfs = Vec::new();
    if !dir.is_dir() {
        return pdfs;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pdfs.extend(scan_directory(&path));
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("pdf"))
                .unwrap_or(false)
            {
                pdfs.push(path);
            }
        }
    }
    pdfs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    #[test]
    fn scan_finds_pdfs_in_nested_directories() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        write(&root.join("a.pdf"), b"%PDF-1.4");
        write(&root.join("sub/b.pdf"), b"%PDF-1.4");
        write(&root.join("sub/deep/c.pdf"), b"%PDF-1.4");
        write(&root.join("notes.txt"), b"not a pdf");

        let found = scan_directory(root);
        let mut names: Vec<PathBuf> = found
            .iter()
            .map(|p| p.strip_prefix(root).unwrap().to_path_buf())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                PathBuf::from("a.pdf"),
                PathBuf::from("sub/b.pdf"),
                PathBuf::from("sub/deep/c.pdf"),
            ],
            "nested scan must recurse and skip non-PDF files"
        );
    }

    #[test]
    fn scan_matches_extensions_case_insensitively() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        write(&root.join("upper.PDF"), b"%PDF-1.4");
        write(&root.join("mixed.Pdf"), b"%PDF-1.4");
        write(&root.join("lower.pdf"), b"%PDF-1.4");
        write(&root.join("fake.pdf.txt"), b"trick extension");

        let mut found: Vec<String> = scan_directory(root)
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        found.sort();
        assert_eq!(
            found,
            vec!["lower.pdf", "mixed.Pdf", "upper.PDF"],
            "extension matching must be case-insensitive and not fooled by suffix"
        );
    }

    #[test]
    fn scan_missing_directory_returns_empty() {
        let missing = Path::new("/nonexistent/dune/browser/scan-target");
        assert!(!missing.exists());
        assert!(scan_directory(missing).is_empty());
    }

    #[test]
    fn scan_empty_directory_returns_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(scan_directory(tmp.path()).is_empty());
    }
}
