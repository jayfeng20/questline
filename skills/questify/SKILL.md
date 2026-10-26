---
name: questify
description: Generate a runnable, test-checked Rust course ("quest" module) on a topic and register it with questline. Use when the user runs /questify <topic> (optionally --auto or --manual) or asks to learn, study, or be taught a Rust crate, library, runtime, or concept hands-on. Topics join the existing quest on the same subject (sharing its dependencies) or start a new one; broad topics get a table of contents to choose from first; each lesson gets a lecture, runnable steps with checked claims, exercises with tests, reference solutions, fixtures, and references.
---

# /questify [--auto | --manual] <topic>

Build a **module**: a sub-course the learner runs, reads, modifies, and is *tested* on, inside a
**quest** (one course per subject, one Cargo workspace) registered with questline. The learner
should finish each lesson able to use the API, explain what happens under the hood, and back it
with evidence they produced (a test, a measurement, a printed tree).

The questline root is the directory holding `quests/`: `scripts/questline-root.sh` prints it.
Read, before writing anything:

- `<root>/docs/contract.md`: what questline registers, the commands a module supports, the
  progress file. A module that breaks it shows as `Invalid` in the dashboard.
- `references/course-spec.md`: the shape of a module, a lesson, an exercise, a test.
- `references/placement.md`: which quest a topic goes into (autosort / manualsort).
- `references/toc-format.md`: how to scope a broad topic into a course map.
- `references/examples.md`: each **lesson archetype** → a verified example module under
  `examples/modules/` (behaviour under load, data that streams, rebuild a core API against std
  as an oracle, a real library's core structs with IO evidence). Read the matching example's
  lesson, stub, tests and solution before writing a lesson of that kind.

## Workflow

1. **Parse.** `--manual` selects manualsort, `--auto` or no flag autosort. No topic: ask for one.
2. **Place.** Follow `references/placement.md`: list the quests (`scripts/quests.sh`), choose
   the quest (existing or new), the module id and its place in learning order, asking first
   when the mode or an ambiguity says so.
3. **Scope.** Decide whether the topic fits in one module of 3–8 lessons.
   - Fits (e.g. "tokio worker threads", "parquet writing", "serde enums"): go to 4.
   - Too broad (e.g. "tokio", "lance", "rust async"): build the course map in
     `references/toc-format.md` (for an existing quest, extend its README's map), show it, and
     ask which modules to generate (use your question tool if you have one; otherwise ask and
     stop). Generate only those, each as its own module.
4. **Pin the facts.** Reuse the quest's `[workspace.dependencies]` pins. For a crate the quest
   does not have yet, find the *current* version (crates.io / docs.rs, or `cargo search`) and
   pin it exactly in the workspace. Read the real API (docs.rs, `~/.cargo/registry/src` after a
   first `cargo fetch`), never write lessons from memory alone. Collect reference URLs: official
   docs, docs.rs pages, the specific source files, papers/specs/blog posts.
5. **Plan the lessons.** For each: the one idea it teaches, its archetype (A–D in
   `references/examples.md`), the evidence that proves it (a test assertion, a measurement, a
   visualization), 2–4 exercises, fixtures needed. "How does X work?" topics on std or
   protocols (HashMap, Vec, Future, Mutex, channels, an LRU, a parser) get archetype C: the
   learner implements the core API and std/a reference is the test oracle. Order lessons so
   each uses only vocabulary from earlier ones (and from the modules in `requires`).
6. **Scaffold.** `scripts/scaffold.sh quest <quest>` (new quest only; then fill `quest.toml`
   with title, summary and tags), then `scripts/scaffold.sh module <quest> <module>`. Both
   refuse to overwrite, and both put the new quest or module last in learning order: if it
   belongs earlier, `questline order <quest>[/<module>] <n>` (placement.md §4). Fill
   `module.toml` (title, summary, `requires`, one `[[lesson]]` per lesson) and delete the
   template's sample lesson files.
7. **Write each lesson** in this order: lesson file → exercise stubs → tests → solutions →
   fixtures in `data/` via `scripts/fetch-data.sh`. Follow course-spec.md exactly.
8. **Verify — mandatory, do not skip.** From the module directory:
   ```bash
   cargo run -- all                                    # every lesson runs; every check!() holds
   cargo test --features solutions --test exercises    # every test passes with the solutions
   cargo test --test exercises --no-run                # learner stubs compile, no warnings
   cargo run -- check                                  # stubs: 0/N passed, all todo; writes .quest/
   questline validate quests/<quest>                   # from <root>: the contract holds
   ```
   (`questline` not installed: `cargo run -q --manifest-path <root>/Cargo.toml -- validate …`.)
   If a claim in a lesson fails, the lesson was wrong: fix the text, not the check. Fix every
   warning. Run any timing-based lesson with `--release` too. When the module added a crate to
   the workspace, also `cargo build --workspace` from the quest so the other modules still build.
9. **Hand over.** Update the quest README's course map (row → **generated**, linked). Tell the
   user the quest and module (and why it went there), the syllabus, and the commands:
   `cargo run -- 1`, `cargo run -- check 1`, `scripts/fetch-data.sh`, and `questline` for the
   dashboard. Mention anything you could not verify (e.g. a fixture that needs credentials).

## Non-negotiables

- Every lesson has exercises with tests; a lesson that only prints is not done.
- Every claim a lesson makes about behaviour is either asserted with `check(..)` while it runs
  or tested in `tests/exercises/`. Prefer evidence over prose: counts, bytes, pointers, thread
  ids, timings with a stated ratio, encoding names, row counts held in memory.
- Exercises name where to look when stuck (lesson step, docs URL, source file) before the
  solution. Solutions live only in `solutions/`.
- Fixtures are downloaded into `data/` by `scripts/fetch-data.sh` (small by default, `--large`
  for the realistic size), documented in `data/README.md`, and gitignored. Generate synthetic
  data in code when the topic does not need real data.
- Rust only for now: stable toolchain, edition 2024, exact pins in the quest workspace,
  `anyhow` for errors, `questkit::run` as the entry point. No build scripts, no nightly.
- Never change another module's files, and never bump a shared pin without asking.
