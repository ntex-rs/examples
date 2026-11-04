//! Register nested product and part routes through application configuration.
//!
//! Look at `appconfig::config_app` for nested scopes and `handlers` for
//! JSON, query, and path extractors. The handlers are storage-free stubs.
//! Run the `main` binary to listen on <http://127.0.0.1:8080>.

pub mod appconfig;
pub mod common;
pub mod handlers;
