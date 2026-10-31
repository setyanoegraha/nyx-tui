//! Thin HTTP client for VulnyX. No authentication exists on the platform —
//! the only requirement is a browser-like User-Agent (default curl UAs are
//! blocked with a 403 page).
//!
//! The CAPTCHA download flow depends on both requests sharing one `PHPSESSID`
//! cookie jar: the code fetched from `/captcha.php` is only valid for the
//! session that fetched it, and a second fetch replaces it.

use std::sync::Arc;

use anyhow::{bail, Context};
use reqwest::cookie::Jar;
use reqwest::Client;

pub const BASE_URL: &str = "https://vulnyx.com";

/// Exact message used for HTTP 429 so the UI can localize it.
pub const RATE_LIMIT: &str =
    "VulnyX rate limit: at most 2 VM downloads per minute. Wait a minute and try again.";
pub const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/120.0.0.0 Safari/537.36 nyx-tui/",
    env!("CARGO_PKG_VERSION")
);

/// What the CAPTCHA submission produced: the Proton Drive link, or the HTML of
/// a rejection page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostOutcome {
    Redirect(String),
    Rejected(String),
}

/// A session-scoped HTTP client (cookies enabled for the CAPTCHA flow).
#[derive(Clone)]
pub struct NyxClient {
    client: Client,
    /// Same cookie jar, but redirects are left unfollowed so the CAPTCHA POST
    /// can read the `Location` header instead of chasing it to Proton Drive.
    no_redirect: Client,
}

impl NyxClient {
    pub fn new() -> anyhow::Result<Self> {
        let jar = Arc::new(Jar::default());
        let build = |follow: bool| -> anyhow::Result<Client> {
            let mut builder = Client::builder()
                .user_agent(USER_AGENT)
                .timeout(std::time::Duration::from_secs(60))
                .connect_timeout(std::time::Duration::from_secs(15))
                .cookie_provider(jar.clone());
            builder = if follow {
                builder.redirect(reqwest::redirect::Policy::limited(10))
            } else {
                builder.redirect(reqwest::redirect::Policy::none())
            };
            builder.build().context("Failed to build the HTTP client")
        };
        Ok(Self {
            client: build(true)?,
            no_redirect: build(false)?,
        })
    }

    /// GET a path (or absolute URL) and return the response body text.
    pub async fn get(&self, path: &str) -> anyhow::Result<String> {
        let resp = self
            .client
            .get(resolve_url(path))
            .send()
            .await
            .context("Connection error")?;
        let resp = check_status(resp).await?;
        Ok(resp.text().await?)
    }

    /// GET a path (or absolute URL) and return the raw body bytes.
    pub async fn get_bytes(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let resp = self
            .client
            .get(resolve_url(path))
            .send()
            .await
            .context("Connection error")?;
        let resp = check_status(resp).await?;
        Ok(resp.bytes().await?.to_vec())
    }

    /// POST a JSON body and return the response text.
    pub async fn post_json(&self, path: &str, body: &serde_json::Value) -> anyhow::Result<String> {
        let resp = self
            .client
            .post(resolve_url(path))
            .header("X-Requested-With", "XMLHttpRequest")
            .json(body)
            .send()
            .await
            .context("Connection error")?;
        let resp = check_status(resp).await?;
        Ok(resp.text().await?)
    }

    /// POST a form without following redirects: a `302` carries the Proton
    /// Drive link the caller wants, and a `200` is the rejection page.
    pub async fn post_form(
        &self,
        path: &str,
        form: &[(&str, &str)],
    ) -> anyhow::Result<PostOutcome> {
        let url = resolve_url(path);
        let resp = self
            .no_redirect
            .post(&url)
            .header("Referer", url)
            .form(form)
            .send()
            .await
            .context("Connection error")?;
        let resp = check_status(resp).await?;
        if resp.status().is_redirection() {
            let location = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            return match location {
                Some(location) => Ok(PostOutcome::Redirect(location)),
                None => bail!("The server redirected without a Location header."),
            };
        }
        Ok(PostOutcome::Rejected(resp.text().await?))
    }
}

/// Turns the platform's status codes into actionable errors; the callers
/// otherwise mistake a rate-limit page for a rejected flag.
async fn check_status(resp: reqwest::Response) -> anyhow::Result<reqwest::Response> {
    let status = resp.status();
    if status.is_success() || status.is_redirection() {
        return Ok(resp);
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        bail!("{RATE_LIMIT}");
    }
    bail!("VulnyX returned HTTP {}", status.as_u16())
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
