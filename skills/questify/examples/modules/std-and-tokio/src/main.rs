//! Lesson runner: `cargo run -- list | all | <number|name> | check [n]` (questline contract).

fn main() -> anyhow::Result<()> {
    questkit::run(env!("CARGO_MANIFEST_DIR"), std_and_tokio::lessons::run)
}
