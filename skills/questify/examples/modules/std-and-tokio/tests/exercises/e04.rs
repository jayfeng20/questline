use std::collections::HashMap;

use std_and_tokio::exercises::e04_hashmap_from_scratch::*;
use std_and_tokio::lessons::l04_hashmap_from_scratch::BuildIdentity;

/// Deterministic pseudo-random numbers without a dependency (64-bit LCG).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

#[test]
fn ex1_to_ex4_agree_with_std_on_random_operations() {
    let mut mine: MyHashMap<u64, u64> = MyHashMap::new();
    let mut std_map: HashMap<u64, u64> = HashMap::new();
    let mut rng = Lcg(7);
    for step in 0..20_000 {
        let key = rng.next() % 500; // small key space: many overwrites and removes
        match rng.next() % 4 {
            0 | 1 => assert_eq!(mine.insert(key, step), std_map.insert(key, step), "insert({key}) at step {step}"),
            2 => assert_eq!(mine.remove(&key), std_map.remove(&key), "remove({key}) at step {step}"),
            _ => assert_eq!(mine.get(&key), std_map.get(&key), "get({key}) at step {step}"),
        }
        assert_eq!(mine.len(), std_map.len(), "len after step {step}");
    }
}

#[test]
fn ex1_grows_by_doubling_at_seven_eighths() {
    let mut m: MyHashMap<u32, ()> = MyHashMap::new();
    assert_eq!(m.slot_count(), 0, "no allocation before the first insert");
    m.insert(0, ());
    assert_eq!(m.slot_count(), 8);
    for i in 1..7 {
        m.insert(i, ());
    }
    assert_eq!(m.slot_count(), 8, "7 entries fit in 8 slots (7/8)");
    m.insert(7, ());
    assert_eq!(m.slot_count(), 16, "the 8th entry doubles the table");
    for i in 8..1000 {
        m.insert(i, ());
        assert!(m.slot_count().is_power_of_two() && m.len() * 8 <= m.slot_count() * 7);
    }
}

#[test]
fn ex2_borrowed_lookups() {
    let mut m: MyHashMap<String, u32> = MyHashMap::new();
    m.insert("ann".to_string(), 1);
    m.insert("bob".to_string(), 2);
    assert_eq!(m.get("ann"), Some(&1), "query a String-keyed map with &str");
    assert_eq!(m.get("zed"), None);
    *m.get_mut("bob").unwrap() += 40;
    assert_eq!(m.get("bob"), Some(&42));
}

#[test]
fn ex4_removal_keeps_later_keys_in_the_chain_reachable() {
    // Identity hash, 8 slots: 3, 11 and 19 all have home slot 3 and form one probe chain.
    let mut m: MyHashMap<u64, &str, BuildIdentity> = MyHashMap::with_hasher(BuildIdentity::default());
    m.insert(3, "a");
    m.insert(11, "b");
    m.insert(19, "c");
    assert_eq!(m.remove(&11), Some("b"));
    assert_eq!(m.get(&19), Some(&"c"), "19 sits after 11's old slot: lookups must probe past the tombstone");
    m.insert(27, "d"); // may reuse the tombstone
    assert_eq!((m.get(&3), m.get(&19), m.get(&27), m.len()), (Some(&"a"), Some(&"c"), Some(&"d"), 3));
}

#[test]
fn ex4_tombstones_do_not_leak() {
    let mut m: MyHashMap<u64, u64> = MyHashMap::new();
    for round in 0..10_000u64 {
        m.insert(round, round);
        m.remove(&round);
    }
    assert!(m.is_empty());
    assert!(m.slot_count() <= 64, "insert/remove churn must not grow the table forever: {} slots", m.slot_count());
}

#[test]
fn ex5_get_or_insert_counts_words() {
    let mut counts: MyHashMap<&str, u32> = MyHashMap::new();
    for w in "the cat and the hat and the bat".split(' ') {
        *counts.get_or_insert(w, 0) += 1;
    }
    let mut got: Vec<_> = counts.iter().map(|(k, v)| (*k, *v)).collect();
    got.sort();
    assert_eq!(got, vec![("and", 2), ("bat", 1), ("cat", 1), ("hat", 1), ("the", 3)]);
}

#[test]
fn ex6_iter_yields_every_entry_once() {
    let mut m: MyHashMap<u32, u32> = MyHashMap::new();
    for i in 0..300 {
        m.insert(i, i * 2);
    }
    for i in (0..300).step_by(3) {
        m.remove(&i);
    }
    let mut keys: Vec<u32> = m.iter().map(|(k, v)| {
        assert_eq!(*v, k * 2);
        *k
    }).collect();
    keys.sort();
    assert_eq!(keys, (0..300).filter(|i| i % 3 != 0).collect::<Vec<_>>());
}

#[test]
fn ex7_a_bad_hasher_makes_long_probe_chains() {
    let mut good: MyHashMap<u64, ()> = MyHashMap::new();
    let mut bad: MyHashMap<u64, (), BuildIdentity> = MyHashMap::with_hasher(BuildIdentity::default());
    for i in 0..1000u64 {
        good.insert(i * 1024, ());
        bad.insert(i * 1024, ()); // low 10 bits are 0: with 2048 slots only 2 home slots exist
    }
    assert!(good.max_probe_len() < 30, "SipHash spreads keys: max probe {}", good.max_probe_len());
    assert!(bad.max_probe_len() > 400, "identity hash of multiples of 1024 collides: max probe {}", bad.max_probe_len());
}
