use anyhow::Result;
use bytes::Bytes;
use spin_sdk::redis_subscriber;
use std::str::from_utf8;

/// A simple Spin Redis component.
#[redis_subscriber]
async fn on_message(message: Bytes) -> Result<()> {
    println!("{}", from_utf8(&message)?);
    Ok(())
}
