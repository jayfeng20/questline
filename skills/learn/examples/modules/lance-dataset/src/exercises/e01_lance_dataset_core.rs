//! # Exercises — lesson 01: Lance's core structs
//!
//! Needs `scripts/fetch-data.sh`. Check: `cargo run -- check 1`
//! Stuck: `cargo run -- 1`, https://docs.rs/lance/13.0.0/lance/dataset/struct.Dataset.html,
//! then `solutions/e01_lance_dataset_core.rs`.

use std::path::Path;

use lance::Dataset;

/// Exercise 1: create a Lance dataset at `uri` from the Parquet file, streaming batches, with
/// at most `rows_per_file` rows per fragment.
///
/// 📖 lesson 01 step 1 (`Dataset::write`, `WriteParams::max_rows_per_file`)
pub async fn ex1_write(parquet: &Path, uri: &str, rows_per_file: usize) -> anyhow::Result<Dataset> {
    todo!("ex1: ParquetRecordBatchReaderBuilder -> Dataset::write")
}

/// Exercise 2: the mean `fare_amount` of trips with `payment_type = 1` (credit card), streaming
/// a scan that reads ONLY the columns it needs. Skip null fares.
///
/// 📖 lesson 01 step 3 (`scan()`, `project`, `filter`, `try_into_stream`)
pub async fn ex2_mean_card_fare(ds: &Dataset) -> anyhow::Result<f64> {
    todo!("ex2: project + filter, fold over batches")
}

/// Exercise 3 (measure): bytes Lance reads from storage to scan just `columns`.
///
/// 📖 lesson 01 `read_bytes_since_last` (reset the counter before you scan)
pub async fn ex3_scan_bytes(ds: &Dataset, columns: &[&str]) -> anyhow::Result<u64> {
    todo!("ex3: reset io stats, scan the projection to the end, read stats")
}

/// Exercise 4: `fare_amount` at the given row offsets, using random access (not a scan).
///
/// 📖 lesson 01 step 4 (`take`, `ProjectionRequest::from_columns`)
pub async fn ex4_fares_at(ds: &Dataset, rows: &[u64]) -> anyhow::Result<Vec<Option<f64>>> {
    todo!("ex4: take, then read the Float64 column")
}

/// Exercise 5: append the first `n` rows of the dataset to itself, and return
/// `(version before, version after, row count when checking out the version before)`.
///
/// 📖 lesson 01 step 5 (`append`, `version`, `checkout_version`)
pub async fn ex5_append_and_time_travel(ds: &mut Dataset, n: i64) -> anyhow::Result<(u64, u64, usize)> {
    todo!("ex5: scan().limit(..).try_into_batch(), append, checkout_version")
}
