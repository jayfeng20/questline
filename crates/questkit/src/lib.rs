//! The questline quest contract (see `docs/contract.md`).
//!
//! A quest is a Cargo workspace under `quests/<quest>/` with a `quest.toml`; each module is a
//! member crate under `modules/<module>/` with a `module.toml`. This crate is shared by both
//! sides of the contract:
//!
//! * quest modules call [`run`] from their `main.rs` to get the standard commands
//!   (`list`, `<lesson>`, `all`, `check`);
//! * the questline backend reads the same [`manifest`] and [`progress`] types and calls
//!   [`check::evaluate`], so a check started from the dashboard and one started from a terminal
//!   produce the same progress file.

pub mod check;
pub mod fingerprint;
pub mod libtest;
pub mod manifest;
pub mod progress;
mod runner;

pub use manifest::{CONTRACT_VERSION, Lesson, ModuleManifest, QuestManifest};
pub use progress::{Counts, ExerciseResult, ExerciseStatus, Outcome, Progress};
pub use runner::run;
