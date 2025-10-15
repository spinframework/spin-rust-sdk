use axum::body::Body;
use spin_sdk::http_wasip3::{http_component, send, IncomingRequest, IntoResponse, Result};

/// Sends a request to a URL.
#[http_component]
async fn send_request(_req: IncomingRequest) -> Result<impl IntoResponse> {
    let outgoing = http::Request::get("https://bytecodealliance.org").body(Body::empty())?;

    Ok(send(outgoing).await?)
}
