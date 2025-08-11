//! Core type definitions for the gradual type system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Central abstraction - not a type, but a "type resolution" with confidence level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeResolution {
    /// Level of confidence in the resolution
    pub certainty: Certainty,
    
    /// The actual resolution result
    pub result: ResolutionResult,
    
    /// Source of the type information
    pub source: ResolutionSource,
    
    /// Metadata for debugging and diagnostics
    pub metadata: ResolutionMetadata,
    
    /// Active facet for configuration objects
    pub active_facet: Option<FacetKind>,
    
    /// Available facets for this type
    pub available_facets: Vec<FacetKind>,
}

/// Confidence levels for type resolution
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Certainty {
    /// Type is 100% known statically
    Known,
    
    /// Type is inferred with confidence level (0.0 - 1.0)
    Inferred(f32),
    
    /// Type cannot be determined statically
    Unknown,
}

/// Result of type resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionResult {
    /// Concrete type is known
    Concrete(ConcreteType),
    
    /// One of several possible types (union)
    Union(Vec<WeightedType>),
    
    /// Type depends on runtime conditions
    Conditional(Box<ConditionalType>),
    
    /// Type with context and effects
    Contextual(Box<ContextualType>),
    
    /// Type is fully dynamic (determined at runtime)
    Dynamic,
}

/// A type with probability weight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedType {
    pub type_: ConcreteType,
    pub weight: f32,
}

/// Concrete BSL type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConcreteType {
    /// Platform type (Array, Map, etc.)
    Platform(PlatformType),
    
    /// Configuration object (Catalog, Document, etc.)
    Configuration(ConfigurationType),
    
    /// Primitive type (String, Number, Boolean, etc.)
    Primitive(PrimitiveType),
    
    /// Special types (Undefined, Null)
    Special(SpecialType),
}

/// Platform-provided types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformType {
    pub name: String,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
}

/// Configuration-specific types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigurationType {
    pub kind: MetadataKind,
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub tabular_sections: Vec<TabularSection>,
}

/// Metadata kinds in 1C
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetadataKind {
    Catalog,
    Document,
    Register,
    Report,
    DataProcessor,
    Enum,
    ChartOfAccounts,
    ChartOfCharacteristicTypes,
}

/// Primitive BSL types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    String,
    Number,
    Boolean,
    Date,
}

/// Special BSL types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecialType {
    Undefined,
    Null,
    Type,
}

/// Method definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Method {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
}

/// Property definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub type_: String,
    pub readonly: bool,
}

/// Method parameter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_: Option<String>,
    pub optional: bool,
}

/// Configuration attribute
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub type_: String,  // Can be composite like "СправочникСсылка.Контрагенты,СправочникСсылка.Организации,Строка(10)"
    pub is_composite: bool,  // True if type contains multiple types
    pub types: Vec<String>,  // Individual types if composite
}

/// Tabular section of a configuration object
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabularSection {
    pub name: String,
    pub synonym: Option<String>,
    pub attributes: Vec<Attribute>,
}

/// Conditional type that depends on runtime conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalType {
    pub condition: String,
    pub then_type: ResolutionResult,
    pub else_type: ResolutionResult,
}

/// Type with context and effects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualType {
    pub base_type: ResolutionResult,
    pub effects: Vec<TypeEffect>,
    pub context: ExecutionContext,
}

/// Type effects that modify behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeEffect {
    MayBeNull,
    RequiresTransaction,
    RequiresLock(String),
    RequiresContext(ExecutionContext),
    ModifiedByExtension(String),
}

/// Execution context for types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionContext {
    Server,
    Client,
    ThickClient,
    WebClient,
    MobileClient,
    ExternalConnection,
}

/// Source of type resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionSource {
    Static,
    Inferred,
    Annotated,
    Runtime,
    Predicted,
}

/// Metadata for type resolution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolutionMetadata {
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub notes: Vec<String>,
}

/// Combined type information with gradual typing support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedBslType {
    /// Core name of the type
    pub core_name: String,
    
    /// Metadata kind (for configuration types)
    pub metadata_kind: Option<MetadataKind>,
    
    /// Available facets for this type
    pub facets: HashMap<FacetKind, Facet>,
    
    /// Type resolution information
    pub resolution: TypeResolution,
    
    /// Gradual typing information
    pub gradual_info: GradualInfo,
    
    /// Version information
    pub version_info: VersionInfo,
}

/// Facet kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacetKind {
    Manager,
    Object,
    Reference,
    Metadata,
    Constructor,
    Collection,
    Singleton,
}

/// Facet definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Facet {
    pub kind: FacetKind,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
}

/// Gradual typing information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GradualInfo {
    pub static_type: Option<StaticType>,
    pub dynamic_contract: Option<Contract>,
    pub confidence: f32,
}

/// Static type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticType {
    pub name: String,
    pub signature: Option<String>,
}

/// Runtime contract for dynamic checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub check_code: String,
    pub error_message: String,
}

/// Version information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VersionInfo {
    pub platform_version: String,
    pub last_modified: u64,
    pub cache_key: String,
}

impl TypeResolution {
    /// Create a known type resolution
    pub fn known(concrete: ConcreteType) -> Self {
        Self {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(concrete),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }
    
    /// Create an unknown type resolution
    pub fn unknown() -> Self {
        Self {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }
    
    /// Check if the type is fully resolved
    pub fn is_resolved(&self) -> bool {
        matches!(self.certainty, Certainty::Known)
    }
}