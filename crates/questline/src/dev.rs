//! Commands for writing quests, and for scripts and CI.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Subcommand;
use questline_core::EvalEvent;
use questline_core::Questline;
use questline_core::SkipReason;
use questline_core::Status;
use questline_core::Target;

#[derive(Subcommand)]
pub enum Command {
    /// Run exercise tests where files changed since the last evaluation, and record progress.
    Eval {
        /// `quest` or `quest/module`; default: everything.
        targets: Vec<Target>,
        /// Re-run even modules whose cached result is current.
        #[arg(long)]
        force: bool,
        /// Print build and test output.
        #[arg(long, short)]
        verbose: bool,
        /// One JSON event per line.
        #[arg(long, conflicts_with = "verbose")]
        json: bool,
    },
    /// Check quest directories against the contract (default: every quest under the root).
    Validate { paths: Vec<PathBuf> },
    /// Move a quest or module to a position in learning order and renumber the others.
    Order {
        /// `quest` or `quest/module`.
        target: Target,
        /// 1 = learn first; past the end = last.
        #[arg(value_parser = clap::value_parser!(u32).range(1..))]
        position: u32,
    },
    /// Print the questline root.
    Root,
}

pub fn run(command: Command, ql: Questline) -> anyhow::Result<ExitCode> {
    match command {
        Command::Root => println!("{}", ql.root().display()),
        Command::Eval {
            targets,
            force,
            verbose,
            json,
        } => {
            let targets = if targets.is_empty() {
                vec![Target::All]
            } else {
                targets
            };
            let mut failed = false;
            ql.evaluate(&targets, force, &mut |event| {
                if json {
                    println!("{}", serde_json::to_string(&event).unwrap_or_default());
                    return;
                }
                match event {
                    EvalEvent::Skipped {
                        key,
                        reason: SkipReason::UpToDate,
                    } => println!("·  {key}: up to date"),
                    EvalEvent::Skipped {
                        key,
                        reason: SkipReason::Invalid,
                    } => {
                        failed = true;
                        println!("✗  {key}: breaks the contract (questline validate)");
                    }
                    EvalEvent::Started { key } => println!("▶  {key}: checking…"),
                    EvalEvent::Output { line, .. } if verbose => println!("   {line}"),
                    EvalEvent::Output { .. } => {}
                    EvalEvent::Finished { module } => {
                        let c = module.counts();
                        let built = module.status() != Status::BuildFailed;
                        failed |= !built;
                        let mark = if built { "✓" } else { "✗" };
                        println!(
                            "{mark}  {}: {}/{} ({:.0}%) {}",
                            module.key,
                            c.passed,
                            c.total(),
                            c.ratio() * 100.0,
                            module.status()
                        );
                    }
                    EvalEvent::Failed { key, error } => {
                        failed = true;
                        println!("✗  {key}: {error}");
                    }
                }
            })?;
            if failed {
                return Ok(ExitCode::FAILURE);
            }
        }
        Command::Order { target, position } => {
            let snapshot = ql.set_order(&target, position)?;
            let rows: Vec<(u32, &str)> = match &target {
                Target::Module(key) => snapshot
                    .quest(&key.quest)
                    .into_iter()
                    .flat_map(|q| &q.modules)
                    .map(|m| (m.order(), m.key.module.as_str()))
                    .collect(),
                _ => snapshot
                    .quests
                    .iter()
                    .filter_map(|q| Some((q.manifest.as_ref()?.order, q.id.as_str())))
                    .collect(),
            };
            for (order, id) in rows {
                println!("{order:>3}  {id}");
            }
        }
        Command::Validate { paths } => {
            let dirs = if paths.is_empty() {
                ql.sync()?.quests.into_iter().map(|q| q.path).collect()
            } else {
                paths
            };
            let mut ok = true;
            for dir in dirs {
                let quest = questline_core::load_quest(&dir);
                let problems: Vec<String> = quest
                    .problems
                    .iter()
                    .cloned()
                    .chain(quest.modules.iter().flat_map(|m| {
                        m.problems
                            .iter()
                            .map(move |p| format!("{}: {p}", m.key.module))
                    }))
                    .collect();
                if problems.is_empty() {
                    let n = quest.modules.len();
                    println!(
                        "✓  {} ({n} module{})",
                        quest.id,
                        if n == 1 { "" } else { "s" }
                    );
                } else {
                    ok = false;
                    println!("✗  {} ({})", quest.id, dir.display());
                    for p in problems {
                        println!("     {p}");
                    }
                }
            }
            if !ok {
                return Ok(ExitCode::FAILURE);
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
