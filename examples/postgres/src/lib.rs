#![allow(dead_code)]
use anyhow::Result;
use futures::SinkExt;
use spin_sdk::http::{body, IntoResponse, Request};
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

    let (mut tx, body) = body::stream();

    spin_sdk::wit_bindgen::spawn(async move {
        let mut had_ruler = false;

        while let Some(ruler) = rulers.next().await {
            if had_ruler {
                tx.send(" and ".to_owned()).await.unwrap();
            }
            had_ruler = true;
            let name = ruler.get::<String>("name").unwrap();
            tx.send(name).await.unwrap();
        }

        if let Err(e) = rulers.result().await {
            eprintln!("query error: {e}");
        }
    });

    Ok(http::Response::builder().body(body)?)
}
