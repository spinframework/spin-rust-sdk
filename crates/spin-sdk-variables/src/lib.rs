//! The Rust Spin Variables SDK.
//!
//! Component variables must be defined in the application
//! manifest, in the `[component.<name>.variables]` section.
//! Component variables typically use template syntax to
//! derive values from application variables, which are
//! the only variables that may be overridden directly (for
//! example, on the Spin command line).
//!
//! # Examples
//!
//! Get the value of a component variable.
//!
//! ```no_run
//! # async fn run() -> anyhow::Result<()> {
//! let region = spin_sdk_variables::get("region_id").await?;
//! let regional_url = format!("https://{region}.db.example.com");
//! # Ok(())
//! # }
//! ```
//!
//! Fail gracefully if a variable is not set.
//!
//! ```no_run
//! use spin_sdk_variables::Error;
//!
//! # async fn run() -> anyhow::Result<()> {
//! let favourite = match spin_sdk_variables::get("favourite").await {
//!     Ok(value) => value,
//!     Err(Error::Undefined(_)) => "not playing favourites".to_owned(),
//!     Err(e) => anyhow::bail!(e),
//! };
//! # Ok(())
//! # }
//! ```
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[doc(hidden)]
/// Module containing wit bindgen generated code.
///
/// This is only meant for internal consumption.
pub mod wit {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "spin-sdk-variables",
        path: "../../wit",
        generate_all,
    });

    pub use spin::variables::variables;
}

#[doc(inline)]
pub use wit::variables::Error;

/// Get an application variable value for the current component.
///
/// The name must match one defined in in the component manifest.
pub async fn get(key: impl AsRef<str>) -> Result<String, Error> {
    wit::variables::get(key.as_ref().to_string()).await
}
