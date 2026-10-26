//! Reference solution for `src/exercises/e01_vec_growth.rs`.

pub fn ex1_capacity_steps(n: usize) -> Vec<usize> {
    let mut v: Vec<u64> = Vec::new();
    let mut steps = vec![v.capacity()];
    for i in 0..n as u64 {
        v.push(i);
        if steps.last() != Some(&v.capacity()) {
            steps.push(v.capacity());
        }
    }
    steps
}

pub fn ex2_predict_reallocations_after_reserving_10() -> usize {
    // The 11th push exceeds the reservation once.
    1
}
