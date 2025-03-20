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
//! # fn main() -> anyhow::Result<()> {
//! let conn = Connection::open("redis://127.0.0.1:6379")?;
//! let payload = conn.get("archimedes-data")?;
//! if let Some(data) = payload {
//!     println!("{}", String::from_utf8_lossy(&data));
//! }
//! # Ok(())
//! # }
//! ```
//!
//! See the [`Connection`] type for further examples.

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
/// # fn main() -> anyhow::Result<()> {
/// let conn = Connection::open("redis://127.0.0.1:6379")?;
/// let payload = conn.get("archimedes-data")?;
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
/// # fn main() -> anyhow::Result<()> {
/// let conn = Connection::open("redis://127.0.0.1:6379")?;
/// let payload = "Eureka!".to_owned().into_bytes();
/// conn.set("archimedes-data", &payload)?;
/// # Ok(())
/// # }
/// ```
///
/// Delete a value from the Redis database.
///
/// ```no_run
/// use spin_sdk::redis::Connection;
///
/// # fn main() -> anyhow::Result<()> {
/// let conn = Connection::open("redis://127.0.0.1:6379")?;
/// conn.del(&["archimedes-data".to_owned()])?;
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
/// # fn use_redis(request: spin_sdk::http::Request) -> anyhow::Result<()> {
/// let conn = Connection::open("redis://127.0.0.1:6379")?;
///
/// let payload = request.body().to_vec();
/// ensure_pet_picture(&payload)?;
///
/// conn.publish("pet-pictures", &payload)?;
/// # Ok(())
/// # }
/// ```
pub use super::wit::v2::redis::Connection;

pub use super::wit::v2::redis::{Error, Payload, RedisParameter, RedisResult};

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
