//! # Lesson 01 — Lance's core structs: Dataset, fragments, Scanner, take, versions
//!
//! ## Lecture
//!
//! | struct / method                       | what it is                                                                 |
//! |---------------------------------------|----------------------------------------------------------------------------|
//! | `Dataset`                             | a directory: `data/*.lance` files + `_versions/*.manifest` + `_transactions/` |
//! | fragment (`get_fragments()`)          | a horizontal slice of rows: one or more data files with the same rows      |
//! | `Dataset::write(reader, uri, params)` | consumes a `RecordBatchReader` batch by batch; `WriteParams::max_rows_per_file` cuts fragments |
//! | manifest / `version()`                | every commit writes a new manifest; old versions stay readable             |
//! | `scan()` → `Scanner`                  | a builder: `project`, `filter` (SQL), `batch_size`, `limit`, then `try_into_stream()` |
//! | `take(&[row], projection)`            | random access by row offset without scanning                               |
//!
//! Nothing is loaded at `open`: the manifest says which fragments and files exist. A scan
//! reads only the projected columns' pages and yields `RecordBatch`es one at a time.
//! This lesson proves both with Lance's own IO counters
//! (`dataset.object_store(None).await?.io_stats_incremental()` → `read_bytes`, `read_iops`).
//!
//! ## Reading list
//!   * https://lance.org/format/  (table format: fragments, manifests, versions)
//!   * https://docs.rs/lance/13.0.0/lance/dataset/struct.Dataset.html
//!   * https://docs.rs/lance/13.0.0/lance/dataset/scanner/struct.Scanner.html
//!   * source: ~/.cargo/registry/src/*/lance-13.0.0/src/dataset.rs (`write`, `take`, `checkout_version`)
//!   * source: ~/.cargo/registry/src/*/lance-io-13.0.0/src/utils/tracking_store.rs (`IoStats`)

use std::fs::File;

use arrow_array::RecordBatchIterator;
use futures::TryStreamExt;
use lance::Dataset;
use lance::dataset::ProjectionRequest;
use lance::dataset::WriteMode;
use lance::dataset::WriteParams;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use crate::util::{banner, check, data_dir, fixture, human_bytes, note, section, try_it};

/// Bytes Lance read from storage since the last call (and resets the counter).
pub async fn read_bytes_since_last(ds: &Dataset) -> anyhow::Result<u64> {
    Ok(ds.object_store(None).await?.io_stats_incremental().read_bytes)
}

pub fn run() -> anyhow::Result<()> {
    tokio::runtime::Runtime::new()?.block_on(lesson())
}

async fn lesson() -> anyhow::Result<()> {
    banner("Lesson 01 — Lance core structs");
    let parquet = match fixture("green_tripdata_2024-01.parquet") {
        Ok(p) => p,
        Err(e) => {
            println!("   {e}");
            return Ok(());
        }
    };
    let uri = data_dir().join("taxi.lance");
    let _ = std::fs::remove_dir_all(&uri);
    let uri = uri.to_str().unwrap().to_string();

    // STEP 1: write. The Parquet reader is a RecordBatchReader, so Lance pulls batches from it.
    //   📖 https://docs.rs/lance/13.0.0/lance/dataset/struct.WriteParams.html
    section("1. Dataset::write from a stream of batches");
    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(&parquet)?)?.with_batch_size(8192).build()?;
    let params = WriteParams { max_rows_per_file: 20_000, ..Default::default() };
    let ds = Dataset::write(reader, uri.as_str(), Some(params)).await?;
    let rows = ds.count_rows(None).await?;
    println!("   {rows} rows, {} fragments, version {}", ds.count_fragments(), ds.version().version);
    check(ds.count_fragments() == rows.div_ceil(20_000), "max_rows_per_file cut the rows into ceil(rows / 20000) fragments")?;
    for f in ds.get_fragments() {
        println!("   fragment {}: {} rows", f.id(), f.count_rows(None).await?);
    }

    // STEP 2: open reads metadata only.
    section("2. open() reads the manifest, not the data");
    let ds = Dataset::open(&uri).await?;
    let open_bytes = read_bytes_since_last(&ds).await?;
    let on_disk: u64 = walk_size(std::path::Path::new(&uri));
    println!("   open read {} of a {} dataset", human_bytes(open_bytes), human_bytes(on_disk));
    check(open_bytes * 10 < on_disk, "opening reads a small fraction of the dataset")?;

    // STEP 3: a projected, filtered scan is a stream of small batches.
    //   📖 https://docs.rs/lance/13.0.0/lance/dataset/scanner/struct.Scanner.html
    section("3. Scanner: project + filter + batch_size, streamed");
    let mut scanner = ds.scan();
    scanner.project(&["fare_amount", "trip_distance"])?.filter("trip_distance > 10")?.batch_size(1024);
    let mut stream = scanner.try_into_stream().await?;
    let (mut batches, mut max_rows, mut matched) = (0, 0, 0);
    while let Some(batch) = stream.try_next().await? {
        batches += 1;
        max_rows = max_rows.max(batch.num_rows());
        matched += batch.num_rows();
    }
    let scan_bytes = read_bytes_since_last(&ds).await?;
    println!("   {matched} trips > 10 miles in {batches} batches (max {max_rows} rows); read {}", human_bytes(scan_bytes));
    check(matched == ds.count_rows(Some("trip_distance > 10".into())).await?, "the streamed rows equal count_rows(filter)")?;
    let mut full = ds.scan();
    full.batch_size(1024);
    let _all: Vec<_> = full.try_into_stream().await?.try_collect().await?;
    let full_bytes = read_bytes_since_last(&ds).await?;
    println!("   SELECT * read {}", human_bytes(full_bytes));
    check(scan_bytes * 3 < full_bytes, "2 projected columns read a fraction of the bytes of all 20")?;

    // STEP 4: random access.
    section("4. take(): rows by offset, no scan");
    let rows_wanted = [0u64, 30_000, 56_000];
    let taken = ds.take(&rows_wanted, ProjectionRequest::from_columns(["fare_amount"], ds.schema())).await?;
    let take_bytes = read_bytes_since_last(&ds).await?;
    println!("   took {} rows reading {}", taken.num_rows(), human_bytes(take_bytes));
    check(taken.num_rows() == 3 && take_bytes * 5 < full_bytes, "3 rows cost a fraction of a full scan")?;

    // STEP 5: versions. Appending commits a new manifest; the old version is still there.
    section("5. Versions and time travel");
    let mut ds = ds;
    let before = ds.version().version;
    let extra = ds.scan().limit(Some(100), None)?.try_into_batch().await?;
    let schema = extra.schema();
    ds.append(RecordBatchIterator::new([Ok(extra)], schema), Some(WriteParams { mode: WriteMode::Append, ..Default::default() })).await?;
    let old = ds.checkout_version(before).await?;
    println!("   version {before} -> {}; rows now {}, rows at version {before}: {}", ds.version().version, ds.count_rows(None).await?, old.count_rows(None).await?);
    check(ds.count_rows(None).await? == rows + 100 && old.count_rows(None).await? == rows, "old versions stay readable after an append")?;
    note("Each commit is a new `_versions/N.manifest`; data files are shared, not copied.");

    try_it("set max_rows_per_file to 1000: how do fragments, open bytes and scan IOPS change?");
    try_it("compare read_bytes for filter `trip_distance > 10` with and without projecting trip_distance only.");
    try_it("ds.delete(\"fare_amount < 0\") then checkout the previous version: where did the rows go?");
    Ok(())
}

fn walk_size(p: &std::path::Path) -> u64 {
    std::fs::read_dir(p).map(|d| d.flatten().map(|e| {
        let path = e.path();
        if path.is_dir() { walk_size(&path) } else { e.metadata().map(|m| m.len()).unwrap_or(0) }
    }).sum()).unwrap_or(0)
}
