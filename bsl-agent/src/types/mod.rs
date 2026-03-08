use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use bsl_runtime::system::runtime_config::ApplyOverridesReport;

use crate::semantic::dto::{DiagnosticDto, DocumentRefDto, RangeDto};

#[derive(Debug, thiserror::Error)]
pub enum BslAgentError {
    #[error("invalid params: {0}")]
    InvalidParams(String),
    #[error("{0}")]
    Other(String),
}

impl BslAgentError {
    pub fn into_rmcp(self) -> rmcp::ErrorData {
        match self {
            BslAgentError::InvalidParams(msg) => rmcp::ErrorData::invalid_params(msg, None),
            BslAgentError::Other(msg) => rmcp::ErrorData::internal_error(msg, None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootDto {
    pub root_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceOpenResponse {
    pub session_id: String,
    pub roots: Vec<RootDto>,
    pub analysis_revision: u64,
    pub ready: bool,
    #[serde(default)]
    pub startup_job_id: Option<String>,
    pub warnings: Vec<String>,
    pub missing_inputs: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProgressDto {
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStatusResponse {
    pub ready: bool,
    pub analysis_revision: u64,
    pub phase: String,
    pub progress: ProgressDto,
    pub warnings: Vec<String>,
    pub missing_inputs: Vec<String>,
    #[serde(default)]
    pub startup_job_id: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSettingsReportDto {
    #[serde(rename = "ignoredUnknownKeys")]
    pub ignored_unknown_keys: Vec<String>,
    #[serde(rename = "ignoredInvalidValues")]
    pub ignored_invalid_values: Vec<String>,
    #[serde(rename = "ignoredWrongTierKeys")]
    pub ignored_wrong_tier_keys: Vec<String>,
    #[serde(rename = "devOverridesIgnored")]
    pub dev_overrides_ignored: bool,
    #[serde(rename = "requiresRestartKeys")]
    pub requires_restart_keys: Vec<String>,
}

impl From<ApplyOverridesReport> for RuntimeSettingsReportDto {
    fn from(value: ApplyOverridesReport) -> Self {
        Self {
            ignored_unknown_keys: value.ignored_unknown_keys,
            ignored_invalid_values: value.ignored_invalid_values,
            ignored_wrong_tier_keys: value.ignored_wrong_tier_keys,
            dev_overrides_ignored: value.dev_overrides_ignored,
            requires_restart_keys: value.requires_restart_keys,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceGetSettingsResponse {
    pub session_id: String,
    #[serde(rename = "allowDevOverrides")]
    pub allow_dev_overrides: bool,
    #[serde(rename = "envOverrides")]
    pub env_overrides: HashMap<String, JsonValue>,
    #[serde(rename = "devEnvOverrides")]
    pub dev_env_overrides: HashMap<String, JsonValue>,
    #[serde(rename = "runtimeConfig")]
    pub runtime_config: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceUpdateSettingsResponse {
    pub ok: bool,
    pub session_id: String,
    #[serde(rename = "allowDevOverrides")]
    pub allow_dev_overrides: bool,
    #[serde(rename = "envOverrides")]
    pub env_overrides: HashMap<String, JsonValue>,
    #[serde(rename = "devEnvOverrides")]
    pub dev_env_overrides: HashMap<String, JsonValue>,
    pub report: RuntimeSettingsReportDto,
    #[serde(rename = "runtimeConfig")]
    pub runtime_config: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceObservabilityMetricsResponse {
    pub metrics: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceListItemDto {
    pub session_id: String,
    pub roots: Vec<String>,
    pub analysis_revision: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceListResponse {
    pub sessions: Vec<WorkspaceListItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiUrlResponse {
    pub enabled: bool,
    #[serde(default)]
    pub ui_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfoResponse {
    pub package: String,
    pub version: String,
    pub profile: String,
    pub target: String,
    #[serde(default)]
    pub git_sha: Option<String>,
    #[serde(default)]
    pub git_describe: Option<String>,
    #[serde(default)]
    pub build_unix_secs: Option<u64>,
    pub pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpHelpResponse {
    pub summary: String,
    pub quickstart: Vec<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub examples: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStateDto {
    Queued,
    Running,
    Succeeded,
    Failed,
    Canceled,
    AbortedByRestart,
}

impl JobStateDto {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStateDto::Queued => "queued",
            JobStateDto::Running => "running",
            JobStateDto::Succeeded => "succeeded",
            JobStateDto::Failed => "failed",
            JobStateDto::Canceled => "canceled",
            JobStateDto::AbortedByRestart => "aborted_by_restart",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusResponse {
    pub job_id: String,
    pub state: JobStateDto,
    pub phase: String,
    pub progress: ProgressDto,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCancelResponse {
    pub ok: bool,
    pub job_id: String,
    pub state: JobStateDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStartResponse {
    pub job_id: String,
    #[serde(default)]
    pub recommended_poll_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceDocumentsSetResponse {
    pub ok: bool,
    pub analysis_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceDocumentsClearResponse {
    pub ok: bool,
    pub analysis_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BslDiagnosticsResponse {
    pub analysis_revision: u64,
    pub flow_sensitive_enabled: bool,
    pub diagnostics: Vec<DiagnosticDto>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDto {
    pub symbol_id: String,
    pub name: String,
    pub kind: String,
    pub file: DocumentRefDto,
    pub range: RangeDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BslSymbolSearchResponse {
    pub analysis_revision: u64,
    pub symbols: Vec<SymbolDto>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfoDto {
    pub name: String,
    pub certainty: String,
    #[serde(default)]
    pub active_facet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfoDto {
    pub kind: String,
    pub range: RangeDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BslTypeAtPositionResponse {
    pub analysis_revision: u64,
    pub flow_sensitive_enabled: bool,
    #[serde(default)]
    pub type_info: Option<TypeInfoDto>,
    #[serde(default)]
    pub node: Option<NodeInfoDto>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberDto {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_identity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BslMembersResponse {
    pub analysis_revision: u64,
    pub flow_sensitive_enabled: bool,
    pub members: Vec<MemberDto>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationDto {
    pub file: DocumentRefDto,
    pub range: RangeDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BslDefinitionResponse {
    pub analysis_revision: u64,
    #[serde(default)]
    pub location: Option<LocationDto>,
    #[serde(default)]
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceDto {
    pub file: DocumentRefDto,
    pub range: RangeDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BslReferencesResponse {
    pub analysis_revision: u64,
    pub count: u64,
    pub references: Vec<ReferenceDto>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletenessDto {
    Full,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackItemDto {
    pub item_id: String,
    pub kind: String,
    #[serde(default)]
    pub file: Option<DocumentRefDto>,
    #[serde(default)]
    pub range: Option<RangeDto>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackResponse {
    pub analysis_revision: u64,
    pub pack_id: String,
    pub text: String,
    pub items: Vec<ContextPackItemDto>,
    pub truncated: bool,
    pub completeness: CompletenessDto,
    pub missing_inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextExpandResponse {
    pub analysis_revision: u64,
    pub text: String,
    pub truncated: bool,
}
