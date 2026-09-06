//! Community writeups: the full map (`GET /data/writeups-map.json`) and
//! writeup submission (`POST /includes/writeup.php`, self-declared username,
//! human-reviewed by the platform).

use anyhow::{bail, Result};
use serde::Deserialize;
use std::collections::HashMap;

use crate::modules::session::NyxClient;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct WriteupEntry {
    pub author: String,
    pub url: String,
    #[serde(rename = "type")]
    pub tipo: String, // "Text" | "Video"
    pub language: String, // "es" | "en" | "uz" | ...
    pub submitted_at: String,
}

pub struct WriteupManager {
    client: NyxClient,
}

impl WriteupManager {
    pub fn new(client: NyxClient) -> Self {
        Self { client }
    }

    /// Fetches the whole writeups map, flattened per machine. The embedded
    /// `writeups` arrays in the catalog are partial — this map is the
    /// authoritative source.
    pub async fn fetch_all(&self) -> Result<Vec<(String, WriteupEntry)>> {
        let body = self.client.get("/data/writeups-map.json").await?;
        let map: HashMap<String, Vec<WriteupEntry>> = serde_json::from_str(&body)?;
        let mut flat: Vec<(String, WriteupEntry)> = map
            .into_iter()
            .flat_map(|(slug, entries)| {
                entries
                    .into_iter()
                    .map(move |mut w| {
                        if w.author.is_empty() {
                            w.author = slug.clone();
                        }
                        (slug.clone(), w)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        flat.sort_by_key(|a| a.0.to_lowercase());
        Ok(flat)
    }

    /// Submits a writeup URL for a machine. `tipo` is "Text" or "Video",
    /// `language` one of the platform's accepted codes (es/en/fr/de/pt/zh/other).
    /// Submissions are human-reviewed within 48h.
    pub async fn submit(
        &self,
        machine_slug: &str,
        username: &str,
        url: &str,
        tipo: &str,
        language: &str,
    ) -> Result<String> {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            bail!("The writeup URL must start with http:// or https://.");
        }
        if !matches!(tipo, "Text" | "Video") {
            bail!("Type must be Text or Video.");
        }
        let body = self
            .client
            .post_json(
                "/includes/writeup.php",
                &serde_json::json!({
                    "machine": machine_slug,
                    "creator": username,
                    "url": url.trim(),
                    "type": tipo,
                    "language": language,
                }),
            )
            .await?;
        parse_ok(&body, "Writeup submitted — pending admin review (48h).")
    }
}

/// VulnyX answers `{"ok": bool, "message"?: String}` (or an `error` string).
/// Returns the server message when ok, else an error.
pub fn parse_ok(body: &str, ok_message: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(body)
        .unwrap_or(serde_json::Value::String(body.trim().to_string()));
    if value.get("ok").and_then(serde_json::Value::as_bool) == Some(true) {
        Ok(value
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(ok_message)
            .to_string())
    } else {
        let message = value
            .get("message")
            .or_else(|| value.get("error"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("The server rejected the operation.");
        bail!("{message}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_writeup_entry() {
        let json = r#"{"author": "noneofyour",
            "url": "https://github.com/setyanoegraha/vulnyx-writeups/blob/main/build/build.md",
            "type": "Text", "language": "en", "platform": "",
            "submitted_at": "2026-08-08 15:27"}"#;
        let entry: WriteupEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.author, "noneofyour");
        assert_eq!(entry.tipo, "Text");
        assert_eq!(entry.submitted_at, "2026-08-08 15:27");
    }

    #[test]
    fn parse_ok_accepts_and_rejects() {
        assert_eq!(
            parse_ok(r#"{"ok":true,"message":"nice"}"#, "fallback").unwrap(),
            "nice"
        );
        assert!(parse_ok(r#"{"ok":false,"error":"nope"}"#, "fallback").is_err());
    }
}
