//! API client для получения данных типов

#[cfg(feature = "web-ui")]
pub mod client;

#[cfg(feature = "web-ui")]
pub use client::*;
