//! Pure state -> widget rendering for the dashboard. Nord theme, English UI.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Row, Table, TableState, Tabs};
use ratatui::Frame;

use super::{
    ActionReport, AppState, InputMode, Popup, PopupKind, ReportKind, Tab, WRITEUP_LANGS,
    WRITEUP_TYPES, WriteupsPopup,
};
use crate::i18n::{Lang, Msg};

const ACCENT: Color = Color::Rgb(136, 192, 208); // Nord8 frost blue
const WARN: Color = Color::Rgb(235, 203, 139); // Nord13 yellow
const OK: Color = Color::Rgb(163, 190, 140); // Nord14 green
const BAD: Color = Color::Rgb(191, 97, 106); // Nord11 red
const FROST: Color = Color::Rgb(143, 188, 187); // Nord7 teal
const PURPLE: Color = Color::Rgb(180, 142, 173); // Nord15
const BRIGHT: Color = Color::Rgb(236, 239, 244); // Nord6
const LINK: Color = Color::Rgb(94, 129, 172); // Nord10
const HL_BG: Color = Color::Rgb(59, 66, 82); // Nord1

/// Renders a message in `lang` (owned `String`, no lifetime tricks).
fn render_msg(lang: Lang, msg: Msg) -> String {
    msg.render(lang)
}

pub fn draw(frame: &mut Frame, app: &mut AppState) {
    let [header, tabs, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    draw_header(frame, header, app);
    draw_tabs(frame, tabs, app);

    match app.tab {
        Tab::Machines => draw_machines(frame, body, app),
        Tab::Progress => draw_progress(frame, body, app),
    }

    draw_footer(frame, footer, app);

    if let Some(popup) = &app.popup {
        draw_popup(frame, frame.area(), popup, app.lang);
    }
    if let Some(report) = &app.report {
        draw_report(frame, frame.area(), report, app.lang);
    }
    if app.writeups_popup.is_some() {
        if let Some(popup) = app.writeups_popup.clone() {
            draw_writeups_popup(frame, frame.area(), &popup, app.lang);
        }
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &AppState) {
    let username = crate::config::ConfigManager::new().username();
    let username = if username.is_empty() { "-" } else { &username };
    let fb = app
        .data
        .first_bloods_of(&crate::config::ConfigManager::new().username());
    let fb_user = fb.iter().filter(|(_, k)| *k == "user").count();
    let fb_root = fb.iter().filter(|(_, k)| *k == "root").count();
    let position = app.data.leaderboard_position(username);
    let line = Line::from(vec![
        Span::styled(" VulNyx", Style::new().fg(ACCENT).bold()),
        Span::styled(" dashboard", Style::new().dim()),
        Span::raw("  ·  "),
        Span::styled(username, Style::new().fg(BRIGHT).bold()),
        Span::raw("  ·  "),
        Span::styled(
            app.tr(Msg::MachinesCount(app.data.machines.len())),
            Style::new().fg(FROST),
        ),
        Span::raw("  ·  "),
        Span::styled(
            app.tr(Msg::HeaderFirstBlood(fb_user, fb_root)),
            Style::new().fg(OK),
        ),
        Span::raw("  ·  "),
        match position {
            Some((rank, _)) => Span::styled(app.tr(Msg::Leaderboard(rank)), Style::new().fg(WARN)),
            None => Span::styled(app.tr(Msg::LeaderboardNone), Style::new().dim()),
        },
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_tabs(frame: &mut Frame, area: Rect, app: &AppState) {
    let titles: Vec<&str> = Tab::ALL.iter().map(|t| t.title()).collect();
    let index = Tab::ALL.iter().position(|t| *t == app.tab).unwrap_or(0);
    let tabs = Tabs::new(titles)
        .select(index)
        // Dim the inactive tabs so the highlighted one stands out.
        .style(Style::new().dim())
        .highlight_style(Style::new().fg(ACCENT).bold().underlined());
    frame.render_widget(tabs, area);
}

fn draw_machines(frame: &mut Frame, area: Rect, app: &mut AppState) {
    let visible = app.visible_machines();
    let auto_done = app.completed_auto();
    let header = Row::new([
        app.tr(Msg::ColMachine),
        app.tr(Msg::ColDifficulty),
        app.tr(Msg::ColOs),
        app.tr(Msg::ColCreator),
        app.tr(Msg::ColDate),
        app.tr(Msg::ColFirstBlood),
    ])
    .style(Style::new().fg(ACCENT).bold());

    let rows: Vec<Row> = visible
        .iter()
        .map(|m| {
            let done = app.is_completed_with(&m.slug, &auto_done);
            let difficulty = m.difficulty.to_uppercase();
            let diff_span = match difficulty.as_str() {
                "LOW" => Span::styled(difficulty, Style::new().fg(FROST)),
                "EASY" => Span::styled(difficulty, Style::new().fg(OK)),
                "MEDIUM" => Span::styled(difficulty, Style::new().fg(WARN)),
                "HARD" => Span::styled(difficulty, Style::new().fg(BAD)),
                _ => Span::raw(difficulty),
            };
            let os_span = if m.os == "Windows" {
                Span::styled(m.os.clone(), Style::new().fg(FROST))
            } else {
                Span::styled(m.os.clone(), Style::new().fg(WARN))
            };
            let fb = match (m.user_slot_open(), m.root_slot_open()) {
                (true, true) => Span::styled("user+root", Style::new().fg(OK).bold()),
                (true, false) => Span::styled("user", Style::new().fg(OK)),
                (false, true) => Span::styled("root", Style::new().fg(OK)),
                (false, false) => Span::styled("taken", Style::new().dim()),
            };
            Row::new([
                Span::styled(
                    if done { format!("✓ {}", m.name) } else { m.name.clone() },
                    if done {
                        Style::new().fg(OK).bold()
                    } else {
                        Style::new().fg(BRIGHT).bold()
                    },
                ),
                diff_span,
                os_span,
                Span::styled(m.creator.clone(), Style::new().fg(PURPLE)),
                Span::styled(m.release_date.clone(), Style::new().dim()),
                fb,
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(19),
            Constraint::Length(11),
            Constraint::Length(9),
            Constraint::Length(21),
            Constraint::Length(11),
            Constraint::Length(11),
            Constraint::Fill(1),
        ],
    )
    .header(header)
    .row_highlight_style(Style::new().bg(HL_BG).add_modifier(Modifier::BOLD))
    .block(filter_block(app));

    let mut state = TableState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(table, area, &mut state);

    app.set_visible_rows(visible_rows_in(area.height));
}

fn filter_block(app: &AppState) -> Block<'_> {
    let count_line = match app.tab {
        Tab::Machines => {
            let open = app
                .data
                .machines
                .iter()
                .filter(|m| m.any_slot_open())
                .count();
            let auto_done = app.completed_auto();
            let done_count = app
                .data
                .machines
                .iter()
                .filter(|m| app.is_completed_with(&m.slug, &auto_done))
                .count();
            let done_note = if app.hide_completed {
                app.tr(Msg::DoneNote(done_count, true))
            } else {
                app.tr(Msg::DoneNote(done_count, false))
            };
            let fb_note = if app.only_first_blood {
                app.tr(Msg::FbOnly)
            } else if open > 0 {
                app.tr(Msg::FbOpen(open))
            } else {
                String::new()
            };
            format!(
                "{}{}{}{}{}",
                app.tr(Msg::MachinesTitle(app.visible_machines().len(), app.data.machines.len())),
                app.machine_sort.indicator(app.lang),
                done_note,
                fb_note,
                " "
            )
        }
        Tab::Progress => app.tr(Msg::ProgressTitle(
            app.data
                .first_bloods_of(&crate::config::ConfigManager::new().username())
                .len(),
            app.own_writeups_rows().len(),
        )),
    };

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().dim());

    if app.input_mode == InputMode::Filter {
        block = block
            .title(Span::styled(
                format!(" filter: {}▏", app.filter),
                Style::new().fg(WARN).bold(),
            ))
            .title_position(ratatui::widgets::block::Position::Top)
            .border_style(Style::new().fg(WARN));
    } else if !app.filter.is_empty() {
        block = block.title(Span::styled(
            format!(" filter: {} ", app.filter),
            Style::new().fg(WARN),
        ));
    }

    block = block.title_bottom(Span::styled(count_line, Style::new().dim()));
    block
}

fn draw_progress(frame: &mut Frame, area: Rect, app: &mut AppState) {
    let username = crate::config::ConfigManager::new().username();
    let username = if username.is_empty() { "-" } else { &username };
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Fill(1)]).areas(area);

    // ---- left: identity + first bloods + statistics ---------------------
    let first_bloods = app.data.first_bloods_of(username);
    let (position, points) = app
        .data
        .leaderboard_position(username)
        .map(|(p, pts)| (p.to_string(), pts))
        .unzip();
    let position = position.unwrap_or_else(|| "-".to_string());
    let points = points.unwrap_or(0);

    let mut lines = vec![
        Line::from(Span::styled("[ Identity ]", Style::new().fg(ACCENT).bold())),
        Line::from(format!("  Username: {username}")),
        Line::from("  (self-declared — attached to flags & writeups)"),
        Line::from(""),
        Line::from(Span::styled(
            "[ First Bloods ]",
            Style::new().fg(ACCENT).bold(),
        )),
    ];
    if first_bloods.is_empty() {
        lines.push(Line::from(
            "  None yet — first to submit a flag gets the blood.",
        ));
    } else {
        for (machine, kind) in &first_bloods {
            lines.push(Line::from(format!("  ● {} — {kind}", machine.name)));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[ Statistics ]",
        Style::new().fg(ACCENT).bold(),
    )));
    lines.push(Line::from(format!(
        "  First blood  : user {} · root {}",
        first_bloods.iter().filter(|(_, k)| *k == "user").count(),
        first_bloods.iter().filter(|(_, k)| *k == "root").count(),
    )));
    lines.push(Line::from(format!(
        "  Writeups     : {}",
        app.own_writeups_rows().len()
    )));
    lines.push(Line::from(format!(
        "  Leaderboard  : #{position} ({points} pts)"
    )));

    let left_area = Rect {
        x: left.x + 2,
        y: left.y,
        width: left.width.saturating_sub(4),
        height: left.height,
    };
    frame.render_widget(Paragraph::new(lines), left_area);

    // ---- right: your writeups ------------------------------------------
    let own = app.own_writeups_rows();
    let header = Row::new([
        app.tr(Msg::ColMachine),
        app.tr(Msg::ColType),
        app.tr(Msg::ColDate),
        app.tr(Msg::ColUrl),
    ])
    .style(Style::new().fg(ACCENT).bold());
    let rows: Vec<Row> = own
        .iter()
        .map(|(slug, w)| {
            let (tipo, tipo_style) = if w.tipo.contains("Video") {
                ("🎥", Style::new().fg(FROST))
            } else {
                ("📝", Style::new().fg(PURPLE))
            };
            let date = w.submitted_at.split(' ').next().unwrap_or("-").to_string();
            Row::new([
                Span::styled(slug.clone(), Style::new().fg(BRIGHT).bold()),
                Span::styled(tipo, tipo_style),
                Span::styled(date, Style::new().dim()),
                Span::styled(w.url.clone(), Style::new().fg(LINK)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(18),
            Constraint::Length(6),
            Constraint::Length(11),
            Constraint::Fill(1),
        ],
    )
    .header(header)
    .row_highlight_style(Style::new().bg(HL_BG).add_modifier(Modifier::BOLD))
    .block(
        Block::bordered()
            .title(Span::styled(
                format!(" Your writeups ({}) ", own.len()),
                Style::new().fg(ACCENT).bold(),
            ))
            .border_style(Style::new().dim()),
    );

    let mut state = TableState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(table, right, &mut state);

    app.set_visible_rows(visible_rows_in(right.height));
}

fn draw_popup(frame: &mut Frame, area: Rect, popup: &Popup, lang: Lang) {
    // Description popup: read-only, wrapped details, sized to its content.
    if popup.kind == PopupKind::Descripcion {
        let width = area.width.saturating_sub(8).max(40);
        let inner = usize::from(width).saturating_sub(2);
        let mut lines = Vec::new();
        if let Some(text) = &popup.text {
            for line in text.split('\n') {
                lines.extend(wrap_text(line, inner).into_iter().map(Line::from));
            }
        }
        let height = lines.len() as u16 + 4; // blank + "Esc close" + 2 borders
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(render_msg(lang, Msg::HintEscClose), Style::new().dim())));
        let box_area = popup_area(area, width, height);
        frame.render_widget(Clear, box_area);
        let block = Block::bordered()
            .title(Span::styled(
                format!(" {} ", popup.machine),
                Style::new().fg(ACCENT).bold(),
            ))
            .border_style(Style::new().fg(ACCENT));
        frame.render_widget(Paragraph::new(lines).block(block), box_area);
        return;
    }

    // Input popups: 76-wide box whose inner width shrinks on narrow
    // terminals so no line clips inside the border; height follows the
    // wrapped content (picker rows included).
    let inner = usize::from(area.width).saturating_sub(2).min(74).max(1);
    let (title, prompts, hint): (String, Vec<String>, String) = match popup.kind {
        PopupKind::Flag => (
            Msg::SubmitFlagTitle(popup.machine.clone()).render(lang),
            vec![Msg::PromptUserFlag.render(lang), Msg::PromptRootFlag.render(lang)],
            Msg::HintSubmit.render(lang),
        ),
        PopupKind::WriteupSubmit => (
            Msg::SubmitWriteupTitle(popup.machine.clone()).render(lang),
            vec![Msg::PromptUrl.render(lang), Msg::PromptType.render(lang), Msg::PromptLanguage.render(lang)],
            Msg::HintSubmitPickers.render(lang),
        ),
        PopupKind::Username => (
            Msg::UsernameTitle.render(lang),
            vec![Msg::PromptUsername.render(lang)],
            Msg::HintSave.render(lang),
        ),
        PopupKind::Descripcion => unreachable!("rendered by the description branch above"),
    };

    let mut lines = Vec::new();
    if let Some(notice) = &popup.notice {
        let notice = format!("⚠ {notice}");
        for chunk in wrap_text(&notice, inner) {
            lines.push(Line::from(Span::styled(
                chunk,
                Style::new().fg(WARN).bold(),
            )));
        }
        lines.push(Line::from(""));
    }
    for (index, prompt) in prompts.iter().enumerate() {
        let active = index == popup.field;
        let marker = if active { "▏" } else { "" };
        let raw = popup.buffers.get(index).map(String::as_str).unwrap_or("");
        let style = if active {
            Style::new().fg(BRIGHT).add_modifier(Modifier::BOLD)
        } else {
            Style::new().dim()
        };
        let text = format!("{prompt} {raw}{marker}");
        for chunk in wrap_text(&text, inner) {
            lines.push(Line::from(Span::styled(chunk, style)));
        }
        if index + 1 < prompts.len() {
            lines.push(Line::from(""));
        }
    }
    if popup.type_open {
        lines.push(Line::from(""));
        for chunk in wrap_text(&Msg::TypePanelHeader.render(lang), inner) {
            lines.push(Line::from(Span::styled(chunk, Style::new().dim())));
        }
        for (index, label) in WRITEUP_TYPES.iter().enumerate() {
            let mark = if popup.buffers.get(1).map(String::as_str) == Some(*label) {
                "✓"
            } else {
                " "
            };
            let mut style = Style::new();
            if index == popup.type_cursor {
                style = style.bg(HL_BG).add_modifier(Modifier::BOLD);
            }
            lines.push(Line::from(Span::styled(format!("{mark} {label}"), style)));
        }
    }
    if popup.lang_open {
        lines.push(Line::from(""));
        for chunk in wrap_text(&Msg::LangPanelHeader.render(lang), inner) {
            lines.push(Line::from(Span::styled(chunk, Style::new().dim())));
        }
        let visible = 10.min(WRITEUP_LANGS.len());
        let start = popup.lang_cursor.saturating_sub(visible - 1);
        for (index, (code, label)) in WRITEUP_LANGS
            .iter()
            .enumerate()
            .skip(start)
            .take(visible)
        {
            let mark = if popup.lang_selected.contains(code) {
                "✓"
            } else {
                " "
            };
            let mut style = Style::new();
            if index == popup.lang_cursor {
                style = style.bg(HL_BG).add_modifier(Modifier::BOLD);
            }
            lines.push(Line::from(Span::styled(
                format!("{mark} {label} ({code})"),
                style,
            )));
        }
    }
    lines.push(Line::from(""));
    for chunk in wrap_text(&hint, inner) {
        lines.push(Line::from(Span::styled(chunk, Style::new().dim())));
    }

    let height = lines.len() as u16 + 2; // borders
    let box_area = popup_area(area, 76, height);
    frame.render_widget(Clear, box_area);

    let block = Block::bordered()
        .title(Span::styled(title, Style::new().fg(WARN).bold()))
        .border_style(Style::new().fg(WARN));

    frame.render_widget(Paragraph::new(lines).block(block), box_area);
}

fn draw_report(frame: &mut Frame, area: Rect, report: &ActionReport, lang: Lang) {
    let (width, height) = report_box(&report.entries, (area.width, area.height));
    let box_area = popup_area(area, width, height);
    frame.render_widget(Clear, box_area);

    // `report_box` wraps entry text at inner - 2; the "  " indent is
    // prefixed per wrapped line so every row stays inside the box.
    let text_width = usize::from(width).saturating_sub(4);
    let mut lines = vec![Line::from("")];
    for (kind, text) in &report.entries {
        let style = match kind {
            ReportKind::Success => Style::new().fg(OK).bold(),
            ReportKind::Failure => Style::new().fg(BAD).bold(),
            ReportKind::Info => Style::new().fg(WARN),
        };
        for chunk in wrap_text(text, text_width) {
            lines.push(Line::from(Span::styled(format!("  {chunk}"), style)));
        }
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(
        render_msg(lang, Msg::HintEnterEscClose),
        Style::new().dim(),
    )));

    let block = Block::bordered()
        .title(Span::styled(
            report.title.clone(),
            Style::new().fg(ACCENT).bold(),
        ))
        .border_style(Style::new().fg(ACCENT));

    frame.render_widget(Paragraph::new(lines).block(block), box_area);
}

fn draw_writeups_popup(frame: &mut Frame, area: Rect, popup: &WriteupsPopup, lang: Lang) {
    let rows: Vec<Row> = popup
        .entries
        .iter()
        .map(|w| {
            let (tipo, tipo_style) = if w.tipo.contains("Video") {
                ("🎥", Style::new().fg(FROST))
            } else {
                ("📝", Style::new().fg(PURPLE))
            };
            let date = w.submitted_at.split(' ').next().unwrap_or("-").to_string();
            Row::new(vec![
                Span::styled(w.author.clone(), Style::new().fg(PURPLE)),
                Span::styled(tipo, tipo_style),
                Span::styled(date, Style::new().dim()),
                Span::styled(w.url.clone(), Style::new().fg(LINK)),
            ])
        })
        .collect();

    let height = (popup.entries.len() as u16 + 3).clamp(6, 18);
    // Nearly full terminal width so writeup links stay readable.
    let width = area.width.saturating_sub(4).max(60);
    let box_area = popup_area(area, width, height);
    frame.render_widget(Clear, box_area);

    let header = Row::new([
        render_msg(lang, Msg::ColAuthor),
        render_msg(lang, Msg::ColType),
        render_msg(lang, Msg::ColDate),
        render_msg(lang, Msg::ColUrl),
    ])
    .style(Style::new().fg(ACCENT).bold());
    let hint_text = render_msg(lang, Msg::HintWriteupsRow);
    let hint = Row::new(vec![" ", " ", " ", hint_text.as_str()]).style(Style::new().dim());

    let table = Table::new(
        rows,
        [
            Constraint::Length(18),
            Constraint::Length(6),
            Constraint::Length(11),
            Constraint::Fill(1),
        ],
    )
    .header(header)
    .footer(hint)
    .row_highlight_style(Style::new().bg(HL_BG).add_modifier(Modifier::BOLD))
    .block(
        Block::bordered()
            .title(Span::styled(
                format!(" Writeups — {} ", popup.machine),
                Style::new().fg(ACCENT).bold(),
            ))
            .border_style(Style::new().fg(ACCENT)),
    );

    let mut state = TableState::default().with_selected(Some(popup.selected));
    frame.render_stateful_widget(table, box_area, &mut state);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &AppState) {
    // Two rows so the key hints never get truncated: row 1 = context
    // actions, row 2 = global keys + status.
    let [actions, bottom] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
    let [global_area, status_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Min(24)]).areas(bottom);

    let actions_line: String = if app.popup.is_some() {
        match app.popup.as_ref().map(|p| p.kind) {
            Some(PopupKind::Flag) => app.tr(Msg::HintSubmit),
            Some(PopupKind::WriteupSubmit) => app.tr(Msg::HintSubmitPickers),
            Some(PopupKind::Username) => app.tr(Msg::HintSave),
            Some(PopupKind::Descripcion) => app.tr(Msg::HintEscClose),
            _ => app.tr(Msg::FooterPopupDefault),
        }
    } else {
        match app.input_mode {
            InputMode::Filter => app.tr(Msg::FooterFilter),
            InputMode::Normal => match app.tab {
                Tab::Machines => app.tr(Msg::FooterMachines),
                Tab::Progress => app.tr(Msg::FooterProgress),
            },
        }
    };
    frame.render_widget(
        Paragraph::new(Span::styled(actions_line, Style::new().dim())),
        actions,
    );

    let global = app.tr(Msg::FooterGlobal);
    frame.render_widget(
        Paragraph::new(Span::styled(global, Style::new().dim())),
        global_area,
    );

    let status = if let Some(label) = &app.fetching {
        Span::styled(format!("⟳ {label}"), Style::new().fg(WARN).bold())
    } else {
        match (&app.status, app.status_expiry) {
            (Some(message), Some(expiry)) if std::time::Instant::now() < expiry => {
                Span::styled(message.clone(), Style::new().fg(WARN))
            }
            _ => Span::raw(""),
        }
    };
    frame.render_widget(
        Paragraph::new(status).alignment(ratatui::layout::Alignment::Right),
        status_area,
    );
}

/// How many table rows fit in `height` (border 2 + header 1).
fn visible_rows_in(height: u16) -> usize {
    height.saturating_sub(3) as usize
}

/// Greedy word-wrap. Splits on spaces, accumulates words while
/// `current.len() + 1 + word.len() <= width` (char counts = display width;
/// ASCII-oriented: used for server/notice text, never language label rows).
/// Words longer than `width` are hard-broken at `width` chars;
/// `wrap_text("", _)` yields one empty line.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;
    for word in text.split(' ') {
        if word.is_empty() {
            if current_len > 0 && current_len < width {
                current.push(' ');
                current_len += 1;
            }
            continue;
        }
        let mut rest = word;
        while !rest.is_empty() {
            let rest_len = rest.chars().count();
            let sep = usize::from(current_len > 0);
            if current_len + sep + rest_len <= width {
                if sep == 1 {
                    current.push(' ');
                }
                current.push_str(rest);
                current_len += sep + rest_len;
                rest = "";
            } else if current_len == 0 {
                let head: String = rest.chars().take(width).collect();
                out.push(head.clone());
                rest = &rest[head.len()..];
            } else {
                out.push(std::mem::take(&mut current));
                current_len = 0;
            }
        }
    }
    out.push(current);
    out
}

/// Box size for the report popup: width follows the longest entry (inner
/// floored at 42, capped at 76), height fits every wrapped line. Shared with
/// the sizing tests; `draw_report` renders inside it.
fn report_box(entries: &[(ReportKind, String)], area: (u16, u16)) -> (u16, u16) {
    let inner = entries
        .iter()
        .map(|(_, text)| 2 + text.chars().count())
        .max()
        .unwrap_or(0)
        .clamp(42, 76);
    let mut lines = 2; // leading blank + hint
    for (_, text) in entries {
        lines += wrap_text(text, inner - 2).len() + 1; // entry + blank
    }
    let guide = Rect::new(0, 0, area.0, area.1);
    let box_area = popup_area(guide, inner as u16 + 2, lines as u16 + 2);
    (box_area.width, box_area.height)
}

fn popup_area(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_text_wraps_breaks_and_preserves_empty() {
        assert_eq!(wrap_text("aa bb cc", 5), vec!["aa bb", "cc"]);
        assert_eq!(wrap_text("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
        assert_eq!(wrap_text("", 10), vec![String::new()]);
    }

    #[test]
    fn report_box_caps_width_and_fits_area() {
        let long = vec![(ReportKind::Info, "a".repeat(200))];
        let (width, height) = report_box(&long, (80, 24));
        assert_eq!(width, 78); // inner capped at 76 + 2 borders
        // 200 chars wrapped at inner-2 -> 3 lines; blanks + hint + borders.
        assert_eq!(height, 2 + 3 + 1 + 2);

        let huge = vec![(ReportKind::Failure, "b".repeat(400))];
        let (width, height) = report_box(&huge, (80, 24));
        assert_eq!(width, 78);
        assert!(height <= 24, "report popup must fit an 80x24 area");

        // Narrow terminal: popup_area clamps both dimensions.
        let (width, height) = report_box(&huge, (30, 6));
        assert_eq!((width, height), (30, 6));
    }
}
