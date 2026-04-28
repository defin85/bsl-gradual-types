use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use bsl_analysis_v2::{FileId, SettingsId};
use bsl_runtime::application::type_system::web_api_service;
use bsl_runtime::application::{
    CancellationPolicy, DiagnosticsDisposition, DiagnosticsProfile, DiagnosticsTrigger,
    ExecutionContext, ExecutionSettings, IntellisenseV2Facade, ObservabilityOrigin,
    ObservabilityStage, PreparedOperationSnapshot, SemanticOperation,
};
use bsl_runtime::data::loaders::progress::ProgressUpdate;
use bsl_runtime::data::loaders::ConfigurationDiscovery;
use bsl_runtime::system::runtime_config::{global_runtime_config, RuntimeKey};
use bsl_shared::api::dtos::{
    AnalysisResultDto, GlobalContextDocsStatusDto, McpRootDto, McpSessionDto,
    McpSnapshotStatusResponseDto, MetricsDto, SnapshotInputsDto, SnapshotMetaDto,
    SnapshotReadinessDto, SnapshotReadinessStateDto, SnapshotTaskStateDto, SnapshotTriggerDto,
    SNAPSHOT_READINESS_SCHEMA_VERSION,
};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{Certainty, ResolutionResult};
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::formatting::{user_facing_resolution_type_name, DetailLevel};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::jobs::{JobContext, JobManager};
use crate::semantic::dto::{DocumentRefDto, PositionDto, RangeDto};
use crate::semantic::facade::SemanticFacade;
use crate::semantic::ids;
use crate::semantic::sort;
use crate::server::types::{
    BslDefinitionParams, BslDiagnosticsParams, BslMembersParams, BslReferencesParams,
    BslSymbolSearchParams, BslTypeAtPositionParams, BslTypeGetParams, BslTypeSource,
    BslTypesListParams, BslTypesSearchParams, BslTypesView, CanonicalDocumentRef,
    ContextExpandParams, ContextFocus, ContextPackParams, DocumentRef, FileRef,
    WorkspaceDocumentsSetFile, WorkspaceOpenParams, WorkspaceScope, WorkspaceScopeTagged,
};
use crate::types::{
    BslDefinitionResponse, BslDiagnosticsResponse, BslMembersResponse, BslReferencesResponse,
    BslSymbolSearchResponse, BslTypeAtPositionResponse, CompletenessDto, ContextExpandResponse,
    ContextPackItemDto, ContextPackResponse, LocationDto, MemberDto, NodeInfoDto, ProgressDto,
    ReferenceDto, RootDto, RuntimeSettingsReportDto, SymbolDto, TypeInfoDto,
    WorkspaceDocumentsClearResponse, WorkspaceDocumentsSetResponse, WorkspaceGetSettingsResponse,
    WorkspaceListItemDto, WorkspaceListResponse, WorkspaceObservabilityMetricsResponse,
    WorkspaceOpenResponse, WorkspaceStatusResponse, WorkspaceUpdateSettingsResponse,
};

const MAX_OVERLAY_BYTES: usize = 2 * 1024 * 1024;
const MAX_DISK_FILE_BYTES: u64 = MAX_OVERLAY_BYTES as u64;
const MAX_TOTAL_READ_BYTES: u64 = 8 * 1024 * 1024;
const DEFAULT_BUDGET_CHARS: usize = 7000;
const CHARS_PER_TOKEN: usize = 4;
const PACK_SNIPPET_CONTEXT_LINES: u32 = 20;
const EXPAND_SNIPPET_CONTEXT_LINES: u32 = 80;

mod store;
use store::{PersistedSession, SessionStore};

const SINGLE_SESSION_ERROR: &str = "only one session is allowed; close the existing session first";

pub struct SessionManager {
    sessions: RwLock<HashMap<Uuid, WorkspaceSession>>,
    store: Option<SessionStore>,
}

struct WorkspaceSession {
    roots: Vec<RootEntry>,
    documents: DocumentStore,
    analysis_revision: u64,
    settings: WorkspaceSettings,
    startup: Option<bsl_runtime::system::StartupResultV2>,
    startup_job_id: Option<String>,
    startup_phase: String,
    startup_progress: u8,
    startup_error: Option<String>,
    created_at: u64,
    id_map: IdMap,
    pack_store: PackStore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RootEntry {
    root_id: String,
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
struct WorkspaceSettings {
    platform_docs_archive: Option<PathBuf>,
    platform_version: Option<String>,
    configuration_path: Option<PathBuf>,
    mode: Option<String>,
    env_overrides: HashMap<String, serde_json::Value>,
    dev_env_overrides: HashMap<String, serde_json::Value>,
    allow_dev_overrides: bool,
}

#[derive(Default)]
struct DocumentStore {
    overlays: HashMap<DocumentKey, DocumentOverlay>,
    hot_set: HashSet<DocumentKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DocumentKey {
    root_id: String,
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DocumentOverlay {
    text: String,
    version: u64,
}

#[derive(Default)]
struct IdMap {
    analysis_revision: u64,
    symbols: HashMap<String, StoredSymbol>,
}

#[derive(Debug, Clone)]
struct StoredSymbol {
    name: String,
    kind: String,
    file: DocumentRefDto,
    range: RangeDto,
}

#[derive(Default)]
struct PackStore {
    analysis_revision: u64,
    packs: HashMap<String, StoredPack>,
}

#[derive(Debug, Clone)]
struct StoredPack {
    items: HashMap<String, StoredPackItem>,
}

#[derive(Debug, Clone)]
enum StoredPackItem {
    Snippet {
        file: DocumentRefDto,
        center_line: u32,
    },
}

include!("manager_session.rs");
include!("manager_documents.rs");
include!("manager_semantic_core.rs");
include!("manager_semantic_navigation.rs");
include!("manager_context.rs");

include!("helpers_core.rs");
include!("helpers_fs.rs");
include!("helpers_progress.rs");
include!("helpers_semantic.rs");

#[cfg(test)]
mod tests;
