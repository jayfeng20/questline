//! Reference solution for `src/exercises/e01_typed_arrays_and_builders.rs`.

use arrow::array::Int32Array;
use arrow::array::ArrayBuilder;
use arrow::array::Int32Builder;
use arrow::array::Int64Array;
use arrow::array::Int64Builder;
use arrow::array::TimestampMillisecondArray;

pub fn ex1_parse_ints(raw: &[&str]) -> Int32Array {
    let mut builder = Int32Builder::with_capacity(raw.len());
    for s in raw {
        builder.append_option(s.parse::<i32>().ok());
    }
    builder.finish()
}

pub fn ex2_event_times(epoch_ms: Vec<i64>, tz: &str) -> TimestampMillisecondArray {
    TimestampMillisecondArray::from(epoch_ms).with_timezone(tz)
}

pub fn ex3_chunks(values: &[i64], chunk: usize) -> Vec<Int64Array> {
    let mut builder = Int64Builder::with_capacity(chunk);
    let mut out = Vec::new();
    for v in values {
        builder.append_value(*v);
        if builder.len() == chunk {
            // finish() resets the builder; finish_cloned() kept every earlier row.
            out.push(builder.finish());
        }
    }
    if builder.len() > 0 {
        out.push(builder.finish());
    }
    out
}

pub fn ex4_predict() -> (usize, Option<i32>, usize) {
    // The null still has a slot (holding 0), so values() has 3 entries and its min is -5; only 2 are valid.
    (3, Some(-5), 2)
}
