//! The Rust Spin SDK.

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// #[cfg(test)]
// mod test;

/// Re-export Spin HTTP SDK
pub use spin_sdk_http as http;
/// Re-export SPIN KeyValue SDK
pub use spin_sdk_kv as key_value;
/// Re-export Spin LLM SDK
pub use spin_sdk_llm as llm;
/// Re-export entrypoint macros
pub use spin_sdk_macro::{http_service, redis_subscriber};
/// Re-export Spin Mqtt SDK
pub use spin_sdk_mqtt as mqtt;
/// Re-export Spin MySQL SDK
pub use spin_sdk_mysql as mysql;
/// Re-export Spin Postgres SDK
pub use spin_sdk_pg as pg;
/// Re-export Spin Redis SDK
pub use spin_sdk_redis as redis;
/// Re-export Spin SQLite SDK
pub use spin_sdk_sqlite as sqlite;
/// Re-export Spin Variables SDK
pub use spin_sdk_variables as variables;

#[export_name = concat!("spin-sdk-version-", env!("SDK_VERSION"))]
extern "C" fn __spin_sdk_version() {}

#[cfg(feature = "export-sdk-language")]
#[export_name = "spin-sdk-language-rust"]
extern "C" fn __spin_sdk_language() {}

#[export_name = concat!("spin-sdk-commit-", env!("SDK_COMMIT"))]
extern "C" fn __spin_sdk_hash() {}

#[doc(hidden)]
pub use wit_bindgen;
