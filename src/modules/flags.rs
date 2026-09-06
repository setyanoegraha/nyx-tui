//! First-blood flag submission (`POST /includes/flag.php`). No account
//! needed — a self-declared username plus the MD5 flag found inside the
//! machine (user.txt / root.txt). First bloods are recorded publicly.
//! Rapid submissions are rate-limited by the platform (HTTP 429).

use anyhow::{bail, Result};

use crate::modules::session::NyxClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagType {
    User,
    Root,
}

impl FlagType {
    pub fn as_str(self) -> &'static str {
        match self {
            FlagType::User => "user",
            FlagType::Root => "root",
        }
    }
}

pub struct FlagManager {
    client: NyxClient,
}

impl FlagManager {
    pub fn new(client: NyxClient) -> Self {
        Self { client }
    }

    /// Submits a flag (32-char MD5 hex) for a machine under `username`.
    /// Returns the server message; a first blood may carry a bonus note.
    pub async fn submit(
        &self,
        machine_slug: &str,
        flag_type: FlagType,
        username: &str,
        flag_value: &str,
    ) -> Result<String> {
        let flag = flag_value.trim();
        if flag.len() != 32 || !flag.chars().all(|c| c.is_ascii_hexdigit()) {
            bail!("The flag must be an MD5 hash: 32 hexadecimal characters.");
        }
        if username.trim().len() < 2 {
            bail!("Set your username first (a).");
        }
        let body = self
            .client
            .post_json(
                "/includes/flag.php",
                &serde_json::json!({
                    "machine": machine_slug,
                    "flag_type": flag_type.as_str(),
                    "username": username.trim(),
                    "flag_value": flag.to_lowercase(),
                }),
            )
            .await?;
        parse_flag_response(&body)
    }
}

/// The endpoint answers `{"ok": bool, ...}` with optional bonus/message
/// fields; HTTP-level errors (429 rate limit) surface as connection errors
/// upstream, so here we only interpret the body.
pub fn parse_flag_response(body: &str) -> Result<String> {
    let value: serde_json::Value =
        serde_json::from_str(body).unwrap_or(serde_json::Value::String(body.trim().to_string()));
    if value.get("ok").and_then(serde_json::Value::as_bool) == Some(true) {
        let mut message = value
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Flag accepted")
            .to_string();
        if let Some(bonus) = value
            .get("first_blood_bonus")
            .and_then(serde_json::Value::as_u64)
        {
            message.push_str(&format!(" (+{bonus} first blood bonus pts)"));
        }
        Ok(message)
    } else {
        let message = value
            .get("message")
            .or_else(|| value.get("error"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("The server rejected the flag.");
        bail!("{message}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_flag_response_accepts_with_bonus() {
        let message =
            parse_flag_response(r#"{"ok":true,"message":"First blood!","first_blood_bonus":20}"#)
                .unwrap();
        assert!(message.contains("First blood!"));
        assert!(message.contains("+20"));
    }

    #[test]
    fn parse_flag_response_rejects() {
        assert!(parse_flag_response(r#"{"ok":false,"message":"Wrong flag"}"#).is_err());
    }
}
