use {anyhow::Result, bytes::Bytes, spin_sdk::redis_component, std::ops::Deref};

#[redis_component]
fn on_message(message: Bytes) -> Result<()> {
    assert_eq!(message.deref(), b"foo");
    Ok(())
}
