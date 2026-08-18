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

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
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

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn word_count(text: &str) -> u32 {
    text.split_whitespace().count() as u32
}
