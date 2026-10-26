//! # Lesson 01 — EXAMPLE (replace me): how `Vec` grows
//!
//! ## Lecture
//!
//! A `Vec<T>` is (pointer, capacity, length). Pushing past the capacity reallocates: the
//! allocator grows the buffer in place or moves the elements to a bigger one (the pointer
//! changes; how often is up to the allocator). The standard library grows
//! geometrically (amortised O(1) push), so the number of reallocations for n pushes is
//! O(log n). `Vec::with_capacity(n)` allocates once up front.
//!
//! ## Reading list
//!   * https://doc.rust-lang.org/std/vec/struct.Vec.html#capacity-and-reallocation
//!   * source: library/alloc/src/raw_vec.rs (`grow_amortized`)

use crate::util::{banner, check, note, section, try_it};

/// Count how often the buffer moved while pushing `n` elements.
fn reallocations(mut v: Vec<u64>, n: usize) -> usize {
    let mut moves = 0;
    let mut ptr = v.as_ptr();
    for i in 0..n as u64 {
        v.push(i);
        if v.as_ptr() != ptr {
            moves += 1;
            ptr = v.as_ptr();
        }
    }
    moves
}

pub fn run() -> anyhow::Result<()> {
    banner("Lesson 01 — How Vec grows");

    // STEP 1: watch the capacity sequence.
    //   📖 https://doc.rust-lang.org/std/vec/struct.Vec.html#method.capacity
    section("1. Capacity as you push");
    let mut v: Vec<u64> = Vec::new();
    let mut seen = vec![v.capacity()];
    for i in 0..100 {
        v.push(i);
        if *seen.last().unwrap() != v.capacity() {
            seen.push(v.capacity());
        }
    }
    println!("   capacities: {seen:?}");
    check(seen.windows(2).all(|w| w[1] >= 2 * w[0].max(1) || w[0] == 0), "capacity at least doubles on each growth")?;

    // STEP 2: the evidence: reallocations are logarithmic, and zero with with_capacity.
    section("2. Reallocations for 1M pushes");
    let grown = reallocations(Vec::new(), 1_000_000);
    let reserved = reallocations(Vec::with_capacity(1_000_000), 1_000_000);
    println!("   Vec::new(): {grown} moves; Vec::with_capacity(1M): {reserved} moves");
    check(grown <= 21, "at most ~log2(1M) = 20 moves, not 1M")?;
    check(reserved == 0, "a reserved Vec never moves")?;
    note("Fewer than 20? The allocator can sometimes grow a block in place, so the pointer");
    note("stays put. Each real move copies every element so far; geometric growth keeps the");
    note("total copy work O(n).");

    try_it("repeat step 2 with Vec<[u8; 4096]>: does the allocator change the count?");
    try_it("call shrink_to_fit() after the pushes and print capacity and pointer.");
    Ok(())
}
