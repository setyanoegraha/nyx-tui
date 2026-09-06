//! Machine download flow with the platform's CAPTCHA gate:
//!   1. GET /download.php?vm=<Name>  → HTML interstitial + PHPSESSID cookie
//!   2. GET /captcha.php             → 160x50 PNG with a 5-character code
//!   3. POST the code                → 302 redirect to a Proton Drive URL
//!
//! The .ova file is hosted on Proton Drive — the TUI opens the URL in the
//! user's browser for the actual download. The platform caps downloads at
//! 2 VMs per minute.

use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::modules::session::NyxClient;

/// Fetches the CAPTCHA interstitial for a machine and saves the CAPTCHA
/// image to `captcha_path`, ready to be shown to the user.
pub async fn fetch_captcha(
    client: &NyxClient,
    machine_name: &str,
    captcha_path: &Path,
) -> Result<String> {
    let html = client
        .get(&format!("/download.php?vm={machine_name}"))
        .await?;
    if !html.contains("captcha") {
        bail!("The download page did not present a CAPTCHA.");
    }
    // The captcha image is binary — fetch it as bytes, not text.
    let image = client
        .raw()
        .get("https://vulnyx.com/captcha.php")
        .send()
        .await
        .context("Failed to fetch the CAPTCHA image")?
        .error_for_status()
        .context("The CAPTCHA endpoint returned an error")?
        .bytes()
        .await?;
    if image.len() < 4 || image[0] != 0x89 || image[1] != b'P' {
        bail!("The CAPTCHA endpoint returned an unexpected response.");
    }
    std::fs::write(captcha_path, &image[..])
        .with_context(|| format!("Failed to write {}", captcha_path.display()))?;
    Ok(html)
}

/// Submits the CAPTCHA code and returns the Proton Drive download URL
/// (from the 302 redirect's Location header). The user opens this URL in
/// their browser to download the .ova file.
pub async fn resolve_captcha(
    client: &NyxClient,
    machine_name: &str,
    captcha_code: &str,
) -> Result<String> {
    let url = format!("/download.php?vm={machine_name}");
    let resp = client
        .raw()
        .post(resolve(url))
        .header("X-Requested-With", "XMLHttpRequest")
        .form(&[("captcha", captcha_code)])
        .send()
        .await
        .context("Connection error while submitting captcha")?;

    let status = resp.status().as_u16();
    if status == 200 {
        // HTML response — captcha was wrong or session expired
        let body = resp.text().await.unwrap_or_default();
        if body.contains("Verification failed") {
            bail!("CAPTCHA verification failed — wrong code.");
        }
        bail!("The download was rejected by the server.");
    }

    let location = resp
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    match location {
        Some(url) if url.starts_with("http") => Ok(url),
        _ => bail!("No download URL found in the server response."),
    }
}

fn resolve(path: String) -> String {
    if path.starts_with("http") {
        path
    } else {
        format!("https://vulnyx.com{path}")
    }
}
