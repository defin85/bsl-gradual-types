//! Domain Layer: Type Resolver Module
//!
//! Чистая бизнес-логика разрешения типов без Application concerns
//!
//! ## Структура модуля
//!
//! - `result_types` - Типы результатов резолюции и валидации
//! - `resolver_core` - Основной TypeResolver struct и методы
//! - `strategies` - Стратегии резолюции составных типов (Union, Intersection, Generic, Nullable)
//! - `member_resolution` - Резолюция доступа к членам типов
//! - `helpers` - Вспомогательные функции форматирования и сравнения

mod helpers;
mod member_resolution;
mod resolver_core;
mod result_types;
mod strategies;

// Re-exports for public API
pub use resolver_core::TypeResolver;
pub use result_types::{ConstructorResolution, ValidationResult, ValidationResultV2};

// Re-export helpers for external use
pub use helpers::{
    format_generic_type, format_intersection_type, format_nullable_type, format_union_type,
    is_type_compatible, names_equal_ignore_case,
};

// Re-export strategies for advanced use cases
pub use strategies::{GenericStrategy, IntersectionStrategy, NullableStrategy, UnionStrategy};

// Re-export member resolution
pub use member_resolution::MemberResolver;

// ===== Tests =====

// Milestone 2.20: Function Signature Validation tests
#[cfg(test)]
mod resolver_validation_tests;

// Milestone 2.3: Union Types tests
#[cfg(test)]
mod resolver_union_tests;

// Milestone 2.3 Task 2: Intersection Types tests
#[cfg(test)]
mod resolver_intersection_tests;

// Milestone 2.3 Task 3: Generic Types tests
#[cfg(test)]
mod resolver_generic_tests;

// Milestone 2.3 Task 4: Nullable Types tests
#[cfg(test)]
mod resolver_nullable_tests;

// Generic Tabular Sections: Resolver tests
#[cfg(test)]
mod resolver_tabular_tests;

// Milestone 2.21: Constructor Resolution tests
#[cfg(test)]
mod resolver_constructor_tests;
