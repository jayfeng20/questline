use std::fs::File;

use arrow::array::AsArray;
use arrow::datatypes::Float64Type;
use arrow::datatypes::Int64Type;
use lance_dataset::exercises::e01_lance_dataset_core::*;
use lance_dataset::util::fixture;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

fn parquet() -> std::path::PathBuf {
    fixture("green_tripdata_2024-01.parquet").expect("run scripts/fetch-data.sh")
}

fn uri(name: &str) -> String {
    let p = std::env::temp_dir().join(format!("learn-lance-{}-{name}.lance", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p.to_str().unwrap().to_string()
}

/// (fare_amount, payment_type) for every row, read straight from the Parquet fixture.
fn truth() -> Vec<(Option<f64>, Option<i64>)> {
    let mut out = Vec::new();
    for b in ParquetRecordBatchReaderBuilder::try_new(File::open(parquet()).unwrap()).unwrap().build().unwrap() {
        let b = b.unwrap();
        let fare = b.column_by_name("fare_amount").unwrap().as_primitive::<Float64Type>().clone();
        let pay = b.column_by_name("payment_type").unwrap().as_primitive::<Int64Type>().clone();
        out.extend(fare.iter().zip(pay.iter()));
    }
    out
}

#[tokio::test]
async fn ex1_fragments_follow_rows_per_file() {
    let ds = ex1_write(&parquet(), &uri("ex1"), 10_000).await.unwrap();
    let rows = truth().len();
    assert_eq!(ds.count_rows(None).await.unwrap(), rows);
    assert_eq!(ds.count_fragments(), rows.div_ceil(10_000), "one fragment per 10k rows");
}

#[tokio::test]
async fn ex2_matches_parquet() {
    let ds = ex1_write(&parquet(), &uri("ex2"), 1_000_000).await.unwrap();
    let card: Vec<f64> = truth().into_iter().filter(|(_, p)| *p == Some(1)).filter_map(|(f, _)| f).collect();
    let want = card.iter().sum::<f64>() / card.len() as f64;
    let got = ex2_mean_card_fare(&ds).await.unwrap();
    assert!((got - want).abs() < 1e-9, "{got} vs {want}");
}

#[tokio::test]
async fn ex3_bytes_follow_the_projection() {
    let ds = ex1_write(&parquet(), &uri("ex3"), 1_000_000).await.unwrap();
    let one = ex3_scan_bytes(&ds, &["fare_amount"]).await.unwrap();
    let two = ex3_scan_bytes(&ds, &["fare_amount", "lpep_pickup_datetime"]).await.unwrap();
    let names: Vec<String> = ds.schema().fields.iter().map(|f| f.name.clone()).collect();
    let all = ex3_scan_bytes(&ds, &names.iter().map(String::as_str).collect::<Vec<_>>()).await.unwrap();
    assert!(one > 0 && one * 5 < all, "1 column read {one} of {all} bytes");
    assert!(two > one, "a second column costs more: {two} vs {one}");
}

#[tokio::test]
async fn ex4_random_access_returns_the_right_rows() {
    let ds = ex1_write(&parquet(), &uri("ex4"), 20_000).await.unwrap();
    let t = truth();
    let rows = [0u64, 19_999, 20_000, 56_000];
    assert_eq!(ex4_fares_at(&ds, &rows).await.unwrap(), rows.iter().map(|r| t[*r as usize].0).collect::<Vec<_>>(), "rows across fragment boundaries");
}

#[tokio::test]
async fn ex5_old_versions_survive_appends() {
    let mut ds = ex1_write(&parquet(), &uri("ex5"), 1_000_000).await.unwrap();
    let rows = ds.count_rows(None).await.unwrap();
    let (before, after, old_rows) = ex5_append_and_time_travel(&mut ds, 250).await.unwrap();
    assert_eq!(after, before + 1, "one append = one new version");
    assert_eq!(old_rows, rows, "the old version still has the old rows");
    assert_eq!(ds.count_rows(None).await.unwrap(), rows + 250);
}
