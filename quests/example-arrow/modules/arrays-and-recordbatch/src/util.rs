//! Shared helpers: printing, checked claims, fixtures, measurement.

use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

/// Directory for files lessons write (gitignored).
pub fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("course-data");
    std::fs::create_dir_all(&dir).expect("create course-data dir");
    dir
}

/// Path to a downloaded fixture in `./data/`; the error says how to get it.
pub fn fixture(name: &str) -> anyhow::Result<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data").join(name);
    anyhow::ensure!(path.exists(), "fixture {} is missing: run `scripts/fetch-data.sh`", path.display());
    Ok(path)
}

pub fn banner(title: &str) {
    println!("\n{}\n  {title}\n{}", "=".repeat(88), "=".repeat(88));
}

pub fn section(title: &str) {
    println!("\n--- {title} {}", "-".repeat(84usize.saturating_sub(title.len())));
}

/// An explanation printed next to the output it explains.
pub fn note(msg: &str) {
    println!("   ℹ  {msg}");
}

/// An experiment for the learner: change something, re-run, compare.
pub fn try_it(msg: &str) {
    println!("   🧪 TRY: {msg}");
}

/// A claim the lesson makes, verified while it runs. A false claim fails the lesson.
pub fn check(holds: bool, claim: &str) -> anyhow::Result<()> {
    anyhow::ensure!(holds, "lesson claim does not hold: {claim}");
    println!("   ✅ {claim}");
    Ok(())
}

/// Best wall-clock time of `runs` runs. Compare ratios, not absolute numbers.
pub fn best_of<T>(runs: usize, mut f: impl FnMut() -> T) -> (T, Duration) {
    let mut best: Option<(T, Duration)> = None;
    for _ in 0..runs {
        let start = Instant::now();
        let value = f();
        let elapsed = start.elapsed();
        if best.as_ref().is_none_or(|(_, d)| elapsed < *d) {
            best = Some((value, elapsed));
        }
    }
    best.expect("runs >= 1")
}

pub fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let (mut v, mut u) = (n as f64, 0);
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 { format!("{n} B") } else { format!("{v:.2} {}", UNITS[u]) }
}

/// Runs `f` and returns its panic message, or `None` if it did not panic. The default panic
/// hook is muted meanwhile, so the expected panic does not clutter the lesson output.
pub fn panic_message<T>(f: impl FnOnce() -> T) -> Option<String> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::panic::set_hook(hook);
    let payload = result.err()?;
    let msg = payload.downcast_ref::<String>().cloned();
    Some(msg.or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default())
}
