use std_and_tokio::exercises::e01_vec_growth::*;

#[test]
fn ex1_starts_at_zero_and_grows_geometrically() {
    let steps = ex1_capacity_steps(100);
    assert_eq!(steps[0], 0, "Vec::new() does not allocate");
    assert!(*steps.last().unwrap() >= 100);
    assert!(steps.windows(2).skip(1).all(|w| w[1] >= 2 * w[0]), "each growth at least doubles: {steps:?}");
}

#[test]
fn ex2_prediction_matches_measurement() {
    // Capacity changes, not pointer moves: whether a reallocation moves the buffer is up to
    // the allocator.
    let mut v: Vec<u64> = Vec::with_capacity(10);
    let mut capacity = v.capacity();
    let mut reallocations = 0;
    for i in 0..11 {
        v.push(i);
        if v.capacity() != capacity {
            reallocations += 1;
            capacity = v.capacity();
        }
    }
    assert_eq!(ex2_predict_reallocations_after_reserving_10(), reallocations);
}
