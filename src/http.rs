//! Sending and receiving HTTP requests.
//!
//! This module includes:
//!
//! * Common HTTP types used in sending and receiving.
//! * The [`send`] function for making HTTP requests.
//! * The [`Router`] type for routing received HTTP requests.
//! * The [`conversions`] and [`responses`] modules to make common cases easier.
//!
//! For writing a component which handles HTTP requests, see the
//! [`http_component`](super::http_component) macro.

/// Traits for converting between the various types
pub mod conversions;

use std::collections::HashMap;

#[doc(inline)]
pub use conversions::IntoResponse;
#[doc(inline)]
pub use types::{
    ErrorCode, Headers, IncomingResponse, Method, OutgoingBody, OutgoingRequest, Scheme,
    StatusCode, Trailers,
};

use self::conversions::{TryFromIncomingResponse, TryIntoOutgoingRequest};
use super::wit::wasi::http0_2_0::types;
use futures::SinkExt;
use spin_executor::bindings::wasi::io::streams::{self, StreamError};

/// Represents an incoming HTTP request.
///
/// If you don't need streaming access to the request body, you may find it
/// easier to work with [Request] instead. To make outgoing requests, use
/// [Request] (non-streaming) or [OutgoingRequest].
///
/// # Examples
///
/// Access the request body as a Rust stream:
///
/// ```no_run
/// # use spin_sdk::http::{IncomingRequest, ResponseOutparam};
/// async fn handle_request(req: IncomingRequest, response_outparam: ResponseOutparam) {
///     use futures::stream::StreamExt;
///
///     let mut stream = req.into_body_stream();
///     loop {
///         let chunk = stream.next().await;
///         match chunk {
///             None => {
///                 println!("end of request body");
///                 break;
///             }
///             Some(Ok(chunk)) => {
///                 // process the data from the stream in a very realistic way
///                 println!("read {} bytes", chunk.len());
///             }
///             Some(Err(e)) => {
///                 println!("error reading body: {e:?}");
///                 break;
///             }
///         }
///     }
/// }
/// ```
///
/// Access the body in a non-streaming way. This can be useful where your component
/// must take IncomingRequest because some scenarios need streaming, but you
/// have other scenarios that do not.
///
/// ```no_run
/// # use spin_sdk::http::{IncomingRequest, ResponseOutparam};
/// async fn handle_request(req: IncomingRequest, response_outparam: ResponseOutparam) {
///     let body = req.into_body().await.unwrap();
/// }
/// ```
#[doc(inline)]
pub use types::IncomingRequest;

/// Represents an outgoing HTTP response.
///
/// OutgoingResponse is used in conjunction with [ResponseOutparam] in cases where
/// you want to stream the response body. In cases where you don't need to stream,
/// it is often simpler to use [Response].
///
/// # Examples
///
/// Send a streaming response to an incoming request:
///
/// ```no_run
/// # use spin_sdk::http::{Fields, IncomingRequest, OutgoingResponse, ResponseOutparam};
/// async fn handle_request(req: IncomingRequest, response_outparam: ResponseOutparam) {
///     use futures::SinkExt;
///     use std::io::Read;
///
///     let response_headers = Fields::from_list(&[
///         ("content-type".to_owned(), "text/plain".into())
///     ]).unwrap();
///
///     let response = OutgoingResponse::new(response_headers);
///     response.set_status_code(200).unwrap();
///     let mut response_body = response.take_body();
///
///     response_outparam.set(response);
///
///     let mut file = std::fs::File::open("war-and-peace.txt").unwrap();
///
///     loop {
///         let mut buf = vec![0; 1024];
///         let count = file.read(&mut buf).unwrap();
///
///         if count == 0 {
///             break;  // end of file
///         }
///
///         buf.truncate(count);
///         response_body.send(buf).await.unwrap();
///     }
/// }
/// ```
///
/// Send a response then continue processing:
///
/// ```no_run
/// # use spin_sdk::http::{Fields, IncomingRequest, OutgoingResponse, ResponseOutparam};
/// async fn handle_request(req: IncomingRequest, response_outparam: ResponseOutparam) {
///     use futures::SinkExt;
///
///     let response_headers = Fields::from_list(&[
///         ("content-type".to_owned(), "text/plain".into())
///     ]).unwrap();
///
///     let response = OutgoingResponse::new(response_headers);
///     response.set_status_code(200).unwrap();
///     let mut response_body = response.take_body();
///
///     response_outparam.set(response);
///
///     response_body
///         .send("Request accepted".as_bytes().to_vec())
///         .await
///         .unwrap();
///
///     // End the HTTP response so the client deems it complete.
///     response_body.flush().await.unwrap();
///     response_body.close().await.unwrap();
///     drop(response_body);
///
///     // Perform any additional processing
///     println!("While the cat's away, the mice will play");
/// }
/// ```
#[doc(inline)]
pub use types::OutgoingResponse;

/// A common representation for headers and trailers.
///
/// # Examples
///
/// Initialise response headers from a list:
///
/// ```no_run
/// # use spin_sdk::http::{Fields, IncomingRequest, OutgoingResponse};
/// # fn handle_request(req: IncomingRequest) {
/// let response_headers = Fields::from_list(&[
///     ("content-type".to_owned(), "text/plain".into())
/// ]).unwrap();
///
/// let response = OutgoingResponse::new(response_headers);
/// # }
/// ```
///
/// Build response headers up dynamically:
///
/// ```no_run
/// # use spin_sdk::http::{Fields, IncomingRequest, OutgoingResponse};
/// # fn handle_request(req: IncomingRequest) {
/// let accepts_json = req.headers()
///     .get(&"accept".to_owned())
///     .iter()
///     .flat_map(|v| String::from_utf8(v.clone()).ok())
///     .any(|s| s == "application/json");
///
/// let response_headers = Fields::new();
/// if accepts_json {
///     response_headers.set(&"content-type".to_owned(), &["application/json".into()]).unwrap();
/// };
/// # }
/// ```
///
/// # WASI resource documentation
///
#[doc(inline)]
pub use types::Fields;

/// A unified request object that can represent both incoming and outgoing requests.
///
/// This should be used in favor of [IncomingRequest] and [OutgoingRequest] when there
/// is no need for streaming bodies.
///
/// # Examples
///
/// Read the method, a header, and the body an incoming HTTP request, without streaming:
///
/// ```no_run
/// # use spin_sdk::http::{Method, Request, Response};
///
/// fn handle_request(req: Request) -> anyhow::Result<Response> {
///     let method = req.method();
///     let content_type = req.header("content-type");
///     if *method == Method::Post {
///         let body = String::from_utf8_lossy(req.body());
///     }
///     todo!()
/// }
/// ```
///
/// Send an outgoing GET request (no body) to `example.com`:
///
/// ```no_run
/// use spin_sdk::http::{Request, Response};
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let request = Request::get("https://example.com");
/// let response: Response = spin_sdk::http::send(request).await?;
/// # Ok(())
/// # }
/// ```
///
/// Send an outgoing POST request with a non-streaming body to `example.com`.
///
/// ```no_run
/// use spin_sdk::http::{Request, Response};
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let request = Request::post("https://example.com", "it's a-me, Spin")
///     .header("content-type", "text/plain")
///     .build();
/// let response: Response = spin_sdk::http::send(request).await?;
/// # Ok(())
/// # }
/// ```
///
/// Build and send an outgoing request without using the helper shortcut.
///
/// ```no_run
/// use spin_sdk::http::{Method, Request, Response};
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let mut request = Request::new(Method::Put, "https://example.com/message/safety");
/// request.set_header("content-type", "text/plain");
/// *request.body_mut() = "beware the crocodile".as_bytes().to_vec();
/// let response: Response = spin_sdk::http::send(request).await?;
/// # Ok(())
/// # }
/// ```
///
/// Build and send an outgoing request using the fluent builder.
///
/// ```no_run
/// use spin_sdk::http::{Method, Request, Response};
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let request = Request::builder()
///     .uri("https://example.com/message/motivational")
///     .method(Method::Put)
///     .header("content-type", "text/plain")
///     .body("the capybaras of creativity fly higher than the bluebirds of banality")
///     .build();
/// let response: Response = spin_sdk::http::send(request).await?;
/// # Ok(())
/// # }
/// ```
pub struct Request {
    /// The method of the request
    method: Method,
    /// The uri for the request
    ///
    /// The first item is set to `None` if the supplied uri is malformed
    uri: (Option<hyperium::Uri>, String),
    /// The request headers
    headers: HashMap<String, HeaderValue>,
    /// The request body as bytes
    body: Vec<u8>,
}

impl Request {
    /// Creates a new request from a method and uri
    pub fn new(method: Method, uri: impl Into<String>) -> Self {
        Self {
            method,
            uri: Self::parse_uri(uri.into()),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Creates a [`RequestBuilder`]
    pub fn builder() -> RequestBuilder {
        RequestBuilder::new(Method::Get, "/")
    }

    /// Creates a [`RequestBuilder`] to GET the given `uri`
    pub fn get(uri: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(Method::Get, uri)
    }

    /// Creates a [`RequestBuilder`] to POST the given `body` to `uri`
    pub fn post(uri: impl Into<String>, body: impl conversions::IntoBody) -> RequestBuilder {
        let mut builder = RequestBuilder::new(Method::Post, uri);
        builder.body(body);
        builder
    }

    /// Creates a [`RequestBuilder`] to PUT the given `body` to `uri`
    pub fn put(uri: impl Into<String>, body: impl conversions::IntoBody) -> RequestBuilder {
        let mut builder = RequestBuilder::new(Method::Put, uri);
        builder.body(body);
        builder
    }

    /// Creates a [`RequestBuilder`] to PATCH the resource specified by `uri`
    pub fn patch(uri: impl Into<String>, body: impl conversions::IntoBody) -> RequestBuilder {
        let mut builder = RequestBuilder::new(Method::Patch, uri);
        builder.body(body);
        builder
    }

    /// Creates a [`RequestBuilder`] to DELETE the resource specified by `uri`
    pub fn delete(uri: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(Method::Delete, uri)
    }

    /// The request method
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// The request uri
    pub fn uri(&self) -> &str {
        &self.uri.1
    }

    /// The request uri path
    pub fn path(&self) -> &str {
        self.uri.0.as_ref().map(|u| u.path()).unwrap_or_default()
    }

    /// The request uri query
    pub fn query(&self) -> &str {
        self.uri
            .0
            .as_ref()
            .and_then(|u| u.query())
            .unwrap_or_default()
    }

    /// The request headers
    pub fn headers(&self) -> impl Iterator<Item = (&str, &HeaderValue)> {
        self.headers.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Return a header value
    ///
    /// Will return `None` if the header does not exist.
    pub fn header(&self, name: &str) -> Option<&HeaderValue> {
        self.headers.get(&name.to_lowercase())
    }

    /// Set a header
    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(
            name.into(),
            HeaderValue {
                inner: HeaderValueRep::String(value.into()),
            },
        );
    }

    /// The request body
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// The request body
    pub fn body_mut(&mut self) -> &mut Vec<u8> {
        &mut self.body
    }

    /// Consume this type and return its body
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }

    fn parse_uri(uri: String) -> (Option<hyperium::Uri>, String) {
        (
            hyperium::Uri::try_from(&uri)
                .or_else(|_| hyperium::Uri::try_from(&format!("http://{uri}")))
                .ok(),
            uri,
        )
    }

    /// Whether the request is an HTTPS request
    fn is_https(&self) -> bool {
        self.uri
            .0
            .as_ref()
            .and_then(|u| u.scheme())
            .map(|s| s == &hyperium::uri::Scheme::HTTPS)
            .unwrap_or(true)
    }

    /// The URI's authority
    fn authority(&self) -> Option<&str> {
        self.uri
            .0
            .as_ref()
            .and_then(|u| u.authority())
            .map(|a| a.as_str())
    }

    /// The request path and query combined
    pub fn path_and_query(&self) -> Option<&str> {
        self.uri
            .0
            .as_ref()
            .and_then(|u| u.path_and_query())
            .map(|s| s.as_str())
    }
}

/// A builder for non-streaming outgoing HTTP requests. You can obtain
/// a RequestBuilder from the [Request::builder()] method, or from
/// method-specific helpers such as [Request::get()] or [Request::post()].
///
/// # Examples
///
/// Use a method helper to build an outgoing POST request:
///
/// ```no_run
/// use spin_sdk::http::{Request, Response};
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let request = Request::post("https://example.com", "it's a-me, Spin")
///     .header("content-type", "text/plain")
///     .build();
/// let response: Response = spin_sdk::http::send(request).await?;
/// # Ok(())
/// # }
/// ```
///
/// Build and send an outgoing request using the RequestBuilder.
///
/// ```no_run
/// use spin_sdk::http::{Method, Request, Response};
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let request = Request::builder()
///     .uri("https://example.com/message/motivational")
///     .method(Method::Put)
///     .header("content-type", "text/plain")
///     .body("the capybaras of creativity fly higher than the bluebirds of banality")
///     .build();
/// let response: Response = spin_sdk::http::send(request).await?;
/// # Ok(())
/// # }
/// ```
pub struct RequestBuilder {
    request: Request,
}

impl RequestBuilder {
    /// Create a new `RequestBuilder`
    pub fn new(method: Method, uri: impl Into<String>) -> Self {
        Self {
            request: Request::new(method, uri.into()),
        }
    }

    /// Set the method
    pub fn method(&mut self, method: Method) -> &mut Self {
        self.request.method = method;
        self
    }

    /// Set the uri
    pub fn uri(&mut self, uri: impl Into<String>) -> &mut Self {
        self.request.uri = Request::parse_uri(uri.into());
        self
    }

    /// Set the headers
    pub fn headers(&mut self, headers: impl conversions::IntoHeaders) -> &mut Self {
        self.request.headers = into_header_rep(headers);
        self
    }

    /// Set a header
    pub fn header(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.request
            .headers
            .insert(key.into().to_lowercase(), HeaderValue::string(value.into()));
        self
    }

    /// Set the body
    pub fn body(&mut self, body: impl conversions::IntoBody) -> &mut Self {
        self.request.body = body.into_body();
        self
    }

    /// Build the `Request`
    pub fn build(&mut self) -> Request {
        std::mem::replace(&mut self.request, Request::new(Method::Get, "/"))
    }
}

/// A unified response object that can represent both outgoing and incoming responses.
///
/// This should be used in favor of `OutgoingResponse` and `IncomingResponse` when there
/// is no need for streaming bodies.
///
/// # Examples
///
/// Send a response to an incoming HTTP request:
///
/// ```no_run
/// use spin_sdk::http::{Request, Response};
///
/// fn handle_request(req: Request) -> anyhow::Result<Response> {
///     Ok(Response::builder()
///         .status(200)
///         .header("content-type", "text/plain")
///         .body("Hello, world")
///         .build())
/// }
/// ```
///
/// Parse a response from an outgoing HTTP request:
///
/// ```no_run
/// # use spin_sdk::http::{Request, Response};
/// #[derive(serde::Deserialize)]
/// struct User {
///     name: String,
/// }
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let request = Request::get("https://example.com");
/// let response: Response = spin_sdk::http::send(request).await?;
/// if *response.status() == 200 {
///     let body = response.body();
///     let user: User = serde_json::from_slice(body)?;
/// }
/// # Ok(())
/// # }
/// ```
pub struct Response {
    /// The status of the response
    status: StatusCode,
    /// The response headers
    headers: HashMap<String, HeaderValue>,
    /// The body of the response as bytes
    body: Vec<u8>,
}

impl Response {
    /// Create a new response from a status and body
    pub fn new(status: impl conversions::IntoStatusCode, body: impl conversions::IntoBody) -> Self {
        Self {
            status: status.into_status_code(),
            headers: HashMap::new(),
            body: body.into_body(),
        }
    }

    /// The response status
    pub fn status(&self) -> &StatusCode {
        &self.status
    }

    /// The request headers
    pub fn headers(&self) -> impl Iterator<Item = (&str, &HeaderValue)> {
        self.headers.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Return a header value
    ///
    /// Will return `None` if the header does not exist.
    pub fn header(&self, name: &str) -> Option<&HeaderValue> {
        self.headers.get(&name.to_lowercase())
    }

    /// Set a response header
    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(
            name.into(),
            HeaderValue {
                inner: HeaderValueRep::String(value.into()),
            },
        );
    }

    /// The response body
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// The response body
    pub fn body_mut(&mut self) -> &mut Vec<u8> {
        &mut self.body
    }

    /// Consume this type and return its body
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }

    /// Converts this response into a [`ResponseBuilder`]. This can be used to
    /// update a response before passing it on.
    pub fn into_builder(self) -> ResponseBuilder {
        ResponseBuilder { response: self }
    }

    /// Creates a [`ResponseBuilder`]
    pub fn builder() -> ResponseBuilder {
        ResponseBuilder::new(200)
    }
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .field("body.len()", &self.body.len())
            .finish()
    }
}

/// A builder for `Response``
pub struct ResponseBuilder {
    response: Response,
}

impl ResponseBuilder {
    /// Create a new `ResponseBuilder`
    pub fn new(status: impl conversions::IntoStatusCode) -> Self {
        ResponseBuilder {
            response: Response::new(status, Vec::new()),
        }
    }

    /// Set the status
    pub fn status(&mut self, status: impl conversions::IntoStatusCode) -> &mut Self {
        self.response.status = status.into_status_code();
        self
    }

    /// Set the headers
    pub fn headers(&mut self, headers: impl conversions::IntoHeaders) -> &mut Self {
        self.response.headers = into_header_rep(headers.into_headers());
        self
    }

    /// Set a header
    pub fn header(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.response
            .headers
            .insert(key.into().to_lowercase(), HeaderValue::string(value.into()));
        self
    }

    /// Set the body
    pub fn body(&mut self, body: impl conversions::IntoBody) -> &mut Self {
        self.response.body = body.into_body();
        self
    }

    /// Build the `Response`
    pub fn build(&mut self) -> Response {
        std::mem::replace(&mut self.response, Response::new(200, Vec::new()))
    }
}

/// A header value.
///
/// Since header values do not have to be valid utf8, this allows for
/// both utf8 strings and bags of bytes.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HeaderValue {
    inner: HeaderValueRep,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum HeaderValueRep {
    /// Header value encoded as a utf8 string
    String(String),
    /// Header value as a bag of bytes
    Bytes(Vec<u8>),
}

impl HeaderValue {
    /// Construct a `HeaderValue` from a string
    pub fn string(str: String) -> HeaderValue {
        HeaderValue {
            inner: HeaderValueRep::String(str),
        }
    }

    /// Construct a `HeaderValue` from a bag of bytes
    pub fn bytes(bytes: Vec<u8>) -> HeaderValue {
        HeaderValue {
            inner: String::from_utf8(bytes)
                .map(HeaderValueRep::String)
                .unwrap_or_else(|e| HeaderValueRep::Bytes(e.into_bytes())),
        }
    }

    /// Get the `HeaderValue` as a utf8 encoded string
    ///
    /// Returns `None` if the value is a non utf8 encoded header value
    pub fn as_str(&self) -> Option<&str> {
        match &self.inner {
            HeaderValueRep::String(s) => Some(s),
            HeaderValueRep::Bytes(b) => std::str::from_utf8(b).ok(),
        }
    }

    /// Get the `HeaderValue` as bytes
    pub fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }

    /// Turn the `HeaderValue` into a String (in a lossy way if the `HeaderValue` is a bag of bytes)
    pub fn into_utf8_lossy(self) -> String {
        match self.inner {
            HeaderValueRep::String(s) => s,
            HeaderValueRep::Bytes(b) => String::from_utf8_lossy(&b).into_owned(),
        }
    }

    /// Turn the `HeaderValue` into bytes
    pub fn into_bytes(self) -> Vec<u8> {
        match self.inner {
            HeaderValueRep::String(s) => s.into_bytes(),
            HeaderValueRep::Bytes(b) => b,
        }
    }
}

impl AsRef<[u8]> for HeaderValue {
    fn as_ref(&self) -> &[u8] {
        match &self.inner {
            HeaderValueRep::String(s) => s.as_bytes(),
            HeaderValueRep::Bytes(b) => b,
        }
    }
}

fn into_header_rep(headers: impl conversions::IntoHeaders) -> HashMap<String, HeaderValue> {
    headers
        .into_headers()
        .into_iter()
        .map(|(k, v)| {
            let v = String::from_utf8(v)
                .map(HeaderValueRep::String)
                .unwrap_or_else(|e| HeaderValueRep::Bytes(e.into_bytes()));
            (k.to_lowercase(), HeaderValue { inner: v })
        })
        .collect()
}

impl std::hash::Hash for Method {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl Eq for Method {}

impl PartialEq for Method {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Other(l), Self::Other(r)) => l == r,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Connect => "CONNECT",
            Method::Trace => "TRACE",
            Method::Other(o) => o,
        })
    }
}

impl IncomingRequest {
    /// The incoming request Uri
    pub fn uri(&self) -> String {
        let scheme_and_authority =
            if let (Some(scheme), Some(authority)) = (self.scheme(), self.authority()) {
                let scheme = match &scheme {
                    Scheme::Http => "http://",
                    Scheme::Https => "https://",
                    Scheme::Other(s) => s.as_str(),
                };
                format!("{scheme}{authority}")
            } else {
                String::new()
            };
        let path_and_query = self.path_with_query().unwrap_or_default();
        format!("{scheme_and_authority}{path_and_query}")
    }

    /// Return a `Stream` from which the body of the specified request may be read.
    ///
    /// # Panics
    ///
    /// Panics if the body was already consumed.
    pub fn into_body_stream(self) -> impl futures::Stream<Item = Result<Vec<u8>, streams::Error>> {
        executor::incoming_body(self.consume().expect("request body was already consumed"))
    }

    /// Return a `Vec<u8>` of the body or fails
    pub async fn into_body(self) -> Result<Vec<u8>, streams::Error> {
        use futures::TryStreamExt;
        let mut stream = self.into_body_stream();
        let mut body = Vec::new();
        while let Some(chunk) = stream.try_next().await? {
            body.extend(chunk);
        }
        Ok(body)
    }
}

impl IncomingResponse {
    /// Return a `Stream` from which the body of the specified response may be read.
    ///
    /// # Panics
    ///
    /// Panics if the body was already consumed.
    // TODO: This should ideally take ownership of `self` and be called `into_body_stream` (i.e. symmetric with
    // `IncomingRequest::into_body_stream`).  However, as of this writing, `wasmtime-wasi-http` is implemented in
    // such a way that dropping an `IncomingResponse` will cause the request to be cancelled, meaning the caller
    // won't necessarily have a chance to send the request body if they haven't started doing so yet (or, if they
    // have started, they might not be able to finish before the connection is closed).  See
    // https://github.com/bytecodealliance/wasmtime/issues/7413 for details.
    pub fn take_body_stream(&self) -> impl futures::Stream<Item = Result<Vec<u8>, streams::Error>> {
        executor::incoming_body(self.consume().expect("response body was already consumed"))
    }

    /// Return a `Vec<u8>` of the body or fails
    ///
    /// # Panics
    ///
    /// Panics if the body was already consumed.
    pub async fn into_body(self) -> Result<Vec<u8>, streams::Error> {
        use futures::TryStreamExt;
        let mut stream = self.take_body_stream();
        let mut body = Vec::new();
        while let Some(chunk) = stream.try_next().await? {
            body.extend(chunk);
        }
        Ok(body)
    }
}

impl OutgoingResponse {
    /// Construct a `Sink` which writes chunks to the body of the specified response.
    ///
    /// # Panics
    ///
    /// Panics if the body was already taken.
    pub fn take_body(&self) -> impl futures::Sink<Vec<u8>, Error = StreamError> {
        executor::outgoing_body(self.body().expect("response body was already taken"))
    }
}

impl OutgoingRequest {
    /// Construct a `Sink` which writes chunks to the body of the specified response.
    ///
    /// # Panics
    ///
    /// Panics if the body was already taken.
    pub fn take_body(&self) -> impl futures::Sink<Vec<u8>, Error = StreamError> {
        executor::outgoing_body(self.body().expect("request body was already taken"))
    }
}

/// A parameter provided by Spin for setting a streaming [OutgoingResponse].
///
/// # Examples
///
/// Send a streaming response to an incoming request:
///
/// ```no_run
/// # use spin_sdk::http::{Fields, IncomingRequest, OutgoingResponse, ResponseOutparam};
/// async fn handle_request(req: IncomingRequest, response_outparam: ResponseOutparam) {
///     use futures::SinkExt;
///     use std::io::Read;
///
///     let response_headers = Fields::from_list(&[
///         ("content-type".to_owned(), "text/plain".into())
///     ]).unwrap();
///
///     let response = OutgoingResponse::new(response_headers);
///     response.set_status_code(200).unwrap();
///     let mut response_body = response.take_body();
///
///     response_outparam.set(response);
///
///     let mut file = std::fs::File::open("war-and-peace.txt").unwrap();
///
///     loop {
///         let mut buf = vec![0; 1024];
///         let count = file.read(&mut buf).unwrap();
///
///         if count == 0 {
///             break;  // end of file
///         }
///
///         buf.truncate(count);
///         response_body.send(buf).await.unwrap();
///     }
/// }
/// ```
pub struct ResponseOutparam(types::ResponseOutparam);

impl ResponseOutparam {
    #[doc(hidden)]
    // This is needed for the macro so we can transfrom the macro's
    // `ResponseOutparam` to this `ResponseOutparam`
    pub unsafe fn from_handle(handle: u32) -> Self {
        Self(types::ResponseOutparam::from_handle(handle))
    }

    /// Set the outgoing response
    pub fn set(self, response: OutgoingResponse) {
        types::ResponseOutparam::set(self.0, Ok(response));
    }

    /// Set with the outgoing response and the supplied buffer
    ///
    /// Will panic if response body has already been taken
    pub async fn set_with_body(
        self,
        response: OutgoingResponse,
        buffer: Vec<u8>,
    ) -> Result<(), StreamError> {
        let mut body = response.take_body();
        self.set(response);
        body.send(buffer).await
    }

    /// Return the inner, `wit-bindgen`-generated instance
    pub fn into_inner(self) -> types::ResponseOutparam {
        self.0
    }
}

/// Send an outgoing request
///
/// # Examples
///
/// Get the example.com home page:
///
/// ```no_run
/// use spin_sdk::http::{Request, Response};
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let request = Request::get("example.com").build();
/// let response: Response = spin_sdk::http::send(request).await?;
/// println!("{}", response.body().len());
/// # Ok(())
/// # }
/// ```
///
/// Use the `http` crate Request type to send a data transfer value:
///
/// ```no_run
/// use hyperium::Request;
///
/// #[derive(serde::Serialize)]
/// struct User {
///     name: String,
/// }
///
/// impl spin_sdk::http::conversions::TryIntoBody for User {
///     type Error = serde_json::Error;
///
///     fn try_into_body(self) -> Result<Vec<u8>, Self::Error> {
///         serde_json::to_vec(&self)
///     }
/// }
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let user = User {
///     name: "Alice".to_owned(),
/// };
///
/// let request = hyperium::Request::builder()
///     .method("POST")
///     .uri("https://example.com/users")
///     .header("content-type", "application/json")
///     .body(user)?;
///
/// let response: hyperium::Response<()> = spin_sdk::http::send(request).await?;
///
/// println!("{}", response.status().is_success());
/// # Ok(())
/// # }
/// ```
#[doc(alias = "fetch")]
pub async fn send<I, O>(request: I) -> Result<O, SendError>
where
    I: TryIntoOutgoingRequest,
    I::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    O: TryFromIncomingResponse,
    O::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
{
    let (request, body_buffer) = I::try_into_outgoing_request(request)
        .map_err(|e| SendError::RequestConversion(e.into()))?;
    let response = if let Some(body_buffer) = body_buffer {
        // It is part of the contract of the trait that implementors of `TryIntoOutgoingRequest`
        // do not call `OutgoingRequest::write`` if they return a buffered body.
        let mut body_sink = request.take_body();
        let response = executor::outgoing_request_send(request);
        body_sink.send(body_buffer).await.map_err(SendError::Io)?;
        drop(body_sink);
        response.await.map_err(SendError::Http)?
    } else {
        executor::outgoing_request_send(request)
            .await
            .map_err(SendError::Http)?
    };

    TryFromIncomingResponse::try_from_incoming_response(response)
        .await
        .map_err(|e: O::Error| SendError::ResponseConversion(e.into()))
}

/// An error encountered when performing an HTTP request
#[derive(thiserror::Error, Debug)]
pub enum SendError {
    /// Error converting to a request
    #[error(transparent)]
    RequestConversion(Box<dyn std::error::Error + Send + Sync>),
    /// Error converting from a response
    #[error(transparent)]
    ResponseConversion(Box<dyn std::error::Error + Send + Sync>),
    /// An I/O error
    #[error(transparent)]
    Io(StreamError),
    /// An HTTP error
    #[error(transparent)]
    Http(ErrorCode),
}

#[doc(hidden)]
/// The executor for driving wasi-http futures to completion
mod executor;
#[doc(hidden)]
pub use executor::run;

/// An error parsing a JSON body
#[cfg(feature = "json")]
#[derive(Debug)]
pub struct JsonBodyError(serde_json::Error);

#[cfg(feature = "json")]
impl std::error::Error for JsonBodyError {}

#[cfg(feature = "json")]
impl std::fmt::Display for JsonBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("could not convert body to json")
    }
}

/// An error when the body is not UTF-8
#[derive(Debug)]
pub struct NonUtf8BodyError;

impl std::error::Error for NonUtf8BodyError {}

impl std::fmt::Display for NonUtf8BodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("body was expected to be utf8 but was not")
    }
}

mod router;
#[doc(inline)]
pub use router::*;

/// A Body extractor
#[derive(Debug)]
pub struct Body<T>(pub T);

impl<T> std::ops::Deref for Body<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A Json extractor
#[derive(Debug)]
pub struct Json<T>(pub T);

impl<T> std::ops::Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Helper functions for creating responses
pub mod responses {
    use super::Response;

    /// Helper function to return a 404 Not Found response.
    pub fn not_found() -> Response {
        Response::new(404, "Not Found")
    }

    /// Helper function to return a 500 Internal Server Error response.
    pub fn internal_server_error() -> Response {
        Response::new(500, "Internal Server Error")
    }

    /// Helper function to return a 405 Method Not Allowed response.
    pub fn method_not_allowed() -> Response {
        Response::new(405, "Method Not Allowed")
    }

    pub(crate) fn bad_request(msg: Option<String>) -> Response {
        Response::new(400, msg.map(|m| m.into_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uri_parses() {
        let uri = "/hello?world=1";
        let req = Request::new(Method::Get, uri);
        assert_eq!(req.uri(), uri);
        assert_eq!(req.path(), "/hello");
        assert_eq!(req.query(), "world=1");

        let uri = "http://localhost:3000/hello?world=1";
        let req = Request::new(Method::Get, uri);
        assert_eq!(req.uri(), uri);
        assert_eq!(req.path(), "/hello");
        assert_eq!(req.query(), "world=1");

        let uri = "localhost:3000/hello?world=1";
        let req = Request::new(Method::Get, uri);
        assert_eq!(req.uri(), uri);
        assert_eq!(req.path(), "/hello");
        assert_eq!(req.query(), "world=1");
    }
}
