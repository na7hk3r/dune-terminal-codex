use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const APP_NAME: &str = "dune";
const CONFIG_FILE: &str = "config.toml";
const DB_FILE: &str = "dune.db";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub library: LibraryConfig,

    #[serde(default)]
    pub reader: ReaderConfig,

    #[serde(default)]
    pub codex: CodexConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    #[serde(default = "default_library_paths")]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderConfig {
    #[serde(default = "default_reader_command")]
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexConfig {
    #[serde(default = "default_wiki_url")]
    pub wiki_url: String,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            paths: default_library_paths(),
        }
    }
}

impl Default for ReaderConfig {
    fn default() -> Self {
        Self {
            command: default_reader_command(),
            args: Vec::new(),
        }
    }
}

impl Default for CodexConfig {
    fn default() -> Self {
        Self {
            wiki_url: default_wiki_url(),
        }
    }
}

fn default_library_paths() -> Vec<String> {
    vec!["~/Books/Dune".to_string()]
}

fn default_reader_command() -> String {
    "okular".to_string()
}

fn default_wiki_url() -> String {
    "https://dune-api-production.up.railway.app".to_string()
}

impl Config {
    pub fn config_dir() -> anyhow::Result<PathBuf> {
        let base =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("cannot determine config directory"))?;
        Ok(base.join(APP_NAME))
    }

    pub fn data_dir() -> anyhow::Result<PathBuf> {
        let base = dirs::data_dir().ok_or_else(|| anyhow::anyhow!("cannot determine data directory"))?;
        Ok(base.join(APP_NAME))
    }

    pub fn cache_dir() -> anyhow::Result<PathBuf> {
        let base = dirs::cache_dir().ok_or_else(|| anyhow::anyhow!("cannot determine cache directory"))?;
        Ok(base.join(APP_NAME))
    }

    pub fn config_path() -> anyhow::Result<PathBuf> {
        Ok(Self::config_dir()?.join(CONFIG_FILE))
    }

    pub fn db_path() -> anyhow::Result<PathBuf> {
        Ok(Self::data_dir()?.join(DB_FILE))
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            anyhow::bail!("configuration not found at {}", path.display());
        }
        let content = fs::read_to_string(&path)?;
        let config: Config =
            toml::from_str(&content).map_err(|e| anyhow::anyhow!("invalid configuration: {}", e))?;
        Ok(config)
    }

    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<PathBuf> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir)?;
        let path = dir.join(CONFIG_FILE);
        let content = toml::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("cannot serialize configuration: {}", e))?;
        fs::write(&path, content)?;
        Ok(path)
    }

    pub fn ensure_dirs(&self) -> anyhow::Result<()> {
        fs::create_dir_all(Self::config_dir()?)?;
        fs::create_dir_all(Self::data_dir()?)?;
        fs::create_dir_all(Self::cache_dir()?)?;
        Ok(())
    }

    pub fn resolve_library_paths(&self) -> Vec<PathBuf> {
        self.library
            .paths
            .iter()
            .map(|p| {
                let expanded = shellexpand::tilde(p);
                PathBuf::from(expanded.as_ref())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        assert_eq!(config.library.paths.len(), 1);
        assert_eq!(config.reader.command, "okular");
    }

    #[test]
    fn config_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.library.paths, parsed.library.paths);
        assert_eq!(config.reader.command, parsed.reader.command);
    }

    #[test]
    fn resolve_library_paths_expands_tilde() {
        let mut config = Config::default();
        config.library.paths = vec!["~/Books/Dune".to_string()];
        let paths = config.resolve_library_paths();
        assert_eq!(paths.len(), 1);
        assert!(!paths[0].to_string_lossy().contains('~'));
    }

    #[test]
    fn save_and_load_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        let config = Config::default();
        let content = toml::to_string_pretty(&config).unwrap();
        fs::write(&config_path, content).unwrap();
        let loaded_content = fs::read_to_string(&config_path).unwrap();
        let loaded: Config = toml::from_str(&loaded_content).unwrap();
        assert_eq!(config.library.paths, loaded.library.paths);
    }
}
