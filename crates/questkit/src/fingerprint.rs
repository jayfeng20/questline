//! A cheap identity for "the inputs a check would see", used to skip checks whose result is
//! already known.
//!
//! It hashes the path, size and mtime (not the content) of every file in the module directory
//! plus the quest's workspace `Cargo.toml`. Skipped: `target/`, `course-data/` (lesson output),
//! dot-files and dot-directories (`.quest/`, `.DS_Store`), and `Cargo.lock`, which cargo may
//! rewrite during the check itself. A false "changed" costs one extra check; a false
//! "unchanged" would need a file edited without its size or mtime changing.

use std::path::Path;
use std::time::UNIX_EPOCH;

const SKIPPED_DIRS: &[&str] = &["target", "course-data"];

/// Fingerprint of `module_dir` (`<quest>/modules/<module>`), as 16 hex digits.
pub fn fingerprint(module_dir: &Path) -> std::io::Result<String> {
    let mut entries = Vec::new();
    walk(module_dir, module_dir, &mut entries)?;
    if let Some(quest_dir) = module_dir.parent().and_then(Path::parent) {
        let workspace = quest_dir.join("Cargo.toml");
        if workspace.exists() {
            entries.push(entry("../../Cargo.toml".to_string(), &workspace)?);
        }
    }
    entries.sort();
    let mut hash = Fnv::new();
    for e in &entries {
        hash.write(e.as_bytes());
        hash.write(b"\n");
    }
    Ok(format!("{:016x}", hash.0))
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
    for item in std::fs::read_dir(dir)? {
        let item = item?;
        let name = item.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        let path = item.path();
        let kind = item.file_type()?;
        if kind.is_dir() {
            if dir == root && SKIPPED_DIRS.contains(&name.as_ref()) {
                continue;
            }
            walk(root, &path, out)?;
        } else if name != "Cargo.lock" {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push(entry(rel, &path)?);
        }
    }
    Ok(())
}

fn entry(rel: String, path: &Path) -> std::io::Result<String> {
    let meta = std::fs::metadata(path)?;
    let mtime = meta
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(format!("{rel}\0{}\0{mtime}", meta.len()))
}

/// FNV-1a, 64-bit: stable across Rust versions, unlike `DefaultHasher`.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn changes_with_sources_but_not_with_build_output() {
        let quest = tempfile::tempdir().unwrap();
        let module = quest.path().join("modules/basics");
        fs::create_dir_all(module.join("src/exercises")).unwrap();
        fs::write(quest.path().join("Cargo.toml"), "[workspace]").unwrap();
        fs::write(module.join("src/exercises/e01_x.rs"), "todo!()").unwrap();
        let before = fingerprint(&module).unwrap();
        assert_eq!(before.len(), 16);

        for ignored in [
            "target/debug/x",
            "course-data/out.bin",
            ".quest/progress.json",
            "Cargo.lock",
        ] {
            let path = module.join(ignored);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "noise").unwrap();
        }
        assert_eq!(
            fingerprint(&module).unwrap(),
            before,
            "build output and state do not count"
        );

        fs::write(module.join("src/exercises/e01_x.rs"), "42 // solved").unwrap();
        let edited = fingerprint(&module).unwrap();
        assert_ne!(edited, before, "an exercise edit changes the fingerprint");

        fs::write(quest.path().join("Cargo.toml"), "[workspace]\nmembers = []").unwrap();
        assert_ne!(
            fingerprint(&module).unwrap(),
            edited,
            "workspace dependencies count"
        );
    }
}
