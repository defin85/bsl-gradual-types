//! The core domain logic of the type system.

pub mod analysis;
pub mod flow_analysis; // Flow-sensitive analysis for tracking type changes
pub mod generic_inference; // Generic type inference from method calls (Milestone 2.3)
pub mod metadata_lookup; // Bridge between TypeResolution and RawTypeData
pub mod null_safety; // Null safety analysis via CFG (Milestone 2.3)
pub mod repository;
pub mod resolver;
pub mod types;
pub mod validators; // Type validation rules from Balyuk & Popova (2021)

#[cfg(test)]
mod advanced_types_test; // Tests for Milestone 2.3: Advanced Type System

// Re-export key components for easier access
pub use flow_analysis::{
    CfgEdge, CfgNode, CfgNodeKind, ControlFlowGraph, EdgeKind, FlowAnalysisContext,
};
pub use generic_inference::{GenericInference, GenericTypeInfo};
pub use metadata_lookup::TypeMetadataLookup;
pub use null_safety::{NullSafetyAnalyzer, NullSafetyResult, NullSafetyWarning, NullWarningKind};
pub use repository::{CompletionItem, CompletionKind, TypeRepository};
pub use resolver::TypeResolver;
pub use types::{RawTypeData, TypeResolution};
pub use validators::{TypeErrorKind, TypeValidator};
