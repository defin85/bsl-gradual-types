//! BSL Gradual Type System
//!
//! A gradual type system for 1C:Enterprise BSL language that combines
//! static analysis with runtime contracts for comprehensive type safety.

pub mod adapters;
pub mod data;
pub mod core;
pub mod unified;
// Публичный модуль целевой архитектуры
pub mod architecture;

pub mod documentation;

// Плоская структура модулей (адаптеры на период миграции на плоскую структуру)
pub mod domain;
pub mod application;
pub mod presentation;
pub mod system;
pub mod parsing;

pub use core::resolution::TypeResolver;
pub use domain::types;
pub use parsing::bsl::{BslParser, Expression, Statement};

/// Version of the type system
pub const VERSION: &str = "0.1.0";
