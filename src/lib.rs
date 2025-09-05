//! BSL Gradual Type System
//!
//! A gradual type system for 1C:Enterprise BSL language that combines
//! static analysis with runtime contracts for comprehensive type safety.

// === FLAT ARCHITECTURE MODULES ===
pub mod data;
pub mod parsing;
pub mod domain;
pub mod application;
pub mod presentation;
pub mod system;

// Main public API re-exports
pub use domain::types;
pub use domain::resolution_service::TypeResolver;
pub use application::TypeSystemService; // CLEANUP: unified API
pub use parsing::bsl::{BslParser, Expression, Statement};

/// Version of the type system
pub const VERSION: &str = "0.1.0";
