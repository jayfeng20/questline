//! One module per lesson, in order. Later lessons assume the vocabulary of earlier ones.

pub mod l01_vec_growth;
pub mod l02_worker_threads;
pub mod l03_parquet_streaming;
pub mod l04_hashmap_from_scratch;
pub mod l05_futures_from_scratch;

pub fn run(number: u8) -> anyhow::Result<()> {
    match number {
        1 => l01_vec_growth::run(),
        2 => l02_worker_threads::run(),
        3 => l03_parquet_streaming::run(),
        4 => l04_hashmap_from_scratch::run(),
        5 => l05_futures_from_scratch::run(),
        other => anyhow::bail!("no lesson {other}"),
    }
}
