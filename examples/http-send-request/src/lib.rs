use spin_sdk::http::{send, EmptyBody, IntoResponse, Request, Result};
use spin_sdk::http_service;

/// Sends a request to a URL.
#[http_service]
async fn send_request(_req: Request) -> Result<impl IntoResponse> {
    let outgoing = Request::get("https://bytecodealliance.org").body(EmptyBody::new())?;
    Ok(send(outgoing).await?)
}
