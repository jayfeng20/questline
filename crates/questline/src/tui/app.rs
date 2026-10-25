//! Dashboard state and key handling; no drawing (see `ui.rs`) and no terminal (see `mod.rs`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Instant;

use questline_core::EvalEvent;
use questline_core::Module;
use questline_core::ModuleKey;
use questline_core::Quest;
use questline_core::Questline;
use questline_core::SkipReason;
use questline_core::Snapshot;
use questline_core::Status;
use questline_core::Target;
use questline_core::questkit::ExerciseResult;
use questline_core::questkit::ExerciseStatus;
use questline_core::questkit::Lesson;
use questline_core::questkit::Progress;
use questline_core::questkit::manifest::MODULE_MANIFEST;
use questline_core::questkit::manifest::QUEST_MANIFEST;
use questline_core::questkit::progress::now_ms;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyModifiers;
use ratatui::widgets::TableState;

/// Live output kept per module while it is checked.
const MAX_LIVE_LINES: usize = 5_000;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Quests,
    /// `None`: modules of every quest.
    Modules(Option<String>),
    Lessons(ModuleKey),
    Exercises(ModuleKey, u8),
}

impl View {
    pub fn crumb(&self) -> String {
        match self {
            Self::Quests => "quests".into(),
            Self::Modules(None) => "modules".into(),
            Self::Modules(Some(q)) => format!("modules:{q}"),
            Self::Lessons(key) => format!("lessons:{}", key.module),
            Self::Exercises(_, n) => format!("exercises:{n}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Progress,
    Status,
    Age,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Filter,
    Command,
}

pub enum OverlayKind {
    Text(Vec<String>),
    /// Live output while the module is checked, else its `.quest/check.log`.
    Log(ModuleKey),
    Help,
}

pub struct Overlay {
    pub title: String,
    pub kind: OverlayKind,
    pub scroll: u16,
}

/// What a row shows in its STATUS column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    Status(Status),
    Checking,
    Queued,
    Exercise(ExerciseStatus),
}

impl Badge {
    pub fn label(self) -> &'static str {
        match self {
            Self::Status(s) => s.label(),
            Self::Checking => "Checking",
            Self::Queued => "Queued",
            Self::Exercise(ExerciseStatus::Passed) => "Passed",
            Self::Exercise(ExerciseStatus::Failed) => "Failed",
            Self::Exercise(ExerciseStatus::Todo) => "Todo",
            Self::Exercise(ExerciseStatus::Ignored) => "Ignored",
        }
    }

    /// Rows needing attention sort first.
    fn rank(self) -> u8 {
        match self {
            Self::Checking => 0,
            Self::Queued => 1,
            Self::Status(Status::Invalid) | Self::Exercise(ExerciseStatus::Failed) => 2,
            Self::Status(Status::BuildFailed) => 3,
            Self::Status(Status::Stale) => 4,
            Self::Status(Status::InProgress) | Self::Exercise(ExerciseStatus::Todo) => 5,
            Self::Status(Status::NotStarted) => 6,
            Self::Status(Status::Pending) | Self::Exercise(ExerciseStatus::Ignored) => 7,
            Self::Status(Status::Completed) | Self::Exercise(ExerciseStatus::Passed) => 8,
        }
    }
}

pub enum Item<'a> {
    Quest(&'a Quest),
    Module(&'a Module),
    Lesson(&'a Module, &'a Lesson),
    Exercise(&'a Module, &'a ExerciseResult),
}

impl Item<'_> {
    pub fn name(&self) -> &str {
        match self {
            Item::Quest(q) => &q.id,
            Item::Module(m) => &m.key.module,
            Item::Lesson(_, l) => &l.name,
            Item::Exercise(_, e) => e.short_name(),
        }
    }

    fn search_text(&self) -> String {
        let text = match self {
            Item::Quest(q) => format!(
                "{} {} {}",
                q.id,
                q.title(),
                q.manifest
                    .as_ref()
                    .map(|m| m.tags.join(" "))
                    .unwrap_or_default()
            ),
            Item::Module(m) => format!("{} {} {}", m.key.quest, m.key.module, m.title()),
            Item::Lesson(_, l) => format!("{} {} {}", l.number, l.name, l.summary),
            Item::Exercise(_, e) => {
                format!("{} {}", e.name, e.message.as_deref().unwrap_or_default())
            }
        };
        text.to_lowercase()
    }

    pub fn ratio(&self) -> f64 {
        match self {
            Item::Quest(q) => q.counts().ratio(),
            Item::Module(m) => m.counts().ratio(),
            Item::Lesson(m, l) => m.lesson_counts(l.number).ratio(),
            Item::Exercise(_, e) => f64::from(u8::from(e.status == ExerciseStatus::Passed)),
        }
    }

    pub fn evaluated_at(&self) -> Option<i64> {
        match self {
            Item::Quest(q) => q.modules.iter().filter_map(evaluated_at).max(),
            Item::Module(m) | Item::Lesson(m, _) | Item::Exercise(m, _) => evaluated_at(m),
        }
    }

    fn selection(&self) -> Selection {
        match self {
            Item::Quest(q) => Selection::Quest(q.id.clone()),
            Item::Module(m) => Selection::Module(m.key.clone()),
            Item::Lesson(m, l) => Selection::Lesson(m.key.clone(), l.number),
            Item::Exercise(m, e) => Selection::Exercise(m.key.clone(), e.lesson, e.name.clone()),
        }
    }
}

fn evaluated_at(m: &Module) -> Option<i64> {
    m.progress.as_ref().map(|p| p.evaluated_at_ms)
}

/// The selected row, detached from the snapshot so handlers can mutate the app.
#[derive(Debug, Clone)]
enum Selection {
    Quest(String),
    Module(ModuleKey),
    Lesson(ModuleKey, u8),
    Exercise(ModuleKey, u8, String),
}

impl Selection {
    fn module(&self) -> Option<&ModuleKey> {
        match self {
            Self::Quest(_) => None,
            Self::Module(k) | Self::Lesson(k, _) | Self::Exercise(k, _, _) => Some(k),
        }
    }

    fn lesson(&self) -> Option<u8> {
        match self {
            Self::Lesson(_, n) | Self::Exercise(_, n, _) => Some(*n),
            _ => None,
        }
    }

    fn target(&self) -> Target {
        match self {
            Self::Quest(q) => Target::Quest(q.clone()),
            other => Target::Module(other.module().cloned().expect("module selection")),
        }
    }
}

pub struct Job {
    pub targets: Vec<Target>,
    pub force: bool,
}

/// What the event loop must do with the terminal.
pub enum Action {
    None,
    Quit,
    /// Leave the dashboard, run `program` in `dir`, come back (after Enter when `pause`).
    Suspend {
        dir: PathBuf,
        program: String,
        args: Vec<String>,
        pause: bool,
    },
}

pub struct Flash {
    pub text: String,
    pub error: bool,
    pub at: Instant,
}

pub struct App {
    pub ql: Arc<Questline>,
    pub snapshot: Snapshot,
    pub stack: Vec<View>,
    pub table: TableState,
    pub mode: Mode,
    pub input: String,
    pub filter: String,
    pub sort: Option<(SortKey, bool)>,
    pub overlay: Option<Overlay>,
    pub flash: Option<Flash>,
    pub checking: Option<ModuleKey>,
    pub queued: Vec<ModuleKey>,
    pub jobs_in_flight: usize,
    pub live: HashMap<ModuleKey, Vec<String>>,
    jobs: Sender<Job>,
}

impl App {
    pub fn new(ql: Arc<Questline>, snapshot: Snapshot, jobs: Sender<Job>) -> Self {
        let mut table = TableState::default();
        table.select(Some(0));
        Self {
            ql,
            snapshot,
            stack: vec![View::Modules(None)],
            table,
            mode: Mode::Normal,
            input: String::new(),
            filter: String::new(),
            sort: None,
            overlay: None,
            flash: None,
            checking: None,
            queued: Vec::new(),
            jobs_in_flight: 0,
            live: HashMap::new(),
            jobs,
        }
    }

    pub fn view(&self) -> &View {
        self.stack.last().expect("the stack is never empty")
    }

    /// Rows of the current view, filtered and sorted.
    pub fn items(&self) -> Vec<Item<'_>> {
        let s = &self.snapshot;
        let mut items: Vec<Item> = match self.view() {
            View::Quests => s.quests.iter().map(Item::Quest).collect(),
            View::Modules(None) => s.modules().map(Item::Module).collect(),
            View::Modules(Some(q)) => s
                .quest(q)
                .map(|q| q.modules.iter().map(Item::Module).collect())
                .unwrap_or_default(),
            View::Lessons(key) => s
                .module(key)
                .and_then(|m| {
                    Some(
                        m.manifest
                            .as_ref()?
                            .lessons
                            .iter()
                            .map(|l| Item::Lesson(m, l))
                            .collect(),
                    )
                })
                .unwrap_or_default(),
            View::Exercises(key, n) => s
                .module(key)
                .and_then(|m| {
                    Some(
                        m.progress
                            .as_ref()?
                            .exercises
                            .iter()
                            .filter(|e| e.lesson == *n)
                            .map(|e| Item::Exercise(m, e))
                            .collect(),
                    )
                })
                .unwrap_or_default(),
        };
        if !self.filter.is_empty() {
            let needle = self.filter.to_lowercase();
            items.retain(|i| i.search_text().contains(&needle));
        }
        if let Some((key, desc)) = self.sort {
            items.sort_by(|a, b| {
                let order = match key {
                    SortKey::Name => a.name().cmp(b.name()),
                    SortKey::Progress => a.ratio().total_cmp(&b.ratio()),
                    SortKey::Status => self.badge(a).rank().cmp(&self.badge(b).rank()),
                    SortKey::Age => b.evaluated_at().cmp(&a.evaluated_at()),
                };
                if desc { order.reverse() } else { order }
            });
        }
        items
    }

    pub fn badge(&self, item: &Item) -> Badge {
        let module_badge = |m: &Module, status: Status| {
            if self.checking.as_ref() == Some(&m.key) {
                Badge::Checking
            } else if self.queued.contains(&m.key) {
                Badge::Queued
            } else {
                Badge::Status(status)
            }
        };
        match item {
            Item::Quest(q) => {
                if q.modules
                    .iter()
                    .any(|m| self.checking.as_ref() == Some(&m.key))
                {
                    Badge::Checking
                } else {
                    Badge::Status(q.status())
                }
            }
            Item::Module(m) => module_badge(m, m.status()),
            Item::Lesson(m, l) => module_badge(m, m.lesson_status(l.number)),
            Item::Exercise(_, e) => Badge::Exercise(e.status),
        }
    }

    fn selected(&self) -> Option<Selection> {
        let items = self.items();
        items.get(self.table.selected()?).map(Item::selection)
    }

    pub fn flash(&mut self, text: impl Into<String>) {
        self.flash = Some(Flash {
            text: text.into(),
            error: false,
            at: Instant::now(),
        });
    }

    pub fn flash_error(&mut self, text: impl Into<String>) {
        self.flash = Some(Flash {
            text: text.into(),
            error: true,
            at: Instant::now(),
        });
    }

    fn push(&mut self, view: View) {
        self.stack.push(view);
        self.reset_view_state();
    }

    fn reset_to(&mut self, view: View) {
        self.stack = vec![view];
        self.reset_view_state();
    }

    fn reset_view_state(&mut self) {
        self.filter.clear();
        self.table.select(Some(0));
    }

    fn clamp_selection(&mut self) {
        let len = self.items().len();
        let i = self
            .table
            .selected()
            .unwrap_or(0)
            .min(len.saturating_sub(1));
        self.table.select(Some(i));
    }

    pub fn reload(&mut self) {
        match self.ql.sync() {
            Ok(snapshot) => {
                self.snapshot = snapshot;
                self.clamp_selection();
            }
            Err(e) => self.flash_error(format!("reload failed: {e}")),
        }
    }

    fn enqueue(&mut self, targets: Vec<Target>, force: bool) {
        let keys: Vec<ModuleKey> = targets
            .iter()
            .flat_map(|t| match t {
                Target::All => self
                    .snapshot
                    .modules()
                    .map(|m| m.key.clone())
                    .collect::<Vec<_>>(),
                Target::Quest(q) => self
                    .snapshot
                    .quest(q)
                    .map(|q| q.modules.iter().map(|m| m.key.clone()).collect())
                    .unwrap_or_default(),
                Target::Module(k) => vec![k.clone()],
            })
            .collect();
        for key in keys {
            if !self.queued.contains(&key) {
                self.queued.push(key);
            }
        }
        self.jobs_in_flight += 1;
        if self.jobs.send(Job { targets, force }).is_err() {
            self.jobs_in_flight -= 1;
            self.flash_error("the evaluation worker stopped; restart questline");
        }
    }

    pub fn on_eval(&mut self, event: EvalEvent) {
        match event {
            EvalEvent::Skipped { key, reason } => {
                self.queued.retain(|k| *k != key);
                if reason == SkipReason::Invalid {
                    self.flash_error(format!(
                        "{key} breaks the contract: press d for its problems"
                    ));
                }
            }
            EvalEvent::Started { key } => {
                self.queued.retain(|k| *k != key);
                self.live.insert(key.clone(), Vec::new());
                self.flash(format!("checking {key}…"));
                self.checking = Some(key);
            }
            EvalEvent::Output { key, line, .. } => {
                let lines = self.live.entry(key).or_default();
                if lines.len() < MAX_LIVE_LINES {
                    lines.push(line);
                }
            }
            EvalEvent::Finished { module } => {
                let c = module.counts();
                let text = format!(
                    "{}: {}/{} ({:.0}%) {}",
                    module.key,
                    c.passed,
                    c.total(),
                    c.ratio() * 100.0,
                    module.status()
                );
                if module.status() == Status::BuildFailed {
                    self.flash_error(format!("{text}: press l for the log"));
                } else {
                    self.flash(text);
                }
                self.live.remove(&module.key);
                if let Some(slot) = self
                    .snapshot
                    .quests
                    .iter_mut()
                    .flat_map(|q| &mut q.modules)
                    .find(|m| m.key == module.key)
                {
                    *slot = *module;
                }
                self.checking = None;
            }
            EvalEvent::Failed { key, error } => {
                self.flash_error(format!("{key}: {error}"));
                self.live.remove(&key);
                self.checking = None;
            }
        }
    }

    pub fn on_eval_done(&mut self, result: Result<Snapshot, String>) {
        self.jobs_in_flight = self.jobs_in_flight.saturating_sub(1);
        if self.jobs_in_flight == 0 {
            self.checking = None;
            self.queued.clear();
        }
        match result {
            Ok(snapshot) => {
                self.snapshot = snapshot;
                self.clamp_selection();
            }
            Err(e) => self.flash_error(format!("evaluation failed: {e}")),
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) -> Action {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Action::Quit;
        }
        if self.overlay.is_some() {
            self.overlay_key(key);
            return Action::None;
        }
        match self.mode {
            Mode::Filter | Mode::Command => self.input_key(key),
            Mode::Normal => self.normal_key(key),
        }
    }

    fn overlay_key(&mut self, key: KeyEvent) {
        let Some(overlay) = self.overlay.as_mut() else {
            return;
        };
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.overlay = None,
            KeyCode::Down | KeyCode::Char('j') => overlay.scroll = overlay.scroll.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => overlay.scroll = overlay.scroll.saturating_sub(1),
            KeyCode::PageDown | KeyCode::Char(' ') => {
                overlay.scroll = overlay.scroll.saturating_add(20)
            }
            KeyCode::PageUp => overlay.scroll = overlay.scroll.saturating_sub(20),
            KeyCode::Char('g') | KeyCode::Home => overlay.scroll = 0,
            KeyCode::Char('G') | KeyCode::End => overlay.scroll = u16::MAX,
            _ => {}
        }
    }

    fn input_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => {
                if self.mode == Mode::Filter {
                    self.filter.clear();
                }
                self.mode = Mode::Normal;
                self.input.clear();
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut self.input);
                let mode = std::mem::replace(&mut self.mode, Mode::Normal);
                if mode == Mode::Command {
                    return self.command(input.trim());
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) => self.input.push(c),
            _ => {}
        }
        if self.mode == Mode::Filter {
            self.filter = self.input.clone();
            self.table.select(Some(0));
        }
        Action::None
    }

    /// `:` commands, as in k9s: a resource name switches the view.
    pub fn command(&mut self, cmd: &str) -> Action {
        let (word, arg) = cmd
            .split_once(' ')
            .map_or((cmd, ""), |(w, a)| (w, a.trim()));
        match word {
            "" => {}
            "q" | "q!" | "quit" | "exit" => return Action::Quit,
            "quests" | "quest" | "qs" => self.reset_to(View::Quests),
            "modules" | "module" | "mo" | "mods" if arg.is_empty() => {
                self.reset_to(View::Modules(None))
            }
            "modules" | "module" | "mo" | "mods" => return self.command(arg),
            "reload" => self.reload(),
            "check" => self.enqueue(vec![arg.parse().unwrap_or(Target::All)], true),
            "help" => self.open_help(),
            other => match other.parse::<Target>().unwrap_or(Target::All) {
                Target::Quest(q) if self.snapshot.quest(&q).is_some() => {
                    self.reset_to(View::Modules(Some(q)))
                }
                Target::Module(k) if self.snapshot.module(&k).is_some() => {
                    self.reset_to(View::Modules(Some(k.quest.clone())));
                    self.push(View::Lessons(k));
                }
                _ => self.flash_error(format!(
                    "unknown command or quest '{other}' (try :quests, :modules, :<quest>, :q)"
                )),
            },
        }
        Action::None
    }

    fn normal_key(&mut self, key: KeyEvent) -> Action {
        let len = self.items().len();
        let i = self.table.selected().unwrap_or(0);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.table.select(Some((i + 1).min(len.saturating_sub(1))))
            }
            KeyCode::Up | KeyCode::Char('k') => self.table.select(Some(i.saturating_sub(1))),
            KeyCode::Char('f') if ctrl => {
                self.table.select(Some((i + 20).min(len.saturating_sub(1))))
            }
            KeyCode::Char('b') if ctrl => self.table.select(Some(i.saturating_sub(20))),
            KeyCode::PageDown => self.table.select(Some((i + 20).min(len.saturating_sub(1)))),
            KeyCode::PageUp => self.table.select(Some(i.saturating_sub(20))),
            KeyCode::Char('g') | KeyCode::Home => self.table.select(Some(0)),
            KeyCode::Char('G') | KeyCode::End => self.table.select(Some(len.saturating_sub(1))),
            KeyCode::Char('r') if ctrl => {
                self.reload();
                self.flash("reloaded");
            }
            KeyCode::Char('/') => {
                self.mode = Mode::Filter;
                self.input = self.filter.clone();
            }
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.input.clear();
            }
            KeyCode::Char('?') => self.open_help(),
            KeyCode::Esc | KeyCode::Char('q') => {
                if !self.filter.is_empty() {
                    self.filter.clear();
                } else if self.stack.len() > 1 {
                    self.stack.pop();
                    self.clamp_selection();
                } else if key.code == KeyCode::Char('q') {
                    self.flash("ctrl-c or :q quits");
                }
            }
            KeyCode::Char('0') => self.reset_to(View::Modules(None)),
            KeyCode::Char('N') => self.toggle_sort(SortKey::Name),
            KeyCode::Char('P') => self.toggle_sort(SortKey::Progress),
            KeyCode::Char('S') => self.toggle_sort(SortKey::Status),
            KeyCode::Char('A') => self.toggle_sort(SortKey::Age),
            KeyCode::Char('C') => {
                let targets: Vec<Target> = self
                    .items()
                    .iter()
                    .map(|i| i.selection().target())
                    .collect();
                if targets.is_empty() {
                    self.flash("nothing to check here");
                } else {
                    self.enqueue(targets, false);
                }
            }
            _ => return self.selection_key(key),
        }
        Action::None
    }

    /// Keys that act on the selected row.
    fn selection_key(&mut self, key: KeyEvent) -> Action {
        let Some(sel) = self.selected() else {
            return Action::None;
        };
        match key.code {
            KeyCode::Enter => match &sel {
                Selection::Quest(q) => self.push(View::Modules(Some(q.clone()))),
                Selection::Module(k) => self.push(View::Lessons(k.clone())),
                Selection::Lesson(k, n) => {
                    if self
                        .snapshot
                        .module(k)
                        .is_some_and(|m| m.progress.is_some())
                    {
                        self.push(View::Exercises(k.clone(), *n));
                    } else {
                        self.flash("no results yet: press c to check this module");
                    }
                }
                Selection::Exercise(..) => self.describe(&sel),
            },
            KeyCode::Char('d') => self.describe(&sel),
            KeyCode::Char('c') => self.enqueue(vec![sel.target()], true),
            KeyCode::Char('e') => match self.ql.open_in_ide(&sel.target(), sel.lesson()) {
                Ok(launch) => self.flash(format!(
                    "opened {} in `{}`",
                    launch.dir.display(),
                    launch.editor
                )),
                Err(e) => self.flash_error(e.to_string()),
            },
            KeyCode::Char('l') => match sel.module() {
                Some(k) => {
                    self.overlay = Some(Overlay {
                        title: format!("Logs {k}"),
                        kind: OverlayKind::Log(k.clone()),
                        scroll: u16::MAX,
                    })
                }
                None => self.flash("logs are per module: select a module"),
            },
            KeyCode::Char('y') => self.show_manifest(&sel),
            KeyCode::Char('s') => {
                let dir = self.dir_of(&sel);
                let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".into());
                return Action::Suspend {
                    dir,
                    program: shell,
                    args: vec![],
                    pause: false,
                };
            }
            KeyCode::Char('r') => match (sel.module(), sel.lesson()) {
                (Some(k), Some(n)) => {
                    let dir = self.dir_of(&sel);
                    let args = vec!["run".into(), "--quiet".into(), "--".into(), n.to_string()];
                    self.flash(format!("ran lesson {n} of {k}"));
                    return Action::Suspend {
                        dir,
                        program: "cargo".into(),
                        args,
                        pause: true,
                    };
                }
                _ => self.flash("r runs a lesson: open a module (enter) and select one"),
            },
            _ => {}
        }
        Action::None
    }

    fn toggle_sort(&mut self, key: SortKey) {
        self.sort = match self.sort {
            Some((k, desc)) if k == key => Some((key, !desc)),
            _ => Some((key, false)),
        };
        self.table.select(Some(0));
    }

    fn dir_of(&self, sel: &Selection) -> PathBuf {
        let s = &self.snapshot;
        match sel {
            Selection::Quest(q) => s.quest(q).map(|q| q.path.clone()),
            other => other
                .module()
                .and_then(|k| s.module(k))
                .map(|m| m.path.clone()),
        }
        .unwrap_or_else(|| s.root.clone())
    }

    fn open_help(&mut self) {
        self.overlay = Some(Overlay {
            title: "Help".into(),
            kind: OverlayKind::Help,
            scroll: 0,
        });
    }

    fn show_manifest(&mut self, sel: &Selection) {
        let (title, path) = match sel {
            Selection::Quest(_) => (QUEST_MANIFEST, self.dir_of(sel).join(QUEST_MANIFEST)),
            _ => (MODULE_MANIFEST, self.dir_of(sel).join(MODULE_MANIFEST)),
        };
        let lines = match std::fs::read_to_string(&path) {
            Ok(text) => text.lines().map(String::from).collect(),
            Err(e) => vec![format!("{}: {e}", path.display())],
        };
        self.overlay = Some(Overlay {
            title: format!("{title} {}", path.display()),
            kind: OverlayKind::Text(lines),
            scroll: 0,
        });
    }

    fn describe(&mut self, sel: &Selection) {
        let s = &self.snapshot;
        let lines = match sel {
            Selection::Quest(q) => s.quest(q).map(describe_quest),
            Selection::Module(k) => s.module(k).map(describe_module),
            Selection::Lesson(k, n) => s
                .module(k)
                .and_then(|m| Some(describe_lesson(m, m.manifest.as_ref()?.lesson(*n)?))),
            Selection::Exercise(k, _, name) => s.module(k).and_then(|m| {
                Some(describe_exercise(
                    m.progress
                        .as_ref()?
                        .exercises
                        .iter()
                        .find(|e| e.name == *name)?,
                ))
            }),
        };
        let title = match sel {
            Selection::Quest(q) => format!("Describe quest {q}"),
            Selection::Module(k) => format!("Describe module {k}"),
            Selection::Lesson(k, n) => format!("Describe lesson {k} #{n}"),
            Selection::Exercise(k, _, name) => format!("Describe exercise {k} {name}"),
        };
        if let Some(lines) = lines {
            self.overlay = Some(Overlay {
                title,
                kind: OverlayKind::Text(lines),
                scroll: 0,
            });
        }
    }

    /// Lines for the log overlay of `key`.
    pub fn log_lines(&self, key: &ModuleKey) -> Vec<String> {
        if let Some(lines) = self.live.get(key) {
            return lines.clone();
        }
        let Some(module) = self.snapshot.module(key) else {
            return vec![];
        };
        match std::fs::read_to_string(Progress::log_path(&module.path)) {
            Ok(text) => text.lines().map(String::from).collect(),
            Err(_) => vec!["No log yet: press c to check this module.".into()],
        }
    }
}

pub fn age(evaluated_at_ms: Option<i64>) -> String {
    let Some(at) = evaluated_at_ms else {
        return "-".into();
    };
    let secs = (now_ms() - at).max(0) / 1000;
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86_400 => format!("{}h", s / 3600),
        s => format!("{}d", s / 86_400),
    }
}

fn field(lines: &mut Vec<String>, key: &str, value: impl std::fmt::Display) {
    lines.push(format!("{:<13}{value}", format!("{key}:")));
}

fn progress_line(c: questline_core::questkit::Counts) -> String {
    format!(
        "{}/{} passed · {} failed · {} todo ({:.0}%)",
        c.passed,
        c.total(),
        c.failed,
        c.todo,
        c.ratio() * 100.0
    )
}

fn problems(lines: &mut Vec<String>, problems: &[String]) {
    if !problems.is_empty() {
        lines.push(String::new());
        lines.push("Problems:".into());
        lines.extend(problems.iter().map(|p| format!("  - {p}")));
    }
}

fn describe_quest(q: &Quest) -> Vec<String> {
    let mut l = Vec::new();
    field(&mut l, "Name", &q.id);
    if let Some(m) = &q.manifest {
        field(&mut l, "Title", &m.title);
        field(&mut l, "Summary", &m.summary);
        field(&mut l, "Tags", m.tags.join(", "));
        if m.example {
            field(&mut l, "Example", "yes (autosort never adds modules here)");
        }
    }
    field(&mut l, "Path", q.path.display());
    field(&mut l, "Status", q.status());
    field(&mut l, "Progress", progress_line(q.counts()));
    l.push(String::new());
    l.push("Modules:".into());
    for m in &q.modules {
        let c = m.counts();
        l.push(format!(
            "  {:<28} {:>7}  {}",
            m.key.module,
            format!("{}/{}", c.passed, c.total()),
            m.status()
        ));
    }
    problems(&mut l, &q.problems);
    l
}

fn describe_module(m: &Module) -> Vec<String> {
    let mut l = Vec::new();
    field(&mut l, "Name", &m.key.module);
    field(&mut l, "Quest", &m.key.quest);
    if let Some(mm) = &m.manifest {
        field(&mut l, "Title", &mm.title);
        field(&mut l, "Summary", &mm.summary);
        field(&mut l, "Order", mm.order);
        if !mm.requires.is_empty() {
            field(&mut l, "Requires", mm.requires.join(", "));
        }
    }
    field(&mut l, "Path", m.path.display());
    field(&mut l, "Status", m.status());
    match &m.progress {
        Some(p) => {
            field(&mut l, "Progress", progress_line(p.counts()));
            let inputs = if m.is_fresh() {
                "the current files"
            } else {
                "older files (Stale: press c)"
            };
            field(
                &mut l,
                "Evaluated",
                format!("{} ago, of {inputs}", age(Some(p.evaluated_at_ms))),
            );
            if let Some(msg) = &p.message {
                field(&mut l, "Message", msg);
            }
        }
        None => field(&mut l, "Progress", "never evaluated (press c)"),
    }
    field(&mut l, "Fingerprint", &m.fingerprint);
    l.push(String::new());
    l.push(format!("Commands (in {}):", m.path.display()));
    for cmd in [
        "cargo run -- list",
        "cargo run -- 1",
        "cargo run -- check 1",
        "cargo run -- all",
    ] {
        l.push(format!("  {cmd}"));
    }
    if let Some(mm) = &m.manifest {
        l.push(String::new());
        l.push("Lessons:".into());
        for lesson in &mm.lessons {
            let c = m.lesson_counts(lesson.number);
            l.push(format!(
                "  {:>2} {:<32} {:>5}  {}",
                lesson.number,
                lesson.name,
                format!("{}/{}", c.passed, c.total()),
                lesson.summary
            ));
        }
    }
    problems(&mut l, &m.problems);
    l
}

fn describe_lesson(m: &Module, lesson: &Lesson) -> Vec<String> {
    let mut l = Vec::new();
    field(
        &mut l,
        "Lesson",
        format!("{} {}", lesson.number, lesson.name),
    );
    field(&mut l, "Module", &m.key);
    field(&mut l, "Summary", &lesson.summary);
    field(&mut l, "Status", m.lesson_status(lesson.number));
    field(
        &mut l,
        "Progress",
        progress_line(m.lesson_counts(lesson.number)),
    );
    l.push(String::new());
    l.push("Files:".into());
    for file in [
        lesson.lesson_file(),
        lesson.exercise_file(),
        lesson.test_file(),
        lesson.solution_file(),
    ] {
        l.push(format!("  {file}"));
    }
    l.push(String::new());
    l.push(format!(
        "Run: cargo run -- {0}    Check: cargo run -- check {0}",
        lesson.number
    ));
    if let Some(p) = &m.progress {
        l.push(String::new());
        l.push("Exercises:".into());
        for e in p.exercises.iter().filter(|e| e.lesson == lesson.number) {
            l.push(format!(
                "  {:<8} {}",
                Badge::Exercise(e.status).label(),
                e.short_name()
            ));
        }
    }
    l
}

fn describe_exercise(e: &ExerciseResult) -> Vec<String> {
    let mut l = Vec::new();
    field(&mut l, "Exercise", &e.name);
    field(&mut l, "Lesson", e.lesson);
    field(&mut l, "Status", Badge::Exercise(e.status).label());
    if let Some(msg) = &e.message {
        l.push(String::new());
        l.extend(msg.lines().map(String::from));
    }
    l
}
