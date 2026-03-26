use spin_sdk::{http::Request, http_service};

/// A simple Spin HTTP component.
#[http_service]
async fn hello_world(_req: Request) -> &'static str {
    "Hello, world!"
}
