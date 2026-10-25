//! # Lesson 05 — Schema and RecordBatch
//!
//! ## Lecture
//!
//! | type                 | what it is                                                   | clone costs                      |
//! |----------------------|--------------------------------------------------------------|----------------------------------|
//! | `Field` / `FieldRef` | name + `DataType` + `nullable` + `Metadata`                  | `FieldRef = Arc<Field>`: a bump  |
//! | `Schema` / `SchemaRef` | `Fields` (an `Arc<[FieldRef]>`) + schema-level `Metadata`  | `SchemaRef = Arc<Schema>`: a bump |
//! | `RecordBatch`        | `SchemaRef` + `Vec<ArrayRef>` + a row count                   | one bump per column              |
//! | `RecordBatchReader`  | `Iterator<Item = Result<RecordBatch, ArrowError>>` + `schema()` | the streaming interface        |
//!
//! A `RecordBatch` is one chunk of a table: equal-length columns that match a schema.
//! `RecordBatch::try_new` checks, in this order: the column count matches the field count, no
//! non-nullable field has nulls, all columns have the same length, and each column's `DataType`
//! equals its field's exactly. `RecordBatchOptions` can set a row count for zero columns or relax
//! nested field-name matching. `nullable: false` is a promise checked here once. Readers and
//! kernels may rely on it afterwards.
//!
//! Everything shape-only is zero-copy: `project`, `slice`, `column`, `clone` and `with_schema`
//! (the new schema must be a superset of the old one), and `schema_metadata_mut` to edit metadata. A `RecordBatchReader` is how Parquet, IPC, CSV and JSON readers and
//! DataFusion hand you a table without holding it all: you read `schema()` first, then
//! pull batches. `RecordBatchIterator::new(batches, schema)` turns any iterator into one.
//!
//! In arrow 60, `Metadata` is its own type (a copy-on-write `BTreeMap<String, String>`), so
//! cloning fields and schemas never copies their metadata.
//!
//! ## Reading list
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/struct.RecordBatch.html#method.try_new
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/trait.RecordBatchReader.html
//!   * https://docs.rs/arrow-schema/60.0.0/arrow_schema/struct.Schema.html#method.try_merge
//!   * https://docs.rs/arrow-schema/60.0.0/arrow_schema/struct.Metadata.html
//!   * source: ~/.cargo/registry/src/*/arrow-array-60.0.0/src/record_batch.rs (`try_new_impl`)

use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::array::AsArray;
use arrow::array::Float64Array;
use arrow::array::Int32Array;
use arrow::array::Int64Array;
use arrow::array::RecordBatch;
use arrow::array::RecordBatchIterator;
use arrow::array::RecordBatchReader;
use arrow::array::StringArray;
use arrow::datatypes::DataType;
use arrow::datatypes::Field;
use arrow::datatypes::Float64Type;
use arrow::datatypes::Int64Type;
use arrow::datatypes::Schema;
use arrow::datatypes::SchemaRef;
use arrow::util::pretty::pretty_format_batches;

use crate::util::{banner, check, human_bytes, note, section, try_it};

pub fn trips_schema() -> SchemaRef {
    Arc::new(
        Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("city", DataType::Utf8, true),
            Field::new("fare", DataType::Float64, true).with_metadata([("unit", "USD")]),
        ])
        .with_metadata([("source", "lesson 05")]),
    )
}

/// What a consumer of any reader sees: (schema, batches, rows).
fn consume(reader: impl RecordBatchReader) -> anyhow::Result<(SchemaRef, usize, usize)> {
    let schema = reader.schema();
    let (mut batches, mut rows) = (0, 0);
    for batch in reader {
        batches += 1;
        rows += batch?.num_rows();
    }
    Ok((schema, batches, rows))
}

pub fn run() -> anyhow::Result<()> {
    banner("Lesson 05 — Schema and RecordBatch");

    // STEP 1: a schema with metadata, a batch that matches it.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/struct.RecordBatch.html
    section("1. Schema + columns = RecordBatch");
    let schema = trips_schema();
    let ids: ArrayRef = Arc::new(Int64Array::from(vec![1, 2, 3, 4]));
    let cities: ArrayRef = Arc::new(StringArray::from(vec![Some("Lisbon"), None, Some("Oslo"), Some("Lima")]));
    let fares: ArrayRef = Arc::new(Float64Array::from(vec![Some(12.5), Some(8.0), None, Some(30.25)]));
    let batch = RecordBatch::try_new(schema.clone(), vec![ids.clone(), cities.clone(), fares.clone()])?;
    println!("{}", pretty_format_batches(std::slice::from_ref(&batch))?.to_string().lines().map(|l| format!("   {l}\n")).collect::<String>());
    check(batch.num_rows() == 4 && batch.num_columns() == 3, "4 rows x 3 columns")?;
    check(batch.schema().field(2).metadata().get("unit").map(String::as_str) == Some("USD"), "field metadata travels with the schema")?;

    // STEP 2: try_new's checks.
    //   📖 source: record_batch.rs `try_new_impl`
    section("2. What try_new rejects");
    let short: ArrayRef = Arc::new(Int64Array::from(vec![1, 2]));
    let with_null: ArrayRef = Arc::new(Int64Array::from(vec![Some(1), None, Some(3), Some(4)]));
    let wrong_type: ArrayRef = Arc::new(Int32Array::from(vec![1, 2, 3, 4]));
    let cases = [
        ("2 columns for 3 fields", vec![ids.clone(), cities.clone()]),
        ("a null in non-nullable id", vec![with_null, cities.clone(), fares.clone()]),
        ("id has 2 rows, the rest 4", vec![short, cities.clone(), fares.clone()]),
        ("Int32 where the schema says Int64", vec![wrong_type, cities.clone(), fares.clone()]),
    ];
    let mut rejected = 0;
    for (what, columns) in cases {
        let err = RecordBatch::try_new(schema.clone(), columns).err();
        println!("   {what:<36} -> {}", err.as_ref().map_or("accepted?!".to_string(), |e| e.to_string()));
        rejected += err.is_some() as usize;
    }
    check(rejected == 4, "column count, nullability, length and exact type are all checked")?;

    // STEP 3: shape operations share the columns.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/struct.RecordBatch.html#method.project
    section("3. project, slice, clone: zero-copy");
    let projected = batch.project(&[2, 0])?;
    check(Arc::ptr_eq(projected.column(0), batch.column(2)), "project([2, 0]) reorders the same ArrayRefs (fare is now column 0)")?;
    let sliced = batch.slice(1, 2);
    let base = batch.column(2).as_primitive::<Float64Type>().values().as_ptr() as usize;
    let moved = sliced.column(2).as_primitive::<Float64Type>().values().as_ptr() as usize;
    check(sliced.num_rows() == 2 && moved - base == 8, "slice(1, 2) slices every column: fare's pointer moved 8 bytes")?;
    let before = Arc::strong_count(batch.column(0));
    let copy = batch.clone();
    check(Arc::strong_count(batch.column(0)) == before + 1 && Arc::ptr_eq(copy.column(0), batch.column(0)), "clone() bumps one refcount per column")?;
    println!("   batch memory: {}", human_bytes(batch.get_array_memory_size() as u64));

    // STEP 4: a stream of batches.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/struct.RecordBatchIterator.html
    section("4. RecordBatchReader: schema first, then batches");
    let n = 10_000;
    let big = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Int64Array::from_iter_values(0..n as i64)),
            Arc::new(StringArray::from_iter_values((0..n).map(|i| format!("city{}", i % 7)))),
            Arc::new(Float64Array::from_iter_values((0..n).map(|i| i as f64))),
        ],
    )?;
    let chunks: Vec<RecordBatch> = (0..n).step_by(1000).map(|start| big.slice(start, 1000.min(n - start))).collect();
    let chunk_ptr = chunks[3].column(0).as_primitive::<Int64Type>().values().as_ptr() as usize;
    let big_ptr = big.column(0).as_primitive::<Int64Type>().values().as_ptr() as usize;
    let reader = RecordBatchIterator::new(chunks.into_iter().map(Ok), schema.clone());
    let (seen_schema, batches, rows) = consume(reader)?;
    println!("   {batches} batches, {rows} rows, schema has {} fields", seen_schema.fields().len());
    check(batches == 10 && rows == n && seen_schema == schema, "the consumer saw the schema, then 10 batches of 1000 rows")?;
    check(chunk_ptr - big_ptr == 3000 * 8, "chunk 3 is a window into the big batch (3000 x 8 bytes in)")?;

    // STEP 5: evolving schemas.
    //   📖 https://docs.rs/arrow-schema/60.0.0/arrow_schema/struct.Schema.html#method.try_merge
    section("5. try_merge and with_schema");
    let v1 = Schema::new(vec![Field::new("id", DataType::Int64, false), Field::new("city", DataType::Utf8, true)]);
    let v2 = Schema::new(vec![Field::new("id", DataType::Int64, true), Field::new("tip", DataType::Float64, true)]);
    let merged = Schema::try_merge([v1.clone(), v2])?;
    println!("   merged: {:?}", merged.fields().iter().map(|f| format!("{}{}", f.name(), if f.is_nullable() { "?" } else { "" })).collect::<Vec<_>>());
    check(merged.fields().len() == 3 && merged.field(0).is_nullable(), "fields are unioned; nullable wins (id becomes nullable)")?;
    let clash = Schema::try_merge([v1, Schema::new(vec![Field::new("id", DataType::Utf8, false)])]);
    check(clash.is_err(), "the same name with a different type cannot merge")?;
    let v2_schema = schema.as_ref().clone().with_metadata(schema.metadata().clone().with("version", "2"));
    let relabeled = batch.clone().with_schema(Arc::new(v2_schema))?;
    check(Arc::ptr_eq(relabeled.column(1), batch.column(1)) && relabeled.schema().metadata().len() == 2, "with_schema accepts a superset (old metadata kept, plus \"version\") and keeps the columns")?;
    let changed = batch.clone().with_schema(Arc::new(schema.as_ref().clone().with_metadata([("source", "changed")])));
    check(changed.is_err(), "changing a metadata value is not a superset: rejected")?;
    note("The test is Schema::contains: compatible fields, and every old metadata key present with the same value.");
    let mut edited = batch.clone();
    edited.schema_metadata_mut().insert("source", "changed");
    let source = |b: &RecordBatch| b.schema().metadata().get("source").cloned();
    check(source(&edited).as_deref() == Some("changed") && source(&batch).as_deref() == Some("lesson 05") && Arc::ptr_eq(edited.column(0), batch.column(0)), "schema_metadata_mut edits a copy-on-write schema; the columns stay shared")?;

    try_it("RecordBatch::try_new(schema with zero fields, vec![]): why does it fail, and what does RecordBatchOptions::with_row_count fix?");
    try_it("batch.column_by_name(\"fare\") vs batch.schema().index_of(\"fare\"): which can tell you a name is missing, with a message?");
    try_it("normalize(\".\", None) on a batch with a Struct column: what columns come out?");
    Ok(())
}
