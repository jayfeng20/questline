//! Reference solution for `src/exercises/e04_hashmap_from_scratch.rs`.

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
    pub fn with_hasher(hasher: S) -> Self {
        Self { slots: Vec::new(), len: 0, tombstones: 0, hasher }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    fn home<Q: Hash + ?Sized>(&self, key: &Q) -> usize {
        // slots.len() is a power of two, so `& (n - 1)` is `% n`.
        (self.hasher.hash_one(key) as usize) & (self.slots.len() - 1)
    }

    /// Index of the Full slot holding `key`, if any. Stops only at Empty, never at Deleted.
    fn find<Q>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.slots.is_empty() {
            return None;
        }
        let mask = self.slots.len() - 1;
        let mut i = self.home(key);
        for _ in 0..self.slots.len() {
            match &self.slots[i] {
                Slot::Empty => return None,
                Slot::Full(k, _) if k.borrow() == key => return Some(i),
                _ => i = (i + 1) & mask,
            }
        }
        None
    }

    fn resize(&mut self, new_slots: usize) {
        let old = std::mem::replace(&mut self.slots, (0..new_slots).map(|_| Slot::Empty).collect());
        self.len = 0;
        self.tombstones = 0; // rehashing drops every tombstone
        for slot in old {
            if let Slot::Full(k, v) = slot {
                self.insert(k, v);
            }
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.insert_at(key, value).1
    }

    /// Insert and report which slot now holds the key (and the replaced value, if any).
    fn insert_at(&mut self, key: K, value: V) -> (usize, Option<V>) {
        if (self.len + self.tombstones + 1) * 8 > self.slots.len() * 7 {
            // Mostly tombstones? Rehash at the same size (clears them) instead of growing.
            let grow = (self.len + 1) * 2 > self.slots.len();
            self.resize(if grow { (self.slots.len() * 2).max(8) } else { self.slots.len() });
        }
        let mask = self.slots.len() - 1;
        let mut i = self.home(&key);
        let mut first_tombstone = None;
        loop {
            match &mut self.slots[i] {
                Slot::Full(k, v) if *k == key => return (i, Some(std::mem::replace(v, value))),
                Slot::Full(..) => {}
                Slot::Deleted => {
                    first_tombstone.get_or_insert(i);
                }
                Slot::Empty => break,
            }
            i = (i + 1) & mask;
        }
        let target = match first_tombstone {
            Some(t) => {
                self.tombstones -= 1;
                t
            }
            None => i,
        };
        self.slots[target] = Slot::Full(key, value);
        self.len += 1;
        (target, None)
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match &self.slots[self.find(key)?] {
            Slot::Full(_, v) => Some(v),
            _ => None,
        }
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let i = self.find(key)?;
        match &mut self.slots[i] {
            Slot::Full(_, v) => Some(v),
            _ => None,
        }
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let i = self.find(key)?;
        match std::mem::replace(&mut self.slots[i], Slot::Deleted) {
            Slot::Full(_, v) => {
                self.len -= 1;
                self.tombstones += 1;
                Some(v)
            }
            _ => unreachable!("find returns Full slots only"),
        }
    }

    pub fn get_or_insert(&mut self, key: K, default: V) -> &mut V {
        let i = match self.find(&key) {
            Some(i) => i,
            None => self.insert_at(key, default).0,
        };
        match &mut self.slots[i] {
            Slot::Full(_, v) => v,
            _ => unreachable!(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.slots.iter().filter_map(|s| match s {
            Slot::Full(k, v) => Some((k, v)),
            _ => None,
        })
    }

    pub fn max_probe_len(&self) -> usize {
        let n = self.slots.len();
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| match s {
                Slot::Full(k, _) => Some((i + n - self.home(k)) % n + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }
}
