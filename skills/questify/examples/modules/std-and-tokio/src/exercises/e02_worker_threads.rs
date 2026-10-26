//! # Exercises — lesson 02: Tokio worker threads
//!
//! Check: `cargo run -- check 2`
//! Stuck: `cargo run -- 2`, https://docs.rs/tokio/1/tokio/runtime/struct.Builder.html, then
//! `solutions/e02_worker_threads.rs`.

use std::time::Duration;

use tokio::runtime::Runtime;

/// Exercise 1: a multi-thread runtime with exactly `workers` worker threads and timers enabled.
///
/// 📖 lesson 02 step 1 (`Builder::new_multi_thread`, `worker_threads`, `enable_time`)
pub fn ex1_runtime(workers: usize) -> Runtime {
    todo!("ex1: Builder::new_multi_thread()...")
}

/// Exercise 2: this function blocks the async worker it runs on. Keep the 100 ms of *blocking*
/// work (`std::thread::sleep` stands in for a sync database call) but stop it from freezing
/// other tasks.
///
/// 📖 lesson 02 step 4 (`tokio::task::spawn_blocking`)
pub async fn ex2_fetch_without_blocking(id: u32) -> u32 {
    std::thread::sleep(Duration::from_millis(100)); // fix me
    id * 2
}

/// Exercise 3 (predict, then test): on a `current_thread` runtime, how long (in ms, roughly) do
/// 100 tasks that each `tokio::time::sleep(100 ms)` take to all finish? One of: 100, 1000, 10000.
pub fn ex3_predict_ms_for_100_sleeps_on_one_thread() -> u64 {
    todo!("ex3: 100, 1000 or 10000")
}
