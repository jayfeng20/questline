//! Exercises: one module per lesson. Replace each `todo!()`, then run
//! `cargo run -- check NN`. Each exercise names the lesson step and docs to read when
//! stuck; reference solutions are in `solutions/` (`--features solutions` swaps them in).

// Stubs ignore their arguments until you fill them in.
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

#[cfg_attr(feature = "solutions", path = "../../solutions/e01_typed_arrays_and_builders.rs")]
pub mod e01_typed_arrays_and_builders;

#[cfg_attr(feature = "solutions", path = "../../solutions/e02_downcasting_and_dispatch.rs")]
pub mod e02_downcasting_and_dispatch;

#[cfg_attr(feature = "solutions", path = "../../solutions/e03_nested_types.rs")]
pub mod e03_nested_types;

#[cfg_attr(feature = "solutions", path = "../../solutions/e04_encodings.rs")]
pub mod e04_encodings;

#[cfg_attr(feature = "solutions", path = "../../solutions/e05_schema_and_recordbatch.rs")]
pub mod e05_schema_and_recordbatch;
