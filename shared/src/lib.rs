//! BSL Gradual Type System - Shared Components
//!
//! Common types, domain logic, and API contracts shared across all crates.
//! This crate contains no I/O or heavy dependencies to ensure WASM compatibility.

pub mod api;
pub mod domain;
pub mod types;
pub mod engine;

// Re-export main types (simplified according to architecture specification)
pub use api::*;
pub use domain::{TypeResolver, TypeRepository}; // Only essential domain components
pub use types::*;

/// Version of the shared components
pub const VERSION: &str = "0.4.2";
