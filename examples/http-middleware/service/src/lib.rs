use spin_sdk::http::{IntoResponse, Request};
use spin_sdk::http_service;
use std::fmt::Write;

/// The terminal component in the chain: the actual service being wrapped.
///
/// It reports the request headers it received so you can confirm that the
/// `x-added-by-middleware` header was injected by the middleware *before* the
/// request reached this component.
#[http_service]
async fn service(req: Request) -> impl IntoResponse {
    let mut body = String::from("Hello from the wrapped service!\n\n");
    body.push_str("Request headers seen by this component:\n");
    for (name, value) in req.headers() {
        let value = value.to_str().unwrap_or("<non-utf8>");
        let _ = writeln!(body, "  {name}: {value}");
    }
    body
}
