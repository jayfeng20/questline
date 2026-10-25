use arrow::array::Array;
use arrow::array::Int32Array;
use arrow::datatypes::DataType;
use arrow::datatypes::TimeUnit;
use arrays_and_recordbatch::exercises::e01_typed_arrays_and_builders::*;

#[test]
fn ex1_parses_and_nulls_the_rest() {
    let a = ex1_parse_ints(&["1", "-20", "", "x", "3000000000", "+7", " 8"]);
    assert_eq!(a.iter().collect::<Vec<_>>(), [Some(1), Some(-20), None, None, None, Some(7), None], "3000000000 overflows i32; ' 8' has a space");
    assert_eq!(a.null_count(), 4);
    assert_eq!(ex1_parse_ints(&[]).len(), 0);
    assert!(ex1_parse_ints(&["1", "2"]).nulls().is_none(), "no nulls appended: no bitmap");
}

#[test]
fn ex2_is_a_zero_copy_timestamp_column() {
    let ms = vec![0, 1_000, 1_700_000_000_000];
    let ptr = ms.as_ptr();
    let ts = ex2_event_times(ms, "+02:00");
    assert_eq!(ts.data_type(), &DataType::Timestamp(TimeUnit::Millisecond, Some("+02:00".into())));
    assert_eq!(ts.values().as_ptr(), ptr, "the i64s must not be copied");
    assert_eq!(ts.value(2), 1_700_000_000_000);
}

#[test]
fn ex3_each_chunk_has_only_its_own_rows() {
    let values: Vec<i64> = (0..10).collect();
    let chunks = ex3_chunks(&values, 4);
    let got: Vec<Vec<i64>> = chunks.iter().map(|a| a.values().to_vec()).collect();
    assert_eq!(got, [vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9]], "chunks of 4, then the remaining 2");
    assert_eq!(ex3_chunks(&values[..8], 4).len(), 2, "8 values = exactly 2 chunks, no empty third one");
    assert!(ex3_chunks(&[], 4).is_empty());
}

#[test]
fn ex4_prediction_matches() {
    let a = Int32Array::from(vec![Some(-5), None, Some(-3)]);
    let measured = (a.values().len(), a.values().iter().copied().min(), a.iter().flatten().count());
    assert_eq!(ex4_predict(), measured);
}
