use std::sync::Arc;

use arrow::array::*;
use arrow::datatypes::*;
use arrays_and_recordbatch::exercises::e03_nested_types::*;

fn as_rows(list: &ListArray) -> Vec<Option<Vec<String>>> {
    list.iter().map(|row| row.map(|a| a.as_string::<i32>().iter().map(|s| s.unwrap().to_string()).collect())).collect()
}

#[test]
fn ex1_builds_null_empty_and_full_lists() {
    let rows = vec![Some(vec!["rust", "arrow"]), None, Some(vec![]), Some(vec!["x"])];
    let list = ex1_tags(&rows);
    assert_eq!(list.data_type(), &DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))));
    assert_eq!(list.value_offsets(), [0, 2, 2, 2, 3]);
    assert_eq!(as_rows(&list), vec![Some(vec!["rust".to_string(), "arrow".to_string()]), None, Some(vec![]), Some(vec!["x".to_string()])]);
    assert!(list.is_null(1) && list.is_valid(2), "row 1 is a null list, row 2 an empty one");
}

#[test]
fn ex2_lengths_from_offsets_even_when_sliced() {
    let list = ListArray::from_iter_primitive::<Int32Type, _, _>((0..50).map(|i| (i % 4 != 1).then(|| vec![Some(i); (i % 5) as usize])));
    let oracle = |l: &ListArray| -> Vec<Option<usize>> { l.iter().map(|v| v.map(|a| a.len())).collect() };
    assert_eq!(ex2_list_lengths(&list), oracle(&list));
    for (start, len) in [(0, 0), (7, 13), (49, 1)] {
        let s = list.slice(start, len);
        assert_eq!(ex2_list_lengths(&s), oracle(&s), "slice({start}, {len})");
    }
}

#[test]
fn ex3_struct_nulls_only_when_needed() {
    let p = ex3_points(vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0], vec![true, false, true]).unwrap();
    assert_eq!(p.data_type().to_string(), r#"Struct("x": non-null Float64, "y": non-null Float64)"#);
    assert_eq!(p.nulls().map(|n| n.iter().collect::<Vec<_>>()), Some(vec![true, false, true]));
    assert_eq!(p.column(1).as_primitive::<Float64Type>().values().as_ref(), &[4.0, 5.0, 6.0], "fields keep their values under a null row");
    let all_valid = ex3_points(vec![1.0], vec![2.0], vec![true]).unwrap();
    assert!(all_valid.nulls().is_none(), "every row valid: no NullBuffer");
    let mismatch = std::panic::catch_unwind(|| ex3_points(vec![1.0, 2.0], vec![1.0], vec![true, true]));
    assert!(matches!(mismatch, Ok(Err(_))), "mismatched lengths must be Err, not a panic or Ok");
}

#[test]
fn ex4_looks_up_per_row() {
    let rows: Vec<Option<Vec<(&str, Option<i64>)>>> = vec![
        Some(vec![("a", Some(1)), ("b", Some(2))]),
        None,
        Some(vec![]),
        Some(vec![("b", None), ("a", Some(30)), ("a", Some(31))]),
        Some(vec![("c", Some(5))]),
    ];
    let mut builder = MapBuilder::new(None, StringBuilder::new(), Int64Builder::new());
    for row in &rows {
        if let Some(entries) = row {
            for (k, v) in entries {
                builder.keys().append_value(k);
                builder.values().append_option(*v);
            }
        }
        builder.append(row.is_some()).unwrap();
    }
    let map = builder.finish();
    assert_eq!(ex4_map_get(&map, "a"), [Some(1), None, None, Some(30), None], "first match wins");
    assert_eq!(ex4_map_get(&map, "b"), [Some(2), None, None, None, None], "a present key with a null value is None");
    assert_eq!(ex4_map_get(&map.slice(3, 2), "a"), [Some(30), None], "works on a sliced map");
}
