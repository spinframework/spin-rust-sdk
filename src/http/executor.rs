use crate::wit::wasi::http0_2_0::outgoing_handler;
use crate::wit::wasi::http0_2_0::types::{
    ErrorCode, FutureIncomingResponse, IncomingBody, IncomingResponse, OutgoingBody,
    OutgoingRequest,
};

use spin_executor::bindings::wasi::io;
use spin_executor::bindings::wasi::io::streams::{InputStream, OutputStream, StreamError};

use futures::{future, sink, stream, Sink, Stream};

pub use spin_executor::{run, CancelOnDropToken};

use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::task::Poll;

const READ_SIZE: u64 = 16 * 1024;

pub(crate) fn outgoing_body(body: OutgoingBody) -> impl Sink<Vec<u8>, Error = StreamError> {
    struct Outgoing {
        stream_and_body: Option<(OutputStream, OutgoingBody)>,
        cancel_token: Option<CancelOnDropToken>,
    }

    impl Drop for Outgoing {
        fn drop(&mut self) {
            drop(self.cancel_token.take());

            if let Some((stream, body)) = self.stream_and_body.take() {
                drop(stream);
                _ = OutgoingBody::finish(body, None);
            }
        }
    }

    let stream = body.write().expect("response body should be writable");
    let outgoing = Rc::new(RefCell::new(Outgoing {
        stream_and_body: Some((stream, body)),
        cancel_token: None,
    }));

    sink::unfold((), {
        move |(), chunk: Vec<u8>| {
            future::poll_fn({
                let mut offset = 0;
                let mut flushing = false;
                let outgoing = outgoing.clone();

                move |context| {
                    let mut outgoing = outgoing.borrow_mut();
                    let (stream, _) = &outgoing.stream_and_body.as_ref().unwrap();
                    loop {
                        match stream.check_write() {
                            Ok(0) => {
                                outgoing.cancel_token = Some(CancelOnDropToken::from(
                                    spin_executor::push_waker_and_get_token(
                                        stream.subscribe(),
                                        context.waker().clone(),
                                    ),
                                ));
                                break Poll::Pending;
                            }
                            Ok(count) => {
                                if offset == chunk.len() {
                                    if flushing {
                                        break Poll::Ready(Ok(()));
                                    } else {
                                        match stream.flush() {
                                            Ok(()) => flushing = true,
                                            Err(StreamError::Closed) => break Poll::Ready(Ok(())),
                                            Err(e) => break Poll::Ready(Err(e)),
                                        }
                                    }
                                } else {
                                    let count =
                                        usize::try_from(count).unwrap().min(chunk.len() - offset);

                                    match stream.write(&chunk[offset..][..count]) {
                                        Ok(()) => {
                                            offset += count;
                                        }
                                        Err(e) => break Poll::Ready(Err(e)),
                                    }
                                }
                            }
                            // If the stream is closed but the entire chunk was
                            // written then we've done all we could so this
                            // chunk is now complete.
                            Err(StreamError::Closed) if offset == chunk.len() => {
                                break Poll::Ready(Ok(()))
                            }
                            Err(e) => break Poll::Ready(Err(e)),
                        }
                    }
                }
            })
        }
    })
}

/// Send the specified request and return the response.
pub(crate) fn outgoing_request_send(
    request: OutgoingRequest,
) -> impl Future<Output = Result<IncomingResponse, ErrorCode>> {
    struct State {
        response: Option<Result<FutureIncomingResponse, ErrorCode>>,
        cancel_token: Option<CancelOnDropToken>,
    }

    impl Drop for State {
        fn drop(&mut self) {
            drop(self.cancel_token.take());
            drop(self.response.take());
        }
    }

    let response = outgoing_handler::handle(request, None);
    let mut state = State {
        response: Some(response),
        cancel_token: None,
    };
    future::poll_fn({
        move |context| match &state.response.as_ref().unwrap() {
            Ok(response) => {
                if let Some(response) = response.get() {
                    Poll::Ready(response.unwrap())
                } else {
                    state.cancel_token = Some(CancelOnDropToken::from(
                        spin_executor::push_waker_and_get_token(
                            response.subscribe(),
                            context.waker().clone(),
                        ),
                    ));
                    Poll::Pending
                }
            }
            Err(error) => Poll::Ready(Err(error.clone())),
        }
    })
}

#[doc(hidden)]
pub fn incoming_body(
    body: IncomingBody,
) -> impl Stream<Item = Result<Vec<u8>, io::streams::Error>> {
    struct Incoming {
        stream_and_body: Option<(InputStream, IncomingBody)>,
        cancel_token: Option<CancelOnDropToken>,
    }

    impl Drop for Incoming {
        fn drop(&mut self) {
            drop(self.cancel_token.take());

            if let Some((stream, body)) = self.stream_and_body.take() {
                drop(stream);
                IncomingBody::finish(body);
            }
        }
    }

    stream::poll_fn({
        let stream = body.stream().expect("response body should be readable");
        let mut incoming = Incoming {
            stream_and_body: Some((stream, body)),
            cancel_token: None,
        };

        move |context| {
            if let Some((stream, _)) = &incoming.stream_and_body {
                match stream.read(READ_SIZE) {
                    Ok(buffer) => {
                        if buffer.is_empty() {
                            incoming.cancel_token = Some(CancelOnDropToken::from(
                                spin_executor::push_waker_and_get_token(
                                    stream.subscribe(),
                                    context.waker().clone(),
                                ),
                            ));
                            Poll::Pending
                        } else {
                            Poll::Ready(Some(Ok(buffer)))
                        }
                    }
                    Err(StreamError::Closed) => Poll::Ready(None),
                    Err(StreamError::LastOperationFailed(error)) => Poll::Ready(Some(Err(error))),
                }
            } else {
                Poll::Ready(None)
            }
        }
    })
}
