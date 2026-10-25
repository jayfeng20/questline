use std::sync::Arc;

use arrow::array::*;
use arrow::datatypes::*;
use arrays_and_recordbatch::exercises::e05_schema_and_recordbatch::*;
use arrays_and_recordbatch::lessons::l05_schema_and_recordbatch::trips_schema;

fn trips() -> Vec<Trip> {
    vec![
        Trip { id: 1, city: Some("Lisbon".into()), fare: Some(12.5) },
        Trip { id: 2, city: None, fare: Some(8.0) },
        Trip { id: 3, city: Some("Oslo".into()), fare: None },
    ]
}

#[test]
fn ex1_transposes_rows_into_columns() {
    let batch = ex1_rows_to_batch(&trips()).unwrap();
    assert_eq!(batch.schema(), trips_schema(), "fields, nullability and metadata must match trips_schema()");
    assert_eq!(batch.column(0).as_primitive::<Int64Type>().values().as_ref(), &[1, 2, 3]);
    assert_eq!(batch.column(1).as_string::<i32>().iter().collect::<Vec<_>>(), [Some("Lisbon"), None, Some("Oslo")]);
    assert_eq!(batch.column(2).as_primitive::<Float64Type>().iter().collect::<Vec<_>>(), [Some(12.5), Some(8.0), None]);
    assert_eq!(ex1_rows_to_batch(&[]).unwrap().num_rows(), 0);
}

#[test]
fn ex2_adds_a_nullable_column_and_keeps_everything_else() {
    let batch = ex1_rows_to_batch(&trips()).unwrap_or_else(|_| {
        // Do not depend on ex1: build the batch directly.
        RecordBatch::try_new(trips_schema(), vec![
            Arc::new(Int64Array::from(vec![1, 2, 3])),
            Arc::new(StringArray::from(vec![Some("Lisbon"), None, Some("Oslo")])),
            Arc::new(Float64Array::from(vec![Some(12.5), Some(8.0), None])),
        ]).unwrap()
    });
    let out = ex2_with_tip(&batch, vec![Some(1.0), None, Some(2.5)]).expect("a missing tip is a null, not an error");
    let schema = out.schema();
    assert_eq!(schema.field(3).name(), "tip");
    assert!(schema.field(3).is_nullable(), "tip must be nullable");
    assert_eq!(schema.metadata().get("source").map(String::as_str), Some("lesson 05"), "the schema-level metadata was dropped");
    assert_eq!(schema.field(2).metadata().get("unit").map(String::as_str), Some("USD"), "field metadata must survive too");
    for i in 0..3 {
        assert!(Arc::ptr_eq(out.column(i), batch.column(i)), "column {i} was copied, not shared");
    }
}

#[test]
fn ex3_chunks_are_zero_copy_windows() {
    let n: usize = 2_500;
    let batch = RecordBatch::try_new(trips_schema(), vec![
        Arc::new(Int64Array::from_iter_values(0..n as i64)),
        Arc::new(StringArray::from_iter_values((0..n).map(|i| format!("c{i}")))),
        Arc::new(Float64Array::from_iter_values((0..n).map(|i| i as f64))),
    ]).unwrap();
    let base = batch.column(0).as_primitive::<Int64Type>().values().as_ptr() as usize;
    for max_rows in [1000, 2500, 7] {
        let reader = ex3_chunks(&batch, max_rows);
        assert_eq!(reader.schema(), batch.schema());
        let chunks: Vec<RecordBatch> = reader.map(Result::unwrap).collect();
        assert_eq!(chunks.len(), n.div_ceil(max_rows), "{n} rows in chunks of {max_rows}");
        let mut start = 0;
        for c in &chunks {
            assert!(c.num_rows() > 0 && c.num_rows() <= max_rows);
            let ptr = c.column(0).as_primitive::<Int64Type>().values().as_ptr() as usize;
            assert_eq!(ptr - base, start * 8, "chunk at row {start} should point into the original column");
            start += c.num_rows();
        }
        assert_eq!(start, n);
    }
    let empty = RecordBatch::new_empty(trips_schema());
    assert_eq!(ex3_chunks(&empty, 10).count(), 0, "an empty batch streams no chunks");
}

#[test]
fn ex4_selects_by_name() {
    let batch = RecordBatch::try_new(trips_schema(), vec![
        Arc::new(Int64Array::from(vec![1])),
        Arc::new(StringArray::from(vec!["x"])),
        Arc::new(Float64Array::from(vec![2.0])),
    ]).unwrap();
    let out = ex4_select(&batch, &["fare", "id"]).unwrap();
    let names: Vec<&str> = out.schema_ref().fields().iter().map(|f| f.name().as_str()).collect();
    assert_eq!(names, ["fare", "id"], "in the order asked for");
    assert!(Arc::ptr_eq(out.column(0), batch.column(2)) && Arc::ptr_eq(out.column(1), batch.column(0)), "columns must be shared");
    assert_eq!(out.schema().metadata().get("source").map(String::as_str), Some("lesson 05"), "keep the schema metadata");
    assert!(ex4_select(&batch, &["id", "nope"]).is_err(), "unknown column name");
}
