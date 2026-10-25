//! # Lesson 01 — Typed arrays and builders
//!
//! ## Lecture
//!
//! `PrimitiveArray<T>` is generic over a *marker type* `T: ArrowPrimitiveType`. The marker says
//! two things: the Rust value type (`T::Native`) and the Arrow logical type (`T::DATA_TYPE`).
//! `Int32Array` is just `PrimitiveArray<Int32Type>`. Many markers share one native type, so
//! a timestamp, a date and a duration are all an i64 buffer with a different `DataType`:
//!
//! | marker                      | `Native` | `DATA_TYPE`                       |
//! |-----------------------------|----------|-----------------------------------|
//! | `Int64Type`                 | i64      | Int64                             |
//! | `TimestampMillisecondType`  | i64      | Timestamp(Millisecond, tz)        |
//! | `Date32Type`                | i32      | Date32 (days since 1970-01-01)    |
//! | `Decimal128Type`            | i128     | Decimal128(precision, scale)      |
//!
//! Ways to make an array, and what each costs:
//!
//! | you have                  | use                                              | cost                          |
//! |---------------------------|--------------------------------------------------|-------------------------------|
//! | a `Vec<T>` (no nulls)     | `Int32Array::from(vec)`, `PrimitiveArray::new(vec.into(), nulls)` | zero-copy        |
//! | values one at a time      | `Int32Builder::with_capacity(n)`: `append_value`, `append_null`, `append_option`, `append_slice`, `finish()` | one buffer grows, then becomes the array |
//! | an iterator of `Option<T>`| `Int32Array::from_iter(..)`                      | one pass through a builder    |
//! | strings                   | `StringBuilder::with_capacity(items, bytes)`     | offsets + bytes, two buffers  |
//!
//! `len()`/`is_empty()` on a builder come from the `ArrayBuilder` trait (import it). `finish()`
//! hands the builder's buffers to the array and resets the builder, which you can then
//! reuse. `finish_cloned()` copies them and leaves the builder as it was. Builders write
//! `Default::default()` under a null, so `values()` sees zeros where `iter()` sees `None`.
//!
//! ## Reading list
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/array/struct.PrimitiveArray.html (the constructor table)
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/types/index.html (every marker type)
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/builder/index.html
//!   * source: ~/.cargo/registry/src/*/arrow-array-60.0.0/src/builder/primitive_builder.rs (`finish`, `finish_cloned`)
//!   * source: ~/.cargo/registry/src/*/arrow-array-60.0.0/src/types.rs (`make_type!`)

use std::sync::Arc;

use arrow::array::Array;
use arrow::array::ArrayBuilder;
use arrow::array::ArrayRef;
use arrow::array::Int32Array;
use arrow::array::Int32Builder;
use arrow::array::Int64Array;
use arrow::array::StringBuilder;
use arrow::array::TimestampMillisecondArray;
use arrow::datatypes::ArrowPrimitiveType;
use arrow::datatypes::DataType;
use arrow::datatypes::Int32Type;
use arrow::datatypes::TimeUnit;
use arrow::datatypes::TimestampMillisecondType;

use crate::util::{banner, check, note, section, try_it};

/// How many times the builder's bytes buffer moved (reallocated) while appending `words`.
fn buffer_moves(mut builder: StringBuilder, words: &[String]) -> usize {
    let mut moves = 0;
    let mut last = builder.values_slice().as_ptr();
    for w in words {
        builder.append_value(w);
        if builder.values_slice().as_ptr() != last {
            moves += 1;
            last = builder.values_slice().as_ptr();
        }
    }
    moves
}

pub fn run() -> anyhow::Result<()> {
    banner("Lesson 01 — Typed arrays and builders");

    // STEP 1: marker types: one native buffer, many logical types.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/types/struct.TimestampMillisecondType.html
    section("1. Marker types: Native + DATA_TYPE");
    println!("   Int32Type: {} bytes, {:?}", size_of::<<Int32Type as ArrowPrimitiveType>::Native>(), Int32Type::DATA_TYPE);
    println!("   TimestampMillisecondType: {} bytes, {:?}", size_of::<<TimestampMillisecondType as ArrowPrimitiveType>::Native>(), TimestampMillisecondType::DATA_TYPE);
    check(TimestampMillisecondType::DATA_TYPE == DataType::Timestamp(TimeUnit::Millisecond, None), "a timestamp marker is i64 + Timestamp(Millisecond, no timezone)")?;
    let millis = vec![0i64, 86_400_000, 1_700_000_000_000];
    let ptr = millis.as_ptr();
    let ts = TimestampMillisecondArray::from(millis).with_timezone("UTC");
    println!("   {:?} -> {:?}", ts.data_type(), ts.value_as_datetime(1));
    check(ts.values().as_ptr() == ptr, "with_timezone changes the DataType, not the i64 buffer")?;
    check(ts.value_as_datetime(1).map(|d| d.to_string()) == Some("1970-01-02 00:00:00".into()), "86 400 000 ms reads back as 1970-01-02")?;
    let as_int: Int64Array = ts.reinterpret_cast();
    check(as_int.values().as_ptr() == ptr && as_int.value(2) == 1_700_000_000_000, "reinterpret_cast views the same i64s as plain Int64 (no copy)")?;

    // STEP 2: builders.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/builder/struct.PrimitiveBuilder.html
    section("2. Int32Builder: append, finish, reuse");
    let mut b = Int32Builder::with_capacity(1000);
    let start = b.values_slice().as_ptr();
    b.append_value(1);
    b.append_null();
    b.append_option(Some(3));
    b.append_slice(&[4, 5]);
    let first = b.finish();
    println!("   first: {:?}, builder len after finish: {}", first.iter().collect::<Vec<_>>(), b.len());
    check(first.iter().collect::<Vec<_>>() == [Some(1), None, Some(3), Some(4), Some(5)], "append_value / append_null / append_option / append_slice")?;
    check(first.values().as_ptr() == start, "finish() hands the builder's own buffer to the array: no copy")?;
    check(b.is_empty(), "finish() resets the builder, ready for the next batch")?;
    b.append_value(7);
    let snapshot = b.finish_cloned();
    b.append_value(8);
    check(snapshot.len() == 1 && b.len() == 2, "finish_cloned() copies: the builder keeps its rows and keeps growing")?;

    // STEP 3: with_capacity means no reallocation.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/builder/type.StringBuilder.html
    section("3. Sizing builders up front");
    let words: Vec<String> = (0..10_000).map(|i| format!("word{i}")).collect();
    let bytes: usize = words.iter().map(String::len).sum();
    let moves = buffer_moves(StringBuilder::with_capacity(words.len(), bytes), &words);
    let unsized_moves = buffer_moves(StringBuilder::new(), &words);
    println!("   {bytes} bytes of strings: values buffer moved {moves}x when sized, {unsized_moves}x from StringBuilder::new()");
    check(moves == 0 && unsized_moves > 3, "with_capacity(items, bytes) appends without ever moving the bytes")?;

    // STEP 4: values() vs iter(): what sits under nulls matters for min/max.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/array/struct.PrimitiveArray.html#method.values
    section("4. values() includes the slots under nulls");
    let temps = Int32Array::from(vec![Some(-5), None, Some(-3)]);
    let raw_max = temps.values().iter().copied().max();
    let real_max = temps.iter().flatten().max();
    println!("   values() {:?}: max {raw_max:?}; iter().flatten() max {real_max:?}", temps.values());
    check(raw_max == Some(0) && real_max == Some(-3), "the null slot holds 0, which wins a max over values()")?;
    note("Sums happen to survive (0 is the identity). Min, max, mean and count do not.");

    // STEP 5: ArrayRef: share, don't copy.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/array/type.ArrayRef.html
    section("5. ArrayRef = Arc<dyn Array>");
    let column: ArrayRef = Arc::new(first);
    let shared = Arc::clone(&column);
    check(Arc::ptr_eq(&column, &shared) && Arc::strong_count(&column) == 2, "cloning an ArrayRef bumps a refcount; batches hold columns this way")?;
    check(column.data_type() == &DataType::Int32 && column.null_count() == 1, "the dyn Array still knows its DataType and null count")?;

    try_it("Int32Builder::new() and append 10 000 values: print capacity() each time it changes");
    try_it("Date32Array::from(vec![19_000]).value_as_date(0): which day is it?");
    try_it("Decimal128Array::from(vec![12345]).with_precision_and_scale(10, 2)?.value_as_string(0)");
    Ok(())
}
