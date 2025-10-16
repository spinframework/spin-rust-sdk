//! Experimental Rust SDK for WASIp3 http.

#![deny(missing_docs)]

#[doc(hidden)]
pub use wasip3_http_ext::wasip3;

use hyperium as http;
use std::any::Any;
use wasip3::{http::types, wit_bindgen};
use wasip3_http_ext::body_writer::BodyWriter;
use wasip3_http_ext::helpers::{
    header_map_to_wasi, method_from_wasi, method_to_wasi, scheme_from_wasi, scheme_to_wasi,
    to_internal_error_code,
};
use wasip3_http_ext::RequestOptionsExtension;
use wasip3_http_ext::{IncomingRequestBody, IncomingResponseBody};

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
/// # Variants
///
/// - [`Error::ErrorCode`]: Wraps a low-level [`wasip3::http::types::ErrorCode`]
///   reported by the WASI HTTP runtime (e.g. connection errors, protocol errors).
///
/// - [`Error::Other`]: Represents an arbitrary dynamic error implementing
///   [`std::error::Error`]. This allows integration with external libraries or
///   application-specific failure types.
///
/// - [`Error::Response`]: Contains a full [`wasip3::http::types::Response`]
///   representing an HTTP-level error (for example, a `4xx` or `5xx` response
///   that should be treated as an error condition).
///
/// # See also
/// - [`wasip3::http::types::ErrorCode`]: Standard WASI HTTP error codes.
/// - [`wasip3::http::types::Response`]: Used when an error represents an HTTP response body.
#[derive(Debug)]
pub enum Error {
    /// A low-level WASI HTTP error code.
    ///
    /// Wraps [`wasip3::http::types::ErrorCode`] to represent
    /// transport-level or protocol-level failures.
    ErrorCode(wasip3::http::types::ErrorCode),
    /// A dynamic application or library error.
    ///
    /// Used for any runtime error that implements [`std::error::Error`],
    /// allowing flexibility for different error sources.
    Other(Box<dyn ::std::error::Error + Send + Sync>),
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
            Error::Other(e) => write!(f, "{e}"),
            Error::Response(e) => match http::StatusCode::from_u16(e.get_status_code()) {
                Ok(status) => write!(f, "{status}"),
                Err(e) => write!(f, "{e}"),
            },
        }
    }
}

impl std::error::Error for Error {}

impl From<http::Error> for Error {
    fn from(_err: http::Error) -> Error {
        todo!("map to specific error codes")
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
    let response = wasip3::http::handler::handle(request).await?;
    Response::from_response(response)
}

/// A type alias for an HTTP request with a customizable body type.
///
/// This is a convenience wrapper around [`http::Request`], parameterized
/// by the body type `T`. By default, it uses [`IncomingRequestBody`],
/// which represents the standard incoming body used by this runtime.
///
/// # Type Parameters
///
/// * `T` — The request body type. Defaults to [`IncomingRequestBody`].
///
/// # See also
/// - [`wasip3_http_ext::IncomingRequestBody`]: The body type for inbound HTTP requests.
/// - [`http::Request`]: The standard HTTP request type from the `http` crate.
pub type Request<T = IncomingRequestBody> = http::Request<T>;

/// A type alias for an HTTP response with a customizable body type.
///
/// This is a convenience wrapper around [`http::Response`], parameterized
/// by the body type `T`. By default, it uses [`IncomingResponseBody`],
/// which represents the standard incoming body type used by this runtime.
///
/// # Type Parameters
///
/// * `T` — The response body type. Defaults to [`IncomingResponseBody`].
///
/// # See also
/// - [`wasip3_http_ext::IncomingResponseBody`]: The body type for inbound HTTP responses.
/// - [`http::Response`]: The standard HTTP response type from the `http` crate.
pub type Response<T = IncomingResponseBody> = http::Response<T>;

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
/// # Type Parameters
///
/// * `T` — The data type of the full body, such as [`bytes::Bytes`] or [`String`].
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

impl<T> IntoRequest for http::Request<T>
where
    T: http_body::Body + Any,
    T::Data: Into<Vec<u8>>,
    T::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    fn into_request(mut self) -> HttpResult<types::Request> {
        if let Some(incoming_body) =
            (&mut self as &mut dyn Any).downcast_mut::<IncomingRequestBody>()
        {
            if let Some(request) = incoming_body.take_unstarted() {
                return Ok(request);
            }
        }

        let (parts, body) = self.into_parts();

        let options = parts
            .extensions
            .get::<RequestOptionsExtension>()
            .cloned()
            .map(|o| o.0);

        let headers = header_map_to_wasi(&parts.headers)?;

        let (body_writer, contents_rx, trailers_rx) = BodyWriter::new();

        let (req, _result) = types::Request::new(headers, Some(contents_rx), trailers_rx, options);

        req.set_method(&method_to_wasi(&parts.method))
            .map_err(|()| types::ErrorCode::HttpRequestMethodInvalid)?;

        let scheme = parts.uri.scheme().map(scheme_to_wasi);
        req.set_scheme(scheme.as_ref())
            .map_err(|()| types::ErrorCode::HttpProtocolError)?;

        req.set_authority(parts.uri.authority().map(|a| a.as_str()))
            .map_err(|()| types::ErrorCode::HttpRequestUriInvalid)?;

        req.set_path_with_query(parts.uri.path_and_query().map(|pq| pq.as_str()))
            .map_err(|()| types::ErrorCode::HttpRequestUriInvalid)?;

        wit_bindgen::spawn(async move {
            let mut body = std::pin::pin!(body);
            _ = body_writer.forward_http_body(&mut body).await;
        });

        Ok(req)
    }
}

impl FromRequest for types::Request {
    fn from_request(req: types::Request) -> HttpResult<Self> {
        Ok(req)
    }
}

impl<T: FromRequest> FromRequest for http::Request<T> {
    fn from_request(req: types::Request) -> HttpResult<Self> {
        let uri = {
            let mut builder = http::Uri::builder();
            if let Some(scheme) = req.get_scheme() {
                builder = builder.scheme(scheme_from_wasi(scheme)?);
            }
            if let Some(authority) = req.get_authority() {
                builder = builder.authority(authority);
            }
            if let Some(path_and_query) = req.get_path_with_query() {
                builder = builder.path_and_query(path_and_query);
            }
            builder
                .build()
                .map_err(|_| types::ErrorCode::HttpRequestUriInvalid)?
        };

        let mut builder = http::Request::builder()
            .method(method_from_wasi(req.get_method())?)
            .uri(uri);

        if let Some(options) = req.get_options().map(RequestOptionsExtension) {
            builder = builder.extension(options);
        }

        for (k, v) in req.get_headers().copy_all() {
            builder = builder.header(k, v);
        }

        let body = T::from_request(req)?;

        builder.body(body).map_err(to_internal_error_code) // TODO: downcast to more specific http error codes
    }
}

impl<T: FromResponse> FromResponse for http::Response<T> {
    fn from_response(resp: types::Response) -> HttpResult<Self> {
        let mut builder = http::Response::builder().status(resp.get_status_code());

        for (k, v) in resp.get_headers().copy_all() {
            builder = builder.header(k, v);
        }

        let body = T::from_response(resp)?;
        builder.body(body).map_err(to_internal_error_code) // TODO: downcast to more specific http error codes
    }
}

impl FromRequest for () {
    fn from_request(_req: types::Request) -> HttpResult<Self> {
        Ok(())
    }
}

impl IntoResponse for types::Response {
    fn into_response(self) -> HttpResult<types::Response> {
        Ok(self)
    }
}

impl<T: http_body::Body> IntoResponse for (http::StatusCode, T) {
    fn into_response(self) -> HttpResult<types::Response> {
        unreachable!()
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
    fn into_response(mut self) -> HttpResult<types::Response> {
        if let Some(incoming_body) =
            (&mut self as &mut dyn Any).downcast_mut::<IncomingResponseBody>()
        {
            if let Some(response) = incoming_body.take_unstarted() {
                return Ok(response);
            }
        }

        let headers = header_map_to_wasi(self.headers())?;

        let (body_writer, body_rx, body_result_rx) = BodyWriter::new();

        let (response, _future_result) =
            types::Response::new(headers, Some(body_rx), body_result_rx);

        _ = response.set_status_code(self.status().as_u16());

        wit_bindgen::spawn(async move {
            let mut body = std::pin::pin!(self.into_body());
            _ = body_writer.forward_http_body(&mut body).await;
        });

        Ok(response)
    }
}

impl FromRequest for IncomingRequestBody {
    fn from_request(req: types::Request) -> HttpResult<Self>
    where
        Self: Sized,
    {
        Self::new(req)
    }
}

impl FromResponse for IncomingResponseBody {
    fn from_response(response: types::Response) -> HttpResult<Self>
    where
        Self: Sized,
    {
        Self::new(response)
    }
}

/// Helpers for consuming an [`IncomingBody`].
///
/// This module provides extension traits and utilities for working with
/// [`IncomingBody`] instances, such as streaming or collecting the entire
/// body into memory.
///
/// These helpers make it easier to transform low-level streaming body types
/// into higher-level forms (e.g., [`Bytes`]) for simplified data handling.
pub mod body {
    use bytes::Bytes;
    use http_body_util::{BodyDataStream, BodyExt};
    use wasip3_http_ext::wasip3::http::types::ErrorCode;
    use wasip3_http_ext::{IncomingBody, IncomingMessage};

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
