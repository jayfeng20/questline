# __PACKAGE__ — a runnable course module

<one paragraph: what this course teaches, the crate versions it pins, who it is for>

## How a lesson works

1. **Run & read** `cargo run -- 1`: the lesson file (`src/lessons/l01_*.rs`) is the lecture;
   `✅` lines are claims checked as it runs; `🧪 TRY` lines are experiments.
2. **Exercises** in `src/exercises/e01_*.rs`: replace each `todo!()`.
3. **Check** `cargo run -- check 1`: runs the tests, prints what passed and saves your progress
   for the questline dashboard (`cargo test --test exercises e01` works too, it just saves nothing).
4. **Stuck?** Re-run the lesson, follow the `📖` links on the exercise, then read `solutions/`.

```bash
cargo run -- list                 # syllabus
cargo run -- all                  # every lesson
cargo run -- check 1              # your answers for lesson 1, progress saved
cargo run -- check                # all your answers
scripts/fetch-data.sh             # fixtures into ./data/
```

## Syllabus

| # | lesson | you will be able to | evidence you produce |
|---|--------|---------------------|----------------------|

## Maintainers

```bash
cargo test --features solutions --test exercises   # tests pass with the reference solutions
cargo test --test exercises --no-run               # learner stubs compile
```

## References
