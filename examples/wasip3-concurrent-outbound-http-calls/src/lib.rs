use std::pin::pin;

use futures::future::Either;
use http::Request;
use spin_sdk::http_wasip3::{send, EmptyBody, IntoResponse};
use spin_sdk::http_wasip3::http_component;

#[http_component]
async fn handle_concurrent_outbound_http_calls(_req: spin_sdk::http_wasip3::Request) -> anyhow::Result<impl IntoResponse> {

    let spin = pin!(get_content_length("https://spinframework.dev"));
    let book = pin!(get_content_length("https://component-model.bytecodealliance.org/"));

    let (first, len) = match futures::future::select(spin, book).await {
        Either::Left(len) => ("Spin docs", len),
        Either::Right(len) => ("Component model book", len),
    };

    let response = format!("{first} site was first response with content-length {:?}\n", len.0?);

    Ok(response)
}

async fn get_content_length(url: &str) -> anyhow::Result<Option<u64>> {
    let request = Request::get(url).body(EmptyBody::new())?;
    let response = send(request).await?;
    let cl_header = response.headers().get("content-length");
    let cl = cl_header.and_then(|hval| hval.to_str().ok()).and_then(|hval| hval.parse().ok());
    Ok(cl)
}
