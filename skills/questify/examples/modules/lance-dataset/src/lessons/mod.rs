//! One module per lesson, in order. Later lessons assume the vocabulary of earlier ones.

pub mod l01_lance_dataset_core;

pub fn run(number: u8) -> anyhow::Result<()> {
    match number {
        1 => l01_lance_dataset_core::run(),
        other => anyhow::bail!("no lesson {other}"),
    }
}
