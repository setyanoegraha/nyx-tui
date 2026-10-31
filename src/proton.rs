//! Proton Drive public-link downloads for the machine archives.
//!
//! VulnyX serves each machine behind a `https://drive.proton.me/urls/<token>#<key>`
//! link. The protocol is: `drive/urls/<token>/info` returns an SRP handshake,
//! `POST drive/urls/<token>/auth` (password = the `#` fragment) returns an
//! anonymous session plus the share key, and the file's content blocks decrypt
//! under that key. `proton-drive-rs` implements all of it; this module only
//! picks the archive node, streams it to disk, and reports progress.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use proton_drive_rs::{Node, NodeKind, ProtonDrivePublicLinkClient};
use proton_sdk::config::ProtonClientConfiguration;

/// Product segment Proton validates server-side. `external-drive-nyxtui` is
/// accepted; `external-nyx-tui` is rejected with "Product `nyx` is not valid".
const APP_VERSION: &str = concat!("external-drive-nyxtui@", env!("CARGO_PKG_VERSION"));

/// Live state of one download, shared between the transfer task and the renderer.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    pub machine: String,
    pub file: String,
    pub done: u64,
    pub total: u64,
    pub speed_bps: u64,
    /// Set once the transfer failed; the job is finished and carries the reason.
    pub failed: Option<String>,
    /// Set when the archive was already on disk, so nothing was transferred.
    pub already_present: bool,
}

/// Handle to a running download. Cloning shares the same state.
#[derive(Clone)]
pub struct ProtonDownload {
    state: Arc<Mutex<Progress>>,
    cancel: Arc<AtomicBool>,
}

impl std::fmt::Debug for ProtonDownload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProtonDownload")
            .field("progress", &self.progress())
            .finish()
    }
}

impl ProtonDownload {
    /// Snapshot for rendering.
    pub fn progress(&self) -> Progress {
        self.state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default()
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// True once the transfer ended (success, failure, or nothing to do).
    pub fn is_finished(&self) -> bool {
        let progress = self.progress();
        progress.failed.is_some()
            || progress.already_present
            || (progress.total > 0 && progress.done >= progress.total)
    }
}

/// Opens the link, picks its archive and streams it into `dir`.
///
/// Returns as soon as the transfer is queued: the returned handle is polled by
/// the renderer, and a failure ends up in [`Progress::failed`] instead of
/// unwinding the UI.
pub async fn start(machine: String, link: String, dir: PathBuf) -> Result<ProtonDownload> {
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Cannot create download directory {}", dir.display()))?;

    let config = ProtonClientConfiguration::new(APP_VERSION);
    let client = open_link(&config, &link).await?;
    let (node, size) = pick_archive(&client).await?;

    let file = crate::download::sanitize_filename(&node.name);
    let destination = dir.join(&file);
    let state = Arc::new(Mutex::new(Progress {
        machine,
        file: file.clone(),
        total: size,
        ..Progress::default()
    }));

    if let Ok(metadata) = std::fs::metadata(&destination) {
        if metadata.len() == size && size > 0 {
            if let Ok(mut progress) = state.lock() {
                progress.done = size;
                progress.already_present = true;
            }
            return Ok(ProtonDownload {
                state,
                cancel: Arc::new(AtomicBool::new(false)),
            });
        }
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let job = ProtonDownload {
        state: state.clone(),
        cancel: cancel.clone(),
    };

    // The transfer runs as its own task: writes are buffered to disk, so the
    // await points keep the render thread free to draw progress.
    let task_state = state.clone();
    let task_cancel = cancel.clone();
    tokio::spawn(async move {
        let part = destination.with_extension(part_extension(&destination));
        let outcome = async {
            let file = std::fs::File::create(&part)
                .with_context(|| format!("Cannot create {}", part.display()))?;
            let mut writer = CountingWriter {
                inner: std::io::BufWriter::new(file),
                done: 0,
                state: task_state.clone(),
                cancel: task_cancel.clone(),
                last_bytes: 0,
                last_time: Instant::now(),
            };
            client.download_file_to(&node.uid, &mut writer).await?;
            writer.flush().ok();
            drop(writer);
            std::fs::rename(&part, &destination)
                .with_context(|| format!("Cannot finalize {}", destination.display()))?;
            Ok::<(), anyhow::Error>(())
        }
        .await;

        if let Err(error) = outcome {
            let _ = std::fs::remove_file(&part);
            if let Ok(mut progress) = task_state.lock() {
                progress.failed = Some(if task_cancel.load(Ordering::Relaxed) {
                    "cancelled".to_string()
                } else {
                    format!("{error:#}")
                });
            }
        }
    });

    Ok(job)
}

/// Opens the public link; a custom password turns the crypto error into a
/// sentence instead.
async fn open_link(
    config: &ProtonClientConfiguration,
    link: &str,
) -> Result<ProtonDrivePublicLinkClient> {
    match ProtonDrivePublicLinkClient::open(config.clone(), link, None).await {
        Ok(client) => Ok(client),
        Err(error) => {
            if let Ok(info) = ProtonDrivePublicLinkClient::info(config.clone(), link).await {
                if info.is_custom_password_protected {
                    bail!("Proton Drive link is password protected");
                }
            }
            Err(error).context("Cannot open the Proton Drive link")
        }
    }
}

/// Picks the single archive in the shared tree and returns it with its size.
async fn pick_archive(client: &ProtonDrivePublicLinkClient) -> Result<(Node, u64)> {
    let root = client.get_root_node().await?;
    let mut files: Vec<Node> = Vec::new();
    if matches!(root.kind, NodeKind::File { .. }) {
        files.push(root);
    } else {
        for node in client.enumerate_subtree(&root.uid).await? {
            if matches!(node.kind, NodeKind::File { .. }) {
                files.push(node);
            }
        }
    }

    if files.len() == 1 {
        let node = files.remove(0);
        let size = node_size(&node);
        return Ok((node, size));
    }
    if files.is_empty() {
        bail!("the Proton Drive link contains no file");
    }
    // VulnyX ships `.zip` archives today (the `APex` link resolves to
    // `APex.zip`); older machines may hand out `.ova`/`.7z`/`.tar.gz`.
    if let Some(index) = files.iter().position(|node| is_archive(&node.name)) {
        let node = files.remove(index);
        let size = node_size(&node);
        return Ok((node, size));
    }
    files.sort_by_key(|node| std::cmp::Reverse(node_size(node)));
    if node_size(&files[0]) > node_size(&files[1]) {
        let node = files.remove(0);
        let size = node_size(&node);
        return Ok((node, size));
    }
    bail!("the Proton Drive link has no single archive")
}

fn is_archive(name: &str) -> bool {
    const ARCHIVE_SUFFIXES: [&str; 5] = [".zip", ".ova", ".7z", ".tar.gz", ".tar"];
    let name = name.to_lowercase();
    ARCHIVE_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
}

fn node_size(node: &Node) -> u64 {
    match &node.kind {
        NodeKind::File {
            claimed_size,
            total_size_on_storage,
            ..
        } => claimed_size
            .filter(|size| *size > 0)
            .unwrap_or(*total_size_on_storage)
            .max(0) as u64,
        NodeKind::Folder => 0,
    }
}

/// Keeps the archive extension: `APex.ova` stages as `APex.ova.part`.
fn part_extension(destination: &Path) -> String {
    match destination
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some(extension) => format!("{extension}.part"),
        None => "part".to_string(),
    }
}

/// Writes through to the archive while publishing byte counts, and aborts the
/// transfer when the user asks for it.
struct CountingWriter {
    inner: std::io::BufWriter<std::fs::File>,
    done: u64,
    state: Arc<Mutex<Progress>>,
    cancel: Arc<AtomicBool>,
    last_bytes: u64,
    last_time: Instant,
}

impl Write for CountingWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(std::io::Error::other("cancelled"));
        }
        let written = self.inner.write(buffer)?;
        self.done += written as u64;
        if let Ok(mut progress) = self.state.lock() {
            progress.done = self.done;
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_time).as_secs_f64();
            // Sampling at 4 Hz keeps the speed readable without a lock per write.
            if elapsed >= 0.25 {
                progress.speed_bps = ((self.done - self.last_bytes) as f64 / elapsed) as u64;
                self.last_bytes = self.done;
                self.last_time = now;
            }
        }
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_size_prefers_the_claimed_size() {
        let node = Node {
            uid: proton_sdk::ids::NodeUid::new(
                proton_sdk::ids::VolumeId::new("v"),
                proton_sdk::ids::LinkId::new("l"),
            ),
            parent_uid: None,
            kind: NodeKind::File {
                media_type: String::new(),
                total_size_on_storage: 4096,
                active_revision_state: None,
                active_revision_id: None,
                claimed_size: Some(1024),
                claimed_modification_time: None,
                content_sha1: None,
            },
            name: "APex.ova".into(),
            creation_time: 0,
            modification_time: 0,
            trashed: false,
            is_shared: false,
            is_shared_publicly: false,
            signature_email: None,
            membership: None,
            direct_role: None,
            share_id: None,
            photo: None,
            album: None,
            verification: Default::default(),
        };
        assert_eq!(node_size(&node), 1024);
    }

    #[test]
    fn archive_detection_covers_the_suffixes_vulnyx_serves() {
        assert!(is_archive("APex.zip"));
        assert!(is_archive("APex.OVA"));
        assert!(is_archive("disk.tar.gz"));
        assert!(!is_archive("readme.txt"));
    }

    #[test]
    fn part_extension_keeps_the_archive_suffix() {
        assert_eq!(part_extension(Path::new("/tmp/APex.ova")), "ova.part");
        assert_eq!(part_extension(Path::new("/tmp/APex")), "part");
    }

    /// Live check against a real VulnyX link:
    /// NYX_PROTON_LINK='https://drive.proton.me/urls/<token>#<key>' \
    ///   cargo test --offline -- --ignored --nocapture proton::tests::real_link
    #[tokio::test]
    #[ignore]
    async fn real_link() {
        let Ok(link) = std::env::var("NYX_PROTON_LINK") else {
            return;
        };
        let config = ProtonClientConfiguration::new(APP_VERSION);
        let client = open_link(&config, &link).await.expect("open link");
        let (node, size) = pick_archive(&client).await.expect("pick archive");
        println!("archive: {} ({} bytes)", node.name, size);
        assert!(!node.name.is_empty());
    }
}
