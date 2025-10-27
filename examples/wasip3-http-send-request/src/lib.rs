use spin_sdk::http_wasip3::{http_service, send, EmptyBody, IntoResponse, Request, Result};

/// Sends a request to a URL.
#[http_service]
async fn send_request(_req: Request) -> Result<impl IntoResponse> {
    let outgoing = http::Request::get("https://bytecodealliance.org").body(EmptyBody::new())?;

    Ok(send(outgoing).await?)
}
