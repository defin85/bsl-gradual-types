//! BSL Gradual Type System
//! 
//! A gradual type system for 1C:Enterprise BSL language that combines
//! static analysis with runtime contracts for comprehensive type safety.

pub mod core;
pub mod adapters;
pub mod parser;
pub mod query;
pub mod target;
// Временная совместимость: старый путь `ideal` указывает на `target`
pub use crate::target as ideal;

pub mod documentation;

pub use core::{
    types::{UnifiedBslType, TypeResolution, Certainty, Contract},
    resolution::TypeResolver,
};

pub use parser::{BslParser, Statement, Expression};

/// Version of the type system
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
