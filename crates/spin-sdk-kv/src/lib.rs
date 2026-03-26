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
//! # fn main() -> anyhow::Result<()> {
//! let store = spin_sdk::key_value::Store::open_default()?;
//! store.set("message", "Hello world".as_bytes())?;
//! # Ok(())
//! # }
//! ```
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

use wit::spin::key_value::key_value;

#[cfg(feature = "json")]
use serde::{de::DeserializeOwned, Serialize};

#[doc(inline)]
pub use key_value::Error;

/// An open key-value store.
///
/// # Examples
///
/// Open the default store and set the 'message' key:
///
/// ```no_run
/// # fn main() -> anyhow::Result<()> {
/// let store = spin_sdk::key_value::Store::open_default()?;
/// store.set("message", "Hello world".as_bytes())?;
/// # Ok(())
/// # }
/// ```
///
/// Open the default store and get the 'message' key:
///
/// ```no_run
/// # fn main() -> anyhow::Result<()> {
/// let store = spin_sdk::key_value::Store::open_default()?;
/// let message = store.get("message")?;
/// let response = message.unwrap_or_else(|| "not found".into());
/// # Ok(())
/// # }
/// ```
///
/// Open a named store and list all the keys defined in it:
///
/// ```no_run
/// # fn main() -> anyhow::Result<()> {
/// let store = spin_sdk::key_value::Store::open("finance")?;
/// let keys = store.get_keys()?;
/// # Ok(())
/// # }
/// ```
///
/// Open the default store and delete the 'message' key:
///
/// ```no_run
/// # async fn main() -> anyhow::Result<()> {
/// let store = spin_sdk::key_value::Store::open_default(),await?;
/// store.delete("message")?;
/// # Ok(())
/// # }
/// ```
#[doc(inline)]
pub use key_value::Store;

impl Store {
    /// Open the default store.
    ///
    /// This is equivalent to `Store::open("default").await`.
    pub async fn open_default() -> Result<Self, Error> {
        Self::open("default".into()).await
    }
}

impl Store {
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
    /// # fn main() -> anyhow::Result<()> {
    /// let customer_id = "CR1234567";
    /// let customer = Customer {
    ///     name: "Alice".to_owned(),
    ///     address: vec!["Wonderland Way".to_owned()],
    /// };
    ///
    /// let store = spin_sdk::key_value::Store::open_default()?;
    /// store.set_json(customer_id, &customer)?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_json<T: Serialize>(
        &self,
        key: impl AsRef<str>,
        value: &T,
    ) -> Result<(), anyhow::Error> {
        Ok(self
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
    /// # fn main() -> anyhow::Result<()> {
    /// let customer_id = "CR1234567";
    ///
    /// let store = spin_sdk::key_value::Store::open_default()?;
    /// let customer = store.get_json::<Customer>(customer_id)?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        key: impl AsRef<str>,
    ) -> Result<Option<T>, anyhow::Error> {
        let Some(value) = self.get(key.as_ref().to_string()).await? else {
            return Ok(None);
        };
        Ok(serde_json::from_slice(&value)?)
    }
}

#[doc(hidden)]
/// Module containing wit bindgen generated code.
///
/// This is only meant for internal consumption.
pub mod wit {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "imports",
        path: "./wit",
        generate_all,
    });
}
