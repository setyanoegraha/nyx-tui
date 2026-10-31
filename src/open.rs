//! Opens a URL in the user's browser. The release binaries cover Linux, macOS
//! and Windows, so the opener is chosen per target.

pub fn open_url(url: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};

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
