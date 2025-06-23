//! The Rust Spin SDK.

#![deny(missing_docs)]

#[cfg(test)]
mod test;

/// Key/Value storage.
pub mod key_value;

/// SQLite storage.
pub mod sqlite;

/// Large Language Model (Serverless AI) APIs
pub mod llm;

pub use spin_macro::*;

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
        }
    });
    pub use fermyon::spin2_0_0 as v2;
    pub use spin::postgres::postgres as pg3;
}

/// Needed by the export macro
///
/// See [this commit](https://github.com/bytecodealliance/wit-bindgen/pull/394/commits/9d2ea88f986f4a883ba243449e3a070cac18958e) for more info.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub use wit::__link_section;

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

pub mod mysql;

pub mod variables;

#[doc(hidden)]
pub use wit_bindgen;
