//! Analysis components in the domain layer

pub mod dependency_graph;
pub mod facets;
pub mod flow_sensitive;
pub mod interprocedural;
pub mod type_checker;
pub mod type_narrowing;
pub mod union_types;

// Экспорт основных типов для удобного импорта
pub use dependency_graph::{DependencyNode, DependencyType, Scope, TypeDependencyGraph};
pub use flow_sensitive::FlowSensitiveAnalyzer;
pub use interprocedural::{CallGraph, InterproceduralAnalyzer};
pub use type_checker::{
    DiagnosticSeverity, FunctionSignature, TypeChecker, TypeContext, TypeDiagnostic,
};
pub use type_narrowing::TypeNarrower;
pub use union_types::UnionTypeManager;
