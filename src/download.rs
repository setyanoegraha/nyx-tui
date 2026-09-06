//! Machine download flow with the platform's CAPTCHA gate:
//!   1. GET /download.php?vm=<Name>  → HTML interstitial + PHPSESSID cookie
//!   2. GET /captcha.php             → 160x50 PNG with a 5-character code
//!   3. the user types the code (or the built-in solver reads it)
//!   4. POST the code back to the same URL → the .ova streams as the reply
//!
//! The archive is staged as `<name>.part` and MD5-verified against the
//! catalog before being renamed. The platform caps downloads at 2 VMs per
//! minute — respected by design (the CAPTCHA gate throttles naturally).

use anyhow::{bail, Context, Result};
use md5::{Digest, Md5};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::modules::session::NyxClient;

/// Callbacks handed to [`download_ova`]: the terminal/TUI renders through
/// these hooks — this module never touches the screen.
pub struct DownloadHooks<'a> {
    /// Set to true to abort; the `.part` file is cleaned up.
    pub cancel: &'a AtomicBool,
    /// Called once after metadata: (total bytes, file name).
    pub on_metadata: &'a (dyn Fn(u64, &str) + Send + Sync),
    /// Called on every chunk with the absolute number of bytes written.
    pub on_progress: &'a (dyn Fn(u64) + Send + Sync),
    /// Called once when the `.part` staging file is created.
    pub on_part: &'a (dyn Fn(&Path) + Send + Sync),
    /// Called when the CAPTCHA image has been saved and should be shown to
    /// the user (opened in an external viewer).
    pub on_captcha: &'a (dyn Fn(&Path) + Send + Sync),
}

/// Fetches the CAPTCHA interstitial for a machine and saves the CAPTCHA
/// image to `captcha_path`, ready to be shown to the user. Returns the
/// download page HTML (parsed by the caller for the form target).
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

/// Submits the CAPTCHA code and streams the `.ova` reply to disk, staged
/// through a `.part` file and MD5-verified against the catalog value.
pub async fn download_ova(
    client: &NyxClient,
    machine_name: &str,
    captcha_code: &str,
    expected_md5: &str,
    destination: &Path,
    hooks: &DownloadHooks<'_>,
) -> Result<PathBuf> {
    let url = format!("/download.php?vm={machine_name}");
    let resp = client
        .raw()
        .post(resolve(url))
        .header("X-Requested-With", "XMLHttpRequest")
        .form(&[("captcha", captcha_code)])
        .send()
        .await
        .context("Connection error while downloading")?;

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if content_type.contains("text/html") {
        // The server answers the rejected code with the interstitial again.
        let body = resp.text().await.unwrap_or_default();
        if body.contains("Verification failed") {
            bail!("CAPTCHA verification failed — wrong code.");
        }
        bail!("The download was rejected by the server (HTML response).");
    }
    let resp = resp.error_for_status().context("Download rejected")?;

    let total = resp.content_length().unwrap_or(0);
    let filename = format!("{machine_name}.ova");
    let output = destination.join(&filename);
    if output.exists() {
        return Ok(output);
    }

    let part = destination.join(format!("{filename}.part"));
    (hooks.on_part)(&part);
    (hooks.on_metadata)(total, &filename);

    let result = stream_to_file(resp, &part, expected_md5, hooks).await;
    match result {
        Ok(()) => {
            tokio::fs::rename(&part, &output)
                .await
                .context("Failed to finalize the download")?;
            Ok(output)
        }
        Err(error) => {
            let _ = tokio::fs::remove_file(&part).await;
            Err(error)
        }
    }
}

async fn stream_to_file(
    resp: reqwest::Response,
    part: &Path,
    expected_md5: &str,
    hooks: &DownloadHooks<'_>,
) -> Result<()> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let total = resp.content_length().unwrap_or(0);
    let mut stream = resp.bytes_stream();
    let mut file = tokio::fs::File::create(part)
        .await
        .context("Failed to create the staging file")?;
    let mut hasher = Md5::new();
    let mut written = 0u64;

    while let Some(chunk) = stream.next().await {
        if hooks.cancel.load(std::sync::atomic::Ordering::Relaxed) {
            bail!("Download cancelled.");
        }
        let chunk = chunk.context("Network error during the download")?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        written += chunk.len() as u64;
        (hooks.on_progress)(written);
    }
    file.flush().await?;

    if total > 0 && written != total {
        bail!("Incomplete download: {written} of {total} bytes");
    }
    let digest = format!("{:X}", hasher.finalize());
    if !expected_md5.is_empty()
        && !digest.eq_ignore_ascii_case(expected_md5.trim())
    {
        bail!(
            "MD5 mismatch: expected {expected_md5}, got {digest} — the download is corrupt."
        );
    }
    Ok(())
}

fn resolve(path: String) -> String {
    if path.starts_with("http") {
        path
    } else {
        format!("https://vulnyx.com{path}")
    }
}
