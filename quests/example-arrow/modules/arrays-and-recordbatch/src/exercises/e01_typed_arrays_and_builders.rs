//! # Exercises — lesson 01: typed arrays and builders
//!
//! Check: `cargo run -- check 1`
//! Stuck: `cargo run -- 1`, the 📖 links below, then `solutions/e01_typed_arrays_and_builders.rs`.

use arrow::array::Array;
use arrow::array::ArrayBuilder;
use arrow::array::Int32Array;
use arrow::array::Int32Builder;
use arrow::array::Int64Array;
use arrow::array::Int64Builder;
use arrow::array::TimestampMillisecondArray;

/// Exercise 1: parse decimal integers; anything that does not parse as an `i32` (including
/// "") becomes a null. Size the builder up front.
///
/// 📖 lesson 01 step 2; https://docs.rs/arrow-array/60.0.0/arrow_array/builder/struct.PrimitiveBuilder.html
pub fn ex1_parse_ints(raw: &[&str]) -> Int32Array {
    todo!("ex1: Int32Builder::with_capacity + append_option")
}

/// Exercise 2: epoch milliseconds as a `Timestamp(Millisecond, Some(tz))` array, WITHOUT
/// copying the Vec (the test compares pointers).
///
/// 📖 lesson 01 step 1 (`From<Vec<i64>>`, `with_timezone`)
pub fn ex2_event_times(epoch_ms: Vec<i64>, tz: &str) -> TimestampMillisecondArray {
    todo!("ex2: from the Vec, then attach the timezone")
}

/// Exercise 3 (fix): split `values` into arrays of `chunk` values (the last one may be shorter),
/// reusing ONE builder. Each chunk comes out wrong. Find out why and fix it.
///
/// 📖 lesson 01 step 2 (`finish` vs `finish_cloned`)
pub fn ex3_chunks(values: &[i64], chunk: usize) -> Vec<Int64Array> {
    let mut builder = Int64Builder::with_capacity(chunk);
    let mut out = Vec::new();
    for (i, v) in values.iter().enumerate() {
        builder.append_value(*v);
        if (i + 1) % chunk == 0 {
            out.push(builder.finish_cloned());
        }
    }
    if builder.len() % chunk != 0 {
        out.push(builder.finish_cloned());
    }
    out
}

/// Exercise 4 (predict): for `let a = Int32Array::from(vec![Some(-5), None, Some(-3)])`, return
/// `(a.values().len(), a.values().iter().min(), a.iter().flatten().count())`.
///
/// 📖 lesson 01 step 4
pub fn ex4_predict() -> (usize, Option<i32>, usize) {
    todo!("ex4: three answers")
}
