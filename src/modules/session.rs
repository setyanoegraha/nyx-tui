//! Thin HTTP client for VulnyX. No authentication exists on the platform —
//! the only requirement is a browser-like User-Agent (default curl UAs are
//! blocked with a 403 page).

use anyhow::Context;
use reqwest::Client;

pub const BASE_URL: &str = "https://vulnyx.com";
pub const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/120.0.0.0 Safari/537.36 nyx-tui/",
    env!("CARGO_PKG_VERSION")
);

/// A stateless HTTP client (cookies enabled for the CAPTCHA download flow).
#[derive(Clone)]
pub struct NyxClient {
    client: Client,
}

impl NyxClient {
    pub fn new() -> anyhow::Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(60))
            .connect_timeout(std::time::Duration::from_secs(15))
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .context("Failed to build the HTTP client")?;
        Ok(Self { client })
    }

    /// GET a path (or absolute URL) and return the response body text.
    pub async fn get(&self, path: &str) -> anyhow::Result<String> {
        let url = resolve_url(path);
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .context("Connection error")?;
        Ok(resp.text().await?)
    }

    /// POST a JSON body and return the response text.
    pub async fn post_json(&self, path: &str, body: &serde_json::Value) -> anyhow::Result<String> {
        let url = resolve_url(path);
        let resp = self
            .client
            .post(url)
            .header("X-Requested-With", "XMLHttpRequest")
            .json(body)
            .send()
            .await
            .context("Connection error")?;
        Ok(resp.text().await?)
    }
}

fn resolve_url(path: &str) -> String {
    if path.starts_with("http") {
        path.to_string()
    } else if path.starts_with('/') {
        format!("{BASE_URL}{path}")
    } else {
        format!("{BASE_URL}/{path}")
    }
}
