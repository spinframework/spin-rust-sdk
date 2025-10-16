//! The Rust Spin SDK.

#![deny(missing_docs)]

#[cfg(test)]
mod test;

/// Key/Value storage.
pub mod key_value;

/// SQLite storage for Spin 2 and earlier. Applications that do not require
/// this backward compatibility should use the [`sqlite3`] module instead.
pub mod sqlite;
/// SQLite storage.
pub mod sqlite3;

/// Large Language Model (Serverless AI) APIs
pub mod llm;

pub use spin_macro::*;

/// WASIp3 HTTP APIs and helpers
#[cfg(feature = "wasip3-unstable")]
pub mod http_wasip3 {
    /// Re-exports the helpers types for converting between WASIp3 HTTP types and
    /// Rust ecosystem HTTP types.
    pub use spin_wasip3_http::*;
    /// Re-exports the macro to enable WASIp3 HTTP handlers
    pub use spin_wasip3_http_macro::*;
}

#[doc(hidden)]
/// Module containing wit bindgen generated code.
///
/// This is only meant for internal consumption.
pub mod wit {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "platform",
        path: "./wit",
        with: {
            "wasi:io/error@0.2.0": ::wasi::io::error,
            "wasi:io/streams@0.2.0": ::wasi::io::streams,
            "wasi:io/poll@0.2.0": ::wasi::io::poll,
        },
        generate_all,
    });
    pub use fermyon::spin2_0_0 as v2;
    pub use spin::postgres3_0_0::postgres as pg3;
    pub use spin::postgres4_0_0::postgres as pg4;
    pub use spin::sqlite::sqlite as sqlite3;
}

#[export_name = concat!("spin-sdk-version-", env!("SDK_VERSION"))]
extern "C" fn __spin_sdk_version() {}

#[cfg(feature = "export-sdk-language")]
#[export_name = "spin-sdk-language-rust"]
extern "C" fn __spin_sdk_language() {}

#[export_name = concat!("spin-sdk-commit-", env!("SDK_COMMIT"))]
extern "C" fn __spin_sdk_hash() {}

pub mod http;

#[allow(missing_docs)]
pub mod mqtt;

#[allow(missing_docs)]
pub mod redis;

pub mod pg;
pub mod pg3;
pub mod pg4;

pub mod mysql;

pub mod variables;

#[doc(hidden)]
pub use wit_bindgen;
