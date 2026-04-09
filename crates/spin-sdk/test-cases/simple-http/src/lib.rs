use spin_sdk::{
    http::{IntoResponse, Request},
    http_service,
};

#[http_service]
async fn hello_world(_req: Request) -> impl IntoResponse {
    "Hello, world!"
}
