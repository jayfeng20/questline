//! # Lesson 04 — How a HashMap works (then you build one)
//!
//! ## Lecture
//!
//! A hash map is an array of *slots*. To store a key, hash it (`BuildHasher` → `Hasher` → `u64`),
//! reduce the hash to a slot index, and if that slot is taken, *probe* for another one.
//!
//! | concern        | std::collections::HashMap (hashbrown, a SwissTable)      | the map you will build            |
//! |----------------|-----------------------------------------------------------|-----------------------------------|
//! | hashing        | `RandomState` (SipHash-1-3, random keys per map: HashDoS-resistant) | any `S: BuildHasher`, default `RandomState` |
//! | collisions     | open addressing, probes 16 control bytes at a time with SIMD | open addressing, linear probing   |
//! | capacity       | power-of-two buckets, max load factor 7/8                 | same                              |
//! | removal        | marks the slot DELETED (a tombstone)                      | tombstones (`Slot::Deleted`)      |
//! | lookups by borrowed key | `K: Borrow<Q>`: a `HashMap<String, _>` is queried with `&str` | same |
//!
//! Two invariants make it correct: equal keys must hash equally (`Hash` agrees with `Eq`,
//! and with `Borrow`), and a lookup may stop only at an EMPTY slot, never at a tombstone,
//! because the key it is looking for may have been pushed past the slot that was later freed.
//!
//! ## Reading list
//!   * https://doc.rust-lang.org/std/collections/struct.HashMap.html
//!   * https://doc.rust-lang.org/std/borrow/trait.Borrow.html (why `get(&str)` works on `String` keys)
//!   * hashbrown design: https://github.com/rust-lang/hashbrown and the CppCon 2017 talk
//!     "Designing a Fast, Efficient, Cache-friendly Hash Table" (Matt Kulukundis)
//!   * https://en.wikipedia.org/wiki/Linear_probing

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::hash::BuildHasherDefault;
use std::hash::Hasher;

use crate::util::{banner, check, note, section, try_it};

/// A hasher that returns the key itself (for u64 keys): makes collisions predictable for demos
/// and tests. Never use it for real data.
#[derive(Default)]
pub struct IdentityHasher(u64);

impl Hasher for IdentityHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 << 8) | u64::from(*b);
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }
}

pub type BuildIdentity = BuildHasherDefault<IdentityHasher>;

pub fn run() -> anyhow::Result<()> {
    banner("Lesson 04 — How a HashMap works");

    // STEP 1: std grows in powers of two at a 7/8 load factor.
    //   📖 https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.capacity
    section("1. When does std's HashMap resize?");
    let mut m = HashMap::new();
    let mut growth = vec![(0usize, m.capacity())];
    for i in 0..200u32 {
        m.insert(i, i);
        if m.capacity() != growth.last().unwrap().1 {
            growth.push((m.len(), m.capacity()));
        }
    }
    println!("   (len when it grew, new capacity): {growth:?}");
    check(
        growth.iter().skip(1).all(|(_, c)| if *c < 7 { (c + 1).is_power_of_two() } else { (c * 8 / 7).is_power_of_two() }),
        "capacity = 7/8 of a power-of-two bucket count (bucket count - 1 for tiny tables)",
    )?;
    note("capacity() is how many items fit before the next resize, not the bucket count.");

    // STEP 2: hashing is per-map random by default.
    section("2. RandomState: same keys, different order");
    let a: HashMap<u32, ()> = (0..8).map(|i| (i, ())).collect();
    let b: HashMap<u32, ()> = (0..8).map(|i| (i, ())).collect();
    let (oa, ob): (Vec<_>, Vec<_>) = (a.keys().copied().collect(), b.keys().copied().collect());
    println!("   map a iterates {oa:?}\n   map b iterates {ob:?}");
    let s = std::collections::hash_map::RandomState::new();
    check(s.hash_one(42u32) == s.hash_one(42u32), "within one map, a key always hashes the same")?;
    note("Different random keys per map is what makes HashDoS attacks impractical. It is also");
    note("why you must never depend on HashMap iteration order.");

    // STEP 3: Borrow lets a String-keyed map be queried with &str (no allocation).
    section("3. Lookups through Borrow");
    let mut names: HashMap<String, u32> = HashMap::new();
    names.insert("ann".to_string(), 1);
    check(names.get("ann") == Some(&1), "a HashMap<String, _> answers get(&str)")?;
    check(s.hash_one("ann") == s.hash_one("ann".to_string()), "str and String hash identically (the Borrow contract)")?;

    // STEP 4: linear probing, visualised. 8 slots, identity hash, slot = hash % 8.
    section("4. Linear probing and why removal needs tombstones");
    let mut slots: [Option<u64>; 8] = [None; 8];
    for key in [3u64, 11, 19, 4] {
        let mut i = (BuildIdentity::default().hash_one(key) % 8) as usize;
        let mut probes = 1;
        while slots[i].is_some() {
            i = (i + 1) % 8;
            probes += 1;
        }
        slots[i] = Some(key);
        println!("   insert {key:>2}: home slot {}, landed in {i} after {probes} probe(s)", key % 8);
    }
    println!("   slots: {slots:?}");
    note("11 and 19 both hash to slot 3 and were pushed to 4 and 5; then 4 (home slot 4) went to 6.");
    note("If removing 11 made slot 4 EMPTY, a lookup for 19 would stop at slot 4 and miss it.");
    note("So removal leaves a tombstone: lookups skip it, inserts may reuse it.");
    check(slots[5] == Some(19) && slots[6] == Some(4), "colliding keys spill into the following slots")?;

    try_it("insert 1M u64 keys into HashMap<u64, u64, BuildIdentity> and into the default map; time both.");
    try_it("why is the identity hasher terrible for keys that are all multiples of 1024? (slot = hash % capacity)");
    Ok(())
}
