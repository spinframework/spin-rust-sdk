use spin_sdk::http::{self, IntoResponse, Request, Result};
use spin_sdk::http_service;

/// Sends a request to a URL.
#[http_service]
async fn send_request(_req: Request) -> Result<impl IntoResponse> {
    Ok(http::get("https://bytecodealliance.org").await?)
}
