use spin_sdk::http_wasip3::{http_component, Request};

/// A simple Spin HTTP component.
#[http_component]
async fn hello_world(_req: Request) -> &'static str {
    "Hello, world!"
}
