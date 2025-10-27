use spin_sdk::http_wasip3::{http_service, Request};

/// A simple Spin HTTP component.
#[http_service]
async fn hello_world(_req: Request) -> &'static str {
    "Hello, world!"
}
