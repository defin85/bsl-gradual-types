//! Domain layer (simplified structure)
//! Essential Components Only: TypeResolver + TypeRepository + basic types
//! WASM-compatible core domain logic

// Core domain modules (only essential components)
pub mod repository;
pub mod standard_types;
pub mod types;

// Re-export main types for easier access (simplified according to specification)
pub use repository::{
    CompletionItem, CompletionKind, InMemoryTypeRepository, TypeCheckerService, TypeRepository,
    TypeResolver,
};
pub use types::*;

// Re-export basic analysis types (for compatibility during migration)
pub mod analysis {
    pub mod type_checker {
        pub use crate::domain::types::{
            DiagnosticSeverity, FunctionSignature, ParameterInfo, TypeChecker, TypeContext,
            TypeDiagnostic,
        };
    }
    pub mod dependency_graph {
        pub use crate::domain::types::{
            DependencyEdge, DependencyNode, DependencyType, Scope, SourceLocation,
            TypeDependencyGraph,
        };
    }
    pub mod facets {
        pub use crate::domain::types::FacetRegistry;
    }
    pub mod union_types {
        pub use crate::domain::types::UnionTypeManager;
    }
    pub use crate::domain::types::TypeContext;

    // Direct re-exports for easier access
    pub use crate::domain::types::{DependencyNode, DependencyType, Scope, TypeDependencyGraph};
}

// Basic context module (for compatibility)
pub mod context {
    pub use crate::domain::types::ContextResolver;
}
