# The quest contract (version 1)

What a directory must look like and do for questline to register it, show its progress and run
it. `questline validate` checks the static parts (§1–§3); the `check` command (§4) is exercised
by every evaluation. [`crates/questkit`](../crates/questkit) implements the quest side, so a
module that calls `questkit::run` gets §4 and §5 for free.

Terms: a **quest** is a course on one subject (e.g. arrow-rs) and a Cargo workspace; a
**module** is a sub-course of a quest (e.g. parquet-reading) and a member crate of that
workspace; a **lesson** is one runnable lecture in a module with its **exercises**, each checked
by one test.

## 1. Registration

A quest is registered by existing: every directory `quests/<quest>/` under the questline root
that contains a `quest.toml` is a quest. There is no other registry to update; the dashboard
picks a new quest up on its next sync (start-up or `ctrl-r`).

- Ids (`<quest>`, `<module>`) are directory names: lowercase kebab-case
  (`^[a-z0-9]([a-z0-9-]*[a-z0-9])?$`). A module id is also its Cargo package name, so it must
  not be a built-in crate name (`std`, `core`, `alloc`, `test`, `proc-macro`).
- A quest with problems is still listed, with status `Invalid` and its problems in `d`escribe.

## 2. Layout

```
quests/<quest>/
├── quest.toml                    # §3.1
├── Cargo.toml                    # [workspace] members = ["modules/*"], [workspace.dependencies]
├── README.md                     # course map: every module, generated or not
└── modules/<module>/             # one member crate per module
    ├── module.toml               # §3.2: the syllabus
    ├── Cargo.toml                # package `<module>`, feature `solutions`, dep `questkit`
    ├── README.md
    ├── .gitignore                # at least /.quest /course-data /data/*
    ├── src/main.rs               # calls questkit::run (§4)
    ├── src/lib.rs                # pub mod lessons; pub mod exercises; ...
    ├── src/lessons/lNN_<name>.rs       # one file per lesson
    ├── src/exercises/eNN_<name>.rs     # stubs (todo!()) the learner fills in
    ├── solutions/eNN_<name>.rs         # same API, solved; swapped in by `--features solutions`
    ├── tests/exercises/main.rs         # `mod eNN;` per lesson: the `exercises` test target
    ├── tests/exercises/eNN.rs          # tests for lesson NN, in module `eNN`
    ├── scripts/fetch-data.sh           # fixtures into data/ (gitignored)
    └── data/README.md
```

`NN` is the two-digit lesson number and `<name>` the lesson's `name` from `module.toml`.

**Dependencies.** Versions live in the quest's `[workspace.dependencies]`, pinned exactly
(`=x.y.z`); modules inherit them (`arrow = { workspace = true, features = [..] }`). A module
pins its own version only when it cannot share the workspace's (a library that links an older
major of a shared crate). `questkit` comes from the questline checkout:
`questkit = { path = "../../crates/questkit" }`. Profiles (`[profile.dev] opt-level = 1`) are
set once, in the workspace `Cargo.toml`.

## 3. Manifests

Unknown keys are errors, so typos do not pass silently.

### 3.1 `quest.toml`

| key | type | required | meaning |
|-----|------|----------|---------|
| `contract` | integer | yes | `1`, this document |
| `title` | string | yes | shown in the dashboard |
| `summary` | string | yes | one line: what the quest covers, pinned versions |
| `tags` | array of strings | no | crate names and subjects; `/questify` autosort matches topics against them |
| `example` | bool | no | a published sample: autosort never adds modules to it |

### 3.2 `module.toml`

| key | type | required | meaning |
|-----|------|----------|---------|
| `title` | string | yes | |
| `summary` | string | yes | one line, verb-first: what the learner will be able to do |
| `order` | integer | yes | position in the course map; `questline list` lists modules by it |
| `requires` | array of module ids | no | modules of this quest whose vocabulary this one assumes |
| `[[lesson]]` | array of tables | yes, ≥ 1 | the syllabus, in order |
| `lesson.number` | integer | yes | 1, 2, 3, … with no gaps |
| `lesson.name` | string | yes | snake_case; not `all`, `check`, `help` or `list` |
| `lesson.summary` | string | yes | one line |

`module.toml` is the only syllabus: the runner reads it at run time, so a lesson is added by
adding its files and its `[[lesson]]` entry, and wiring it into `src/lessons/mod.rs`.

## 4. Commands

Run from the module directory (or anywhere with `-p <module>` / `--manifest-path`):

| command | does | exit code |
|---------|------|-----------|
| `cargo run -- list` (or no argument) | the syllabus, with per-lesson progress when known | 0 |
| `cargo run -- <n>` / `<name>` | runs one lesson (a name matches by substring), then names its exercise file | non-zero if a lesson claim fails |
| `cargo run -- all` | runs every lesson in order | non-zero if any lesson fails |
| `cargo run -- check [n\|name]` | builds and runs the `exercises` test target, writes §5, prints results (only lesson `n`'s when given) | 0 when the tests ran (whatever they reported), 1 when the build failed |
| `cargo test --test exercises [eNN]` | the same tests, plain; records nothing (the dashboard will show the module as `Stale`) | libtest's |
| `cargo test --features solutions --test exercises` | the tests against the reference solutions: must all pass | libtest's |

Requirements on the tests the `check` command runs:

- Test names start with `eNN::` (the test module per lesson), so each result is attributed to
  lesson `NN`.
- An untouched stub fails with `todo!()`'s message (`not yet implemented…`), which records the
  exercise as `todo` rather than `failed`.
- The whole suite finishes in well under 120 s; a run still going after 120 s is killed and
  recorded with a message (an exercise hangs).
- Tests must not depend on the working directory beyond the module directory and must write
  temporary files to `std::env::temp_dir()`.

## 5. Progress file

`check` writes `modules/<module>/.quest/progress.json` (and the full build and test output to
`.quest/check.log`). Both are gitignored; the dashboard's evaluator writes the same files
through the same code, so a check from the terminal and one from the dashboard are
interchangeable.

```json
{
  "contract": 1,
  "fingerprint": "b2107dc6d5120c27",
  "evaluated_at_ms": 1792960360458,
  "outcome": "tested",
  "exercises": [
    { "lesson": 1, "name": "e01::ex1_starts_at_zero", "status": "passed" },
    { "lesson": 1, "name": "e01::ex2_prediction", "status": "todo", "message": "not yet implemented: ex2: …" }
  ]
}
```

| field | meaning |
|-------|---------|
| `fingerprint` | identity of the module's inputs when the check started (below) |
| `outcome` | `tested`, or `build_failed` (then `exercises` keeps the last results that ran and `message` holds the first compiler error) |
| `message` | build error, or a note about an incomplete run (timeout, crash) |
| `exercises[].status` | `passed`, `failed` (attempted, wrong), `todo` (still the stub), `ignored` (`#[ignore]`, not counted) |

**Completion** of a module is `passed / (passed + failed + todo)`; of a quest, the same sums
over its modules.

**Fingerprint.** FNV-1a over the sorted `(relative path, size, mtime)` of every file in the
module directory, plus the quest's workspace `Cargo.toml`, skipping `target/`, `course-data/`,
dot-files and dot-directories (`.quest/`, `.venv/`) and `Cargo.lock`. A module whose current
fingerprint equals its last evaluation's is **fresh**: its result is reused and no cargo runs.
Any edit (an exercise, a fixture, a dependency pin) makes it **stale**.

## 6. Status

Derived by `questline-core`, shown in the dashboard's STATUS column:

| status | when |
|--------|------|
| `Invalid` | the module (or its quest) breaks §1–§3 |
| `Pending` | never evaluated |
| `Stale` | files changed since the last evaluation |
| `BuildFailed` | the last evaluation of the current files did not compile |
| `NotStarted` / `InProgress` / `Completed` | fresh, with 0 / some / all exercises passed |

## Versioning

`contract` in `quest.toml` names the version a quest was written for. A change that existing
quests could not satisfy bumps it; questline then lists older quests as `Invalid` with the
reason, rather than misreading them.
