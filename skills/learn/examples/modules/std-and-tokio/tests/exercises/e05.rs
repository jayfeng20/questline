use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use std::time::Instant;

use std_and_tokio::exercises::e05_futures_from_scratch::*;

/// Wraps a future and counts how often it is polled.
struct Counted<F> {
    inner: Pin<Box<F>>,
    polls: Arc<AtomicUsize>,
}

impl<F: Future> Future for Counted<F> {
    type Output = F::Output;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        self.inner.as_mut().poll(cx)
    }
}

fn counted<F: Future>(f: F) -> (Counted<F>, Arc<AtomicUsize>) {
    let polls = Arc::new(AtomicUsize::new(0));
    (Counted { inner: Box::pin(f), polls: polls.clone() }, polls)
}

#[test]
fn ex1_ready_futures_and_async_blocks() {
    assert_eq!(ex1_block_on(async { 1 + 1 }), 2);
    assert_eq!(ex1_block_on(std::future::ready("x")), "x");
}

#[test]
fn ex1_and_ex2_sleep_without_spinning() {
    let (fut, polls) = counted(Delay::new(Duration::from_millis(50)));
    let start = Instant::now();
    ex1_block_on(fut);
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(50), "returned before the delay: {elapsed:?}");
    assert!(polls.load(Ordering::SeqCst) <= 3, "polled {} times in 50 ms: the executor or the Delay busy-spins", polls.load(Ordering::SeqCst));
}

#[test]
fn ex2_delay_is_lazy() {
    let d = Delay::new(Duration::from_millis(100));
    std::thread::sleep(Duration::from_millis(100));
    assert!(d.state.is_none(), "creating a Delay must not start its timer");
    let start = Instant::now();
    ex1_block_on(d);
    assert!(start.elapsed() >= Duration::from_millis(90), "the timer must start at the first poll, not at creation");
}

#[test]
fn ex3_join_overlaps_waits() {
    let start = Instant::now();
    let (a, b) = ex1_block_on(ex3_join(
        async {
            Delay::new(Duration::from_millis(100)).await;
            "a"
        },
        async {
            Delay::new(Duration::from_millis(100)).await;
            "b"
        },
    ));
    let elapsed = start.elapsed();
    assert_eq!((a, b), ("a", "b"));
    assert!(elapsed < Duration::from_millis(180), "two 100 ms delays joined took {elapsed:?}: they ran one after the other");
}

#[test]
fn ex3_does_not_repoll_finished_futures() {
    let (quick, quick_polls) = counted(std::future::ready(1));
    let out = ex1_block_on(ex3_join(quick, Delay::new(Duration::from_millis(30))));
    assert_eq!(out.0, 1);
    assert_eq!(quick_polls.load(Ordering::SeqCst), 1, "a Ready future must never be polled again");
}
