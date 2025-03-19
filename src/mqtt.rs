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
//! # fn use_mqtt(request: spin_sdk::http::Request) -> anyhow::Result<()> {
//! let user = spin_sdk::variables::get("mqtt_username")?;
//! let password = spin_sdk::variables::get("mqtt_password")?;
//!
//! let conn = Connection::open(
//!     "mqtt://localhost:1883?client_id=123",
//!     &user,
//!     &password,
//!     30 /* seconds */
//! )?;
//!
//! let payload = request.body().to_vec();
//! ensure_pet_picture(&payload)?;
//!
//! conn.publish("pet-pictures", &payload, Qos::AtLeastOnce)?;
//! # Ok(())
//! # }
//! ```

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
/// # fn use_mqtt(request: spin_sdk::http::Request) -> anyhow::Result<()> {
/// let user = spin_sdk::variables::get("mqtt_username")?;
/// let password = spin_sdk::variables::get("mqtt_password")?;
///
/// let conn = Connection::open(
///     "mqtt://localhost:1883?client_id=123",
///     &user,
///     &password,
///     30 /* seconds */
/// )?;
///
/// let payload = request.body().to_vec();
/// ensure_pet_picture(&payload)?;
///
/// conn.publish("pet-pictures", &payload, Qos::AtLeastOnce)?;
/// # Ok(())
/// # }
/// ```
pub use super::wit::v2::mqtt::Connection;

pub use super::wit::v2::mqtt::{Error, Payload, Qos};
