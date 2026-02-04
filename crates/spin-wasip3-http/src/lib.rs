//! Experimental Rust SDK for WASIp3 HTTP.

#![deny(missing_docs)]

#[doc(hidden)]
pub use wasip3;

pub use wasip3::{
    http_compat::{IncomingMessage, Request, Response},
    wit_bindgen::{self, spawn},
    wit_future, wit_stream,
};

use hyperium as http;
use std::any::Any;
use wasip3::{
    http::types,
    http_compat::{
        http_from_wasi_request, http_from_wasi_response, http_into_wasi_request,
        http_into_wasi_response,
    },
};

/// A alias for [`std::result::Result`] that uses [`Error`] as the default error type.
///
/// This allows functions throughout the crate to return `Result<T>`
/// instead of writing out `Result<T, Error>` explicitly.
pub type Result<T, E = Error> = ::std::result::Result<T, E>;

type HttpResult<T> = Result<T, types::ErrorCode>;

/// The error type used for HTTP operations within the WASI environment.
///
/// This enum provides a unified representation of all errors that can occur
/// during HTTP request or response handling, whether they originate from
/// WASI-level error codes, dynamic runtime failures, or full HTTP responses
/// returned as error results.
///
/// # See also
/// - [`http::Error`]: Error type originating from the [`http`] crate.
/// - [`wasip3::http::types::ErrorCode`]: Standard WASI HTTP error codes.
/// - [`wasip3::http::types::Response`]: Used when an error represents an HTTP response body.
#[derive(Debug)]
pub enum Error {
    /// A low-level WASI HTTP error code.
    ///
    /// Wraps [`wasip3::http::types::ErrorCode`] to represent
    /// transport-level or protocol-level failures.
    ErrorCode(wasip3::http::types::ErrorCode),
    /// An error originating from the [`http`] crate.
    ///
    /// Covers errors encountered during the construction,
    /// parsing, or validation of [`http`] types (e.g. invalid headers,
    /// malformed URIs, or protocol violations).
    HttpError(http::Error),
    /// A dynamic application or library error.
    ///
    /// Used for any runtime error that implements [`std::error::Error`],
    /// allowing flexibility for different error sources.
    Other(Box<dyn std::error::Error + Send + Sync>),
    /// An HTTP response treated as an error.
    ///
    /// Contains a full [`wasip3::http::types::Response`], such as
    /// a `404 Not Found` or `500 Internal Server Error`, when
    /// the response itself represents an application-level failure.
    Response(wasip3::http::types::Response),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ErrorCode(e) => write!(f, "{e}"),
            Error::HttpError(e) => write!(f, "{e}"),
            Error::Other(e) => write!(f, "{e}"),
            Error::Response(resp) => match http::StatusCode::from_u16(resp.get_status_code()) {
                Ok(status) => write!(f, "{status}"),
                Err(_) => write!(f, "invalid status code {}", resp.get_status_code()),
            },
        }
    }
}

impl std::error::Error for Error {}

impl From<http::Error> for Error {
    fn from(err: http::Error) -> Error {
        Error::HttpError(err)
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Error {
        match err.downcast::<types::ErrorCode>() {
            Ok(code) => Error::ErrorCode(code),
            Err(other) => match other.downcast::<Error>() {
                Ok(err) => err,
                Err(other) => Error::Other(other.into_boxed_dyn_error()),
            },
        }
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(v: std::convert::Infallible) -> Self {
        match v {}
    }
}

impl From<types::ErrorCode> for Error {
    fn from(code: types::ErrorCode) -> Self {
        Error::ErrorCode(code)
    }
}

impl From<types::Response> for Error {
    fn from(resp: types::Response) -> Self {
        Error::Response(resp)
    }
}

impl<Ok: IntoResponse, Err: Into<Error>> IntoResponse for Result<Ok, Err> {
    fn into_response(self) -> HttpResult<types::Response> {
        match self {
            Ok(ok) => ok.into_response(),
            Err(err) => match err.into() {
                Error::ErrorCode(code) => Err(code),
                Error::Response(resp) => Ok(resp),
                Error::HttpError(err) => match err {
                    err if err.is::<http::method::InvalidMethod>() => {
                        Err(types::ErrorCode::HttpRequestMethodInvalid)
                    }
                    err if err.is::<http::uri::InvalidUri>() => {
                        Err(types::ErrorCode::HttpRequestUriInvalid)
                    }
                    err => Err(types::ErrorCode::InternalError(Some(err.to_string()))),
                },
                Error::Other(other) => {
                    Err(types::ErrorCode::InternalError(Some(other.to_string())))
                }
            },
        }
    }
}

/// Sends an HTTP request and returns the corresponding [`wasip3::http::types::Response`].
///
/// This function converts the provided value into a [`wasip3::http::types::Request`] using the
/// [`IntoRequest`] trait, dispatches it to the WASI HTTP handler, and awaits
/// the resulting response. It provides a convenient high-level interface for
/// issuing HTTP requests within a WASI environment.
pub async fn send(request: impl IntoRequest) -> HttpResult<Response> {
    let request = request.into_request()?;
    let response = wasip3::http::client::send(request).await?;
    Response::from_response(response)
}

/// A body type representing an empty payload.
///
/// This is a convenience alias for [`http_body_util::Empty<bytes::Bytes>`],
/// used when constructing HTTP requests or responses with no body.
///
/// # Examples
///
/// ```ignore
/// use spin_wasip3_http::EmptyBody;
///
/// let empty = EmptyBody::new();
/// let response = http::Response::builder()
///     .status(204)
///     .body(empty)
///     .unwrap();
/// ```
pub type EmptyBody = http_body_util::Empty<bytes::Bytes>;

/// A body type representing a complete, in-memory payload.
///
/// This is a convenience alias for [`http_body_util::Full<T>`], used when the
/// entire body is already available as a single value of type `T`.
///
/// It is typically used for sending small or pre-buffered request or response
/// bodies without the need for streaming.
///
/// # Examples
///
/// ```ignore
/// use spin_wasip3_http::FullBody;
/// use bytes::Bytes;
///
/// let body = FullBody::new(Bytes::from("hello"));
/// let request = http::Request::builder()
///     .method("POST")
///     .uri("https://example.com")
///     .body(body)
///     .unwrap();
/// ```
pub type FullBody<T> = http_body_util::Full<T>;

/// A trait for constructing a value from a [`wasip3::http::types::Request`].
///
/// This is the inverse of [`IntoRequest`], allowing higher-level request
/// types to be built from standardized WASI HTTP requests—for example,
/// to parse structured payloads, extract query parameters, or perform
/// request validation.
///
/// # See also
/// - [`IntoRequest`]: Converts a type into a [`wasip3::http::types::Request`].
pub trait FromRequest {
    /// Attempts to construct `Self` from a [`wasip3::http::types::Request`].
    fn from_request(req: wasip3::http::types::Request) -> HttpResult<Self>
    where
        Self: Sized;
}

impl FromRequest for types::Request {
    fn from_request(req: types::Request) -> HttpResult<Self> {
        Ok(req)
    }
}

impl FromRequest for Request {
    fn from_request(req: types::Request) -> HttpResult<Self> {
        http_from_wasi_request(req)
    }
}

/// A trait for any type that can be converted into a [`wasip3::http::types::Request`].
///
/// This trait provides a unified interface for adapting user-defined request
/// types into the lower-level [`wasip3::http::types::Request`] format used by
/// the WASI HTTP subsystem.  
///
/// Implementing `IntoRequest` allows custom builders or wrapper types to
/// interoperate seamlessly with APIs that expect standardized WASI HTTP
/// request objects.
///
/// # See also
/// - [`FromRequest`]: The inverse conversion trait.
pub trait IntoRequest {
    /// Converts `self` into a [`wasip3::http::types::Request`].
    fn into_request(self) -> HttpResult<wasip3::http::types::Request>;
}

impl<T> IntoRequest for http::Request<T>
where
    T: http_body::Body + Any,
    T::Data: Into<Vec<u8>>,
    T::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    fn into_request(self) -> HttpResult<types::Request> {
        http_into_wasi_request(self)
    }
}

/// A trait for constructing a value from a [`wasip3::http::types::Response`].
///
/// This is the inverse of [`IntoResponse`], allowing higher-level response
/// types to be derived from standardized WASI HTTP responses—for example,
/// to deserialize JSON payloads or map responses to domain-specific types.
///
/// # See also
/// - [`IntoResponse`]: Converts a type into a [`wasip3::http::types::Response`].
pub trait FromResponse {
    /// Attempts to construct `Self` from a [`wasip3::http::types::Response`].
    fn from_response(response: wasip3::http::types::Response) -> HttpResult<Self>
    where
        Self: Sized;
}

impl FromResponse for Response {
    fn from_response(resp: types::Response) -> HttpResult<Self> {
        http_from_wasi_response(resp)
    }
}

/// A trait for any type that can be converted into a [`wasip3::http::types::Response`].
///
/// This trait provides a unified interface for adapting user-defined response
/// types into the lower-level [`wasip3::http::types::Response`] format used by
/// the WASI HTTP subsystem.  
///
/// Implementing `IntoResponse` enables ergonomic conversion from domain-level
/// response types or builders into standardized WASI HTTP responses.
///
/// # See also
/// - [`FromResponse`]: The inverse conversion trait.
pub trait IntoResponse {
    /// Converts `self` into a [`wasip3::http::types::Response`].
    fn into_response(self) -> HttpResult<wasip3::http::types::Response>;
}

impl IntoResponse for types::Response {
    fn into_response(self) -> HttpResult<types::Response> {
        Ok(self)
    }
}

impl<T> IntoResponse for (http::StatusCode, T)
where
    T: http_body::Body + Any,
    T::Data: Into<Vec<u8>>,
    T::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    fn into_response(self) -> HttpResult<types::Response> {
        http_into_wasi_response(
            http::Response::builder()
                .status(self.0)
                .body(self.1)
                .unwrap(),
        )
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> HttpResult<types::Response> {
        http::Response::new(http_body_util::Full::new(self.as_bytes())).into_response()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> HttpResult<types::Response> {
        http::Response::new(self).into_response()
    }
}

impl<T> IntoResponse for http::Response<T>
where
    T: http_body::Body + Any,
    T::Data: Into<Vec<u8>>,
    T::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    fn into_response(self) -> HttpResult<types::Response> {
        http_into_wasi_response(self)
    }
}

/// Helpers for consuming an [`wasip3::http_compat::IncomingBody`].
///
/// This module provides extension traits and utilities for working with
/// [`wasip3::http_compat::IncomingBody`] instances, such as streaming or collecting the entire
/// body into memory.
///
/// These helpers make it easier to transform low-level streaming body types
/// into higher-level forms (e.g., [`bytes::Bytes`]) for simplified data handling.
pub mod body {
    use bytes::Bytes;
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
        /// This method enables iteration over the body’s data chunks as they
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
}
