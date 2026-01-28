//! BSL Gradual Type System - Shared Components
//!
//! Common types, domain logic, and API contracts shared across all crates.
//! This crate contains no I/O or heavy dependencies to ensure WASM compatibility.

pub mod analysis; // Milestone 3.7: Type Guards & Narrowing Engine
pub mod api;
pub mod domain;
pub mod engine;
pub mod formatting; // Milestone 3.6 Phase 1: DetailLevel enum для hover
pub mod ir; // Milestone 2.8: Intermediate Representation (IR)
pub mod types;
pub mod utils; // Утилиты (hash, и т.д.)
               // УДАЛЕНО Phase 1: loaders переехали в backend/src/data/
               // pub mod loaders;

// Re-export main types (simplified according to architecture specification)
pub use api::*;
pub use domain::{TypeRepository, TypeResolver}; // Only essential domain components
pub use types::*;

/// Version of the shared components
pub const VERSION: &str = "0.4.2";
