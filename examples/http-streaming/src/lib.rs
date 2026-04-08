use spin_sdk::http::body::IncomingBodyExt;
use spin_sdk::http::{body, IntoResponse, Request};
use spin_sdk::http_service;

use bytes::Bytes;
use futures::{SinkExt, StreamExt};

/// A streaming Spin HTTP component.
///
/// This component is an (almost) pure echo server: it streams the
/// request body back to the client, with a start and end marker
/// for UI visibility. A real service would of course process the
/// request in some way!
#[http_service]
async fn handle_streaming(request: Request) -> impl IntoResponse {
    // We want to process the incoming body as a stream. This allows
    // us to send the first bytes of the response without waiting
    // for the request to arrive in full (and without requiring us to
    // hold the whole request body in memory).
    let mut in_body = request.into_body().stream();

    // Create a streaming Body implementation that backs onto a `mpsc`
    // channel. The function returns the sender side of the channel; the
    // receiver end becomes the body. So anything written to the sender
    // side will be sent out over the HTTP response.
    let (mut tx, body) = body::stream::<Bytes>();

    // Use wasip3::spawn to allow the async block to keep running
    // after the handler returns.
    spin_sdk::wasip3::spawn(async move {
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
