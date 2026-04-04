//! The Rust Spin SDK.
//!
//! This crate is the main entry point for building [Spin](https://spinframework.dev)
//! components in Rust. Each capability is exposed as a feature-gated
//! module. All features are enabled by default; disable `default-features`
//! and pick only what you need to slim down compile times.
//!
//! # Modules
//!
//! | Module | Feature | Purpose |
//! |--------|---------|---------|
//! | [`http`] | `http` | Incoming and outgoing HTTP requests |
//! | [`key_value`] | `key-value` | Persistent key-value storage |
//! | [`llm`] | `llm` | Large-language-model inference |
//! | [`mqtt`] | `mqtt` | MQTT message publishing |
//! | [`mysql`] | `mysql` | MySQL database access |
//! | [`pg`] | `pg` | PostgreSQL database access |
//! | [`redis`] | `redis` | Redis storage and pub/sub |
//! | [`sqlite`] | `sqlite` | SQLite database access |
//! | [`variables`] | `variables` | Application variable lookup |
//!
//! The [`http_service`] and [`redis_subscriber`] attribute macros
//! generate the boilerplate required to expose a component to the
//! Spin runtime.

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

/// Re-export entrypoint macros
pub use spin_sdk_macro::{dependencies, http_service, redis_subscriber};

/// Incoming and outgoing HTTP requests.
#[cfg(feature = "http")]
#[cfg_attr(docsrs, doc(cfg(feature = "http")))]
pub mod http;

/// Persistent key-value storage.
#[cfg(feature = "key-value")]
#[cfg_attr(docsrs, doc(cfg(feature = "key-value")))]
pub mod key_value;

/// Large-language-model inference.
#[cfg(feature = "llm")]
#[cfg_attr(docsrs, doc(cfg(feature = "llm")))]
pub mod llm;

/// MQTT message publishing.
#[cfg(feature = "mqtt")]
#[cfg_attr(docsrs, doc(cfg(feature = "mqtt")))]
pub mod mqtt;

/// MySQL database access.
#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
pub mod mysql;

/// PostgreSQL database access.
#[cfg(feature = "pg")]
#[cfg_attr(docsrs, doc(cfg(feature = "pg")))]
pub mod pg;

/// Redis storage and pub/sub.
#[cfg(feature = "redis")]
#[cfg_attr(docsrs, doc(cfg(feature = "redis")))]
pub mod redis;

/// SQLite database access.
#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
pub mod sqlite;

/// Application variable lookup.
#[cfg(feature = "variables")]
#[cfg_attr(docsrs, doc(cfg(feature = "variables")))]
pub mod variables;

#[export_name = concat!("spin-sdk-version-", env!("SDK_VERSION"))]
extern "C" fn __spin_sdk_version() {}

#[cfg(feature = "export-sdk-language")]
#[export_name = "spin-sdk-language-rust"]
extern "C" fn __spin_sdk_language() {}

#[export_name = concat!("spin-sdk-commit-", env!("SDK_COMMIT"))]
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
