# arrays-and-recordbatch — arrays, encodings and RecordBatch in arrow-rs

The example module of questline's example quest (see `../../README.md`): the everyday API, plus what each
call does underneath. Typed arrays and builders, getting values back out of `dyn Array`,
nested types, choosing an encoding by measuring it, and the Schema/RecordBatch/RecordBatchReader
trio every Arrow reader speaks. Pinned to **arrow 60.0.0** (with `prettyprint`). No downloads: all data is generated in code.

## How a lesson works

1. **Run & read** `cargo run -- 1`: the lesson file (`src/lessons/l01_*.rs`) is the lecture;
   `✅` lines are claims checked as it runs; `🧪 TRY` lines are experiments.
2. **Exercises** in `src/exercises/e01_*.rs`: replace each `todo!()` (exercises marked
   **fix** start as working-but-wrong code: find the bug).
3. **Check** `cargo run -- check 1`: runs the tests, prints what passed and saves your progress for the questline dashboard (`cargo test --test exercises e01` works too, it just saves nothing).
4. **Stuck?** Re-run the lesson, follow the `📖` links on the exercise, then read `solutions/`.

```bash
cargo run -- list                 # syllabus
cargo run -- 1                    # one lesson (or its name: cargo run -- encodings)
cargo run -- all                  # every lesson
cargo run -- check 1              # your answers for lesson 01, progress saved
cargo run -- check                # all your answers, progress saved
```

## Syllabus

| # | lesson | you will be able to | evidence you produce |
|---|--------|---------------------|----------------------|
| 1 | typed_arrays_and_builders | pick a constructor or builder by cost, use marker types for timestamps/dates/decimals, reuse builders correctly | `with_timezone` keeps the i64 pointer; `finish()` hands over the builder's buffer; 0 vs 7 buffer moves with/without `with_capacity`; `max(values())` = 0 under a null |
| 2 | downcasting_and_dispatch | recover typed values from `dyn Array` four ways and write one generic function for every primitive type | `downcast_ref` Some/None; `as_string::<i32>` vs `<i64>`; `downcast_primitive_array!` over Int32/Float64/UInt8/Timestamp; `==` vs `equals_datatype` on List field names |
| 3 | nested_types | build and read List, FixedSizeList, Struct and Map columns, including both levels of nulls | offsets `[0,2,2,2,4]`; a list slice keeping the whole child; child length = rows × 3; struct null over a valid field; Map entries `key`/`value` |
| 4 | encodings | choose Utf8 vs Dictionary vs Run-End vs View from the data's shape, and count nulls logically | measured table: Dict 391 KiB vs Utf8 1.34 MiB (20 countries), REE 275 B (sorted), Utf8 wins for unique URLs; builder slack 781 KiB until `shrink_to_fit`; Int8 key overflow at 129; dedup 3.1 MB → 620 B |
| 5 | schema_and_recordbatch | build validated batches with metadata, slice/project/stream them without copying, evolve schemas | the four `try_new` errors; `Arc::ptr_eq` after `project`; slice pointer +8 B; 10 × 1000-row reader windows; `try_merge` nullability; superset rule of `with_schema` |

## Maintainers

```bash
cargo run -- all                                     # every lesson runs, every check holds
cargo test --features solutions --test exercises     # tests pass with the reference solutions
cargo test --test exercises --no-run                 # learner stubs compile, no warnings
cargo test --test exercises 2>&1 | grep "test result" # stubs: 0 passed
```

## References

- Arrow columnar format: https://arrow.apache.org/docs/format/Columnar.html
- docs.rs: https://docs.rs/arrow-array/60.0.0, https://docs.rs/arrow-schema/60.0.0, https://docs.rs/arrow/60.0.0
- arrow-rs repository: https://github.com/apache/arrow-rs
- Source, after the first build: `~/.cargo/registry/src/*/arrow-array-60.0.0/src/` (`builder/`, `cast.rs`, `record_batch.rs`), `.../arrow-schema-60.0.0/src/`
