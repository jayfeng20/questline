# Worked examples, by lesson archetype

Agents imitate the closest example, so pick by **kind of lesson**, not by topic. Every example
below is a real module of the example quest under `examples/` that compiled and passed (solutions green 3 runs in a
row, every stub test failing, `cargo run -- all` exit 0). Read the files themselves; the
template's `__CRATE__` placeholder is `std_and_tokio` / `lance_dataset` there.

| archetype | use it when the topic is... | example | what makes it work |
|-----------|-----------------------------|---------|--------------------|
| **A. Behaviour under load** | a runtime, scheduler, allocator, lock, pool: "what happens when..." | `examples/modules/std-and-tokio/src/lessons/l02_worker_threads.rs` (tokio worker threads) | measured evidence: distinct thread ids, 1 vs 4 worker wall-clock ratio, a heartbeat that stops ticking while a worker is blocked; a **fix** exercise whose stub *is* the classic bug |
| **B. Data that streams, not loads** | readers, scanners, iterators, file formats, "is it all in memory?" | `examples/modules/std-and-tokio/src/lessons/l03_parquet_streaming.rs` (Arrow RecordBatch streams over the NYC taxi fixture) | rows/bytes per batch vs the collected table; projection memory; a real fixture from `scripts/fetch-data.sh` |
| **C. Rebuild a core API** | std collections and traits, protocols: "how does X work inside?" | `examples/modules/std-and-tokio/src/lessons/l04_hashmap_from_scratch.rs` (HashMap), `l05_futures_from_scratch.rs` (Future, Waker, block_on, join, Delay) | the lesson explains and *measures std*; the exercise implements a skeleton with the representation given; tests use **std as an oracle** on thousands of random ops plus targeted edge cases (tombstones in a probe chain, no busy-spin via poll counts, Ready futures never re-polled) |
| **D. A real library's core structs** | a crate's main types and methods: "how do I use X, and what does it do underneath?" | `examples/modules/lance-dataset/src/lessons/l01_lance_dataset_core.rs` (Lance `Dataset`, fragments, `Scanner`, `take`, versions) | every method shown with the evidence of what it does: fragment counts from `max_rows_per_file`, bytes read via the library's own IO counters (open ≪ scan, 2 columns ≪ all, `take` ≪ scan), time travel row counts; tests compare against the source Parquet read with plain Arrow |
| (warm-up) | trivial, std only | `templates/module/src/lessons/l01_vec_growth.rs` | smallest complete lesson: claims checked, predict exercise |

For each example read, in this order: the lesson (`src/lessons/lNN_*.rs`), the stub
(`src/exercises/eNN_*.rs`), the tests (`tests/exercises/eNN.rs`), then the solution
(`solutions/eNN_*.rs`).

## Patterns worth copying

- **Oracle tests** (archetype C): drive your implementation and the reference (std, a second
  API, the source file read another way) with the same seeded random operations and compare
  after every step. A tiny LCG avoids a `rand` dependency:
  ```rust
  struct Lcg(u64);
  impl Lcg { fn next(&mut self) -> u64 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); self.0 >> 33 } }
  ```
- **Hostile inputs that expose the real invariant**: an identity hasher forcing three keys into
  one probe chain, then removing the middle one; insert/remove churn that must not grow the
  table forever (this test caught a bug in the first draft of the reference solution).
- **Counting wrappers**: wrap a future to count polls, a reader to count bytes, a waker to count
  wakes. Counts are deterministic where timings are not.
- **The library's own instrumentation first** (archetype D): Lance has
  `object_store(None).await?.io_stats_incremental()`, Vortex has a metrics registry, DataFusion
  has `EXPLAIN ANALYZE`. Use them before writing your own wrapper.
- **Fix exercises**: give the buggy code (blocking call in async, naive removal) as the stub,
  and a test that detects the bug's symptom.
- **Verify every lesson claim by running it.** Drafts of these examples had a wrong capacity
  formula and a wrong collision-chain count; `check(..)` and the tests caught both.

## When the topic is too big for one archetype

Mix them across lessons: e.g. "lance" → D (core structs) → B (scanner streaming) → C
(implement a tiny columnar page encoder and compare sizes with Lance's) → A (concurrent writers
and commit conflicts).

## Anti-examples (do not ship these)

- A lesson that only prints API calls and their output, with no `check(..)` and no exercises.
- An exercise whose test only checks `is_ok()` or a hard-coded value the lesson printed.
- "Performance" claims without a ratio and a stated machine caveat; timing tests with < 2x margin.
- A rebuild-the-API exercise tested only on the happy path (no collisions, no removal, no resize).
- Fixtures committed to git, or lessons that silently skip a missing fixture without saying how to get it.
