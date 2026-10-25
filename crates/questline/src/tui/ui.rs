//! Drawing, k9s style: a header (info, key hints, logo), an optional prompt, the resource
//! table, breadcrumbs and a flash line.

use std::time::Duration;

use questline_core::Status;
use questline_core::questkit::Counts;
use questline_core::questkit::ExerciseStatus;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Flex;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Cell;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Row;
use ratatui::widgets::Table;

use super::app::App;
use super::app::Badge;
use super::app::Item;
use super::app::Mode;
use super::app::OverlayKind;
use super::app::SortKey;
use super::app::View;
use super::app::age;

const ORANGE: Color = Color::Rgb(255, 165, 0);
const BORDER: Color = Color::Rgb(30, 144, 255);
const FLASH_FOR: Duration = Duration::from_secs(8);
const BAR_WIDTH: usize = 10;

const LOGO: [&str; 4] = [
    r" ___  _   _ ___ ___ _____ _    ___ _  _ ___ ",
    r"/ _ \| | | | __/ __|_   _| |  |_ _| \| | __|",
    r"| (_) | |_| | _|\__ \ | | | |__ | || .` | _| ",
    r" \__\_\\___/|___|___/ |_| |____|___|_|\_|___|",
];

pub const HELP: &[(&str, &str)] = &[
    ("enter", "drill down: quest → modules → lessons → exercises"),
    ("esc / q", "back (or clear the filter)"),
    ("j k ↑ ↓", "move; g / G top / bottom; ctrl-f / ctrl-b page"),
    (
        "c",
        "check the selection now: run its exercise tests and record progress",
    ),
    (
        "C",
        "check everything in this view whose files changed (cached results are reused)",
    ),
    (
        "e",
        "open the selection in your IDE as a new window ($QUESTLINE_IDE)",
    ),
    (
        "r",
        "run the selected lesson (cargo run -- <n>) in the terminal",
    ),
    ("s", "shell in the selection's directory; exit to come back"),
    ("d", "describe: metadata, progress, files, problems"),
    ("l", "logs of the latest check (live while it runs)"),
    ("y", "the manifest (quest.toml / module.toml)"),
    ("0", "modules of every quest"),
    (
        "N P S A",
        "sort by name, progress, status, age (again: reverse)",
    ),
    ("/", "filter rows"),
    (
        ":",
        "command: quests, modules, <quest>, <quest>/<module>, check [target], reload, q",
    ),
    (
        "ctrl-r",
        "reload: rescan quests/ and pick up checks run elsewhere",
    ),
    ("ctrl-c / :q", "quit"),
];

pub fn draw(frame: &mut Frame, app: &mut App) {
    let prompt = if app.mode == Mode::Normal { 0 } else { 3 };
    let [header, prompt_area, table, crumbs, flash] = Layout::vertical([
        Constraint::Length(6),
        Constraint::Length(prompt),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    draw_header(frame, app, header);
    if prompt > 0 {
        draw_prompt(frame, app, prompt_area);
    }
    draw_table(frame, app, table);
    draw_crumbs(frame, app, crumbs);
    draw_flash(frame, app, flash);
    if app.overlay.is_some() {
        draw_overlay(frame, app);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    // Narrow terminals lose the logo before the info column gets squeezed.
    let logo_width = LOGO[0].len() as u16 + 1;
    let logo_width = if area.width >= 40 + 58 + logo_width {
        logo_width
    } else {
        0
    };
    let [info, keys, logo] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(58),
        Constraint::Length(logo_width),
    ])
    .spacing(1)
    .areas(area);
    let s = &app.snapshot;
    let total = s.counts();
    let checking = match (&app.checking, app.queued.len()) {
        (Some(k), 0) => k.to_string(),
        (Some(k), n) => format!("{k} (+{n} queued)"),
        (None, 0) => "-".into(),
        (None, n) => format!("{n} queued"),
    };
    let home = std::env::var("HOME").unwrap_or_default();
    let root = s.root.display().to_string();
    let root = match root.strip_prefix(&home) {
        Some(rest) if !home.is_empty() => format!("~{rest}"),
        _ => root,
    };
    let rows = [
        ("Root", root),
        ("Quests", s.quests.len().to_string()),
        ("Modules", s.modules().count().to_string()),
        (
            "Progress",
            format!("{} {}/{}", bar(total), total.passed, total.total()),
        ),
        ("Checking", checking),
        ("Version", env!("CARGO_PKG_VERSION").to_string()),
    ];
    let info_lines: Vec<Line> = rows
        .into_iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(format!("{k:<10}"), Style::new().fg(ORANGE).bold()),
                Span::raw(v),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(info_lines), info);

    let hints = key_hints(app.view());
    let half = hints.len().div_ceil(2);
    let key_lines: Vec<Line> = (0..half)
        .map(|i| {
            let mut spans = Vec::new();
            for (k, d) in [hints.get(i), hints.get(i + half)].into_iter().flatten() {
                spans.push(Span::styled(
                    format!("{:<9}", format!("<{k}>")),
                    Style::new().fg(BORDER).bold(),
                ));
                spans.push(Span::styled(
                    format!("{d:<20}"),
                    Style::new().fg(Color::Gray),
                ));
            }
            Line::from(spans)
        })
        .collect();
    frame.render_widget(Paragraph::new(key_lines), keys);

    let logo_lines: Vec<Line> = LOGO
        .iter()
        .map(|l| Line::styled(*l, Style::new().fg(ORANGE).bold()))
        .collect();
    frame.render_widget(Paragraph::new(logo_lines), logo);
}

fn key_hints(view: &View) -> Vec<(&'static str, &'static str)> {
    let mut hints = vec![
        ("enter", "Drill down"),
        ("c", "Check"),
        ("C", "Check changed"),
        ("e", "Open IDE"),
        ("d", "Describe"),
    ];
    match view {
        View::Lessons(_) | View::Exercises(..) => hints.push(("r", "Run lesson")),
        _ => hints.push(("0", "All modules")),
    }
    hints.extend([
        ("l", "Logs"),
        ("s", "Shell"),
        ("y", "Manifest"),
        ("/", "Filter"),
        (":", "Command"),
        ("?", "Help"),
    ]);
    hints
}

fn draw_prompt(frame: &mut Frame, app: &App, area: Rect) {
    let (icon, color) = if app.mode == Mode::Filter {
        ("/", Color::LightBlue)
    } else {
        (":", ORANGE)
    };
    let line = Line::from(vec![
        Span::styled(format!(" {icon}"), Style::new().fg(color).bold()),
        Span::raw(&app.input),
        Span::raw("▏"),
    ]);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(color));
    frame.render_widget(Paragraph::new(line).block(block), area);
}

fn draw_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let items = app.items();
    let view = app.view().clone();
    let (header, widths): (Vec<&str>, Vec<Constraint>) = match &view {
        View::Quests => (
            vec![
                "NAME", "TITLE", "MODULES", "PROGRESS", "DONE", "STATUS", "AGE",
            ],
            vec![
                len(20),
                Constraint::Fill(1),
                len(7),
                len(16),
                len(9),
                len(12),
                len(4),
            ],
        ),
        View::Modules(None) => (
            vec![
                "QUEST", "NAME", "TITLE", "LESSONS", "PROGRESS", "DONE", "STATUS", "AGE",
            ],
            vec![
                len(16),
                len(26),
                Constraint::Fill(1),
                len(7),
                len(16),
                len(9),
                len(12),
                len(4),
            ],
        ),
        View::Modules(Some(_)) => (
            vec![
                "NAME", "TITLE", "LESSONS", "PROGRESS", "DONE", "STATUS", "AGE",
            ],
            vec![
                len(26),
                Constraint::Fill(1),
                len(7),
                len(16),
                len(9),
                len(12),
                len(4),
            ],
        ),
        View::Lessons(_) => (
            vec!["#", "NAME", "SUMMARY", "PROGRESS", "DONE", "STATUS"],
            vec![
                len(3),
                len(30),
                Constraint::Fill(1),
                len(16),
                len(7),
                len(12),
            ],
        ),
        View::Exercises(..) => (
            vec!["NAME", "STATUS", "MESSAGE"],
            vec![len(44), len(9), Constraint::Fill(1)],
        ),
    };
    let rows: Vec<Row> = items
        .iter()
        .map(|item| {
            let badge = app.badge(item);
            let cells = cells(item, badge, &view);
            Row::new(cells.into_iter().map(Cell::from)).style(Style::new().fg(badge_color(badge)))
        })
        .collect();

    let (resource, scope) = match &view {
        View::Quests => ("Quests", "all".to_string()),
        View::Modules(q) => ("Modules", q.clone().unwrap_or_else(|| "all".into())),
        View::Lessons(k) => ("Lessons", k.to_string()),
        View::Exercises(k, n) => ("Exercises", format!("{k}#{n}")),
    };
    let mut title = vec![
        Span::styled(format!(" {resource}"), Style::new().fg(Color::Cyan).bold()),
        Span::styled(format!("({scope})"), Style::new().fg(Color::Magenta).bold()),
        Span::styled(
            format!("[{}] ", items.len()),
            Style::new().fg(Color::White).bold(),
        ),
    ];
    if !app.filter.is_empty() {
        title.push(Span::styled(
            format!("</{}> ", app.filter),
            Style::new().fg(Color::LightBlue),
        ));
    }
    if let Some((key, desc)) = app.sort {
        let name = match key {
            SortKey::Name => "name",
            SortKey::Progress => "progress",
            SortKey::Status => "status",
            SortKey::Age => "age",
        };
        title.push(Span::styled(
            format!("<sort:{name}{}> ", if desc { "↓" } else { "↑" }),
            Style::new().fg(Color::DarkGray),
        ));
    }
    let empty_hint = rows.is_empty();
    let block = Block::bordered()
        .border_style(Style::new().fg(BORDER))
        .title(Line::from(title).centered());
    let table = Table::new(rows, widths)
        .header(Row::new(header).style(Style::new().fg(Color::White).bold()))
        .block(block)
        .column_spacing(2)
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    drop(items);
    frame.render_stateful_widget(table, area, &mut app.table);

    if empty_hint {
        let hint = match &view {
            View::Exercises(..) => {
                "No results for this lesson yet: press esc, then c to check the module."
            }
            _ if !app.filter.is_empty() => "Nothing matches the filter (esc clears it).",
            _ => "No quests yet: generate one with the /learn skill (see skills/learn).",
        };
        let inner = area.inner(ratatui::layout::Margin::new(2, 2));
        frame.render_widget(Paragraph::new(hint).fg(Color::DarkGray), inner);
    }
}

fn len(n: u16) -> Constraint {
    Constraint::Length(n)
}

fn cells(item: &Item, badge: Badge, view: &View) -> Vec<String> {
    let done = |c: Counts| format!("{}/{}", c.passed, c.total());
    let status = badge.label().to_string();
    match item {
        Item::Quest(q) => vec![
            q.id.clone(),
            q.title().to_string(),
            q.modules.len().to_string(),
            bar(q.counts()),
            done(q.counts()),
            status,
            age(item.evaluated_at()),
        ],
        Item::Module(m) => {
            let lessons = m
                .manifest
                .as_ref()
                .map_or(0, |mm| mm.lessons.len())
                .to_string();
            let mut row = vec![
                m.key.module.clone(),
                m.title().to_string(),
                lessons,
                bar(m.counts()),
                done(m.counts()),
                status,
                age(item.evaluated_at()),
            ];
            if *view == View::Modules(None) {
                row.insert(0, m.key.quest.clone());
            }
            row
        }
        Item::Lesson(m, l) => {
            let c = m.lesson_counts(l.number);
            vec![
                l.number.to_string(),
                l.name.clone(),
                l.summary.clone(),
                bar(c),
                done(c),
                status,
            ]
        }
        Item::Exercise(_, e) => {
            let message = e
                .message
                .as_deref()
                .and_then(|m| m.lines().next())
                .unwrap_or_default()
                .to_string();
            vec![e.short_name().to_string(), status, message]
        }
    }
}

/// `███████░░░  70%`.
fn bar(c: Counts) -> String {
    if c.total() == 0 {
        return format!("{}    -", "░".repeat(BAR_WIDTH));
    }
    let filled = ((c.ratio() * BAR_WIDTH as f64).round() as usize).min(BAR_WIDTH);
    format!(
        "{}{} {:>3.0}%",
        "█".repeat(filled),
        "░".repeat(BAR_WIDTH - filled),
        c.ratio() * 100.0
    )
}

fn badge_color(badge: Badge) -> Color {
    match badge {
        Badge::Checking => ORANGE,
        Badge::Queued => Color::Magenta,
        Badge::Status(Status::Completed) | Badge::Exercise(ExerciseStatus::Passed) => Color::Green,
        Badge::Status(Status::InProgress) => Color::Cyan,
        Badge::Status(Status::NotStarted) | Badge::Exercise(ExerciseStatus::Todo) => Color::Gray,
        Badge::Status(Status::Pending) | Badge::Exercise(ExerciseStatus::Ignored) => {
            Color::DarkGray
        }
        Badge::Status(Status::Stale) => Color::Yellow,
        Badge::Status(Status::BuildFailed | Status::Invalid)
        | Badge::Exercise(ExerciseStatus::Failed) => Color::Red,
    }
}

fn draw_crumbs(frame: &mut Frame, app: &App, area: Rect) {
    let last = app.stack.len() - 1;
    let spans: Vec<Span> = app
        .stack
        .iter()
        .enumerate()
        .flat_map(|(i, v)| {
            let bg = if i == last { ORANGE } else { BORDER };
            [
                Span::styled(
                    format!(" <{}> ", v.crumb()),
                    Style::new().bg(bg).fg(Color::Black).bold(),
                ),
                Span::raw(" "),
            ]
        })
        .collect();
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_flash(frame: &mut Frame, app: &App, area: Rect) {
    let Some(flash) = app.flash.as_ref().filter(|f| f.at.elapsed() < FLASH_FOR) else {
        return;
    };
    let style = if flash.error {
        Style::new().fg(Color::Red).bold()
    } else {
        Style::new().fg(Color::LightBlue)
    };
    frame.render_widget(
        Paragraph::new(Line::styled(format!(" {}", flash.text), style)),
        area,
    );
}

fn draw_overlay(frame: &mut Frame, app: &mut App) {
    let area = centered(frame.area(), 90, 85);
    let lines: Vec<Line> = match &app.overlay.as_ref().expect("checked by caller").kind {
        OverlayKind::Help => HELP
            .iter()
            .map(|(k, d)| {
                Line::from(vec![
                    Span::styled(format!("  {k:<14}"), Style::new().fg(BORDER).bold()),
                    Span::raw(*d),
                ])
            })
            .collect(),
        OverlayKind::Text(text) => text.iter().map(|l| describe_line(l)).collect(),
        OverlayKind::Log(key) => app.log_lines(key).into_iter().map(Line::raw).collect(),
    };
    let overlay = app.overlay.as_mut().expect("checked by caller");
    let visible = area.height.saturating_sub(2);
    let max_scroll = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(visible);
    overlay.scroll = overlay.scroll.min(max_scroll);
    let title = Line::from(vec![
        Span::styled(
            format!(" {} ", overlay.title),
            Style::new().fg(Color::Cyan).bold(),
        ),
        Span::styled(
            "<esc> close  <j/k> scroll ",
            Style::new().fg(Color::DarkGray),
        ),
    ]);
    let block = Block::bordered()
        .border_style(Style::new().fg(BORDER))
        .title(title.centered());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .scroll((overlay.scroll, 0)),
        area,
    );
}

/// `Key:        value` lines get a coloured key, like `kubectl describe`.
fn describe_line(line: &str) -> Line<'static> {
    match line.split_once(':') {
        Some((key, rest)) if !key.starts_with(' ') && key.len() < 13 && !key.is_empty() => {
            Line::from(vec![
                Span::styled(format!("{key}:"), Style::new().fg(ORANGE).bold()),
                Span::raw(rest.to_string()),
            ])
        }
        _ => Line::raw(line.to_string()),
    }
}

fn centered(area: Rect, width_pct: u16, height_pct: u16) -> Rect {
    let [area] = Layout::vertical([Constraint::Percentage(height_pct)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::horizontal([Constraint::Percentage(width_pct)])
        .flex(Flex::Center)
        .areas(area);
    area
}
