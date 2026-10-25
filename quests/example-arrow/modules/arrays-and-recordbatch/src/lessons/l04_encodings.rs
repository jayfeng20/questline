//! # Lesson 04 — Encodings: Dictionary, Run-End, View
//!
//! ## Lecture
//!
//! One logical column of strings can be stored four ways, and the right choice depends on the
//! *shape* of the data, not on its type:
//!
//! | encoding                    | physical layout                        | small when                     | pay with                                  |
//! |-----------------------------|----------------------------------------|--------------------------------|-------------------------------------------|
//! | `Utf8` (plain)              | offsets + bytes                        | values are mostly unique       | repeated strings are stored again         |
//! | `Dictionary<K, Utf8>`       | K keys + the distinct values           | few distinct values, any order | one key per row; K caps the distinct count (Int8 → 128) |
//! | `RunEndEncoded<R, Utf8>`    | run ends (R) + one value per run       | long runs (sorted, clustered)  | value `i` = binary search over run ends   |
//! | `Utf8View`                  | 16-byte views (+ data buffers)         | short strings, or shared bytes | 16 B per row before any bytes             |
//!
//! `get_array_memory_size()` counts *capacity*, and builders reserve from the iterator's size hint:
//! measure after `shrink_to_fit()` when you compare encodings.
//!
//! `len()` is always the *logical* length. The physical size hides in the children:
//! `dict.values().len()` is the number of distinct values, `ree.values().len()` the number of runs.
//!
//! Nulls get physical too. A dictionary slot is null if its key is null (`nulls()`) *or* its key
//! points at a null value. A run-end array has no validity buffer at all: nulls are values.
//! `logical_nulls()` / `logical_null_count()` compute the answer that matches what `iter()` shows.
//!
//! ## Reading list
//!   * https://arrow.apache.org/docs/format/Columnar.html#dictionary-encoded-layout
//!   * https://arrow.apache.org/docs/format/Columnar.html#run-end-encoded-layout
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/array/struct.DictionaryArray.html
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/array/struct.RunArray.html#method.get_physical_index
//!   * https://docs.rs/arrow-array/60.0.0/arrow_array/builder/struct.GenericByteViewBuilder.html#method.with_deduplicate_strings
//!   * source: ~/.cargo/registry/src/*/arrow-array-60.0.0/src/array/run_array.rs (`logical_nulls`)

use std::sync::Arc;

use arrow::array::Array;
use arrow::array::DictionaryArray;
use arrow::array::Int32Array;
use arrow::array::RunArray;
use arrow::array::StringArray;
use arrow::array::StringDictionaryBuilder;
use arrow::array::StringViewArray;
use arrow::array::StringViewBuilder;
use arrow::datatypes::Int8Type;
use arrow::datatypes::Int32Type;

use crate::util::{banner, check, human_bytes, note, section, try_it};

/// Deterministic pseudo-random numbers (64-bit LCG).
pub struct Lcg(pub u64);
impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

/// The three column shapes this lesson (and exercise 3) measures, 100 000 rows each.
pub fn countries() -> Vec<String> {
    let mut rng = Lcg(42);
    (0..100_000).map(|_| format!("country-{:02}", rng.next() % 20)).collect()
}
pub fn sorted_statuses() -> Vec<String> {
    ["active", "blocked", "closed", "pending", "trial"].iter().flat_map(|s| std::iter::repeat_n(s.to_string(), 20_000)).collect()
}
pub fn urls() -> Vec<String> {
    (0..100_000).map(|i| format!("https://example.com/item/{i:08}")).collect()
}

/// Memory of an array after `shrink_to_fit()`: the encoding's size, not the builder's slack.
pub fn fitted_size(mut array: impl Array) -> usize {
    array.shrink_to_fit();
    array.get_array_memory_size()
}

/// Fitted size of `values` as (Utf8, Dictionary<Int32>, RunEnd<Int32>, Utf8View).
pub fn sizes(values: &[String]) -> [usize; 4] {
    let strs = values.iter().map(String::as_str);
    [
        fitted_size(StringArray::from_iter_values(strs.clone())),
        fitted_size(strs.clone().collect::<DictionaryArray<Int32Type>>()),
        fitted_size(strs.clone().collect::<RunArray<Int32Type>>()),
        fitted_size(StringViewArray::from_iter_values(strs)),
    ]
}

pub fn run() -> anyhow::Result<()> {
    banner("Lesson 04 — Encodings: Dictionary, Run-End, View");

    // STEP 1: one table of sizes, three data shapes.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/array/trait.Array.html#tymethod.get_array_memory_size
    section("1. Same strings, four encodings");
    let shapes = [("20 countries, random order", countries()), ("5 statuses, sorted", sorted_statuses()), ("unique URLs", urls())];
    println!("   {:<28} {:>11} {:>11} {:>11} {:>11}", "100k rows of", "Utf8", "Dict<i32>", "REE<i32>", "Utf8View");
    let mut table = Vec::new();
    for (name, values) in &shapes {
        let s = sizes(values);
        println!("   {name:<28} {:>11} {:>11} {:>11} {:>11}", human_bytes(s[0] as u64), human_bytes(s[1] as u64), human_bytes(s[2] as u64), human_bytes(s[3] as u64));
        table.push(s);
    }
    let smallest = |s: &[usize; 4]| (0..4).min_by_key(|&i| s[i]).unwrap();
    check(smallest(&table[0]) == 1 && table[0][1] * 3 < table[0][0], "low cardinality, any order: Dictionary wins (4-byte keys vs ~14 bytes per row)")?;
    check(smallest(&table[1]) == 2 && table[1][2] * 100 < table[1][1], "sorted runs: Run-End wins by orders of magnitude (5 runs)")?;
    check(smallest(&table[2]) == 0 && table[2][1] > table[2][0], "unique values: plain Utf8 wins; a dictionary only adds keys")?;
    let unfitted: RunArray<Int32Type> = shapes[1].1.iter().map(String::as_str).collect();
    println!("   REE of the sorted statuses straight from collect(): {}", human_bytes(unfitted.get_array_memory_size() as u64));
    check(unfitted.get_array_memory_size() > 100 * table[1][2], "memory sizes count capacity: from_iter reserved room for 100 000 runs until shrink_to_fit()")?;

    // STEP 2: physical vs logical length.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/array/struct.DictionaryArray.html#method.values
    section("2. Logical length vs physical children");
    let dict: DictionaryArray<Int32Type> = shapes[0].1.iter().map(String::as_str).collect();
    let ree: RunArray<Int32Type> = shapes[1].1.iter().map(String::as_str).collect();
    println!("   dictionary: len {}, keys {}, distinct values {}", dict.len(), dict.keys().len(), dict.values().len());
    println!("   run-end:    len {}, run_ends {:?}, values {}", ree.len(), ree.run_ends().values(), ree.values().len());
    check(dict.len() == 100_000 && dict.values().len() == 20, "a dictionary stores 100 000 keys but only 20 strings")?;
    check(ree.len() == 100_000 && ree.run_ends().values() == [20_000, 40_000, 60_000, 80_000, 100_000], "run ends are cumulative row counts: 5 runs cover 100 000 rows")?;
    let phys = ree.get_physical_index(50_000);
    check(phys == 2, "row 50 000 sits in run 2 (rows 40 000..60 000): a binary search over run ends")?;

    // STEP 3: the key type caps the dictionary.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/builder/type.StringDictionaryBuilder.html
    section("3. Key overflow");
    let mut small = StringDictionaryBuilder::<Int8Type>::new();
    let fits = (0..128).all(|i| small.append(format!("v{i}")).is_ok());
    let overflow = small.append("one too many");
    println!("   Int8 keys: 128 distinct values appended: {fits}; the 129th: {:?}", overflow.as_ref().map_err(|e| e.to_string()));
    check(fits && overflow.is_err(), "Int8 keys hold 128 distinct values (0..=127); the 129th is an error, not a wrap")?;
    check(small.append("v5").is_ok(), "repeats of existing values still fit: the cap is on distinct values")?;

    // STEP 4: physical nulls vs logical nulls.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/array/trait.Array.html#method.logical_nulls
    section("4. Where do nulls live?");
    let keys = Int32Array::from(vec![Some(0), None, Some(1), Some(0)]);
    let d = DictionaryArray::try_new(keys, Arc::new(StringArray::from(vec![Some("a"), None])))?;
    println!("   dict keys [0, null, 1, 0] over values [a, null]: null_count {}, logical_null_count {}", d.null_count(), d.logical_null_count());
    check(d.null_count() == 1 && d.logical_null_count() == 2, "null_count sees only the null key; logical_null_count also sees key 1 -> null value")?;
    let r: RunArray<Int32Type> = [Some("a"), Some("a"), None, None, Some("b")].into_iter().collect();
    println!("   REE [a, a, null, null, b]: nulls() {:?}, logical_null_count {}", r.nulls().map(|n| n.null_count()), r.logical_null_count());
    check(r.nulls().is_none() && r.logical_null_count() == 2, "a run-end array has no validity buffer: its nulls are a null run value")?;
    note("Kernels that ask is_null(i) on these types get the physical answer. Use logical_nulls() for the truth.");

    // STEP 5: views can share bytes too.
    //   📖 https://docs.rs/arrow-array/60.0.0/arrow_array/builder/struct.GenericByteViewBuilder.html#method.with_deduplicate_strings
    section("5. Utf8View with deduplicated strings");
    let long_categories: Vec<String> = (0..100_000).map(|i| format!("a long category name, number {:02}", i % 20)).collect();
    let mut plain = StringViewBuilder::new();
    let mut dedup = StringViewBuilder::new().with_deduplicate_strings();
    for s in &long_categories {
        plain.append_value(s);
        dedup.append_value(s);
    }
    let (plain, dedup) = (plain.finish(), dedup.finish());
    let data_bytes = |a: &StringViewArray| a.data_buffers().iter().map(|b| b.len()).sum::<usize>();
    let len = long_categories[0].len();
    println!("   100k rows, 20 distinct {len}-byte strings: data bytes {} plain vs {} deduplicated", data_bytes(&plain), data_bytes(&dedup));
    check(data_bytes(&dedup) == 20 * len && data_bytes(&plain) == 100_000 * len, "deduplicated views point at one copy of each distinct string")?;
    check(dedup.iter().eq(plain.iter()), "logically the same column")?;

    try_it("sizes(&countries()) with Dictionary<Int8Type> keys: how much smaller than Int32 keys?");
    try_it("shuffle sorted_statuses() and rerun step 1: what happens to the Run-End size?");
    try_it("time ree.get_physical_index(i) for every i vs reading dict.keys().value(i) (--release)");
    Ok(())
}
