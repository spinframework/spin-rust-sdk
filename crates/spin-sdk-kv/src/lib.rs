//! The Rust Spin KeyValue SDK.
//!
//! Spin key-value persistent storage
//!
//! This module provides a generic interface for key-value storage, which may be implemented by the host various
//! ways (e.g. via an in-memory table, a local file, or a remote database). Details such as consistency model and
//! durability will depend on the implementation and may vary from one to store to the next.
//!
//! # Examples
//!
//! Open the default store and set the 'message' key:
//!
//! ```no_run
//! # async fn run() -> anyhow::Result<()> {
//! let store = spin_sdk_kv::Store::open_default().await?;
//! store.set("message", "Hello world".as_bytes()).await?;
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
        world: "spin-sdk-kv",
        path: "../../wit",
        generate_all,
    });

    pub use spin::key_value::key_value;
}

#[cfg(feature = "json")]
use serde::{de::DeserializeOwned, Serialize};

#[doc(inline)]
pub use wit::key_value::Error;

/// An open key-value store.
///
/// # Examples
///
/// Open the default store and set the 'message' key:
///
/// ```no_run
/// # async fn run() -> anyhow::Result<()> {
/// let store = spin_sdk_kv::Store::open_default().await?;
/// store.set("message", "Hello world".as_bytes()).await?;
/// # Ok(())
/// # }
/// ```
///
/// Open the default store and get the 'message' key:
///
/// ```no_run
/// # async fn run() -> anyhow::Result<()> {
/// let store = spin_sdk_kv::Store::open_default().await?;
/// let message = store.get("message").await?;
/// let response = message.unwrap_or_else(|| "not found".into());
/// # Ok(())
/// # }
/// ```
///
/// Open a named store and list all the keys defined in it:
///
/// ```no_run
/// # async fn run() -> anyhow::Result<()> {
/// let store = spin_sdk_kv::Store::open("finance").await?;
/// let (keys, result) = store.get_keys().await?;
/// # Ok(())
/// # }
/// ```
///
/// Open the default store and delete the 'message' key:
///
/// ```no_run
/// # async fn run() -> anyhow::Result<()> {
/// let store = spin_sdk_kv::Store::open_default().await?;
/// store.delete("message").await?;
/// # Ok(())
/// # }
/// ```
pub struct Store(wit::key_value::Store);

impl Store {
    /// Open the default store.
    ///
    /// This is equivalent to `Store::open("default").await`.
    pub async fn open_default() -> Result<Self, Error> {
        wit::key_value::Store::open("default".into())
            .await
            .map(Store)
    }
}

impl Store {
    /// Open the store with the specified label.
    ///
    /// `label` must refer to a store allowed in the spin.toml manifest.
    ///
    /// `error::no-such-store` will be raised if the `label` is not recognized.
    pub async fn open(label: impl AsRef<str>) -> Result<Self, Error> {
        wit::key_value::Store::open(label.as_ref().to_string())
            .await
            .map(Store)
    }

    /// Get the value associated with the specified `key`
    ///
    /// Returns `ok(none)` if the key does not exist.
    pub async fn get(&self, key: impl AsRef<str>) -> Result<Option<Vec<u8>>, Error> {
        self.0.get(key.as_ref().to_string()).await
    }

    /// Set the `value` associated with the specified `key` overwriting any existing value.
    pub async fn set(&self, key: impl AsRef<str>, value: impl AsRef<[u8]>) -> Result<(), Error> {
        self.0
            .set(key.as_ref().to_string(), value.as_ref().to_vec())
            .await
    }

    /// Delete the tuple with the specified `key`
    ///
    /// No error is raised if a tuple did not previously exist for `key`.
    pub async fn delete(&self, key: impl AsRef<str>) -> Result<(), Error> {
        self.0.delete(key.as_ref().to_string()).await
    }

    /// Return whether a tuple exists for the specified `key`
    pub async fn exists(&self, key: impl AsRef<str>) -> Result<bool, Error> {
        self.0.exists(key.as_ref().to_string()).await
    }

    /// Return a list of all the keys
    pub async fn get_keys(
        &self,
    ) -> (
        wit_bindgen::StreamReader<String>,
        wit_bindgen::FutureReader<Result<(), Error>>,
    ) {
        self.0.get_keys().await
    }

    #[cfg(feature = "json")]
    /// Serialize the given data to JSON, then set it as the value for the specified `key`.
    ///
    /// # Examples
    ///
    /// Open the default store and save a customer information document against the customer ID:
    ///
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// #[derive(Deserialize, Serialize)]
    /// struct Customer {
    ///     name: String,
    ///     address: Vec<String>,
    /// }
    ///
    /// # async fn run() -> anyhow::Result<()> {
    /// let customer_id = "CR1234567";
    /// let customer = Customer {
    ///     name: "Alice".to_owned(),
    ///     address: vec!["Wonderland Way".to_owned()],
    /// };
    ///
    /// let store = spin_sdk_kv::Store::open_default().await?;
    /// store.set_json(customer_id, &customer).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_json<T: Serialize>(
        &self,
        key: impl AsRef<str>,
        value: &T,
    ) -> Result<(), anyhow::Error> {
        Ok(self
            .0
            .set(key.as_ref().to_string(), serde_json::to_vec(value)?)
            .await?)
    }

    #[cfg(feature = "json")]
    /// Deserialize an instance of type `T` from the value of `key`.
    ///
    /// # Examples
    ///
    /// Open the default store and retrieve a customer information document by customer ID:
    ///
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// #[derive(Deserialize, Serialize)]
    /// struct Customer {
    ///     name: String,
    ///     address: Vec<String>,
    /// }
    ///
    /// # async fn run() -> anyhow::Result<()> {
    /// let customer_id = "CR1234567";
    ///
    /// let store = spin_sdk_kv::Store::open_default().await?;
    /// let customer = store.get_json::<Customer>(customer_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        key: impl AsRef<str>,
    ) -> Result<Option<T>, anyhow::Error> {
        let Some(value) = self.0.get(key.as_ref().to_string()).await? else {
            return Ok(None);
        };
        Ok(serde_json::from_slice(&value)?)
    }
}
