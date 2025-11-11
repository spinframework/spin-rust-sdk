use std::pin::pin;
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures::{
    channel::mpsc::{channel, Sender},
    SinkExt, StreamExt,
};
use http::Request;
use spin_sdk::http_wasip3::http_service;
use spin_sdk::http_wasip3::{send, EmptyBody, IntoResponse};

// In this streaming scenario, the entry point is a shim
// which kicks off the main async work of the application as
// a `spawn` and then immediately returns a Response. The response
// content will continue streaming from the "main application"
// function despite the entry point having returned.
#[http_service]
async fn handle_concurrent_outbound_http_calls(_req: spin_sdk::http_wasip3::Request) -> anyhow::Result<impl IntoResponse> {
    // Create a streaming Body implementation that backs onto a `mpsc`
    // channel. The function returns the sender side of the channel; the
    // receiver end becomes the body. So anything written to the sender
    // side will be sent out over the HTTP response.
    let (tx, body) = bytes_stream_body();

    // Spawn a task to run the application logic and stream the results
    // to the client. `spawn` continues to run this future even after the
    // function has exited with the return of the Response object.
    spin_sdk::http_wasip3::wasip3::wit_bindgen::spawn(
        handle_concurrent_outbound_http_calls_impl(tx)
    );

    Ok(http::Response::new(body))
}

// This is the real body of the application! Here `tx` is the
// sender through which we stream data to the client.
async fn handle_concurrent_outbound_http_calls_impl(mut tx: Sender<Bytes>) {
    // Start two async tasks to make concurrent outbound requests.
    let spin = pin!(get_content_length("https://spinframework.dev"));
    let book = pin!(get_content_length("https://component-model.bytecodealliance.org/"));

    // `select` completes when the first task completes.
    let first_completion = futures::future::select(spin, book).await;

    // Retrieve the result of whichever task completed, retaining the other
    // task for later use.
    let (first_result, second_fut) = first_completion.factor_first();

    // Write the outcome of that first task to the response.
    let first_message = first_result.unwrap().as_message("first");
    tx.send(Bytes::from(first_message)).await.unwrap();

    // Await the second task...
    let second_result = second_fut.await;

    // ...and write its result to the response too.
    let second_message = second_result.unwrap().as_message("second");
    tx.send(Bytes::from(second_message)).await.unwrap();

    // The `tx` sender drops at the end of the function, which ends the
    // response stream: if you need to close it explicitly in order to
    // continue doing work after completing the response, you can use `tx.close_channel()`.
}

struct TaskResult {
    url: String,
    time_taken: Duration,
    content_length: Option<usize>,
}

impl TaskResult {
    fn as_message(&self, position: &str) -> String {
        format!(
            "{} was {position} with a content-length of {:?} in {}ms\n",
            self.url,
            self.content_length,
            self.time_taken.as_millis()
        )
    }
}

async fn get_content_length(url: &str) -> anyhow::Result<TaskResult> {
    let request = Request::get(url).body(EmptyBody::new())?;
    let sent_at = Instant::now();
    let response = send(request).await?;
    let time_taken = Instant::now().duration_since(sent_at);
    let cl_header = response.headers().get("content-length");
    let content_length = cl_header
        .and_then(|hval| hval.to_str().ok())
        .and_then(|hval| hval.parse().ok());

    Ok(TaskResult {
        url: url.to_string(),
        time_taken,
        content_length,
    })
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
