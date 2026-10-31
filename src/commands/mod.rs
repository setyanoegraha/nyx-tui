//! Dashboard orchestration: data fetching and the host callbacks handed to
//! the TUI event loop. No authentication — VulnyX works with a
//! self-declared username.

use anyhow::Result;

use crate::config::ConfigManager;
use crate::modules::flags::{FlagManager, FlagType};
use crate::modules::machines::MachineFetcher;
use crate::modules::session::NyxClient;
use crate::modules::writeups::WriteupManager;
use crate::i18n::Msg;
use crate::tui::{ActionKind, ActionReport, ReportKind, TuiAction, TuiData};

pub async fn tui_cmd() -> Result<()> {
    let client = NyxClient::new()?;
    let initial = crate::tui::AppState::loading();

    let refetch_client = client.clone();
    let refetch = move || {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(fetch_tui_data(&refetch_client))
        })
    };
    let action_client = client.clone();
    let run_action = move |action: TuiAction| {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(run_tui_action(&action_client, action))
        })
    };

    let mut host = crate::tui::Host {
        refetch: &refetch,
        run_action: &run_action,
        pending_fetch: false,
    };

    crate::tui::run(initial, &mut host)
}

/// Fetches everything the dashboard shows: the machine catalog and the full
/// writeups map. Both are public JSON files.
async fn fetch_tui_data(client: &NyxClient) -> Result<TuiData> {
    let machines = MachineFetcher::new(client.clone()).fetch().await?;
    let writeups = WriteupManager::new(client.clone()).fetch_all().await?;
    Ok(TuiData { machines, writeups })
}

/// Executes a user action from a popup; returns the result popup content.
async fn run_tui_action(client: &NyxClient, action: TuiAction) -> Result<ActionReport> {
    match action.kind {
        ActionKind::SubmitFlag => {
            let username = ConfigManager::new().username();
            let mut entries = Vec::new();
            let mut accepted = 0usize;
            let mut failed = 0usize;
            for (_, value) in &action.values {
                let Some((flag_type, flag_value)) = value.split_once(':') else {
                    continue;
                };
                let flag_type = match flag_type {
                    "user" => FlagType::User,
                    _ => FlagType::Root,
                };
                // One rejected flag must not discard an accepted one, so each
                // submission reports itself instead of aborting the loop.
                match FlagManager::new(client.clone())
                    .submit(&action.machine, flag_type, &username, flag_value)
                    .await
                {
                    Ok(message) => {
                        accepted += 1;
                        entries.push((
                            ReportKind::Success,
                            Msg::FlagAccepted(flag_type.as_str(), message).render(action.lang),
                        ));
                    }
                    Err(error) => {
                        failed += 1;
                        entries.push((
                            ReportKind::Failure,
                            format!("{}: {error:#}", flag_type.as_str()),
                        ));
                    }
                }
            }
            if entries.is_empty() {
                anyhow::bail!("{}", Msg::NoFlagSubmitted.render(action.lang));
            }
            let status = if accepted > 0 {
                Msg::FlagsSubmitted(accepted, action.machine.clone()).render(action.lang)
            } else {
                Msg::ActionFailed(format!("{failed} flag(s) rejected")).render(action.lang)
            };
            Ok(ActionReport {
                title: Msg::FlagsReportTitle(action.machine.clone()).render(action.lang),
                entries,
                changed: accepted > 0,
                status,
                download: None,
            })
        }
        ActionKind::SubmitWriteup => {
            let username = ConfigManager::new().username();
            let url = action.values[0].1.clone();
            let tipo = action.values[1].1.clone();
            let language = action.values[2].1.clone();
            let message = WriteupManager::new(client.clone())
                .submit(&action.machine, &username, &url, &tipo, &language)
                .await?;
            Ok(ActionReport {
                title: Msg::WriteupReportTitle(action.machine.clone()).render(action.lang),
                entries: vec![(
                    ReportKind::Success,
                    Msg::WriteupSubmitted(message).render(action.lang),
                )],
                changed: false,
                status: Msg::WriteupPending(action.machine.clone()).render(action.lang),
                download: None,
            })
        }
        ActionKind::Download => {
            let url = action.values.first().map(|(_, v)| v.clone()).unwrap_or_default();
            let name = action
                .values
                .get(1)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| action.machine.clone());
            let outcome = crate::download::download_machine(client, &name, &url).await?;
            let mut download = None;
            let (mut entries, status) = match outcome {
                crate::download::Download::Link(link) => {
                    let mut entries = vec![(
                        ReportKind::Success,
                        Msg::DownloadLinkReady(link.clone()).render(action.lang),
                    )];
                    // The link is resolved; hand the archive transfer to the
                    // event loop and let the footer report its progress.
                    match crate::proton::start(
                        name.clone(),
                        link.clone(),
                        ConfigManager::new().download_dir(),
                    )
                    .await
                    {
                        Ok(job) => {
                            let progress = job.progress();
                            // Nothing is transferred when the archive is
                            // already complete, so do not claim a download.
                            let message = if progress.already_present {
                                Msg::DownloadAlreadyPresent(progress.file).render(action.lang)
                            } else {
                                Msg::DownloadStarted(progress.file).render(action.lang)
                            };
                            entries.push((ReportKind::Info, message));
                            download = Some(job);
                            (entries, Msg::DownloadLinkReady(link).render(action.lang))
                        }
                        Err(error) => {
                            // Keep the browser route usable when the transfer
                            // cannot even be prepared.
                            let _ = crate::open::open_url(&link);
                            entries.push((
                                ReportKind::Failure,
                                Msg::DownloadFailed(format!("{error:#}")).render(action.lang),
                            ));
                            let status = Msg::DownloadFailed(format!("{error:#}")).render(action.lang);
                            (entries, status)
                        }
                    }
                }
                crate::download::Download::BrowserFallback { url, reason } => {
                    let opened = crate::open::open_url(&url);
                    let mut entries = vec![(
                        ReportKind::Failure,
                        Msg::DownloadFallback(reason.clone()).render(action.lang),
                    )];
                    if let Err(error) = opened {
                        entries.push((
                            ReportKind::Info,
                            Msg::BrowserOpenFailed(error.to_string()).render(action.lang),
                        ));
                    }
                    let status = Msg::DownloadFallback(reason).render(action.lang);
                    (entries, status)
                }
            };
            // The link line is noise once the transfer owns the download.
            if download.is_some() {
                entries.retain(|(kind, _)| *kind != ReportKind::Success);
            }
            Ok(ActionReport {
                title: format!(" {name} "),
                entries,
                changed: false,
                status,
                download,
            })
        }
    }
}
