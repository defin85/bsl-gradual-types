//! Analysis Layer: Type Guards and Narrowing Engine
//!
//! Milestone 3.7: Advanced Type Narrowing
//!
//! Этот модуль содержит логику для:
//! - Обнаружения type guards (проверок типов в условиях)
//! - Сужения типов на основе control-flow анализа
//! - Интеграции с Type Resolver для flow-sensitive typing

pub mod narrowing_engine;
pub mod type_guards;

pub use narrowing_engine::{NarrowingContext, NarrowingEngine};
pub use type_guards::{detect_type_guards, TypeGuard};
