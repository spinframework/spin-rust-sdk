use bytes::Bytes;
use spin_sdk::http::{body::IncomingBodyExt, EmptyBody, FullBody, IntoResponse, Request};
use spin_sdk::http_service;
use spin_sdk::key_value::Store;

#[http_service]
async fn handle_request(req: Request) -> anyhow::Result<impl IntoResponse> {
    // Open the default key-value store
    let store = Store::open_default().await?;

    let (status, body) = match *req.method() {
        http::Method::POST => {
            // Add the request (URI, body) tuple to the store
            let key = req.uri().path().to_string();
            let bytes = req.into_body().bytes().await?;
            store.set(key, bytes).await?;
            (http::StatusCode::OK, None)
        }
        http::Method::GET => {
            // Get the value associated with the request URI, or return a 404 if it's not present
            match store.get(req.uri().path()).await? {
                Some(value) => (http::StatusCode::OK, Some(value)),
                None => (http::StatusCode::NOT_FOUND, None),
            }
        }
        http::Method::DELETE => {
            // Delete the value associated with the request URI, if present
            store.delete(req.uri().path()).await?;
            (http::StatusCode::OK, None)
        }
        http::Method::HEAD => {
            // Like GET, except do not return the value
            let key = req.uri().path();
            let code = if store.exists(key).await? {
                http::StatusCode::OK
            } else {
                http::StatusCode::NOT_FOUND
            };
            (code, None)
        }
        // No other methods are currently supported
        _ => (http::StatusCode::METHOD_NOT_ALLOWED, None),
    };

    let resp = http::Response::builder().status(status);

    if let Some(body) = body.map(Bytes::from) {
        Ok(resp.body(FullBody::new(body)).into_response()?)
    } else {
        Ok(resp.body(EmptyBody::new()).into_response()?)
    }
}
