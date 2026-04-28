//! The core domain logic of the type system.

pub mod analysis;
pub mod code_location; // Code location and execution context determination (Milestone 3.12)
pub mod facet_utils; // Centralized facet type extraction utilities (Phase 4.1 refactoring)
pub mod flow_analysis; // Flow-sensitive analysis for tracking type changes
pub mod generic_inference; // Generic type inference from method calls (Milestone 2.3)
pub mod global_context; // Global-context properties from Syntax Helper
pub mod metadata_constants; // Centralized metadata collections and faceted types constants
pub mod metadata_lookup; // Bridge between TypeResolution and RawTypeData
pub mod metadata_patterns; // MetadataKind pattern registry from Syntax Helper (Milestone 3.13)
pub mod null_safety; // Null safety analysis via CFG (Milestone 2.3)
pub mod repository;
pub mod resolver;
pub mod runtime_context; // Runtime execution context tracking (Milestone 3.11 Phase 2)
pub mod signature_index; // Function signature validation system (Milestone 2.20)
pub mod signature_registry; // Registry pattern for SignatureIndex data sources
pub mod type_definition_location; // Go To Definition location types (Milestone 3.14)
pub mod type_id; // TypeId infrastructure for type-safe identifiers (R1)
pub mod types;
pub mod validators; // Type validation rules from Balyuk & Popova (2021)

#[cfg(test)]
mod advanced_types_test; // Tests for Milestone 2.3: Advanced Type System

// Re-export key components for easier access
pub use code_location::{
    CodeLocation, CompilerDirective, ExecutionContext, MetadataContext, ModuleType,
};
pub use facet_utils::{
    extract_base_facet_type, extract_base_facet_type_universal, extract_placeholder_base_type,
    is_known_facet_prefix,
};
pub use flow_analysis::{
    CfgEdge, CfgNode, CfgNodeKind, ControlFlowGraph, EdgeKind, FlowAnalysisContext,
};
pub use generic_inference::{GenericInference, GenericTypeInfo};
pub use global_context::{
    normalize_global_context_property_key, strip_global_context_property_owner,
    GlobalContextAvailability, GlobalContextIndex, GlobalContextPropertyData,
    GLOBAL_CONTEXT_SOURCE_KEY_NOTE_PREFIX, GLOBAL_CONTEXT_SOURCE_NOTE,
};
pub use metadata_constants::{
    get_base_type_info, get_collection_kind, get_faceted_type_info, is_configuration_type_pattern,
    is_faceted_type, is_metadata_collection, FACETED_TYPES, METADATA_COLLECTIONS,
};
pub use metadata_lookup::TypeMetadataLookup;
pub use metadata_patterns::{ExtractedPattern, MetadataPatternRegistry};
pub use null_safety::{NullSafetyAnalyzer, NullSafetyResult, NullSafetyWarning, NullWarningKind};
pub use repository::{CompletionItem, CompletionKind, TypeRepository};
pub use resolver::{ConstructorResolution, TypeResolver, ValidationResult, ValidationResultV2};
pub use runtime_context::{ContextRequirements, RuntimeExecutionContext};
pub use signature_index::{
    ConstructorSignature, ContextRequirements as SignatureContextRequirements, MethodSignature,
    SignatureIndex, SignatureMismatch, SignatureSource, SignatureValidationResult,
};
pub use signature_registry::{SignatureDataSource, SignatureSourceRegistry};
pub use type_definition_location::{ModulePaths, TypeDefinitionLocation};
pub use type_id::{camel_to_spaced, normalize, spaced_to_camel, TypeId};
pub use types::{RawTypeData, TypeCompatibility, TypeRef, TypeResolution};
pub use validators::{TypeErrorKind, TypeValidator};
