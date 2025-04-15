use bindings::wasi::io;
use std::future::Future;
use std::mem;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

/// Module containing the generated WIT bindings.
pub mod bindings {
    wit_bindgen::generate!({
        world: "imports",
        path: "io.wit",
    });
}

impl std::fmt::Display for io::streams::Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_debug_string())
    }
}

impl std::error::Error for io::streams::Error {}

type Wrapped = Arc<Mutex<Option<io::poll::Pollable>>>;

static WAKERS: Mutex<Vec<(Wrapped, Waker)>> = Mutex::new(Vec::new());

/// Handle to a Pollable pushed using `push_waker` which may be used to cancel
/// and drop the Pollable.
pub struct CancelToken(Wrapped);

impl CancelToken {
    /// Cancel and drop the Pollable.
    pub fn cancel(self) {
        drop(self.0.lock().unwrap().take())
    }
}

/// Handle to a Pollable pushed using `push_waker` which, when dropped, will
/// cancel and drop the Pollable.
pub struct CancelOnDropToken(Wrapped);

impl From<CancelToken> for CancelOnDropToken {
    fn from(token: CancelToken) -> Self {
        Self(token.0)
    }
}

impl Drop for CancelOnDropToken {
    fn drop(&mut self) {
        drop(self.0.lock().unwrap().take())
    }
}

/// Push a Pollable and Waker to WAKERS.
pub fn push_waker(pollable: io::poll::Pollable, waker: Waker) -> CancelToken {
    let wrapped = Arc::new(Mutex::new(Some(pollable)));
    WAKERS.lock().unwrap().push((wrapped.clone(), waker));
    CancelToken(wrapped)
}

/// Run the specified future to completion blocking until it yields a result.
///
/// Based on an executor using `wasi::io/poll/poll-list`,
pub fn run<T>(future: impl Future<Output = T>) -> T {
    futures::pin_mut!(future);
    struct DummyWaker;

    impl Wake for DummyWaker {
        fn wake(self: Arc<Self>) {}
    }

    let waker = Arc::new(DummyWaker).into();

    loop {
        match future.as_mut().poll(&mut Context::from_waker(&waker)) {
            Poll::Pending => {
                let mut new_wakers = Vec::new();

                let wakers = mem::take(WAKERS.lock().unwrap().deref_mut())
                    .into_iter()
                    .filter_map(|(wrapped, waker)| {
                        let pollable = wrapped.lock().unwrap().take();
                        pollable.map(|pollable| (wrapped, pollable, waker))
                    })
                    .collect::<Vec<_>>();

                let pollables = wakers
                    .iter()
                    .map(|(_, pollable, _)| pollable)
                    .collect::<Vec<_>>();

                let mut ready = vec![false; wakers.len()];

                for index in io::poll::poll(&pollables) {
                    ready[usize::try_from(index).unwrap()] = true;
                }

                for (ready, (wrapped, pollable, waker)) in ready.into_iter().zip(wakers) {
                    if ready {
                        waker.wake()
                    } else {
                        *wrapped.lock().unwrap() = Some(pollable);
                        new_wakers.push((wrapped, waker));
                    }
                }

                *WAKERS.lock().unwrap() = new_wakers;
            }
            Poll::Ready(result) => break result,
        }
    }
}
