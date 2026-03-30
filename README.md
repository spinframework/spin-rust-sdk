# The Spin Rust SDK

The Spin Rust SDK makes it easy to build Spin components in Rust.

## Spin documentation

This `README` file provides a few examples, such as writing Spin HTTP components in Rust and making outbound HTTP requests. For comprehensive information, visit the official [Spin Documentation website](https://spinframework.dev). This resource includes [a page on installing Spin](https://spinframework.dev/install#installing-spin), [a quickstart guide](https://spinframework.dev/quickstart), and [a language support overview page](https://spinframework.dev/language-support-overview). The latter lists all of Spin's features—including key-value storage, SQLite, MySQL, Redis, Serverless AI, etc.—and their implementation in specific languages such as Rust, TS/JS, Python, and TinyGo.

### Writing Spin HTTP Components in Rust

This library simplifies writing Spin HTTP components. Below is an example of
such a component:

```rust
// lib.rs
use spin_sdk::http::{IntoResponse, Request};
use spin_sdk::http_service;

/// A simple Spin HTTP component.
#[http_service]
async fn hello_world(_req: Request) -> impl IntoResponse {
    "Hello, Spin"
}
```

The important things to note about the function above are:

- the `spin_sdk::http_service` macro marks the function as the entry point for the Spin component,
- in the function signature, `req` can be the built-in `Request` type (a re-export of `http::Request`) or any type that implements `FromRequest`,
- the response type can be anything that implements `IntoResponse`, including `&str`, `String`, `http::StatusCode`, `http::Response<T>`, a tuple of `(StatusCode, body)`, or `Result<impl IntoResponse>`.

### Making Outbound HTTP Requests

Let's see an example where the component makes an outbound HTTP request to a server, modifies the result, and then returns it:

```rust
use spin_sdk::http::{send, EmptyBody, IntoResponse, Request, Response};
use spin_sdk::http_service;

#[http_service]
async fn handle_hello_world(_req: Request) -> anyhow::Result<impl IntoResponse> {
    // Create the outbound request object
    let outgoing = Request::get("https://random-data-api.fermyon.app/animals/json")
        .body(EmptyBody::new())
        .unwrap();

    // Send the request and await the response
    let resp: Response = send(outgoing).await?;

    Ok(resp)
}
```

For the component above to be allowed to make the outbound HTTP request, the destination host must be declared, using the `allowed_outbound_hosts` configuration, in the Spin application's manifest (the `spin.toml` file):

```toml
spin_manifest_version = 2

[application]
name = "hello_world"
version = "0.1.0"
authors = ["Your Name <your-name@example.com>"]
description = "An example application"

[[trigger.http]]
route = "/..."
component = "hello-world"

[component.hello-world]
source = "target/wasm32-wasip2/release/hello_world.wasm"
allowed_outbound_hosts = ["https://random-data-api.fermyon.app"]
[component.hello-world.build]
command = "cargo build --target wasm32-wasip2 --release"
watch = ["src/**/*.rs", "Cargo.toml"]
```

### Building and Running the Spin Application

Spin build can be used to build all components defined in the Spin manifest file at the same time, and also has a flag that starts the application after finishing the compilation, `spin build --up`:

```bash
$ spin build --up
Building component hello-world with `cargo build --target wasm32-wasip2 --release`
    Finished release [optimized] target(s) in 0.12s
Finished building all Spin components
Logging component stdio to ".spin/logs/"

Serving http://127.0.0.1:3000
Available Routes:
  hello-world: http://127.0.0.1:3000 (wildcard)
```

Once our application is running, we can make a request (by visiting `http://localhost:3000/` in a web browser) or using `curl` as shown below:

```bash
$ curl -i localhost:3000
HTTP/1.1 200 OK
content-length: 77
content-type: application/json

{"timestamp":1702599575198,"fact":"Sharks lay the biggest eggs in the world"}
```
