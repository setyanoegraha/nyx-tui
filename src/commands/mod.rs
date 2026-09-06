//! Dashboard orchestration: data fetching and the host callbacks handed to
//! the TUI event loop. No authentication — VulnyX works with a
//! self-declared username.

use anyhow::Result;

use crate::config::ConfigManager;
use crate::modules::flags::{FlagManager, FlagType};
use crate::modules::machines::MachineFetcher;
use crate::modules::session::NyxClient;
use crate::modules::writeups::WriteupManager;
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
        client,
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
    Ok(TuiData {
        machines,
        writeups,
    })
}

/// Executes a user action from a popup; returns the result popup content.
async fn run_tui_action(client: &NyxClient, action: TuiAction) -> Result<ActionReport> {
    match action.kind {
        ActionKind::SubmitFlag => {
            let username = ConfigManager::new().username();
            let mut reports = Vec::new();
            for (_, value) in &action.values {
                let Some((flag_type, flag_value)) = value.split_once(':') else {
                    continue;
                };
                let flag_type = match flag_type {
                    "user" => FlagType::User,
                    _ => FlagType::Root,
                };
                let message =
                    FlagManager::new(client.clone())
                        .submit(&action.machine, flag_type, &username, flag_value)
                        .await?;
                reports.push((
                    flag_type.as_str().to_string(),
                    format!(
                        "{} flag: ✓ ACCEPTED — {}",
                        flag_type.as_str(),
                        message
                    ),
                ));
            }
            if reports.is_empty() {
                anyhow::bail!("No flag was submitted.");
            }
            let entries = reports
                .iter()
                .map(|(_, text)| (ReportKind::Success, text.clone()))
                .collect();
            let status = format!(
                "[✓] {} flag(s) submitted for {} — check First Bloods in Progress.",
                reports.len(),
                action.machine
            );
            Ok(ActionReport {
                title: format!(" Flags — {} ", action.machine),
                entries,
                changed: true,
                status,
            })
        }
        ActionKind::SubmitWriteup => {
            let username = ConfigManager::new().username();
            let url = action.values[0].1.clone();
            let tipo = action.values[1].1.clone();
            let language = action.values[2].1.clone();
            let creator = action.values[3].1.clone();
            let message = WriteupManager::new(client.clone())
                .submit(
                    &action.machine,
                    &creator,
                    &username,
                    &url,
                    &tipo,
                    &language,
                )
                .await?;
            Ok(ActionReport {
                title: format!(" Writeup — {} ", action.machine),
                entries: vec![(
                    ReportKind::Success,
                    format!("Writeup: ✓ SUBMITTED — {message}"),
                )],
                changed: false,
                status: format!(
                    "[✓] Writeup submitted for {} — pending admin review (48h).",
                    action.machine
                ),
            })
        }
    }
}
