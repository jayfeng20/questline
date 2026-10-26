//! # Lesson 03 — Streams of RecordBatches: the table is never in memory
//!
//! ## Lecture
//!
//! An Arrow `RecordBatch` is a schema plus equal-length column arrays: one *chunk* of a table.
//! Readers (`RecordBatchReader`, the Parquet reader, Lance/DataFusion scanners) are iterators of
//! batches: they decode the next batch when you ask for it. A "table" exists only if you
//! collect the iterator. With a projection the reader does not even decode the other columns.
//!
//! What you will measure: rows and bytes held per batch vs the whole table, the number of
//! batches, and what a projection does to memory.
//!
//! ## Reading list
//!   * https://docs.rs/arrow/59/arrow/record_batch/struct.RecordBatch.html
//!   * https://docs.rs/parquet/59/parquet/arrow/arrow_reader/struct.ParquetRecordBatchReaderBuilder.html
//!   * https://arrow.apache.org/docs/format/Columnar.html

use std::fs::File;

use parquet::arrow::ProjectionMask;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use crate::util::{banner, check, fixture, human_bytes, note, section, try_it};

pub fn run() -> anyhow::Result<()> {
    banner("Lesson 03 — RecordBatch streams");
    let path = match fixture("green_tripdata_2024-01.parquet") {
        Ok(p) => p,
        Err(e) => {
            println!("   {e}");
            return Ok(());
        }
    };

    // STEP 1: collect everything (what you should NOT do with big data), as the baseline.
    //   📖 https://docs.rs/arrow/59/arrow/array/trait.Array.html#tymethod.get_array_memory_size
    section("1. Baseline: the whole table in memory");
    let all: Vec<_> = ParquetRecordBatchReaderBuilder::try_new(File::open(&path)?)?.build()?.collect::<Result<_, _>>()?;
    let total_rows: usize = all.iter().map(|b| b.num_rows()).sum();
    let total_bytes: usize = all.iter().map(|b| b.get_array_memory_size()).sum();
    println!("   {total_rows} rows, {} decoded in memory", human_bytes(total_bytes as u64));

    // STEP 2: stream with a small batch size and keep only the current batch.
    section("2. Streaming 4096-row batches");
    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(&path)?)?.with_batch_size(4096).build()?;
    let (mut batches, mut max_rows, mut max_bytes, mut rows) = (0, 0, 0, 0);
    for batch in reader {
        let batch = batch?;
        batches += 1;
        rows += batch.num_rows();
        max_rows = max_rows.max(batch.num_rows());
        max_bytes = max_bytes.max(batch.get_array_memory_size());
    }
    println!("   {batches} batches, at most {max_rows} rows / {} at a time", human_bytes(max_bytes as u64));
    check(rows == total_rows, "the stream still sees every row")?;
    check(max_rows <= 4096 && max_bytes * 5 < total_bytes, "never more than one small batch held at once")?;

    // STEP 3: projection: undecoded columns cost nothing.
    section("3. Projection: 2 of 20 columns");
    let builder = ParquetRecordBatchReaderBuilder::try_new(File::open(&path)?)?;
    let mask = ProjectionMask::columns(builder.parquet_schema(), ["fare_amount", "trip_distance"]);
    let projected: Vec<_> = builder.with_projection(mask).build()?.collect::<Result<_, _>>()?;
    let projected_bytes: usize = projected.iter().map(|b| b.get_array_memory_size()).sum();
    println!("   projected table: {} columns, {}", projected[0].num_columns(), human_bytes(projected_bytes as u64));
    check(projected_bytes * 5 < total_bytes, "two columns take a fraction of the memory of twenty")?;
    note("Same idea in Lance (`scanner().project(..)`), DataFusion and Vortex: push the projection");
    note("into the reader so it never decodes what you do not use.");

    try_it("set the batch size to 1: what happens to the time per row? (util::best_of)");
    try_it("add a RowSelection that skips the first 50 000 rows: how many batches now?");
    Ok(())
}
