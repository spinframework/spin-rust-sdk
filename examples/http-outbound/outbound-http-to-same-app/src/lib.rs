use anyhow::Result;
use spin_sdk::{
    http::{send, EmptyBody, IntoResponse, Request},
    http_service,
};

/// Send an HTTP request and return the response.
#[http_service]
async fn send_outbound(_req: Request) -> Result<impl IntoResponse> {
    let outgoing = http::Request::get("/hello").body(EmptyBody::new())?;
    let mut resp = send(outgoing).await?;
    resp.headers_mut().insert(
        http::HeaderName::from_static("spin-component"),
        http::HeaderValue::from_static("rust-outbound-http"),
    );
    Ok(resp)
}
