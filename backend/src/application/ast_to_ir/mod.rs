//! AST to IR Converter - преобразование AST в Intermediate Representation
//!
//! Этот модуль предоставляет функциональность для конвертации синтаксического
//! дерева (AST) из tree-sitter в семантическое представление (IR).
//!
//! # Архитектура
//!
//! ```text
//! AST (backend) -> AstToIrConverter -> SemanticProgram (shared)
//! ```
//!
//! # Структура модуля
//!
//! - `converter` - основная структура `AstToIrConverter` и публичный API
//! - `statement_converter` - конвертация statements (If, While, For, etc.)
//! - `expression_converter` - конвертация выражений (Call, PropertyAccess)
//! - `type_inference` - вывод типов выражений
//! - `global_collections` - информация о глобальных коллекциях 1С
//!
//! # Пример использования
//!
//! ```no_run
//! use bsl_backend::application::ast_to_ir::AstToIrConverter;
//! use bsl_backend::parsing::bsl::ast::Program;
//! use bsl_shared::domain::repository::InMemoryTypeRepository;
//! use bsl_shared::domain::signature_index::SignatureIndex;
//! use std::sync::Arc;
//!
//! let ast = Program { statements: vec![] };
//! let repo = Arc::new(InMemoryTypeRepository::new());
//! let sig_idx = SignatureIndex::new();
//! let ir = AstToIrConverter::convert(
//!     ast,
//!     "source code".to_string(),
//!     "test.bsl".to_string(),
//!     repo,
//!     sig_idx,
//! )?;
//! # Ok::<(), anyhow::Error>(())
//! ```

mod converter;
mod expression_converter;
mod global_collections;
mod statement_converter;
mod type_inference;

#[cfg(test)]
mod tests;

// Re-exports
pub use converter::AstToIrConverter;
pub use global_collections::{
    get_manager_type_for_metadata, is_global_collection, lookup_global_collection,
    GlobalCollectionInfo, GLOBAL_COLLECTIONS_INFO,
};
