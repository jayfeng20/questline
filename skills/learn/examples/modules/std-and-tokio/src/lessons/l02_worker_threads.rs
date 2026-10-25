//! # Lesson 02 — Tokio worker threads: what they buy you, and what blocks them
//!
//! ## Lecture
//!
//! A Tokio runtime runs *tasks* on *worker threads*. `current_thread` has one (the caller's);
//! `multi_thread` has `worker_threads(n)` (default: one per core) plus a separate, growable
//! pool for `spawn_blocking`. Three kinds of work behave very differently:
//!
//! | work                          | needs            | worker count matters? |
//! |-------------------------------|------------------|-----------------------|
//! | IO-bound (`.await` on timers, sockets) | the reactor; tasks park while waiting | no: 1 thread interleaves thousands |
//! | CPU-bound (a hot loop, no `.await`)   | a whole thread per running task       | yes: throughput ~ min(tasks, workers) |
//! | blocking (`std::thread::sleep`, sync IO) | a whole thread, *wasted* while waiting | yes, and it starves every other task on that worker |
//!
//! ## Reading list
//!   * https://docs.rs/tokio/1/tokio/runtime/struct.Builder.html (`worker_threads`, `max_blocking_threads`)
//!   * https://docs.rs/tokio/1/tokio/task/fn.spawn_blocking.html
//!   * https://tokio.rs/blog/2020-04-preemption  and  https://ryhl.io/blog/async-what-is-blocking/

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use tokio::runtime::Builder;
use tokio::runtime::Runtime;

use crate::util::{banner, check, note, section, try_it};

fn runtime(workers: usize) -> Runtime {
    Builder::new_multi_thread().worker_threads(workers).enable_time().build().expect("runtime")
}

/// Busy-spin for `d` without yielding: a stand-in for real CPU work.
fn spin(d: Duration) {
    let start = Instant::now();
    while start.elapsed() < d {
        std::hint::spin_loop();
    }
}

/// Run `tasks` CPU-bound tasks of `each` on `rt`; return the wall time.
fn cpu_bound(rt: &Runtime, tasks: usize, each: Duration) -> Duration {
    rt.block_on(async {
        let start = Instant::now();
        let handles: Vec<_> = (0..tasks).map(|_| tokio::spawn(async move { spin(each) })).collect();
        for h in handles {
            h.await.unwrap();
        }
        start.elapsed()
    })
}

pub fn run() -> anyhow::Result<()> {
    banner("Lesson 02 — Tokio worker threads");

    // STEP 1: how many OS threads actually ran our tasks?
    //   📖 https://docs.rs/tokio/1/tokio/runtime/struct.Builder.html#method.worker_threads
    section("1. Tasks land on worker threads");
    for workers in [1usize, 4] {
        let rt = runtime(workers);
        let ids: HashSet<String> = rt.block_on(async {
            let hs: Vec<_> = (0..64).map(|_| tokio::spawn(async { spin(Duration::from_millis(2)); format!("{:?}", std::thread::current().id()) })).collect();
            let mut ids = HashSet::new();
            for h in hs {
                ids.insert(h.await.unwrap());
            }
            ids
        });
        println!("   worker_threads({workers}): 64 tasks ran on {} distinct threads", ids.len());
        check(ids.len() <= workers, &format!("never more threads than the {workers} workers"))?;
    }

    // STEP 2: CPU-bound work scales with workers (up to your core count).
    section("2. CPU-bound: 4 tasks x 100 ms");
    let t1 = cpu_bound(&runtime(1), 4, Duration::from_millis(100));
    let t4 = cpu_bound(&runtime(4), 4, Duration::from_millis(100));
    println!("   1 worker: {t1:?}   4 workers: {t4:?}");
    check(t1 >= Duration::from_millis(390), "1 worker runs the four 100 ms tasks one after another")?;
    if std::thread::available_parallelism()?.get() >= 4 {
        check(t4 * 2 < t1, "4 workers finish at least 2x sooner on a 4+ core machine")?;
    }

    // STEP 3: IO-bound work does not need workers: one thread interleaves 1000 sleeps.
    section("3. IO-bound: 1000 tasks each awaiting a 50 ms timer, on ONE thread");
    let rt = Builder::new_current_thread().enable_time().build()?;
    let elapsed = rt.block_on(async {
        let start = Instant::now();
        let hs: Vec<_> = (0..1000).map(|_| tokio::spawn(tokio::time::sleep(Duration::from_millis(50)))).collect();
        for h in hs {
            h.await.unwrap();
        }
        start.elapsed()
    });
    println!("   1000 x 50 ms of waiting took {elapsed:?}");
    check(elapsed < Duration::from_millis(500), "waits overlap: ~50 ms total, not 50 s")?;

    // STEP 4: blocking a worker starves everything scheduled on it.
    section("4. Blocking inside async vs spawn_blocking");
    let rt = Builder::new_current_thread().enable_time().build()?;
    let ticks_blocked = rt.block_on(heartbeat_while(|| async { std::thread::sleep(Duration::from_millis(200)) }));
    let ticks_offloaded = rt.block_on(heartbeat_while(|| async {
        tokio::task::spawn_blocking(|| std::thread::sleep(Duration::from_millis(200))).await.unwrap()
    }));
    println!("   heartbeat ticks during 200 ms of std::thread::sleep: {ticks_blocked}; with spawn_blocking: {ticks_offloaded}");
    check(ticks_blocked <= 1 && ticks_offloaded >= 10, "a blocking call freezes its worker; spawn_blocking does not")?;
    note("Rule: never block a worker. Move blocking calls to spawn_blocking (or a dedicated thread).");

    try_it("set worker_threads to 2 * your cores in step 2: does it get faster? why not?");
    try_it("in step 4, use a multi_thread runtime with 2 workers: how many ticks now, and why?");
    Ok(())
}

/// Run `work` while a 10 ms heartbeat task counts ticks; return how many ticks happened.
pub async fn heartbeat_while<F, Fut>(work: F) -> usize
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let ticks = Arc::new(AtomicUsize::new(0));
    let counter = ticks.clone();
    let hb = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(10));
        loop {
            interval.tick().await;
            counter.fetch_add(1, Ordering::Relaxed);
        }
    });
    tokio::task::yield_now().await; // let the heartbeat take its first tick
    let start = ticks.load(Ordering::Relaxed);
    work().await;
    let during = ticks.load(Ordering::Relaxed) - start;
    hb.abort();
    during
}
