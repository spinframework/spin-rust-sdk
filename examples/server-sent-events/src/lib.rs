use std::time::Duration;

use futures::channel::mpsc::Sender;
use spin_sdk::http::{body, IntoResponse, Request, Response};
use spin_sdk::http_service;

use futures::SinkExt;

const PINGS_PER_SECOND: u64 = 2;

#[http_service]
async fn handle_sse(request: Request) -> impl IntoResponse {
    let (mut tx, body) = body::stream();

    let mut response = Response::builder().status(200);

    if request.uri().path() == "/" {
        response = response.header("Content-Type", "text/html");

        spin_sdk::wasip3::spawn(async move {
            let html = include_str!("test.html");
            tx.send(html.to_owned()).await.unwrap();
        });
    } else if request.uri().path() == "/sse" {
        response = response
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache");

        spin_sdk::wasip3::spawn(run_sse_loop(tx));
    } else {
        response = response.status(404);
    }

    response.body(body)
}

async fn run_sse_loop(tx: Sender<String>) {
    // When the client disconnects, a `send` will fail, but that's
    // not an error. No other errors can happen in this implementation,
    // so just discard it.
    _ = run_sse_loop_impl(tx).await;
}

async fn run_sse_loop_impl(mut tx: Sender<String>) -> anyhow::Result<()> {
    let mut counter = rand::random_range(3..=10);
    let mut tick_count = 0;

    loop {
        tx.send("event: ping\n".to_owned()).await?;
        tx.send(format!(
            "data: Current time: {}\n\n",
            chrono::Utc::now().to_rfc3339()
        ))
        .await?;

        tick_count = (tick_count + 1) % PINGS_PER_SECOND;

        if tick_count == 0 {
            counter -= 1;

            if counter == 0 {
                counter = rand::random_range(3..=10);
                tx.send(format!("data: This is a surprise message! Next surprise message in {counter} seconds\n\n")).await?;
            }
        }

        spin_sdk::time::sleep(Duration::from_millis(1000 / PINGS_PER_SECOND)).await;
    }
}
