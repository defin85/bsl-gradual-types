//! VSCode-specific webview entry points
//!
//! This module contains WASM entry points for VSCode webviews.
//! Each webview has its own entry point that handles VSCode-specific communication
//! via postMessage API.

pub mod common;
pub mod type_details_app;

#[cfg(test)]
mod tests;

// Re-exports
pub use type_details_app::VsCodeTypeDetailsApp;
