use spin_sdk::http_wasip3::{http_component, IncomingRequest};

/// A simple Spin HTTP component.
#[http_component]
async fn hello_world(_req: IncomingRequest) -> &'static str {
    "Hello, world!"
}
