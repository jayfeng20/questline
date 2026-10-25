use __CRATE__::exercises::e01_vec_growth::*;

#[test]
fn ex1_starts_at_zero_and_grows_geometrically() {
    let steps = ex1_capacity_steps(100);
    assert_eq!(steps[0], 0, "Vec::new() does not allocate");
    assert!(*steps.last().unwrap() >= 100);
    assert!(steps.windows(2).skip(1).all(|w| w[1] >= 2 * w[0]), "each growth at least doubles: {steps:?}");
}

#[test]
fn ex2_prediction_matches_measurement() {
    let mut v: Vec<u64> = Vec::with_capacity(10);
    let mut ptr = v.as_ptr();
    let mut moves = 0;
    for i in 0..11 {
        v.push(i);
        if v.as_ptr() != ptr {
            moves += 1;
            ptr = v.as_ptr();
        }
    }
    assert_eq!(ex2_predict_moves_after_reserving_10(), moves);
}
