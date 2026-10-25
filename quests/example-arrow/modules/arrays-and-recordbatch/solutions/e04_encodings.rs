//! Reference solution for `src/exercises/e04_encodings.rs`.

use arrow::array::Array;
use arrow::array::AsArray;
use arrow::array::DictionaryArray;
use arrow::array::StringDictionaryBuilder;
use arrow::datatypes::Int16Type;
use arrow::datatypes::Int32Type;
use arrow::error::ArrowError;

pub fn ex1_dictionary(values: &[Option<&str>]) -> Result<DictionaryArray<Int16Type>, ArrowError> {
    let mut builder = StringDictionaryBuilder::<Int16Type>::new();
    for v in values {
        match v {
            Some(s) => {
                builder.append(s)?;
            }
            None => builder.append_null(),
        }
    }
    Ok(builder.finish())
}

pub fn ex2_run_ends(values: &[i32]) -> (Vec<i32>, Vec<i32>) {
    let (mut ends, mut runs) = (Vec::new(), Vec::new());
    for (i, v) in values.iter().enumerate() {
        if runs.last() == Some(v) {
            *ends.last_mut().unwrap() = i as i32 + 1;
        } else {
            runs.push(*v);
            ends.push(i as i32 + 1);
        }
    }
    (ends, runs)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Dictionary,
    RunEnd,
    View,
}

pub fn ex3_predict_smallest() -> [Encoding; 4] {
    // [0] 4 B keys vs 44 B/row; [1] 4 + 8 B/row beats 16 B views and any dictionary;
    // [2] 1000 runs vs 100 000 keys; [3] 4 B keys vs 4 + 2.5 B/row and 100 000 runs.
    [Encoding::Dictionary, Encoding::Utf8, Encoding::RunEnd, Encoding::Dictionary]
}

pub fn ex4_decode(dict: &DictionaryArray<Int32Type>) -> Vec<Option<String>> {
    let values = dict.values().as_string::<i32>();
    (0..dict.len())
        .map(|i| {
            let k = dict.key(i)?; // None for a null key
            values.is_valid(k).then(|| values.value(k).to_string())
        })
        .collect()
}
