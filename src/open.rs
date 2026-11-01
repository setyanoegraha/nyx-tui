//! Opens a URL in the user's browser. The release binaries cover Linux, macOS
//! and Windows, so the opener is chosen per target.

/// Only web URLs are ever opened. Rejecting other schemes keeps
/// server-supplied links from launching arbitrary handlers and, on Windows,
/// from smuggling arguments through `cmd /C start`.
fn is_web_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

pub fn open_url(url: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};

    if !is_web_url(url) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "unsupported URL scheme",
        ));
    }

    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(url);
        command
    } else if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_child| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_schemes_are_opened() {
        assert!(is_web_url("https://vulnyx.com/download.php?vm=APex"));
        assert!(is_web_url("HTTP://example.com"));
        assert!(!is_web_url("file:///etc/passwd"));
        assert!(!is_web_url("javascript:alert(1)"));
        assert!(!is_web_url("--version"));
        assert!(!is_web_url(""));
    }
}
