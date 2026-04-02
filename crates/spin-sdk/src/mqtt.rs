//! MQTT message publishing.
//!
//! To receive MQTT messages, use the MQTT trigger.
//!
//! # Examples
//!
//! Send an MQTT message.
//!
//! ```no_run
//! use spin_sdk::mqtt::{Connection, Qos};
//!
//! # async fn run() -> anyhow::Result<()> {
//! let conn = Connection::open(
//!     "mqtt://localhost:1883?client_id=123",
//!     "user",
//!     "password",
//!     30 /* seconds */
//! ).await?;
//!
//! let payload = b"hello mqtt".to_vec();
//!
//! conn.publish("pet-pictures", payload, Qos::AtLeastOnce).await?;
//! # Ok(())
//! # }
//! ```

#[doc(hidden)]
/// Module containing wit bindgen generated code.
///
/// This is only meant for internal consumption.
pub mod wit {
    #![allow(missing_docs)]
    use crate::wit_bindgen;

    wit_bindgen::generate!({
        runtime_path: "crate::wit_bindgen::rt",
        world: "spin-sdk-mqtt",
        path: "wit",
        generate_all,
    });

    pub use spin::mqtt::mqtt;
}

/// An open connection to an MQTT queue.
///
/// The address must be in URL form, and must include a `client_id`:
/// `mqtt://hostname?client_id=...`
///
/// # Examples
///
/// Send an MQTT message.
///
/// ```no_run
/// use spin_sdk::mqtt::{Connection, Qos};
///
/// # async fn run() -> anyhow::Result<()> {
/// let conn = Connection::open(
///     "mqtt://localhost:1883?client_id=123",
///     "user",
///     "password",
///     30 /* seconds */
/// ).await?;
///
/// let payload = b"hello mqtt".to_vec();
///
/// conn.publish("pet-pictures", payload, Qos::AtLeastOnce).await?;
/// # Ok(())
/// # }
/// ```
pub struct Connection(wit::mqtt::Connection);

pub use wit::mqtt::{Error, Payload, Qos};

impl Connection {
    /// Open a connection to the Mqtt instance at `address`.
    pub async fn open(
        address: impl AsRef<str>,
        username: impl AsRef<str>,
        password: impl AsRef<str>,
        keep_alive_interval_in_secs: u64,
    ) -> Result<Self, Error> {
        wit::mqtt::Connection::open(
            address.as_ref().to_string(),
            username.as_ref().to_string(),
            password.as_ref().to_string(),
            keep_alive_interval_in_secs,
        )
        .await
        .map(Connection)
    }

    /// Publish an Mqtt message to the specified `topic`.
    pub async fn publish(
        &self,
        topic: impl AsRef<str>,
        payload: Vec<u8>,
        qos: Qos,
    ) -> Result<(), Error> {
        self.0
            .publish(topic.as_ref().to_string(), payload, qos)
            .await
    }
}
