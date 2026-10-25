//! # Lesson 02 — Downcasting and dispatch: from `&dyn Array` back to values
//!
//! ## Lecture
//!
//! Batches, readers and kernels pass columns around as `ArrayRef` (`Arc<dyn Array>`). The concrete
//! Rust type is erased; the `DataType` is not. There are four ways back to typed values:
//!
//! | way                                                     | on a mismatch | use it when                          |
//! |---------------------------------------------------------|---------------|--------------------------------------|
//! | `array.as_any().downcast_ref::<Int32Array>()`           | `None`        | std `Any`; you know the Rust type    |
//! | `array.as_primitive::<Int32Type>()`, `as_string::<i32>()`, `as_string_view()`, `as_boolean()`, `as_list::<i32>()`, `as_struct()`, `as_dictionary::<K>()` (the `AsArray` trait) | panics | you already matched on the DataType |
//! | the `_opt` variants: `as_primitive_opt::<T>()`, ...     | `None`        | probing                              |
//! | `downcast_primitive_array!(array => f(array), t => ..)` | your fallback arm | one generic fn, every primitive type |
//!
//! `AsArray` is implemented for `dyn Array` and `ArrayRef`, so on a concrete array write
//! `(&array as &dyn Array).as_primitive_opt::<..>()`.
//!
//! Dispatch happens once per array, not once per value. After the match you call generic code
//! (`fn f<T: ArrowPrimitiveType>(a: &PrimitiveArray<T>)`) that the compiler monomorphises into
//! a tight loop over `T::Native` for every type.
//!
//! Two kinds of "type" are easy to mix up. A downcast checks the exact *Rust* type, and
//! `PrimitiveArray<TimestampMillisecondType>` is not `PrimitiveArray<Int64Type>` even though both
//! hold i64s. `DataType` equality is exact too: timezone, precision, scale, nested field names
//! and nullability all count. `equals_datatype` ignores nested field names and metadata.
//!
//! ## Reading list
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/cast/trait.AsArray.html
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/macro.downcast_primitive_array.html
//!   * https://docs.rs/arrow-schema/60.0.0/arrow_schema/enum.DataType.html#method.equals_datatype
//!   * source: ~/.cargo/registry/src/*/arrow-array-60.0.0/src/cast.rs (`AsArray`, the downcast macros)

use std::sync::Arc;

use arrow::array::Array;
use arrow::array::ArrayRef;
use arrow::array::ArrowPrimitiveType;
use arrow::array::AsArray;
use arrow::array::Float64Array;
use arrow::array::Int32Array;
use arrow::array::Int64Array;
use arrow::array::LargeStringArray;
use arrow::array::PrimitiveArray;
use arrow::array::StringArray;
use arrow::array::TimestampMillisecondArray;
use arrow::array::UInt8Array;
use arrow::datatypes::DataType;
use arrow::datatypes::Field;
use arrow::datatypes::Int32Type;
use arrow::datatypes::Int64Type;
use arrow::datatypes::TimeUnit;
use arrow::downcast_primitive_array;

use crate::util::{banner, check, note, panic_message, section, try_it};

/// Generic over every primitive type: count valid values greater than the type's default (0).
fn count_positive<T: ArrowPrimitiveType>(a: &PrimitiveArray<T>) -> usize
where
    T::Native: PartialOrd,
{
    a.iter().flatten().filter(|v| *v > T::Native::default()).count()
}

/// One match on the DataType, then a monomorphised loop.
pub fn positives(array: &dyn Array) -> Option<usize> {
    downcast_primitive_array!(
        array => Some(count_positive(array)),
        _ => None
    )
}

pub fn run() -> anyhow::Result<()> {
    banner("Lesson 02 — Downcasting and dispatch");

    // STEP 1: std Any.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/array/trait.Array.html#tymethod.as_any
    section("1. as_any().downcast_ref");
    let column: ArrayRef = Arc::new(Int32Array::from(vec![Some(3), None, Some(-1)]));
    println!("   column.data_type() = {}", column.data_type());
    check(column.as_any().downcast_ref::<Int32Array>().is_some(), "downcast_ref::<Int32Array>() recovers the concrete type")?;
    check(column.as_any().downcast_ref::<Int64Array>().is_none(), "the wrong type gives None, never a reinterpretation")?;

    // STEP 2: AsArray: shorter, panics or _opt.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/cast/trait.AsArray.html
    section("2. AsArray helpers");
    let ints = column.as_primitive::<Int32Type>();
    check(ints.value(0) == 3, "as_primitive::<Int32Type>() names the marker, not the array type")?;
    check(column.as_primitive_opt::<Int64Type>().is_none(), "as_primitive_opt returns None on a mismatch")?;
    let msg = panic_message(|| column.as_string::<i32>().len());
    println!("   as_string::<i32>() on Int32 panicked: {msg:?}");
    check(msg.is_some(), "as_string on a non-string column panics: match the DataType first")?;
    let utf8: ArrayRef = Arc::new(StringArray::from(vec!["a"]));
    let large: ArrayRef = Arc::new(LargeStringArray::from(vec!["a"]));
    check(utf8.as_string_opt::<i32>().is_some() && utf8.as_string_opt::<i64>().is_none(), "the offset width is part of the type: Utf8 is as_string::<i32>")?;
    check(large.as_string_opt::<i64>().is_some(), "LargeUtf8 is as_string::<i64>")?;

    // STEP 3: one generic function, every primitive type.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/macro.downcast_primitive_array.html
    section("3. downcast_primitive_array! + a generic fn");
    let columns: Vec<ArrayRef> = vec![
        column.clone(),
        Arc::new(Float64Array::from(vec![0.5, -2.0, 1e9])),
        Arc::new(UInt8Array::from(vec![0, 1, 2, 255])),
        Arc::new(TimestampMillisecondArray::from(vec![-1, 1])),
        utf8.clone(),
    ];
    for c in &columns {
        println!("   {:<28} positives = {:?}", c.data_type().to_string(), positives(c.as_ref()));
    }
    let got: Vec<Option<usize>> = columns.iter().map(|c| positives(c.as_ref())).collect();
    check(got == [Some(1), Some(2), Some(3), Some(1), None], "one function handles Int32, Float64, UInt8 and Timestamp; Utf8 falls through")?;
    note("The macro expands to a match with one arm per primitive DataType; each arm calls a separate");
    note("monomorphised copy of count_positive. The per-value loop never touches dyn Array.");

    // STEP 4: Rust types are exact: a timestamp is not an Int64Array.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/array/struct.PrimitiveArray.html#method.reinterpret_cast
    section("4. Same bytes, different types");
    let ts: ArrayRef = Arc::new(TimestampMillisecondArray::from(vec![1_000, 2_000]));
    check(ts.as_primitive_opt::<Int64Type>().is_none(), "a Timestamp column does not downcast to Int64, though both hold i64")?;
    let as_i64: Int64Array = ts.as_primitive::<arrow::datatypes::TimestampMillisecondType>().reinterpret_cast();
    check(as_i64.values() == &[1_000, 2_000], "reinterpret_cast is the explicit, zero-copy way to see it as Int64")?;

    // STEP 5: DataType equality is exact.
    //   📖 https://docs.rs/arrow-schema/60.0.0/arrow_schema/enum.DataType.html
    section("5. When are two DataTypes equal?");
    let naive = DataType::Timestamp(TimeUnit::Millisecond, None);
    let utc = DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into()));
    check(naive != utc, "Timestamp(ms) without a timezone != Timestamp(ms, \"UTC\")")?;
    let item = DataType::List(Arc::new(Field::new("item", DataType::Int32, true)));
    let element = DataType::List(Arc::new(Field::new("element", DataType::Int32, true)));
    println!("   {item} vs {element}: == {}, equals_datatype {}", item == element, item.equals_datatype(&element));
    check(item != element && item.equals_datatype(&element), "List child field names count for ==, not for equals_datatype")?;
    note("Parquet files from other writers often name list children \"element\"; arrow-rs builders say \"item\".");

    try_it("write fn mean<T: ArrowPrimitiveType>(a: &PrimitiveArray<T>) -> f64 where T::Native: Into<f64>: why won't Int64 compile?");
    try_it("add a Decimal128Array to `columns` in step 3: what does positives() return? why does it compile?");
    try_it("time 1M as_any().downcast_ref calls vs 1M value(i) reads (--release): which one is the per-value cost?");
    Ok(())
}
