use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_limit_200() -> u32 {
    200
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceOpenParams {
    pub roots: Vec<String>,
    #[serde(default)]
    pub platform_docs_archive: Option<String>,
    #[serde(default)]
    pub platform_version: Option<String>,
    #[serde(default)]
    pub configuration_path: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceStatusParams {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceCloseParams {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceResumeParams {
    pub session_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceListParams {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DocumentRef {
    pub root_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileRef {
    pub doc: DocumentRef,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub version: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceDocumentsSetParams {
    pub session_id: String,
    pub files: Vec<FileRef>,
    #[serde(default = "default_true")]
    pub mark_hot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceDocumentsClearParams {
    pub session_id: String,
    pub documents: Vec<DocumentRef>,
    #[serde(default = "default_true")]
    pub clear_hot: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceScope {
    Project,
    Hot,
    File { document: DocumentRef },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BslDiagnosticsParams {
    pub session_id: String,
    pub scope: WorkspaceScope,
    #[serde(default = "default_limit_200")]
    pub limit: u32,
    #[serde(default)]
    pub include_impact: bool,
    #[serde(default)]
    pub include_coverage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BslSymbolSearchParams {
    pub session_id: String,
    pub query: String,
    #[serde(default = "default_limit_200")]
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BslTypeAtPositionParams {
    pub session_id: String,
    pub file: FileRef,
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BslMembersParams {
    pub session_id: String,
    pub file: FileRef,
    pub position: Position,
    #[serde(default = "default_limit_200")]
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BslDefinitionParams {
    pub session_id: String,
    #[serde(default)]
    pub symbol_id: Option<String>,
    #[serde(default)]
    pub file: Option<FileRef>,
    #[serde(default)]
    pub position: Option<Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BslReferencesParams {
    pub session_id: String,
    pub symbol_id: String,
    #[serde(default = "default_limit_200")]
    pub limit: u32,
    #[serde(default)]
    pub include_snippets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContextFocus {
    Diagnostic { diagnostic_id: String },
    Symbol { symbol_id: String },
    Position { file: FileRef, position: Position },
    Query { query: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContextInclude {
    pub snippets: bool,
    pub diagnostics: bool,
    pub types: bool,
    pub members: bool,
    pub references: bool,
    pub symbols: bool,
}

impl Default for ContextInclude {
    fn default() -> Self {
        Self {
            snippets: true,
            diagnostics: true,
            types: true,
            members: true,
            references: true,
            symbols: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContextPackParams {
    pub session_id: String,
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default)]
    pub focus: Option<ContextFocus>,
    #[serde(default)]
    pub scope: Option<WorkspaceScope>,
    #[serde(default)]
    pub budget_chars: Option<u32>,
    #[serde(default)]
    pub budget_tokens: Option<u32>,
    #[serde(default)]
    pub include: ContextInclude,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContextExpandParams {
    pub session_id: String,
    pub pack_id: String,
    pub item_id: String,
    #[serde(default)]
    pub budget_chars: Option<u32>,
    #[serde(default)]
    pub budget_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JobStatusParams {
    pub job_id: String,
}

fn default_timeout_ms() -> u64 {
    5_000
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JobWaitParams {
    pub job_id: String,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JobResultParams {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JobCancelParams {
    pub job_id: String,
}
