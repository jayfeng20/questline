//! Running a module's exercise tests and recording the result.
//!
//! Two steps, so that the test process is ours to time out and kill: build the `exercises`
//! test target with cargo (`--no-run`, JSON messages name the binary), then run that binary.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;

use crate::Outcome;
use crate::Progress;
use crate::fingerprint::fingerprint;
use crate::libtest;
use crate::progress::write_atomic;

/// The exercise suite is fast by contract (timing tests stay under ~200 ms); anything slower
/// is a hung exercise.
pub const TEST_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stream {
    Stdout,
    Stderr,
}

/// Builds and runs `module_dir`'s exercise tests, writes `.quest/progress.json` and
/// `.quest/check.log`, and returns the new progress. `on_line` sees every output line as it
/// arrives (cargo's build output on stderr, the test output on stdout).
///
/// `Err` only when the check itself could not run (no cargo, unwritable state dir); a failed
/// build is an `Ok` progress with [`Outcome::BuildFailed`].
pub fn evaluate(
    module_dir: &Path,
    mut on_line: impl FnMut(Stream, &str),
) -> anyhow::Result<Progress> {
    let fingerprint =
        fingerprint(module_dir).with_context(|| format!("fingerprint {}", module_dir.display()))?;
    let mut log = Vec::new();
    let mut tee = |stream: Stream, line: &str| {
        log.push(line.to_string());
        on_line(stream, line);
    };

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut build = Command::new(cargo);
    without_cargo_run_env(&mut build);
    let build = build
        .args([
            "test",
            "--test",
            "exercises",
            "--no-run",
            "--color",
            "never",
        ])
        .args([
            "--message-format",
            "json-render-diagnostics",
            "--manifest-path",
        ])
        .arg(module_dir.join("Cargo.toml"))
        .current_dir(module_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("run cargo (is it on PATH?)")?;
    // Cargo's stdout is JSON messages: keep them out of the log.
    let mut json = Vec::new();
    let built = stream(build, None, |s, line| match s {
        Stream::Stdout => json.push(line.to_string()),
        Stream::Stderr => tee(s, line),
    })?;
    let executable = test_executable(&json);

    let previous = Progress::load(module_dir).ok().flatten();
    let progress = match executable.filter(|_| built.status.success()) {
        None => {
            let mut p = Progress::new(
                fingerprint,
                Outcome::BuildFailed,
                previous.map(|p| p.exercises).unwrap_or_default(),
            );
            p.message = Some(
                first_error(&log)
                    .unwrap_or_else(|| format!("cargo test --no-run failed ({})", built.status)),
            );
            p
        }
        Some(exe) => {
            let test = Command::new(&exe)
                .args(["--color", "never"])
                .current_dir(module_dir)
                .env("CARGO_MANIFEST_DIR", module_dir)
                .env("RUST_BACKTRACE", "0")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .with_context(|| format!("run {}", exe.display()))?;
            let mut stdout = String::new();
            let ran = stream(test, Some(TEST_TIMEOUT), |s, line| {
                if s == Stream::Stdout {
                    stdout.push_str(line);
                    stdout.push('\n');
                }
                tee(s, line);
            })?;
            match libtest::parse(&stdout) {
                None => {
                    let mut p = Progress::new(fingerprint, Outcome::BuildFailed, Vec::new());
                    p.message = Some(format!("the test binary did not start ({})", ran.status));
                    p
                }
                Some(run) => {
                    let reported = run.results.len();
                    let mut p = Progress::new(fingerprint, Outcome::Tested, run.results);
                    if ran.timed_out {
                        p.message = Some(format!(
                            "tests were stopped after {} s with {reported} of {} reported: an exercise hangs",
                            TEST_TIMEOUT.as_secs(),
                            run.announced
                        ));
                    } else if reported < run.announced {
                        p.message = Some(format!(
                            "the test binary exited after {reported} of {} tests ({})",
                            run.announced, ran.status
                        ));
                    }
                    p
                }
            }
        }
    };

    progress.save(module_dir)?;
    write_atomic(
        &Progress::log_path(module_dir),
        (log.join("\n") + "\n").as_bytes(),
    )?;
    Ok(progress)
}

struct Finished {
    status: ExitStatus,
    timed_out: bool,
}

/// Drops the variables `cargo run` sets for the binary it runs (this process, under
/// `cargo run -- check`), so the child cargo sees the same environment as one started from a
/// shell. Otherwise build scripts that track one (ring tracks `CARGO_MANIFEST_DIR`) rebuild
/// on every switch between `check` and a plain `cargo test`.
fn without_cargo_run_env(cmd: &mut Command) {
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy();
        if name.starts_with("CARGO_PKG_")
            || matches!(
                &*name,
                "CARGO"
                    | "CARGO_MANIFEST_DIR"
                    | "CARGO_MANIFEST_PATH"
                    | "CARGO_CRATE_NAME"
                    | "CARGO_BIN_NAME"
                    | "CARGO_PRIMARY_PACKAGE"
            )
        {
            cmd.env_remove(&key);
        }
    }
}

/// Forwards both pipes line by line until the child exits, killing it after `timeout`.
fn stream(
    mut child: Child,
    timeout: Option<Duration>,
    mut on_line: impl FnMut(Stream, &str),
) -> anyhow::Result<Finished> {
    let (tx, rx) = mpsc::channel();
    let readers = [
        child
            .stdout
            .take()
            .map(|r| pump(r, Stream::Stdout, tx.clone())),
        child
            .stderr
            .take()
            .map(|r| pump(r, Stream::Stderr, tx.clone())),
    ];
    drop(tx);
    let deadline = timeout.map(|t| Instant::now() + t);
    let mut timed_out = false;
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok((s, line)) => on_line(s, &line),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if !timed_out && deadline.is_some_and(|d| Instant::now() > d) {
                    timed_out = true;
                    child.kill().ok();
                }
            }
        }
    }
    for reader in readers.into_iter().flatten() {
        reader.join().ok();
    }
    Ok(Finished {
        status: child.wait()?,
        timed_out,
    })
}

fn pump(
    reader: impl Read + Send + 'static,
    stream: Stream,
    tx: mpsc::Sender<(Stream, String)>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else { break };
            if tx.send((stream, line)).is_err() {
                break;
            }
        }
    })
}

/// The `exercises` test binary from cargo's `compiler-artifact` messages.
fn test_executable(json_lines: &[String]) -> Option<PathBuf> {
    json_lines.iter().rev().find_map(|line| {
        let msg: serde_json::Value = serde_json::from_str(line).ok()?;
        let target = msg.get("target")?;
        let is_exercises = msg.get("reason")?.as_str()? == "compiler-artifact"
            && target.get("name")?.as_str()? == "exercises"
            && target
                .get("kind")?
                .as_array()?
                .iter()
                .any(|k| k.as_str() == Some("test"));
        is_exercises.then(|| msg.get("executable")?.as_str().map(PathBuf::from))?
    })
}

fn first_error(log: &[String]) -> Option<String> {
    let i = log.iter().position(|l| l.starts_with("error"))?;
    // rustc puts the location on the line after `error[E..]: message`.
    let location = log
        .get(i + 1)
        .map(|l| l.trim())
        .filter(|l| l.starts_with("-->"));
    Some(match location {
        Some(loc) => format!("{} {loc}", log[i]),
        None => log[i].clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_exercises_binary_among_artifacts() {
        let lines = [
            r#"{"reason":"compiler-artifact","target":{"name":"anyhow","kind":["lib"]},"executable":null}"#,
            r#"{"reason":"compiler-artifact","target":{"name":"exercises","kind":["test"]},"executable":"/t/debug/deps/exercises-1a2b"}"#,
            r#"{"reason":"build-finished","success":true}"#,
        ]
        .map(String::from);
        assert_eq!(
            test_executable(&lines),
            Some(PathBuf::from("/t/debug/deps/exercises-1a2b"))
        );
        assert_eq!(test_executable(&lines[..1]), None);
    }

    #[test]
    fn first_error_includes_the_location() {
        let log = [
            "   Compiling x v0.1.0",
            "error[E0308]: mismatched types",
            "  --> src/exercises/e01_a.rs:3:5",
            "...",
        ]
        .map(String::from);
        assert_eq!(
            first_error(&log).unwrap(),
            "error[E0308]: mismatched types --> src/exercises/e01_a.rs:3:5"
        );
    }
}
