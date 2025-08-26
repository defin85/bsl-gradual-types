//! Domain layer (flat structure)
//! Includes type definitions, analysis components and resolvers

pub mod analysis;
pub mod context;
pub mod contracts;
pub mod events;
pub mod repository;
pub mod resolution_service;
pub mod resolvers;
pub mod search;
pub mod standard_types;
pub mod type_system_service;
pub mod types;
pub mod unified_type_system;

// Re-export main types for easier access
pub use analysis::type_checker::{TypeChecker, TypeContext, TypeDiagnostic};
pub use repository::{
    InMemoryTypeRepository, TypeCheckerService, TypeRepository, TypeResolutionService,
};
pub use resolution_service::TypeResolver;
pub use search::{
    AdvancedSearchQuery, ParseMetadata, RawMethodData, RawParameterData, RawPropertyData,
    RawTypeData, SearchResults, TypeHierarchy, TypeSearchResult,
};
pub use types::*;
