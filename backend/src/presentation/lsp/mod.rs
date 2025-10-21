//! LSP (Language Server Protocol) presentation layer
//!
//! Содержит все компоненты для LSP сервера:
//! - Enhanced LSP server
//! - Code actions
//! - Type hints
//! - Position utilities

// TODO Phase 2: Update enhanced.rs to use TypeInferenceService instead of TypeResolver
// pub mod enhanced;
pub mod code_actions;
pub mod position;
pub mod type_hints;

// Re-export главных компонентов
// pub use enhanced::*;
pub use code_actions::*;
pub use position::*;
pub use type_hints::*;
