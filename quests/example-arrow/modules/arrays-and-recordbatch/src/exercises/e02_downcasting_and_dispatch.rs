//! # Exercises — lesson 02: downcasting and dispatch
//!
//! Check: `cargo run -- check 2`
//! Stuck: `cargo run -- 2`, https://docs.rs/arrow-array/60.0.0/arrow_array/cast/trait.AsArray.html,
//! then `solutions/e02_downcasting_and_dispatch.rs`.

use arrow::array::Array;
use arrow::array::ArrowPrimitiveType;
use arrow::array::AsArray;
use arrow::array::PrimitiveArray;
use arrow::datatypes::DataType;
use arrow::error::ArrowError;

/// Exercise 1: every value of an integer or floating-point column (Int8 … Int64, UInt8 … UInt64,
/// Float32, Float64) as `f64`, nulls as `None`. Any other DataType (strings, timestamps,
/// decimals, ...) is an `ArrowError::InvalidArgumentError`.
///
/// 📖 lesson 02 steps 2–3; match on `array.data_type()`, then `as_primitive::<..Type>()`
/// 💡 `as f64` works on every native type here; a small `macro_rules!` saves ten copies of the arm.
pub fn ex1_to_f64(array: &dyn Array) -> Result<Vec<Option<f64>>, ArrowError> {
    todo!("ex1: one arm per numeric DataType")
}

/// Exercise 2: the longest string (by bytes, first one wins ties) in a Utf8, LargeUtf8 OR Utf8View
/// column. `None` if there is no valid string or the column is not a string type.
///
/// 📖 lesson 02 step 2 (`as_string::<i32>`, `as_string::<i64>`, `as_string_view`)
pub fn ex2_longest(array: &dyn Array) -> Option<String> {
    todo!("ex2: three string layouts, one answer")
}

/// Exercise 3: how many valid values satisfy `pred`, for ANY primitive array.
///
/// 📖 lesson 02 step 3 (`count_positive`)
pub fn ex3_count_where<T: ArrowPrimitiveType>(array: &PrimitiveArray<T>, pred: impl Fn(T::Native) -> bool) -> usize {
    todo!("ex3: iter().flatten()")
}

/// Exercise 4 (predict): true or false?
///   [0] `TimestampSecondArray::from(vec![1]).as_any().downcast_ref::<Int64Array>().is_some()`
///   [1] `(&Date32Array::from(vec![1]) as &dyn Array).as_primitive_opt::<Int32Type>().is_some()`
///   [2] `DataType::Decimal128(10, 2) == DataType::Decimal128(12, 2)`
///   [3] `List(Field::new("item", Int32, true))` `.equals_datatype(&List(Field::new("item", Int32, false)))`
///
/// 📖 lesson 02 steps 4–5; docs of `DataType::equals_datatype`
pub fn ex4_predict() -> [bool; 4] {
    todo!("ex4: four bools")
}
