//! BSL Gradual Type System - Backend
//!
//! Backend server with API, LSP, and all server-side logic.
//! Contains System, Application, Parsing, Presentation, and Data layers.

#[cfg(not(target_arch = "wasm32"))]
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod presentation;
#[cfg(not(target_arch = "wasm32"))]
pub use bsl_runtime::{application, data, domain, helpers, parsing, system};

// Re-export main services
#[cfg(not(target_arch = "wasm32"))]
pub use bsl_runtime::SystemCoordinator;

/// Version of the backend
pub const VERSION: &str = "0.4.2";
