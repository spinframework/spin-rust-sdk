use spin_sdk::http_wasip3::body::IncomingBodyExt;
use spin_sdk::http_wasip3::http_service;
use spin_sdk::http_wasip3::{IntoResponse, Request};

use bytes::Bytes;
use futures::{
    channel::mpsc::{channel, Sender},
    SinkExt, StreamExt,
};

/// A streaming Spin HTTP component.
///
/// This component is an (almost) pure echo server: it streams the
/// request body back to the client, with a start and end marker
/// for UI visibility. A real service would of course process the
/// request in some way!
#[http_service]
async fn handle_wasip3_streaming(request: Request) -> impl IntoResponse {
    // We want to process the incoming body as a stream. This allows
    // us to send the first bytes of the response without waiting
    // for the request to arrive in full (and without requiring us to
    // hold the whole request body in memory).
    let mut in_body = request.into_body().stream();

    // Create a streaming Body implementation that backs onto a `mpsc`
    // channel. The function returns the sender side of the channel; the
    // receiver end becomes the body. So anything written to the sender
    // side will be sent out over the HTTP response.
    let (mut tx, body) = bytes_stream_body();

    // Use wit_bindgen::spawn to allow the async block to keep running
    // after the handler returns.
    spin_sdk::http_wasip3::spawn(async move {
        tx.send("-- INBOUND MESSAGE --\n".into()).await.unwrap();
        // Keep processing data from the incoming body stream until it ends...
        loop {
            let Some(chunk) = in_body.next().await else {
                break;
            };
            // and just copy that data to the response body stream. Each chunk
            // is sent to the client immediately (for some value of immediately) -
            // we do not wait to build up the whole response on the server.
            tx.send(chunk.unwrap()).await.unwrap();
        }
        tx.send("\n---------------------\n".into()).await.unwrap();
    });

    // Return the streaming Response object. The async block continues
    // to stream data into the response body despite the function returning.
    http::Response::new(body)
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
