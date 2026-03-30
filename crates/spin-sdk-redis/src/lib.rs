//! The Spin Redis SDK.
//!
//! Redis storage and message publishing.
//!
//! To receive Redis messages, use the Redis trigger.
//!
//! # Examples
//!
//! Get a value from the Redis database.
//!
//! ```no_run
//! use spin_sdk::redis::Connection;
//!
//! # async fn main() -> anyhow::Result<()> {
//! let conn = Connection::open("redis://127.0.0.1:6379").await?;
//! let payload = conn.get("archimedes-data").await?;
//! if let Some(data) = payload {
//!     println!("{}", String::from_utf8_lossy(&data));
//! }
//! # Ok(())
//! # }
//! ```
//!
//! See the [`Connection`] type for further examples.
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[doc(hidden)]
/// Module containing wit bindgen generated code.
///
/// This is only meant for internal consumption.
pub mod wit {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "spin-sdk-redis",
        path: "../../wit",
        generate_all,
    });

    pub use spin::redis::redis;
}

use std::hash::{Hash, Hasher};

/// An open connection to a Redis server.
///
/// # Examples
///
/// Get a value from the Redis database.
///
/// ```no_run
/// use spin_sdk::redis::Connection;
///
/// # async fn main() -> anyhow::Result<()> {
/// let conn = Connection::open("redis://127.0.0.1:6379").await?;
/// let payload = conn.get("archimedes-data").await?;
/// if let Some(data) = payload {
///     println!("{}", String::from_utf8_lossy(&data));
/// }
/// # Ok(())
/// # }
/// ```
///
/// Set a value in the Redis database.
///
/// ```no_run
/// use spin_sdk::redis::Connection;
///
/// # async fn main() -> anyhow::Result<()> {
/// let conn = Connection::open("redis://127.0.0.1:6379").await?;
/// let payload = "Eureka!".to_owned().into_bytes();
/// conn.set("archimedes-data", &payload).await?;
/// # Ok(())
/// # }
/// ```
///
/// Delete a value from the Redis database.
///
/// ```no_run
/// use spin_sdk::redis::Connection;
///
/// # async fn main() -> anyhow::Result<()> {
/// let conn = Connection::open("redis://127.0.0.1:6379").await?;
/// conn.del(&["archimedes-data".to_owned()]).await?;
/// # Ok(())
/// # }
/// ```
///
/// Publish a message to a Redis channel.
///
/// ```no_run
/// use spin_sdk::redis::Connection;
///
/// # fn ensure_pet_picture(_: &[u8]) -> anyhow::Result<()> { Ok(()) }
/// # async fn use_redis(request: spin_sdk::http::Request) -> anyhow::Result<()> {
/// let conn = Connection::open("redis://127.0.0.1:6379").await?;
///
/// let payload = request.body().to_vec();
/// ensure_pet_picture(&payload)?;
///
/// conn.publish("pet-pictures", &payload).await?;
/// # Ok(())
/// # }
/// ```
pub struct Connection(wit::redis::Connection);

pub use wit::redis::{Error, Payload, RedisParameter, RedisResult};

impl Connection {
    /// Open a connection to the Redis instance at `address`.
    pub async fn open(address: impl AsRef<str>) -> Result<Self, Error> {
        wit::redis::Connection::open(address.as_ref().to_string())
            .await
            .map(Connection)
    }

    /// Publish a Redis message to the specified channel.
    // publish: async func(channel: string, payload: payload) -> result<_, error>;
    pub async fn publish(
        &self,
        channel: impl AsRef<str>,
        payload: impl AsRef<[u8]>,
    ) -> Result<(), Error> {
        self.0
            .publish(channel.as_ref().to_string(), payload.as_ref().to_vec())
            .await
    }

    /// Get the value of a key.
    pub async fn get(&self, key: impl AsRef<str>) -> Result<Option<Payload>, Error> {
        self.0.get(key.as_ref().to_string()).await
    }

    /// Set key to value.
    ///
    /// If key already holds a value, it is overwritten.
    pub async fn set(&self, key: impl AsRef<str>, value: impl AsRef<[u8]>) -> Result<(), Error> {
        self.0
            .set(key.as_ref().to_string(), value.as_ref().to_vec())
            .await
    }

    /// Increments the number stored at key by one.
    ///
    /// If the key does not exist, it is set to 0 before performing the operation.
    /// An `error::type-error` is returned if the key contains a value of the wrong type
    /// or contains a string that can not be represented as integer.
    pub async fn incr(&self, key: impl AsRef<str>) -> Result<i64, Error> {
        self.0.incr(key.as_ref().to_string()).await
    }

    /// Removes the specified keys.
    ///
    /// A key is ignored if it does not exist. Returns the number of keys deleted.
    // del: async func(keys: list<string>) -> result<u32, error>;
    pub async fn del(&self, keys: impl IntoIterator<Item = String>) -> Result<u32, Error> {
        self.0.del(keys.into_iter().collect()).await
    }

    /// Add the specified `values` to the set named `key`, returning the number of newly-added values.
    // sadd: async func(key: string, values: list<string>) -> result<u32, error>;
    pub async fn sadd(
        &self,
        key: impl AsRef<str>,
        values: impl IntoIterator<Item = String>,
    ) -> Result<u32, Error> {
        self.0
            .sadd(key.as_ref().to_string(), values.into_iter().collect())
            .await
    }

    /// Retrieve the contents of the set named `key`.
    // smembers: async func(key: string) -> result<list<string>, error>;
    pub async fn smembers(&self, key: impl AsRef<str>) -> Result<Vec<String>, Error> {
        self.0.smembers(key.as_ref().to_string()).await
    }

    /// Remove the specified `values` from the set named `key`, returning the number of newly-removed values.
    // srem: async func(key: string, values: list<string>) -> result<u32, error>;
    pub async fn srem(
        &self,
        key: impl AsRef<str>,
        values: impl IntoIterator<Item = String>,
    ) -> Result<u32, Error> {
        self.0
            .srem(key.as_ref().to_string(), values.into_iter().collect())
            .await
    }

    /// Execute an arbitrary Redis command and receive the result.
    // execute: async func(command: string, arguments: list<redis-parameter>) -> result<list<redis-result>, error>;
    pub async fn execute(
        &self,
        command: impl AsRef<str>,
        arguments: impl IntoIterator<Item = RedisParameter>,
    ) -> Result<Vec<RedisResult>, Error> {
        self.0
            .execute(
                command.as_ref().to_string(),
                arguments.into_iter().collect(),
            )
            .await
    }
}

impl PartialEq for RedisResult {
    fn eq(&self, other: &Self) -> bool {
        use RedisResult::*;
        match (self, other) {
            (Nil, Nil) => true,
            (Status(a), Status(b)) => a == b,
            (Int64(a), Int64(b)) => a == b,
            (Binary(a), Binary(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for RedisResult {}

impl Hash for RedisResult {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use RedisResult::*;

        match self {
            Nil => (),
            Status(s) => s.hash(state),
            Int64(v) => v.hash(state),
            Binary(v) => v.hash(state),
        }
    }
}
