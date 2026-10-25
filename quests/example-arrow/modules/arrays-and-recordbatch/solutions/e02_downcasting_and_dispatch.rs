//! Reference solution for `src/exercises/e02_downcasting_and_dispatch.rs`.

use arrow::array::Array;
use arrow::array::ArrowPrimitiveType;
use arrow::array::AsArray;
use arrow::array::PrimitiveArray;
use arrow::datatypes::*;
use arrow::error::ArrowError;

pub fn ex1_to_f64(array: &dyn Array) -> Result<Vec<Option<f64>>, ArrowError> {
    macro_rules! to_f64 {
        ($t:ty) => {
            array.as_primitive::<$t>().iter().map(|v| v.map(|x| x as f64)).collect()
        };
    }
    Ok(match array.data_type() {
        DataType::Int8 => to_f64!(Int8Type),
        DataType::Int16 => to_f64!(Int16Type),
        DataType::Int32 => to_f64!(Int32Type),
        DataType::Int64 => to_f64!(Int64Type),
        DataType::UInt8 => to_f64!(UInt8Type),
        DataType::UInt16 => to_f64!(UInt16Type),
        DataType::UInt32 => to_f64!(UInt32Type),
        DataType::UInt64 => to_f64!(UInt64Type),
        DataType::Float32 => to_f64!(Float32Type),
        DataType::Float64 => to_f64!(Float64Type),
        other => return Err(ArrowError::InvalidArgumentError(format!("not a numeric column: {other}"))),
    })
}

pub fn ex2_longest(array: &dyn Array) -> Option<String> {
    fn longest<'a>(values: impl Iterator<Item = Option<&'a str>>) -> Option<String> {
        // max_by_key keeps the LAST maximum; reduce with > keeps the first.
        values.flatten().reduce(|best, s| if s.len() > best.len() { s } else { best }).map(String::from)
    }
    match array.data_type() {
        DataType::Utf8 => longest(array.as_string::<i32>().iter()),
        DataType::LargeUtf8 => longest(array.as_string::<i64>().iter()),
        DataType::Utf8View => longest(array.as_string_view().iter()),
        _ => None,
    }
}

pub fn ex3_count_where<T: ArrowPrimitiveType>(array: &PrimitiveArray<T>, pred: impl Fn(T::Native) -> bool) -> usize {
    array.iter().flatten().filter(|v| pred(*v)).count()
}

pub fn ex4_predict() -> [bool; 4] {
    // Rust types are exact; Decimal parameters are part of the type; equals_datatype ignores
    // nested field names but not nullability.
    [false, false, false, false]
}
