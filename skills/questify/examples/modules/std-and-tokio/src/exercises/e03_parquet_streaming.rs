//! # Exercises — lesson 03: RecordBatch streams
//!
//! Needs `scripts/fetch-data.sh`. Check: `cargo run -- check 3`
//! Stuck: `cargo run -- 3`, the docs.rs links in the lesson, then `solutions/e03_parquet_streaming.rs`.

use std::path::Path;

/// Exercise 1: stream the Parquet file at `path` in batches of `batch_size` rows and return
/// `(number of batches, largest batch's row count)` without collecting the batches.
///
/// 📖 lesson 03 step 2 (`with_batch_size`)
pub fn ex1_batch_shape(path: &Path, batch_size: usize) -> anyhow::Result<(usize, usize)> {
    todo!("ex1: iterate, count, track the max")
}

/// Exercise 2: the mean of the `fare_amount` column (skip nulls), computed while streaming
/// ONLY that column.
///
/// 📖 lesson 03 step 3 (`ProjectionMask::columns`); `arrow::array::AsArray::as_primitive`
pub fn ex2_mean_fare(path: &Path) -> anyhow::Result<f64> {
    todo!("ex2: project one column, fold sum and count over batches")
}
