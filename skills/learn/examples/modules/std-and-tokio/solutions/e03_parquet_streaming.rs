//! Reference solution for `src/exercises/e03_parquet_streaming.rs`.

use std::fs::File;
use std::path::Path;

use arrow::array::AsArray;
use arrow::datatypes::Float64Type;
use parquet::arrow::ProjectionMask;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

pub fn ex1_batch_shape(path: &Path, batch_size: usize) -> anyhow::Result<(usize, usize)> {
    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(path)?)?.with_batch_size(batch_size).build()?;
    let (mut n, mut max) = (0, 0);
    for batch in reader {
        n += 1;
        max = max.max(batch?.num_rows());
    }
    Ok((n, max))
}

pub fn ex2_mean_fare(path: &Path) -> anyhow::Result<f64> {
    let builder = ParquetRecordBatchReaderBuilder::try_new(File::open(path)?)?;
    let mask = ProjectionMask::columns(builder.parquet_schema(), ["fare_amount"]);
    let (mut sum, mut count) = (0.0, 0usize);
    for batch in builder.with_projection(mask).build()? {
        let batch = batch?;
        anyhow::ensure!(batch.num_columns() == 1, "decode only the column you need");
        for v in batch.column(0).as_primitive::<Float64Type>().iter().flatten() {
            sum += v;
            count += 1;
        }
    }
    Ok(sum / count as f64)
}
