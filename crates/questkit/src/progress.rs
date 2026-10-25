//! `<module>/.quest/progress.json`: the result of the latest check of one module.

use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use serde::Deserialize;
use serde::Serialize;

use crate::CONTRACT_VERSION;

/// Per-module state written by checks; gitignored.
pub const STATE_DIR: &str = ".quest";
pub const PROGRESS_FILE: &str = "progress.json";
/// Full output of the latest check (build and test), for when the summary is not enough.
pub const LOG_FILE: &str = "check.log";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    pub contract: u32,
    /// [`crate::fingerprint::fingerprint`] of the module's inputs when the check started.
    pub fingerprint: String,
    pub evaluated_at_ms: i64,
    pub outcome: Outcome,
    /// Why the build failed, or a note about an incomplete test run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// On a failed build: the results of the last build that ran.
    pub exercises: Vec<ExerciseResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Tested,
    BuildFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExerciseResult {
    /// From the test path `eNN::...`; 0 when the test is outside an `eNN` module.
    pub lesson: u8,
    /// Full libtest name, e.g. `e03::ex2_rejects_bad_ids`.
    pub name: String,
    pub status: ExerciseStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExerciseStatus {
    Passed,
    /// Attempted and wrong.
    Failed,
    /// Still the `todo!()` stub.
    Todo,
    Ignored,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub passed: u32,
    pub failed: u32,
    pub todo: u32,
}

impl Counts {
    /// Exercises that count towards completion (ignored tests do not).
    pub fn total(&self) -> u32 {
        self.passed + self.failed + self.todo
    }

    pub fn ratio(&self) -> f64 {
        match self.total() {
            0 => 0.0,
            t => f64::from(self.passed) / f64::from(t),
        }
    }

    pub fn tally<'a>(results: impl IntoIterator<Item = &'a ExerciseResult>) -> Self {
        results.into_iter().fold(Self::default(), |mut c, r| {
            match r.status {
                ExerciseStatus::Passed => c.passed += 1,
                ExerciseStatus::Failed => c.failed += 1,
                ExerciseStatus::Todo => c.todo += 1,
                ExerciseStatus::Ignored => {}
            }
            c
        })
    }
}

impl std::ops::AddAssign for Counts {
    fn add_assign(&mut self, other: Self) {
        self.passed += other.passed;
        self.failed += other.failed;
        self.todo += other.todo;
    }
}

impl ExerciseResult {
    /// The test's own name without the `eNN::` module path.
    pub fn short_name(&self) -> &str {
        self.name.rsplit("::").next().unwrap_or(&self.name)
    }
}

impl Progress {
    pub fn new(fingerprint: String, outcome: Outcome, exercises: Vec<ExerciseResult>) -> Self {
        Self {
            contract: CONTRACT_VERSION,
            fingerprint,
            evaluated_at_ms: now_ms(),
            outcome,
            message: None,
            exercises,
        }
    }

    pub fn path(module_dir: &Path) -> PathBuf {
        module_dir.join(STATE_DIR).join(PROGRESS_FILE)
    }

    pub fn log_path(module_dir: &Path) -> PathBuf {
        module_dir.join(STATE_DIR).join(LOG_FILE)
    }

    /// `Ok(None)` when the module has never been checked.
    pub fn load(module_dir: &Path) -> anyhow::Result<Option<Self>> {
        let path = Self::path(module_dir);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
        };
        let progress =
            serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        Ok(Some(progress))
    }

    /// Writes atomically, so a reader never sees half a file.
    pub fn save(&self, module_dir: &Path) -> anyhow::Result<()> {
        let path = Self::path(module_dir);
        write_atomic(&path, serde_json::to_string_pretty(self)?.as_bytes())
    }

    pub fn counts(&self) -> Counts {
        Counts::tally(&self.exercises)
    }

    pub fn lesson_counts(&self, lesson: u8) -> Counts {
        Counts::tally(self.exercises.iter().filter(|e| e.lesson == lesson))
    }
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let dir = path.parent().context("path has no parent")?;
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, bytes).with_context(|| format!("write {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("rename to {}", path.display()))
}

pub fn now_ms() -> i64 {
    let since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(since_epoch.as_millis()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(lesson: u8, name: &str, status: ExerciseStatus) -> ExerciseResult {
        ExerciseResult {
            lesson,
            name: name.into(),
            status,
            message: None,
        }
    }

    #[test]
    fn counts_ignore_ignored_tests() {
        let p = Progress::new(
            "f".into(),
            Outcome::Tested,
            vec![
                result(1, "e01::a", ExerciseStatus::Passed),
                result(1, "e01::b", ExerciseStatus::Todo),
                result(2, "e02::a", ExerciseStatus::Failed),
                result(2, "e02::b", ExerciseStatus::Ignored),
            ],
        );
        assert_eq!(
            p.counts(),
            Counts {
                passed: 1,
                failed: 1,
                todo: 1
            }
        );
        assert_eq!(p.counts().total(), 3);
        assert_eq!(
            p.lesson_counts(2),
            Counts {
                passed: 0,
                failed: 1,
                todo: 0
            }
        );
        assert_eq!(p.exercises[0].short_name(), "a");
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Progress::load(dir.path()).unwrap().is_none());
        let mut p = Progress::new(
            "abc".into(),
            Outcome::BuildFailed,
            vec![result(1, "e01::x", ExerciseStatus::Passed)],
        );
        p.message = Some("error[E0308]: mismatched types".into());
        p.save(dir.path()).unwrap();
        assert_eq!(Progress::load(dir.path()).unwrap(), Some(p));
    }
}
