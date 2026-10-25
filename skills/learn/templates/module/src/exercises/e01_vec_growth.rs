//! # Exercises — lesson 01: how Vec grows
//!
//! Check: `cargo run -- check 1`
//! Stuck: `cargo run -- 1`, https://doc.rust-lang.org/std/vec/struct.Vec.html, then
//! `solutions/e01_vec_growth.rs`.

/// Exercise 1: the distinct capacities a `Vec<u64>` goes through while pushing `n` elements
/// (starting from `Vec::new()`, include the initial 0).
///
/// 📖 lesson 01 step 1
pub fn ex1_capacity_steps(n: usize) -> Vec<usize> {
    todo!("ex1: push n times, record capacity whenever it changes")
}

/// Exercise 2 (predict, then test): how many times does the buffer of `Vec::with_capacity(10)`
/// move while pushing 11 elements?
pub fn ex2_predict_moves_after_reserving_10() -> usize {
    todo!("ex2: a number")
}
