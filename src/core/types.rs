//! Core type definitions for the gradual type system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Central abstraction - not a type, but a "type resolution" with confidence level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    /// Global function (Min, Max, String, etc.)
    GlobalFunction(GlobalFunction),
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

impl ToString for PrimitiveType {
    fn to_string(&self) -> String {
        match self {
            PrimitiveType::String => "Строка".to_string(),
            PrimitiveType::Number => "Число".to_string(),
            PrimitiveType::Boolean => "Булево".to_string(),
            PrimitiveType::Date => "Дата".to_string(),
        }
    }
}

/// Special BSL types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecialType {
    Undefined,
    Null,
    Type,
}

/// Global function definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalFunction {
    pub name: String,
    pub english_name: String,
    pub parameters: Vec<GlobalFunctionParameter>,
    pub return_type: Option<Box<TypeResolution>>,
    pub pure: bool,                              // Pure function without side effects
    pub polymorphic: bool,                       // Polymorphic function (type depends on args)
    pub context_required: Vec<ExecutionContext>, // Where available
}

/// Global function parameter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalFunctionParameter {
    pub name: String,
    pub type_: Option<Box<TypeResolution>>,
    pub is_optional: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

/// Method definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Method {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub is_function: bool,
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
    pub by_value: bool,
}

/// Configuration attribute
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub type_: String, // Can be composite like "СправочникСсылка.Контрагенты,СправочникСсылка.Организации,Строка(10)"
    pub is_composite: bool, // True if type contains multiple types
    pub types: Vec<String>, // Individual types if composite
}

/// Tabular section of a configuration object
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabularSection {
    pub name: String,
    pub synonym: Option<String>,
    pub attributes: Vec<Attribute>,
}

/// Conditional type that depends on runtime conditions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionalType {
    pub condition: String,
    pub then_type: ResolutionResult,
    pub else_type: ResolutionResult,
}

/// Type with context and effects
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextualType {
    pub base_type: ResolutionResult,
    pub effects: Vec<TypeEffect>,
    pub context: ExecutionContext,
}

/// Type effects that modify behavior
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

    /// Create an inferred type resolution
    pub fn inferred(confidence: f32, result: ResolutionResult) -> Self {
        Self {
            certainty: Certainty::Inferred(confidence),
            result,
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Check if the type is fully resolved
    pub fn is_resolved(&self) -> bool {
        matches!(self.certainty, Certainty::Known)
    }

    pub fn get_name(&self) -> Option<String> {
        match &self.result {
            ResolutionResult::Concrete(concrete) => match concrete {
                ConcreteType::Platform(p) => Some(p.name.clone()),
                ConcreteType::Configuration(c) => Some(c.name.clone()),
                ConcreteType::Primitive(p) => Some(p.to_string()),
                ConcreteType::GlobalFunction(f) => Some(f.name.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Create TypeResolution from RawTypeData
    pub fn from_raw_data(raw_data: &crate::architecture::data::RawTypeData) -> Self {
        use crate::architecture::data::TypeSource;
        use crate::core::types::*;

        // Конвертируем методы
        let methods: Vec<Method> = raw_data
            .methods
            .iter()
            .map(|raw_method| {
                let parameters: Vec<Parameter> = raw_method
                    .parameters
                    .iter()
                    .map(|raw_param| {
                        Parameter {
                            name: raw_param.name.clone(),
                            type_: Some(raw_param.type_name.clone()),
                            optional: raw_param.is_optional,
                            by_value: true, // По умолчанию параметры передаются по значению
                        }
                    })
                    .collect();

                Method {
                    name: raw_method.name.clone(),
                    parameters,
                    return_type: raw_method.return_type.clone(),
                    is_function: raw_method.return_type.is_some(),
                }
            })
            .collect();

        // Конвертируем свойства
        let properties: Vec<Property> = raw_data
            .properties
            .iter()
            .map(|raw_prop| Property {
                name: raw_prop.name.clone(),
                type_: raw_prop.type_name.clone(),
                readonly: raw_prop.is_readonly,
            })
            .collect();

        // Определяем тип результата на основе источника
        let result = match &raw_data.source {
            TypeSource::Platform { .. } => {
                ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                    name: raw_data.russian_name.clone(),
                    methods,
                    properties,
                }))
            }
            TypeSource::Configuration { .. } => {
                // Определяем тип конфигурационного объекта по пути категории
                let kind = if raw_data.category_path.contains(&"Справочник".to_string()) {
                    MetadataKind::Catalog
                } else if raw_data.category_path.contains(&"Документ".to_string()) {
                    MetadataKind::Document
                } else {
                    MetadataKind::Catalog // По умолчанию
                };

                let attributes: Vec<Attribute> = raw_data
                    .properties
                    .iter()
                    .map(|raw_prop| Attribute {
                        name: raw_prop.name.clone(),
                        type_: raw_prop.type_name.clone(),
                        is_composite: false,
                        types: vec![raw_prop.type_name.clone()],
                    })
                    .collect();

                ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                    kind,
                    name: raw_data.russian_name.clone(),
                    attributes,
                    tabular_sections: Vec::new(), // TODO: конвертировать табличные части
                }))
            }
            TypeSource::UserDefined { .. } => {
                ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                    name: raw_data.russian_name.clone(),
                    methods,
                    properties,
                }))
            }
        };

        // Определяем источник разрешения
        let source = match &raw_data.source {
            TypeSource::Platform { .. } => ResolutionSource::Static,
            TypeSource::Configuration { .. } => ResolutionSource::Static,
            TypeSource::UserDefined { .. } => ResolutionSource::Static,
        };

        Self {
            certainty: Certainty::Known,
            result,
            source,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: raw_data
                .available_facets
                .iter()
                .map(|facet| facet.kind)
                .collect(),
        }
    }

    /// Convert TypeResolution to RawTypeData
    pub fn to_raw_data(&self) -> crate::architecture::data::RawTypeData {
        use crate::architecture::data::{
            RawMethodData, RawParameterData, RawPropertyData, TypeSource,
        };

        let name = self.get_name().unwrap_or_else(|| "Unknown".to_string());

        let source = match &self.result {
            ResolutionResult::Concrete(ConcreteType::Platform(_)) => TypeSource::Platform {
                version: "8.3".to_string(),
            },
            ResolutionResult::Concrete(ConcreteType::Configuration(_)) => {
                TypeSource::Configuration {
                    config_version: "8.3".to_string(),
                }
            }
            _ => TypeSource::Platform {
                version: "8.3".to_string(),
            },
        };

        let mut methods = Vec::new();
        let mut properties = Vec::new();

        match &self.result {
            ResolutionResult::Concrete(ConcreteType::Platform(platform_type)) => {
                methods = platform_type
                    .methods
                    .iter()
                    .map(|method| {
                        let params: Vec<RawParameterData> = method
                            .parameters
                            .iter()
                            .map(|param| RawParameterData {
                                name: param.name.clone(),
                                type_name: param.type_.clone().unwrap_or_else(String::new),
                                description: String::new(),
                                is_optional: param.optional,
                                is_by_value: param.by_value,
                            })
                            .collect();

                        RawMethodData {
                            name: method.name.clone(),
                            documentation: String::new(),
                            parameters: params.clone(),
                            return_type: method.return_type.clone(),
                            return_type_name: method.return_type.clone(),
                            params,
                            is_function: method.is_function,
                            examples: Vec::new(),
                        }
                    })
                    .collect();

                properties = platform_type
                    .properties
                    .iter()
                    .map(|prop| RawPropertyData {
                        name: prop.name.clone(),
                        type_name: prop.type_.clone(),
                        is_readonly: prop.readonly,
                        description: String::new(),
                    })
                    .collect();
            }
            ResolutionResult::Concrete(ConcreteType::Configuration(config_type)) => {
                properties = config_type
                    .attributes
                    .iter()
                    .map(|attr| RawPropertyData {
                        name: attr.name.clone(),
                        type_name: attr.type_.clone(),
                        is_readonly: false,
                        description: String::new(),
                    })
                    .collect();
            }
            _ => {}
        }

        crate::architecture::data::RawTypeData {
            id: name.clone(),
            russian_name: name.clone(),
            english_name: name.clone(),
            source,
            category_path: vec!["Platform".to_string()],
            methods,
            properties,
            documentation: format!("Тип: {}", name),
            examples: vec![format!("объект = Новый {};", name)],
            available_facets: self
                .available_facets
                .iter()
                .map(|kind| Facet {
                    kind: *kind,
                    methods: Vec::new(),
                    properties: Vec::new(),
                })
                .collect(),
            parse_metadata: crate::architecture::data::raw_models::ParseMetadata {
                file_path: "unknown".to_string(),
                line: 0,
                column: 0,
            },
        }
    }
}

impl GlobalFunction {
    /// Resolve return type for polymorphic functions based on arguments
    pub fn resolve_return_type(&self, args: &[TypeResolution]) -> TypeResolution {
        if !self.polymorphic {
            // For non-polymorphic functions, return the static return type
            return self
                .return_type
                .as_ref()
                .map(|t| (**t).clone())
                .unwrap_or_else(TypeResolution::unknown);
        }

        // Handle polymorphic functions
        match self.name.as_str() {
            "Мин" | "Min" | "Макс" | "Max" => {
                if args.is_empty() {
                    return TypeResolution::unknown();
                }

                // Return type is determined by the first argument
                match &args[0].result {
                    ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number)) => {
                        TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number))
                    }
                    ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)) => {
                        TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String))
                    }
                    ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Date)) => {
                        TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Date))
                    }
                    ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Boolean)) => {
                        TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Boolean))
                    }
                    _ => TypeResolution::inferred(0.5, args[0].result.clone()),
                }
            }
            _ => {
                // For other polymorphic functions, use default behavior
                self.return_type
                    .as_ref()
                    .map(|t| (**t).clone())
                    .unwrap_or_else(TypeResolution::unknown)
            }
        }
    }
}
