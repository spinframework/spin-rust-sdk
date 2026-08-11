//! The Rust Spin SDK.
//!
//! This crate is the main entry point for building [Spin](https://spinframework.dev)
//! components in Rust. Each capability is exposed as a feature-gated module, so
//! a component only pulls in the parts of the SDK it actually uses.
//!
//! The [`http_service`] and [`redis_subscriber`] attribute macros generate the
//! boilerplate required to expose a component to the Spin runtime, while the
//! remaining modules provide access to host capabilities such as
//! [key-value storage](key_value), [SQL databases](sqlite),
//! [outbound HTTP](http), and [LLM inference](llm).
//!
//! # Examples
//!
//! A minimal HTTP component looks like this:
//!
//! ```ignore
//! use spin_sdk::http::{IntoResponse, Request};
//! use spin_sdk::http_service;
//!
//! #[http_service]
//! async fn handle(req: Request) -> impl IntoResponse {
//!     "Hello, world!"
//! }
//! ```
//!
//! Complete, runnable examples for every capability live in the
//! [`examples`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples)
//! directory of the repository, including:
//!
//! - HTTP:
//!   [`hello-world`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/hello-world),
//!   [`http-hello-world`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/http-hello-world),
//!   [`http-axum-router`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/http-axum-router),
//!   [`http-send-request`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/http-send-request),
//!   [`http-concurrent-outbound-calls`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/http-concurrent-outbound-calls),
//!   [`http-outbound`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/http-outbound),
//!   [`http-streaming`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/http-streaming),
//!   [`server-sent-events`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/server-sent-events)
//! - gRPC:
//!   [`grpc`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/grpc),
//!   [`grpc-streaming`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/grpc-streaming)
//! - Storage and databases:
//!   [`key-value`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/key-value),
//!   [`sqlite`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/sqlite),
//!   [`mysql`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/mysql),
//!   [`postgres`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/postgres),
//!   [`redis`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/redis)
//! - Messaging:
//!   [`mqtt-outbound`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/mqtt-outbound),
//!   [`redis-outbound`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/redis-outbound)
//! - Configuration:
//!   [`variables`](https://github.com/spinframework/spin-rust-sdk/tree/main/examples/variables)
//!
//! # Feature Flags
//!
//! Capabilities are gated behind Cargo features so you can compile only what
//! you need. All of the features below are enabled by default; to opt out, set
//! `default-features = false` in your `Cargo.toml` and re-enable just the ones
//! you want.
//!
//! - `http`: Enables the [`http`] module for handling inbound requests and
//!   making outbound HTTP calls. Enabled by default.
//! - `http-middleware`: Enables `http::next` for forwarding a request along a
//!   middleware chain. Implies `http`. Not enabled by default.
//! - `grpc`: Enables the `http::grpc` helpers for serving
//!   [tonic](https://docs.rs/tonic) gRPC services from a Spin HTTP component.
//!   Implies `http`. Not enabled by default.
//! - `key-value`: Enables the [`key_value`] module for persistent key-value
//!   storage. Enabled by default.
//! - `json`: Enables JSON support built on [serde](https://docs.rs/serde),
//!   including the `http::Json` extractor and JSON helpers on the key-value
//!   store. Enabled by default.
//! - `llm`: Enables the [`llm`] module for large-language-model inference and
//!   embeddings. Enabled by default.
//! - `mqtt`: Enables the [`mqtt`] module for publishing MQTT messages. Enabled
//!   by default.
//! - `mysql`: Enables the [`mysql`] module for MySQL database access. Enabled
//!   by default.
//! - `pg`: Enables the [`pg`] module for PostgreSQL database access. Enabled by
//!   default.
//! - `postgres4-types`: Adds support for extended PostgreSQL types — decimals,
//!   UUIDs, ranges, and JSON — to the [`pg`] module. Implies `pg` and `json`.
//!   Enabled by default.
//! - `redis`: Enables the [`redis`] module for Redis storage and message
//!   publishing. Enabled by default.
//! - `sqlite`: Enables the [`sqlite`] module for SQLite database access.
//!   Enabled by default.
//! - `variables`: Enables the [`variables`] module for looking up application
//!   variables. Enabled by default.
//! - `export-sdk-language`: Exports a marker symbol identifying the component
//!   as built with the Rust SDK. Enabled by default.

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Re-export entrypoint macros
pub use spin_macro::{dependencies, http_service, redis_subscriber};

// Incoming and outgoing HTTP requests.
#[cfg(feature = "http")]
#[cfg_attr(docsrs, doc(cfg(feature = "http")))]
pub mod http;

// Persistent key-value storage.
#[cfg(feature = "key-value")]
#[cfg_attr(docsrs, doc(cfg(feature = "key-value")))]
pub mod key_value;

// Large-language-model inference.
#[cfg(feature = "llm")]
#[cfg_attr(docsrs, doc(cfg(feature = "llm")))]
pub mod llm;

// MQTT message publishing.
#[cfg(feature = "mqtt")]
#[cfg_attr(docsrs, doc(cfg(feature = "mqtt")))]
pub mod mqtt;

// MySQL database access.
#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
pub mod mysql;

// PostgreSQL database access.
#[cfg(feature = "pg")]
#[cfg_attr(docsrs, doc(cfg(feature = "pg")))]
pub mod pg;

// Redis storage and pub/sub.
#[cfg(feature = "redis")]
#[cfg_attr(docsrs, doc(cfg(feature = "redis")))]
pub mod redis;

// SQLite database access.
#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
pub mod sqlite;

// Time-related functions.
pub mod time;

// Application variable lookup.
#[cfg(feature = "variables")]
#[cfg_attr(docsrs, doc(cfg(feature = "variables")))]
pub mod variables;

// SAFETY: There should only be a single definition of the spin-sdk-version-* symbol.
#[unsafe(export_name = concat!("spin-sdk-version-", env!("SDK_VERSION")))]
extern "C" fn __spin_sdk_version() {}

// SAFETY: There should only be a single definition of the spin-sdk-language-rust symbol.
#[cfg(feature = "export-sdk-language")]
#[unsafe(export_name = "spin-sdk-language-rust")]
extern "C" fn __spin_sdk_language() {}

// SAFETY: There should only be a single definition of the spin-sdk-commit-* symbol.
#[unsafe(export_name = concat!("spin-sdk-commit-", env!("SDK_COMMIT")))]
extern "C" fn __spin_sdk_hash() {}

pub use wasip3::{self, wit_bindgen};

#[doc(hidden)]
pub mod experimental {
    #![allow(missing_docs)]
    use crate::wit_bindgen;

    wit_bindgen::generate!({
        runtime_path: "crate::wit_bindgen::rt",
        world: "spin-sdk-experimental",
        path: "wit",
        generate_all,
    });
}

#[cfg(test)]
mod test;
