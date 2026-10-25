//! # Exercises — lesson 05: build futures and an executor (std only)
//!
//! Check: `cargo run -- check 5`
//! Stuck: `cargo run -- 5`, the async book chapter on executors (lesson reading list), then
//! `solutions/e05_futures_from_scratch.rs`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::time::Duration;

/// Exercise 1: run a future to completion on the current thread. While the future is Pending,
/// the thread must SLEEP (park) until woken, not spin: the tests count polls.
///
/// 💡 A `Waker` from an `Arc<impl Wake>` whose `wake` unparks the blocked thread
///    (`std::thread::current()` / `Thread::unpark` / `std::thread::park`).
pub fn ex1_block_on<F: Future>(fut: F) -> F::Output {
    todo!("ex1: pin the future, build a parking waker, loop poll/park")
}

/// Exercise 2: a future that completes `after` its FIRST poll (lazy: creating it starts no
/// timer). On first poll, start a thread that sleeps `after` and then wakes the latest waker.
///
/// 📖 https://rust-lang.github.io/async-book/02_execution/03_wakeups.html
pub struct Delay {
    pub after: Duration,
    /// `None` until first polled. Shared with the timer thread: (done, latest waker).
    pub state: Option<Arc<Mutex<(bool, Option<Waker>)>>>,
}

impl Delay {
    pub fn new(after: Duration) -> Self {
        Self { after, state: None }
    }
}

impl Future for Delay {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        todo!("ex2: start the timer thread on first poll; store cx.waker(); Ready when done")
    }
}

/// Exercise 3: poll two futures concurrently; Ready with both outputs once both are done.
/// Both must make progress while either is pending (the test runs two Delays in parallel).
///
/// 💡 Keep each side's output in an `Option` once it is Ready; never poll a finished future.
pub fn ex3_join<A: Future, B: Future>(a: A, b: B) -> impl Future<Output = (A::Output, B::Output)> {
    todo!("ex3: a struct or std::future::poll_fn holding both pinned futures");
    #[allow(unreachable_code)]
    async { unreachable!() }
}
