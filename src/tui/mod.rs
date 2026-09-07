//! Interactive dashboard: application state, input handling and the event
//! loop. Rendering lives in `render.rs`; all state transitions here are pure
//! and unit-tested. UI text is in English.

pub mod render;

use std::collections::HashSet;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::i18n::{Lang, Msg};
use crate::modules::leaderboard::{compute, position_of};
use crate::modules::machines::Machine;
use crate::modules::writeups::WriteupEntry;

/// What a popup asks the user for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupKind {
    /// First-blood flag input (User / Root MD5 fields as slots allow).
    Flag,
    /// Writeup submission (URL, type, language).
    WriteupSubmit,
    /// Username setting (`a`) — the self-declared submission identity.
    Username,
    /// Read-only machine description (`i` / Enter on Machines).
    Descripcion,
}

/// Language options of vulnyx.com (`WRITEUP_LANGS` in the site's /js/index.js,
/// snapshot 2026-09-07). `(code, display label)`, site order.
pub const WRITEUP_LANGS: &[(&str, &str)] = &[
    ("en", "English"),
    ("es", "Español - Spanish"),
    ("zh", "中文 - Chinese"),
    ("uz", "Oʻzbekcha - Uzbek"),
    ("ru", "Русский - Russian"),
    ("fr", "Français - French"),
    ("de", "Deutsch - German"),
    ("it", "Italiano - Italian"),
    ("pt", "Português - Portuguese"),
    ("ar", "العربية - Arabic"),
    ("hi", "हिन्दी - Hindi"),
    ("ja", "日本語 - Japanese"),
    ("ko", "한국어 - Korean"),
    ("tr", "Türkçe - Turkish"),
    ("fa", "فارسی - Persian"),
    ("id", "Bahasa Indonesia - Indonesian"),
    ("vi", "Tiếng Việt - Vietnamese"),
    ("pl", "Polski - Polish"),
    ("uk", "Українська - Ukrainian"),
    ("nl", "Nederlands - Dutch"),
    ("ro", "Română - Romanian"),
    ("el", "Ελληνικά - Greek"),
    ("he", "עברית - Hebrew"),
    ("th", "ไทย - Thai"),
    ("cs", "Čeština - Czech"),
    ("sv", "Svenska - Swedish"),
    ("hu", "Magyar - Hungarian"),
    ("bn", "বাংলা - Bengali"),
    ("so", "Soomaali - Somali"),
    ("ms", "Bahasa Melayu - Malay"),
    ("da", "Dansk - Danish"),
    ("fi", "Suomi - Finnish"),
    ("no", "Norsk - Norwegian"),
    ("bg", "Български - Bulgarian"),
    ("hr", "Hrvatski - Croatian"),
    ("sr", "Српски - Serbian"),
    ("sk", "Slovenčina - Slovak"),
    ("sl", "Slovenščina - Slovenian"),
    ("lt", "Lietuvių - Lithuanian"),
    ("lv", "Latviešu - Latvian"),
    ("et", "Eesti - Estonian"),
    ("ka", "ქართული - Georgian"),
    ("hy", "Հայերեն - Armenian"),
    ("az", "Azərbaycan - Azerbaijani"),
    ("kk", "Қазақ - Kazakh"),
    ("ky", "Кыргызча - Kyrgyz"),
    ("tg", "Тоҷикӣ - Tajik"),
    ("tk", "Türkmen - Turkmen"),
    ("mn", "Монгол - Mongolian"),
    ("ne", "नेपाली - Nepali"),
    ("si", "සිංහල - Sinhala"),
    ("ta", "தமிழ் - Tamil"),
    ("te", "తెలుగు - Telugu"),
    ("ml", "മലയാളം - Malayalam"),
    ("kn", "ಕನ್ನಡ - Kannada"),
    ("mr", "मराठी - Marathi"),
    ("gu", "ગુજરાતી - Gujarati"),
    ("pa", "ਪੰਜਾਬੀ - Punjabi"),
    ("ur", "اردو - Urdu"),
    ("sw", "Kiswahili - Swahili"),
    ("other", "Other"),
    ("none", "None (video only)"),
];

/// Canonical-order, comma-joined language codes, e.g. ["es","en"] -> "en,es".
fn join_langs(selected: &[&'static str]) -> String {
    WRITEUP_LANGS
        .iter()
        .filter(|(code, _)| selected.contains(code))
        .map(|(code, _)| *code)
        .collect::<Vec<_>>()
        .join(",")
}

/// Writeup submission types (single-select), mirroring the site's form.
pub const WRITEUP_TYPES: &[&str] = &["Text", "Video"];

/// A popup bound to one machine. Input popups carry `buffers`; the
/// description popup is read-only and renders `text`.
#[derive(Debug, Clone)]
pub struct Popup {
    pub kind: PopupKind,
    pub machine: String,
    /// Machine slug for API submissions ("" when not applicable).
    pub machine_slug: String,
    pub buffers: Vec<String>,
    pub field: usize,
    pub notice: Option<String>,
    pub readonly: bool,
    /// Body text for read-only popups (descripción).
    pub text: Option<String>,
    /// Flag slot types parallel to `buffers` ("user" / "root").
    pub flag_types: Vec<&'static str>,
    /// WriteupSubmit only: language picker state.
    pub lang_open: bool,
    pub lang_cursor: usize,
    pub lang_selected: Vec<&'static str>,
    /// WriteupSubmit only: type picker state (single-select).
    pub type_open: bool,
    pub type_cursor: usize,
}

impl Popup {
    pub fn push(&mut self, c: char) {
        if self.lang_field_active() || self.type_field_active() {
            return;
        }
        if let Some(buffer) = self.buffers.get_mut(self.field) {
            buffer.push(c);
        }
    }

    pub fn pop(&mut self) {
        if self.lang_field_active() || self.type_field_active() {
            return;
        }
        if let Some(buffer) = self.buffers.get_mut(self.field) {
            buffer.pop();
        }
    }

    pub fn next_field(&mut self) {
        if self.buffers.len() > 1 {
            self.field = (self.field + 1) % self.buffers.len();
        }
    }

    pub fn previous_field(&mut self) {
        if self.buffers.len() > 1 {
            self.field = (self.field + self.buffers.len() - 1) % self.buffers.len();
        }
    }

    /// True when the active field is the writeup language picker (field 2 of
    /// WriteupSubmit).
    pub fn lang_field_active(&self) -> bool {
        self.kind == PopupKind::WriteupSubmit && self.field == 2
    }

    pub fn open_lang_panel(&mut self) {
        self.lang_open = true;
    }

    pub fn close_lang_panel(&mut self) {
        self.lang_open = false;
    }

    pub fn move_lang_cursor(&mut self, delta: isize) {
        let last = WRITEUP_LANGS.len().saturating_sub(1);
        self.lang_cursor = (self.lang_cursor as isize + delta).clamp(0, last as isize) as usize;
    }

    /// Toggles the option under `lang_cursor`, then reserializes `buffers[2]`
    /// in canonical order via `join_langs`.
    pub fn toggle_lang(&mut self) {
        let Some((code, _)) = WRITEUP_LANGS.get(self.lang_cursor) else {
            return;
        };
        let code = *code;
        if let Some(pos) = self.lang_selected.iter().position(|c| *c == code) {
            self.lang_selected.remove(pos);
        } else {
            self.lang_selected.push(code);
        }
        if let Some(buffer) = self.buffers.get_mut(2) {
            *buffer = join_langs(&self.lang_selected);
        }
    }

    /// True when the active field is the writeup type picker (field 1 of
    /// WriteupSubmit).
    pub fn type_field_active(&self) -> bool {
        self.kind == PopupKind::WriteupSubmit && self.field == 1
    }

    pub fn open_type_panel(&mut self) {
        self.type_open = true;
    }

    pub fn close_type_panel(&mut self) {
        self.type_open = false;
    }

    pub fn move_type_cursor(&mut self, delta: isize) {
        let last = WRITEUP_TYPES.len().saturating_sub(1);
        self.type_cursor = (self.type_cursor as isize + delta).clamp(0, last as isize) as usize;
    }

    /// Selects the highlighted type (single-select) into `buffers[1]`.
    pub fn select_type(&mut self) {
        let Some(label) = WRITEUP_TYPES.get(self.type_cursor) else {
            return;
        };
        if let Some(buffer) = self.buffers.get_mut(1) {
            *buffer = (*label).to_string();
        }
    }
}

/// A user action queued from a popup, executed by the host application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    SubmitFlag,
    SubmitWriteup,
}

#[derive(Debug, Clone)]
pub struct TuiAction {
    pub kind: ActionKind,
    pub machine: String,
    pub values: Vec<(usize, String)>,
    /// Interface language at queue time, for host-built reports.
    pub lang: Lang,
}

#[derive(Debug, Clone, Default)]
pub struct TuiData {
    pub machines: Vec<Machine>,
    /// Flattened writeups map: (machine slug, entry).
    pub writeups: Vec<(String, WriteupEntry)>,
}

impl TuiData {
    /// Machines the given username holds first blood on (user or root).
    pub fn first_bloods_of(&self, username: &str) -> Vec<(&Machine, &'static str)> {
        let name = username.trim().to_lowercase();
        let mut out = Vec::new();
        for machine in &self.machines {
            if machine.first_user.trim().to_lowercase() == name {
                out.push((machine, "user"));
            }
            if machine.first_root.trim().to_lowercase() == name {
                out.push((machine, "root"));
            }
        }
        out
    }

    /// Writeups published by the given username.
    pub fn own_writeups(&self, username: &str) -> Vec<&(String, WriteupEntry)> {
        let name = username.trim().to_lowercase();
        self.writeups
            .iter()
            .filter(|(_, w)| w.author.trim().to_lowercase() == name)
            .collect()
    }

    /// The user's overall leaderboard position (1-based), computed from the
    /// public data with the site's own scoring rules.
    pub fn leaderboard_position(&self, username: &str) -> Option<(usize, u64)> {
        let list = compute(&self.machines, &self.writeups);
        let position = position_of(&list, username)?;
        Some((position, list[position - 1].points))
    }
}

/// One line of an action result popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Success,
    Failure,
    Info,
}

/// Shown after an action; persists until dismissed.
#[derive(Debug, Clone)]
pub struct ActionReport {
    pub title: String,
    pub entries: Vec<(ReportKind, String)>,
    pub changed: bool,
    pub status: String,
}

/// Community writeups of one machine (`w`), rendered as a table popup.
#[derive(Debug, Clone)]
pub struct WriteupsPopup {
    pub machine: String,
    pub entries: Vec<WriteupEntry>,
    pub selected: usize,
}

impl WriteupsPopup {
    pub fn move_selection(&mut self, delta: isize) {
        let last = self.entries.len().saturating_sub(1);
        self.selected = self.selected.saturating_add_signed(delta).min(last);
    }

    pub fn selected_url(&self) -> Option<&str> {
        self.entries.get(self.selected).map(|w| w.url.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Machines,
    Progress,
}

impl Tab {
    pub const ALL: [Tab; 2] = [Tab::Machines, Tab::Progress];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Machines => "Machines",
            Tab::Progress => "Progress",
        }
    }

    fn next(self) -> Self {
        let index = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    fn previous(self) -> Self {
        let index = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        let last = Self::ALL.len() - 1;
        Self::ALL[(index + last) % Self::ALL.len()]
    }
}

/// Sort order of the Machines tab (`s` cycles: site order -> name -> date ->
/// difficulty).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MachineSort {
    #[default]
    Sitio,
    Nombre,
    Fecha,
    Dificultad,
}

impl MachineSort {
    fn next(self) -> Self {
        match self {
            MachineSort::Sitio => MachineSort::Nombre,
            MachineSort::Nombre => MachineSort::Fecha,
            MachineSort::Fecha => MachineSort::Dificultad,
            MachineSort::Dificultad => MachineSort::Sitio,
        }
    }

    pub fn indicator(self, lang: Lang) -> String {
        match self {
            MachineSort::Sitio => Msg::SortSite.render(lang),
            MachineSort::Nombre => Msg::SortName.render(lang),
            MachineSort::Fecha => Msg::SortDate.render(lang),
            MachineSort::Dificultad => Msg::SortDifficulty.render(lang),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filter,
}

pub struct AppState {
    pub tab: Tab,
    pub input_mode: InputMode,
    pub filter: String,
    pub selected: usize,
    pub scroll: usize,
    pub machine_sort: MachineSort,
    pub only_first_blood: bool,
    pub lang: Lang,
    /// Manual completed marks (slugs), persisted in config.json.
    pub completed: Vec<String>,
    /// Hide completed machines from the Machines list (`x` toggle).
    pub hide_completed: bool,
    pub quit: bool,
    pub refresh_requested: bool,
    pub fetching: Option<String>,
    pub status: Option<String>,
    pub status_expiry: Option<std::time::Instant>,
    pub popup: Option<Popup>,
    pub pending_action: Option<TuiAction>,
    pub report: Option<ActionReport>,
    pub writeups_popup: Option<WriteupsPopup>,
    pub pending_refresh_after_close: bool,
    pub data: TuiData,
    pub last_visible_rows: Option<usize>,
}

/// How long a status message stays visible in the footer.
const STATUS_LIFETIME: Duration = Duration::from_secs(5);

impl AppState {
    pub fn new(data: TuiData) -> Self {
        Self {
            tab: Tab::Machines,
            input_mode: InputMode::Normal,
            filter: String::new(),
            selected: 0,
            scroll: 0,
            machine_sort: MachineSort::default(),
            only_first_blood: false,
            lang: crate::config::ConfigManager::new().language(),
            completed: crate::config::ConfigManager::new().completed_machines(),
            hide_completed: false,
            quit: false,
            refresh_requested: false,
            fetching: None,
            status: None,
            status_expiry: None,
            popup: None,
            pending_action: None,
            report: None,
            writeups_popup: None,
            pending_refresh_after_close: false,
            data,
            last_visible_rows: None,
        }
    }

    /// Entry state for `nyx`: draws immediately, then loads all data.
    pub fn loading() -> Self {
        let mut state = Self::new(TuiData::default());
        state.fetching = Some(Msg::FetchingLoad.render(state.lang));
        state
    }

    pub fn request_quit(&mut self) {
        self.quit = true;
    }

    /// Shows a status message in the footer, auto-expiring after 5 seconds.
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status = Some(message.into());
        self.status_expiry = Some(std::time::Instant::now() + STATUS_LIFETIME);
    }

    /// Renders a message in the interface language.
    pub fn tr(&self, msg: Msg) -> String {
        msg.render(self.lang)
    }

    /// Clears expired status messages; called once per event-loop iteration.
    pub fn tick(&mut self) {
        if let Some(expiry) = self.status_expiry {
            if std::time::Instant::now() >= expiry {
                self.status = None;
                self.status_expiry = None;
            }
        }
    }

    pub fn set_data(&mut self, data: TuiData) {
        self.data = data;
        self.selected = 0;
        self.scroll = 0;
    }

    pub fn next_tab(&mut self) {
        self.tab = self.tab.next();
        self.reset_list_position();
    }

    pub fn previous_tab(&mut self) {
        self.tab = self.tab.previous();
        self.reset_list_position();
    }

    fn reset_list_position(&mut self) {
        self.selected = 0;
        self.scroll = 0;
    }

    /// Slugs with an own writeup already published on the site (approved).
    /// Cheap: derives from data already in memory.
    pub fn completed_auto(&self) -> HashSet<String> {
        self.data
            .own_writeups(&crate::config::ConfigManager::new().username())
            .into_iter()
            .map(|(slug, _)| slug.clone())
            .collect()
    }

    /// True when the machine is completed: manual mark or own published
    /// writeup. `auto` precomputes `completed_auto()` once for the whole
    /// list pass.
    pub fn is_completed_with(&self, slug: &str, auto: &HashSet<String>) -> bool {
        self.completed.iter().any(|s| s == slug) || auto.contains(slug)
    }

    /// Toggles the manual completed mark of the selected machine.
    pub fn toggle_completed(&mut self) {
        let Some(machine) = self.selected_machine() else {
            self.set_status(self.tr(Msg::NothingSelected));
            return;
        };
        let slug = machine.slug.clone();
        let name = machine.name.clone();
        let removing = self.completed.iter().any(|s| *s == slug);
        if removing {
            self.completed.retain(|s| *s != slug);
        } else {
            self.completed.push(slug.clone());
        }
        if let Err(error) =
            crate::config::ConfigManager::new().save_completed_machines(self.completed.clone())
        {
            if removing {
                self.completed.push(slug);
            } else {
                self.completed.pop();
            }
            self.set_status(format!("[!] {error:#}"));
            return;
        }
        self.set_status(if removing {
            self.tr(Msg::MarkedNotCompleted(name))
        } else {
            self.tr(Msg::MarkedCompleted(name))
        });
    }
    /// Filtered catalog for the Machines tab; filter matches name,
    /// difficulty, OS, creator or tech tag; `machine_sort` orders the
    /// result. `only_first_blood` keeps machines with an open slot.
    pub fn visible_machines(&self) -> Vec<&Machine> {
        let needle = self.filter.to_lowercase();
        let auto = self.completed_auto();
        let mut machines: Vec<&Machine> = self
            .data
            .machines
            .iter()
            .filter(|m| {
                (needle.is_empty()
                    || m.name.to_lowercase().contains(&needle)
                    || m.difficulty.to_lowercase().contains(&needle)
                    || m.os.to_lowercase().contains(&needle)
                    || m.creator.to_lowercase().contains(&needle)
                    || m.tech_tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&needle)))
                    && (!self.only_first_blood || m.any_slot_open())
                    && (!self.hide_completed || !self.is_completed_with(&m.slug, &auto))
            })
            .collect();
        match self.machine_sort {
            MachineSort::Sitio => {}
            MachineSort::Nombre => machines.sort_by_key(|m| m.name.to_lowercase()),
            MachineSort::Fecha => machines.sort_by_key(|m| {
                std::cmp::Reverse(crate::modules::machines::release_sort_key(&m.release_date))
            }),
            MachineSort::Dificultad => {
                machines.sort_by_key(|m| crate::modules::machines::difficulty_rank(&m.difficulty))
            }
        }
        machines
    }

    /// Opens the writeup URL of the selected row (Progress / Writeups).
    pub fn open_selected_writeup_link(&mut self) {
        let url = self
            .own_writeups_rows()
            .get(self.selected)
            .map(|(_, w)| w.url.clone());
        match url {
            Some(url) => {
                let opened = std::process::Command::new("xdg-open")
                    .arg(&url)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
                self.set_status(match opened {
                    Ok(_) => self.tr(Msg::OpenedInBrowser(url)),
                    Err(error) => self.tr(Msg::XdgOpenFailed(error.to_string())),
                });
            }
            None => self.set_status(self.tr(Msg::NoWriteupLink)),
        }
    }

    /// Own writeups (Progress right panel), aligned with the rendered rows.
    pub fn own_writeups_rows(&self) -> Vec<&(String, crate::modules::writeups::WriteupEntry)> {
        self.data
            .own_writeups(&crate::config::ConfigManager::new().username())
    }

    /// Dismisses the result popup; returns true if a refresh was queued.
    pub fn close_report(&mut self) -> bool {
        self.report = None;
        std::mem::take(&mut self.pending_refresh_after_close)
    }

    /// The full machine under the selection (Machines tab only).
    pub fn selected_machine(&self) -> Option<&Machine> {
        if self.tab != Tab::Machines {
            return None;
        }
        self.visible_machines().get(self.selected).copied()
    }

    /// Opens the username popup (`a`) prefilled with the stored name.
    pub fn open_username_popup(&mut self) {
        if self.popup.is_some() || self.report.is_some() {
            return;
        }
        let username = crate::config::ConfigManager::new().username();
        self.popup = Some(Popup {
            kind: PopupKind::Username,
            machine: String::new(),
            machine_slug: String::new(),
            buffers: vec![username],
            field: 0,
            notice: Some(
                "Your username is attached to flags, writeups and the leaderboard.".to_string(),
            ),
            readonly: false,
            text: None,
            flag_types: Vec::new(),
            lang_open: false, lang_cursor: 0, lang_selected: Vec::new(),
            type_open: false, type_cursor: 0,
        });
    }

    /// Read-only description popup for the selected machine (Machines).
    pub fn open_descripcion_popup(&mut self) {
        if self.popup.is_some() || self.report.is_some() || self.writeups_popup.is_some() {
            return;
        }
        let Some(machine) = self.selected_machine() else {
            self.set_status(self.tr(Msg::NothingSelected));
            return;
        };
        let user_line = if machine.user_slot_open() {
            self.tr(Msg::SlotOpen)
        } else {
            self.tr(Msg::TakenBy(machine.first_user.clone()))
        };
        let root_line = if machine.root_slot_open() {
            self.tr(Msg::SlotOpen)
        } else {
            self.tr(Msg::TakenBy(machine.first_root.clone()))
        };
        let text = [
            self.tr(Msg::DescDifficulty(
                machine.difficulty.clone(),
                machine.os.clone(),
                machine.pts_user,
                machine.pts_root,
            )),
            self.tr(Msg::DescPlatform(
                machine.platforms.join(", "),
                machine.size.clone(),
            )),
            self.tr(Msg::DescCreator(machine.creator.clone(), machine.release_date.clone())),
            self.tr(Msg::DescMd5)
                + " "
                + if machine.md5.is_empty() { "-" } else { &machine.md5 },
            self.tr(Msg::DescTags(
                if machine.tech_tags.is_empty() {
                    "-".to_string()
                } else {
                    machine.tech_tags.join(", ")
                },
            )),
            String::new(),
            self.tr(Msg::DescBloodUser(user_line)),
            self.tr(Msg::DescBloodRoot(root_line)),
        ]
        .join("\n");
        self.popup = Some(Popup {
            kind: PopupKind::Descripcion,
            machine: machine.name.clone(),
            machine_slug: machine.slug.clone(),
            buffers: Vec::new(),
            field: 0,
            notice: None,
            readonly: true,
            text: Some(text),
            flag_types: Vec::new(),
            lang_open: false, lang_cursor: 0, lang_selected: Vec::new(),
            type_open: false, type_cursor: 0,
        });
    }

    /// Queues a writeups popup for the selected machine (`w`, Machines).
    pub fn open_writeups_popup(&mut self) {
        if self.writeups_popup.is_some() || self.popup.is_some() || self.report.is_some() {
            return;
        }
        if self.tab != Tab::Machines {
            self.set_status(self.tr(Msg::WriteupsWrongTab));
            return;
        }
        let Some(machine) = self.selected_machine() else {
            self.set_status(self.tr(Msg::NothingSelected));
            return;
        };
        let entries: Vec<WriteupEntry> = self
            .data
            .writeups
            .iter()
            .filter(|(slug, _)| slug == &machine.slug)
            .map(|(_, w)| w.clone())
            .collect();
        if entries.is_empty() {
            self.set_status(self.tr(Msg::NoCommunityWriteups(machine.name.clone())));
            return;
        }
        self.writeups_popup = Some(WriteupsPopup {
            machine: machine.name.clone(),
            entries,
            selected: 0,
        });
    }

    /// Opens the writeup submission popup (`u`, Machines).
    pub fn open_writeup_submit_popup(&mut self) {
        if self.popup.is_some() || self.report.is_some() || self.writeups_popup.is_some() {
            return;
        }
        if self.tab != Tab::Machines {
            self.set_status(self.tr(Msg::SubmitWrongTab));
            return;
        }
        let Some(machine) = self.selected_machine() else {
            self.set_status(self.tr(Msg::NothingSelected));
            return;
        };
        if crate::config::ConfigManager::new()
            .username()
            .trim()
            .is_empty()
        {
            self.set_status(self.tr(Msg::SetUsernameFirst));
            return;
        }
        self.popup = Some(Popup {
            kind: PopupKind::WriteupSubmit,
            machine: machine.name.clone(),
            machine_slug: machine.slug.clone(),
            buffers: vec![String::new(), "Text".to_string(), "en".to_string()],
            field: 0,
            notice: None,
            readonly: false,
            text: None,
            flag_types: Vec::new(),
            lang_open: false, lang_cursor: 0, lang_selected: vec!["en"],
            type_open: false, type_cursor: 0,
        });
    }

    /// Opens the first-blood flag popup for the selected machine (`f`,
    /// Machines). Slots already taken render as notices; the popup only
    /// offers fields for the open slots.
    pub fn open_flag_popup(&mut self) {
        if self.popup.is_some() || self.report.is_some() {
            return;
        }
        if self.tab != Tab::Machines {
            self.set_status(self.tr(Msg::FlagsWrongTab));
            return;
        }
        let Some(machine) = self.selected_machine() else {
            self.set_status(self.tr(Msg::NothingSelected));
            return;
        };
        if crate::config::ConfigManager::new()
            .username()
            .trim()
            .is_empty()
        {
            self.set_status(self.tr(Msg::SetUsernameFirst));
            return;
        }
        let user_open = machine.user_slot_open();
        let root_open = machine.root_slot_open();
        if !user_open && !root_open {
            self.report = Some(ActionReport {
                title: format!(" First blood — {} ", machine.name),
                entries: vec![
                    (
                        ReportKind::Failure,
                        self.tr(Msg::SlotsTaken),
                    ),
                    (
                        ReportKind::Info,
                        self.tr(Msg::BloodHolders(
                            machine.first_user.clone(),
                            machine.first_root.clone(),
                        )),
                    ),
                ],
                changed: false,
                status: format!("[!] {}: no first-blood slots left.", machine.name),
            });
            return;
        }

        let mut buffers = Vec::new();
        let mut flag_types = Vec::new();
        let mut notices = Vec::new();
        if user_open {
            buffers.push(String::new());
            flag_types.push("user");
        } else {
            notices.push(self.tr(Msg::FlagTakenNotice("user", machine.first_user.clone())));
        }
        if root_open {
            buffers.push(String::new());
            flag_types.push("root");
        } else {
            notices.push(self.tr(Msg::FlagTakenNotice("root", machine.first_root.clone())));
        }
        let notice = if notices.is_empty() {
            None
        } else {
            Some(notices.join(" · "))
        };
        self.popup = Some(Popup {
            kind: PopupKind::Flag,
            machine: machine.name.clone(),
            machine_slug: machine.slug.clone(),
            buffers,
            field: 0,
            notice,
            readonly: false,
            text: None,
            flag_types,
            lang_open: false, lang_cursor: 0, lang_selected: Vec::new(),
            type_open: false, type_cursor: 0,
        });
    }

    /// Opens the VulnyX download page in the browser (`d`, Machines).
    pub fn open_download_page(&mut self) {
        if self.tab != Tab::Machines {
            self.set_status(self.tr(Msg::DownloadsWrongTab));
            return;
        }
        let Some(machine) = self.selected_machine() else {
            self.set_status(self.tr(Msg::NothingSelected));
            return;
        };
        let url = format!("https://vulnyx.com/download.php?vm={}", machine.name);
        let opened = std::process::Command::new("xdg-open")
            .arg(&url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        self.set_status(match opened {
            Ok(_) => self.tr(Msg::DownloadOpened(url)),
            Err(error) => self.tr(Msg::XdgOpenFailed(error.to_string())),
        });
    }

    /// Confirms the popup: validates input, queues the corresponding action.
    pub fn confirm_popup(&mut self) {
        let Some(popup) = self.popup.take() else {
            return;
        };
        let values: Vec<(usize, String)> = popup
            .buffers
            .iter()
            .enumerate()
            .map(|(index, b)| (index, b.trim().to_string()))
            .collect();

        match popup.kind {
            PopupKind::Username => {
                let username = values.first().map(|(_, v)| v.clone()).unwrap_or_default();
                if username.len() < 2 {
                    self.popup = Some(popup);
                    self.set_status(self.tr(Msg::UsernameTooShort));
                    return;
                }
                if let Err(error) = crate::config::ConfigManager::new().save_username(&username) {
                    self.set_status(self.tr(Msg::SaveFailed(format!("{error:#}"))));
                    return;
                }
                self.set_status(self.tr(Msg::UsernameSet(username)));
            }
            PopupKind::Flag => {
                // values carry "type:md5" — validate each and queue.
                let flag_types = popup.flag_types.clone();
                let mut typed = Vec::new();
                for ((_, value), flag_type) in values.iter().zip(flag_types.iter()) {
                    let flag = value.trim();
                    if flag.is_empty() {
                        continue;
                    }
                    if flag.len() != 32 || !flag.chars().all(|c| c.is_ascii_hexdigit()) {
                        self.popup = Some(popup);
                        self.set_status(self.tr(Msg::FlagNotMd5((*flag_type).to_string())));
                        return;
                    }
                    typed.push((0usize, format!("{flag_type}:{flag}")));
                }
                if typed.is_empty() {
                    self.popup = Some(popup);
                    self.set_status(self.tr(Msg::FillOneFlag));
                    return;
                }
                self.pending_action = Some(TuiAction {
                    kind: ActionKind::SubmitFlag,
                    machine: popup.machine_slug.clone(),
                    values: typed,
                    lang: self.lang,
                });
            }
            PopupKind::WriteupSubmit => {
                let url = values.first().map(|(_, v)| v.clone()).unwrap_or_default();
                if url.is_empty() {
                    self.popup = Some(popup);
                    self.set_status(self.tr(Msg::IndicateUrl));
                    return;
                }
                let tipo = values
                    .get(1)
                    .map(|(_, v)| v.clone())
                    .filter(|t| t == "Text" || t == "Video")
                    .unwrap_or_else(|| "Text".to_string());
                let language = values
                    .get(2)
                    .map(|(_, v)| v.clone())
                    .filter(|l| !l.is_empty())
                    .unwrap_or_else(|| "en".to_string());
                self.pending_action = Some(TuiAction {
                    kind: ActionKind::SubmitWriteup,
                    machine: popup.machine_slug.clone(),
                    values: vec![(0, url), (1, tipo), (2, language)],
                    lang: self.lang,
                });
            }
            PopupKind::Descripcion => {}
        }
    }

    fn row_count(&self) -> usize {
        match self.tab {
            Tab::Machines => self.visible_machines().len(),
            Tab::Progress => self
                .data
                .own_writeups(&crate::config::ConfigManager::new().username())
                .len(),
        }
    }

    pub fn move_down(&mut self) {
        let last = self.row_count().saturating_sub(1);
        self.selected = (self.selected + 1).min(last);
        self.ensure_selected_visible();
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.ensure_selected_visible();
    }

    pub fn move_start(&mut self) {
        self.selected = 0;
        self.scroll = 0;
    }

    /// Keeps `selected` inside the `[scroll, scroll + visible)` window.
    pub fn ensure_selected_visible(&mut self) {
        let visible = self.last_visible_rows.unwrap_or(10).max(1);
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + visible {
            self.scroll = self.selected + 1 - visible;
        }
    }

    /// Row budget reported by the renderer after layout.
    pub fn set_visible_rows(&mut self, rows: usize) {
        self.last_visible_rows = Some(rows.max(1));
        self.ensure_selected_visible();
    }

    pub fn enter_filter_mode(&mut self) {
        self.input_mode = InputMode::Filter;
    }

    pub fn filter_push(&mut self, c: char) {
        self.filter.push(c);
        self.reset_list_position();
    }

    pub fn filter_pop(&mut self) {
        self.filter.pop();
        self.reset_list_position();
    }

    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.reset_list_position();
    }

    /// Manual refresh: only allowed while idle to keep the state machine
    /// sane. The event loop owns the `fetching` label.
    pub fn request_refresh(&mut self) {
        if self.fetching.is_none() {
            self.refresh_requested = true;
        }
    }

    /// Whether the event loop should run a (re)fetch right now.
    pub fn should_fetch(&self, pending_fetch: bool) -> bool {
        pending_fetch || self.refresh_requested
    }
}

/// Host-provided callbacks the event loop calls synchronously (blocking the
/// render thread for the duration of each network call).
pub struct Host<'a> {
    pub refetch: &'a dyn Fn() -> Result<TuiData>,
    pub run_action: &'a dyn Fn(TuiAction) -> Result<ActionReport>,
    /// Set when the next loop iteration must (re)fetch all data; `run()`
    /// seeds it from the entry state's `fetching` label.
    pub pending_fetch: bool,
}

pub fn run(mut app: AppState, host: &mut Host<'_>) -> Result<()> {
    let mut terminal = ratatui::init();
    // Kick off the first load (and any pending request) before looping.
    host.pending_fetch = app.fetching.is_some();
    let result = event_loop(&mut terminal, &mut app, host);
    ratatui::restore();
    result
}

fn event_loop(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    app: &mut AppState,
    host: &mut Host<'_>,
) -> Result<()> {
    loop {
        terminal.draw(|frame| crate::tui::render::draw(frame, app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(app, key);
                }
            }
        }

        app.tick();

        // User actions from popups (submit flag, submit writeup).
        if let Some(action) = app.pending_action.take() {
            let label = match action.kind {
                ActionKind::SubmitFlag => {
                    format!("Submitting flag for {}...", action.machine)
                }
                ActionKind::SubmitWriteup => {
                    format!("Submitting writeup for {}...", action.machine)
                }
            };
            app.fetching = Some(label);
            match (host.run_action)(action) {
                Ok(report) => {
                    app.set_status(report.status.clone());
                    app.pending_refresh_after_close = report.changed;
                    app.report = Some(report);
                }
                Err(error) => app.set_status(app.tr(Msg::ActionFailed(format!("{error:#}")))),
            }
            app.fetching = None;
        }

        if app.should_fetch(host.pending_fetch) {
            host.pending_fetch = false;
            app.refresh_requested = false;
            // Draw immediately so the `⟳ <label>` shows while the blocking
            // fetch runs, instead of freezing silently.
            app.fetching = Some(Msg::FetchingRefresh.render(app.lang));
            terminal.draw(|frame| crate::tui::render::draw(frame, app))?;

            let result = (host.refetch)();
            app.fetching = None;
            match result {
                Ok(data) => {
                    app.set_data(data);
                    app.set_status(app.tr(Msg::DataRefreshed));
                }
                Err(error) => app.set_status(app.tr(Msg::FetchFailed(format!("{error:#}")))),
            }
        }

        if app.quit {
            return Ok(());
        }
    }
}

fn handle_key(app: &mut AppState, key: crossterm::event::KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.request_quit();
        return;
    }

    // Writeups popup captures everything until dismissed.
    if app.writeups_popup.is_some() {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => app.writeups_popup = None,
            KeyCode::Enter => {
                // The popup always carries the machine's writeups, regardless
                // of the tab underneath.
                if let Some(url) = app.writeups_popup.as_ref().and_then(|p| p.selected_url()) {
                    let opened = std::process::Command::new("xdg-open")
                        .arg(url)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();
                    app.set_status(match opened {
                        Ok(_) => app.tr(Msg::OpenedInBrowser(url.to_string())),
                        Err(error) => app.tr(Msg::XdgOpenFailed(error.to_string())),
                    });
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(popup) = app.writeups_popup.as_mut() {
                    popup.move_selection(-1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(popup) = app.writeups_popup.as_mut() {
                    popup.move_selection(1);
                }
            }
            _ => {}
        }
        return;
    }

    // Result report popup captures everything until dismissed.
    if app.report.is_some() {
        match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                let _ = app.close_report();
            }
            _ => {}
        }
        return;
    }

    // Generic popup input mode captures everything first.
    if app.popup.is_some() {
        if app.popup.as_ref().map(|p| p.readonly) == Some(true) {
            match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => app.popup = None,
                _ => {}
            }
            return;
        }
        let (lang_field, lang_open, type_field, type_open) = app
            .popup
            .as_ref()
            .map(|p| {
                (
                    p.lang_field_active(),
                    p.lang_open,
                    p.type_field_active(),
                    p.type_open,
                )
            })
            .unwrap_or((false, false, false, false));
        let panel_open = lang_open || type_open;
        if panel_open {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(p) = app.popup.as_mut() {
                        if p.lang_open {
                            p.move_lang_cursor(-1);
                        }
                        if p.type_open {
                            p.move_type_cursor(-1);
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(p) = app.popup.as_mut() {
                        if p.lang_open {
                            p.move_lang_cursor(1);
                        }
                        if p.type_open {
                            p.move_type_cursor(1);
                        }
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(p) = app.popup.as_mut() {
                        if p.lang_open {
                            p.toggle_lang();
                        }
                        if p.type_open {
                            p.select_type();
                        }
                    }
                }
                KeyCode::Enter | KeyCode::Esc | KeyCode::Tab => {
                    if let Some(p) = app.popup.as_mut() {
                        p.close_lang_panel();
                        p.close_type_panel();
                    }
                }
                _ => {}
            }
            return;
        }
        if lang_field && matches!(key.code, KeyCode::Char(' ')) {
            if let Some(p) = app.popup.as_mut() {
                p.open_lang_panel();
            }
            return;
        }
        if type_field && matches!(key.code, KeyCode::Char(' ')) {
            if let Some(p) = app.popup.as_mut() {
                p.open_type_panel();
            }
            return;
        }
        // On a picker field with the panel closed, every other key falls
        // through to the generic match below: push/pop are no-ops there,
        // Enter still confirms and Esc still cancels.

        match key.code {
            KeyCode::Esc => {
                app.popup = None;
                app.set_status(app.tr(Msg::Cancelled));
            }
            KeyCode::Enter => app.confirm_popup(),
            KeyCode::Backspace => {
                if let Some(popup) = app.popup.as_mut() {
                    popup.pop();
                }
            }
            KeyCode::Tab => {
                if let Some(popup) = app.popup.as_mut() {
                    popup.next_field();
                }
            }
            KeyCode::Down => {
                if let Some(popup) = app.popup.as_mut() {
                    popup.next_field();
                }
            }
            KeyCode::Up => {
                if let Some(popup) = app.popup.as_mut() {
                    popup.previous_field();
                }
            }
            KeyCode::Char(c) => {
                if let Some(popup) = app.popup.as_mut() {
                    popup.push(c);
                }
            }
            _ => {}
        }
        return;
    }
    match app.input_mode {
        InputMode::Filter => match key.code {
            KeyCode::Esc => {
                app.clear_filter();
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => app.input_mode = InputMode::Normal,
            KeyCode::Backspace => app.filter_pop(),
            KeyCode::Char(c) => app.filter_push(c),
            _ => {}
        },
        InputMode::Normal => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => app.request_quit(),
            KeyCode::Char('r') => app.request_refresh(),
            KeyCode::Tab | KeyCode::Right => app.next_tab(),
            KeyCode::Left | KeyCode::BackTab => app.previous_tab(),
            KeyCode::Down | KeyCode::Char('j') => app.move_down(),
            KeyCode::Up | KeyCode::Char('k') => app.move_up(),
            KeyCode::Home | KeyCode::Char('g') => app.move_start(),
            KeyCode::Char('/') => app.enter_filter_mode(),
            KeyCode::Char('a') => app.open_username_popup(),
            KeyCode::Char('l') => {
                app.lang = app.lang.toggle();
                let _ = crate::config::ConfigManager::new().save_language(app.lang);
                app.set_status(app.tr(Msg::LanguageSet(app.lang)));
            }
            KeyCode::Char('s') => {
                if app.tab == Tab::Machines {
                    app.machine_sort = app.machine_sort.next();
                    app.reset_list_position();
                }
            }
            KeyCode::Char('d') => app.open_download_page(),
            KeyCode::Char('f') => app.open_flag_popup(),
            KeyCode::Char('w') => app.open_writeups_popup(),
            KeyCode::Char('u') => app.open_writeup_submit_popup(),
            KeyCode::Char('b') => {
                app.only_first_blood = !app.only_first_blood;
                app.reset_list_position();
            }
            KeyCode::Char('m') => {
                if app.tab == Tab::Machines {
                    app.toggle_completed();
                    app.reset_list_position();
                }
            }
            KeyCode::Char('x') => {
                if app.tab == Tab::Machines {
                    app.hide_completed = !app.hide_completed;
                    app.reset_list_position();
                    let note = if app.hide_completed {
                        app.tr(Msg::CompletedHidden)
                    } else {
                        app.tr(Msg::CompletedShown)
                    };
                    app.set_status(note);
                }
            }
            KeyCode::Char('i') => app.open_descripcion_popup(),
            KeyCode::Enter => match app.tab {
                Tab::Machines => app.open_descripcion_popup(),
                Tab::Progress => app.open_selected_writeup_link(),
            },
            _ => {}
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn submit_popup() -> Popup {
        Popup {
            kind: PopupKind::WriteupSubmit,
            machine: String::new(),
            machine_slug: String::new(),
            buffers: vec![String::new(), "Text".to_string(), "en".to_string()],
            field: 2,
            notice: None,
            readonly: false,
            text: None,
            flag_types: Vec::new(),
            lang_open: false,
            lang_cursor: 0,
            lang_selected: vec!["en"],
            type_open: false,
            type_cursor: 0,
        }
    }

    fn lang_index(code: &str) -> usize {
        WRITEUP_LANGS
            .iter()
            .position(|(c, _)| *c == code)
            .unwrap_or_else(|| panic!("language {code} missing from WRITEUP_LANGS"))
    }

    #[test]
    fn join_langs_orders_canonically() {
        assert_eq!(join_langs(&["es", "en"]), "en,es");
        assert_eq!(join_langs(&[]), "");
        assert_eq!(join_langs(&["en"]), "en");
    }

    #[test]
    fn writeup_langs_snapshot_is_complete() {
        assert_eq!(WRITEUP_LANGS.len(), 62);
        assert_eq!(WRITEUP_LANGS.first().map(|(c, _)| *c), Some("en"));
        assert_eq!(WRITEUP_LANGS.last().map(|(c, _)| *c), Some("none"));
    }

    #[test]
    fn toggle_lang_updates_buffer_and_selection() {
        let mut popup = submit_popup();
        popup.lang_cursor = lang_index("it");
        popup.toggle_lang();
        assert_eq!(popup.buffers[2], "en,it");
        popup.toggle_lang();
        assert_eq!(popup.buffers[2], "en");
        assert_eq!(popup.lang_selected, vec!["en"]);
        popup.lang_cursor = lang_index("es");
        popup.toggle_lang();
        assert_eq!(popup.buffers[2], "en,es");
    }

    #[test]
    fn type_select_sets_buffer_single_choice() {
        let mut popup = submit_popup();
        popup.field = 1;
        assert!(popup.type_field_active());
        popup.open_type_panel();
        popup.move_type_cursor(1);
        popup.select_type();
        assert_eq!(popup.buffers[1], "Video");
        popup.move_type_cursor(-1);
        popup.select_type();
        assert_eq!(popup.buffers[1], "Text");
    }

    #[test]
    fn type_field_rejects_text_input() {
        let mut popup = submit_popup();
        popup.field = 1;
        popup.push('x');
        popup.pop();
        assert_eq!(popup.buffers[1], "Text");
    }

    #[test]
    fn language_field_rejects_text_input() {
        let mut popup = submit_popup();
        assert!(popup.lang_field_active());
        popup.push('x');
        popup.pop();
        assert_eq!(popup.buffers[2], "en");
    }
}
