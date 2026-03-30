//! The Rust Spin SDK.
//!
//! This crate is the main entry point for building [Spin](https://spinframework.dev)
//! components in Rust. It re-exports the individual SDK crates under
//! a unified namespace so that most applications only need to depend
//! on `spin-sdk`.
//!
//! # SDK layout
//!
//! The SDK is split into focused crates, each covering a single Spin
//! capability. They can be used standalone or through the re-exports
//! below.
//!
//! | Re-export | Crate | Purpose |
//! |-----------|-------|---------|
//! | [`http`] | `spin-sdk-http` | Incoming and outgoing HTTP requests |
//! | [`key_value`] | `spin-sdk-kv` | Persistent key-value storage |
//! | [`llm`] | `spin-sdk-llm` | Large-language-model inference |
//! | [`mqtt`] | `spin-sdk-mqtt` | MQTT message publishing |
//! | [`mysql`] | `spin-sdk-mysql` | MySQL database access |
//! | [`pg`] | `spin-sdk-pg` | PostgreSQL database access |
//! | [`redis`] | `spin-sdk-redis` | Redis storage and pub/sub |
//! | [`sqlite`] | `spin-sdk-sqlite` | SQLite database access |
//! | [`variables`] | `spin-sdk-variables` | Application variable lookup |
//!
//! The [`http_service`] and [`redis_subscriber`] attribute macros
//! (from `spin-sdk-macro`) generate the boilerplate required to
//! expose a component to the Spin runtime.

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// #[cfg(test)]
// mod test;

/// Re-export Spin HTTP SDK
pub use spin_sdk_http as http;
/// Re-export SPIN KeyValue SDK
pub use spin_sdk_kv as key_value;
/// Re-export Spin LLM SDK
pub use spin_sdk_llm as llm;
/// Re-export entrypoint macros
pub use spin_sdk_macro::{http_service, redis_subscriber};
/// Re-export Spin Mqtt SDK
pub use spin_sdk_mqtt as mqtt;
/// Re-export Spin MySQL SDK
pub use spin_sdk_mysql as mysql;
/// Re-export Spin Postgres SDK
pub use spin_sdk_pg as pg;
/// Re-export Spin Redis SDK
pub use spin_sdk_redis as redis;
/// Re-export Spin SQLite SDK
pub use spin_sdk_sqlite as sqlite;
/// Re-export Spin Variables SDK
pub use spin_sdk_variables as variables;

#[export_name = concat!("spin-sdk-version-", env!("SDK_VERSION"))]
extern "C" fn __spin_sdk_version() {}

#[cfg(feature = "export-sdk-language")]
#[export_name = "spin-sdk-language-rust"]
extern "C" fn __spin_sdk_language() {}

#[export_name = concat!("spin-sdk-commit-", env!("SDK_COMMIT"))]
extern "C" fn __spin_sdk_hash() {}

#[doc(hidden)]
pub use wit_bindgen;

#[doc(hidden)]
/// Various WASI APIs
pub mod experimental {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "spin-sdk-wasi",
        path: "../../wit",
        generate_all,
    });
}
