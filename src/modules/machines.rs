//! Machine catalog from the public JSON feed
//! (`GET /data/machines-index.json`, browser User-Agent required).

use anyhow::Result;
use serde::Deserialize;

use crate::modules::session::NyxClient;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Machine {
    pub slug: String,
    pub name: String,
    pub creator: String,
    pub os: String,
    pub difficulty: String,
    pub release_date: String,
    pub platforms: Vec<String>,
    pub size: String,
    pub md5: String,
    pub download_url: String,
    pub first_user: String,
    pub first_root: String,
    pub tech_tags: Vec<String>,
    pub pts_user: u64,
    pub pts_root: u64,
}

impl Machine {
    /// A first-blood slot is open when the platform records no holder yet.
    pub fn user_slot_open(&self) -> bool {
        self.first_user.trim().is_empty()
    }

    pub fn root_slot_open(&self) -> bool {
        self.first_root.trim().is_empty()
    }

    pub fn any_slot_open(&self) -> bool {
        self.user_slot_open() || self.root_slot_open()
    }
}

pub struct MachineFetcher {
    client: NyxClient,
}

impl MachineFetcher {
    pub fn new(client: NyxClient) -> Self {
        Self { client }
    }

    pub async fn fetch(&self) -> Result<Vec<Machine>> {
        let body = self.client.get("/data/machines-index.json").await?;
        Ok(serde_json::from_str(&body)?)
    }
}

/// "2025-01-22" -> (2025, 1, 22) for chronological sorting; unparseable
/// dates sort as the oldest possible.
pub fn release_sort_key(date: &str) -> (u32, u32, u32) {
    let parts: Vec<u32> = date
        .split('-')
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    match parts.as_slice() {
        [y, m, d] => (*y, *m, *d),
        _ => (0, 0, 0),
    }
}

/// Difficulty rank for sorting (Low first, matching the site's ladder).
pub fn difficulty_rank(difficulty: &str) -> u8 {
    match difficulty.trim().to_lowercase().as_str() {
        "low" => 0,
        "easy" => 1,
        "medium" => 2,
        "hard" => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_machine_fixture() {
        let json = r#"{
            "slug": "apex", "name": "APex", "creator": "d4t4s3c",
            "os": "Linux", "difficulty": "Easy", "release_date": "2025-01-22",
            "platforms": ["VirtualBox", "VMware"], "size": "1.2GB",
            "md5": "616D84A31B380C79CCEED4886E5448E2",
            "download_url": "https://vulnyx.com/download.php?vm=APex",
            "first_user": "suraxddq", "first_root": "",
            "summary": {"en": "", "es": ""},
            "tech_tags": ["osint", "finger"],
            "pts_root": 30, "pts_user": 20
        }"#;
        let machine: Machine = serde_json::from_str(json).unwrap();
        assert_eq!(machine.name, "APex");
        assert_eq!(machine.os, "Linux");
        assert_eq!(machine.platforms.len(), 2);
        assert!(!machine.user_slot_open(), "first_user taken");
        assert!(machine.root_slot_open(), "first_root open");
        assert_eq!(machine.tech_tags, vec!["osint", "finger"]);
    }

    #[test]
    fn sorts_by_date_and_difficulty() {
        assert!(release_sort_key("2025-01-22") > release_sort_key("2024-12-01"));
        assert_eq!(release_sort_key("basura"), (0, 0, 0));
        assert!(difficulty_rank("Low") < difficulty_rank("Easy"));
        assert!(difficulty_rank("Easy") < difficulty_rank("Medium"));
        assert!(difficulty_rank("Medium") < difficulty_rank("Hard"));
    }
}
