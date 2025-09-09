//! LSP (Language Server Protocol) presentation layer
//! 
//! Содержит все компоненты для LSP сервера:
//! - Enhanced LSP server
//! - Code actions
//! - Type hints
//! - Position utilities

pub mod enhanced;
pub mod code_actions;
pub mod type_hints;
pub mod position;

// Re-export главных компонентов
pub use enhanced::*;
pub use code_actions::*;
pub use type_hints::*;
pub use position::*;
