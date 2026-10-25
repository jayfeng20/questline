//! Reference solution for `src/exercises/e05_futures_from_scratch.rs`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Wake;
use std::task::Waker;
use std::thread::Thread;
use std::time::Duration;

struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

pub fn ex1_block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = std::pin::pin!(fut);
    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
        // A wake that happened before we park leaves a token: park returns immediately.
        std::thread::park();
    }
}

pub struct Delay {
    pub after: Duration,
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
        let after = self.after;
        let state = self.state.get_or_insert_with(|| {
            let state = Arc::new(Mutex::new((false, None::<Waker>)));
            let timer = state.clone();
            std::thread::spawn(move || {
                std::thread::sleep(after);
                let mut s = timer.lock().unwrap();
                s.0 = true;
                if let Some(w) = s.1.take() {
                    w.wake();
                }
            });
            state
        });
        let mut s = state.lock().unwrap();
        if s.0 {
            return Poll::Ready(());
        }
        // Always store the latest waker: the task may have moved to another executor.
        s.1 = Some(cx.waker().clone());
        Poll::Pending
    }
}

pub fn ex3_join<A: Future, B: Future>(a: A, b: B) -> impl Future<Output = (A::Output, B::Output)> {
    let (mut a, mut b) = (Box::pin(a), Box::pin(b));
    let (mut out_a, mut out_b) = (None, None);
    std::future::poll_fn(move |cx| {
        if out_a.is_none() {
            if let Poll::Ready(v) = a.as_mut().poll(cx) {
                out_a = Some(v);
            }
        }
        if out_b.is_none() {
            if let Poll::Ready(v) = b.as_mut().poll(cx) {
                out_b = Some(v);
            }
        }
        if out_a.is_some() && out_b.is_some() {
            Poll::Ready((out_a.take().unwrap(), out_b.take().unwrap()))
        } else {
            Poll::Pending
        }
    })
}
