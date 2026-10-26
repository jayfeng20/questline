# Placement — which quest a new module goes into

One quest per subject, so modules that share dependencies share one workspace (one `Cargo.lock`,
one `target/`, one set of pins): `/questify arrow-parquet` belongs in the existing `arrow-rs` quest
as a module, not in a new quest that compiles arrow a second time.

## 1. Gather

```bash
scripts/quests.sh        # every quest: title, summary, tags, example flag, workspace pins, modules
```

## 2. Decide

For the topic, name its **primary crates** (e.g. `arrow-parquet` → `parquet`, `arrow`;
`tokio channels` → `tokio`) and its **subject** (arrow-rs, tokio, serde, rust std, ...).

Skip quests with `example = true`. A quest **matches** when any of these holds:

1. a primary crate is already in its `[workspace.dependencies]`, or is in the same crate family
   as one there (`parquet`, `arrow-*`, `arrow-flight` are the arrow-rs family; `tokio-*` are
   tokio's);
2. the topic is a sub-area of its subject or appears in its `tags`;
3. its README course map lists the topic as a module not generated yet (then reuse that row's
   module id and `order`).

Exactly one match → **place it there**. Several → the one with the most shared primary crates;
still tied → treat as ambiguous. None → **a new quest**, named after the subject (`tokio`,
`serde`, `rust-std`), never after the narrow topic, so later modules on the subject join it.

The module id is the sub-topic (`parquet-writing`, `channels`), unique within the quest. It
must not be the name of a crate the module depends on (`lance` cannot depend on `lance`), nor a
built-in crate name (`std`, `core`, `alloc`, `test`, `proc-macro`).

Pins: reuse the quest's pin for every crate it already has. If the topic needs a different
major of a pinned crate (the quest has `arrow = "=58"`, the topic needs 60), do not bump the
pin: ask the user (bump the quest and re-verify its modules, pin locally in the module, or use
a new quest).

## 3. Act, by mode

| mode | flag | match found | no match | ambiguous |
|------|------|-------------|----------|-----------|
| **autosort** (default) | `--auto` or none | place it, then say where and why in one line | create the quest, say so | ask (as manual) |
| **manualsort** | `--manual` | ask before creating anything | ask before creating anything | ask |

When asking (use your question tool if you have one), offer the concrete choices with your
recommendation first:

```
"arrow-parquet" fits the arrow-rs quest (it already pins parquet 60.0.0 and arrow 60.0.0).
  1. arrow-rs / parquet-writing  (Recommended: shares pins and target/)
  2. a new quest "parquet"
  3. another existing quest: <id>
```

For a new quest also confirm the quest id and title. Then scaffold:

```bash
scripts/scaffold.sh quest <quest>              # only for a new quest; fill quest.toml
scripts/scaffold.sh module <quest> <module>    # picks the next free `order`
```
