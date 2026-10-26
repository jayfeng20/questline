//! One module per lesson, in order. Later lessons assume the vocabulary of earlier ones.

pub mod l01_vec_growth;

pub fn run(number: u8) -> anyhow::Result<()> {
    match number {
        1 => l01_vec_growth::run(),
        other => anyhow::bail!("no lesson {other}"),
    }
}
