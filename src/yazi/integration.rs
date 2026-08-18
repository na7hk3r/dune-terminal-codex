pub fn open_pdf(path: &str, page: Option<u32>, command: &str) -> anyhow::Result<()> {
    use std::process::Command;

    let mut cmd = Command::new(command);

    if command == "okular" {
        if let Some(p) = page {
            cmd.arg("--page").arg(p.to_string());
        }
    }

    cmd.arg(path);

    cmd.spawn()
        .map_err(|e| anyhow::anyhow!("failed to open PDF with {}: {}", command, e))?;

    Ok(())
}
