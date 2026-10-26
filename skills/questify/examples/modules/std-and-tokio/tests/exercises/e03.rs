use std::fs::File;

use arrow::array::AsArray;
use arrow::datatypes::Float64Type;
use std_and_tokio::exercises::e03_parquet_streaming::*;
use std_and_tokio::util::fixture;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

fn taxi() -> std::path::PathBuf {
    fixture("green_tripdata_2024-01.parquet").expect("run scripts/fetch-data.sh")
}

#[test]
fn ex1_batches_follow_the_batch_size() {
    let rows = ParquetRecordBatchReaderBuilder::try_new(File::open(taxi()).unwrap()).unwrap().metadata().file_metadata().num_rows() as usize;
    for size in [1000usize, 8192] {
        let (n, max) = ex1_batch_shape(&taxi(), size).unwrap();
        assert_eq!(max, size, "full batches are exactly batch_size rows");
        assert_eq!(n, rows.div_ceil(size), "{rows} rows in {size}-row batches");
    }
}

#[test]
fn ex2_mean_matches_a_full_read() {
    let mut sum = 0.0;
    let mut count = 0usize;
    for batch in ParquetRecordBatchReaderBuilder::try_new(File::open(taxi()).unwrap()).unwrap().build().unwrap() {
        let batch = batch.unwrap();
        for v in batch.column_by_name("fare_amount").unwrap().as_primitive::<Float64Type>().iter().flatten() {
            sum += v;
            count += 1;
        }
    }
    let got = ex2_mean_fare(&taxi()).unwrap();
    assert!((got - sum / count as f64).abs() < 1e-9, "{got} vs {}", sum / count as f64);
}
