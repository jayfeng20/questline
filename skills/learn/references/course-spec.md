# Course spec — what `/learn` must produce

`/learn` produces **modules** inside **quests**. A quest is a Cargo workspace at
`<root>/quests/<quest>/`; a module is one member crate at `modules/<module>/`, a focused course
of 3–8 lessons. The machine contract (manifests, commands, progress file) is
`<root>/docs/contract.md`; this file is how to write a good module that satisfies it. A module
is finished when a learner can run every lesson, has exercises for every lesson, and the
verification commands in SKILL.md step 7 all pass. Everything below is required unless marked
optional.

## 1. Layout

```
quests/<quest>/
├── quest.toml                 # contract = 1, title, summary, tags
├── Cargo.toml                 # [workspace] members = ["modules/*"]; [workspace.dependencies] pins; [profile.dev]
├── README.md                  # course map: every module of the subject, generated or not
└── modules/<module>/
    ├── module.toml            # title, summary, order, requires, [[lesson]] number/name/summary: THE syllabus
    ├── Cargo.toml             # package = <module>; deps `x.workspace = true`; features: solutions
    ├── README.md              # how a lesson works, syllabus table, commands, references
    ├── .gitignore             # /target /course-data /data/* !/data/README.md /.quest
    ├── src/
    │   ├── main.rs            # questkit::run(env!("CARGO_MANIFEST_DIR"), <crate>::lessons::run)
    │   ├── lib.rs             # `pub mod lessons, exercises, util` (no syllabus here)
    │   ├── util.rs            # banner/section/note/try_it/check/fixture/data_dir/best_of/human_bytes
    │   ├── lessons/
    │   │   ├── mod.rs         # `pub mod lNN_name;` + `pub fn run(n: u8)` dispatch
    │   │   └── lNN_<name>.rs  # one lesson = one file
    │   └── exercises/
    │       ├── mod.rs         # #![allow(unused_variables, unused_imports)] + cfg_attr path swap per module
    │       └── eNN_<name>.rs  # stubs with todo!(), learner-facing docs
    ├── solutions/eNN_<name>.rs # same public API as the stub, implemented
    ├── tests/exercises/
    │   ├── main.rs            # `mod eNN;` per lesson (one test binary: fast to link)
    │   └── eNN.rs             # tests for lesson NN, named exN_<what_it_proves>
    ├── scripts/fetch-data.sh  # idempotent downloads into data/, `--large` for big variants
    └── data/README.md         # every fixture: source URL, license, size/rows, lessons using it
```

The solution swap (in `src/exercises/mod.rs`):

```rust
#[cfg_attr(feature = "solutions", path = "../../solutions/e03_streams.rs")]
pub mod e03_streams;
```

Start from `scripts/scaffold.sh quest <quest>` and `scripts/scaffold.sh module <quest>
<module>`; replace the `__TITLE__`/`__SUMMARY__` placeholders, and delete the example lesson
once the first real lesson exists (remove it from `module.toml`, `lessons/mod.rs`,
`exercises/mod.rs`, the tests, `solutions/`).

**Dependencies.** Add each crate's version once to the quest's `[workspace.dependencies]`
(pinned `=x.y.z`, no features) and inherit it in the module with the features it needs:
`arrow = { workspace = true, features = ["prettyprint"] }`. When the quest already pins a crate,
use that pin; never bump a pin other modules use without asking the user. Pin in the module
itself only when it must differ (a library that links an older major of a shared crate), and
say why in a comment. Settings cargo reads from `.cargo/config.toml` (e.g. `PYO3_PYTHON`) go in
the **quest root's** `.cargo/config.toml`, so they apply from the root and from every module.

## 2. Anatomy of a lesson file (`src/lessons/lNN_<name>.rs`)

1. **Module doc = the lecture.** `# Lesson NN — <title>`, then:
   - `## Lecture`: the mental model in prose, with a table when comparing cases (e.g. IO-bound
     vs CPU-bound vs blocking). State what happens *under the hood* (buffers, pointers,
     threads, allocations, what is lazy vs materialised), not only the API.
   - `## Reading list`: official docs, docs.rs pages for the pinned version, the specific
     source files (`~/.cargo/registry/src/*/<crate>-<ver>/src/...`), papers/specs/talks.
2. **`pub fn run() -> anyhow::Result<()>`** made of numbered steps:
   ```rust
   // STEP 3: <what this step shows, one or two lines>.
   //   📖 <docs URL or source path that explains the API used here>
   section("3. <title>");
   ... code that does the thing and prints what happened ...
   check(<condition>, "<the claim, in plain words>")?;
   note("<why it happened / what to notice>");
   ```
   Async topics build their runtime inside the lesson (that is often part of the lesson).
3. **Evidence, not assertions in prose.** Every behavioural claim gets a `check(..)`:
   pointer equality for zero-copy, distinct thread ids for parallelism, wall-clock *ratios*
   for performance, bytes read / rows held / chunk counts for streaming, encoding or type names
   for "what got chosen". If a check fails when you run it, the lesson text was wrong: fix the
   text. Guard machine-dependent checks (core count, OS) instead of loosening them into
   meaninglessness.
4. **End with 2–4 `try_it(..)` experiments**: concrete edits ("set worker_threads to 2x
   cores: faster? why not?"), not "explore the API".
5. **Size:** 100–250 lines. One idea per lesson; split if a lesson needs a second lecture.

## 3. Exercises (`src/exercises/eNN_<name>.rs`)

- Module doc: title, `Check: cargo run -- check NN` (records progress; `cargo test --test exercises eNN` also works), `Stuck:` list (lesson step →
  docs → `solutions/eNN_*.rs`, in that order).
- 2–4 exercises per lesson, `pub fn exN_<verb_phrase>(..)` with `todo!("exN: <one-line nudge>")`.
- Each exercise doc: `/// Exercise N: <what to build or answer>` + `/// 📖 <where to look>`;
  add `/// 💡 <hint>` only where most learners would be stuck.
- Mix the kinds — every lesson should have at least one of the first two:
  | kind | the learner... | example |
  |------|----------------|---------|
  | **build** | uses the API to produce something the test inspects | "a multi-thread runtime with exactly N workers" |
  | **fix** | repairs given code that has the classic bug | "this async fn blocks its worker; keep the work, stop the blocking" |
  | **predict** | returns a constant *before* running; the test measures reality | "100 × sleep(100ms) on one thread takes ~100/1000/10000 ms?" |
  | **measure** | returns a number the test compares across configurations | "bytes read for a 1-column vs 2-column scan" |
  | **implement** | fills a trait impl (custom function, codec, strategy) | "dispatch() for is_power_of_two" |
- Signatures take what the test needs to control (sizes, paths, configs) and return values the
  test can check; never print-only. Async exercises are `pub async fn`.

## 4. Tests (`tests/exercises/eNN.rs`)

- Name tests after what they prove: `ex2_heartbeat_keeps_ticking`, `ex3_clustering_makes_a_filter_prunable`.
- Assert **behaviour**, computed independently of the learner's code where possible: compare
  against plain std/Rust, against a second API path, or against a measurement.
- Assertion messages say what was expected and what happened, so a failure teaches:
  `assert!(ticks >= 5, "only {ticks} heartbeat ticks in ~100 ms: the worker was blocked")`.
- Predict-type tests compute the truth and compare: `assert_eq!(predicted, measured_bucket, "measured {ms} ms")`.
- Cover the edge that separates understanding from copying: off-by-one widths, null semantics,
  empty input, "exactly at the limit".
- Timing: use ratios with 2x+ margins, short durations (≤ 200 ms), and prefer counts (ticks,
  thread ids, bytes) over durations. Run the suite 3 times; flaky tests are bugs.
- Temp files go to `std::env::temp_dir()` with the pid in the name; fixtures via `util::fixture`.

## 5. Solutions (`solutions/eNN_<name>.rs`)

Same public items as the stub, idiomatic, short comments only where the *why* is the lesson
("Timers do not occupy the thread: all 100 sleeps overlap"). First line:
`//! Reference solution for src/exercises/eNN_<name>.rs.`

## 6. Fixtures and data

- Synthetic data generated in code (seeded RNG) when the concept does not need real data;
  shape it so each property you teach is visible (sorted column, low-cardinality column, nulls).
- Real data when the topic is data (formats, query engines, compression): pick a public,
  stable, license-clear dataset with a small variant (≤ 5 MB, default) and a realistic one
  (`--large`). Examples: NYC TLC trip records (Parquet), TPC-H via `tpchgen-cli`, GitHub
  Archive JSON, OpenStreetMap extracts, Wikipedia page views.
- `scripts/fetch-data.sh` is idempotent, uses `curl -fsSL --retry 3`, writes `*.part` then
  renames. Lessons that need a missing fixture print how to fetch it and return `Ok(())`;
  tests fail with the same message (`util::fixture`).

## 7. Evidence toolbox (reuse these patterns)

| to show... | measure |
|------------|---------|
| zero-copy | `std::ptr::eq(a.as_ptr(), b.as_ptr())` or pointer offset arithmetic |
| parallelism / thread placement | `HashSet` of `std::thread::current().id()` from inside tasks |
| a blocked executor | a heartbeat task counting interval ticks during the work |
| streaming / bounded memory | max rows or `nbytes` per batch vs the total; number of batches |
| IO pushdown | a counting reader wrapper (bytes and calls of `read_at`) around the real source |
| laziness | the object's type/tree *before* vs *after* execution; work counters in closures |
| allocation behaviour | capacity sequence, pointer changes, a counting `GlobalAlloc` (optional) |
| performance | `util::best_of(3, ..)`, report ratios, recommend `--release` before quoting |

## 8. README

Module README sections: one-paragraph pitch with pinned versions; "How a lesson works" (run →
exercises → `cargo run -- check N` → stuck); commands; syllabus table
`| # | lesson | you will be able to | evidence you produce |`; maintainer commands; references.
Keep it accurate: update counts when lessons change. The quest README keeps the course map
(every module of the subject with its status: generated or not) current whenever a module is
added.

## 9. Done checklist

- [ ] `cargo run -- all` exits 0 and prints at least one ✅ per lesson
- [ ] `cargo test --features solutions --test exercises` all green, 3 runs in a row
- [ ] `cargo test --test exercises --no-run` compiles with zero warnings
- [ ] stub run: `0 passed`
- [ ] every exercise has a 📖 pointer; every lesson has a reading list and try_its
- [ ] fixtures documented in `data/README.md`; nothing in `data/` committed
- [ ] README syllabus matches `module.toml`; the quest README's course map lists the module
- [ ] `cargo run -- check` reports `0/N` for the stubs and writes `.quest/progress.json`
- [ ] `questline validate quests/<quest>` (or `cargo run -p questline -- validate ...` from the root) passes
