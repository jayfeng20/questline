//! One module per lesson, in order. Later lessons assume the vocabulary of earlier ones.

pub mod l01_typed_arrays_and_builders;
pub mod l02_downcasting_and_dispatch;
pub mod l03_nested_types;
pub mod l04_encodings;
pub mod l05_schema_and_recordbatch;

pub fn run(number: u8) -> anyhow::Result<()> {
    match number {
        1 => l01_typed_arrays_and_builders::run(),
        2 => l02_downcasting_and_dispatch::run(),
        3 => l03_nested_types::run(),
        4 => l04_encodings::run(),
        5 => l05_schema_and_recordbatch::run(),
        other => anyhow::bail!("no lesson {other}"),
    }
}
