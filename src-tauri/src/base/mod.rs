//! Basics every layer uses: configuration, keyring, log, connection status.
pub mod config;
pub mod html;
pub mod i18n;
pub mod log;
#[cfg(target_os = "linux")]
pub mod migrate;
pub mod notify;
pub mod open;
pub mod status;
