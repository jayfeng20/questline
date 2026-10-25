use std::sync::Arc;

use arrow::array::*;
use arrow::datatypes::*;
use arrays_and_recordbatch::exercises::e02_downcasting_and_dispatch::*;

#[test]
fn ex1_handles_every_numeric_type() {
    let cases: Vec<ArrayRef> = vec![
        Arc::new(Int8Array::from(vec![Some(-8), None])),
        Arc::new(Int16Array::from(vec![Some(-16), None])),
        Arc::new(Int32Array::from(vec![Some(-32), None])),
        Arc::new(Int64Array::from(vec![Some(-64), None])),
        Arc::new(UInt8Array::from(vec![Some(8), None])),
        Arc::new(UInt16Array::from(vec![Some(16), None])),
        Arc::new(UInt32Array::from(vec![Some(32), None])),
        Arc::new(UInt64Array::from(vec![Some(64), None])),
        Arc::new(Float32Array::from(vec![Some(0.5), None])),
        Arc::new(Float64Array::from(vec![Some(-0.25), None])),
    ];
    let want = [-8.0, -16.0, -32.0, -64.0, 8.0, 16.0, 32.0, 64.0, 0.5, -0.25];
    for (array, w) in cases.iter().zip(want) {
        assert_eq!(ex1_to_f64(array.as_ref()).unwrap(), vec![Some(w), None], "{}", array.data_type());
    }
}

#[test]
fn ex1_rejects_non_numeric_types() {
    let cases: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(vec!["1"])),
        Arc::new(TimestampMillisecondArray::from(vec![1])),
        Arc::new(BooleanArray::from(vec![true])),
        Arc::new(Decimal128Array::from(vec![1])),
    ];
    for array in cases {
        assert!(ex1_to_f64(array.as_ref()).is_err(), "{} is not an int/float column", array.data_type());
    }
}

#[test]
fn ex2_works_for_all_three_string_layouts() {
    let rows = vec![Some("pear"), None, Some("banana"), Some("cherry"), Some("fig")];
    let layouts: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(rows.clone())),
        Arc::new(LargeStringArray::from(rows.clone())),
        Arc::new(StringViewArray::from(rows.clone())),
    ];
    for array in layouts {
        assert_eq!(ex2_longest(array.as_ref()), Some("banana".to_string()), "{}: banana and cherry tie, the first wins", array.data_type());
    }
    assert_eq!(ex2_longest(&StringArray::from(vec![None::<&str>, None])), None, "all null");
    assert_eq!(ex2_longest(&Int32Array::from(vec![1])), None, "not a string column");
}

#[test]
fn ex3_is_generic_over_primitive_types() {
    assert_eq!(ex3_count_where(&Int32Array::from(vec![Some(1), None, Some(10), Some(20)]), |v| v >= 10), 2);
    assert_eq!(ex3_count_where(&Float64Array::from(vec![f64::NAN, 1.0, -1.0]), |v| v.is_nan()), 1);
    let ts = TimestampMillisecondArray::from(vec![Some(0), Some(86_400_000), None]);
    assert_eq!(ex3_count_where(&ts, |ms| ms >= 86_400_000), 1, "timestamps are primitive arrays too");
    assert_eq!(ex3_count_where(&Int32Array::from(vec![None, None]), |_| true), 0, "nulls never count");
}

#[test]
fn ex4_prediction_matches() {
    let item = |nullable| DataType::List(Arc::new(Field::new("item", DataType::Int32, nullable)));
    let measured = [
        TimestampSecondArray::from(vec![1]).as_any().downcast_ref::<Int64Array>().is_some(),
        (&Date32Array::from(vec![1]) as &dyn Array).as_primitive_opt::<Int32Type>().is_some(),
        DataType::Decimal128(10, 2) == DataType::Decimal128(12, 2),
        item(true).equals_datatype(&item(false)),
    ];
    assert_eq!(ex4_predict(), measured);
}
