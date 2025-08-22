//! BSL Gradual Type System
//!
//! A gradual type system for 1C:Enterprise BSL language that combines
//! static analysis with runtime contracts for comprehensive type safety.

pub mod adapters;
pub mod architecture;
pub mod core;
pub mod parser;
pub mod query;
// Временная совместимость: старый путь `ideal` указывает на `target`
pub use crate::architecture as ideal;
// Новый алиас целевой архитектуры под именем `target`
pub use crate::architecture as target;

pub mod documentation;

pub use core::{resolution::TypeResolver, types};

pub use parser::{BslParser, Expression, Statement};

/// Version of the type system
pub const VERSION: &str = "0.1.0";
