//! Reference solution for `src/exercises/e05_schema_and_recordbatch.rs`.

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

#[derive(Debug, Clone, PartialEq)]
pub struct Trip {
    pub id: i64,
    pub city: Option<String>,
    pub fare: Option<f64>,
}

pub fn ex1_rows_to_batch(rows: &[Trip]) -> Result<RecordBatch, ArrowError> {
    let mut ids = Int64Builder::with_capacity(rows.len());
    let mut cities = StringBuilder::new();
    let mut fares = Float64Builder::with_capacity(rows.len());
    for t in rows {
        ids.append_value(t.id);
        cities.append_option(t.city.as_deref());
        fares.append_option(t.fare);
    }
    let columns: Vec<ArrayRef> = vec![Arc::new(ids.finish()), Arc::new(cities.finish()), Arc::new(fares.finish())];
    RecordBatch::try_new(trips_schema(), columns)
}

pub fn ex2_with_tip(batch: &RecordBatch, tips: Vec<Option<f64>>) -> Result<RecordBatch, ArrowError> {
    let mut fields: Vec<FieldRef> = batch.schema().fields().iter().cloned().collect();
    fields.push(Arc::new(Field::new("tip", DataType::Float64, true)));
    let mut columns: Vec<ArrayRef> = batch.columns().to_vec(); // Arc clones: shared, not copied
    columns.push(Arc::new(Float64Array::from(tips)));
    // Schema::new would drop the schema-level metadata.
    let schema = Schema::new_with_metadata(fields, batch.schema().metadata().clone());
    RecordBatch::try_new(Arc::new(schema), columns)
}

pub fn ex3_chunks(batch: &RecordBatch, max_rows: usize) -> impl RecordBatchReader {
    let n = batch.num_rows();
    let chunks: Vec<Result<RecordBatch, ArrowError>> = (0..n).step_by(max_rows).map(|start| Ok(batch.slice(start, max_rows.min(n - start)))).collect();
    RecordBatchIterator::new(chunks, batch.schema())
}

pub fn ex4_select(batch: &RecordBatch, names: &[&str]) -> Result<RecordBatch, ArrowError> {
    let schema = batch.schema();
    let indices = names.iter().map(|n| schema.index_of(n)).collect::<Result<Vec<_>, _>>()?;
    batch.project(&indices)
}
