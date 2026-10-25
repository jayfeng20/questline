//! # Exercises — lesson 04: build a HashMap
//!
//! Implement every `todo!()` in `MyHashMap`. The representation is given (open addressing,
//! linear probing, tombstones); the algorithms are yours.
//!
//! Check: `cargo run -- check 4` (the tests compare you against std's HashMap on
//! thousands of random operations, and against hand-built collision chains).
//! Stuck: `cargo run -- 4` step 4, https://en.wikipedia.org/wiki/Linear_probing, then
//! `solutions/e04_hashmap_from_scratch.rs`.

use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;
use std::hash::Hash;

pub enum Slot<K, V> {
    Empty,
    /// A removed entry: lookups must probe past it, inserts may reuse it.
    Deleted,
    Full(K, V),
}

pub struct MyHashMap<K, V, S = RandomState> {
    /// Always empty or a power of two in length.
    slots: Vec<Slot<K, V>>,
    /// Number of `Full` slots.
    len: usize,
    /// Number of `Deleted` slots (they also count towards the load factor).
    tombstones: usize,
    hasher: S,
}

impl<K: Hash + Eq, V> MyHashMap<K, V, RandomState> {
    pub fn new() -> Self {
        Self::with_hasher(RandomState::new())
    }
}

impl<K: Hash + Eq, V> Default for MyHashMap<K, V, RandomState> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> MyHashMap<K, V, S> {
    /// An empty map that has not allocated.
    pub fn with_hasher(hasher: S) -> Self {
        Self { slots: Vec::new(), len: 0, tombstones: 0, hasher }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Number of slots (0 before the first insert, then a power of two >= 8).
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Exercise 1: insert, returning the previous value for an equal key. Before inserting,
    /// when `len + tombstones + 1 > slots * 7 / 8`, rebuild the table: double it (8 slots
    /// minimum) if live entries would exceed half of it, otherwise rehash at the SAME size,
    /// which clears the tombstones (else insert/remove churn grows the table forever).
    ///
    /// 💡 Probe from `hash % slots`. Remember the first tombstone you pass: reuse it if the key
    ///    turns out to be absent (you only know that when you reach an Empty slot).
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        todo!("ex1: maybe resize, then probe")
    }

    /// Exercise 2: look up by any borrowed form of the key (`&str` for `String` keys).
    ///
    /// 📖 lesson 04 step 3; https://doc.rust-lang.org/std/borrow/trait.Borrow.html
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        todo!("ex2: probe until the key or an Empty slot")
    }

    /// Exercise 3: like `get`, but mutable.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        todo!("ex3: find the slot index, then borrow mutably")
    }

    /// Exercise 4: remove and return the value, leaving a tombstone.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        todo!("ex4: find, replace the slot with Slot::Deleted")
    }

    /// Exercise 5: the `entry(key).or_insert(default)` idiom, as one method: a mutable
    /// reference to the value for `key`, inserting `default` first if absent.
    pub fn get_or_insert(&mut self, key: K, default: V) -> &mut V {
        todo!("ex5: insert if missing, then get_mut")
    }

    /// Exercise 6: iterate over all entries (any order).
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        // Replace `std::iter::empty()` with an iterator over the Full slots.
        todo!("ex6: filter_map over the slots");
        #[allow(unreachable_code)]
        std::iter::empty()
    }

    /// The longest probe sequence any stored key needs (1 = found in its home slot).
    /// Exercise 7: implement it; the test uses it to show what a bad hasher does.
    pub fn max_probe_len(&self) -> usize {
        todo!("ex7: for each Full slot, distance from its home slot + 1")
    }
}
