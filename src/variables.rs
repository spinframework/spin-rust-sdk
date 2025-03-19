//! Component configuration variables.
//!
//! Component variables must be defined in the application
//! manifest, in the `[component.<name>.variables]` section.
//! Component variables typically use template syntax to
//! derive values from application variables, which are
//! the only variables that may be overridden directly (for
//! example, on the Spin command line).
//!
//! # Examples
//!
//! Get the value of a component variable.
//!
//! ```no_run
//! # fn main() -> anyhow::Result<()> {
//! let region = spin_sdk::variables::get("region_id")?;
//! let regional_url = format!("https://{region}.db.example.com");
//! # Ok(())
//! # }
//! ```
//!
//! Fail gracefully if a variable is not set.
//!
//! ```no_run
//! use spin_sdk::variables::Error;
//!
//! # fn main() -> anyhow::Result<()> {
//! let favourite = match spin_sdk::variables::get("favourite") {
//!     Ok(value) => value,
//!     Err(Error::Undefined(_)) => "not playing favourites".to_owned(),
//!     Err(e) => anyhow::bail!(e),
//! };
//! # Ok(())
//! # }
//! ```

/// Get the value of a component variable.
///
/// # Examples
///
/// Get the value of a component variable.
///
/// ```no_run
/// # fn main() -> anyhow::Result<()> {
/// let region = spin_sdk::variables::get("region_id")?;
/// let regional_url = format!("https://{region}.db.example.com");
/// # Ok(())
/// # }
/// ```
///
/// Fail gracefully if a variable is not set.
///
/// ```no_run
/// use spin_sdk::variables::Error;
///
/// # fn main() -> anyhow::Result<()> {
/// let favourite = match spin_sdk::variables::get("favourite") {
///     Ok(value) => value,
///     Err(Error::Undefined(_)) => "not playing favourites".to_owned(),
///     Err(e) => anyhow::bail!(e),
/// };
/// # Ok(())
/// # }
/// ```
#[doc(inline)]
pub use super::wit::v2::variables::get;

#[doc(inline)]
pub use super::wit::v2::variables::Error;
