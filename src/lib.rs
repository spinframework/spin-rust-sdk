//! The Rust Spin SDK.

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

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

pub use spin_dep_macro::*;
pub use spin_macro::*;

/// WASIp3 HTTP APIs and helpers.
///
/// **The contents of this module are unstable.** Module APIs may change in future releases,
/// and may not work with future versions of Spin (as they bind to a particular WASI RC
/// which Spin will retire once a stable WASIp3 is available)/
#[cfg(feature = "wasip3-unstable")]
#[cfg_attr(docsrs, doc(cfg(feature = "wasip3-unstable")))]
pub mod http_wasip3 {
    pub use spin_wasip3_http::*;

    /// Marks an `async fn` as an HTTP component entrypoint for Spin.
    ///
    /// The `#[http_service]` attribute designates an asynchronous function as the
    /// handler for incoming HTTP requests in a Spin component using the WASI Preview 3
    /// (`wasip3`) HTTP ABI.  
    ///
    /// When applied, this macro generates the necessary boilerplate to export the
    /// function to the Spin runtime as a valid HTTP handler. The function must be
    /// declared `async` and take a single argument implementing
    /// [`FromRequest`], typically
    /// [`Request`], and must return a type that
    /// implements [`IntoResponse`].
    ///
    /// # Requirements
    ///
    /// - The annotated function **must** be `async`.
    /// - The function’s parameter type must implement [`FromRequest`].
    /// - The return type must implement [`IntoResponse`].
    /// - The Spin manifest must specify `executor = { type = "wasip3-unstable" }`
    ///
    /// If the function is not asynchronous, the macro emits a compile-time error.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use spin_sdk::http_wasip3::{http_service, Request, IntoResponse};
    ///
    /// #[http_service]
    /// async fn my_handler(request: Request) -> impl IntoResponse {
    ///   // Your logic goes here
    /// }
    /// ```
    ///
    /// # Generated Code
    ///
    /// The macro expands into a module containing a `Spin` struct that implements the
    /// WASI `http.handler/Guest` interface, wiring the annotated function as the
    /// handler’s entrypoint. This allows the function to be invoked automatically
    /// by the Spin runtime when HTTP requests are received.
    pub use spin_wasip3_http_macro::http_service;
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
