// shared/src/api/dtos.rs

//! Data Transfer Objects (DTOs) for the public API.
//! These structures define the contract between the core analysis engine and any consumer (backend, frontend, etc.).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use bsl_repository::RepositoryStats;

/// Backend mode for unified UI (Web Server vs MCP Agent).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum McpBackendModeDto {
    McpAgent,
    WebServer,
}

/// Capability detection for the unified SPA (target/site).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatusDto {
    /// Backend mode the SPA is connected to.
    pub mode: McpBackendModeDto,
    /// Whether MCP dashboard endpoints are supported.
    pub supported: bool,
    /// Whether the MCP dashboard is strictly read-only.
    pub read_only: bool,
    /// Optional instance identifier (useful for per-agent mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    /// Optional UI URL (useful when server binds to :0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_url: Option<String>,
    /// Optional cache directory used by this instance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<String>,
}

/// MCP session summary for read-only dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSessionDto {
    pub session_id: String,
    pub roots: Vec<McpRootDto>,
    pub ready: bool,
    pub analysis_revision: u64,
    pub phase: String,
    pub progress_percent: u8,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub missing_inputs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Root for MCP session summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRootDto {
    pub root_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSessionsResponseDto {
    pub sessions: Vec<McpSessionDto>,
}

/// MCP job summary for read-only dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpJobDto {
    pub job_id: String,
    pub state: String,
    pub phase: String,
    pub progress_percent: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpJobsResponseDto {
    pub jobs: Vec<McpJobDto>,
}

/// Прогресс запуска/инициализации системы (Web API polling).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupProgressDto {
    /// Стадия (человекочитаемая).
    pub phase: String,
    /// Текущий прогресс в рамках стадии.
    pub current: u64,
    /// Всего элементов в рамках стадии.
    pub total: u64,
    /// Общий процент (0..100), монотонный.
    pub percentage: f32,
    /// Дополнительное сообщение (например, имя файла/объекта).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Признак завершения старта.
    pub done: bool,
}

impl Default for StartupProgressDto {
    fn default() -> Self {
        Self {
            phase: "Инициализация".to_string(),
            current: 0,
            total: 0,
            percentage: 0.0,
            message: None,
            done: false,
        }
    }
}

/// Normalized startup inputs that affect deps/config/index on startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInputsDto {
    pub syntax_helper_path: Option<String>,
    pub configuration_path: Option<String>,
    pub platform_version: Option<String>,
    #[serde(default)]
    pub rules_config_path: Option<String>,
    pub cache_enabled: bool,
    pub strict_fingerprint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GlobalContextDocsStatusDto {
    pub state: String,
    pub property_count: usize,
    pub fingerprint: String,
    #[serde(default)]
    pub degraded_reason: Option<String>,
}

/// Metadata about the currently loaded deps snapshot (Web UI parity diagnostics).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMetaDto {
    pub deps_id: String,
    pub index_snapshot_id: String,
    pub platform_version: String,
    pub platform_fingerprint: Option<String>,
    pub config_fingerprint: Option<String>,
    pub strict_fingerprint: bool,
    #[serde(default)]
    pub global_context: GlobalContextDocsStatusDto,
    pub repository_stats: RepositoryStats,
    pub inputs: SnapshotInputsDto,
}

pub const SNAPSHOT_READINESS_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotReadinessStateDto {
    Idle,
    Building,
    Ready,
    Stale,
    ShadowOnly,
    Failed,
}

impl SnapshotReadinessStateDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Building => "building",
            Self::Ready => "ready",
            Self::Stale => "stale",
            Self::ShadowOnly => "shadow_only",
            Self::Failed => "failed",
        }
    }
}

impl fmt::Display for SnapshotReadinessStateDto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotTaskStateDto {
    Absent,
    InFlightSameRevision,
    InFlightOtherRevision,
    ReadySameRevision,
    ReadyStaleRevision,
    NotApplicable,
}

impl SnapshotTaskStateDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::InFlightSameRevision => "in_flight_same_revision",
            Self::InFlightOtherRevision => "in_flight_other_revision",
            Self::ReadySameRevision => "ready_same_revision",
            Self::ReadyStaleRevision => "ready_stale_revision",
            Self::NotApplicable => "not_applicable",
        }
    }
}

impl fmt::Display for SnapshotTaskStateDto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotPhaseDto {
    Waiting,
    Parsing,
    Materializing,
}

impl SnapshotPhaseDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Parsing => "parsing",
            Self::Materializing => "materializing",
        }
    }
}

impl fmt::Display for SnapshotPhaseDto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotTriggerDto {
    DidOpen,
    DidChange,
    DidSave,
    CurrentContext,
    DocumentsSet,
    Job,
}

impl SnapshotTriggerDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DidOpen => "did_open",
            Self::DidChange => "did_change",
            Self::DidSave => "did_save",
            Self::CurrentContext => "current_context",
            Self::DocumentsSet => "documents_set",
            Self::Job => "job",
        }
    }
}

impl fmt::Display for SnapshotTriggerDto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotArtifactStateDto {
    Unknown,
    Missing,
    Building,
    Ready,
    Stale,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotFailureStageDto {
    SnapshotBuild,
    ReadyParseSnapshot,
    ExactTypeIndex,
    CompletionHead,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotRecommendationDto {
    Wait,
    Refresh,
    PrimeExactIndex,
    OpenTimeline,
    ExportIncidentBundle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotStatusReasonDto {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotArtifactStatusDto {
    pub state: SnapshotArtifactStateDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotArtifactsDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow_state: Option<SnapshotArtifactStatusDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_parse_snapshot: Option<SnapshotArtifactStatusDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_type_index: Option<SnapshotArtifactStatusDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_head: Option<SnapshotArtifactStatusDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotWorkerDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<SnapshotPhaseDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<SnapshotTriggerDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancellation_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by_version: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotLastFailureDto {
    pub stage: SnapshotFailureStageDto,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurred_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotReadinessDto {
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis_revision: Option<u64>,
    pub state: SnapshotReadinessStateDto,
    pub exact: bool,
    pub task_state: SnapshotTaskStateDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<SnapshotPhaseDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<SnapshotTriggerDto>,
    pub updated_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<SnapshotStatusReasonDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<SnapshotArtifactsDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker: Option<SnapshotWorkerDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_failure: Option<SnapshotLastFailureDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<SnapshotRecommendationDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpSnapshotStatusResponseDto {
    pub schema_version: u32,
    pub entries: Vec<SnapshotReadinessDto>,
}

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

    /// Enable flow-sensitive analysis (opt-in).
    ///
    /// Default: false.
    #[serde(default)]
    pub include_flow_sensitive: bool,

    /// Legacy field: `include_flow_sensitive` (snake_case) is explicitly rejected by adapters.
    #[serde(default, rename = "include_flow_sensitive", skip_serializing)]
    pub legacy_include_flow_sensitive: Option<bool>,
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
    /// Start line number (1-indexed)
    pub line: u32,
    /// Start column number (1-indexed)
    pub column: u32,
    /// End line number (1-indexed)
    pub end_line: u32,
    /// End column number (1-indexed)
    pub end_column: u32,
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

// ============================================================================
// Enhanced Hover Response (Milestone 2.13)
// ============================================================================

/// Request for hover information
#[derive(Debug, Clone, Deserialize)]
pub struct HoverInfoRequest {
    /// BSL code fragment
    pub code: String,
    /// Line number (0-based)
    pub line: u32,
    /// Column number (0-based)
    pub column: u32,
}

/// Enhanced hover response with detailed variable information
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedHoverResponse {
    /// Formatted hover text with type information
    pub hover_text: String,

    /// Variable/symbol name at position
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_name: Option<String>,

    /// Inferred or explicit type name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_type: Option<String>,

    /// Type hint certainty level (e.g., "Explicit", "Inferred", "Unknown")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_hint: Option<String>,

    /// Whether variable was found in scope
    pub found_in_scope: bool,

    /// Requested line number
    pub line: u32,

    /// Requested column number
    pub column: u32,

    /// Analysis duration in milliseconds
    pub duration_ms: u128,
}

// ============================================================================
// Diagnostics Response (Milestone 2.18)
// ============================================================================

/// Syntax error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxErrorDto {
    /// Error message
    pub message: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
}

/// Semantic error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticErrorDto {
    /// Error message
    pub message: String,
    /// Start line number (1-indexed)
    pub line: u32,
    /// Start column number (1-indexed)
    pub column: u32,
    /// End line number (1-indexed)
    pub end_line: u32,
    /// End column number (1-indexed)
    pub end_column: u32,
    /// Error severity: "error", "warning", "info"
    pub severity: String,
}

/// Diagnostics response separating syntax and semantic errors
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsResponseDto {
    /// Syntax errors found during parsing
    pub syntax_errors: Vec<SyntaxErrorDto>,

    /// Semantic errors found during analysis
    pub semantic_errors: Vec<SemanticErrorDto>,

    /// Total count of all errors
    pub total_errors: usize,

    /// Analysis duration in milliseconds
    pub duration_ms: u128,
}

// ============================================================================
// Debug AST Response (Milestone 2.16)
// ============================================================================

/// Node information in the AST
#[derive(Debug, Clone, Serialize)]
pub struct AstNodeDto {
    /// Node kind (e.g., "FunctionDecl", "VariableRef", "MethodCall")
    pub kind: String,

    /// Start line (1-indexed)
    pub start_line: u32,

    /// Start column (1-indexed)
    pub start_column: u32,

    /// End line (1-indexed)
    pub end_line: u32,

    /// End column (1-indexed)
    pub end_column: u32,

    /// Optional text content for terminals
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Symbol table entry for variables
#[derive(Debug, Clone, Serialize)]
pub struct SymbolTableEntryDto {
    /// Variable name
    pub name: String,

    /// Inferred or declared type
    pub type_hint: String,

    /// Line where symbol is declared
    pub declared_line: u32,

    /// Scope level (0 = global, 1+ = nested)
    pub scope_level: u32,
}

/// Debug AST response for code analysis
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugAstResponseDto {
    /// AST nodes
    pub nodes: Vec<AstNodeDto>,

    /// Symbol table with variables
    pub symbol_table: Vec<SymbolTableEntryDto>,

    /// Number of parsing errors
    pub parse_errors: usize,

    /// Analysis duration in milliseconds
    pub duration_ms: u128,
}
