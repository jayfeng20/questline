//! Plain data a frontend renders: a [`Snapshot`] of quests, modules and their progress.

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use questkit::Counts;
use questkit::ModuleManifest;
use questkit::Outcome;
use questkit::Progress;
use questkit::QuestManifest;
use serde::Serialize;

/// `quest/module`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct ModuleKey {
    pub quest: String,
    pub module: String,
}

impl ModuleKey {
    pub fn new(quest: impl Into<String>, module: impl Into<String>) -> Self {
        Self {
            quest: quest.into(),
            module: module.into(),
        }
    }
}

impl fmt::Display for ModuleKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.quest, self.module)
    }
}

/// What to evaluate or open: everything, one quest, or one module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    All,
    Quest(String),
    Module(ModuleKey),
}

impl FromStr for Target {
    type Err = std::convert::Infallible;

    /// `""` → all, `quest` → a quest, `quest/module` → a module.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim_matches('/');
        Ok(match s.split_once('/') {
            _ if s.is_empty() => Self::All,
            Some((quest, module)) => Self::Module(ModuleKey::new(quest, module)),
            None => Self::Quest(s.to_string()),
        })
    }
}

/// Everything under `<root>/quests/`, as of the last sync.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub root: PathBuf,
    pub quests: Vec<Quest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Quest {
    /// The directory name.
    pub id: String,
    pub path: PathBuf,
    /// `None` when `quest.toml` is missing or invalid (see `problems`).
    pub manifest: Option<QuestManifest>,
    /// Contract violations at the quest level; modules carry their own.
    pub problems: Vec<String>,
    /// Ordered by `order`, then id.
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Module {
    pub key: ModuleKey,
    pub path: PathBuf,
    pub manifest: Option<ModuleManifest>,
    pub problems: Vec<String>,
    /// Fingerprint of the module's inputs now; compare with `progress.fingerprint`.
    pub fingerprint: String,
    /// Latest evaluation, possibly of older inputs.
    pub progress: Option<Progress>,
}

/// One label per row, in the spirit of a pod's STATUS column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Breaks the contract; see `problems`.
    Invalid,
    /// Never evaluated.
    Pending,
    /// Files changed since the last evaluation.
    Stale,
    /// The last evaluation of the current files did not compile.
    BuildFailed,
    NotStarted,
    InProgress,
    Completed,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Invalid => "Invalid",
            Self::Pending => "Pending",
            Self::Stale => "Stale",
            Self::BuildFailed => "BuildFailed",
            Self::NotStarted => "NotStarted",
            Self::InProgress => "InProgress",
            Self::Completed => "Completed",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

fn status_of(counts: Counts) -> Status {
    match (counts.passed, counts.total()) {
        (_, 0) | (0, _) => Status::NotStarted,
        (p, t) if p == t => Status::Completed,
        _ => Status::InProgress,
    }
}

impl Snapshot {
    pub fn modules(&self) -> impl Iterator<Item = &Module> {
        self.quests.iter().flat_map(|q| &q.modules)
    }

    pub fn quest(&self, id: &str) -> Option<&Quest> {
        self.quests.iter().find(|q| q.id == id)
    }

    pub fn module(&self, key: &ModuleKey) -> Option<&Module> {
        self.quest(&key.quest)?
            .modules
            .iter()
            .find(|m| m.key.module == key.module)
    }

    pub fn counts(&self) -> Counts {
        let mut c = Counts::default();
        for q in &self.quests {
            c += q.counts();
        }
        c
    }
}

impl Quest {
    pub fn title(&self) -> &str {
        self.manifest.as_ref().map_or(&self.id, |m| &m.title)
    }

    pub fn counts(&self) -> Counts {
        let mut c = Counts::default();
        for m in &self.modules {
            c += m.counts();
        }
        c
    }

    pub fn status(&self) -> Status {
        if !self.problems.is_empty() || self.modules.is_empty() {
            return Status::Invalid;
        }
        let statuses: Vec<Status> = self.modules.iter().map(Module::status).collect();
        if let Some(s) = [Status::Invalid, Status::BuildFailed, Status::Stale]
            .into_iter()
            .find(|s| statuses.contains(s))
        {
            return s;
        }
        if statuses.iter().all(|s| *s == Status::Pending) {
            Status::Pending
        } else if statuses.iter().all(|s| *s == Status::Completed) {
            Status::Completed
        } else if self.counts().passed > 0 {
            Status::InProgress
        } else {
            Status::NotStarted
        }
    }
}

impl Module {
    pub fn title(&self) -> &str {
        self.manifest
            .as_ref()
            .map_or(&self.key.module, |m| &m.title)
    }

    pub fn order(&self) -> u32 {
        self.manifest.as_ref().map_or(u32::MAX, |m| m.order)
    }

    pub fn counts(&self) -> Counts {
        self.progress
            .as_ref()
            .map(Progress::counts)
            .unwrap_or_default()
    }

    pub fn lesson_counts(&self, lesson: u8) -> Counts {
        self.progress
            .as_ref()
            .map(|p| p.lesson_counts(lesson))
            .unwrap_or_default()
    }

    /// The last evaluation saw exactly the current files.
    pub fn is_fresh(&self) -> bool {
        self.progress
            .as_ref()
            .is_some_and(|p| p.fingerprint == self.fingerprint)
    }

    pub fn status(&self) -> Status {
        if !self.problems.is_empty() {
            return Status::Invalid;
        }
        let Some(progress) = &self.progress else {
            return Status::Pending;
        };
        if !self.is_fresh() {
            Status::Stale
        } else if progress.outcome == Outcome::BuildFailed {
            Status::BuildFailed
        } else {
            status_of(progress.counts())
        }
    }

    /// Status of one lesson's exercises (staleness and build failures are module-wide).
    pub fn lesson_status(&self, lesson: u8) -> Status {
        match self.status() {
            Status::NotStarted | Status::InProgress | Status::Completed => {
                status_of(self.lesson_counts(lesson))
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use questkit::ExerciseResult;
    use questkit::ExerciseStatus;

    fn module(progress: Option<Progress>) -> Module {
        Module {
            key: ModuleKey::new("q", "m"),
            path: PathBuf::from("/q/modules/m"),
            manifest: None,
            problems: vec![],
            fingerprint: "now".into(),
            progress,
        }
    }

    fn progress(fingerprint: &str, outcome: Outcome, statuses: &[ExerciseStatus]) -> Progress {
        let exercises = statuses
            .iter()
            .enumerate()
            .map(|(i, s)| ExerciseResult {
                lesson: 1,
                name: format!("e01::ex{i}"),
                status: *s,
                message: None,
            })
            .collect();
        Progress::new(fingerprint.into(), outcome, exercises)
    }

    #[test]
    fn module_status_follows_freshness_then_outcome_then_counts() {
        use ExerciseStatus::*;
        assert_eq!(module(None).status(), Status::Pending);
        assert_eq!(
            module(Some(progress("old", Outcome::Tested, &[Passed]))).status(),
            Status::Stale
        );
        assert_eq!(
            module(Some(progress("now", Outcome::BuildFailed, &[Passed]))).status(),
            Status::BuildFailed
        );
        assert_eq!(
            module(Some(progress("now", Outcome::Tested, &[Todo, Failed]))).status(),
            Status::NotStarted
        );
        assert_eq!(
            module(Some(progress("now", Outcome::Tested, &[Passed, Todo]))).status(),
            Status::InProgress
        );
        assert_eq!(
            module(Some(progress("now", Outcome::Tested, &[Passed, Ignored]))).status(),
            Status::Completed
        );
        let mut invalid = module(None);
        invalid.problems.push("missing module.toml".into());
        assert_eq!(invalid.status(), Status::Invalid);
    }

    #[test]
    fn quest_status_surfaces_the_module_that_needs_attention() {
        use ExerciseStatus::*;
        let quest = |modules| Quest {
            id: "q".into(),
            path: "/q".into(),
            manifest: None,
            problems: vec![],
            modules,
        };
        let done = || module(Some(progress("now", Outcome::Tested, &[Passed])));
        assert_eq!(quest(vec![done(), done()]).status(), Status::Completed);
        assert_eq!(
            quest(vec![done(), module(None)]).status(),
            Status::InProgress
        );
        assert_eq!(
            quest(vec![
                done(),
                module(Some(progress("old", Outcome::Tested, &[Todo])))
            ])
            .status(),
            Status::Stale
        );
        assert_eq!(quest(vec![module(None)]).status(), Status::Pending);
        assert_eq!(quest(vec![]).status(), Status::Invalid);
    }

    #[test]
    fn targets_parse_from_paths() {
        assert_eq!("".parse::<Target>().unwrap(), Target::All);
        assert_eq!(
            "arrow-rs".parse::<Target>().unwrap(),
            Target::Quest("arrow-rs".into())
        );
        assert_eq!(
            "arrow-rs/compute-kernels/".parse::<Target>().unwrap(),
            Target::Module(ModuleKey::new("arrow-rs", "compute-kernels"))
        );
    }
}
