//! bsl-runtime
//!
//! Shared library/runtime layer used by `bsl-backend` (web/LSP adapters) and `bsl-agent` (MCP stdio).
//! Contains System, Application, Parsing, Domain, Data, Helpers layers (no HTTP/MCP adapters).

#[cfg(not(target_arch = "wasm32"))]
pub mod application;
#[cfg(not(target_arch = "wasm32"))]
pub mod data;
#[cfg(not(target_arch = "wasm32"))]
pub mod domain;
#[cfg(not(target_arch = "wasm32"))]
pub mod helpers;
#[cfg(not(target_arch = "wasm32"))]
pub mod parsing;
#[cfg(not(target_arch = "wasm32"))]
pub mod system;

#[cfg(not(target_arch = "wasm32"))]
pub use system::SystemCoordinator;
