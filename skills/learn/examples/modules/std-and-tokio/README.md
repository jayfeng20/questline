# std-and-tokio — example module (archetypes A, B, C)

Example output of `/learn`, kept as a reference for the skill. Pins (from the example quest's
workspace): tokio 1.54.1, arrow/parquet 59.3.0.

## How a lesson works

1. **Run & read** `cargo run -- 2`: the lesson file is the lecture; `✅` lines are checked claims.
2. **Exercises** in `src/exercises/eNN_*.rs`: replace each `todo!()` (lesson 02's ex2 is a *fix*: the stub is the bug).
3. **Check** `cargo run -- check 2`: runs the tests and saves progress (`cargo test --test exercises e02` saves nothing).
4. **Stuck?** Re-run the lesson, follow the `📖` links, then read `solutions/`.

```bash
scripts/fetch-data.sh             # NYC green taxi Parquet (1.3 MB) for lesson 03
cargo run -- all
cargo run -- check
```

## Syllabus

| # | lesson | you will be able to | evidence you produce |
|---|--------|---------------------|----------------------|
| 1 | vec_growth | explain Vec reallocation and when to reserve | capacity sequence, pointer moves (13 for 1M pushes, 0 reserved) |
| 2 | worker_threads | pick runtime flavour and worker count for CPU, IO and blocking work | thread ids, 400 ms vs 100 ms, heartbeat ticks 0 vs 21 |
| 3 | parquet_streaming | stream RecordBatches with bounded memory and projections | max 601 KiB per batch vs 8.26 MiB table |
| 4 | hashmap_from_scratch | implement an open-addressing HashMap with Borrow lookups, resize and tombstones | equality with std over 20k random ops; probe lengths with a bad hasher |
| 5 | futures_from_scratch | write futures, a parking block_on, join and a timer | poll counts (no spinning), joined delays overlap |

## Maintainers

```bash
cargo test --features solutions --test exercises
cargo test --test exercises --no-run
```
