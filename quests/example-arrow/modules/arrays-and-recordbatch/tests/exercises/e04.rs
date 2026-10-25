use std::sync::Arc;

use arrow::array::*;
use arrow::datatypes::*;
use arrays_and_recordbatch::exercises::e04_encodings::*;
use arrays_and_recordbatch::lessons::l04_encodings::Lcg;
use arrays_and_recordbatch::lessons::l04_encodings::fitted_size;

#[test]
fn ex1_numbers_values_in_first_seen_order() {
    let d = ex1_dictionary(&[Some("b"), Some("a"), None, Some("b"), Some("c")]).unwrap();
    assert_eq!(d.values().as_string::<i32>().iter().collect::<Vec<_>>(), [Some("b"), Some("a"), Some("c")]);
    assert_eq!(d.keys().iter().collect::<Vec<_>>(), [Some(0), Some(1), None, Some(0), Some(2)]);
}

#[test]
fn ex1_too_many_distinct_values_is_an_error() {
    let owned: Vec<String> = (0..40_000).map(|i| format!("v{i}")).collect();
    let fits: Vec<Option<&str>> = owned[..32_768].iter().map(|s| Some(s.as_str())).collect();
    assert_eq!(ex1_dictionary(&fits).unwrap().values().len(), 32_768, "Int16 keys 0..=32767 number 32 768 values");
    let too_many: Vec<Option<&str>> = owned.iter().map(|s| Some(s.as_str())).collect();
    let got = std::panic::catch_unwind(|| ex1_dictionary(&too_many));
    assert!(matches!(got, Ok(Err(_))), "40 000 distinct values cannot get Int16 keys: expected Err");
}

#[test]
fn ex2_matches_arrows_run_builder() {
    let mut rng = Lcg(9);
    for n in 0..200 {
        let values: Vec<i32> = (0..n).map(|_| (rng.next() % 3) as i32).collect();
        let (ends, runs) = ex2_run_ends(&values);
        let mut builder = PrimitiveRunBuilder::<Int32Type, Int32Type>::new();
        builder.extend(values.iter().map(|v| Some(*v)));
        let arrow = builder.finish();
        assert_eq!(ends, arrow.run_ends().values(), "run ends of {values:?}");
        assert_eq!(runs, arrow.values().as_primitive::<Int32Type>().values().as_ref(), "run values of {values:?}");
    }
}

fn shape(which: usize) -> Vec<String> {
    let mut rng = Lcg(77);
    let name = |k: u64| format!("product-name-{k:04}-padded-to-forty-bytes!")[..40].to_string();
    match which {
        0 => (0..100_000).map(|_| name(rng.next() % 1000)).collect(),
        1 => (0..100_000).map(|i| format!("id{i:06}")).collect(),
        2 => (0..1000).flat_map(|_| std::iter::repeat_n(name(rng.next() % 1000), 100)).collect(),
        _ => (0..100_000).map(|i| if i % 2 == 0 { "yes" } else { "no" }.to_string()).collect(),
    }
}

#[test]
fn ex3_prediction_matches_measured_sizes() {
    let mut measured = Vec::new();
    for which in 0..4 {
        let values = shape(which);
        let strs = values.iter().map(String::as_str);
        let sizes = [
            (Encoding::Utf8, fitted_size(StringArray::from_iter_values(strs.clone()))),
            (Encoding::Dictionary, fitted_size(strs.clone().collect::<DictionaryArray<Int32Type>>())),
            (Encoding::RunEnd, fitted_size(strs.clone().collect::<RunArray<Int32Type>>())),
            (Encoding::View, fitted_size(StringViewArray::from_iter_values(strs))),
        ];
        measured.push((sizes.iter().min_by_key(|(_, s)| *s).unwrap().0, sizes));
    }
    let predicted = ex3_predict_smallest();
    for (i, (best, sizes)) in measured.iter().enumerate() {
        assert_eq!(predicted[i], *best, "shape [{i}]: measured {sizes:?}");
    }
}

#[test]
fn ex4_decodes_both_kinds_of_null() {
    let keys = Int32Array::from(vec![Some(0), None, Some(2), Some(1), Some(0)]);
    let values = Arc::new(StringArray::from(vec![Some("x"), None, Some("z")]));
    let dict = DictionaryArray::try_new(keys, values).unwrap();
    let s = |v: &str| Some(v.to_string());
    assert_eq!(ex4_decode(&dict), [s("x"), None, s("z"), None, s("x")], "row 1: null key; row 3: key 1 -> null value");
    assert_eq!(ex4_decode(&dict.slice(2, 3)), [s("z"), None, s("x")], "a sliced dictionary");
}
