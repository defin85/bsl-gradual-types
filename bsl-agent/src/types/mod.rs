use serde::{Deserialize, Serialize};

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
    pub warnings: Vec<String>,
    pub missing_inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BslMembersResponse {
    pub analysis_revision: u64,
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
