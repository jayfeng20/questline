//! Learning order: moving a quest or module and renumbering its siblings' `order`.

use std::path::Path;
use std::path::PathBuf;

use questkit::manifest::MODULE_MANIFEST;
use questkit::manifest::QUEST_MANIFEST;

use crate::Error;
use crate::Result;
use crate::model::Quest;
use crate::model::Target;

pub fn set_order(quests: &[Quest], target: &Target, position: u32) -> Result<()> {
    // (id, manifest, order) in listed order. Siblings with a broken manifest cannot be
    // rewritten; they keep sorting last.
    let (id, mut siblings): (&str, Vec<(&str, PathBuf, u32)>) = match target {
        Target::All => return Err(Error::Invalid("order a quest or a module, not all".into())),
        Target::Quest(id) => {
            let example = quests
                .iter()
                .find(|q| q.id == *id)
                .and_then(|q| q.manifest.as_ref())
                .map(|m| m.example);
            let siblings = quests
                .iter()
                .filter_map(|q| {
                    let m = q.manifest.as_ref().filter(|m| Some(m.example) == example)?;
                    Some((q.id.as_str(), q.path.join(QUEST_MANIFEST), m.order))
                })
                .collect();
            (id, siblings)
        }
        Target::Module(key) => {
            let siblings = quests
                .iter()
                .filter(|q| q.id == key.quest)
                .flat_map(|q| &q.modules)
                .filter_map(|m| {
                    let order = m.manifest.as_ref()?.order;
                    Some((m.key.module.as_str(), m.path.join(MODULE_MANIFEST), order))
                })
                .collect();
            (&key.module, siblings)
        }
    };
    let Some(from) = siblings.iter().position(|(s, ..)| *s == id) else {
        return Err(Error::NotFound(format!(
            "no '{}' with a valid manifest",
            match target {
                Target::Module(key) => key.to_string(),
                _ => id.to_string(),
            }
        )));
    };
    let moved = siblings.remove(from);
    let to = (position.max(1) as usize - 1).min(siblings.len());
    siblings.insert(to, moved);
    for (n, (_, manifest, order)) in (1..).zip(&siblings) {
        if *order != n {
            write_order(manifest, n)?;
        }
    }
    Ok(())
}

/// Rewrites the top-level `order` value of a manifest, keeping its formatting and comments.
fn write_order(manifest: &Path, order: u32) -> Result<()> {
    let text = std::fs::read_to_string(manifest)
        .map_err(|e| Error::io(format!("read {}", manifest.display()), e))?;
    let mut doc: toml_edit::DocumentMut = text
        .parse()
        .map_err(|e| Error::Invalid(format!("{}: {e}", manifest.display())))?;
    let value = doc
        .get_mut("order")
        .and_then(toml_edit::Item::as_value_mut)
        .ok_or_else(|| Error::Invalid(format!("{}: no `order`", manifest.display())))?;
    let decor = value.decor().clone();
    *value = i64::from(order).into();
    *value.decor_mut() = decor;
    let tmp = manifest.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, doc.to_string())
        .map_err(|e| Error::io(format!("write {}", tmp.display()), e))?;
    std::fs::rename(&tmp, manifest)
        .map_err(|e| Error::io(format!("rename to {}", manifest.display()), e))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::Questline;
    use crate::discover::tests::write_quest;
    use crate::model::ModuleKey;

    fn set(path: &Path, from: &str, to: &str) {
        let text = fs::read_to_string(path).unwrap();
        assert!(text.contains(from), "{text}");
        fs::write(path, text.replace(from, to)).unwrap();
    }

    fn ids(ql: &Questline, quest: Option<&str>) -> Vec<String> {
        let s = ql.sync().unwrap();
        match quest {
            None => s.quests.iter().map(|q| q.id.clone()).collect(),
            Some(q) => s
                .quest(q)
                .unwrap()
                .modules
                .iter()
                .map(|m| m.key.module.clone())
                .collect(),
        }
    }

    #[test]
    fn moving_a_module_renumbers_its_siblings_and_keeps_comments() {
        let root = tempfile::tempdir().unwrap();
        for (i, m) in ["a", "b", "c"].iter().enumerate() {
            let dir = write_quest(root.path(), "q", m, 1);
            set(
                &dir.join("module.toml"),
                "order = 1",
                &format!("order = {}  # keep me", i + 1),
            );
        }
        let ql = Questline::open(root.path()).unwrap();
        let c = Target::Module(ModuleKey::new("q", "c"));

        ql.set_order(&c, 1).unwrap();
        assert_eq!(ids(&ql, Some("q")), ["c", "a", "b"]);
        let b = fs::read_to_string(root.path().join("quests/q/modules/b/module.toml")).unwrap();
        assert!(b.contains("order = 3  # keep me"), "{b}");

        ql.set_order(&c, 99).unwrap();
        assert_eq!(ids(&ql, Some("q")), ["a", "b", "c"]);
        assert!(
            ql.set_order(&Target::Module(ModuleKey::new("q", "z")), 1)
                .is_err()
        );
        assert!(ql.set_order(&Target::All, 1).is_err());
    }

    #[test]
    fn quests_are_ordered_apart_from_examples() {
        let root = tempfile::tempdir().unwrap();
        for id in ["ex", "one", "two"] {
            write_quest(root.path(), id, "m", 1);
        }
        let ex = root.path().join("quests/ex/quest.toml");
        set(&ex, "order = 1", "order = 1\nexample = true");
        let ql = Questline::open(root.path()).unwrap();
        assert_eq!(ids(&ql, None), ["one", "two", "ex"]);

        ql.set_order(&Target::Quest("two".into()), 1).unwrap();
        assert_eq!(ids(&ql, None), ["two", "one", "ex"]);
        assert!(fs::read_to_string(&ex).unwrap().contains("order = 1"));
    }
}
