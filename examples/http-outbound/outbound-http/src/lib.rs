use anyhow::Result;
use spin_sdk::{
    http::{send, EmptyBody, IntoResponse, Request, Response},
    http_service,
};

/// Send an HTTP request and return the response.
#[http_service]
async fn send_outbound(_req: Request) -> Result<impl IntoResponse> {
    let outgoing = Request::get("https://random-data-api.fermyon.app/animals/json")
        .body(EmptyBody::new())
        .unwrap();

    let mut resp: Response = send(outgoing).await?;
    resp.headers_mut().insert(
        http::HeaderName::from_static("spin-component"),
        http::HeaderValue::from_static("rust-outbound-http"),
    );

    Ok(resp)
}
