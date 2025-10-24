// shared/src/api/dtos.rs

//! Data Transfer Objects (DTOs) for the public API.
//! These structures define the contract between the core analysis engine and any consumer (backend, frontend, etc.).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The main structure representing the complete analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResultDto {
    pub types: Vec<TypeDto>,
    pub categories: HashMap<String, CategoryDto>,
    pub metrics: MetricsDto,
    pub connections: Vec<ConnectionDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationDto>,
}

/// Detailed information about a single type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeDto {
    pub id: String,
    pub name: String,
    pub category: String,
    pub certainty: u8,
    pub certainty_text: String,
    pub facets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub methods_count: Option<usize>,
    /// Full method signatures with parameters and return types (Phase 2: Breaking change)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<MethodDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<String>,
    /// Enum values for platform enumeration types (e.g., "Авто (Auto)", "НеИспользовать (DontUse)")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    /// Табличные части (для документов, справочников)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tabular_sections: Vec<TabularSectionDto>,
    
    pub source: String,
    pub flow_sensitive: bool,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub union_types: Option<Vec<UnionComponentDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_analysis: Option<FlowAnalysisDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections: Option<TypeConnectionsDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
}

/// Табличная часть конфигурационного объекта
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabularSectionDto {
    /// Имя табличной части (например, "Работы", "Стороны")
    pub name: String,
    /// Атрибуты табличной части
    pub attributes: Vec<TabularSectionAttributeDto>,
}

/// Атрибут табличной части
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabularSectionAttributeDto {
    /// Имя атрибута (например, "ВидРаботы", "Сторона")
    pub name: String,
    /// Тип атрибута (например, "xs:string", "ПланВидовХарактеристикСсылка.ВидыРабот")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attr_type: Option<String>,
}

/// A component of a union type with its probability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnionComponentDto {
    #[serde(rename = "type")]
    pub type_name: String,
    pub probability: u8,
}

/// Represents the state of a type through flow analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowAnalysisDto {
    pub init: String,
    pub check: String,
    #[serde(rename = "final")]
    pub final_state: String, // 'final' is a reserved keyword
}

/// Represents incoming and outgoing connections for a type node in a graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeConnectionsDto {
    pub incoming: usize,
    pub outgoing: usize,
}

/// Visual and statistical information about a type category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDto {
    pub color: String,
    pub icon: String,
    pub count: usize,
}

/// General metrics about the analysis process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsDto {
    pub total_types: usize,
    pub certainty_high: usize,
    pub certainty_medium: usize,
    pub certainty_low: usize,
    pub flow_sensitive: usize,
    pub cache_hit_rate: String,
    pub analysis_speed: String,
}

/// Represents a single connection between two types in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionDto {
    pub source: String, // id of the source type
    pub target: String, // id of the target type
    #[serde(rename = "type")]
    pub connection_type: String,
}

/// Pagination information for paged responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationDto {
    pub current_page: usize,
    pub page_size: usize,
    pub total_items: usize,
    pub total_pages: usize,
    pub has_prev: bool,
    pub has_next: bool,
}

// ============================================================================
// Validation DTOs
// ============================================================================

/// Request for code validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateCodeRequest {
    /// Code fragment to validate (e.g., "массив.Добавить()", "таблица.НесуществующийМетод()")
    pub code: String,
    /// Optional file path for context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

/// Response with validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateCodeResponse {
    /// Whether the code is valid (no errors)
    pub is_valid: bool,
    /// List of validation errors found
    pub errors: Vec<ValidationErrorDto>,
    /// Metadata about validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ValidationMetadataDto>,
}

/// A single validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationErrorDto {
    /// Error message in Russian
    pub message: String,
    /// Error severity: "error", "warning", "info"
    pub severity: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
    /// Type of error: "NonExistentMethod", "NonExistentProperty", etc.
    pub error_type: String,
}

/// Metadata about the validation process
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationMetadataDto {
    /// Number of expressions analyzed
    pub expressions_analyzed: usize,
    /// Number of types resolved
    pub types_resolved: usize,
    /// Time taken for validation (milliseconds)
    pub duration_ms: u64,
}

// ============================================================================
// Method Signature DTOs (Phase 2: Breaking change)
// ============================================================================

/// Full method signature with parameters and return type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodDto {
    /// Method name in Russian (e.g., "Добавить")
    pub name: String,

    /// English method name if available (e.g., "Add")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub english_name: Option<String>,

    /// Return type of the method (e.g., "Строка", "Число")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,

    /// Method parameters with detailed information
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<ParamDto>,

    /// Method description/documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this method is deprecated
    #[serde(default)]
    pub is_deprecated: bool,

    /// Whether this is a constructor method
    #[serde(default)]
    pub is_constructor: bool,
}

/// Method parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamDto {
    /// Parameter name (e.g., "Значение", "Ключ")
    pub name: String,

    /// Parameter type (e.g., "Произвольный", "Строка")
    pub param_type: String,

    /// Whether parameter is optional
    #[serde(default)]
    pub is_optional: bool,

    /// Default value if parameter is optional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
}

/// Property information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDto {
    /// Property name (e.g., "Код", "Наименование")
    pub name: String,

    /// Property type (e.g., "Строка", "Число")
    pub prop_type: String,

    /// Whether property is read-only
    #[serde(default)]
    pub is_readonly: bool,

    /// Property description/documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
