use std::collections::HashSet;
use std::time::Duration;
use std::time::Instant;

use std_and_tokio::exercises::e02_worker_threads::*;
use std_and_tokio::lessons::l02_worker_threads::heartbeat_while;

#[test]
fn ex1_exactly_n_workers() {
    for workers in [1usize, 3] {
        let rt = ex1_runtime(workers);
        let ids: HashSet<String> = rt.block_on(async {
            let hs: Vec<_> = (0..200)
                .map(|_| tokio::spawn(async { std::thread::sleep(Duration::from_millis(1)); format!("{:?}", std::thread::current().id()) }))
                .collect();
            let mut s = HashSet::new();
            for h in hs {
                s.insert(h.await.unwrap());
            }
            s
        });
        assert!(ids.len() <= workers, "{workers} workers but tasks ran on {} threads", ids.len());
        rt.block_on(async { tokio::time::sleep(Duration::from_millis(1)).await }); // timers must be enabled
    }
}

#[test]
fn ex2_heartbeat_keeps_ticking() {
    let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
    let mut out = 0;
    let ticks = rt.block_on(heartbeat_while(|| async { out = ex2_fetch_without_blocking(21).await }));
    assert_eq!(out, 42);
    assert!(ticks >= 5, "only {ticks} heartbeat ticks in ~100 ms: the worker was blocked");
}

#[test]
fn ex3_prediction_matches_measurement() {
    let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
    let ms = rt.block_on(async {
        let start = Instant::now();
        let hs: Vec<_> = (0..100).map(|_| tokio::spawn(tokio::time::sleep(Duration::from_millis(100)))).collect();
        for h in hs {
            h.await.unwrap();
        }
        start.elapsed().as_millis() as u64
    });
    let predicted = ex3_predict_ms_for_100_sleeps_on_one_thread();
    let nearest = [100u64, 1000, 10_000].into_iter().min_by_key(|c| c.abs_diff(ms)).unwrap();
    assert_eq!(predicted, nearest, "measured {ms} ms");
}
