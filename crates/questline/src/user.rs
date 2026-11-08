//! Commands for taking a quest. `run` and `check` act on the module the current directory is
//! in, through that module's own runner (`cargo run -- …`, contract §4).

use std::process::ExitCode;

use anyhow::Context;
use clap::Subcommand;
use questline_core::Questline;
use questline_core::Target;

#[derive(Subcommand)]
pub enum Command {
    /// Run a lesson of the module you are in (`cargo run -- <lesson>`); no lesson: the syllabus.
    Run {
        /// Its number, part of its name, or `all`.
        lesson: Option<String>,
    },
    /// Run the exercise tests of the module you are in and record progress (`cargo run -- check`).
    Check {
        /// Show only this lesson's results (its number or part of its name).
        lesson: Option<String>,
    },
    /// Quests and modules with their progress.
    List {
        /// The full snapshot as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Open a quest or module in a new IDE window ($QUESTLINE_IDE, else code/cursor/zed/...).
    Open {
        /// `quest` or `quest/module`.
        target: Target,
        /// Also open this lesson's lesson and exercise files.
        #[arg(long)]
        lesson: Option<u8>,
    },
}

pub fn run(
    command: Command,
    ql: impl FnOnce() -> anyhow::Result<Questline>,
) -> anyhow::Result<ExitCode> {
    match command {
        Command::Run { lesson } => return runner(lesson),
        Command::Check { lesson } => {
            return runner(std::iter::once("check".to_string()).chain(lesson));
        }
        Command::List { json } => {
            let snapshot = ql()?.sync()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                println!(
                    "{:<20} {:<28} {:>9} {:>5}  STATUS",
                    "QUEST", "MODULE", "DONE", "%"
                );
                for m in snapshot.modules() {
                    let c = m.counts();
                    let pct = if m.progress.is_some() {
                        format!("{:.0}", c.ratio() * 100.0)
                    } else {
                        "-".into()
                    };
                    let done = format!("{}/{}", c.passed, c.total());
                    println!(
                        "{:<20} {:<28} {done:>9} {pct:>5}  {}",
                        m.key.quest,
                        m.key.module,
                        m.status()
                    );
                }
            }
        }
        Command::Open { target, lesson } => {
            let launch = ql()?.open_in_ide(&target, lesson)?;
            println!("opened {} in `{}`", launch.dir.display(), launch.editor);
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Runs the runner of the module the current directory is in, and exits as it did.
fn runner(args: impl IntoIterator<Item = String>) -> anyhow::Result<ExitCode> {
    let cwd = std::env::current_dir().context("current directory")?;
    let status = questline_core::runner(questline_core::find_module(&cwd)?)
        .args(args)
        .status()
        .context("run cargo (is it on PATH?)")?;
    // No code when a signal killed it.
    Ok(status
        .code()
        .and_then(|c| u8::try_from(c).ok())
        .map_or(ExitCode::FAILURE, ExitCode::from))
}
