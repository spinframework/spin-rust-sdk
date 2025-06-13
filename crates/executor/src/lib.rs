use std::future::Future;
use std::mem;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use wasi::io;

type Wrapped = Arc<Mutex<Option<io::poll::Pollable>>>;

static WAKERS: Mutex<Vec<(Wrapped, Waker)>> = Mutex::new(Vec::new());

/// Handle to a Pollable registered using `push_waker_and_get_token` which may
/// be used to cancel and drop the Pollable.
pub struct CancelToken(Wrapped);

impl CancelToken {
    /// Cancel and drop the Pollable.
    pub fn cancel(self) {
        drop(self.0.lock().unwrap().take())
    }
}

/// Handle to a Pollable registered using `push_waker_and_get_token` which, when
/// dropped, will cancel and drop the Pollable.
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

/// Register a `Pollable` and `Waker` to be polled as part of the [`run`] event
/// loop.
pub fn push_waker(pollable: io::poll::Pollable, waker: Waker) {
    _ = push_waker_and_get_token(pollable, waker);
}

/// Register a `Pollable` and `Waker` to be polled as part of the [`run`] event
/// loop and retrieve a [`CancelToken`] to cancel the registration later, if
/// desired.
pub fn push_waker_and_get_token(pollable: io::poll::Pollable, waker: Waker) -> CancelToken {
    let wrapped = Arc::new(Mutex::new(Some(pollable)));
    WAKERS.lock().unwrap().push((wrapped.clone(), waker));
    CancelToken(wrapped)
}

/// Run the specified future to completion, blocking until it yields a result.
///
/// This will alternate between polling the specified future and polling any
/// `Pollable`s registered using [`push_waker`] or [`push_waker_and_get_token`]
/// using `wasi::io/poll/poll-list`.  It will panic if the future returns
/// `Poll::Pending` without having registered at least one `Pollable`.
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

                assert!(!wakers.is_empty());

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
