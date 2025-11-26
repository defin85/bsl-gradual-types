//! The core domain logic of the type system.

pub mod analysis;
pub mod code_location; // Code location and execution context determination (Milestone 3.12)
pub mod flow_analysis; // Flow-sensitive analysis for tracking type changes
pub mod generic_inference; // Generic type inference from method calls (Milestone 2.3)
pub mod metadata_lookup; // Bridge between TypeResolution and RawTypeData
pub mod metadata_patterns; // MetadataKind pattern registry from Syntax Helper (Milestone 3.13)
pub mod null_safety; // Null safety analysis via CFG (Milestone 2.3)
pub mod repository;
pub mod resolver;
pub mod signature_index; // Function signature validation system (Milestone 2.20)
pub mod runtime_context; // Runtime execution context tracking (Milestone 3.11 Phase 2)
pub mod type_definition_location; // Go To Definition location types (Milestone 3.14)
pub mod types;
pub mod validators; // Type validation rules from Balyuk & Popova (2021)

#[cfg(test)]
mod advanced_types_test; // Tests for Milestone 2.3: Advanced Type System

// Re-export key components for easier access
pub use code_location::{
    CodeLocation, CompilerDirective, ExecutionContext, MetadataContext, ModuleType,
};
pub use flow_analysis::{
    CfgEdge, CfgNode, CfgNodeKind, ControlFlowGraph, EdgeKind, FlowAnalysisContext,
};
pub use generic_inference::{GenericInference, GenericTypeInfo};
pub use metadata_lookup::TypeMetadataLookup;
pub use null_safety::{NullSafetyAnalyzer, NullSafetyResult, NullSafetyWarning, NullWarningKind};
pub use repository::{CompletionItem, CompletionKind, TypeRepository};
pub use resolver::{ConstructorResolution, TypeResolver, ValidationResult, ValidationResultV2};
pub use signature_index::{
    MethodSignature, SignatureIndex, SignatureMismatch, SignatureSource, SignatureValidationResult,
};
pub use types::{RawTypeData, TypeCompatibility, TypeRef, TypeResolution};
pub use validators::{TypeErrorKind, TypeValidator};
pub use runtime_context::{ContextRequirements, RuntimeExecutionContext};
pub use metadata_patterns::{ExtractedPattern, MetadataPatternRegistry};
pub use type_definition_location::{ModulePaths, TypeDefinitionLocation};
