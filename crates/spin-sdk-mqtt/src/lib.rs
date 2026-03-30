//! The Rust Spin MQTT SDK.
//!
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
//! # fn ensure_pet_picture(_: &[u8]) -> anyhow::Result<()> { Ok(()) }
//! # async fn use_mqtt(request: spin_sdk::http::Request) -> anyhow::Result<()> {
//! let user = spin_sdk::variables::get("mqtt_username").await?;
//! let password = spin_sdk::variables::get("mqtt_password").await?;
//!
//! let conn = Connection::open(
//!     "mqtt://localhost:1883?client_id=123",
//!     &user,
//!     &password,
//!     30 /* seconds */
//! ).await?;
//!
//! let payload = request.body().to_vec();
//! ensure_pet_picture(&payload)?;
//!
//! conn.publish("pet-pictures", &payload, Qos::AtLeastOnce).await?;
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
        world: "spin-sdk-mqtt",
        path: "../../wit",
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
/// # fn ensure_pet_picture(_: &[u8]) -> anyhow::Result<()> { Ok(()) }
/// # async fn use_mqtt(request: spin_sdk::http::Request) -> anyhow::Result<()> {
/// let user = spin_sdk::variables::get("mqtt_username").await?;
/// let password = spin_sdk::variables::get("mqtt_password").await?;
///
/// let conn = Connection::open(
///     "mqtt://localhost:1883?client_id=123",
///     &user,
///     &password,
///     30 /* seconds */
/// ).await?;
///
/// let payload = request.body().to_vec();
/// ensure_pet_picture(&payload)?;
///
/// conn.publish("pet-pictures", &payload, Qos::AtLeastOnce).await?;
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
