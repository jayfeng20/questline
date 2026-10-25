//! Reference solution for `src/exercises/e02_worker_threads.rs`.

use std::time::Duration;

use tokio::runtime::Builder;
use tokio::runtime::Runtime;

pub fn ex1_runtime(workers: usize) -> Runtime {
    Builder::new_multi_thread().worker_threads(workers).enable_time().build().expect("runtime")
}

pub async fn ex2_fetch_without_blocking(id: u32) -> u32 {
    tokio::task::spawn_blocking(move || {
        std::thread::sleep(Duration::from_millis(100));
        id * 2
    })
    .await
    .expect("blocking task panicked")
}

pub fn ex3_predict_ms_for_100_sleeps_on_one_thread() -> u64 {
    // Timers do not occupy the thread: all 100 sleeps overlap.
    100
}
