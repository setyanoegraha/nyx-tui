//! Local preferences: the self-declared username (used for flag & writeup
//! submissions and for computing your leaderboard position) and the last
//! download folder, stored in ~/.nyx-tui/config.json. VulnyX has no accounts
//! and no passwords — nothing sensitive is stored here.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_DIR_NAME: &str = ".nyx-tui";
const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    download_dir: Option<String>,
}

pub struct ConfigManager {
    config_file: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let dir = home::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(CONFIG_DIR_NAME);
        let _ = fs::create_dir_all(&dir);
        Self {
            config_file: dir.join(CONFIG_FILE_NAME),
        }
    }

    fn read_config(&self) -> Option<ConfigFile> {
        let raw = fs::read_to_string(&self.config_file).ok()?;
        serde_json::from_str(&raw).ok()
    }

    /// The self-declared username used for flag & writeup submissions.
    pub fn username(&self) -> String {
        self.read_config()
            .map(|cfg| cfg.username)
            .unwrap_or_default()
    }

    /// Persists the username, preserving other fields.
    pub fn save_username(&self, username: &str) -> Result<()> {
        let cfg = ConfigFile {
            username: username.trim().to_string(),
            download_dir: self.read_config().and_then(|c| c.download_dir),
        };
        fs::write(&self.config_file, serde_json::to_string(&cfg)?)
            .with_context(|| "Failed to write the configuration file")?;
        Ok(())
    }

    /// Last download directory the user chose (survives restarts).
    pub fn download_dir(&self) -> Option<PathBuf> {
        self.read_config()
            .and_then(|cfg| cfg.download_dir)
            .map(PathBuf::from)
    }

    /// Persists the chosen download directory, preserving other fields.
    pub fn save_download_dir(&self, dir: &Path) -> Result<()> {
        let cfg = ConfigFile {
            username: self.username(),
            download_dir: Some(dir.display().to_string()),
        };
        fs::write(&self.config_file, serde_json::to_string(&cfg)?)
            .with_context(|| "Failed to write the configuration file")?;
        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
