//! Domain layer (flat structure)
//! Includes type definitions, analysis components and resolvers

pub mod analysis;
pub mod context;
pub mod contracts;
pub mod events;
pub mod repository;
pub mod resolution_service;
mod resolvers; // ✅ ПРИВАТНЫЙ - недоступен для Application Layer
pub mod search;
pub mod standard_types;
pub mod type_system_service;
pub mod types;

// Re-export main types for easier access
pub use analysis::type_checker::{TypeChecker, TypeContext, TypeDiagnostic};
pub use repository::{
    InMemoryTypeRepository, TypeCheckerService, TypeRepository, TypeResolutionService,
};
pub use resolution_service::TypeResolver;
// ✅ Экспортируем из repository
pub use repository::{CompletionItem, CompletionKind};
pub use search::{
    AdvancedSearchQuery, ParseMetadata, RawMethodData, RawParameterData, RawPropertyData,
    RawTypeData, SearchResults, TypeHierarchy, TypeSearchResult,
};
pub use types::*;
