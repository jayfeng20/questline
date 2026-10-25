//! Reference solution for `src/exercises/e01_lance_dataset_core.rs`.

use std::fs::File;
use std::path::Path;

use arrow::array::AsArray;
use arrow::datatypes::Float64Type;
use arrow_array::RecordBatchIterator;
use futures::TryStreamExt;
use lance::Dataset;
use lance::dataset::ProjectionRequest;
use lance::dataset::WriteParams;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use crate::lessons::l01_lance_dataset_core::read_bytes_since_last;

pub async fn ex1_write(parquet: &Path, uri: &str, rows_per_file: usize) -> anyhow::Result<Dataset> {
    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(parquet)?)?.with_batch_size(8192).build()?;
    let params = WriteParams { max_rows_per_file: rows_per_file, ..Default::default() };
    Ok(Dataset::write(reader, uri, Some(params)).await?)
}

pub async fn ex2_mean_card_fare(ds: &Dataset) -> anyhow::Result<f64> {
    let mut scanner = ds.scan();
    scanner.project(&["fare_amount"])?.filter("payment_type = 1")?;
    let mut stream = scanner.try_into_stream().await?;
    let (mut sum, mut n) = (0.0, 0usize);
    while let Some(batch) = stream.try_next().await? {
        for v in batch.column(0).as_primitive::<Float64Type>().iter().flatten() {
            sum += v;
            n += 1;
        }
    }
    Ok(sum / n as f64)
}

pub async fn ex3_scan_bytes(ds: &Dataset, columns: &[&str]) -> anyhow::Result<u64> {
    read_bytes_since_last(ds).await?;
    let mut scanner = ds.scan();
    scanner.project(columns)?;
    scanner.try_into_stream().await?.try_for_each(|_| async { Ok(()) }).await?;
    read_bytes_since_last(ds).await
}

pub async fn ex4_fares_at(ds: &Dataset, rows: &[u64]) -> anyhow::Result<Vec<Option<f64>>> {
    let batch = ds.take(rows, ProjectionRequest::from_columns(["fare_amount"], ds.schema())).await?;
    Ok(batch.column(0).as_primitive::<Float64Type>().iter().collect())
}

pub async fn ex5_append_and_time_travel(ds: &mut Dataset, n: i64) -> anyhow::Result<(u64, u64, usize)> {
    let before = ds.version().version;
    let batch = ds.scan().limit(Some(n), None)?.try_into_batch().await?;
    let schema = batch.schema();
    ds.append(RecordBatchIterator::new([Ok(batch)], schema), None).await?;
    let old_rows = ds.checkout_version(before).await?.count_rows(None).await?;
    Ok((before, ds.version().version, old_rows))
}
