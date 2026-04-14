use {anyhow::Result, spin_sdk::redis_subscriber};

#[redis_subscriber]
async fn on_message(message: Vec<u8>) -> Result<()> {
    assert_eq!(message, b"foo");
    Ok(())
}
