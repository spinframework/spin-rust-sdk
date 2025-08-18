#![allow(dead_code)]
use anyhow::Result;
use http::{Request, Response};
use spin_sdk::{http_component, pg3, pg3::Decode};

// The environment variable set in `spin.toml` that points to the
// address of the Pg server that the component will write to
const DB_URL_ENV: &str = "DB_URL";

#[http_component]
fn process(req: Request<()>) -> Result<Response<String>> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = pg3::Connection::open(&address)?;

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
            .rows
            .into_iter()
            .map(|r| Decode::decode(&r[0]))
            .collect::<Result<Vec<String>, _>>()?;
        ruler_names.join(" and ")
    };

    Ok(http::Response::builder().body(format!("{response}\n"))?)
}
