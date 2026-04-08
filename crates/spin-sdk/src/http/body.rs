//! Utilities for working with HTTP message bodies.
//!
//! This module provides extension traits and utilities for working with
//! [`wasip3::http_compat::IncomingBody`] instances, such as streaming or collecting the entire
//! body into memory.
//!
//! These helpers make it easier to transform low-level streaming body types
//! into higher-level forms (e.g., [`bytes::Bytes`]) for simplified data handling.

use bytes::Bytes;
use futures::{
    channel::mpsc::{channel, Sender},
    StreamExt,
};
use http_body_util::{BodyDataStream, BodyExt};
use wasip3::{
    http::types::ErrorCode,
    http_compat::{IncomingBody, IncomingMessage},
};

/// Extension trait providing convenient methods for consuming an [`IncomingBody`].
///
/// This trait defines common patterns for handling HTTP body data in
/// asynchronous contexts. It allows converting the body into a stream
/// or fully collecting it into memory as a [`Bytes`] buffer.
#[allow(async_fn_in_trait)]
pub trait IncomingBodyExt {
    /// Convert this [`IncomingBody`] into a [`BodyDataStream`].
    ///
    /// This method enables iteration over the body's data chunks as they
    /// arrive, without collecting them all into memory at once. It is
    /// suitable for processing large or streaming payloads efficiently.
    fn stream(self) -> BodyDataStream<Self>
    where
        Self: Sized;

    /// Consume this [`IncomingBody`] and collect it into a single [`Bytes`] buffer.
    ///
    /// This method reads the entire body asynchronously and returns the
    /// concatenated contents. It is best suited for small or bounded-size
    /// payloads where holding all data in memory is acceptable.
    async fn bytes(self) -> Result<Bytes, ErrorCode>;
}

impl<T: IncomingMessage> IncomingBodyExt for IncomingBody<T> {
    /// Convert this [`IncomingBody`] into a [`BodyDataStream`].
    fn stream(self) -> BodyDataStream<Self>
    where
        Self: Sized,
    {
        BodyDataStream::new(self)
    }

    /// Collect the [`IncomingBody`] into a single [`Bytes`] buffer.
    async fn bytes(self) -> Result<Bytes, ErrorCode> {
        self.collect().await.map(|c| c.to_bytes())
    }
}

/// Create a streaming body, with a `Sender` for writing to the body.
/// This supports strings, `Bytes`, `Vec<u8>`, and any `IntoIterator<Item = u8>`.
/// For types which are not `Into<Bytes>`, use [`stream_any`].
///
/// # Examples
///
/// ```no_run
/// # use spin_sdk::http::Response;
/// # use spin_sdk::http::body::stream;
/// use futures::SinkExt;
///
/// let (mut tx, body) = stream::<String>();
///
/// spin_sdk::wasip3::spawn(async move {
///     for i in 0..10000 {
///         if tx.send(format!("{i}\n")).await.is_err() {
///             break;
///         }
///     }
/// });
///
/// let response = Response::new(body);
/// ```
pub fn stream<T: Into<Bytes>>() -> (
    Sender<T>,
    impl http_body::Body<Data = Bytes, Error = anyhow::Error>,
) {
    stream_any::<T>(|t| t.into())
}

/// Create a streaming body, with a `Sender` for writing to the body.
/// This supports any type, but requires you to provider a converter from your
/// type to `Bytes`.  (For types that implement `Into<Bytes>`, use [`stream`]).
pub fn stream_any<T>(
    f: impl Fn(T) -> Bytes,
) -> (
    Sender<T>,
    impl http_body::Body<Data = Bytes, Error = anyhow::Error>,
) {
    let (tx, rx) = channel::<T>(1024);
    let stm = rx.map(move |value| Ok(http_body::Frame::data(f(value))));
    (tx, http_body_util::StreamBody::new(stm))
}
