//! `quest.toml` and `module.toml`.

use std::path::Path;

use anyhow::Context;
use anyhow::ensure;
use serde::Deserialize;
use serde::Serialize;

/// Version of the quest contract this crate implements; `quest.toml` must declare it.
pub const CONTRACT_VERSION: u32 = 1;
pub const QUEST_MANIFEST: &str = "quest.toml";
pub const MODULE_MANIFEST: &str = "module.toml";
/// Directory under the quest root that holds one member crate per module.
pub const MODULES_DIR: &str = "modules";

/// Lesson names that would shadow a runner command.
const RESERVED_LESSON_NAMES: &[&str] = &["all", "check", "help", "list"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestManifest {
    pub contract: u32,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// A published sample; `/questify` autosort never adds modules to it.
    #[serde(default)]
    pub example: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleManifest {
    pub title: String,
    pub summary: String,
    /// Position within the quest; modules are listed by ascending `order`.
    pub order: u32,
    /// Module ids (same quest) whose vocabulary this module assumes.
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(rename = "lesson")]
    pub lessons: Vec<Lesson>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lesson {
    pub number: u8,
    /// snake_case; names the lesson, exercise and solution files.
    pub name: String,
    pub summary: String,
}

impl QuestManifest {
    pub fn load(quest_dir: &Path) -> anyhow::Result<Self> {
        let manifest: Self = read_toml(&quest_dir.join(QUEST_MANIFEST))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.contract == CONTRACT_VERSION,
            "contract = {} is not supported (this questline speaks contract {CONTRACT_VERSION})",
            self.contract
        );
        ensure!(!self.title.trim().is_empty(), "title is empty");
        Ok(())
    }
}

impl ModuleManifest {
    pub fn load(module_dir: &Path) -> anyhow::Result<Self> {
        let manifest: Self = read_toml(&module_dir.join(MODULE_MANIFEST))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(!self.title.trim().is_empty(), "title is empty");
        ensure!(!self.lessons.is_empty(), "no [[lesson]] entries");
        for (i, lesson) in self.lessons.iter().enumerate() {
            ensure!(
                usize::from(lesson.number) == i + 1,
                "lessons must be numbered 1, 2, 3, ... in order; entry {} has number {}",
                i + 1,
                lesson.number
            );
            ensure!(
                is_snake_case(&lesson.name),
                "lesson {}: name '{}' is not snake_case",
                lesson.number,
                lesson.name
            );
            ensure!(
                !RESERVED_LESSON_NAMES.contains(&lesson.name.as_str()),
                "lesson {}: name '{}' is reserved for a runner command",
                lesson.number,
                lesson.name
            );
        }
        Ok(())
    }

    pub fn lesson(&self, number: u8) -> Option<&Lesson> {
        self.lessons.iter().find(|l| l.number == number)
    }

    /// A lesson by number (`3`) or by a substring of its name (`streams`).
    pub fn resolve(&self, arg: &str) -> Option<&Lesson> {
        if let Ok(n) = arg.parse::<u8>() {
            return self.lesson(n);
        }
        let needle = arg.to_ascii_lowercase();
        self.lessons.iter().find(|l| l.name.contains(&needle))
    }
}

impl Lesson {
    pub fn lesson_file(&self) -> String {
        format!("src/lessons/l{:02}_{}.rs", self.number, self.name)
    }

    pub fn exercise_file(&self) -> String {
        format!("src/exercises/e{:02}_{}.rs", self.number, self.name)
    }

    pub fn solution_file(&self) -> String {
        format!("solutions/e{:02}_{}.rs", self.number, self.name)
    }

    pub fn test_file(&self) -> String {
        format!("tests/exercises/e{:02}.rs", self.number)
    }
}

/// Quest and module ids: lowercase kebab-case, as used for directory names.
pub fn is_valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.ends_with('-')
}

fn is_snake_case(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn read_toml<T: serde::de::DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODULE: &str = r#"
title = "Streams"
summary = "RecordBatch streams"
order = 2
requires = ["basics"]

[[lesson]]
number = 1
name = "batches"
summary = "one"

[[lesson]]
number = 2
name = "bounded_memory"
summary = "two"
"#;

    #[test]
    fn module_manifest_parses_and_resolves_lessons() {
        let m: ModuleManifest = toml::from_str(MODULE).unwrap();
        m.validate().unwrap();
        assert_eq!(m.lessons.len(), 2);
        assert_eq!(m.resolve("2").unwrap().name, "bounded_memory");
        assert_eq!(m.resolve("memory").unwrap().number, 2);
        assert!(m.resolve("9").is_none());
        assert_eq!(
            m.lessons[1].exercise_file(),
            "src/exercises/e02_bounded_memory.rs"
        );
        assert_eq!(m.lessons[1].test_file(), "tests/exercises/e02.rs");
    }

    #[test]
    fn module_manifest_rejects_gaps_and_reserved_names() {
        let mut m: ModuleManifest = toml::from_str(MODULE).unwrap();
        m.lessons[1].number = 3;
        assert!(m.validate().unwrap_err().to_string().contains("numbered"));
        m.lessons[1].number = 2;
        m.lessons[1].name = "check".into();
        assert!(m.validate().unwrap_err().to_string().contains("reserved"));
    }

    #[test]
    fn quest_manifest_requires_the_current_contract() {
        let q: QuestManifest =
            toml::from_str("contract = 2\ntitle = \"t\"\nsummary = \"s\"").unwrap();
        assert!(q.validate().is_err());
        let unknown = toml::from_str::<QuestManifest>(
            "contract = 1\ntitle = \"t\"\nsummary = \"s\"\nowner = \"x\"",
        );
        assert!(unknown.is_err(), "unknown keys are rejected");
    }

    #[test]
    fn ids_are_kebab_case() {
        assert!(is_valid_id("arrow-rs"));
        assert!(is_valid_id("tokio2"));
        assert!(!is_valid_id("Arrow"));
        assert!(!is_valid_id("arrow_rs"));
        assert!(!is_valid_id("-x"));
        assert!(!is_valid_id("x-"));
        assert!(!is_valid_id(""));
    }
}
