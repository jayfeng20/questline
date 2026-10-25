//! Opening a directory in the user's IDE, as a new window.
//!
//! The IDE is `$QUESTLINE_IDE` (a command, e.g. `code -n` or `zed`), else the first known IDE
//! whose CLI is on `PATH`, else (macOS) the first known IDE installed in `/Applications`.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use serde::Serialize;

use crate::Error;

/// (CLI, macOS app name, arguments that open a new window).
const KNOWN: &[(&str, &str, &[&str])] = &[
    ("code", "Visual Studio Code", &["-n"]),
    ("cursor", "Cursor", &["-n"]),
    ("zed", "Zed", &["-n"]),
    ("windsurf", "Windsurf", &["-n"]),
    ("rustrover", "RustRover", &[]),
    ("subl", "Sublime Text", &["-n"]),
];

#[derive(Debug, Clone, Serialize)]
pub struct Launch {
    /// The command that was run, e.g. `zed -n`.
    pub editor: String,
    pub dir: PathBuf,
}

/// Opens `dir` as the window's root, with `files` (which must exist) opened in it.
pub fn open(dir: &Path, files: &[PathBuf]) -> Result<Launch, Error> {
    let (program, args) = detect().ok_or(Error::NoIde)?;
    let editor = std::iter::once(program.as_str())
        .chain(args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    let mut child = Command::new(&program)
        .args(&args)
        .arg(dir)
        .args(files)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Error::io(format!("run `{editor}`"), e))?;
    // IDE launchers return at once; reap them so they do not linger as zombies.
    std::thread::spawn(move || child.wait());
    Ok(Launch {
        editor,
        dir: dir.to_path_buf(),
    })
}

fn detect() -> Option<(String, Vec<String>)> {
    if let Ok(cmd) = std::env::var("QUESTLINE_IDE") {
        let mut words = cmd.split_whitespace().map(String::from);
        if let Some(program) = words.next() {
            return Some((program, words.collect()));
        }
    }
    let strings = |args: &[&str]| args.iter().map(|a| a.to_string()).collect();
    if let Some((cli, _, args)) = KNOWN.iter().find(|(cli, _, _)| on_path(cli)) {
        return Some((cli.to_string(), strings(args)));
    }
    if cfg!(target_os = "macos") {
        let (_, app, _) = KNOWN.iter().find(|(_, app, _)| {
            Path::new("/Applications")
                .join(format!("{app}.app"))
                .exists()
        })?;
        return Some(("open".into(), vec!["-a".into(), app.to_string()]));
    }
    None
}

fn on_path(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}
