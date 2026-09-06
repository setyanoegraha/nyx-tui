//! Local preference: the self-declared username (used for flag & writeup
//! submissions and for computing your leaderboard position), stored in
//! ~/.nyx-tui/config.json. VulnyX has no accounts and no passwords — nothing
//! sensitive is stored here.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_DIR_NAME: &str = ".nyx-tui";
const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    username: String,
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

    /// Persists the username.
    pub fn save_username(&self, username: &str) -> Result<()> {
        let cfg = ConfigFile {
            username: username.trim().to_string(),
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
