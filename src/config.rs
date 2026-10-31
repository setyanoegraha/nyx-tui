//! Local preferences: the self-declared username (used for flag & writeup
//! submissions and for computing your leaderboard position), the machines
//! manually marked as completed, and the interface language — stored in
//! ~/.nyx-tui/config.json. VulnyX has no accounts and no passwords — nothing
//! sensitive is stored here.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_DIR_NAME: &str = ".nyx-tui";
const CONFIG_FILE_NAME: &str = "config.json";

fn default_language() -> String {
    "en".to_string()
}

#[derive(Serialize, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    username: String,
    /// Machine slugs manually marked as completed in the TUI (`m`).
    #[serde(default)]
    completed: Vec<String>,
    /// Interface language ("en" | "es"), toggled with `l`.
    #[serde(default = "default_language")]
    language: String,
    /// Where Proton Drive archives are saved; empty means the default.
    #[serde(default)]
    download_dir: String,
    /// Unknown keys from other versions are preserved on save.
    #[serde(flatten, skip_serializing_if = "BTreeMap::is_empty")]
    extra: BTreeMap<String, serde_json::Value>,
}

pub struct ConfigManager {
    config_file: PathBuf,
}

impl ConfigManager {
    /// Default manager rooted at the user's home directory.
    pub fn new() -> Self {
        let dir = home::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(CONFIG_DIR_NAME);
        Self::in_dir(dir)
    }

    /// Manager rooted at an explicit directory (used by tests).
    pub fn in_dir(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        Self {
            config_file: dir.join(CONFIG_FILE_NAME),
        }
    }

    fn read_config(&self) -> Option<ConfigFile> {
        let raw = match fs::read_to_string(&self.config_file) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
            Err(error) => {
                eprintln!(
                    "[!] ignoring unreadable {}: {error}",
                    self.config_file.display()
                );
                return None;
            }
        };
        match serde_json::from_str(&raw) {
            Ok(cfg) => Some(cfg),
            Err(error) => {
                // Keep the damaged file around: the next save would otherwise
                // silently erase the username and completed list.
                eprintln!(
                    "[!] ignoring unreadable {}: {error}",
                    self.config_file.display()
                );
                let _ = fs::rename(
                    &self.config_file,
                    self.config_file.with_extension("json.corrupt"),
                );
                None
            }
        }
    }

    /// Writes through a temporary file and renames, so a crash mid-write
    /// leaves either the old or the new config, never a truncated one.
    fn save(&self, cfg: &ConfigFile) -> Result<()> {
        let tmp = self.config_file.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string(cfg)?)
            .with_context(|| "Failed to write the configuration file")?;
        fs::rename(&tmp, &self.config_file)
            .with_context(|| "Failed to replace the configuration file")?;
        Ok(())
    }

    /// The self-declared username used for flag & writeup submissions.
    pub fn username(&self) -> String {
        self.read_config()
            .map(|cfg| cfg.username)
            .unwrap_or_default()
    }

    /// The interface language; invalid values fall back to English.
    pub fn language(&self) -> crate::i18n::Lang {
        crate::i18n::Lang::from_code(
            &self
                .read_config()
                .map(|cfg| cfg.language)
                .unwrap_or_default(),
        )
    }

    /// Persists the interface language.
    pub fn save_language(&self, lang: crate::i18n::Lang) -> Result<()> {
        let mut cfg = self.read_config().unwrap_or_default();
        cfg.language = lang.as_str().to_string();
        self.save(&cfg)
    }

    pub fn completed_machines(&self) -> Vec<String> {
        self.read_config()
            .map(|cfg| cfg.completed)
            .unwrap_or_default()
    }

    /// Persists the manual completed-machines list (slugs).
    pub fn save_completed_machines(&self, slugs: Vec<String>) -> Result<()> {
        let mut cfg = self.read_config().unwrap_or_default();
        cfg.completed = slugs;
        self.save(&cfg)
    }

    /// Persists the username.
    pub fn save_username(&self, username: &str) -> Result<()> {
        let mut cfg = self.read_config().unwrap_or_default();
        cfg.username = username.trim().to_string();
        self.save(&cfg)
    }

    /// Directory where machine archives land: the configured path, else
    /// `~/Downloads/nyx`. Created on demand so a first download never fails
    /// because the folder is missing.
    pub fn download_dir(&self) -> PathBuf {
        let configured = self
            .read_config()
            .map(|cfg| cfg.download_dir)
            .unwrap_or_default();
        let dir = if configured.trim().is_empty() {
            home::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Downloads")
                .join("nyx")
        } else {
            PathBuf::from(configured.trim())
        };
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// Persists the download directory (already validated by
    /// [`resolve_download_dir`]).
    pub fn save_download_dir(&self, dir: &Path) -> Result<()> {
        let mut cfg = self.read_config().unwrap_or_default();
        cfg.download_dir = dir.display().to_string();
        self.save(&cfg)
    }
}

/// Expands a leading `~`, rejects anything not absolute, and creates the
/// directory — the TUI stores it in the config, so a relative value would
/// otherwise resolve against whatever directory nyx happens to run from.
pub fn resolve_download_dir(input: &str) -> Result<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        anyhow::bail!("an absolute path is required");
    }
    let expanded = if trimmed == "~" {
        home::home_dir().unwrap_or_else(|| PathBuf::from("/"))
    } else if let Some(rest) = trimmed.strip_prefix("~/") {
        home::home_dir()
            .unwrap_or_else(|| PathBuf::from("/"))
            .join(rest)
    } else {
        PathBuf::from(trimmed)
    };
    if !expanded.is_absolute() {
        anyhow::bail!("{} is not an absolute path", expanded.display());
    }
    fs::create_dir_all(&expanded)
        .with_context(|| format!("Cannot create {}", expanded.display()))?;
    Ok(expanded)
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nyx-config-test-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn completed_roundtrip_and_username_preserved() {
        let dir = temp_dir("roundtrip");
        let manager = ConfigManager::in_dir(dir.clone());
        manager.save_username("tester").unwrap();
        manager
            .save_completed_machines(vec!["alpha".into(), "beta".into()])
            .unwrap();
        let manager = ConfigManager::in_dir(dir.clone());
        assert_eq!(manager.username(), "tester");
        assert_eq!(manager.completed_machines(), vec!["alpha", "beta"]);
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn legacy_config_without_completed_key_reads_empty() {
        let dir = temp_dir("legacy");
        fs::write(dir.join(CONFIG_FILE_NAME), r#"{"username":"oldschool"}"#).unwrap();
        let manager = ConfigManager::in_dir(dir.clone());
        assert_eq!(manager.username(), "oldschool");
        assert!(manager.completed_machines().is_empty());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn resolve_download_dir_requires_an_absolute_path() {
        let error = resolve_download_dir("relative/path").unwrap_err();
        assert!(format!("{error:#}").contains("not an absolute path"));
        let error = resolve_download_dir("   ").unwrap_err();
        assert!(format!("{error:#}").contains("absolute path is required"));

        let absolute = std::env::temp_dir().join(format!("nyx-dir-test-{}", std::process::id()));
        let resolved = resolve_download_dir(&absolute.display().to_string()).unwrap();
        assert_eq!(resolved, absolute);
        assert!(resolved.is_dir());
        fs::remove_dir_all(&resolved).ok();
    }

    #[test]
    fn resolve_download_dir_expands_home() {
        let Some(home) = home::home_dir() else {
            return;
        };
        let resolved = resolve_download_dir("~/nyx-tilde-test").unwrap();
        assert_eq!(resolved, home.join("nyx-tilde-test"));
        fs::remove_dir_all(&resolved).ok();
        assert_eq!(resolve_download_dir("~").unwrap(), home);
    }

    #[test]
    fn saved_download_dir_roundtrips_and_keeps_other_keys() {
        let dir = temp_dir("download-dir");
        fs::write(dir.join(CONFIG_FILE_NAME), r#"{"username":"u","future":7}"#).unwrap();
        let manager = ConfigManager::in_dir(dir.clone());
        let target = dir.join("archives");
        manager.save_download_dir(&target).unwrap();
        assert_eq!(manager.download_dir(), target);
        let raw = fs::read_to_string(dir.join(CONFIG_FILE_NAME)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(value["username"], "u");
        assert_eq!(value["future"], 7);
        assert_eq!(value["download_dir"], target.display().to_string());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn unknown_keys_survive_a_save() {
        let dir = temp_dir("extra");
        fs::write(
            dir.join(CONFIG_FILE_NAME),
            r#"{"username":"u","download_dir":"/tmp/labs","future":42}"#,
        )
        .unwrap();
        let manager = ConfigManager::in_dir(dir.clone());
        manager
            .save_completed_machines(vec!["zeta".into()])
            .unwrap();
        let raw = fs::read_to_string(dir.join(CONFIG_FILE_NAME)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(value["download_dir"], "/tmp/labs");
        assert_eq!(value["future"], 42);
        assert_eq!(value["completed"][0], "zeta");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn language_roundtrip_and_survives_other_saves() {
        let dir = temp_dir("language");
        let manager = ConfigManager::in_dir(dir.clone());
        assert_eq!(manager.language(), crate::i18n::Lang::En);
        manager.save_username("tester").unwrap();
        manager.save_language(crate::i18n::Lang::Es).unwrap();
        manager
            .save_completed_machines(vec!["alpha".into()])
            .unwrap();
        manager.save_username("renamed").unwrap();
        let manager = ConfigManager::in_dir(dir.clone());
        assert_eq!(manager.language(), crate::i18n::Lang::Es);
        assert_eq!(manager.username(), "renamed");
        assert_eq!(manager.completed_machines(), vec!["alpha"]);
        fs::remove_dir_all(dir).ok();
    }
}
