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
            let mut reports = Vec::new();
            for (_, value) in &action.values {
                let Some((flag_type, flag_value)) = value.split_once(':') else {
                    continue;
                };
                let flag_type = match flag_type {
                    "user" => FlagType::User,
                    _ => FlagType::Root,
                };
                let message = FlagManager::new(client.clone())
                    .submit(&action.machine, flag_type, &username, flag_value)
                    .await?;
                reports.push((
                    flag_type.as_str().to_string(),
                    Msg::FlagAccepted(flag_type.as_str(), message).render(action.lang),
                ));
            }
            if reports.is_empty() {
                anyhow::bail!("{}", Msg::NoFlagSubmitted.render(action.lang));
            }
            let entries = reports
                .iter()
                .map(|(_, text)| (ReportKind::Success, text.clone()))
                .collect();
            let status = Msg::FlagsSubmitted(reports.len(), action.machine.clone()).render(action.lang);
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
            let message = WriteupManager::new(client.clone())
                .submit(&action.machine, &username, &url, &tipo, &language)
                .await?;
            Ok(ActionReport {
                title: format!(" Writeup — {} ", action.machine),
                entries: vec![(
                    ReportKind::Success,
                    Msg::WriteupSubmitted(message).render(action.lang),
                )],
                changed: false,
                status: Msg::WriteupPending(action.machine.clone()).render(action.lang),
            })
        }
    }
}
