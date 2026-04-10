use std::time::Duration;

/// Wait until the given [`Duration`] has elapsed.
pub async fn sleep(duration: Duration) {
    let duration_ns = duration.as_nanos().try_into().unwrap_or(u64::MAX);
    crate::wasip3::clocks::monotonic_clock::wait_for(duration_ns).await;
}
