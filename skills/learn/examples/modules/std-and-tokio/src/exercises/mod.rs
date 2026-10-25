//! Exercises: one module per lesson. Replace each `todo!()`, then run
//! `cargo run -- check NN`. Each exercise names the lesson step and docs to read when
//! stuck; reference solutions are in `solutions/` (`--features solutions` swaps them in).

// Stubs ignore their arguments until you fill them in.
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

#[cfg_attr(feature = "solutions", path = "../../solutions/e01_vec_growth.rs")]
pub mod e01_vec_growth;

#[cfg_attr(feature = "solutions", path = "../../solutions/e02_worker_threads.rs")]
pub mod e02_worker_threads;

#[cfg_attr(feature = "solutions", path = "../../solutions/e03_parquet_streaming.rs")]
pub mod e03_parquet_streaming;

#[cfg_attr(feature = "solutions", path = "../../solutions/e04_hashmap_from_scratch.rs")]
pub mod e04_hashmap_from_scratch;

#[cfg_attr(feature = "solutions", path = "../../solutions/e05_futures_from_scratch.rs")]
pub mod e05_futures_from_scratch;
