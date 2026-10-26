# lance-dataset — example module (archetype D: a real library's core structs)

Example output of `/questify lance` → module "dataset basics", kept as a reference for the skill.
Pins: lance 13.0.0, arrow/parquet 58.4.0, pinned in the module rather than the workspace because
lance 13 links against Arrow 58 (the workspace pins 59). Needs `protoc`.

```bash
scripts/fetch-data.sh                 # NYC green taxi Parquet (1.3 MB)
cargo run -- 1                        # lecture with checked claims
cargo run -- check 1                  # your answers, progress saved
```

## Syllabus

| # | lesson | you will be able to | evidence you produce |
|---|--------|---------------------|----------------------|
| 1 | lance_dataset_core | write, open, scan, take and time-travel a Lance dataset | fragments = ceil(rows/max_rows_per_file); bytes read: open 2.5 KiB, 2-column scan 193 KiB vs 1.46 MiB, take 3.3 KiB; row counts per version |

## Maintainers

```bash
cargo test --features solutions --test exercises
cargo test --test exercises --no-run
```
