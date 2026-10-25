//! Lesson runner: `cargo run -- list | all | <number|name> | check [n]` (questline contract).

fn main() -> anyhow::Result<()> {
    questkit::run(env!("CARGO_MANIFEST_DIR"), arrays_and_recordbatch::lessons::run)
}
