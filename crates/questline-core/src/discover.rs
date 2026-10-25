//! Scanning `quests/` and checking each quest against the static part of the contract.

use std::path::Path;
use std::path::PathBuf;

use questkit::ModuleManifest;
use questkit::Progress;
use questkit::QuestManifest;
use questkit::fingerprint::fingerprint;
use questkit::manifest::MODULE_MANIFEST;
use questkit::manifest::MODULES_DIR;
use questkit::manifest::QUEST_MANIFEST;
use questkit::manifest::is_valid_id;

use crate::model::Module;
use crate::model::ModuleKey;
use crate::model::Quest;

pub const QUESTS_DIR: &str = "quests";

/// Module ids that cannot be Cargo package names.
const RESERVED_MODULE_IDS: &[&str] = &["alloc", "core", "proc-macro", "std", "test"];

/// Every quest directory under `<root>/quests/`, sorted by id. Never fails as a whole: a broken
/// quest is returned with its problems.
pub fn scan(root: &Path) -> std::io::Result<Vec<Quest>> {
    let quests = subdirs(&root.join(QUESTS_DIR))?
        .into_iter()
        .map(|dir| load_quest(&dir))
        .collect();
    Ok(quests)
}

/// One quest directory (which need not be under a questline root, e.g. the skill's examples).
pub fn load_quest(dir: &Path) -> Quest {
    let id = dir_name(dir);
    let mut problems = Vec::new();
    if !is_valid_id(&id) {
        problems.push(format!(
            "directory name '{id}' is not a lowercase kebab-case id"
        ));
    }
    let manifest = QuestManifest::load(dir)
        .map_err(|e| problems.push(format!("{QUEST_MANIFEST}: {e:#}")))
        .ok();
    check_workspace(dir, &mut problems);

    let modules_dir = dir.join(MODULES_DIR);
    let mut modules: Vec<Module> = match subdirs(&modules_dir) {
        Ok(dirs) => dirs.iter().map(|m| load_module(&id, m)).collect(),
        Err(_) => Vec::new(),
    };
    if modules.is_empty() {
        problems.push(format!(
            "no modules: add one crate per module under {MODULES_DIR}/"
        ));
    }
    modules.sort_by(|a, b| (a.order(), &a.key.module).cmp(&(b.order(), &b.key.module)));
    Quest {
        id,
        path: dir.to_path_buf(),
        manifest,
        problems,
        modules,
    }
}

fn load_module(quest: &str, dir: &Path) -> Module {
    let id = dir_name(dir);
    let mut problems = Vec::new();
    if !is_valid_id(&id) {
        problems.push(format!(
            "directory name '{id}' is not a lowercase kebab-case id"
        ));
    } else if RESERVED_MODULE_IDS.contains(&id.as_str()) {
        problems.push(format!(
            "'{id}' is a built-in crate name; pick another module id"
        ));
    }
    let manifest = ModuleManifest::load(dir)
        .map_err(|e| problems.push(format!("{MODULE_MANIFEST}: {e:#}")))
        .ok();
    check_module_files(dir, manifest.as_ref(), &mut problems);
    Module {
        key: ModuleKey::new(quest, id),
        path: dir.to_path_buf(),
        fingerprint: fingerprint(dir).unwrap_or_default(),
        // A corrupt progress file is as good as none: the next check rewrites it.
        progress: Progress::load(dir).ok().flatten(),
        manifest,
        problems,
    }
}

fn check_workspace(dir: &Path, problems: &mut Vec<String>) {
    let Some(cargo) = read_toml(&dir.join("Cargo.toml"), problems) else {
        return;
    };
    let members = cargo
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array());
    let glob = format!("{MODULES_DIR}/*");
    if !members.is_some_and(|m| m.iter().any(|v| v.as_str() == Some(glob.as_str()))) {
        problems.push(format!(
            "Cargo.toml: [workspace] members must include \"{glob}\""
        ));
    }
}

fn check_module_files(dir: &Path, manifest: Option<&ModuleManifest>, problems: &mut Vec<String>) {
    if let Some(cargo) = read_toml(&dir.join("Cargo.toml"), problems) {
        if cargo
            .get("features")
            .and_then(|f| f.get("solutions"))
            .is_none()
        {
            problems.push("Cargo.toml: missing feature `solutions = []`".into());
        }
        if cargo
            .get("dependencies")
            .and_then(|d| d.get("questkit"))
            .is_none()
        {
            problems.push("Cargo.toml: missing dependency `questkit.workspace = true`".into());
        }
    }
    let mut required = vec![
        "src/lib.rs".to_string(),
        "tests/exercises/main.rs".to_string(),
    ];
    for lesson in manifest.map(|m| m.lessons.as_slice()).unwrap_or_default() {
        required.extend([
            lesson.lesson_file(),
            lesson.exercise_file(),
            lesson.solution_file(),
            lesson.test_file(),
        ]);
    }
    problems.extend(
        required
            .iter()
            .filter(|f| !dir.join(f).is_file())
            .map(|f| format!("missing {f}")),
    );
    match std::fs::read_to_string(dir.join("src/main.rs")) {
        Ok(main) if main.contains("questkit::run") => {}
        Ok(_) => problems.push("src/main.rs does not call questkit::run".into()),
        Err(_) => problems.push("missing src/main.rs".into()),
    }
}

fn read_toml(path: &Path, problems: &mut Vec<String>) -> Option<toml::Table> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => {
            problems.push(format!("missing {name}"));
            return None;
        }
    };
    text.parse()
        .map_err(|e| problems.push(format!("{name}: {e}")))
        .ok()
}

/// Non-hidden subdirectories, sorted.
fn subdirs(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_type().is_ok_and(|t| t.is_dir())
                && !e.file_name().to_string_lossy().starts_with('.')
        })
        .map(|e| e.path())
        .collect();
    dirs.sort();
    Ok(dirs)
}

fn dir_name(dir: &Path) -> String {
    dir.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::fs;

    /// A minimal valid quest with one module of `lessons` lessons.
    pub(crate) fn write_quest(root: &Path, quest: &str, module: &str, lessons: u8) -> PathBuf {
        let q = root.join(QUESTS_DIR).join(quest);
        let m = q.join(MODULES_DIR).join(module);
        let files = [
            (
                "quest.toml",
                "contract = 1\ntitle = \"Quest\"\nsummary = \"s\"\n".to_string(),
            ),
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"modules/*\"]\n".to_string(),
            ),
        ];
        for (name, text) in files {
            fs::create_dir_all(&q).unwrap();
            fs::write(q.join(name), text).unwrap();
        }
        let mut manifest = "title = \"Module\"\nsummary = \"s\"\norder = 1\n".to_string();
        let mut paths = vec![
            "src/lib.rs".to_string(),
            "tests/exercises/main.rs".to_string(),
        ];
        for n in 1..=lessons {
            manifest += &format!("\n[[lesson]]\nnumber = {n}\nname = \"l{n}\"\nsummary = \"s\"\n");
            paths.extend([
                format!("src/lessons/l{n:02}_l{n}.rs"),
                format!("src/exercises/e{n:02}_l{n}.rs"),
                format!("solutions/e{n:02}_l{n}.rs"),
                format!("tests/exercises/e{n:02}.rs"),
            ]);
        }
        for p in paths {
            fs::create_dir_all(m.join(&p).parent().unwrap()).unwrap();
            fs::write(m.join(p), "").unwrap();
        }
        fs::write(m.join("module.toml"), manifest).unwrap();
        fs::write(
            m.join("Cargo.toml"),
            "[dependencies]\nquestkit.workspace = true\n[features]\nsolutions = []\n",
        )
        .unwrap();
        fs::write(m.join("src/main.rs"), "fn main() { questkit::run(..) }").unwrap();
        m
    }

    #[test]
    fn a_complete_quest_has_no_problems() {
        let root = tempfile::tempdir().unwrap();
        write_quest(root.path(), "arrow", "basics", 2);
        let quests = scan(root.path()).unwrap();
        assert_eq!(quests.len(), 1);
        assert_eq!(quests[0].problems, Vec::<String>::new());
        assert_eq!(quests[0].modules[0].problems, Vec::<String>::new());
        assert_eq!(quests[0].modules[0].key.to_string(), "arrow/basics");
        assert_eq!(quests[0].modules[0].fingerprint.len(), 16);
    }

    #[test]
    fn contract_violations_are_reported_per_file() {
        let root = tempfile::tempdir().unwrap();
        let m = write_quest(root.path(), "arrow", "basics", 2);
        fs::remove_file(m.join("solutions/e02_l2.rs")).unwrap();
        fs::write(m.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(
            m.parent().unwrap().parent().unwrap().join("Cargo.toml"),
            "[workspace]\nmembers = []",
        )
        .unwrap();
        let quest = load_quest(&root.path().join("quests/arrow"));
        assert_eq!(
            quest.problems,
            ["Cargo.toml: [workspace] members must include \"modules/*\""]
        );
        assert_eq!(
            quest.modules[0].problems,
            [
                "missing solutions/e02_l2.rs",
                "src/main.rs does not call questkit::run"
            ]
        );
    }

    #[test]
    fn modules_are_ordered_by_manifest_order() {
        let root = tempfile::tempdir().unwrap();
        let a = write_quest(root.path(), "q", "a-first-by-name", 1);
        write_quest(root.path(), "q", "b-first-by-order", 1);
        let manifest = fs::read_to_string(a.join("module.toml"))
            .unwrap()
            .replace("order = 1", "order = 2");
        fs::write(a.join("module.toml"), manifest).unwrap();
        let quest = load_quest(&root.path().join("quests/q"));
        let ids: Vec<_> = quest
            .modules
            .iter()
            .map(|m| m.key.module.as_str())
            .collect();
        assert_eq!(ids, ["b-first-by-order", "a-first-by-name"]);
    }
}
