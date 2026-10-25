//! # Exercises — lesson 04: encodings
//!
//! Check: `cargo run -- check 4`
//! Stuck: `cargo run -- 4`, the 📖 links below, then `solutions/e04_encodings.rs`.

use arrow::array::Array;
use arrow::array::AsArray;
use arrow::array::DictionaryArray;
use arrow::array::StringDictionaryBuilder;
use arrow::datatypes::Int16Type;
use arrow::datatypes::Int32Type;
use arrow::error::ArrowError;

/// Exercise 1: dictionary-encode with Int16 keys. Distinct values are numbered in first-seen
/// order; more distinct values than Int16 keys can number must be an `Err`, not a panic.
///
/// 📖 lesson 04 step 3; https://docs.rs/arrow-array/60.0.0/arrow_array/builder/type.StringDictionaryBuilder.html
pub fn ex1_dictionary(values: &[Option<&str>]) -> Result<DictionaryArray<Int16Type>, ArrowError> {
    todo!("ex1: append / append_null, propagate the error")
}

/// Exercise 2: run-end encode by hand: `(run_ends, run_values)` where `run_ends[k]` is the
/// (exclusive) end row of run `k`. `[7, 7, 7, 2, 7]` -> `([3, 4, 5], [7, 2, 7])`.
///
/// 📖 lesson 04 step 2 (run ends are cumulative row counts)
pub fn ex2_run_ends(values: &[i32]) -> (Vec<i32>, Vec<i32>) {
    todo!("ex2: close a run whenever the value changes")
}

/// The four encodings of lesson 04's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    /// Dictionary with Int32 keys.
    Dictionary,
    /// Run-End with Int32 run ends.
    RunEnd,
    View,
}

/// Exercise 3 (predict): which encoding is smallest (after `shrink_to_fit`) for 100 000 rows of
///   [0] 1000 distinct 40-byte product names in random order
///   [1] unique 8-byte ids ("id000001", ...)
///   [2] 1000 distinct 40-byte names in runs of 100 equal rows, runs in random order
///   [3] "yes" / "no", alternating
///
/// 📖 lesson 04 step 1: estimate bytes per row for each encoding before you answer
pub fn ex3_predict_smallest() -> [Encoding; 4] {
    todo!("ex3: four encodings")
}

/// Exercise 4: decode a dictionary column to strings, honouring BOTH kinds of null (a null key,
/// and a valid key that points at a null value). Must work on slices.
///
/// 📖 lesson 04 step 4; `keys()`, `values()`, `key(i)`
pub fn ex4_decode(dict: &DictionaryArray<Int32Type>) -> Vec<Option<String>> {
    todo!("ex4: key -> value, both may be null")
}
