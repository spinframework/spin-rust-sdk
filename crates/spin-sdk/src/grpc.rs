//! gRPC helpers for serving tonic services from Spin components.
//!
//! This module provides a thin integration layer between the Spin HTTP
//! subsystem and [tonic](https://docs.rs/tonic)-generated gRPC servers.
//! Tonic's generated server types (e.g. `GreeterServer<T>`) implement
//! [`tower_service::Service`] over HTTP requests, so they can accept
//! Spin's incoming [`Request`](crate::http::Request) directly.
//!
//! # Example
//!
//! ```ignore
//! use spin_sdk::http::{IntoResponse, Request};
//! use spin_sdk::http_service;
//! #[http_service]
//! async fn handler(req: Request) -> impl IntoResponse {
//!     spin_sdk::grpc::serve(GreeterServer::new(MyGreeter), req).await
//! }
//! ```

use hyperium as http;
use std::convert::Infallible;

/// Serve a gRPC request by forwarding it to a tower service.
///
/// This function is designed to work with tonic-generated server types,
/// which implement `tower::Service<http::Request<B>>` for any body `B`
/// satisfying `http_body::Body + Send + 'static`.
///
/// The response is returned as an [`http::Response<B>`] which implements
/// [`IntoResponse`](crate::http::IntoResponse), so it integrates directly
/// with the `#[http_service]` handler return type.
///
/// # Extracting a gRPC service from a [`Router`]
///
/// If you have multiple services, you can compose them with
/// [`tonic::transport::server::Router`] at the type level, or simply
/// match on the request path and delegate to different `serve` calls.
///
/// # Example
///
/// ```ignore
/// use spin_sdk::http::{IntoResponse, Request};
/// use spin_sdk::{grpc, http_service};
/// #[http_service]
/// async fn handler(req: Request) -> impl IntoResponse {
///     grpc::serve(GreeterServer::new(MyGreeter), req).await
/// }
/// ```
pub async fn serve<S, B>(mut svc: S, req: crate::http::Request) -> http::Response<B>
where
    S: tower_service::Service<
        crate::http::Request,
        Response = http::Response<B>,
        Error = Infallible,
    >,
{
    // Infallible error — unwrap is safe.
    svc.call(req).await.unwrap_or_else(|e| match e {})
}
