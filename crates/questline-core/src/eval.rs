//! Events reported while modules are evaluated.

use questkit::check::Stream;
use serde::Serialize;

use crate::model::Module;
use crate::model::ModuleKey;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum EvalEvent {
    /// No cargo run needed (or possible).
    Skipped {
        key: ModuleKey,
        reason: SkipReason,
    },
    Started {
        key: ModuleKey,
    },
    /// One line of build or test output.
    Output {
        key: ModuleKey,
        stream: Stream,
        line: String,
    },
    /// The module after its new progress was recorded.
    Finished {
        module: Box<Module>,
    },
    /// The check could not run at all (as opposed to failing tests or a failed build).
    Failed {
        key: ModuleKey,
        error: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// The cached result was computed from the current files.
    UpToDate,
    /// The module breaks the contract, so there is nothing to run.
    Invalid,
}
