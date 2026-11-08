//! `questline`: the dashboard (default), the [`user`] commands for taking a quest and the
//! [`dev`] commands for writing quests, scripts and CI, all thin wrappers over `questline-core`.

mod dev;
mod tui;
mod user;

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::CommandFactory;
use clap::FromArgMatches;
use clap::Parser;
use clap::Subcommand;
use questline_core::Questline;

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
    #[command(flatten)]
    User(user::Command),
    #[command(flatten)]
    Dev(dev::Command),
}

fn main() -> ExitCode {
    let cli = Cli::from_arg_matches(&cli().get_matches()).unwrap_or_else(|e| e.exit());
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// [`Cli`] with the dev commands listed apart, after the user commands: clap has a single
/// `Commands:` section.
fn cli() -> clap::Command {
    let mut cli = Cli::command();
    let (header, literal) = (
        *cli.get_styles().get_header(),
        *cli.get_styles().get_literal(),
    );
    let mut help = format!("{header}Dev commands:{header:#} writing quests, scripts and CI");
    for dev in dev::Command::augment_subcommands(clap::Command::new("dev")).get_subcommands() {
        let about = dev.get_about().map(ToString::to_string).unwrap_or_default();
        help += &format!("\n  {literal}{:<10}{literal:#}{about}", dev.get_name());
        cli = cli.mut_subcommand(dev.get_name(), |c| c.hide(true));
    }
    cli.after_help(help)
}

fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    let ql = || -> anyhow::Result<Questline> { Ok(Questline::open(find_root(cli.root)?)?) };
    match cli.command.unwrap_or(Command::Tui) {
        Command::Tui => tui::run(ql()?, cli.theme).map(|()| ExitCode::SUCCESS),
        Command::User(command) => user::run(command, ql),
        Command::Dev(command) => dev::run(command, ql()?),
    }
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
