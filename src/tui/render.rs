//! Pure state -> widget rendering for the dashboard. Nord theme, English UI.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Row, Table, TableState, Tabs, Wrap};
use ratatui::Frame;

use super::{ActionReport, AppState, InputMode, Popup, PopupKind, ReportKind, Tab, WriteupsPopup};

const ACCENT: Color = Color::Rgb(136, 192, 208); // Nord8 frost blue
const WARN: Color = Color::Rgb(235, 203, 139); // Nord13 yellow
const OK: Color = Color::Rgb(163, 190, 140); // Nord14 green
const BAD: Color = Color::Rgb(191, 97, 106); // Nord11 red
const FROST: Color = Color::Rgb(143, 188, 187); // Nord7 teal
const PURPLE: Color = Color::Rgb(180, 142, 173); // Nord15
const BRIGHT: Color = Color::Rgb(236, 239, 244); // Nord6
const LINK: Color = Color::Rgb(94, 129, 172); // Nord10
const HL_BG: Color = Color::Rgb(59, 66, 82); // Nord1

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
        draw_popup(frame, frame.area(), popup);
    }
    if let Some(report) = &app.report {
        draw_report(frame, frame.area(), report);
    }
    if app.writeups_popup.is_some() {
        if let Some(popup) = app.writeups_popup.clone() {
            draw_writeups_popup(frame, frame.area(), &popup);
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
            format!("{} machines", app.data.machines.len()),
            Style::new().fg(FROST),
        ),
        Span::raw("  ·  "),
        Span::styled(
            format!("first blood: user {fb_user} · root {fb_root}"),
            Style::new().fg(OK),
        ),
        Span::raw("  ·  "),
        match position {
            Some((rank, _)) => Span::styled(format!("leaderboard #{rank}"), Style::new().fg(WARN)),
            None => Span::styled("leaderboard -", Style::new().dim()),
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
    let header = Row::new([
        "Machine",
        "Difficulty",
        "OS",
        "Creator",
        "Date",
        "First Blood",
    ])
    .style(Style::new().fg(ACCENT).bold());

    let rows: Vec<Row> = visible
        .iter()
        .map(|m| {
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
                Span::styled(m.name.clone(), Style::new().fg(BRIGHT).bold()),
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
            let fb_note = if app.only_first_blood {
                " · first blood only".to_string()
            } else if open > 0 {
                format!(" · first blood open: {open}")
            } else {
                String::new()
            };
            format!(
                " Machines {}/{}{}{} ",
                app.visible_machines().len(),
                app.data.machines.len(),
                app.machine_sort.indicator(),
                fb_note
            )
        }
        Tab::Progress => format!(
            " First bloods {} · writeups {} ",
            app.data
                .first_bloods_of(&crate::config::ConfigManager::new().username())
                .len(),
            app.own_writeups_rows().len()
        ),
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
    let header = Row::new(["Machine", "Type", "Date", "URL"]).style(Style::new().fg(ACCENT).bold());
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

fn draw_popup(frame: &mut Frame, area: Rect, popup: &Popup) {
    // Description popup: read-only, wrapped details.
    if popup.kind == PopupKind::Descripcion {
        let width = area.width.saturating_sub(8).max(40);
        let box_area = popup_area(area, width, 16);
        frame.render_widget(Clear, box_area);
        let mut lines = Vec::new();
        if let Some(text) = &popup.text {
            for line in text.split('\n') {
                lines.push(Line::from(Span::raw(line.to_string())));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Esc close", Style::new().dim())));
        let block = Block::bordered()
            .title(Span::styled(
                format!(" {} ", popup.machine),
                Style::new().fg(ACCENT).bold(),
            ))
            .border_style(Style::new().fg(ACCENT));
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            box_area,
        );
        return;
    }

    let height = match popup.kind {
        PopupKind::Username => 8,
        PopupKind::WriteupSubmit => 11,
        PopupKind::Flag => 11,
        _ => 8,
    };
    let height = height + u16::from(popup.notice.is_some());
    // Clamp popup height to the available body area
    let height = height.min(area.height.saturating_sub(4));
    let box_area = popup_area(area, 76, height);
    frame.render_widget(Clear, box_area);

    let (title, prompts, hint): (String, Vec<&str>, &str) = match popup.kind {
        PopupKind::Flag => (
            format!(" First blood — {} ", popup.machine),
            vec!["User flag (MD5):", "Root flag (MD5):"],
            "Enter submit · ↑↓/Tab switch field · Esc cancel",
        ),
        PopupKind::WriteupSubmit => (
            format!(" Submit writeup — {} ", popup.machine),
            vec!["URL:", "Type (Text/Video):", "Language (en/es/..):"],
            "Enter submit · ↑↓/Tab switch field · Esc cancel",
        ),
        PopupKind::Username => (
            " Your username ".to_string(),
            vec!["Username:"],
            "Enter save · Esc cancel",
        ),
        PopupKind::Descripcion => unreachable!("rendered by the description branch above"),
    };

    let mut lines = Vec::new();
    if let Some(notice) = &popup.notice {
        lines.push(Line::from(Span::styled(
            format!("⚠ {notice}"),
            Style::new().fg(WARN).bold(),
        )));
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
        lines.push(Line::from(Span::styled(
            format!("{prompt} {raw}{marker}"),
            style,
        )));
        if index + 1 < prompts.len() {
            lines.push(Line::from(""));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(hint, Style::new().dim())));

    let block = Block::bordered()
        .title(Span::styled(title, Style::new().fg(WARN).bold()))
        .border_style(Style::new().fg(WARN));

    frame.render_widget(Paragraph::new(lines).block(block), box_area);
}

fn draw_report(frame: &mut Frame, area: Rect, report: &ActionReport) {
    let height = (report.entries.len() as u16 * 2 + 4).clamp(5, 14);
    let width = 66;
    let box_area = popup_area(area, width, height);
    frame.render_widget(Clear, box_area);

    let mut lines = vec![Line::from("")];
    for (kind, text) in &report.entries {
        let span = match kind {
            ReportKind::Success => Span::styled(text.clone(), Style::new().fg(OK).bold()),
            ReportKind::Failure => Span::styled(text.clone(), Style::new().fg(BAD).bold()),
            ReportKind::Info => Span::styled(text.clone(), Style::new().fg(WARN)),
        };
        lines.push(Line::from(format!("  {span}")));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(
        "Enter / Esc close",
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

fn draw_writeups_popup(frame: &mut Frame, area: Rect, popup: &WriteupsPopup) {
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

    let header = Row::new(["Author", "Type", "Date", "URL"]).style(Style::new().fg(ACCENT).bold());
    let hint =
        Row::new([" ", " ", " ", "Enter open · jk select · Esc close"]).style(Style::new().dim());

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
            Some(PopupKind::Flag) => "Enter submit · ↑↓/Tab switch field · Esc cancel".to_string(),
            Some(PopupKind::WriteupSubmit) => {
                "Enter submit · ↑↓/Tab switch field · Esc cancel".to_string()
            }
            Some(PopupKind::Username) => "Enter save · Esc cancel".to_string(),
            Some(PopupKind::Descripcion) => "Esc close".to_string(),
            _ => "Enter confirm · Esc cancel".to_string(),
        }
    } else {
        match app.input_mode {
            InputMode::Filter => "Enter confirm · Esc clears & exits".to_string(),
            InputMode::Normal => match app.tab {
                Tab::Machines => "jk move · / filter · s sort · d download · f flag · w writeups · u submit · b first blood · i info".to_string(),
                Tab::Progress => "jk move · Enter open writeup".to_string(),
            },
        }
    };
    frame.render_widget(
        Paragraph::new(Span::styled(actions_line, Style::new().dim())),
        actions,
    );

    let global = "Tab tabs · a username · r refresh · q quit";
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
