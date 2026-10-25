//! Parses the human output of a libtest binary (the stable format; `--format json` is not).

use crate::ExerciseResult;
use crate::ExerciseStatus;

/// Lines of a panic message kept per failed exercise.
const MAX_MESSAGE_LINES: usize = 24;

#[derive(Debug, PartialEq)]
pub struct TestRun {
    /// From `running N tests`.
    pub announced: usize,
    pub results: Vec<ExerciseResult>,
}

/// `None` when the output never reached `running N tests` (the binary did not start).
pub fn parse(stdout: &str) -> Option<TestRun> {
    let mut announced = None;
    let mut results: Vec<ExerciseResult> = Vec::new();
    let mut lines = stdout.lines().peekable();
    while let Some(line) = lines.next() {
        if let Some(n) = line
            .strip_prefix("running ")
            .and_then(|r| r.split(' ').next())
        {
            if line.ends_with(" tests") || line.ends_with(" test") {
                announced = Some(announced.unwrap_or(0) + n.parse::<usize>().unwrap_or(0));
            }
        } else if let Some((name, status)) = line
            .strip_prefix("test ")
            .and_then(|r| r.split_once(" ... "))
        {
            let status = match status {
                "ok" => ExerciseStatus::Passed,
                "FAILED" => ExerciseStatus::Failed,
                s if s.starts_with("ignored") => ExerciseStatus::Ignored,
                _ => continue,
            };
            results.push(ExerciseResult {
                lesson: lesson_of(name),
                name: name.to_string(),
                status,
                message: None,
            });
        } else if let Some(name) = line
            .strip_prefix("---- ")
            .and_then(|r| r.strip_suffix(" stdout ----"))
        {
            let mut section = Vec::new();
            while let Some(next) = lines.peek() {
                if next.starts_with("---- ") || *next == "failures:" {
                    break;
                }
                section.push(lines.next().unwrap_or_default());
            }
            if let Some(result) = results.iter_mut().find(|r| r.name == name) {
                let message = panic_message(&section);
                if message.starts_with("not yet implemented") {
                    result.status = ExerciseStatus::Todo;
                }
                result.message = Some(message);
            }
        }
    }
    // Parallel tests report in completion order.
    results.sort_by(|a, b| (a.lesson, &a.name).cmp(&(b.lesson, &b.name)));
    Some(TestRun {
        announced: announced?,
        results,
    })
}

/// `e03::ex2_x` → 3.
fn lesson_of(name: &str) -> u8 {
    name.split("::")
        .next()
        .and_then(|m| m.strip_prefix('e'))
        .filter(|n| n.len() == 2)
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// The text after `thread '..' panicked at ..:`, up to the backtrace note; else the whole
/// section (a test returning `Err` prints `Error: ..` instead of panicking).
fn panic_message(section: &[&str]) -> String {
    // `thread 'e01::x' panicked at ..` (Rust 1.96 adds the thread id: `thread 'e01::x' (123) panicked at`).
    let start = section
        .iter()
        .position(|l| l.starts_with("thread '") && l.contains(" panicked at "));
    let body: Vec<&str> = match start {
        Some(i) => section[i + 1..]
            .iter()
            .take_while(|l| {
                !l.starts_with("note: run with `RUST_BACKTRACE")
                    && !l.starts_with("stack backtrace:")
            })
            .copied()
            .collect(),
        None => section.to_vec(),
    };
    let text = body.join("\n");
    let text = text.trim();
    let mut kept: Vec<&str> = text.lines().take(MAX_MESSAGE_LINES).collect();
    if text.lines().count() > MAX_MESSAGE_LINES {
        kept.push("…");
    }
    kept.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUTPUT: &str = "
running 4 tests
test e01::ex1_builds ... ok
test e01::ex2_nulls ... FAILED
test e02::ex1_stub ... FAILED
test e02::ex2_slow ... ignored, takes a minute

failures:

---- e01::ex2_nulls stdout ----

thread 'e01::ex2_nulls' panicked at tests/exercises/e01.rs:12:5:
assertion `left == right` failed: 3 nulls expected
  left: 2
 right: 3
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- e02::ex1_stub stdout ----

thread 'e02::ex1_stub' (18397211) panicked at src/exercises/e02_x.rs:4:5:
not yet implemented: ex1: build it
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    e01::ex2_nulls
    e02::ex1_stub

test result: FAILED. 1 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s
";

    #[test]
    fn classifies_passed_failed_todo_and_ignored() {
        let run = parse(OUTPUT).unwrap();
        assert_eq!(run.announced, 4);
        let statuses: Vec<_> = run
            .results
            .iter()
            .map(|r| (r.lesson, r.short_name(), r.status))
            .collect();
        assert_eq!(
            statuses,
            [
                (1, "ex1_builds", ExerciseStatus::Passed),
                (1, "ex2_nulls", ExerciseStatus::Failed),
                (2, "ex1_stub", ExerciseStatus::Todo),
                (2, "ex2_slow", ExerciseStatus::Ignored),
            ]
        );
        assert_eq!(
            run.results[1].message.as_deref(),
            Some("assertion `left == right` failed: 3 nulls expected\n  left: 2\n right: 3")
        );
        assert_eq!(
            run.results[2].message.as_deref(),
            Some("not yet implemented: ex1: build it")
        );
    }

    #[test]
    fn no_running_line_means_the_binary_never_ran() {
        assert_eq!(parse("error: could not compile `x`"), None);
    }

    #[test]
    fn err_returning_tests_keep_their_output() {
        let out = "running 1 test\ntest e01::ex1 ... FAILED\n\nfailures:\n\n---- e01::ex1 stdout ----\nError: file missing\n\nfailures:\n    e01::ex1\n";
        let run = parse(out).unwrap();
        assert_eq!(run.announced, 1);
        assert_eq!(
            run.results[0].message.as_deref(),
            Some("Error: file missing")
        );
        assert_eq!(run.results[0].status, ExerciseStatus::Failed);
    }
}
