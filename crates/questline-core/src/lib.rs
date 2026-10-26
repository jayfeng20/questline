//! The questline backend: everything a frontend (the TUI today, a macOS app later) needs, as
//! plain data and blocking calls. Frontends choose their own threading: long calls report
//! progress through a callback, and [`Questline`] is `Send + Sync`.
//!
//! ```no_run
//! use questline_core::{Questline, Target};
//!
//! let ql = Questline::open("/path/to/questline")?;
//! let snapshot = ql.sync()?; // rescan quests/, no cargo
//! let snapshot = ql.evaluate(&[Target::All], false, &mut |event| println!("{event:?}"))?;
//! println!("{} of {} exercises", snapshot.counts().passed, snapshot.counts().total());
//! # Ok::<(), questline_core::Error>(())
//! ```

mod discover;
mod eval;
mod ide;
mod model;
mod order;
mod store;

use std::path::Path;
use std::path::PathBuf;

pub use discover::QUESTS_DIR;
pub use discover::load_quest;
pub use eval::EvalEvent;
pub use eval::SkipReason;
pub use ide::Launch;
pub use model::Module;
pub use model::ModuleKey;
pub use model::Quest;
pub use model::Snapshot;
pub use model::Status;
pub use model::Target;
pub use questkit;
pub use questkit::check::Stream;
pub use store::STATE_DIR;

use crate::store::Store;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    NotFound(String),
    #[error("no IDE found: set QUESTLINE_IDE to its command (e.g. `code -n`, `zed`, `cursor -n`)")]
    NoIde,
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("registry {0}")]
    Registry(String),
    #[error("{0}")]
    Invalid(String),
}

impl Error {
    fn io(context: String, source: std::io::Error) -> Self {
        Self::Io { context, source }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A questline root: the directory holding `quests/` (and `.questline/`, its state).
pub struct Questline {
    root: PathBuf,
    store: Store,
}

impl Questline {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        if !is_root(&root) {
            return Err(Error::NotFound(format!(
                "{} has no {QUESTS_DIR}/ directory",
                root.display()
            )));
        }
        let store = Store::new(&root);
        Ok(Self { root, store })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn registry_path(&self) -> PathBuf {
        self.store.path()
    }

    /// Rescans `quests/`, merges each module's `.quest/progress.json` with the registry (the
    /// newer evaluation wins) and rewrites the registry. Cheap: reads manifests and file
    /// metadata, never runs cargo.
    pub fn sync(&self) -> Result<Snapshot> {
        let _lock = self.store.lock()?;
        let mut cached = self.store.load_progress()?;
        let mut quests = discover::scan(&self.root)
            .map_err(|e| Error::io(format!("scan {}", self.root.display()), e))?;
        for module in quests.iter_mut().flat_map(|q| &mut q.modules) {
            if let Some(registry) = cached.remove(&module.key) {
                let newer_on_disk = module
                    .progress
                    .as_ref()
                    .is_some_and(|p| p.evaluated_at_ms >= registry.evaluated_at_ms);
                if !newer_on_disk {
                    module.progress = Some(registry);
                }
            }
        }
        self.store.save(&quests, questkit::progress::now_ms())?;
        Ok(Snapshot {
            root: self.root.clone(),
            quests,
        })
    }

    /// Evaluates the modules in `targets` (in snapshot order), running `cargo test` only for
    /// modules whose files changed since their last evaluation, or for all of them with
    /// `force`. Progress is recorded after each module, so an interrupted run keeps what it
    /// finished.
    pub fn evaluate(
        &self,
        targets: &[Target],
        force: bool,
        on_event: &mut dyn FnMut(EvalEvent),
    ) -> Result<Snapshot> {
        let snapshot = self.sync()?;
        for key in resolve(&snapshot, targets)? {
            let Some(module) = snapshot.module(&key) else {
                continue;
            };
            if !module.problems.is_empty() {
                on_event(EvalEvent::Skipped {
                    key,
                    reason: SkipReason::Invalid,
                });
                continue;
            }
            if !force && module.is_fresh() {
                on_event(EvalEvent::Skipped {
                    key,
                    reason: SkipReason::UpToDate,
                });
                continue;
            }
            on_event(EvalEvent::Started { key: key.clone() });
            let result = questkit::check::evaluate(&module.path, |stream, line| {
                on_event(EvalEvent::Output {
                    key: key.clone(),
                    stream,
                    line: line.to_string(),
                });
            });
            match result {
                Ok(_) => {
                    let updated = self.sync()?;
                    if let Some(module) = updated.module(&key) {
                        on_event(EvalEvent::Finished {
                            module: Box::new(module.clone()),
                        });
                    }
                }
                Err(e) => on_event(EvalEvent::Failed {
                    key,
                    error: format!("{e:#}"),
                }),
            }
        }
        self.sync()
    }

    /// Moves a quest among the quests (an example among the examples), or a module within its
    /// quest, to `position` (1-based, clamped), and renumbers the others `1, 2, 3, …` in their
    /// current order. Only manifests whose `order` changes are rewritten, comments kept.
    pub fn set_order(&self, target: &Target, position: u32) -> Result<Snapshot> {
        let quests = discover::scan(&self.root)
            .map_err(|e| Error::io(format!("scan {}", self.root.display()), e))?;
        order::set_order(&quests, target, position)?;
        self.sync()
    }

    /// Opens a quest or module directory in a new IDE window, with its README (or, given a
    /// lesson, the lesson and exercise files) open.
    pub fn open_in_ide(&self, target: &Target, lesson: Option<u8>) -> Result<Launch> {
        let snapshot = discover::scan(&self.root)
            .map_err(|e| Error::io(format!("scan {}", self.root.display()), e))?;
        let snapshot = Snapshot {
            root: self.root.clone(),
            quests: snapshot,
        };
        let (dir, mut files) = match target {
            Target::All => (self.root.clone(), vec![self.root.join("README.md")]),
            Target::Quest(id) => {
                let quest = snapshot
                    .quest(id)
                    .ok_or_else(|| Error::NotFound(format!("no quest '{id}'")))?;
                (quest.path.clone(), vec![quest.path.join("README.md")])
            }
            Target::Module(key) => {
                let module = snapshot
                    .module(key)
                    .ok_or_else(|| Error::NotFound(format!("no module '{key}'")))?;
                let files = match lesson {
                    None => vec![module.path.join("README.md")],
                    Some(n) => {
                        let manifest = module.manifest.as_ref();
                        let lesson = manifest
                            .and_then(|m| m.lesson(n))
                            .ok_or_else(|| Error::NotFound(format!("no lesson {n} in '{key}'")))?;
                        vec![
                            module.path.join(lesson.lesson_file()),
                            module.path.join(lesson.exercise_file()),
                        ]
                    }
                };
                (module.path.clone(), files)
            }
        };
        files.retain(|f| f.is_file());
        ide::open(&dir, &files)
    }
}

/// Whether `dir` looks like a questline root.
pub fn is_root(dir: &Path) -> bool {
    dir.join(QUESTS_DIR).is_dir()
}

/// The nearest ancestor of `start` (inclusive) that is a questline root.
pub fn find_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|d| is_root(d))
        .map(Path::to_path_buf)
}

fn resolve(snapshot: &Snapshot, targets: &[Target]) -> Result<Vec<ModuleKey>> {
    let mut keys: Vec<ModuleKey> = Vec::new();
    let mut push = |key: &ModuleKey| {
        if !keys.contains(key) {
            keys.push(key.clone());
        }
    };
    for target in targets {
        match target {
            Target::All => snapshot.modules().for_each(|m| push(&m.key)),
            Target::Quest(id) => {
                let quest = snapshot
                    .quest(id)
                    .ok_or_else(|| Error::NotFound(format!("no quest '{id}'")))?;
                quest.modules.iter().for_each(|m| push(&m.key));
            }
            Target::Module(key) => {
                snapshot
                    .module(key)
                    .ok_or_else(|| Error::NotFound(format!("no module '{key}'")))?;
                push(key);
            }
        }
    }
    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use questkit::Outcome;
    use questkit::Progress;

    #[test]
    fn sync_prefers_the_newer_of_registry_and_progress_file() {
        let root = tempfile::tempdir().unwrap();
        let module_dir = discover::tests::write_quest(root.path(), "q", "m", 1);
        let ql = Questline::open(root.path()).unwrap();
        let key = ModuleKey::new("q", "m");

        assert!(ql.sync().unwrap().module(&key).unwrap().progress.is_none());

        let mut first = Progress::new("a".into(), Outcome::Tested, vec![]);
        first.evaluated_at_ms = 1;
        first.save(&module_dir).unwrap();
        assert_eq!(
            ql.sync().unwrap().module(&key).unwrap().progress,
            Some(first.clone())
        );

        // A deleted progress file does not lose the result: the registry still has it.
        std::fs::remove_dir_all(module_dir.join(".quest")).unwrap();
        assert_eq!(
            ql.sync().unwrap().module(&key).unwrap().progress,
            Some(first)
        );

        let mut second = Progress::new("b".into(), Outcome::BuildFailed, vec![]);
        second.evaluated_at_ms = 2;
        second.save(&module_dir).unwrap();
        assert_eq!(
            ql.sync().unwrap().module(&key).unwrap().progress,
            Some(second)
        );
    }

    #[test]
    fn evaluate_skips_fresh_and_invalid_modules_without_running_cargo() {
        let root = tempfile::tempdir().unwrap();
        let fresh = discover::tests::write_quest(root.path(), "q", "fresh", 1);
        let broken = discover::tests::write_quest(root.path(), "q", "broken", 1);
        std::fs::remove_file(broken.join("module.toml")).unwrap();
        let ql = Questline::open(root.path()).unwrap();
        let fingerprint = ql
            .sync()
            .unwrap()
            .module(&ModuleKey::new("q", "fresh"))
            .unwrap()
            .fingerprint
            .clone();
        Progress::new(fingerprint, Outcome::Tested, vec![])
            .save(&fresh)
            .unwrap();

        let mut events = Vec::new();
        ql.evaluate(&[Target::Quest("q".into())], false, &mut |e| events.push(e))
            .unwrap();
        let skipped: Vec<_> = events
            .iter()
            .map(|e| match e {
                EvalEvent::Skipped { key, reason } => (key.module.as_str(), *reason),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        // A module without a manifest has no `order`, so it sorts last.
        assert_eq!(
            skipped,
            [
                ("fresh", SkipReason::UpToDate),
                ("broken", SkipReason::Invalid)
            ]
        );
        assert!(
            ql.evaluate(&[Target::Quest("nope".into())], false, &mut |_| {})
                .is_err()
        );
    }

    #[test]
    fn roots_are_found_from_subdirectories() {
        let root = tempfile::tempdir().unwrap();
        let module = discover::tests::write_quest(root.path(), "q", "m", 1);
        assert_eq!(find_root(&module.join("src")).unwrap(), root.path());
        assert!(Questline::open(module).is_err());
    }
}
