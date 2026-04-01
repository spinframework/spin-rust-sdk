use anyhow::Result;
use serde::Serialize;
use spin_sdk::http::{IntoResponse, Request};
use spin_sdk::http_service;
use spin_sdk::sqlite::{Connection, Value};

/// A simple user record for JSON serialization.
#[derive(Serialize)]
struct User {
    id: i64,
    name: String,
    email: String,
}

#[http_service]
async fn handle_request(req: Request) -> Result<impl IntoResponse> {
    let path = req.uri().path();
    let method = req.method().clone();

    match (method, path) {
        (http::Method::POST, "/init") => init_db().await,
        (http::Method::POST, "/users") => create_user(&req).await,
        (http::Method::GET, "/users") => list_users().await,
        _ => Ok(http::Response::builder()
            .status(http::StatusCode::NOT_FOUND)
            .body("Not Found".to_string())?),
    }
}

/// Create the users table if it doesn't exist.
async fn init_db() -> Result<http::Response<String>> {
    let db = Connection::open_default().await?;

    let query_result = db.execute(
        "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, email TEXT NOT NULL)",
        [],
    ).await?;

    // Await the completion future to ensure the statement finishes.
    query_result.result().await?;

    Ok(http::Response::builder()
        .status(http::StatusCode::OK)
        .body("Database initialized".to_string())?)
}

/// Insert a new user from query parameters `name` and `email`.
async fn create_user(req: &Request) -> Result<http::Response<String>> {
    let query = req.uri().query().unwrap_or("");
    let name = extract_param(query, "name").unwrap_or_default();
    let email = extract_param(query, "email").unwrap_or_default();

    if name.is_empty() || email.is_empty() {
        return Ok(http::Response::builder()
            .status(http::StatusCode::BAD_REQUEST)
            .body("Missing 'name' or 'email' query parameter".to_string())?);
    }

    let db = Connection::open_default().await?;

    let query_result = db
        .execute(
            "INSERT INTO users (name, email) VALUES (?, ?)",
            [Value::Text(name), Value::Text(email)],
        )
        .await?;

    query_result.result().await?;

    let rowid = db.last_insert_rowid().await;

    Ok(http::Response::builder()
        .status(http::StatusCode::CREATED)
        .body(format!("Created user with id {rowid}"))?)
}

/// List all users, streaming rows from the database.
async fn list_users() -> Result<http::Response<String>> {
    let db = Connection::open_default().await?;

    let mut query_result = db
        .execute("SELECT id, name, email FROM users ORDER BY id", [])
        .await?;

    // Resolve column indices once for efficient access.
    let id_idx = query_result
        .columns()
        .iter()
        .position(|c| c == "id")
        .unwrap();
    let name_idx = query_result
        .columns()
        .iter()
        .position(|c| c == "name")
        .unwrap();
    let email_idx = query_result
        .columns()
        .iter()
        .position(|c| c == "email")
        .unwrap();

    let mut users = Vec::new();

    // Consume the row stream.
    while let Some(row) = query_result.next().await {
        users.push(User {
            id: row.get::<i64>(id_idx).unwrap_or_default(),
            name: row.get::<&str>(name_idx).unwrap_or_default().to_owned(),
            email: row.get::<&str>(email_idx).unwrap_or_default().to_owned(),
        });
    }

    // Await completion to surface any errors.
    query_result.result().await?;

    let body = serde_json::to_string_pretty(&users)?;

    Ok(http::Response::builder()
        .status(http::StatusCode::OK)
        .header("content-type", "application/json")
        .body(body)?)
}

/// Extract a query parameter value by name.
fn extract_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == key {
            Some(urldecode(v))
        } else {
            None
        }
    })
}

/// Minimal percent-decoding for query parameter values.
fn urldecode(s: &str) -> String {
    let s = s.replace('+', " ");
    let mut result = Vec::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hi = bytes.next().and_then(|c| (c as char).to_digit(16));
            let lo = bytes.next().and_then(|c| (c as char).to_digit(16));
            if let (Some(h), Some(l)) = (hi, lo) {
                result.push((h * 16 + l) as u8);
            }
        } else {
            result.push(b);
        }
    }
    String::from_utf8_lossy(&result).into_owned()
}
