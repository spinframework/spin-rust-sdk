//! Time-related functions.
//!
//! This module provides asynchronous timing primitives for use inside Spin
//! components. It currently exposes [`sleep`], which suspends the current task
//! until a [`Duration`] has elapsed, driven by the host's monotonic clock.
//!
//! Unlike [`std::thread::sleep`], [`sleep`] is `async` and yields back to the
//! executor while it waits, allowing other tasks to make progress.
//!
//! # Examples
//!
//! Pause execution for half a second:
//!
//! ```no_run
//! use std::time::Duration;
//!
//! # async fn run() {
//! spin_sdk::time::sleep(Duration::from_millis(500)).await;
//! # }
//! ```

use std::time::Duration;

/// Wait until the given [`Duration`] has elapsed.
pub async fn sleep(duration: Duration) {
    let duration_ns = duration.as_nanos().try_into().unwrap_or(u64::MAX);
    crate::wasip3::clocks::monotonic_clock::wait_for(duration_ns).await;
}
