//! Background download jobs for the TUI. Each job runs on the tokio
//! runtime and reports through shared state the renderer reads directly.
//! The CAPTCHA adds a phase: the job fetches the image, opens it in an
//! external viewer and waits for the user's code in `code` — supplied by
//! the Captcha popup through the shared slot.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use tokio::sync::Notify;

use crate::download::{download_ova, fetch_captcha, DownloadHooks};
use crate::modules::session::NyxClient;

/// At most two machine archives are pulled in parallel (and the platform
/// itself caps downloads at 2 per minute).
pub const PARALLEL_DOWNLOADS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Resolving,
    AwaitingCaptcha,
    Downloading,
    Done,
    Failed,
    Cancelled,
}

/// Live progress, shared between the download task and the renderer.
#[derive(Debug)]
pub struct DownloadState {
    pub phase: Phase,
    pub downloaded: u64,
    pub total: u64,
    pub speed_bps: u64,
    pub message: String,
    pub captcha_path: Option<PathBuf>,
    pub part_path: Option<PathBuf>,
    last_bytes: u64,
    last_time: Option<Instant>,
}

impl Default for DownloadState {
    fn default() -> Self {
        Self {
            phase: Phase::Resolving,
            downloaded: 0,
            total: 0,
            speed_bps: 0,
            message: String::new(),
            captcha_path: None,
            part_path: None,
            last_bytes: 0,
            last_time: None,
        }
    }
}

impl DownloadState {
    fn note_progress(&mut self, bytes: u64) {
        let now = Instant::now();
        if let Some(last) = self.last_time {
            let dt = now.duration_since(last).as_secs_f64();
            if dt >= 0.25 {
                self.speed_bps = (bytes.saturating_sub(self.last_bytes) as f64 / dt) as u64;
                self.last_bytes = bytes;
                self.last_time = Some(now);
            }
        } else {
            self.last_time = Some(now);
        }
        self.downloaded = bytes;
    }
}

#[derive(Debug)]
pub struct DownloadJob {
    pub machine: String,
    pub dest_dir: PathBuf,
    pub state: Arc<Mutex<DownloadState>>,
    pub cancel: Arc<AtomicBool>,
    /// The CAPTCHA code typed by the user; the task polls this slot while
    /// in the AwaitingCaptcha phase.
    code: Arc<Mutex<Option<String>>>,
    code_notify: Arc<Notify>,
}

impl DownloadJob {
    pub fn is_active(&self) -> bool {
        matches!(
            self.state.lock().unwrap().phase,
            Phase::Resolving | Phase::AwaitingCaptcha | Phase::Downloading
        )
    }

    pub fn is_awaiting_captcha(&self) -> bool {
        self.state.lock().unwrap().phase == Phase::AwaitingCaptcha
    }

    /// Supplies the CAPTCHA code typed by the user.
    pub fn submit_code(&self, code: String) {
        *self.code.lock().unwrap() = Some(code);
        self.code_notify.notify_one();
    }

    /// Requests cancellation; the task cleans up its `.part` file.
    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.code_notify.notify_one();
    }

    /// Best-effort `.part` cleanup used when quitting with active jobs.
    pub fn remove_part(&self) {
        if let Some(part) = self.state.lock().unwrap().part_path.clone() {
            let _ = std::fs::remove_file(part);
        }
    }
}

/// Spawns the download task for a machine. The CAPTCHA image is fetched,
/// opened in an external viewer, and the code must be supplied via
/// [`DownloadJob::submit_code`].
pub fn start_download(
    client: NyxClient,
    machine: String,
    machine_slug: String,
    machine_md5: String,
    dest_dir: PathBuf,
) -> Result<DownloadJob> {
    crate::config::ConfigManager::new().save_download_dir(&dest_dir)?;
    std::fs::create_dir_all(&dest_dir)
        .with_context(|| format!("Failed to create {}", dest_dir.display()))?;

    let state = Arc::new(Mutex::new(DownloadState::default()));
    let cancel = Arc::new(AtomicBool::new(false));
    let code = Arc::new(Mutex::new(None::<String>));
    let code_notify = Arc::new(Notify::new());

    let task_state = state.clone();
    let task_cancel = cancel.clone();
    let task_code = code.clone();
    let task_notify = code_notify.clone();
    let task_dest = dest_dir.clone();
    let task_machine = machine.clone();
    let task_md5 = machine_md5.clone();
    // The task runs detached: cancellation goes through the shared flag and
    // `.part` cleanup through DownloadJob::remove_part.
    tokio::spawn(async move {
        let outcome = async {
            let captcha_path = std::env::temp_dir().join(format!(
                "nyx-captcha-{}-{}.png",
                std::process::id(),
                machine_slug
            ));
            // The progress/rendering hooks live inside the task.
            let hooks = DownloadHooks {
                cancel: &task_cancel,
                on_metadata: &|total, _name| {
                    task_state.lock().unwrap().total = total;
                },
                on_progress: &|bytes| {
                    task_state.lock().unwrap().note_progress(bytes);
                },
                on_part: &|part| {
                    task_state.lock().unwrap().part_path = Some(part.to_path_buf());
                },
                on_captcha: &|path| {
                    let _ = std::process::Command::new("xdg-open")
                        .arg(path)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();
                },
            };

            // Phase 1: interstitial + captcha image.
            let _html = fetch_captcha(&client, &task_machine, &captcha_path).await?;
            {
                let mut s = task_state.lock().unwrap();
                s.phase = Phase::AwaitingCaptcha;
                s.captcha_path = Some(captcha_path.clone());
            }
            // Open the image in the user's viewer via the hook.
            (hooks.on_captcha)(&captcha_path);

            // Phase 2: wait for the typed code (or cancellation).
            let code_str = loop {
                {
                    let guard = task_code.lock().unwrap();
                    if task_cancel.load(Ordering::Relaxed) {
                        anyhow::bail!("Download cancelled.");
                    }
                    if let Some(code) = guard.clone() {
                        break code;
                    }
                }
                tokio::select! {
                    _ = task_notify.notified() => {}
                    _ = tokio::time::sleep(Duration::from_millis(300)) => {}
                }
            };

            // Phase 3: submit the code and stream the .ova.
            {
                let mut s = task_state.lock().unwrap();
                s.phase = Phase::Downloading;
            }
            download_ova(&client, &task_machine, &code_str, &task_md5, &task_dest, &hooks).await
        }
        .await;

        let mut s = task_state.lock().unwrap();
        match outcome {
            Ok(path) => {
                s.phase = Phase::Done;
                s.message = path.display().to_string();
            }
            Err(error) => {
                if task_cancel.load(Ordering::Relaxed) {
                    s.phase = Phase::Cancelled;
                } else {
                    s.phase = Phase::Failed;
                    s.message = format!("{error:#}");
                }
            }
        }
    });

    Ok(DownloadJob {
        machine: machine.clone(),
        dest_dir,
        state,
        cancel,
        code,
        code_notify,
    })
}

/// Human-readable byte size: "0 B", "842.1 KB", "1.9 GB", ...
pub fn fmt_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_bytes_picks_units() {
        assert_eq!(fmt_bytes(0), "0 B");
        assert_eq!(fmt_bytes(512), "512 B");
        assert_eq!(fmt_bytes(1024), "1.0 KB");
        assert_eq!(fmt_bytes(1_966_080), "1.9 MB");
        assert_eq!(fmt_bytes(2_038_433_792), "1.9 GB");
    }
}
