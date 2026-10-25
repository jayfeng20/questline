//! # Lesson 03 — Nested types: List, FixedSizeList, Struct, Map
//!
//! ## Lecture
//!
//! Nested arrays are ordinary arrays (the *children*) plus a little structure on top:
//!
//! | type                  | parent stores                        | child(ren)                         | null semantics                          |
//! |-----------------------|--------------------------------------|------------------------------------|-----------------------------------------|
//! | `List<T>` / `LargeList<T>` | i32 / i64 offsets (len + 1) + validity | one `T` array with every element of every list | a null list, an empty list and `[null]` are three different things |
//! | `FixedSizeList<T, n>` | validity only                        | one `T` array of `len × n`         | a null list still takes `n` child slots |
//! | `Struct{a, b, ..}`    | validity only                        | one array per field, each of the struct's length | struct-level nulls and field-level nulls are independent |
//! | `Map<K, V>`           | i32 offsets + validity               | `entries: Struct{key: K, value: V}`; keys are never null | a null map ≠ an empty map |
//!
//! Reading `list.value(i)` returns a zero-copy slice of the child. Slicing a list slices its
//! offsets, never its child: `list.values()` is always the *whole* child, so code that walks the
//! child directly must start at `value_offsets()[0]`, not at 0.
//!
//! Builders nest the same way the types do: `ListBuilder<StringBuilder>`, `MapBuilder<K, V>`.
//! Append the child values first, then call `append(true)` on the parent (or `append(false)`
//! for a null).
//!
//! ## Reading list
//!   * https://arrow.apache.org/docs/format/Columnar.html#variable-size-list-layout (and the Struct / Map sections)
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/array/type.ListArray.html
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/array/struct.StructArray.html#method.try_new
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/array/struct.MapArray.html
//!   * source: ~/.cargo/registry/src/*/arrow-array-60.0.0/src/builder/generic_list_builder.rs
//!   * source: ~/.cargo/registry/src/*/arrow-array-60.0.0/src/array/struct_array.rs (`try_new` checks)

use std::sync::Arc;

use arrow::array::Array;
use arrow::array::ArrayRef;
use arrow::array::AsArray;
use arrow::array::FixedSizeListArray;
use arrow::array::Float32Array;
use arrow::array::Float64Array;
use arrow::array::Int32Builder;
use arrow::array::Int64Builder;
use arrow::array::ListBuilder;
use arrow::array::MapBuilder;
use arrow::array::StringArray;
use arrow::array::StringBuilder;
use arrow::array::StructArray;
use arrow::buffer::NullBuffer;
use arrow::datatypes::DataType;
use arrow::datatypes::Field;
use arrow::datatypes::Fields;
use arrow::datatypes::Int32Type;

use crate::util::{banner, check, note, section, try_it};

pub fn run() -> anyhow::Result<()> {
    banner("Lesson 03 — Nested types");

    // STEP 1: a list column, built: [[1, 2], null, [], [3, null]].
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/builder/struct.GenericListBuilder.html
    section("1. ListBuilder: children first, then the parent");
    let mut b = ListBuilder::new(Int32Builder::new());
    b.values().append_value(1);
    b.values().append_value(2);
    b.append(true);
    b.append(false); // null list: no child values
    b.append(true); // empty list
    b.values().append_value(3);
    b.values().append_null();
    b.append(true);
    let list = b.finish();
    println!("   {} : offsets {:?}, child len {}, null_count {}", list.data_type(), list.value_offsets(), list.values().len(), list.null_count());
    check(list.value_offsets() == [0, 2, 2, 2, 4], "null and empty lists both have empty ranges in the offsets")?;
    check(list.is_null(1) && list.is_valid(2) && list.value(2).is_empty(), "only validity separates the null list from the empty one")?;
    check(list.value(3).null_count() == 1 && list.null_count() == 1, "[3, null] is a valid list whose child has a null")?;

    // STEP 2: value(i) and slices share the child.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/array/struct.GenericListArray.html#method.values
    section("2. Slicing a list never slices its child");
    let child_ptr = list.values().as_primitive::<Int32Type>().values().as_ptr();
    let v0 = list.value(0);
    check(v0.as_primitive::<Int32Type>().values().as_ptr() == child_ptr, "value(0) is a zero-copy slice of the child")?;
    let tail = list.slice(2, 2); // [[], [3, null]]
    println!("   slice(2, 2): offsets {:?}, values().len() {}", tail.value_offsets(), tail.values().len());
    check(tail.values().len() == 4 && tail.value_offsets()[0] == 2, "the slice still carries the whole 4-value child; its offsets start at 2")?;
    let first = tail.value_offsets()[0] as usize;
    let last = *tail.value_offsets().last().unwrap() as usize;
    check(tail.values().slice(first, last - first).len() == 2, "the child range the slice covers is offsets[0]..offsets[len]")?;

    // STEP 3: fixed-size lists: no offsets, n child slots per row.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/array/struct.FixedSizeListArray.html#method.try_new
    section("3. FixedSizeList<Float32, 3>: embeddings");
    let field = Arc::new(Field::new("item", DataType::Float32, false));
    let flat = Float32Array::from(vec![0.1, 0.2, 0.3, 0.0, 0.0, 0.0, 0.7, 0.8, 0.9]);
    let vectors = FixedSizeListArray::try_new(field, 3, Arc::new(flat), Some(NullBuffer::from(vec![true, false, true])))?;
    println!("   {} : {} rows over {} child values", vectors.data_type(), vectors.len(), vectors.values().len());
    check(vectors.values().len() == 3 * vectors.len(), "child length = rows x 3, null rows included")?;
    check(vectors.value(2).as_primitive::<arrow::datatypes::Float32Type>().values() == &[0.7, 0.8, 0.9], "row i is child[3i..3i + 3]: no offsets needed")?;

    // STEP 4: structs: two levels of nulls.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/array/struct.StructArray.html
    section("4. Struct{x, y} with struct-level nulls");
    let fields = Fields::from(vec![Field::new("x", DataType::Float64, false), Field::new("y", DataType::Float64, true)]);
    let xs: ArrayRef = Arc::new(Float64Array::from(vec![1.0, 2.0, 3.0]));
    let ys: ArrayRef = Arc::new(Float64Array::from(vec![Some(10.0), Some(20.0), None]));
    let points = StructArray::try_new(fields.clone(), vec![xs.clone(), ys], Some(NullBuffer::from(vec![true, false, true])))?;
    println!("   {} : struct nulls {:?}", points.data_type(), points.nulls().unwrap().iter().collect::<Vec<_>>());
    check(points.is_null(1) && points.column(0).is_valid(1), "point 1 is null, yet its x child slot is valid (2.0): parent validity does not reach the children")?;
    check(points.is_valid(2) && points.column_by_name("y").unwrap().is_null(2), "point 2 is valid with a null y: a field-level null")?;
    let nulls_in_x = StructArray::try_new(fields, vec![Arc::new(Float64Array::from(vec![Some(1.0), None, Some(3.0)])), xs], None);
    println!("   null in non-nullable x: {}", nulls_in_x.as_ref().unwrap_err());
    check(nulls_in_x.is_err(), "try_new rejects a null in a non-nullable field")?;

    // STEP 5: maps are lists of key/value structs.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/builder/struct.MapBuilder.html
    section("5. Map<Utf8, Int64>");
    let mut mb = MapBuilder::new(None, StringBuilder::new(), Int64Builder::new());
    for (k, v) in [("a", 1), ("b", 2)] {
        mb.keys().append_value(k);
        mb.values().append_value(v);
    }
    mb.append(true)?;
    mb.append(false)?; // null map
    mb.keys().append_value("a");
    mb.values().append_value(30);
    mb.append(true)?;
    let map = mb.finish();
    println!("   {}", map.data_type());
    let names: Vec<&str> = map.entries().fields().iter().map(|f| f.name().as_str()).collect();
    check(map.value_offsets() == [0, 2, 2, 3] && names == ["key", "value"], "offsets over one \"entries\" struct of (key, value)")?;
    let row0 = map.value(0);
    let keys: &StringArray = row0.column(0).as_string::<i32>();
    check(keys.iter().collect::<Vec<_>>() == [Some("a"), Some("b")] && map.is_null(1), "value(i) is that row's entries; row 1 is a null map")?;
    note("Keys must be non-null. `unsorted` in the DataType is the writer's statement about key order.");

    try_it("ListArray::from_iter_primitive::<Int32Type, _, _>(...) for step 1's data: same offsets?");
    try_it("LargeListArray for step 1: what changes in value_offsets()'s type and in the buffer size?");
    try_it("StructArray::from(vec![(field, array)]) with arrays of different lengths: error or panic?");
    Ok(())
}
