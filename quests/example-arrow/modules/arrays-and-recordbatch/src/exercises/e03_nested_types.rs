//! # Exercises — lesson 03: nested types
//!
//! Check: `cargo run -- check 3`
//! Stuck: `cargo run -- 3`, the 📖 links below, then `solutions/e03_nested_types.rs`.

use std::sync::Arc;

use arrow::array::Array;
use arrow::array::AsArray;
use arrow::array::Float64Array;
use arrow::array::ListArray;
use arrow::array::ListBuilder;
use arrow::array::MapArray;
use arrow::array::StringBuilder;
use arrow::array::StructArray;
use arrow::buffer::NullBuffer;
use arrow::datatypes::DataType;
use arrow::datatypes::Field;
use arrow::datatypes::Fields;
use arrow::error::ArrowError;

/// Exercise 1: a `List<Utf8>` column of tags; `None` is a null list (not an empty one).
///
/// 📖 lesson 03 step 1; https://docs.rs/arrow-array/60.0.0/arrow_array/builder/struct.GenericListBuilder.html
pub fn ex1_tags(rows: &[Option<Vec<&str>>]) -> ListArray {
    todo!("ex1: ListBuilder<StringBuilder>")
}

/// Exercise 2: each row's list length (`None` for a null list), read from the offsets alone.
/// Must be right for sliced lists too.
///
/// 📖 lesson 03 step 2 (`value_offsets`)
pub fn ex2_list_lengths(list: &ListArray) -> Vec<Option<usize>> {
    todo!("ex2: windows over value_offsets(), masked by validity")
}

/// Exercise 3: a `Struct{x: Float64 (non-null), y: Float64 (non-null)}` column where row `i` is
/// null when `!valid[i]`. No NullBuffer at all when every row is valid. Mismatched lengths are an
/// error, not a panic.
///
/// 📖 lesson 03 step 4; https://docs.rs/arrow-array/60.0.0/arrow_array/array/struct.StructArray.html#method.try_new
pub fn ex3_points(xs: Vec<f64>, ys: Vec<f64>, valid: Vec<bool>) -> Result<StructArray, ArrowError> {
    todo!("ex3: Fields + two Float64Arrays + Option<NullBuffer>")
}

/// Exercise 4: for each row of a `Map<Utf8, Int64>`, the value of the FIRST entry whose key is
/// `key`. `None` for a null map, a missing key or a null value.
///
/// 📖 lesson 03 step 5 (`value(i)`, `keys()`, `values()`, `value_offsets()`)
pub fn ex4_map_get(map: &MapArray, key: &str) -> Vec<Option<i64>> {
    todo!("ex4: walk each row's entries")
}
