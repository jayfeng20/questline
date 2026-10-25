//! Reference solution for `src/exercises/e03_nested_types.rs`.

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
use arrow::datatypes::Int64Type;
use arrow::error::ArrowError;

pub fn ex1_tags(rows: &[Option<Vec<&str>>]) -> ListArray {
    let mut builder = ListBuilder::new(StringBuilder::new());
    for row in rows {
        match row {
            Some(tags) => {
                for t in tags {
                    builder.values().append_value(t);
                }
                builder.append(true);
            }
            None => builder.append(false),
        }
    }
    builder.finish()
}

pub fn ex2_list_lengths(list: &ListArray) -> Vec<Option<usize>> {
    // The offsets of a slice are a window of the original ones: differences are still lengths.
    list.value_offsets()
        .windows(2)
        .enumerate()
        .map(|(i, w)| list.is_valid(i).then(|| (w[1] - w[0]) as usize))
        .collect()
}

pub fn ex3_points(xs: Vec<f64>, ys: Vec<f64>, valid: Vec<bool>) -> Result<StructArray, ArrowError> {
    let fields = Fields::from(vec![Field::new("x", DataType::Float64, false), Field::new("y", DataType::Float64, false)]);
    let nulls = valid.iter().any(|v| !v).then(|| NullBuffer::from(valid));
    StructArray::try_new(fields, vec![Arc::new(Float64Array::from(xs)), Arc::new(Float64Array::from(ys))], nulls)
}

pub fn ex4_map_get(map: &MapArray, key: &str) -> Vec<Option<i64>> {
    let keys = map.keys().as_string::<i32>();
    let values = map.values().as_primitive::<Int64Type>();
    map.value_offsets()
        .windows(2)
        .enumerate()
        .map(|(row, w)| {
            if map.is_null(row) {
                return None;
            }
            let hit = (w[0] as usize..w[1] as usize).find(|&e| keys.value(e) == key)?;
            values.is_valid(hit).then(|| values.value(hit))
        })
        .collect()
}
