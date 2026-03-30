#![allow(dead_code)]
use anyhow::Result;
use bytes::Bytes;
use futures::channel::mpsc::{channel, Sender};
use futures::{SinkExt, StreamExt};
use spin_sdk::http::{IntoResponse, Request};
use spin_sdk::{http_service, pg};

// The environment variable set in `spin.toml` that points to the
// address of the Pg server that the component will write to
const DB_URL_ENV: &str = "DB_URL";

#[http_service]
async fn process(req: Request) -> Result<impl IntoResponse> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = pg::Connection::open(address).await?;

    let year_header = req
        .headers()
        .get("spin-path-match-year")
        .map(|hv| hv.to_str())
        .transpose()?
        .unwrap_or("2025");
    let year: i32 = year_header.parse()?;

    // Due to an ambiguity in the PostgreSQL `<@` operator syntax, we MUST qualify
    // the year as an int4 rather than an int4range in the query.
    let mut rulers = conn
        .query(
            "SELECT name FROM cats WHERE $1::int4 <@ reign",
            &[year.into()],
        )
        .await?;

    let (mut tx, body) = bytes_stream_body();

    spin_sdk::wit_bindgen::spawn(async move {
        let mut had_ruler = false;

        while let Some(ruler) = rulers.next().await {
            if had_ruler {
                tx.send(Bytes::from_static(b" and ")).await.unwrap();
            }
            had_ruler = true;
            let name = ruler.get::<String>("name").unwrap();
            tx.send(Bytes::from_owner(name.into_bytes())).await.unwrap();
        }

        if let Err(e) = rulers.result().await {
            eprintln!("query error: {e}");
        }
    });

    Ok(http::Response::builder().body(body)?)
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
