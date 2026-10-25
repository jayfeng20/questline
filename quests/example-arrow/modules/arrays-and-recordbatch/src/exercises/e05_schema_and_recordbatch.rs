//! # Exercises — lesson 05: Schema and RecordBatch
//!
//! Check: `cargo run -- check 5`
//! Stuck: `cargo run -- 5`, https://docs.rs/arrow-array/60.0.0/arrow_array/struct.RecordBatch.html,
//! then `solutions/e05_schema_and_recordbatch.rs`.

use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::array::Float64Array;
use arrow::array::Float64Builder;
use arrow::array::Int64Builder;
use arrow::array::RecordBatch;
use arrow::array::RecordBatchIterator;
use arrow::array::RecordBatchReader;
use arrow::array::StringBuilder;
use arrow::datatypes::DataType;
use arrow::datatypes::Field;
use arrow::datatypes::FieldRef;
use arrow::datatypes::Schema;
use arrow::error::ArrowError;

use crate::lessons::l05_schema_and_recordbatch::trips_schema;

/// One row, the way application code holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct Trip {
    pub id: i64,
    pub city: Option<String>,
    pub fare: Option<f64>,
}

/// Exercise 1: rows -> columns. The batch's schema must be exactly `trips_schema()` (fields,
/// nullability and metadata).
///
/// 📖 lesson 05 step 1; lesson 01 (builders)
pub fn ex1_rows_to_batch(rows: &[Trip]) -> Result<RecordBatch, ArrowError> {
    todo!("ex1: one builder per column, then RecordBatch::try_new")
}

/// Exercise 2 (fix): append a nullable Float64 column "tip". This version fails on real data
/// (tips can be missing), and even when it succeeds it loses something from the schema. Keep
/// the existing columns shared, not copied.
///
/// 📖 lesson 05 steps 2 and 5
pub fn ex2_with_tip(batch: &RecordBatch, tips: Vec<Option<f64>>) -> Result<RecordBatch, ArrowError> {
    let mut fields: Vec<FieldRef> = batch.schema().fields().iter().cloned().collect();
    fields.push(Arc::new(Field::new("tip", DataType::Float64, false)));
    let mut columns: Vec<ArrayRef> = batch.columns().to_vec();
    columns.push(Arc::new(Float64Array::from(tips)));
    RecordBatch::try_new(Arc::new(Schema::new(fields)), columns)
}

/// Exercise 3: stream `batch` as zero-copy chunks of at most `max_rows` rows (no empty chunks).
///
/// 📖 lesson 05 step 4 (`slice`, `RecordBatchIterator::new`)
pub fn ex3_chunks(batch: &RecordBatch, max_rows: usize) -> impl RecordBatchReader {
    // Replace the empty iterator with the chunks.
    todo!("ex3: slices of the batch");
    #[allow(unreachable_code)]
    RecordBatchIterator::new(Vec::<Result<RecordBatch, ArrowError>>::new(), batch.schema())
}

/// Exercise 4: the columns named in `names`, in that order, sharing the original arrays and
/// keeping the schema metadata. An unknown name is an error.
///
/// 📖 lesson 05 step 3; `Schema::index_of`, `RecordBatch::project`
pub fn ex4_select(batch: &RecordBatch, names: &[&str]) -> Result<RecordBatch, ArrowError> {
    todo!("ex4: names -> indices -> project")
}
