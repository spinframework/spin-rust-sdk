#![allow(dead_code)]
use anyhow::Result;
use spin_sdk::http::Request;
use spin_sdk::{http_service, pg};

// The environment variable set in `spin.toml` that points to the
// address of the Pg server that the component will write to
const DB_URL_ENV: &str = "DB_URL";

#[http_service]
async fn process(req: Request) -> Result<http::Response<String>> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = pg::Connection::open_async(address).await?;

    let year_header = req
        .headers()
        .get("spin-path-match-year")
        .map(|hv| hv.to_str())
        .transpose()?
        .unwrap_or("2025");
    let year: i32 = year_header.parse()?;

    // Due to an ambiguity in the PostgreSQL `<@` operator syntax, we MUST qualify
    // the year as an int4 rather than an int4range in the query.
    let rulers = conn.query(
        "SELECT name FROM cats WHERE $1::int4 <@ reign",
        &[year.into()],
    )?;

    let response = if rulers.rows.is_empty() {
        "it was anarchy".to_owned()
    } else {
        let ruler_names = rulers
            .rows()
            .map(|r| r.get::<String>("name").unwrap())
            .collect::<Vec<_>>();
        ruler_names.join(" and ")
    };

    Ok(http::Response::builder().body(format!("{response}\n"))?)
}
