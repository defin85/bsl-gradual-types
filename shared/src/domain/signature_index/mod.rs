//! Signature Index - индекс сигнатур функций и методов
//!
//! Milestone 2.20: Function Signature Validation System
//! Milestone 3.11: Method Signature Enhancement with Facets and Context
//! Milestone 3.13: MetadataPatternRegistry Integration
//! Milestone 3.15: Lazy Resolution with Arc<OnceLock>
//!
//! # Структура модуля
//! - `types` - Вспомогательные типы (ConstructorSignature, SignatureSource, etc.)
//! - `method` - MethodSignature с lazy resolution
//! - `facet_helpers` - Функции для работы с фасетными типами
//! - `index` - SignatureIndex struct и основная логика
//! - `tests` - Тесты

mod facet_helpers;
mod index;
mod method;
mod method_builder;
#[cfg(test)]
mod tests;
mod types;

// ==================== Re-exports ====================

// Основные типы
pub use index::SignatureIndex;
pub use method::MethodSignature;
pub use method_builder::MethodBuilder;
pub use types::{
    ConstructorSignature, SignatureMismatch, SignatureSource, SignatureValidationResult,
};

// ContextRequirements для обратной совместимости
pub use super::runtime_context::ContextRequirements;

// Facet helper functions (статические методы SignatureIndex делегируют сюда)
pub use facet_helpers::{
    extract_base_facet_type, extract_metadata_name, get_facet_kind_from_prefix,
    substitute_type_name,
};
