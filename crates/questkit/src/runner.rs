//! The command line every module binary exposes (contract §4).

use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;

use crate::ExerciseStatus;
use crate::ModuleManifest;
use crate::Outcome;
use crate::Progress;
use crate::check;
use crate::check::Stream;

const USAGE: &str = "\
usage: cargo run -- [command]
  list            the syllabus with your progress (default)
  <n|name>        run one lesson, e.g. `cargo run -- 2` or `cargo run -- streams`
  all             run every lesson
  check [n|name]  run the exercise tests, save progress, show lesson n (default: all)";

/// Entry point for a module's `main.rs`:
///
/// ```ignore
/// fn main() -> anyhow::Result<()> {
///     questkit::run(env!("CARGO_MANIFEST_DIR"), my_module::lessons::run)
/// }
/// ```
pub fn run(module_dir: &str, run_lesson: fn(u8) -> anyhow::Result<()>) -> anyhow::Result<()> {
    // `cargo run` sets the real directory at run time; the compiled-in one goes stale when a
    // quest is copied together with its target/.
    let dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map_or_else(|| PathBuf::from(module_dir), PathBuf::from);
    let dir = dir.as_path();
    let manifest = ModuleManifest::load(dir)?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("list") => print_syllabus(dir, &manifest),
        Some("help" | "-h" | "--help") => println!("{USAGE}"),
        Some("all") => {
            for lesson in &manifest.lessons {
                run_lesson(lesson.number)
                    .with_context(|| format!("lesson {} failed", lesson.number))?;
            }
        }
        Some("check") => {
            let only = match args.get(1) {
                Some(arg) => Some(manifest.resolve(arg).with_context(|| unknown(arg))?.number),
                None => None,
            };
            if !check_module(dir, &manifest, only)? {
                std::process::exit(1);
            }
        }
        Some(arg) => {
            let lesson = manifest.resolve(arg).with_context(|| unknown(arg))?;
            run_lesson(lesson.number)?;
            println!("\n   ✏️  Your turn: {}", lesson.exercise_file());
            println!("       check it with  cargo run -- check {}", lesson.number);
        }
    }
    Ok(())
}

fn unknown(arg: &str) -> String {
    format!("unknown lesson '{arg}'; try `cargo run -- list`\n\n{USAGE}")
}

fn print_syllabus(dir: &Path, manifest: &ModuleManifest) {
    let progress = Progress::load(dir).ok().flatten();
    println!("{} — {}\n", manifest.title, manifest.summary);
    println!("Lessons (run with `cargo run -- <number|name>` or `all`):");
    for lesson in &manifest.lessons {
        let done = match &progress {
            Some(p) => {
                let c = p.lesson_counts(lesson.number);
                format!("{}/{}", c.passed, c.total())
            }
            None => "·".to_string(),
        };
        println!(
            "  {:>2}  {:<32} {done:>5}  {}",
            lesson.number, lesson.name, lesson.summary
        );
    }
    if progress.is_none() {
        println!("\nProgress appears after `cargo run -- check`.");
    }
}

/// Returns false when the build failed.
fn check_module(dir: &Path, manifest: &ModuleManifest, only: Option<u8>) -> anyhow::Result<bool> {
    println!("Checking {} (cargo test --test exercises)…", manifest.title);
    let progress = check::evaluate(dir, |stream, line| {
        // Build output streams live; test output is summarised below.
        if stream == Stream::Stderr {
            eprintln!("{line}");
        }
    })?;
    println!();

    if progress.outcome == Outcome::BuildFailed {
        println!(
            "❌ build failed: {}",
            progress
                .message
                .as_deref()
                .unwrap_or("see the errors above")
        );
        println!("   Your saved progress keeps the results of the last build that ran.");
        return Ok(false);
    }
    for lesson in manifest
        .lessons
        .iter()
        .filter(|l| only.is_none_or(|n| n == l.number))
    {
        let c = progress.lesson_counts(lesson.number);
        println!(
            "Lesson {} · {:<40} {}/{}",
            lesson.number,
            lesson.name,
            c.passed,
            c.total()
        );
        for e in progress
            .exercises
            .iter()
            .filter(|e| e.lesson == lesson.number)
        {
            let (icon, show_message) = match e.status {
                ExerciseStatus::Passed => ("✅", false),
                ExerciseStatus::Failed => ("❌", true),
                ExerciseStatus::Todo => ("⬜", false),
                ExerciseStatus::Ignored => ("⏭️ ", false),
            };
            println!("   {icon} {}", e.short_name());
            if let Some(message) = e.message.as_deref().filter(|_| show_message) {
                for line in message.lines() {
                    println!("        {line}");
                }
            }
        }
    }
    if let Some(message) = &progress.message {
        println!("\n⚠️  {message}");
    }
    let c = progress.counts();
    println!(
        "\nProgress: {}/{} exercises ({:.0}%) · saved to {}",
        c.passed,
        c.total(),
        c.ratio() * 100.0,
        Progress::path(Path::new(".")).display()
    );
    println!(
        "Full output: {}",
        Progress::log_path(Path::new(".")).display()
    );
    Ok(true)
}
