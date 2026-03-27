use anyhow::Result;
use spin_sdk::{
    http::{IntoResponse, Request},
    http_service, mqtt,
};
use std::env;

// The environment variable set in `spin.toml` that points to the
// address of the Mqtt server (with other attributes) that the component will publish
// a message to.
const MQTT_ADDRESS_ENV: &str = "MQTT_ADDRESS";
const MQTT_USERNAME_ENV: &str = "MQTT_USERNAME";
const MQTT_PASSWORD_ENV: &str = "MQTT_PASSWORD";
const MQTT_KEEP_ALIVE_INTERVAL_ENV: &str = "MQTT_KEEP_ALIVE_INTERVAL";

// The environment variable set in `spin.toml` that specifies
// the Mqtt topic that the component will publish to.
const MQTT_TOPIC_ENV: &str = "MQTT_TOPIC";

/// This HTTP component demonstrates publishing a value to Mqtt
/// topic. The component is triggered by an HTTP
/// request served on the route configured in the `spin.toml`.
#[http_service]
async fn publish(_req: Request) -> Result<impl IntoResponse> {
    let address = env::var(MQTT_ADDRESS_ENV)?;
    let username = env::var(MQTT_USERNAME_ENV)?;
    let password = env::var(MQTT_PASSWORD_ENV)?;
    let keep_alive_interval = env::var(MQTT_KEEP_ALIVE_INTERVAL_ENV)?.parse::<u64>()?;
    let topic = env::var(MQTT_TOPIC_ENV)?;

    let message = Vec::from("Eureka!");

    // Open connection to Mqtt server
    let conn = mqtt::Connection::open(address, username, password, keep_alive_interval).await?;

    // Publish to Mqtt server
    match conn.publish(topic, message, mqtt::Qos::AtLeastOnce).await {
        Ok(()) => Ok(http::StatusCode::OK),
        Err(_e) => Ok(http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}
