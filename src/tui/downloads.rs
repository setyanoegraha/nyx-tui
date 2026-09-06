//! Background download jobs for the TUI. Each job runs on the tokio
//! runtime. The CAPTCHA adds a phase: the job fetches the image, opens it
//! in an external viewer and waits for the user's code — then resolves the
//! Proton Drive URL and opens it in the browser.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

use anyhow::{Context, Result};

use crate::download::fetch_captcha;
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

/// Live state, shared between the download task and the renderer.
#[derive(Debug)]
pub struct DownloadState {
    pub phase: Phase,
    pub message: String,
    pub captcha_path: Option<PathBuf>,
    /// ASCII art rendering of the captcha image for in-terminal display.
    pub ascii_lines: Vec<String>,
}

impl Default for DownloadState {
    fn default() -> Self {
        Self {
            phase: Phase::Resolving,
            message: String::new(),
            captcha_path: None,
            ascii_lines: Vec::new(),
        }
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

    /// Requests cancellation; the task cleans up its temp files.
    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.code_notify.notify_one();
    }

    /// Best-effort cleanup used when quitting with active jobs.
    pub fn remove_part(&self) {
        if let Some(path) = self.state.lock().unwrap().captcha_path.clone() {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Spawns the download task. The CAPTCHA image is fetched, opened in an
/// external viewer, and the code must be supplied via
/// [`DownloadJob::submit_code`]. After the code is submitted, the task
/// resolves the Proton Drive URL and opens it in the user's browser.
pub fn start_download(
    client: NyxClient,
    machine: String,
    machine_slug: String,
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
    let task_machine = machine.clone();
    let task_slug = machine_slug.clone();
    // The task runs detached: cancellation goes through the shared flag.
    tokio::spawn(async move {
        let outcome = async {
            let captcha_path = std::env::temp_dir().join(format!(
                "nyx-captcha-{}-{}.png",
                std::process::id(),
                task_slug
            ));
            // Phase 1: interstitial + captcha image (opened in viewer).
            let _html = fetch_captcha(&client, &task_machine, &captcha_path).await?;

            let ascii_lines = {
                let png = std::fs::read(&captcha_path)
                    .with_context(|| format!("Failed to read {}", captcha_path.display()))?;
                crate::captcha::render_ascii(&png).unwrap_or_default()
            };
            {
                let mut s = task_state.lock().unwrap();
                s.phase = Phase::AwaitingCaptcha;
                s.captcha_path = Some(captcha_path.clone());
                s.ascii_lines = ascii_lines;
            }
            let _ = std::process::Command::new("xdg-open")
                .arg(&captcha_path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();

            // Phase 2: wait for the typed code (or cancellation).
            let code = loop {
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

            // Phase 3: submit the code → resolve the Proton Drive URL.
            {
                let mut s = task_state.lock().unwrap();
                s.phase = Phase::Downloading;
            }
            crate::download::resolve_captcha(&client, &task_machine, &code).await
        }
        .await;

        let mut s = task_state.lock().unwrap();
        match outcome {
            Ok(drive_url) => {
                s.phase = Phase::Done;
                s.message = format!("Download link opened in browser: {drive_url}");
                let _ = std::process::Command::new("xdg-open")
                    .arg(&drive_url)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
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
        machine,
        dest_dir,
        state,
        cancel,
        code,
        code_notify,
    })
}
