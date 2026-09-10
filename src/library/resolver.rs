use std::path::{Component, Path, PathBuf};

fn normalize_components(path: &Path) -> anyhow::Result<PathBuf> {
    let mut normalized = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if normalized.parent().is_some() {
                    normalized.pop();
                }
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

/// Finds the real, existing PDF for a book whose path was stored at index time.
///
/// The stored value may be:
/// * an absolute path that still exists (e.g. `dune index --file`); or
/// * a path relative to a configured library root (portable); or
/// * a stale absolute path from a database written on another machine or
///   before the project folder was moved (repaired by a filename search).
///
/// Resolution order:
/// 1. the stored path itself, if it resolves to an existing file;
/// 2. each configured library root joined with the stored path;
/// 3. a recursive filename search inside each configured library root.
pub fn resolve_file_path(stored: &str, library_roots: &[PathBuf]) -> Option<PathBuf> {
    let stored_path = PathBuf::from(stored);
    let normalized = normalize_components(&stored_path).ok()?;
    let norm = |p: &PathBuf| normalize_components(p).unwrap_or_else(|_| p.clone());

    if normalized.is_absolute() {
        if normalized.is_file() {
            return Some(normalized);
        }
    } else {
        for root in library_roots {
            let candidate = norm(root).join(&normalized);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    let file_name = stored_path.file_name()?;
    for root in library_roots {
        if let Some(found) = find_by_name(root, file_name) {
            return Some(found);
        }
    }

    None
}

fn find_by_name(dir: &Path, name: &std::ffi::OsStr) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_by_name(&path, name) {
                return Some(found);
            }
        } else if path.file_name() == Some(name) {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, PathBuf, Vec<PathBuf>) {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("library");
        let sub = root.join("PDFS");
        std::fs::create_dir_all(&sub).unwrap();
        let pdf = sub.join("1_Dune.pdf");
        std::fs::write(&pdf, b"%PDF").unwrap();
        (tmp, pdf, vec![root])
    }

    #[test]
    fn resolves_relative_path_from_library_root() {
        let (_tmp, pdf, roots) = fixture();
        {
            let stored = "PDFS/1_Dune.pdf";
            assert_eq!(resolve_file_path(stored, &roots), Some(pdf));
        }
    }

    #[test]
    fn resolves_relative_path_containing_dot_slash() {
        let (_tmp, pdf, roots) = fixture();
        {
            let stored = "./PDFS/1_Dune.pdf";
            assert_eq!(resolve_file_path(stored, &roots), Some(pdf));
        }
    }

    #[test]
    fn resolves_existing_absolute_path() {
        let (_tmp, pdf, _roots) = fixture();
        let some_root = vec![PathBuf::from("/nonexistent/library")];
        assert_eq!(resolve_file_path(&pdf.to_string_lossy(), &some_root), Some(pdf));
    }

    #[test]
    fn repairs_stale_absolute_path_by_filename() {
        let (_tmp, pdf, roots) = fixture();
        let stale = pdf
            .parent()
            .unwrap()
            .join("another/location/1_Dune.pdf");
        assert!(!stale.exists());
        assert_eq!(resolve_file_path(&stale.to_string_lossy(), &roots), Some(pdf));
    }

    #[test]
    fn returns_none_when_not_found() {
        let (_tmp, _pdf, roots) = fixture();
        assert_eq!(resolve_file_path("PDFS/9_Missing.pdf", &roots), None);
    }
}