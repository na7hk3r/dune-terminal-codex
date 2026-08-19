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
