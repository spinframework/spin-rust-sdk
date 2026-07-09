use spin_sdk::http::{self, HeaderName, HeaderValue, Request, Response};
use spin_sdk::http_service;

/// An HTTP middleware component.
///
/// A middleware sits *in front of* another component. It receives the incoming
/// request, may inspect or modify it, forwards it to the next handler in the
/// chain with [`http::next`], and may then inspect or modify the response
/// before returning it to the caller.
///
/// The "next handler" is wired up by Spin, not by this code: in `spin.toml`
/// this component is listed in the target component's `dependencies.middleware`
/// array. That handler may be another middleware or the terminal component.
#[http_service]
async fn middleware(mut req: Request) -> http::Result<Response> {
    // Request runs on the way in, before the next handler.
    eprintln!("[middleware] --> {} {}", req.method(), req.uri().path());

    // Inject a request header that downstream handlers will see.
    req.headers_mut().insert(
        HeaderName::from_static("x-added-by-middleware"),
        HeaderValue::from_static("spin-rust-sdk"),
    );

    // Forward the (modified) request to the next handler in the chain and wait
    // for its response.
    let mut resp = http::next(req).await?;

    // Response runs on the way out, after the next handler
    eprintln!("[middleware] <-- {}", resp.status());

    // Inject a response header that the client will see.
    resp.headers_mut().insert(
        HeaderName::from_static("x-processed-by-middleware"),
        HeaderValue::from_static("spin-rust-sdk"),
    );

    Ok(resp)
}
