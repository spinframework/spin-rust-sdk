use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use spin_sdk::http_wasip3::{http_service, IntoResponse, Request};
use tower_service::Service;

/// Demonstrates integration with the Axum web framework
#[http_service]
async fn handler(req: Request) -> impl IntoResponse {
    Router::new()
        .route("/", get(root))
        .route("/users", post(create_user))
        .call(req)
        .await
}

async fn root() -> &'static str {
    "hello, world!"
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    // insert your application logic here
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}
