//! The machine download gate: one HTTP session per press of `d`.
//!
//! VulnyX serves an interstitial (`download.php?vm=<Name>`) that sets a
//! `PHPSESSID`, an image CAPTCHA (`captcha.php`) bound to that session, and on
//! a correct code a `302` to the Proton Drive URL. A second `captcha.php` fetch
//! replaces the code, so the flow never reloads: fetch page, fetch image, solve,
//! POST once. The platform allows 2 downloads per minute and nyx does not retry
//! — an over-limit POST surfaces as HTTP 429 from [`NyxClient`].

use anyhow::Result;

use crate::captcha;
use crate::modules::session::{NyxClient, PostOutcome, BASE_URL};

/// Result of a download attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Download {
    /// Proton Drive URL for the machine archive; the TUI opens it.
    Link(String),
    /// The CAPTCHA could not be completed in-app; open `url` in the browser.
    BrowserFallback { url: String, reason: String },
}

/// Runs the whole CAPTCHA gate in one session and returns the Proton Drive
/// link, or the browser fallback with the reason the automatic read failed.
pub async fn download_machine(
    client: &NyxClient,
    name: &str,
    download_url: &str,
) -> Result<Download> {
    let url = if download_url.is_empty() {
        format!("{BASE_URL}/download.php?vm={name}")
    } else {
        download_url.to_string()
    };

    let page = client.get(&url).await?;
    if !page.contains("captcha.php") {
        // A machine whose gate is disabled links straight to Proton Drive.
        if page.contains("drive.proton.me") {
            if let Some(link) = find_proton_link(&page) {
                return Ok(Download::Link(link));
            }
        }
        return Ok(Download::BrowserFallback {
            url,
            reason: "download page did not present a CAPTCHA".to_string(),
        });
    }

    let png = client.get_bytes(&format!("{BASE_URL}/captcha.php")).await?;
    if png.len() < 8 || &png[..4] != b"\x89PNG" {
        return Ok(Download::BrowserFallback {
            url,
            reason: "CAPTCHA endpoint did not return an image".to_string(),
        });
    }

    let Some(code) = captcha::solve(&png) else {
        return Ok(Download::BrowserFallback {
            url,
            reason: "could not read the CAPTCHA image".to_string(),
        });
    };

    let path = url
        .split_once(BASE_URL)
        .map(|(_, rest)| rest.to_string())
        .unwrap_or_else(|| format!("/download.php?vm={name}"));
    match client
        .post_form(&path, &[("captcha", code.as_str())])
        .await?
    {
        PostOutcome::Redirect(link) if link.starts_with("http") => Ok(Download::Link(link)),
        PostOutcome::Redirect(_) => Ok(Download::BrowserFallback {
            url,
            reason: "unexpected response".to_string(),
        }),
        PostOutcome::Rejected(body) => Ok(Download::BrowserFallback {
            url,
            reason: if body.contains("Verification failed") {
                "CAPTCHA verification failed".to_string()
            } else {
                "unexpected response".to_string()
            },
        }),
    }
}

/// Strips path separators and control characters from a server-supplied file
/// name so a Proton Drive response cannot escape the download directory.
pub fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|character| *character != '/' && *character != '\\' && !character.is_control())
        .collect();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        "machine.ova".to_string()
    } else {
        cleaned
    }
}

/// First absolute Proton Drive link in the page, if any.
fn find_proton_link(page: &str) -> Option<String> {
    let start = page.find("https://drive.proton.me/")?;
    let rest = &page[start..];
    let end = rest
        .find(['"', '\'', '<', ' ', '\n'])
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_filename_blocks_traversal() {
        assert_eq!(sanitize_filename("APex.ova"), "APex.ova");
        assert_eq!(sanitize_filename("../../etc/passwd"), "....etcpasswd");
        assert_eq!(sanitize_filename("a/b\\c"), "abc");
        assert_eq!(sanitize_filename(""), "machine.ova");
        assert_eq!(sanitize_filename(".."), "machine.ova");
        assert_eq!(sanitize_filename("bad\u{0}name.ova"), "badname.ova");
    }
}
