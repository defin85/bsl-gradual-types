//! Raw data structures for type parsing
//!
//! This module contains structures for storing raw type data from parsers:
//! - `RawTypeData`: Complete type information from all parsers
//! - `RawMethodData`, `RawPropertyData`, `RawParamData`: Method and property info
//! - `RawAttributeData`, `RawTabularSectionData`: Configuration object attributes
//! - `GenericInfo`, `InferenceMethodInfo`: Generic type metadata

use serde::{Deserialize, Serialize};

use super::facets::FacetKind;
use super::metadata::MetadataKind;

/// Information about generic parameters of a collection type
///
/// # Examples
/// - `Array<T>` - 1 parameter (element)
/// - `Map<K,V>` - 2 parameters (key, value)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericInfo {
    /// Base type (e.g., "Массив")
    pub base_type: String,

    /// Number of type parameters (1 for Array, 2 for Map)
    pub type_param_count: usize,

    /// Methods that allow inferring the type parameter
    pub inference_methods: Vec<InferenceMethodInfo>,
}

/// Information about a method for generic type inference
///
/// # Examples
/// - `Array.Add(Value: T)` - parameter 0 determines T
/// - `Map.Insert(Key: K, Value: V)` - parameters 0 and 1 determine K and V
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceMethodInfo {
    /// Method name (e.g., "Добавить")
    pub method_name: String,

    /// Indices of method parameters that determine generic types
    /// For Array.Add(Value) - param_indices = [0]
    /// For Map.Insert(Key, Value) - param_indices = [0, 1]
    pub param_indices: Vec<usize>,

    /// Which generic parameters are inferred (0 for T in Array<T>, 0 and 1 for K and V in Map<K,V>)
    pub inferred_type_params: Vec<usize>,
}

/// Source of raw type data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RawDataSource {
    #[default]
    Platform,
    Configuration,
    UserDefined,
}

/// Complete raw type data from parsers
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawTypeData {
    /// Type name (Russian)
    pub name: String,
    /// Type name (English)
    pub english_name: String,
    /// Type description
    pub description: String,
    /// Type category
    pub category: String,
    /// Data source
    pub source: RawDataSource,
    /// Type methods
    pub methods: Vec<RawMethodData>,
    /// Type properties
    pub properties: Vec<RawPropertyData>,
    /// Available facets
    pub facets: Vec<FacetKind>,
    /// Metadata kind (for configuration types)
    pub kind: Option<MetadataKind>,
    /// Object attributes
    pub attributes: Vec<RawAttributeData>,
    /// Tabular sections
    pub tabular_sections: Vec<RawTabularSectionData>,
    /// Enum values for platform enumeration types (e.g., "Авто (Auto)", "НеИспользовать (DontUse)")
    pub enum_values: Vec<String>,
    /// Generic metadata for collection types (Array<T>, Map<K,V>, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generic_info: Option<GenericInfo>,
    /// Тип элемента коллекции (если применимо).
    ///
    /// Заполняется из Syntax Helper для типов-коллекций (например, `ДанныеФормыКоллекция`),
    /// чтобы корректно параметризовать возвращаемые типы методов вида `Получить()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_item_type: Option<String>,
    /// Module paths for configuration types (Milestone 3.14)
    /// Used for Go To Definition navigation to ObjectModule.bsl, ManagerModule.bsl, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_paths: Option<crate::domain::type_definition_location::ModulePaths>,
}

/// Raw method data from parser
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawMethodData {
    /// Method name (Russian)
    pub name: String,
    /// Method name (English)
    pub english_name: String,
    /// Return type
    pub return_type: String,
    /// Method parameters
    pub params: Vec<RawParamData>,
    /// Method description
    #[serde(default)]
    pub description: Option<String>,
    /// Whether method is deprecated
    #[serde(default)]
    pub is_deprecated: bool,
    /// Whether method is a constructor
    #[serde(default)]
    pub is_constructor: bool,
    /// MILESTONE 3.11 Phase 4: Context requirements
    #[serde(default)]
    pub context_requirements: Option<crate::domain::runtime_context::ContextRequirements>,
    /// MILESTONE 3.11 Phase 4: Return type facet
    #[serde(default)]
    pub return_facet: Option<FacetKind>,
}

/// Raw property data from parser
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawPropertyData {
    /// Property name
    pub name: String,
    /// Property type
    pub prop_type: String,
    /// Whether property is read-only
    pub is_readonly: bool,
}

/// Raw parameter data from parser
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawParamData {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: String,
    /// Whether parameter is optional
    pub is_optional: bool,
    /// Default value
    #[serde(default)]
    pub default_value: Option<String>,
}

/// Raw attribute data from parser
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawAttributeData {
    /// Attribute name
    pub name: String,
    /// Attribute type
    pub attr_type: String,
}

/// Raw tabular section data from parser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTabularSectionData {
    /// Tabular section name
    pub name: String,
    /// Tabular section attributes
    pub attributes: Vec<RawAttributeData>,
}
