//! `questline`: the dashboard (default) and a few scriptable subcommands, all thin wrappers
//! over `questline-core`.

mod tui;

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;
use clap::Subcommand;
use questline_core::EvalEvent;
use questline_core::Questline;
use questline_core::SkipReason;
use questline_core::Status;
use questline_core::Target;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The directory holding quests/. Default: $QUESTLINE_ROOT, else the nearest ancestor of
    /// the current directory with a quests/ directory, else the checkout this was built from.
    #[arg(long, global = true, value_name = "DIR")]
    root: Option<PathBuf>,
    /// The dashboard's colours.
    #[arg(long, env = "QUESTLINE_THEME", value_enum, default_value_t)]
    theme: tui::Theme,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// The dashboard (what plain `questline` runs).
    Tui,
    /// Quests and modules with their progress.
    List {
        /// The full snapshot as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Run exercise tests where files changed since the last evaluation, and record progress.
    Eval {
        /// `quest` or `quest/module`; default: everything.
        targets: Vec<String>,
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
    /// Open a quest or module in a new IDE window ($QUESTLINE_IDE, else code/cursor/zed/...).
    Open {
        /// `quest` or `quest/module`.
        target: String,
        /// Also open this lesson's lesson and exercise files.
        #[arg(long)]
        lesson: Option<u8>,
    },
    /// Check quest directories against the contract (default: every quest under the root).
    Validate { paths: Vec<PathBuf> },
    /// Move a quest or module to a position in learning order and renumber the others.
    Order {
        /// `quest` or `quest/module`.
        target: String,
        /// 1 = learn first; past the end = last.
        #[arg(value_parser = clap::value_parser!(u32).range(1..))]
        position: u32,
    },
    /// Print the questline root.
    Root,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    let root = find_root(cli.root)?;
    let ql = Questline::open(&root)?;
    match cli.command.unwrap_or(Command::Tui) {
        Command::Tui => tui::run(ql, cli.theme)?,
        Command::Root => println!("{}", root.display()),
        Command::List { json } => {
            let snapshot = ql.sync()?;
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
        Command::Eval {
            targets,
            force,
            verbose,
            json,
        } => {
            let targets: Vec<Target> = if targets.is_empty() {
                vec![Target::All]
            } else {
                targets
                    .iter()
                    .map(|t| t.parse().unwrap_or(Target::All))
                    .collect()
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
        Command::Open { target, lesson } => {
            let target: Target = target.parse().unwrap_or(Target::All);
            let launch = ql.open_in_ide(&target, lesson)?;
            println!("opened {} in `{}`", launch.dir.display(), launch.editor);
        }
        Command::Order { target, position } => {
            let target: Target = target.parse().unwrap_or(Target::All);
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

fn find_root(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(root) = explicit.or_else(|| std::env::var_os("QUESTLINE_ROOT").map(PathBuf::from)) {
        return Ok(root);
    }
    let cwd = std::env::current_dir().context("current directory")?;
    if let Some(root) = questline_core::find_root(&cwd) {
        return Ok(root);
    }
    // `cargo install --path crates/questline` from a checkout keeps working from anywhere.
    let built_from = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if questline_core::is_root(&built_from) {
        return Ok(built_from.canonicalize().unwrap_or(built_from));
    }
    anyhow::bail!("no questline root: run inside a checkout, pass --root, or set QUESTLINE_ROOT")
}
