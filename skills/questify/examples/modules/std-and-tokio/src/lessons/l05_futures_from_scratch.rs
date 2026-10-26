//! # Lesson 05 — Futures from scratch: Future, Poll, Context, Waker
//!
//! ## Lecture
//!
//! An `async fn` compiles to a state machine implementing `Future`:
//!
//! ```text
//! trait Future { type Output; fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>; }
//! ```
//!
//! | piece        | job                                                                               |
//! |--------------|-----------------------------------------------------------------------------------|
//! | `poll`       | make progress; return `Ready(v)` or `Pending`                                     |
//! | `Pending`    | a promise: "I stored `cx.waker()` and someone will call `wake()` when I can progress" |
//! | `Waker`      | the handle the future (or a timer thread, or the OS reactor) uses to say "poll me again" |
//! | executor     | loops: poll; if Pending, sleep until woken; repeat. `block_on` is the smallest one |
//! | `Pin`        | the state machine may hold references into itself, so it must not move once polled |
//!
//! Futures are **lazy**: creating one does nothing until it is polled. A future that returns
//! `Pending` without arranging a wake-up hangs forever; one that wakes itself immediately
//! busy-spins. Both are bugs this lesson makes visible by *counting polls*.
//!
//! ## Reading list
//!   * https://doc.rust-lang.org/std/future/trait.Future.html and std::task::{Waker, Wake, Context}
//!   * https://rust-lang.github.io/async-book/02_execution/01_chapter.html (build an executor)
//!   * https://without.boats/blog/wakers-i/ and https://fasterthanli.me/articles/pin-and-suffering

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Wake;
use std::task::Waker;

use crate::util::{banner, check, note, section, try_it};

/// Counts how many times it is woken; never parks anything.
struct CountingWaker(AtomicUsize);

impl Wake for CountingWaker {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// Ready after being polled `n` times; wakes itself each time it says Pending.
struct YieldN(u32);

impl Future for YieldN {
    type Output = &'static str;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0 == 0 {
            return Poll::Ready("done");
        }
        self.0 -= 1;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

pub fn run() -> anyhow::Result<()> {
    banner("Lesson 05 — Futures from scratch");

    // STEP 1: poll a future by hand.
    //   📖 https://doc.rust-lang.org/std/task/trait.Wake.html
    section("1. Polling by hand");
    let counter = Arc::new(CountingWaker(AtomicUsize::new(0)));
    let waker = Waker::from(counter.clone());
    let mut cx = Context::from_waker(&waker);
    let mut fut = std::pin::pin!(YieldN(3));
    let mut polls = 0;
    let out = loop {
        polls += 1;
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            break v;
        }
    };
    println!("   result {out:?} after {polls} polls; woken {} times", counter.0.load(Ordering::SeqCst));
    check(polls == 4 && counter.0.load(Ordering::SeqCst) == 3, "3 Pending (each with a wake) then Ready")?;

    // STEP 2: futures are lazy.
    section("2. Creating an async block runs nothing");
    let ran = Arc::new(AtomicUsize::new(0));
    let r = ran.clone();
    let lazy = async move {
        r.fetch_add(1, Ordering::SeqCst);
        7
    };
    check(ran.load(Ordering::SeqCst) == 0, "the async block has not started")?;
    let mut lazy = std::pin::pin!(lazy);
    check(matches!(lazy.as_mut().poll(&mut cx), Poll::Ready(7)) && ran.load(Ordering::SeqCst) == 1, "the first poll runs it")?;

    // STEP 3: an async fn is a state machine; its size is what it must remember across awaits.
    section("3. The state machine's size");
    async fn small() -> u8 {
        YieldN(1).await;
        1
    }
    async fn holds_buffer() -> usize {
        let buf = [0u8; 1024];
        YieldN(1).await; // `buf` is alive across this await, so it lives in the future
        buf.len()
    }
    let (a, b) = (std::mem::size_of_val(&small()), std::mem::size_of_val(&holds_buffer()));
    println!("   size_of small() = {a} B; holds_buffer() = {b} B");
    check(b >= 1024 && a < 64, "locals alive across an .await are stored inside the future")?;
    note("This is why huge futures get Box::pin'ed, and why holding a lock across .await is visible");
    note("to the compiler (the guard is part of the state).");

    try_it("make YieldN return Pending WITHOUT calling wake: what would a real executor do?");
    try_it("print size_of_val for an async fn that awaits holds_buffer() twice in sequence.");
    Ok(())
}
