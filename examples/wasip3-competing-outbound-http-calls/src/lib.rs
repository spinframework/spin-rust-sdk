use std::pin::pin;

use futures::SinkExt;
use http::Request;
use serde::{Deserialize, Serialize};
use spin_sdk::http_wasip3::http_service;
use spin_sdk::http_wasip3::{send, EmptyBody, IntoResponse};

use bytes::Bytes;
use futures::{
    channel::mpsc::{channel, Sender},
    StreamExt,
};

#[http_service]
async fn handle_concurrent_outbound_http_calls(
    _req: spin_sdk::http_wasip3::Request,
) -> impl IntoResponse {
    println!("Handling reuqest");

    // A lot of code taken from: https://github.com/spinframework/spin-rust-sdk/blob/main/examples/wasip3-streaming/src/lib.rs

    // Create a streaming Body implementation that backs onto a `mpsc`
    // channel. The function returns the sender side of the channel; the
    // receiver end becomes the body. So anything written to the sender
    // side will be sent out over the HTTP response.
    let (mut tx, body) = bytes_stream_body();

    // Use wit_bindgen::spawn to allow the async block to keep running
    // after the handler returns.
    spin_sdk::http_wasip3::wasip3::wit_bindgen::spawn(async move {
        // The two outbound calls
        let spin = pin!(get_content_length("https://spinframework.dev"));
        let book = pin!(get_content_length(
            "https://component-model.bytecodealliance.org"
        ));

        // Getting the first response back
        let first_result = futures::future::select(spin, book).await.factor_first();

        // We need to keep the future around to also handle the second response,
        // hence instantiating a new variable for the ContentLength struct
        let first_response = first_result.0.unwrap();

        // Let's print some stats to the server console
        println!(
            "HEAD request done: {:?}, took {:?} ms, content-length: {:?}",
            first_response.url, first_response.ms_taken, first_response.content_length
        );

        // Serializing the struct as JSON represented as bytes
        let bytes: Bytes = serde_json::to_vec_pretty(&first_response)
            .expect("Failed to serialize!")
            .try_into()
            .unwrap();

        // Sends the bytes over the channel and closes te channel, as we're done with the client response
        tx.send(bytes).await.unwrap();
        tx.close_channel();

        // Handles the secons request (future) as it returns a reponse
        let second_response = first_result.1.await.expect("Failed to get second response");

        // And printing stats to the server console, as the client connection is already closed
        println!(
            "HEAD request done: {:?}, took {:?} ms, content-length: {:?}",
            second_response.url, second_response.ms_taken, second_response.content_length
        );

        println!("Done");
    });

    // Returning the body, once the channel closes
    http::Response::new(body)
}

// Getting the content length via an HTTP HEAD request
async fn get_content_length(url: &str) -> anyhow::Result<ResponseStats> {
    let request = Request::head(url).body(EmptyBody::new())?;
    let start = std::time::SystemTime::now();
    println!("HEAD request sent: {url}");
    let response = send(request).await?;
    let cl_header = response.headers().get("content-length");
    let cl = cl_header
        .and_then(|hval| hval.to_str().ok())
        .and_then(|hval| hval.parse().ok());
    let end = std::time::SystemTime::now()
        .duration_since(start)
        .expect("Failed to get time");
    Ok(ResponseStats {
        content_length: cl.unwrap(),
        ms_taken: end.as_millis(),
        url: url.to_string(),
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ResponseStats {
    content_length: u64,
    ms_taken: u128,
    url: String,
}

// Helper function to create a streaming body.
fn bytes_stream_body() -> (
    Sender<bytes::Bytes>,
    impl http_body::Body<Data = Bytes, Error = anyhow::Error>,
) {
    // The send and receive sides of a channel
    let (tx, rx) = channel::<Bytes>(1024);
    // The receive side is a stream, so we can use combinators like `map`
    // to transform it into a form that the response plumbing is happy
    // with. The app logic that writes to the stream doesn't need to see
    // any of this.
    let stm = rx.map(|value| Ok(http_body::Frame::data(value)));
    // Construct a Body implementation over the stream.
    let body = http_body_util::StreamBody::new(stm);
    // Return the send side (so that app logic can write to the body) and the
    // body (so it can be put in a Response!).
    (tx, body)
}
