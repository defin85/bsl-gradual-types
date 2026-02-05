use schemars::JsonSchema;
use serde::{de, Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fmt;

fn default_true() -> bool {
    true
}

fn default_limit_200() -> u32 {
    200
}

fn default_limit_50() -> u32 {
    50
}

fn default_page_1() -> u32 {
    1
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
pub struct WorkspaceGetSettingsParams {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceUpdateSettingsParams {
    pub session_id: String,
    /// Patch stable runtime overrides (null removes a key).
    #[serde(default)]
    pub env_overrides: Option<HashMap<String, JsonValue>>,
    /// Patch dev-only runtime overrides (null removes a key). Applied only when allow_dev_overrides=true.
    #[serde(default)]
    pub dev_env_overrides: Option<HashMap<String, JsonValue>>,
    /// Gate dev-only overrides. When false, dev_env_overrides are ignored.
    #[serde(default)]
    pub allow_dev_overrides: Option<bool>,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct UiUrlParams {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct BuildInfoParams {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct McpHelpParams {
    #[serde(default)]
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CanonicalDocumentRef {
    pub root_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PathDocumentRef {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DocumentRef {
    Canonical(CanonicalDocumentRef),
    PathObject(PathDocumentRef),
    Path(String),
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
#[serde(untagged)]
pub enum WorkspaceDocumentsSetFile {
    File(FileRef),
    Document(DocumentRef),
    Path(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceDocumentsSetParams {
    pub session_id: String,
    pub files: Vec<WorkspaceDocumentsSetFile>,
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
pub enum WorkspaceScopeTagged {
    Project,
    Hot,
    File { document: DocumentRef },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum WorkspaceScope {
    Tagged(WorkspaceScopeTagged),
    Simple(String),
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
    /// Opt-in: include flow-sensitive rules (default: false).
    #[serde(default)]
    pub include_flow_sensitive: bool,
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
    /// Opt-in: include flow-sensitive type narrowing (default: false).
    #[serde(default)]
    pub include_flow_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BslMembersParams {
    pub session_id: String,
    pub file: FileRef,
    pub position: Position,
    #[serde(default = "default_limit_200")]
    pub limit: u32,
    /// Opt-in: include flow-sensitive type narrowing (default: false).
    #[serde(default)]
    pub include_flow_sensitive: bool,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BslTypeSource {
    Platform,
    Configuration,
}

#[derive(Debug, Clone, Copy, Serialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum BslTypesView {
    NamesOnly,
    #[default]
    Summary,
    Full,
}

impl<'de> Deserialize<'de> for BslTypesView {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ViewVisitor;

        impl<'de> de::Visitor<'de> for ViewVisitor {
            type Value = BslTypesView;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("one of: names_only, summary, full")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    "names_only" => Ok(BslTypesView::NamesOnly),
                    "summary" => Ok(BslTypesView::Summary),
                    "full" => Ok(BslTypesView::Full),
                    _ => Err(E::custom("view must be one of: names_only, summary, full")),
                }
            }
        }

        deserializer.deserialize_str(ViewVisitor)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BslTypesListParams {
    pub session_id: String,
    #[serde(default = "default_page_1")]
    pub page: u32,
    #[serde(default = "default_limit_50")]
    pub limit: u32,
    #[serde(default)]
    pub source: Option<BslTypeSource>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub certainty_level: Option<u8>,
    #[serde(default)]
    pub flow_sensitive_only: bool,
    #[serde(default)]
    pub view: BslTypesView,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BslTypesSearchParams {
    pub session_id: String,
    pub query: String,
    #[serde(default = "default_limit_200")]
    pub limit: u32,
    #[serde(default)]
    pub source: Option<BslTypeSource>,
    #[serde(default)]
    pub view: BslTypesView,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BslTypeGetParams {
    pub session_id: String,
    pub type_name: String,
    #[serde(default)]
    pub source: Option<BslTypeSource>,
    #[serde(default)]
    pub include_methods: bool,
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
