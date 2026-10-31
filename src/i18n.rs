//! UI language support: which language the interface renders in (`l`
//! toggles), and the catalog of user-visible strings.
//!
//! Jargon policy: platform terms — first blood, flag, writeup, root, user,
//! MD5, leaderboard, URL — stay English in both languages (the Spanish
//! Vulnyx community uses them untranslated). Difficulty and OS values come
//! from site data and are never touched.

/// Interface language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Es,
}

impl Lang {
    pub fn toggle(self) -> Self {
        match self {
            Lang::En => Lang::Es,
            Lang::Es => Lang::En,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
        }
    }

    /// Human-readable name of the language (its own name, untranslated).
    pub fn display_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
        }
    }

    /// Invalid or empty code falls back to English.
    pub fn from_code(code: &str) -> Self {
        match code.trim().to_lowercase().as_str() {
            "es" => Lang::Es,
            _ => Lang::En,
        }
    }
}

/// One user-visible string of the interface.
#[derive(Debug, Clone)]
pub enum Msg {
    MachinesCount(usize),
    HeaderFirstBlood(usize, usize),
    Leaderboard(usize),
    LeaderboardNone,
    ColMachine,
    ColDifficulty,
    ColOs,
    ColCreator,
    ColDate,
    ColFirstBlood,
    ColType,
    ColUrl,
    ColAuthor,
    MachinesTitle(usize, usize),
    DoneNote(usize, bool),
    FbOnly,
    FbOpen(usize),
    ProgressTitle(usize, usize),
    SortSite,
    SortName,
    SortDate,
    SortDifficulty,
    SubmitFlagTitle(String),
    SubmitWriteupTitle(String),
    UsernameTitle,
    PromptUserFlag,
    PromptRootFlag,
    PromptUrl,
    PromptType,
    PromptLanguage,
    PromptUsername,
    HintSubmit,
    HintSubmitPickers,
    HintSave,
    HintEscClose,
    HintEnterEscClose,
    HintWriteupsRow,
    LangPanelHeader,
    TypePanelHeader,
    FooterMachines,
    FooterProgress,
    FooterGlobal,
    FooterFilter,
    FooterPopupDefault,
    FetchingLoad,
    FetchingRefresh,
    Cancelled,
    NothingSelected,
    MarkedCompleted(String),
    MarkedNotCompleted(String),
    CompletedHidden,
    CompletedShown,
    LanguageSet(Lang),
    OpenedInBrowser(String),
    BrowserOpenFailed(String),
    NoWriteupLink,
    WriteupsWrongTab,
    NoCommunityWriteups(String),
    SubmitWrongTab,
    SetUsernameFirst,
    FlagsWrongTab,
    DownloadsWrongTab,
    UsernameTooShort,
    SaveFailed(String),
    UsernameSet(String),
    FlagNotMd5(String),
    FillOneFlag,
    IndicateUrl,
    ActionFailed(String),
    FetchFailed(String),
    DataRefreshed,
    SlotsTaken,
    TakenBy(String),
    FlagTakenNotice(&'static str, String),
    BloodHolders(String, String),
    DescDifficulty(String, String, u64, u64),
    DescPlatform(String, String),
    DescCreator(String, String),
    DescMd5,
    DescTags(String),
    DescBloodUser(String),
    DescBloodRoot(String),
    SlotOpen,
    FlagAccepted(&'static str, String),
    NoFlagSubmitted,
    FlagsSubmitted(usize, String),
    WriteupSubmitted(String),
    WriteupPending(String),
    CaptchaSolving(String),
    DownloadLinkReady(String),
    DownloadFallback(String),
    DownloadRateLimited,
    FlagsReportTitle(String),
    WriteupReportTitle(String),
    UsernameNotice,
    NoSlotsLeft(String),
    SubmittingFlag(String),
    SubmittingWriteup(String),
    ProgressIdentity,
    ProgressSelfDeclared,
    ProgressFirstBloods,
    ProgressNoBlood,
    ProgressStatistics,
    ProgressStats(u64, u64, usize, String, u64),
    YourWriteups(usize),
    WriteupsTitle(String),
    DownloadStarted(String),
    DownloadDone(String),
    DownloadFailed(String),
    DownloadProgress(String, String, u64, u64, u64),
    DownloadAlreadyPresent(String),
    DownloadDirSet(String),
    MarkDoneConfirm(String),
    UnmarkDoneConfirm(String),
    HelpTitle,
    HelpText,
    DownloadDirTitle,
    PromptDownloadDir,
    HintHelpScroll,
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

impl Msg {
    /// Renders the message in `lang`.
    pub fn render(self, lang: Lang) -> String {
        match self {
            Msg::MachinesCount(n) => match lang {
                Lang::En => format!("{n} machines"),
                Lang::Es => format!("{n} máquinas"),
            },
            Msg::HeaderFirstBlood(u, r) => match lang {
                Lang::En => format!("first blood: user {u} · root {r}"),
                Lang::Es => format!("first blood: usuario {u} · root {r}"),
            },
            Msg::Leaderboard(rank) => format!("leaderboard #{rank}"),
            Msg::LeaderboardNone => "leaderboard -".to_string(),
            Msg::ColMachine => match lang {
                Lang::En => "Machine",
                Lang::Es => "Máquina",
            }
            .to_string(),
            Msg::ColDifficulty => match lang {
                Lang::En => "Difficulty",
                Lang::Es => "Dificultad",
            }
            .to_string(),
            Msg::ColOs => match lang {
                Lang::En => "OS",
                Lang::Es => "SO",
            }
            .to_string(),
            Msg::ColCreator => match lang {
                Lang::En => "Creator",
                Lang::Es => "Creador",
            }
            .to_string(),
            Msg::ColDate => match lang {
                Lang::En => "Date",
                Lang::Es => "Fecha",
            }
            .to_string(),
            Msg::ColFirstBlood => match lang {
                Lang::En => "First Blood",
                Lang::Es => "First Blood",
            }
            .to_string(),
            Msg::ColType => match lang {
                Lang::En => "Type",
                Lang::Es => "Tipo",
            }
            .to_string(),
            Msg::ColUrl => "URL".to_string(),
            Msg::ColAuthor => match lang {
                Lang::En => "Author",
                Lang::Es => "Autor",
            }
            .to_string(),
            Msg::MachinesTitle(vis, total) => match lang {
                Lang::En => format!(" Machines {vis}/{total} "),
                Lang::Es => format!(" Máquinas {vis}/{total} "),
            },
            Msg::DoneNote(n, hidden) => match lang {
                Lang::En => {
                    if hidden {
                        format!(" · done: {n} (hidden)")
                    } else {
                        format!(" · done: {n}")
                    }
                }
                Lang::Es => {
                    if hidden {
                        format!(" · hechas: {n} (ocultas)")
                    } else {
                        format!(" · hechas: {n}")
                    }
                }
            },
            Msg::FbOnly => match lang {
                Lang::En => " · first blood only",
                Lang::Es => " · solo first blood",
            }
            .to_string(),
            Msg::FbOpen(n) => match lang {
                Lang::En => format!(" · first blood open: {n}"),
                Lang::Es => format!(" · first blood libre: {n}"),
            },
            Msg::ProgressTitle(fb, wu) => format!(" First bloods {fb} · writeups {wu} "),
            Msg::SortSite => String::new(),
            Msg::SortName => match lang {
                Lang::En => " · sort: name",
                Lang::Es => " · orden: nombre",
            }
            .to_string(),
            Msg::SortDate => match lang {
                Lang::En => " · sort: date",
                Lang::Es => " · orden: fecha",
            }
            .to_string(),
            Msg::SortDifficulty => match lang {
                Lang::En => " · sort: difficulty",
                Lang::Es => " · orden: dificultad",
            }
            .to_string(),
            Msg::SubmitFlagTitle(machine) => format!(" First blood — {machine} "),
            Msg::SubmitWriteupTitle(machine) => match lang {
                Lang::En => format!(" Submit writeup — {machine} "),
                Lang::Es => format!(" Enviar writeup — {machine} "),
            },
            Msg::UsernameTitle => match lang {
                Lang::En => " Your username ",
                Lang::Es => " Tu usuario ",
            }
            .to_string(),
            Msg::PromptUserFlag => "User flag (MD5):".to_string(),
            Msg::PromptRootFlag => "Root flag (MD5):".to_string(),
            Msg::PromptUrl => "URL:".to_string(),
            Msg::PromptType => match lang {
                Lang::En => "Type:",
                Lang::Es => "Tipo:",
            }
            .to_string(),
            Msg::PromptLanguage => match lang {
                Lang::En => "Language:",
                Lang::Es => "Idioma:",
            }
            .to_string(),
            Msg::PromptUsername => match lang {
                Lang::En => "Username:",
                Lang::Es => "Usuario:",
            }
            .to_string(),
            Msg::HintSubmit => match lang {
                Lang::En => "Enter submit · ↑↓/Tab switch field · Esc cancel",
                Lang::Es => "Enter enviar · ↑↓/Tab cambiar campo · Esc cancelar",
            }
            .to_string(),
            Msg::HintSubmitPickers => match lang {
                Lang::En => "Enter submit · Space pick type/language · ↑↓/Tab switch field · Esc cancel",
                Lang::Es => "Enter enviar · Space elegir tipo/idioma · ↑↓/Tab cambiar campo · Esc cancelar",
            }
            .to_string(),
            Msg::HintSave => match lang {
                Lang::En => "Enter save · Esc cancel",
                Lang::Es => "Enter guardar · Esc cancelar",
            }
            .to_string(),
            Msg::HintEscClose => match lang {
                Lang::En => "Esc close",
                Lang::Es => "Esc cerrar",
            }
            .to_string(),
            Msg::HintEnterEscClose => match lang {
                Lang::En => "Enter / Esc close",
                Lang::Es => "Enter / Esc cerrar",
            }
            .to_string(),
            Msg::HintWriteupsRow => match lang {
                Lang::En => "Enter open · jk select · Esc close",
                Lang::Es => "Enter abrir · jk elegir · Esc cerrar",
            }
            .to_string(),
            Msg::LangPanelHeader => match lang {
                Lang::En => "↑↓ move · Space toggle · Enter/Esc done",
                Lang::Es => "↑↓ mover · Space marcar · Enter/Esc listo",
            }
            .to_string(),
            Msg::TypePanelHeader => match lang {
                Lang::En => "↑↓ move · Space select · Enter/Esc done",
                Lang::Es => "↑↓ mover · Space elegir · Enter/Esc listo",
            }
            .to_string(),
            Msg::FooterMachines => match lang {
                Lang::En => "jk move · / filter · d download · f flag · m done · i info · h help",
                Lang::Es => "jk mover · / filtro · d descargar · f flag · m hecha · i info · h ayuda",
            }
            .to_string(),
            Msg::FooterProgress => match lang {
                Lang::En => "jk move · Enter open writeup",
                Lang::Es => "jk mover · Enter abrir writeup",
            }
            .to_string(),
            Msg::FooterGlobal => match lang {
                Lang::En => "Tab tabs · a username · l language · r refresh · D folder · q quit",
                Lang::Es => "Tab pestañas · a usuario · l idioma · r refrescar · D carpeta · q salir",
            }
            .to_string(),
            Msg::FooterFilter => match lang {
                Lang::En => "Enter confirm · Esc clears & exits",
                Lang::Es => "Enter confirma · Esc limpia y sale",
            }
            .to_string(),
            Msg::FooterPopupDefault => match lang {
                Lang::En => "Enter confirm · Esc cancel",
                Lang::Es => "Enter confirmar · Esc cancelar",
            }
            .to_string(),
            Msg::FetchingLoad => match lang {
                Lang::En => "Loading data...",
                Lang::Es => "Cargando datos...",
            }
            .to_string(),
            Msg::FetchingRefresh => match lang {
                Lang::En => "Refreshing data...",
                Lang::Es => "Actualizando datos...",
            }
            .to_string(),
            Msg::Cancelled => match lang {
                Lang::En => "Cancelled.",
                Lang::Es => "Cancelado.",
            }
            .to_string(),
            Msg::NothingSelected => match lang {
                Lang::En => "Nothing selected.",
                Lang::Es => "Nada seleccionado.",
            }
            .to_string(),
            Msg::MarkedCompleted(name) => match lang {
                Lang::En => format!("{name} marked as completed."),
                Lang::Es => format!("{name} marcada como completada."),
            },
            Msg::MarkedNotCompleted(name) => match lang {
                Lang::En => format!("{name} marked as not completed."),
                Lang::Es => format!("{name} marcada como no completada."),
            },
            Msg::CompletedHidden => match lang {
                Lang::En => "completed machines hidden",
                Lang::Es => "máquinas completadas ocultas",
            }
            .to_string(),
            Msg::CompletedShown => match lang {
                Lang::En => "completed machines shown",
                Lang::Es => "máquinas completadas visibles",
            }
            .to_string(),
            Msg::LanguageSet(l) => match lang {
                Lang::En => format!("Language: {}", l.display_name()),
                Lang::Es => format!("Idioma: {}", l.display_name()),
            },
            Msg::OpenedInBrowser(url) => match lang {
                Lang::En => format!("Opened in browser: {url}"),
                Lang::Es => format!("Abierto en el navegador: {url}"),
            },
            Msg::BrowserOpenFailed(error) => match lang {
                Lang::En => format!("Could not open the browser: {error}"),
                Lang::Es => format!("No se pudo abrir el navegador: {error}"),
            },
            Msg::NoWriteupLink => match lang {
                Lang::En => "No writeup link on this row.",
                Lang::Es => "No hay enlace de writeup en esta fila.",
            }
            .to_string(),
            Msg::WriteupsWrongTab => match lang {
                Lang::En => "Writeups are available on the Machines tab.",
                Lang::Es => "Los writeups están en la pestaña Machines.",
            }
            .to_string(),
            Msg::NoCommunityWriteups(machine) => match lang {
                Lang::En => format!("No community writeups for {machine} yet."),
                Lang::Es => format!("Aún no hay writeups de la comunidad para {machine}."),
            },
            Msg::SubmitWrongTab => match lang {
                Lang::En => "Writeup submission is available on the Machines tab.",
                Lang::Es => "El envío de writeups está en la pestaña Machines.",
            }
            .to_string(),
            Msg::SetUsernameFirst => match lang {
                Lang::En => "Set your username first (a).",
                Lang::Es => "Primero configura tu usuario (a).",
            }
            .to_string(),
            Msg::FlagsWrongTab => match lang {
                Lang::En => "Flags are only available on the Machines tab.",
                Lang::Es => "Los flags están solo en la pestaña Machines.",
            }
            .to_string(),
            Msg::DownloadsWrongTab => match lang {
                Lang::En => "Downloads are only available on the Machines tab.",
                Lang::Es => "Las descargas están solo en la pestaña Machines.",
            }
            .to_string(),
            Msg::UsernameTooShort => match lang {
                Lang::En => "The username needs at least 2 characters.",
                Lang::Es => "El usuario necesita al menos 2 caracteres.",
            }
            .to_string(),
            Msg::SaveFailed(error) => match lang {
                Lang::En => format!("Failed to save: {error}"),
                Lang::Es => format!("Error al guardar: {error}"),
            },
            Msg::UsernameSet(username) => match lang {
                Lang::En => format!("[✓] Username set to {username}."),
                Lang::Es => format!("[✓] Usuario establecido en {username}."),
            },
            Msg::FlagNotMd5(flag_type) => match lang {
                Lang::En => format!(
                    "The {flag_type} flag must be an MD5 hash: 32 hex characters."
                ),
                Lang::Es => format!(
                    "El flag {flag_type} debe ser un hash MD5: 32 caracteres hexadecimales."
                ),
            },
            Msg::FillOneFlag => match lang {
                Lang::En => "Fill at least one flag.",
                Lang::Es => "Completa al menos un flag.",
            }
            .to_string(),
            Msg::IndicateUrl => match lang {
                Lang::En => "Indicate the writeup URL.",
                Lang::Es => "Indica la URL del writeup.",
            }
            .to_string(),
            Msg::ActionFailed(error) => match lang {
                Lang::En => format!("Action failed: {error}"),
                Lang::Es => format!("Acción fallida: {error}"),
            },
            Msg::FetchFailed(error) => match lang {
                Lang::En => format!("Fetch failed: {error}"),
                Lang::Es => format!("Error al obtener datos: {error}"),
            },
            Msg::DataRefreshed => match lang {
                Lang::En => "Data refreshed.",
                Lang::Es => "Datos actualizados.",
            }
            .to_string(),
            Msg::SlotsTaken => match lang {
                Lang::En => "First blood: ✗ BOTH SLOTS ALREADY TAKEN",
                Lang::Es => "First blood: ✗ AMBOS HUECOS YA TOMADOS",
            }
            .to_string(),
            Msg::TakenBy(name) => match lang {
                Lang::En => format!("taken by {name}"),
                Lang::Es => format!("tomado por {name}"),
            },
            Msg::FlagTakenNotice(kind, name) => match lang {
                Lang::En => format!("{kind} flag: taken by {name}"),
                Lang::Es => format!("Flag {kind}: tomado por {name}"),
            },
            Msg::BloodHolders(user, root) => format!("user → {user} · root → {root}"),
            Msg::DescDifficulty(difficulty, os, pts_user, pts_root) => match lang {
                Lang::En => format!(
                    "Difficulty: {difficulty} · OS: {os} · Points: {pts_user}/{pts_root}"
                ),
                Lang::Es => format!(
                    "Dificultad: {difficulty} · SO: {os} · Puntos: {pts_user}/{pts_root}"
                ),
            },
            Msg::DescPlatform(platforms, size) => match lang {
                Lang::En => format!("Platform: {platforms} · Size: {size}"),
                Lang::Es => format!("Plataforma: {platforms} · Tamaño: {size}"),
            },
            Msg::DescCreator(creator, date) => match lang {
                Lang::En => format!("Creator: {creator} · Date: {date}"),
                Lang::Es => format!("Creador: {creator} · Fecha: {date}"),
            },
            Msg::DescMd5 => "MD5:".to_string(),
            Msg::DescTags(tags) => match lang {
                Lang::En => format!("Tech tags: {tags}"),
                Lang::Es => format!("Etiquetas: {tags}"),
            },
            Msg::DescBloodUser(holder) => format!("First blood user: {holder}"),
            Msg::DescBloodRoot(holder) => format!("First blood root: {holder}"),
            Msg::SlotOpen => match lang {
                Lang::En => "OPEN — claim it!",
                Lang::Es => "¡LIBRE — reclámalo!",
            }
            .to_string(),
            Msg::FlagAccepted(flag_type, message) => match lang {
                Lang::En => format!("{flag_type} flag: ✓ ACCEPTED — {message}"),
                Lang::Es => format!("Flag {flag_type}: ✓ ACEPTADO — {message}"),
            },
            Msg::NoFlagSubmitted => match lang {
                Lang::En => "No flag was submitted.",
                Lang::Es => "No se envió ningún flag.",
            }
            .to_string(),
            Msg::FlagsSubmitted(count, machine) => match lang {
                Lang::En => format!(
                    "[✓] {count} flag(s) submitted for {machine} — check First Bloods in Progress."
                ),
                Lang::Es => format!(
                    "[✓] {count} flag(s) enviados para {machine} — revisa First Bloods en Progress."
                ),
            },
            Msg::WriteupSubmitted(message) => match lang {
                Lang::En => format!("Writeup: ✓ SUBMITTED — {message}"),
                Lang::Es => format!("Writeup: ✓ ENVIADO — {message}"),
            },
            Msg::WriteupPending(machine) => match lang {
                Lang::En => format!(
                    "[✓] Writeup submitted for {machine} — pending admin review (48h)."
                ),
                Lang::Es => format!(
                    "[✓] Writeup enviado para {machine} — pendiente de revisión (48h)."
                ),
            },
            Msg::CaptchaSolving(machine) => match lang {
                Lang::En => format!("Reading the CAPTCHA for {machine}…"),
                Lang::Es => format!("Leyendo el CAPTCHA de {machine}…"),
            },
            Msg::DownloadLinkReady(url) => match lang {
                Lang::En => format!("[↓] Proton Drive link opened: {url}"),
                Lang::Es => format!("[↓] Enlace de Proton Drive abierto: {url}"),
            },
            Msg::DownloadFallback(reason) => match lang {
                Lang::En => format!("[!] {reason} — download page opened in the browser."),
                Lang::Es => {
                    format!("[!] {reason} — página de descarga abierta en el navegador.")
                }
            },
            Msg::DownloadRateLimited => match lang {
                Lang::En => {
                    "[!] VulnyX allows 2 VM downloads per minute.".to_string()
                }
                Lang::Es => "[!] VulnyX permite 2 descargas por minuto.".to_string(),
            },
            Msg::FlagsReportTitle(machine) => format!(" Flags — {machine} "),
            Msg::WriteupReportTitle(machine) => format!(" Writeup — {machine} "),
            Msg::UsernameNotice => match lang {
                Lang::En => {
                    "Your username is attached to flags, writeups and the leaderboard.".to_string()
                }
                Lang::Es => {
                    "Tu usuario se adjunta a flags, writeups y la clasificación.".to_string()
                }
            },
            Msg::NoSlotsLeft(machine) => match lang {
                Lang::En => format!("[!] {machine}: no first-blood slots left."),
                Lang::Es => format!("[!] {machine}: no quedan slots de first blood."),
            },
            Msg::SubmittingFlag(machine) => match lang {
                Lang::En => format!("Submitting flag for {machine}…"),
                Lang::Es => format!("Enviando flag de {machine}…"),
            },
            Msg::SubmittingWriteup(machine) => match lang {
                Lang::En => format!("Submitting writeup for {machine}…"),
                Lang::Es => format!("Enviando writeup de {machine}…"),
            },
            Msg::ProgressIdentity => match lang {
                Lang::En => "[ Identity ]",
                Lang::Es => "[ Identidad ]",
            }
            .to_string(),
            Msg::ProgressSelfDeclared => match lang {
                Lang::En => "(self-declared — attached to flags & writeups)",
                Lang::Es => "(autodeclarado — se adjunta a flags y writeups)",
            }
            .to_string(),
            Msg::ProgressFirstBloods => "[ First Bloods ]".to_string(),
            Msg::ProgressNoBlood => match lang {
                Lang::En => "None yet — first to submit a flag gets the blood.",
                Lang::Es => "Ninguna aún — el primero en enviar un flag se lleva la sangre.",
            }
            .to_string(),
            Msg::ProgressStatistics => match lang {
                Lang::En => "[ Statistics ]",
                Lang::Es => "[ Estadísticas ]",
            }
            .to_string(),
            Msg::ProgressStats(user, root, writeups, position, points) => match lang {
                Lang::En => format!(
                    "First blood  : user {user} · root {root}\nWriteups     : {writeups}\nLeaderboard  : #{position} ({points} pts)"
                ),
                Lang::Es => format!(
                    "First blood  : usuario {user} · root {root}\nWriteups     : {writeups}\nLeaderboard  : #{position} ({points} pts)"
                ),
            },
            Msg::YourWriteups(count) => match lang {
                Lang::En => format!(" Your writeups ({count}) "),
                Lang::Es => format!(" Tus writeups ({count}) "),
            },
            Msg::WriteupsTitle(machine) => format!(" Writeups — {machine} "),
            Msg::DownloadStarted(name) => match lang {
                Lang::En => format!("Downloading {name} from Proton Drive…"),
                Lang::Es => format!("Descargando {name} desde Proton Drive…"),
            },
            Msg::DownloadDone(name) => match lang {
                Lang::En => format!("[✓] {name} downloaded."),
                Lang::Es => format!("[✓] {name} descargada."),
            },
            Msg::DownloadFailed(error) => match lang {
                Lang::En => format!("[!] Proton Drive download failed: {error}"),
                Lang::Es => format!("[!] Falló la descarga desde Proton Drive: {error}"),
            },
            Msg::DownloadProgress(machine, file, done, total, speed) => {
                if total == 0 {
                    format!("↓ {machine} · {file} · {}", fmt_bytes(done))
                } else {
                    format!(
                        "↓ {machine} · {file} · {}/{} · {}/s",
                        fmt_bytes(done),
                        fmt_bytes(total),
                        fmt_bytes(speed)
                    )
                }
            }
            Msg::DownloadAlreadyPresent(name) => match lang {
                Lang::En => format!("[✓] {name} is already downloaded."),
                Lang::Es => format!("[✓] {name} ya está descargada."),
            },
            Msg::DownloadDirSet(path) => match lang {
                Lang::En => format!("[✓] Downloads go to {path}"),
                Lang::Es => format!("[✓] Las descargas van a {path}"),
            },
            Msg::MarkDoneConfirm(name) => match lang {
                Lang::En => format!(
                    "Mark {name} as completed?\n\nThis only affects your local list."
                ),
                Lang::Es => format!(
                    "¿Marcar {name} como completada?\n\nSolo afecta a tu lista local."
                ),
            },
            Msg::UnmarkDoneConfirm(name) => match lang {
                Lang::En => {
                    format!("{name} is already marked as completed.\n\nRemove the mark?")
                }
                Lang::Es => {
                    format!("{name} ya está marcada como completada.\n\n¿Quitar la marca?")
                }
            },
            Msg::HelpTitle => match lang {
                Lang::En => " Keyboard shortcuts ",
                Lang::Es => " Atajos de teclado ",
            }
            .to_string(),
            Msg::HelpText => match lang {
                Lang::En => HELP_TEXT_EN,
                Lang::Es => HELP_TEXT_ES,
            }
            .to_string(),
            Msg::DownloadDirTitle => match lang {
                Lang::En => " Download folder ",
                Lang::Es => " Carpeta de descargas ",
            }
            .to_string(),
            Msg::PromptDownloadDir => match lang {
                Lang::En => "Folder:",
                Lang::Es => "Carpeta:",
            }
            .to_string(),
            Msg::HintHelpScroll => match lang {
                Lang::En => "jk scroll · Esc close",
                Lang::Es => "jk desplazar · Esc cerrar",
            }
            .to_string(),
        }
    }
}

const HELP_TEXT_EN: &str = "[ Navigation ]
  Tab / →        next tab
  ← / Shift-Tab  previous tab
  j / ↓          move down
  k / ↑          move up
  g / Home       jump to the first row
  /              filter (Enter confirm · Esc clears and exits)

[ Machines ]
  d              solve the CAPTCHA and download the archive
  D              set the download folder (absolute path)
  f              submit first-blood flags
  w              community writeups
  u              submit a writeup
  b              show only machines with an open slot
  m              mark/unmark completed (asks for confirmation)
  x              hide completed machines
  i / Enter      machine details
  s              cycle the sort order

[ Progress ]
  Enter          open the selected writeup in the browser

[ Anywhere ]
  a              username
  l              switch language (EN/ES)
  r              refresh data
  h              this help
  q / Ctrl-C     quit

[ Popups ]
  Tab / ↓ / ↑    next / previous field
  Space          open a picker (type, language)
  Enter          confirm
  Esc            cancel";

const HELP_TEXT_ES: &str = "[ Navegación ]
  Tab / →        siguiente pestaña
  ← / Shift-Tab  pestaña anterior
  j / ↓          bajar
  k / ↑          subir
  g / Home       ir a la primera fila
  /              filtro (Enter confirma · Esc limpia y sale)

[ Máquinas ]
  d              resolver el CAPTCHA y descargar el archivo
  D              carpeta de descargas (ruta absoluta)
  f              enviar flags first-blood
  w              writeups de la comunidad
  u              enviar un writeup
  b              solo máquinas con un hueco libre
  m              marcar/desmarcar completada (pide confirmación)
  x              ocultar máquinas completadas
  i / Enter      detalles de la máquina
  s              cambiar el orden

[ Progreso ]
  Enter          abrir el writeup seleccionado en el navegador

[ En cualquier lugar ]
  a              usuario
  l              cambiar idioma (EN/ES)
  r              refrescar datos
  h              esta ayuda
  q / Ctrl-C     salir

[ Popups ]
  Tab / ↓ / ↑    siguiente / anterior campo
  Space          abrir un selector (tipo, idioma)
  Enter          confirmar
  Esc            cancelar";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_toggle_and_codes() {
        assert_eq!(Lang::En.toggle(), Lang::Es);
        assert_eq!(Lang::Es.toggle(), Lang::En);
        assert_eq!(Lang::from_code("es"), Lang::Es);
        assert_eq!(Lang::from_code("ES"), Lang::Es);
        assert_eq!(Lang::from_code("fr"), Lang::En);
        assert_eq!(Lang::from_code(""), Lang::En);
        assert_eq!(Lang::default(), Lang::En);
    }

    #[test]
    fn messages_render_in_both_languages() {
        assert_eq!(Msg::Cancelled.render(Lang::En), "Cancelled.");
        assert_eq!(Msg::Cancelled.render(Lang::Es), "Cancelado.");
        assert_eq!(Msg::MachinesCount(5).render(Lang::En), "5 machines");
        assert_eq!(Msg::MachinesCount(5).render(Lang::Es), "5 máquinas");
        assert_eq!(
            Msg::MarkedCompleted("apex".into()).render(Lang::Es),
            "apex marcada como completada."
        );
        assert_eq!(
            Msg::LanguageSet(Lang::Es).render(Lang::Es),
            "Idioma: Español"
        );
        assert_eq!(
            Msg::LanguageSet(Lang::En).render(Lang::Es),
            "Idioma: English"
        );
    }

    #[test]
    fn help_text_covers_every_documented_key() {
        for key in [
            "Tab", "j", "k", "g", "d", "D", "f", "w", "u", "b", "m", "x", "i", "s", "h", "r", "a",
            "l", "q",
        ] {
            for lang in [Lang::En, Lang::Es] {
                let text = Msg::HelpText.render(lang);
                assert!(text.contains(key), "{key} missing from {lang:?} help");
            }
        }
    }

    #[test]
    fn footer_fits_a_ninety_column_terminal() {
        for lang in [Lang::En, Lang::Es] {
            for msg in [Msg::FooterMachines, Msg::FooterGlobal] {
                let line = msg.render(lang);
                assert!(
                    line.chars().count() <= 90,
                    "{line} is {} chars",
                    line.chars().count()
                );
            }
        }
    }

    #[test]
    fn done_confirmation_wording_differs_per_direction() {
        let marking = Msg::MarkDoneConfirm("APex".into()).render(Lang::En);
        let unmarking = Msg::UnmarkDoneConfirm("APex".into()).render(Lang::En);
        assert!(marking.contains("Mark APex as completed"));
        assert!(unmarking.contains("already marked"));
        assert_ne!(marking, unmarking);
    }

    #[test]
    fn fmt_bytes_picks_units() {
        assert_eq!(fmt_bytes(0), "0 B");
        assert_eq!(fmt_bytes(512), "512 B");
        assert_eq!(fmt_bytes(1024), "1.0 KB");
        assert_eq!(fmt_bytes(1_966_080), "1.9 MB");
        assert_eq!(fmt_bytes(2_147_483_648), "2.0 GB");
    }

    #[test]
    fn download_progress_omits_total_when_unknown() {
        assert_eq!(
            Msg::DownloadProgress("APex".into(), "APex.ova".into(), 2048, 0, 0).render(Lang::En),
            "↓ APex · APex.ova · 2.0 KB"
        );
        assert_eq!(
            Msg::DownloadProgress("APex".into(), "APex.ova".into(), 1024, 4096, 512)
                .render(Lang::En),
            "↓ APex · APex.ova · 1.0 KB/4.0 KB · 512 B/s"
        );
    }
}
