use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use bsl_analysis_v2::{AnalysisHostV2, Change, FileId};
use bsl_backend::data::loaders::progress::ProgressUpdate;
use bsl_shared::api::dtos::{McpRootDto, McpSessionDto, SnapshotInputsDto, SnapshotMetaDto};
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::TypeResolver;
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
    BslSymbolSearchParams, BslTypeAtPositionParams, ContextExpandParams, ContextFocus,
    ContextPackParams, DocumentRef, FileRef, WorkspaceOpenParams, WorkspaceScope,
};
use crate::types::{
    BslDefinitionResponse, BslDiagnosticsResponse, BslMembersResponse, BslReferencesResponse,
    BslSymbolSearchResponse, BslTypeAtPositionResponse, CompletenessDto, ContextExpandResponse,
    ContextPackItemDto, ContextPackResponse, LocationDto, MemberDto, NodeInfoDto, ProgressDto,
    ReferenceDto, RootDto, SymbolDto, TypeInfoDto, WorkspaceDocumentsClearResponse,
    WorkspaceDocumentsSetResponse, WorkspaceListItemDto, WorkspaceListResponse,
    WorkspaceOpenResponse, WorkspaceStatusResponse,
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

pub struct SessionManager {
    sessions: RwLock<HashMap<Uuid, WorkspaceSession>>,
    store: Option<SessionStore>,
}

struct WorkspaceSession {
    roots: Vec<RootEntry>,
    documents: DocumentStore,
    analysis_revision: u64,
    settings: WorkspaceSettings,
    startup: Option<bsl_backend::system::StartupResultV2>,
    startup_job_id: Option<String>,
    startup_phase: String,
    startup_progress: u8,
    startup_error: Option<String>,
    created_at: u64,
    id_map: IdMap,
    pack_store: PackStore,
}

#[derive(Debug, Clone)]
struct RootEntry {
    root_id: String,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct WorkspaceSettings {
    platform_docs_archive: Option<PathBuf>,
    platform_version: Option<String>,
    configuration_path: Option<PathBuf>,
    mode: Option<String>,
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

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            store: SessionStore::new(),
        }
    }

    pub async fn open(
        self: &Arc<Self>,
        params: WorkspaceOpenParams,
        job_manager: Arc<JobManager>,
    ) -> Result<WorkspaceOpenResponse, rmcp::ErrorData> {
        if params.roots.is_empty() {
            return Err(rmcp::ErrorData::invalid_params(
                "roots must be non-empty",
                None,
            ));
        }

        let mut roots = Vec::new();
        let mut root_dtos = Vec::new();
        let mut seen = HashSet::new();

        for root_raw in &params.roots {
            let root_path = PathBuf::from(root_raw);
            let canonical = std::fs::canonicalize(&root_path).map_err(|_| {
                rmcp::ErrorData::invalid_params(format!("root does not exist: {root_raw}"), None)
            })?;

            let metadata = std::fs::metadata(&canonical).map_err(|_| {
                rmcp::ErrorData::invalid_params(
                    format!("root is not accessible: {}", canonical.display()),
                    None,
                )
            })?;
            if !metadata.is_dir() {
                return Err(rmcp::ErrorData::invalid_params(
                    format!("root is not a directory: {}", canonical.display()),
                    None,
                ));
            }

            let root_id = root_id(&canonical);
            if !seen.insert(root_id.clone()) {
                continue;
            }
            root_dtos.push(RootDto {
                root_id: root_id.clone(),
                path: canonical.to_string_lossy().to_string(),
            });
            roots.push(RootEntry {
                root_id,
                path: canonical,
            });
        }

        let settings = WorkspaceSettings {
            platform_docs_archive: params.platform_docs_archive.map(PathBuf::from),
            platform_version: params.platform_version,
            configuration_path: params.configuration_path.map(PathBuf::from),
            mode: params.mode,
        };
        let missing_inputs = workspace_missing_inputs(&settings);
        let warnings = workspace_warnings(&settings);

        validate_optional_path(
            settings.platform_docs_archive.as_deref(),
            "platform_docs_archive",
        )?;
        validate_optional_path(settings.configuration_path.as_deref(), "configuration_path")?;

        let session_id = Uuid::new_v4();
        let created_at = crate::state::now_unix_secs();
        let session = WorkspaceSession {
            roots,
            documents: DocumentStore::default(),
            analysis_revision: 0,
            settings,
            startup: None,
            startup_job_id: None,
            startup_phase: "startup/queued".to_string(),
            startup_progress: 0,
            startup_error: None,
            created_at,
            id_map: IdMap::default(),
            pack_store: PackStore::default(),
        };
        self.sessions.write().await.insert(session_id, session);
        self.persist_session(session_id).await;

        let startup_settings = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&session_id)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            workspace_settings_for_startup(&session.settings)
        };

        let session_manager = Arc::clone(self);
        let startup_job_id = job_manager
            .spawn("startup", move |ctx| async move {
                run_startup_job(session_manager, session_id, startup_settings, ctx).await
            })
            .await;

        self.set_startup_job_id(session_id, startup_job_id.clone())
            .await?;

        Ok(WorkspaceOpenResponse {
            session_id: session_id.to_string(),
            roots: root_dtos,
            analysis_revision: 0,
            ready: false,
            startup_job_id: Some(startup_job_id),
            warnings,
            missing_inputs,
        })
    }

    pub async fn status(
        &self,
        session_id: &str,
    ) -> Result<WorkspaceStatusResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;

        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
        let _ = session.roots.len();

        if session.startup.is_some() {
            return Ok(WorkspaceStatusResponse {
                ready: true,
                analysis_revision: session.analysis_revision,
                phase: "idle".to_string(),
                progress: ProgressDto { percent: 100 },
                warnings: workspace_warnings(&session.settings),
                missing_inputs: workspace_missing_inputs(&session.settings),
                startup_job_id: session.startup_job_id.clone(),
                error: None,
            });
        }

        Ok(WorkspaceStatusResponse {
            ready: false,
            analysis_revision: session.analysis_revision,
            phase: session.startup_phase.clone(),
            progress: ProgressDto {
                percent: session.startup_progress,
            },
            warnings: workspace_warnings(&session.settings),
            missing_inputs: workspace_missing_inputs(&session.settings),
            startup_job_id: session.startup_job_id.clone(),
            error: session.startup_error.clone(),
        })
    }

    pub async fn http_list_sessions(&self) -> Vec<McpSessionDto> {
        let sessions = self.sessions.read().await;
        let mut result = Vec::with_capacity(sessions.len());
        for (session_id, session) in sessions.iter() {
            let roots = session
                .roots
                .iter()
                .map(|root| McpRootDto {
                    root_id: root.root_id.clone(),
                    path: root.path.to_string_lossy().to_string(),
                })
                .collect();

            if session.startup.is_some() {
                result.push(McpSessionDto {
                    session_id: session_id.to_string(),
                    roots,
                    ready: true,
                    analysis_revision: session.analysis_revision,
                    phase: "idle".to_string(),
                    progress_percent: 100,
                    warnings: workspace_warnings(&session.settings),
                    missing_inputs: workspace_missing_inputs(&session.settings),
                    startup_job_id: session.startup_job_id.clone(),
                    error: None,
                });
            } else {
                result.push(McpSessionDto {
                    session_id: session_id.to_string(),
                    roots,
                    ready: false,
                    analysis_revision: session.analysis_revision,
                    phase: session.startup_phase.clone(),
                    progress_percent: session.startup_progress,
                    warnings: workspace_warnings(&session.settings),
                    missing_inputs: workspace_missing_inputs(&session.settings),
                    startup_job_id: session.startup_job_id.clone(),
                    error: session.startup_error.clone(),
                });
            }
        }

        result.sort_by(|a, b| a.session_id.cmp(&b.session_id));
        result
    }

    pub async fn http_deps_meta(
        &self,
        session_id: Option<&str>,
    ) -> Result<SnapshotMetaDto, rmcp::ErrorData> {
        let sessions = self.sessions.read().await;
        let uuid = if let Some(session_id) = session_id {
            parse_session_id(session_id)?
        } else if sessions.is_empty() {
            return Err(rmcp::ErrorData::invalid_params("no sessions", None));
        } else if sessions.len() == 1 {
            *sessions
                .keys()
                .next()
                .ok_or_else(|| rmcp::ErrorData::invalid_params("no sessions", None))?
        } else {
            return Err(rmcp::ErrorData::invalid_params(
                "session_id is required when multiple sessions exist",
                None,
            ));
        };

        let session = sessions
            .get(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
        let startup = session.startup.as_ref().ok_or_else(|| {
            rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
        })?;

        let deps_bundle = &startup.deps_bundle_v2;
        let inputs = &startup.inputs;
        Ok(SnapshotMetaDto {
            deps_id: deps_bundle.deps_id.as_str().to_string(),
            index_snapshot_id: deps_bundle.meta.index_snapshot_id.clone(),
            platform_version: deps_bundle.meta.platform_version.clone(),
            platform_fingerprint: deps_bundle.meta.platform_fingerprint.clone(),
            config_fingerprint: deps_bundle.meta.config_fingerprint.clone(),
            strict_fingerprint: deps_bundle.meta.strict_fingerprint,
            repository_stats: deps_bundle.semantic_deps.repository.get_stats(),
            inputs: SnapshotInputsDto {
                syntax_helper_path: inputs
                    .syntax_helper_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                configuration_path: inputs
                    .configuration_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                platform_version: inputs.platform_version.clone(),
                cache_enabled: inputs.cache_enabled,
                strict_fingerprint: inputs.strict_fingerprint,
            },
        })
    }

    pub async fn close(&self, session_id: &str) -> Result<(), rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;
        let mut sessions = self.sessions.write().await;
        if sessions.remove(&uuid).is_none() {
            return Err(rmcp::ErrorData::invalid_params("session not found", None));
        }
        Ok(())
    }

    pub async fn resume(
        self: &Arc<Self>,
        session_id: &str,
        job_manager: Arc<JobManager>,
    ) -> Result<WorkspaceOpenResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;

        if let Some(existing) = self.sessions.read().await.get(&uuid) {
            return Ok(WorkspaceOpenResponse {
                session_id: uuid.to_string(),
                roots: existing
                    .roots
                    .iter()
                    .map(|root| RootDto {
                        root_id: root.root_id.clone(),
                        path: root.path.to_string_lossy().to_string(),
                    })
                    .collect(),
                analysis_revision: existing.analysis_revision,
                ready: existing.startup.is_some(),
                startup_job_id: existing.startup_job_id.clone(),
                warnings: workspace_warnings(&existing.settings),
                missing_inputs: workspace_missing_inputs(&existing.settings),
            });
        }

        let store = self.store.as_ref().ok_or_else(|| {
            rmcp::ErrorData::internal_error("persist store is not available", None)
        })?;
        let persisted = store
            .load(session_id)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;

        let (roots, root_dtos) = restore_roots(&persisted.roots)?;
        let settings = WorkspaceSettings {
            platform_docs_archive: persisted.platform_docs_archive.map(PathBuf::from),
            platform_version: persisted.platform_version,
            configuration_path: persisted.configuration_path.map(PathBuf::from),
            mode: persisted.mode,
        };

        let missing_inputs = workspace_missing_inputs(&settings);
        let warnings = workspace_warnings(&settings);

        validate_optional_path(
            settings.platform_docs_archive.as_deref(),
            "platform_docs_archive",
        )?;
        validate_optional_path(settings.configuration_path.as_deref(), "configuration_path")?;

        let session = WorkspaceSession {
            roots,
            documents: DocumentStore::default(),
            analysis_revision: persisted.analysis_revision,
            settings,
            startup: None,
            startup_job_id: None,
            startup_phase: "startup/queued".to_string(),
            startup_progress: 0,
            startup_error: None,
            created_at: persisted.created_at,
            id_map: IdMap::default(),
            pack_store: PackStore::default(),
        };
        self.sessions.write().await.insert(uuid, session);
        self.persist_session(uuid).await;

        let startup_settings = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            workspace_settings_for_startup(&session.settings)
        };

        let session_manager = Arc::clone(self);
        let startup_job_id = job_manager
            .spawn("startup", move |ctx| async move {
                run_startup_job(session_manager, uuid, startup_settings, ctx).await
            })
            .await;

        self.set_startup_job_id(uuid, startup_job_id.clone())
            .await?;

        Ok(WorkspaceOpenResponse {
            session_id: uuid.to_string(),
            roots: root_dtos,
            analysis_revision: persisted.analysis_revision,
            ready: false,
            startup_job_id: Some(startup_job_id),
            warnings,
            missing_inputs,
        })
    }

    pub async fn list(&self) -> Result<WorkspaceListResponse, rmcp::ErrorData> {
        let Some(store) = self.store.as_ref() else {
            return Ok(WorkspaceListResponse {
                sessions: Vec::new(),
            });
        };
        let sessions = store
            .list()
            .into_iter()
            .map(|session| WorkspaceListItemDto {
                session_id: session.session_id,
                roots: session.roots,
                analysis_revision: session.analysis_revision,
                updated_at: session.updated_at,
            })
            .collect();
        Ok(WorkspaceListResponse { sessions })
    }

    async fn set_startup_job_id(
        &self,
        session_id: Uuid,
        startup_job_id: String,
    ) -> Result<(), rmcp::ErrorData> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
        session.startup_job_id = Some(startup_job_id);
        drop(sessions);

        self.persist_session(session_id).await;
        Ok(())
    }

    async fn set_startup_progress(&self, session_id: Uuid, phase: String, percent: u8) {
        let mut sessions = self.sessions.write().await;
        let Some(session) = sessions.get_mut(&session_id) else {
            return;
        };
        if session.startup.is_some() {
            return;
        }
        session.startup_phase = phase;
        session.startup_progress = session.startup_progress.max(percent.min(100));
    }

    async fn set_startup_result(
        &self,
        session_id: Uuid,
        startup: bsl_backend::system::StartupResultV2,
    ) -> Result<(), rmcp::ErrorData> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
        session.startup = Some(startup);
        session.startup_phase = "idle".to_string();
        session.startup_progress = 100;
        session.startup_error = None;
        drop(sessions);

        self.persist_session(session_id).await;
        Ok(())
    }

    async fn set_startup_error(&self, session_id: Uuid, error: String) {
        let mut sessions = self.sessions.write().await;
        let Some(session) = sessions.get_mut(&session_id) else {
            return;
        };
        if session.startup.is_some() {
            return;
        }
        session.startup_phase = "startup/failed".to_string();
        session.startup_progress = 100;
        session.startup_error = Some(error);
    }

    async fn persist_session(&self, session_id: Uuid) {
        let Some(store) = self.store.as_ref() else {
            return;
        };
        let sessions = self.sessions.read().await;
        let Some(session) = sessions.get(&session_id) else {
            return;
        };

        let mut persisted = PersistedSession {
            session_id: session_id.to_string(),
            roots: session
                .roots
                .iter()
                .map(|root| root.path.to_string_lossy().to_string())
                .collect(),
            platform_docs_archive: session
                .settings
                .platform_docs_archive
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            platform_version: session.settings.platform_version.clone(),
            configuration_path: session
                .settings
                .configuration_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            mode: session.settings.mode.clone(),
            analysis_revision: session.analysis_revision,
            created_at: session.created_at,
            updated_at: crate::state::now_unix_secs(),
            startup_job_id: session.startup_job_id.clone(),
        };
        store.upsert(&mut persisted);
    }

    pub async fn documents_set(
        &self,
        session_id: &str,
        files: &[FileRef],
        mark_hot: bool,
    ) -> Result<WorkspaceDocumentsSetResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;
        let mut did_change_revision = false;
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;

        let mut changed = false;
        for file in files {
            let key = session.document_key(&file.doc)?;
            if let Some(text) = &file.text {
                if file.version.is_none() {
                    return Err(rmcp::ErrorData::invalid_params(
                        "version is required when text is provided",
                        None,
                    ));
                }
                if text.len() > MAX_OVERLAY_BYTES {
                    return Err(rmcp::ErrorData::invalid_params(
                        format!("overlay text exceeds MAX_OVERLAY_BYTES={MAX_OVERLAY_BYTES}"),
                        None,
                    ));
                }
                let overlay = DocumentOverlay {
                    text: text.clone(),
                    version: file.version.unwrap_or(0),
                };
                changed |= session.documents.set_overlay(key.clone(), overlay);
            }

            if mark_hot {
                changed |= session.documents.mark_hot(key);
            }
        }

        if changed {
            session.analysis_revision = session.analysis_revision.saturating_add(1);
            session.id_map.reset(session.analysis_revision);
            session.pack_store.reset(session.analysis_revision);
            did_change_revision = true;
        }

        let analysis_revision = session.analysis_revision;
        drop(sessions);
        if did_change_revision {
            self.persist_session(uuid).await;
        }

        Ok(WorkspaceDocumentsSetResponse {
            ok: true,
            analysis_revision,
        })
    }

    pub async fn documents_clear(
        &self,
        session_id: &str,
        documents: &[DocumentRef],
        clear_hot: bool,
    ) -> Result<WorkspaceDocumentsClearResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;
        let mut did_change_revision = false;
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;

        let mut changed = false;
        for doc in documents {
            let key = session.document_key(doc)?;
            changed |= session.documents.clear_overlay(&key);
            if clear_hot {
                changed |= session.documents.clear_hot(&key);
            }
        }

        if changed {
            session.analysis_revision = session.analysis_revision.saturating_add(1);
            session.id_map.reset(session.analysis_revision);
            session.pack_store.reset(session.analysis_revision);
            did_change_revision = true;
        }

        let analysis_revision = session.analysis_revision;
        drop(sessions);
        if did_change_revision {
            self.persist_session(uuid).await;
        }

        Ok(WorkspaceDocumentsClearResponse {
            ok: true,
            analysis_revision,
        })
    }

    pub async fn bsl_diagnostics(
        &self,
        params: BslDiagnosticsParams,
    ) -> Result<BslDiagnosticsResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        let (roots, hot_set, overlays, analysis_revision, deps_id, deps) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let startup = session.startup.as_ref().ok_or_else(|| {
                rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
            })?;
            (
                session.roots.clone(),
                session.documents.hot_set.clone(),
                session.documents.overlays.clone(),
                session.analysis_revision,
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
            )
        };

        let files = collect_scope_files(&roots, &hot_set, params.scope)?;
        let facade = SemanticFacade;
        let mut diagnostics = Vec::new();
        let mut truncated = false;
        let mut total_read_bytes = 0u64;

        for file in files {
            let snapshot = match load_document_snapshot(&file, &overlays)? {
                Some(snapshot) => snapshot,
                None => continue,
            };

            total_read_bytes = total_read_bytes.saturating_add(snapshot.text.len() as u64);
            if total_read_bytes > MAX_TOTAL_READ_BYTES {
                truncated = true;
                break;
            }

            let mut host = bsl_analysis_v2::AnalysisHostV2::default();
            host.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
                deps_id: deps_id.clone(),
                deps: deps.clone(),
            });
            host.apply_change(bsl_analysis_v2::Change::SetFile {
                file_id: bsl_analysis_v2::FileId(1),
                text: Arc::from(snapshot.text),
                version: snapshot.version,
                path: Arc::from(snapshot.abs_path.to_string_lossy().to_string()),
            });

            let analysis = host.analysis();
            let file_diags = analysis
                .semantic_diagnostics(bsl_analysis_v2::FileId(1))
                .ok()
                .flatten();
            let Some(file_diags) = file_diags else {
                continue;
            };

            for diag in file_diags.iter() {
                if diagnostics.len() >= params.limit as usize {
                    truncated = true;
                    break;
                }

                let range = RangeDto {
                    start: PositionDto {
                        line: diag.line,
                        character: diag.column,
                    },
                    end: PositionDto {
                        line: diag.end_line,
                        character: diag.end_column,
                    },
                };

                let severity = match diag.severity {
                    bsl_shared::domain::types::DiagnosticSeverity::Error => {
                        crate::semantic::dto::DiagnosticSeverityDto::Error
                    }
                    bsl_shared::domain::types::DiagnosticSeverity::Warning => {
                        crate::semantic::dto::DiagnosticSeverityDto::Warning
                    }
                    bsl_shared::domain::types::DiagnosticSeverity::Info
                    | bsl_shared::domain::types::DiagnosticSeverity::Hint => {
                        crate::semantic::dto::DiagnosticSeverityDto::Info
                    }
                };

                diagnostics.push(facade.diagnostic(
                    analysis_revision,
                    DocumentRefDto {
                        root_id: snapshot.file.root_id.clone(),
                        path: snapshot.file.path.clone(),
                    },
                    range,
                    severity,
                    None,
                    diag.message.clone(),
                ));
            }

            if truncated {
                break;
            }
        }

        facade.sort_diagnostics(&mut diagnostics);
        if diagnostics.len() > params.limit as usize {
            diagnostics.truncate(params.limit as usize);
            truncated = true;
        }

        Ok(BslDiagnosticsResponse {
            analysis_revision,
            diagnostics,
            truncated,
        })
    }

    pub async fn bsl_symbol_search(
        &self,
        params: BslSymbolSearchParams,
    ) -> Result<BslSymbolSearchResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        let query = params.query.trim();
        if query.is_empty() {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            return Ok(BslSymbolSearchResponse {
                analysis_revision: session.analysis_revision,
                symbols: Vec::new(),
                truncated: false,
            });
        }

        let (roots, analysis_revision, deps_id, deps) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let startup = session.startup.as_ref().ok_or_else(|| {
                rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
            })?;
            (
                session.roots.clone(),
                session.analysis_revision,
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
            )
        };

        let files = collect_project_files(&roots)?;
        let query_lower = query.to_lowercase();
        let mut symbols = Vec::new();
        let mut truncated = false;
        let mut total_read_bytes = 0u64;

        for file in files {
            let text = match load_disk_text_with_limits(&file.root_path, &file.abs_path)? {
                Some(text) => text,
                None => continue,
            };
            total_read_bytes = total_read_bytes.saturating_add(text.len() as u64);
            if total_read_bytes > MAX_TOTAL_READ_BYTES {
                truncated = true;
                break;
            }

            let mut host = AnalysisHostV2::default();
            host.apply_change(Change::SetDepsSnapshot {
                deps_id: deps_id.clone(),
                deps: deps.clone(),
            });
            host.apply_change(Change::SetFile {
                file_id: FileId(1),
                text: Arc::from(text),
                version: 0,
                path: Arc::from(file.abs_path.to_string_lossy().to_string()),
            });

            let analysis = host.analysis();
            let program = analysis.ir(FileId(1)).ok().flatten();
            let Some(program) = program else {
                continue;
            };

            for node in program.nodes.iter() {
                let (kind, name) = match &node.kind {
                    bsl_shared::ir::SemanticNodeKind::FunctionDeclaration { name, .. } => {
                        ("function", name.as_str())
                    }
                    bsl_shared::ir::SemanticNodeKind::ProcedureDeclaration { name, .. } => {
                        ("procedure", name.as_str())
                    }
                    _ => continue,
                };

                if !name.to_lowercase().contains(&query_lower) {
                    continue;
                }

                if symbols.len() >= params.limit as usize {
                    truncated = true;
                    break;
                }

                let range = RangeDto {
                    start: PositionDto {
                        line: node.span.start_line,
                        character: node.span.start_column,
                    },
                    end: PositionDto {
                        line: node.span.end_line,
                        character: node.span.end_column,
                    },
                };
                let file_ref = DocumentRefDto {
                    root_id: file.root_id.clone(),
                    path: file.rel_path.clone(),
                };
                let document_id = ids::document_id(&file_ref.root_id, &file_ref.path);
                let symbol_id = ids::stable_id_hex(&[
                    ids::IdPart::U64(analysis_revision),
                    ids::IdPart::Str(&document_id),
                    ids::IdPart::Str(kind),
                    ids::IdPart::U32(range.start.line),
                    ids::IdPart::U32(range.start.character),
                    ids::IdPart::U32(range.end.line),
                    ids::IdPart::U32(range.end.character),
                    ids::IdPart::Str(name),
                ]);

                symbols.push(SymbolDto {
                    symbol_id: symbol_id.clone(),
                    name: name.to_string(),
                    kind: kind.to_string(),
                    file: file_ref,
                    range,
                });
            }

            if truncated {
                break;
            }
        }

        sort_symbols(&mut symbols);
        if symbols.len() > params.limit as usize {
            symbols.truncate(params.limit as usize);
            truncated = true;
        }

        {
            let mut sessions = self.sessions.write().await;
            let Some(session) = sessions.get_mut(&uuid) else {
                return Err(rmcp::ErrorData::invalid_params("session not found", None));
            };
            if session.analysis_revision == analysis_revision {
                session.id_map.reset(analysis_revision);
                for symbol in &symbols {
                    session.id_map.symbols.insert(
                        symbol.symbol_id.clone(),
                        StoredSymbol {
                            name: symbol.name.clone(),
                            kind: symbol.kind.clone(),
                            file: symbol.file.clone(),
                            range: symbol.range,
                        },
                    );
                }
            }
        }

        Ok(BslSymbolSearchResponse {
            analysis_revision,
            symbols,
            truncated,
        })
    }

    pub async fn bsl_type_at_position(
        &self,
        params: BslTypeAtPositionParams,
    ) -> Result<BslTypeAtPositionResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        let (analysis_revision, roots, overlays, deps_id, deps, index_snapshot) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let startup = session.startup.as_ref().ok_or_else(|| {
                rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
            })?;
            (
                session.analysis_revision,
                session.roots.clone(),
                session.documents.overlays.clone(),
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
                startup.deps_bundle_v2.index_snapshot.clone(),
            )
        };

        let file_key = document_key_from_ref(&roots, &params.file.doc)?;
        let (root_path, abs_path, _file_ref) = resolve_doc_path(&roots, &file_key)?;
        let text =
            select_effective_text(&params.file, &file_key, &overlays, &root_path, &abs_path)?;
        let version = select_effective_version(&params.file, &file_key, &overlays);

        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id.clone(),
            deps: deps.clone(),
        });
        host.apply_change(Change::SetFile {
            file_id: FileId(1),
            text: Arc::from(text),
            version,
            path: Arc::from(abs_path.to_string_lossy().to_string()),
        });

        let analysis = host.analysis();
        let program = analysis.ir(FileId(1)).ok().flatten();
        let Some(program) = program else {
            return Ok(BslTypeAtPositionResponse {
                analysis_revision,
                type_info: None,
                node: None,
                warnings: vec!["IR not available".to_string()],
            });
        };

        let resolver = deps
            .resolver
            .clone()
            .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));

        let pos = params.position;
        let type_info = get_type_at_position(&program, resolver.as_ref(), pos.line, pos.character)
            .map(|resolution| TypeInfoDto {
                name: resolution.type_name(),
                certainty: format!("{:?}", resolution.certainty).to_lowercase(),
                active_facet: resolution
                    .active_facet
                    .as_ref()
                    .map(|facet| format!("{:?}", facet)),
            });

        let node = program
            .find_node_at_position(pos.line, pos.character)
            .map(|node| NodeInfoDto {
                kind: format!("{:?}", node.kind),
                range: RangeDto {
                    start: PositionDto {
                        line: node.span.start_line,
                        character: node.span.start_column,
                    },
                    end: PositionDto {
                        line: node.span.end_line,
                        character: node.span.end_column,
                    },
                },
            });

        let _ = index_snapshot.id.as_str();

        Ok(BslTypeAtPositionResponse {
            analysis_revision,
            type_info,
            node,
            warnings: Vec::new(),
        })
    }

    pub async fn bsl_members(
        &self,
        params: BslMembersParams,
    ) -> Result<BslMembersResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        let (analysis_revision, roots, overlays, deps_id, deps, index_snapshot) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let startup = session.startup.as_ref().ok_or_else(|| {
                rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
            })?;
            (
                session.analysis_revision,
                session.roots.clone(),
                session.documents.overlays.clone(),
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
                startup.deps_bundle_v2.index_snapshot.clone(),
            )
        };

        let file_key = document_key_from_ref(&roots, &params.file.doc)?;
        let (root_path, abs_path, _file_ref) = resolve_doc_path(&roots, &file_key)?;
        let text =
            select_effective_text(&params.file, &file_key, &overlays, &root_path, &abs_path)?;
        let version = select_effective_version(&params.file, &file_key, &overlays);

        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id.clone(),
            deps: deps.clone(),
        });
        host.apply_change(Change::SetFile {
            file_id: FileId(1),
            text: Arc::from(text.clone()),
            version,
            path: Arc::from(abs_path.to_string_lossy().to_string()),
        });

        let analysis = host.analysis();
        let program = analysis.ir(FileId(1)).ok().flatten();
        let parse_result = analysis.parse_result(FileId(1)).ok().flatten();
        let Some(program) = program else {
            return Ok(BslMembersResponse {
                analysis_revision,
                members: Vec::new(),
                truncated: false,
            });
        };
        let Some(parse_result) = parse_result else {
            return Ok(BslMembersResponse {
                analysis_revision,
                members: Vec::new(),
                truncated: false,
            });
        };

        let resolver = deps
            .resolver
            .clone()
            .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
        let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());

        let result = bsl_backend::application::type_system::get_completion_with_semantic_program_snapshot_v2(
            text.as_str(),
            params.position.line,
            params.position.character,
            None,
            index_snapshot.as_ref(),
            &metadata_lookup,
            abs_path.to_string_lossy().as_ref(),
            resolver.as_ref(),
            program,
            parse_result,
        )
        .await
        .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?;

        let mut members = result
            .items
            .into_iter()
            .filter_map(|candidate| {
                let kind = match candidate.item.kind {
                    bsl_shared::domain::CompletionKind::Method => "method",
                    bsl_shared::domain::CompletionKind::Property => "property",
                    bsl_shared::domain::CompletionKind::Field => "field",
                    bsl_shared::domain::CompletionKind::Function => "function",
                    bsl_shared::domain::CompletionKind::Constructor => "constructor",
                    _ => return None,
                };

                Some(MemberDto {
                    name: candidate.item.label,
                    kind: kind.to_string(),
                    detail: candidate.item.detail,
                })
            })
            .collect::<Vec<_>>();

        members.sort_by(|a, b| {
            (a.kind.as_str(), a.name.as_str()).cmp(&(b.kind.as_str(), b.name.as_str()))
        });
        let truncated = members.len() > params.limit as usize || result.is_incomplete;
        if members.len() > params.limit as usize {
            members.truncate(params.limit as usize);
        }

        Ok(BslMembersResponse {
            analysis_revision,
            members,
            truncated,
        })
    }

    pub async fn bsl_definition(
        &self,
        params: BslDefinitionParams,
    ) -> Result<BslDefinitionResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        if let Some(symbol_id) = params.symbol_id.as_deref() {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let Some(symbol) = session
                .id_map
                .get_symbol(session.analysis_revision, symbol_id)
            else {
                return Err(rmcp::ErrorData::invalid_params(
                    "stale or unknown symbol_id",
                    None,
                ));
            };

            return Ok(BslDefinitionResponse {
                analysis_revision: session.analysis_revision,
                location: Some(LocationDto {
                    file: symbol.file.clone(),
                    range: symbol.range,
                }),
                snippet: None,
            });
        }

        let Some(file) = params.file else {
            return Err(rmcp::ErrorData::invalid_params(
                "expected symbol_id or file+position",
                None,
            ));
        };
        let Some(position) = params.position else {
            return Err(rmcp::ErrorData::invalid_params(
                "expected symbol_id or file+position",
                None,
            ));
        };

        let (analysis_revision, roots, overlays, deps_id, deps) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let startup = session.startup.as_ref().ok_or_else(|| {
                rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
            })?;
            (
                session.analysis_revision,
                session.roots.clone(),
                session.documents.overlays.clone(),
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
            )
        };

        let file_key = document_key_from_ref(&roots, &file.doc)?;
        let (root_path, abs_path, _file_ref) = resolve_doc_path(&roots, &file_key)?;
        let text = select_effective_text(&file, &file_key, &overlays, &root_path, &abs_path)?;
        let version = select_effective_version(&file, &file_key, &overlays);

        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id.clone(),
            deps: deps.clone(),
        });
        host.apply_change(Change::SetFile {
            file_id: FileId(1),
            text: Arc::from(text),
            version,
            path: Arc::from(abs_path.to_string_lossy().to_string()),
        });

        let analysis = host.analysis();
        let program = analysis.ir(FileId(1)).ok().flatten();
        let Some(program) = program else {
            return Ok(BslDefinitionResponse {
                analysis_revision,
                location: None,
                snippet: None,
            });
        };

        let target = bsl_backend::application::type_system::goto_definition_v2(
            abs_path.to_string_lossy().as_ref(),
            program,
            deps,
            position.line,
            position.character,
        );

        let Some(target) = target else {
            return Ok(BslDefinitionResponse {
                analysis_revision,
                location: None,
                snippet: None,
            });
        };

        let location = map_path_to_root(&roots, &target.file_path).map(|(root_id, rel_path)| {
            let range = target
                .span
                .map(span_to_range)
                .unwrap_or_else(RangeDto::default);
            LocationDto {
                file: DocumentRefDto {
                    root_id,
                    path: rel_path,
                },
                range,
            }
        });

        Ok(BslDefinitionResponse {
            analysis_revision,
            location,
            snippet: None,
        })
    }

    pub async fn bsl_references(
        &self,
        params: BslReferencesParams,
    ) -> Result<BslReferencesResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        let (roots, analysis_revision, deps_id, deps, symbol) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let startup = session.startup.as_ref().ok_or_else(|| {
                rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
            })?;
            let symbol = session
                .id_map
                .get_symbol(session.analysis_revision, &params.symbol_id)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("stale or unknown symbol_id", None))?
                .clone();
            (
                session.roots.clone(),
                session.analysis_revision,
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
                symbol,
            )
        };

        if symbol.kind != "function" && symbol.kind != "procedure" {
            return Ok(BslReferencesResponse {
                analysis_revision,
                count: 0,
                references: Vec::new(),
                truncated: false,
            });
        }

        let files = collect_project_files(&roots)?;
        let mut references = Vec::new();
        let mut truncated = false;
        let mut total_read_bytes = 0u64;

        for file in files {
            let text = match load_disk_text_with_limits(&file.root_path, &file.abs_path)? {
                Some(text) => text,
                None => continue,
            };
            total_read_bytes = total_read_bytes.saturating_add(text.len() as u64);
            if total_read_bytes > MAX_TOTAL_READ_BYTES {
                truncated = true;
                break;
            }

            let mut host = AnalysisHostV2::default();
            host.apply_change(Change::SetDepsSnapshot {
                deps_id: deps_id.clone(),
                deps: deps.clone(),
            });
            host.apply_change(Change::SetFile {
                file_id: FileId(1),
                text: Arc::from(text),
                version: 0,
                path: Arc::from(file.abs_path.to_string_lossy().to_string()),
            });

            let analysis = host.analysis();
            let program = analysis.ir(FileId(1)).ok().flatten();
            let Some(program) = program else {
                continue;
            };

            for node in program.nodes.iter() {
                let bsl_shared::ir::SemanticNodeKind::FunctionCall {
                    function_name,
                    object_type,
                    ..
                } = &node.kind
                else {
                    continue;
                };
                if object_type.is_some() {
                    continue;
                }
                if !function_name.eq_ignore_ascii_case(&symbol.name) {
                    continue;
                }
                if references.len() >= params.limit as usize {
                    truncated = true;
                    break;
                }

                references.push(ReferenceDto {
                    file: DocumentRefDto {
                        root_id: file.root_id.clone(),
                        path: file.rel_path.clone(),
                    },
                    range: span_to_range(node.span),
                });
            }

            if truncated {
                break;
            }
        }

        sort_references(&mut references);
        if references.len() > params.limit as usize {
            references.truncate(params.limit as usize);
            truncated = true;
        }

        Ok(BslReferencesResponse {
            analysis_revision,
            count: references.len() as u64,
            references,
            truncated,
        })
    }

    pub async fn context_pack(
        &self,
        params: ContextPackParams,
    ) -> Result<ContextPackResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        let (analysis_revision, roots, overlays, _hot_set, settings, deps_id, deps, index_snapshot) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let startup = session.startup.as_ref().ok_or_else(|| {
                rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
            })?;
            (
                session.analysis_revision,
                session.roots.clone(),
                session.documents.overlays.clone(),
                session.documents.hot_set.clone(),
                session.settings.clone(),
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
                startup.deps_bundle_v2.index_snapshot.clone(),
            )
        };

        let budget_chars = compute_budget_chars(params.budget_chars, params.budget_tokens);
        let budget_chars_u32 = if budget_chars > u32::MAX as usize {
            u32::MAX
        } else {
            budget_chars as u32
        };

        let goal = params.goal.unwrap_or_default();
        let scope = params.scope.unwrap_or(WorkspaceScope::Hot);
        let include_key = include_fingerprint(&params.include);
        let scope_key = scope_key_for_pack(&roots, &scope)?;
        let focus_key = match params.focus.as_ref() {
            Some(focus) => focus_key_for_pack(&roots, focus)?,
            None => "none".to_string(),
        };

        let pack_id = ids::pack_id(
            analysis_revision,
            goal.as_str(),
            focus_key.as_str(),
            scope_key.as_str(),
            include_key.as_str(),
            budget_chars_u32,
        );

        let missing_inputs = workspace_missing_inputs(&settings);
        let completeness = if missing_inputs.is_empty() {
            CompletenessDto::Full
        } else {
            CompletenessDto::Partial
        };

        let mut text = TextBudget::new(budget_chars);
        text.push_line("bsl-agent context_pack");
        if !goal.is_empty() {
            text.push_line(&format!("goal: {goal}"));
        }
        text.push_line(&format!("analysis_revision: {analysis_revision}"));
        text.push_line(&format!(
            "completeness: {}",
            match completeness {
                CompletenessDto::Full => "full",
                CompletenessDto::Partial => "partial",
            }
        ));
        if !missing_inputs.is_empty() {
            text.push_line(&format!("missing_inputs: {}", missing_inputs.join(", ")));
        }
        text.push_line("");

        let mut items: Vec<ContextPackItemDto> = Vec::new();
        let mut stored_items: HashMap<String, StoredPackItem> = HashMap::new();
        let mut truncated = false;

        match params.focus {
            Some(ContextFocus::Position { file, position }) => {
                let file_key = document_key_from_ref(&roots, &file.doc)?;
                let (root_path, abs_path, file_ref) = resolve_doc_path(&roots, &file_key)?;
                let source_text =
                    select_effective_text(&file, &file_key, &overlays, &root_path, &abs_path)?;

                text.push_line(&format!(
                    "focus: position {}:{}:{}",
                    file_ref.path,
                    position.line + 1,
                    position.character + 1
                ));
                text.push_line("");

                if params.include.snippets {
                    let center_line = position.line;
                    let snippet =
                        render_snippet(&source_text, center_line, PACK_SNIPPET_CONTEXT_LINES);
                    let primary = format!(
                        "{}:{}:{}",
                        ids::document_id(&file_ref.root_id, &file_ref.path),
                        center_line,
                        PACK_SNIPPET_CONTEXT_LINES
                    );
                    let item_id = ids::pack_item_id(&pack_id, "snippet", &primary);
                    stored_items.insert(
                        item_id.clone(),
                        StoredPackItem::Snippet {
                            file: file_ref.clone(),
                            center_line,
                        },
                    );
                    items.push(ContextPackItemDto {
                        item_id: item_id.clone(),
                        kind: "snippet".to_string(),
                        file: Some(file_ref.clone()),
                        range: Some(snippet.range),
                        summary: format!("Snippet around {}:{}", file_ref.path, center_line + 1),
                    });
                    text.push_line("```bsl");
                    text.push_str(&snippet.text);
                    text.push_line("```");
                    text.push_line("");
                    truncated |= snippet.truncated;
                }

                if params.include.types {
                    let version = select_effective_version(&file, &file_key, &overlays);
                    let mut host = AnalysisHostV2::default();
                    host.apply_change(Change::SetDepsSnapshot {
                        deps_id: deps_id.clone(),
                        deps: deps.clone(),
                    });
                    host.apply_change(Change::SetFile {
                        file_id: FileId(1),
                        text: Arc::from(source_text),
                        version,
                        path: Arc::from(abs_path.to_string_lossy().to_string()),
                    });

                    let analysis = host.analysis();
                    let program = analysis.ir(FileId(1)).ok().flatten();
                    if let Some(program) = program {
                        let resolver = deps.resolver.clone().unwrap_or_else(|| {
                            Arc::new(TypeResolver::new(deps.repository.clone()))
                        });
                        if let Some(type_info) = get_type_at_position(
                            &program,
                            resolver.as_ref(),
                            position.line,
                            position.character,
                        ) {
                            text.push_line(&format!("type_at_position: {}", type_info.type_name()));
                            text.push_line("");
                        }
                    }
                }

                let _ = index_snapshot.id.as_str();
            }
            Some(ContextFocus::Diagnostic { diagnostic_id }) => {
                text.push_line(&format!("focus: diagnostic {diagnostic_id}"));
                text.push_line("");

                let diagnostics = self
                    .bsl_diagnostics(BslDiagnosticsParams {
                        session_id: params.session_id.clone(),
                        scope: scope.clone(),
                        limit: 500,
                        include_impact: false,
                        include_coverage: false,
                    })
                    .await?;
                let diagnostic = diagnostics
                    .diagnostics
                    .iter()
                    .find(|diag| diag.diagnostic_id == diagnostic_id)
                    .ok_or_else(|| {
                        rmcp::ErrorData::invalid_params("stale or unknown diagnostic_id", None)
                    })?;

                if params.include.snippets {
                    let doc = DocumentRef {
                        root_id: diagnostic.file.root_id.clone(),
                        path: diagnostic.file.path.clone(),
                    };
                    let file_key = document_key_from_ref(&roots, &doc)?;
                    let (root_path, abs_path, file_ref) = resolve_doc_path(&roots, &file_key)?;
                    let file = FileRef {
                        doc,
                        text: None,
                        version: None,
                    };
                    let source_text =
                        select_effective_text(&file, &file_key, &overlays, &root_path, &abs_path)?;

                    let center_line = diagnostic.range.start.line;
                    let snippet =
                        render_snippet(&source_text, center_line, PACK_SNIPPET_CONTEXT_LINES);
                    let primary = format!(
                        "{}:{}:{}",
                        ids::document_id(&file_ref.root_id, &file_ref.path),
                        center_line,
                        PACK_SNIPPET_CONTEXT_LINES
                    );
                    let item_id = ids::pack_item_id(&pack_id, "snippet", &primary);
                    stored_items.insert(
                        item_id.clone(),
                        StoredPackItem::Snippet {
                            file: file_ref.clone(),
                            center_line,
                        },
                    );
                    items.push(ContextPackItemDto {
                        item_id: item_id.clone(),
                        kind: "snippet".to_string(),
                        file: Some(file_ref.clone()),
                        range: Some(snippet.range),
                        summary: format!(
                            "Snippet for diagnostic in {}:{}",
                            file_ref.path,
                            center_line + 1
                        ),
                    });
                    text.push_line(&format!("diagnostic: {}", diagnostic.message));
                    text.push_line("```bsl");
                    text.push_str(&snippet.text);
                    text.push_line("```");
                    text.push_line("");
                    truncated |= snippet.truncated;
                }
                truncated |= diagnostics.truncated;
            }
            Some(ContextFocus::Symbol { symbol_id }) => {
                let symbol = {
                    let sessions = self.sessions.read().await;
                    let session = sessions.get(&uuid).ok_or_else(|| {
                        rmcp::ErrorData::invalid_params("session not found", None)
                    })?;
                    session
                        .id_map
                        .get_symbol(session.analysis_revision, &symbol_id)
                        .ok_or_else(|| {
                            rmcp::ErrorData::invalid_params("stale or unknown symbol_id", None)
                        })?
                        .clone()
                };

                text.push_line(&format!(
                    "focus: symbol {} {} ({})",
                    symbol.kind, symbol.name, symbol_id
                ));
                text.push_line(&format!(
                    "definition: {}:{}",
                    symbol.file.path,
                    symbol.range.start.line + 1
                ));
                text.push_line("");

                if params.include.snippets {
                    let doc = DocumentRef {
                        root_id: symbol.file.root_id.clone(),
                        path: symbol.file.path.clone(),
                    };
                    let file_key = document_key_from_ref(&roots, &doc)?;
                    let (root_path, abs_path, file_ref) = resolve_doc_path(&roots, &file_key)?;
                    let file = FileRef {
                        doc,
                        text: None,
                        version: None,
                    };
                    let source_text =
                        select_effective_text(&file, &file_key, &overlays, &root_path, &abs_path)?;
                    let center_line = symbol.range.start.line;
                    let snippet =
                        render_snippet(&source_text, center_line, PACK_SNIPPET_CONTEXT_LINES);
                    let primary = format!(
                        "{}:{}:{}",
                        ids::document_id(&file_ref.root_id, &file_ref.path),
                        center_line,
                        PACK_SNIPPET_CONTEXT_LINES
                    );
                    let item_id = ids::pack_item_id(&pack_id, "snippet", &primary);
                    stored_items.insert(
                        item_id.clone(),
                        StoredPackItem::Snippet {
                            file: file_ref.clone(),
                            center_line,
                        },
                    );
                    items.push(ContextPackItemDto {
                        item_id: item_id.clone(),
                        kind: "snippet".to_string(),
                        file: Some(file_ref.clone()),
                        range: Some(snippet.range),
                        summary: format!("Snippet for symbol {} in {}", symbol.name, file_ref.path),
                    });
                    text.push_line("```bsl");
                    text.push_str(&snippet.text);
                    text.push_line("```");
                    text.push_line("");
                    truncated |= snippet.truncated;
                }

                if params.include.references {
                    let refs = self
                        .bsl_references(BslReferencesParams {
                            session_id: params.session_id.clone(),
                            symbol_id,
                            limit: 50,
                            include_snippets: false,
                        })
                        .await?;
                    text.push_line(&format!("references: {}", refs.count));
                    text.push_line("");
                    truncated |= refs.truncated;
                }
            }
            Some(ContextFocus::Query { query }) => {
                text.push_line(&format!("focus: query {query:?}"));
                text.push_line("");

                if params.include.symbols {
                    let response = self
                        .bsl_symbol_search(BslSymbolSearchParams {
                            session_id: params.session_id.clone(),
                            query,
                            limit: 20,
                        })
                        .await?;
                    if !response.symbols.is_empty() {
                        text.push_line("symbols:");
                        for symbol in &response.symbols {
                            text.push_line(&format!(
                                "- {} {} ({}:{})",
                                symbol.kind,
                                symbol.name,
                                symbol.file.path,
                                symbol.range.start.line + 1
                            ));
                        }
                        text.push_line("");
                    }
                    truncated |= response.truncated;
                }
            }
            None => {
                text.push_line("focus: none");
                text.push_line("");
            }
        }

        sort_pack_items(&mut items);

        let text_truncated = text.truncated;
        let pack_truncated = text_truncated || truncated;

        let stored_pack = StoredPack {
            items: stored_items,
        };
        {
            let mut sessions = self.sessions.write().await;
            let session = sessions
                .get_mut(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            if session.analysis_revision != analysis_revision {
                return Err(rmcp::ErrorData::invalid_params(
                    "analysis_revision changed; retry",
                    None,
                ));
            }
            session
                .pack_store
                .insert_pack(analysis_revision, pack_id.clone(), stored_pack);
        }

        Ok(ContextPackResponse {
            analysis_revision,
            pack_id,
            text: text.finish(),
            items,
            truncated: pack_truncated,
            completeness,
            missing_inputs,
        })
    }

    pub async fn context_expand(
        &self,
        params: ContextExpandParams,
    ) -> Result<ContextExpandResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        let (analysis_revision, roots, overlays, item) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let item = session
                .pack_store
                .get_item(session.analysis_revision, &params.pack_id, &params.item_id)
                .ok_or_else(|| {
                    rmcp::ErrorData::invalid_params("stale or unknown pack_id/item_id", None)
                })?
                .clone();
            (
                session.analysis_revision,
                session.roots.clone(),
                session.documents.overlays.clone(),
                item,
            )
        };

        let budget_chars = compute_budget_chars(params.budget_chars, params.budget_tokens);
        let mut text = TextBudget::new(budget_chars);

        match item {
            StoredPackItem::Snippet { file, center_line } => {
                let doc = DocumentRef {
                    root_id: file.root_id.clone(),
                    path: file.path.clone(),
                };
                let file_key = document_key_from_ref(&roots, &doc)?;
                let (root_path, abs_path, file_ref) = resolve_doc_path(&roots, &file_key)?;
                let file = FileRef {
                    doc,
                    text: None,
                    version: None,
                };
                let source_text =
                    select_effective_text(&file, &file_key, &overlays, &root_path, &abs_path)?;
                let snippet =
                    render_snippet(&source_text, center_line, EXPAND_SNIPPET_CONTEXT_LINES);
                text.push_line(&format!(
                    "snippet {}:{} (+/-{} lines)",
                    file_ref.path,
                    center_line + 1,
                    EXPAND_SNIPPET_CONTEXT_LINES
                ));
                text.push_line("```bsl");
                text.push_str(&snippet.text);
                text.push_line("```");
                let _ = snippet.truncated;
            }
        }

        let truncated = text.truncated;
        Ok(ContextExpandResponse {
            analysis_revision,
            text: text.finish(),
            truncated,
        })
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceSession {
    fn document_key(&self, doc: &DocumentRef) -> Result<DocumentKey, rmcp::ErrorData> {
        let root = self
            .roots
            .iter()
            .find(|root| root.root_id == doc.root_id)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("unknown root_id", None))?;
        let _ = root.path.as_path();

        let normalized_path = normalize_relative_path(&doc.path)?;
        Ok(DocumentKey {
            root_id: doc.root_id.clone(),
            path: normalized_path,
        })
    }
}

impl DocumentStore {
    fn set_overlay(&mut self, key: DocumentKey, overlay: DocumentOverlay) -> bool {
        match self.overlays.get(&key) {
            Some(existing) if *existing == overlay => false,
            _ => {
                self.overlays.insert(key, overlay);
                true
            }
        }
    }

    fn clear_overlay(&mut self, key: &DocumentKey) -> bool {
        self.overlays.remove(key).is_some()
    }

    fn mark_hot(&mut self, key: DocumentKey) -> bool {
        self.hot_set.insert(key)
    }

    fn clear_hot(&mut self, key: &DocumentKey) -> bool {
        self.hot_set.remove(key)
    }
}

impl IdMap {
    fn reset(&mut self, analysis_revision: u64) {
        self.analysis_revision = analysis_revision;
        self.symbols.clear();
    }

    fn get_symbol(&self, analysis_revision: u64, symbol_id: &str) -> Option<&StoredSymbol> {
        if self.analysis_revision != analysis_revision {
            return None;
        }
        self.symbols.get(symbol_id)
    }
}

impl PackStore {
    fn reset(&mut self, analysis_revision: u64) {
        self.analysis_revision = analysis_revision;
        self.packs.clear();
    }

    fn insert_pack(&mut self, analysis_revision: u64, pack_id: String, pack: StoredPack) {
        if self.analysis_revision != analysis_revision {
            self.reset(analysis_revision);
        }
        self.packs.insert(pack_id, pack);
    }

    fn get_item(
        &self,
        analysis_revision: u64,
        pack_id: &str,
        item_id: &str,
    ) -> Option<&StoredPackItem> {
        if self.analysis_revision != analysis_revision {
            return None;
        }
        self.packs.get(pack_id)?.items.get(item_id)
    }
}

fn parse_session_id(session_id: &str) -> Result<Uuid, rmcp::ErrorData> {
    Uuid::parse_str(session_id)
        .map_err(|_| rmcp::ErrorData::invalid_params("invalid session_id", None))
}

fn root_id(path: &Path) -> String {
    blake3::hash(path.to_string_lossy().as_bytes())
        .to_hex()
        .to_string()
}

fn normalize_relative_path(path: &str) -> Result<String, rmcp::ErrorData> {
    if path.is_empty() {
        return Err(rmcp::ErrorData::invalid_params(
            "path must be non-empty",
            None,
        ));
    }

    let mut components = Vec::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(value) => components.push(value.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(rmcp::ErrorData::invalid_params(
                    "path must not contain '..'",
                    None,
                ))
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(rmcp::ErrorData::invalid_params(
                    "path must be relative",
                    None,
                ))
            }
        }
    }

    if components.is_empty() {
        return Err(rmcp::ErrorData::invalid_params(
            "path must be non-empty",
            None,
        ));
    }

    Ok(components.join("/"))
}

fn workspace_missing_inputs(settings: &WorkspaceSettings) -> Vec<String> {
    let mut missing = Vec::new();
    if settings.platform_docs_archive.is_none() {
        missing.push("platform_docs_archive".to_string());
    }
    if settings.configuration_path.is_none() {
        missing.push("configuration_path".to_string());
    }
    if settings.configuration_path.is_some() && settings.platform_version.is_none() {
        missing.push("platform_version".to_string());
    }
    missing
}

fn workspace_warnings(settings: &WorkspaceSettings) -> Vec<String> {
    let mut warnings = Vec::new();
    if settings.platform_docs_archive.is_none() && settings.platform_version.is_some() {
        warnings
            .push("platform_version provided without platform_docs_archive; ignored".to_string());
    }
    if let Some(mode) = settings.mode.as_deref() {
        if !mode.is_empty() && mode != "progressive" {
            warnings.push(format!("unknown mode: {mode}"));
        }
    }
    warnings
}

fn workspace_settings_for_startup(settings: &WorkspaceSettings) -> WorkspaceSettings {
    if settings.configuration_path.is_some() && settings.platform_version.is_none() {
        return WorkspaceSettings {
            platform_docs_archive: settings.platform_docs_archive.clone(),
            platform_version: settings.platform_version.clone(),
            configuration_path: None,
            mode: settings.mode.clone(),
        };
    }
    settings.clone()
}

fn validate_optional_path(path: Option<&Path>, field: &str) -> Result<(), rmcp::ErrorData> {
    let Some(path) = path else {
        return Ok(());
    };
    if !path.exists() {
        return Err(rmcp::ErrorData::invalid_params(
            format!("{field} does not exist: {}", path.display()),
            None,
        ));
    }
    Ok(())
}

async fn start_semantic_runtime(
    settings: &WorkspaceSettings,
    progress_tx: Option<mpsc::UnboundedSender<ProgressUpdate>>,
) -> Result<bsl_backend::system::StartupResultV2, rmcp::ErrorData> {
    let coordinator = Arc::new(bsl_backend::system::SystemCoordinator::new());
    let inputs = bsl_backend::system::StartupInputs::from_web_flags(
        settings.platform_docs_archive.clone(),
        settings.configuration_path.clone(),
        settings.platform_version.clone(),
        None,
        None,
    );

    bsl_backend::system::startup_v2(coordinator, inputs, progress_tx)
        .await
        .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))
}

async fn run_startup_job(
    session_manager: Arc<SessionManager>,
    session_id: Uuid,
    settings: WorkspaceSettings,
    ctx: JobContext,
) -> Result<serde_json::Value, anyhow::Error> {
    ctx.set_progress("startup/starting".to_string(), 0).await;
    session_manager
        .set_startup_progress(session_id, "startup/starting".to_string(), 0)
        .await;

    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProgressUpdate>();
    let ctx_for_progress = ctx.clone();
    let session_for_progress = Arc::clone(&session_manager);
    let progress_task = tokio::spawn(async move {
        while let Some(update) = progress_rx.recv().await {
            let phase = format!("startup/{:?}", update.phase);
            let percent = update.percentage.round().clamp(0.0, 100.0) as u8;
            ctx_for_progress.set_progress(phase.clone(), percent).await;
            session_for_progress
                .set_startup_progress(session_id, phase, percent)
                .await;
        }
    });

    let startup = match start_semantic_runtime(&settings, Some(progress_tx)).await {
        Ok(value) => value,
        Err(err) => {
            session_manager
                .set_startup_error(session_id, err.to_string())
                .await;
            let _ = progress_task.await;
            return Err(anyhow::anyhow!(err.to_string()));
        }
    };

    if let Err(err) = session_manager
        .set_startup_result(session_id, startup)
        .await
    {
        session_manager
            .set_startup_error(session_id, err.to_string())
            .await;
        let _ = progress_task.await;
        return Err(anyhow::anyhow!(err.to_string()));
    }

    let _ = progress_task.await;
    Ok(serde_json::json!({ "ok": true }))
}

fn restore_roots(roots_raw: &[String]) -> Result<(Vec<RootEntry>, Vec<RootDto>), rmcp::ErrorData> {
    if roots_raw.is_empty() {
        return Err(rmcp::ErrorData::invalid_params(
            "roots must be non-empty",
            None,
        ));
    }

    let mut roots = Vec::new();
    let mut root_dtos = Vec::new();
    let mut seen = HashSet::new();

    for root_raw in roots_raw {
        let root_path = PathBuf::from(root_raw);
        let canonical = std::fs::canonicalize(&root_path).map_err(|_| {
            rmcp::ErrorData::invalid_params(format!("root does not exist: {root_raw}"), None)
        })?;

        let metadata = std::fs::metadata(&canonical).map_err(|_| {
            rmcp::ErrorData::invalid_params(
                format!("root is not accessible: {}", canonical.display()),
                None,
            )
        })?;
        if !metadata.is_dir() {
            return Err(rmcp::ErrorData::invalid_params(
                format!("root is not a directory: {}", canonical.display()),
                None,
            ));
        }

        let root_id = root_id(&canonical);
        if !seen.insert(root_id.clone()) {
            continue;
        }
        root_dtos.push(RootDto {
            root_id: root_id.clone(),
            path: canonical.to_string_lossy().to_string(),
        });
        roots.push(RootEntry {
            root_id,
            path: canonical,
        });
    }

    Ok((roots, root_dtos))
}

#[derive(Debug, Clone)]
struct WorkspaceFile {
    root_id: String,
    root_path: PathBuf,
    rel_path: String,
    abs_path: PathBuf,
}

#[derive(Debug)]
struct DocumentSnapshot {
    file: DocumentRefDto,
    abs_path: PathBuf,
    text: String,
    version: i32,
}

fn collect_scope_files(
    roots: &[RootEntry],
    hot_set: &HashSet<DocumentKey>,
    scope: WorkspaceScope,
) -> Result<Vec<WorkspaceFile>, rmcp::ErrorData> {
    match scope {
        WorkspaceScope::Project => collect_project_files(roots),
        WorkspaceScope::Hot => collect_hot_files(roots, hot_set),
        WorkspaceScope::File { document } => {
            let key = document_key_from_ref(roots, &document)?;
            let (root_path, abs_path, file_ref) = resolve_doc_path(roots, &key)?;
            Ok(vec![WorkspaceFile {
                root_id: file_ref.root_id,
                root_path,
                rel_path: file_ref.path,
                abs_path,
            }])
        }
    }
}

fn collect_hot_files(
    roots: &[RootEntry],
    hot_set: &HashSet<DocumentKey>,
) -> Result<Vec<WorkspaceFile>, rmcp::ErrorData> {
    let mut files = Vec::new();
    for key in hot_set {
        let (root_path, abs_path, file_ref) = resolve_doc_path(roots, key)?;
        files.push(WorkspaceFile {
            root_id: file_ref.root_id,
            root_path,
            rel_path: file_ref.path,
            abs_path,
        });
    }
    files.sort_by(|a, b| {
        (a.root_id.as_str(), a.rel_path.as_str()).cmp(&(b.root_id.as_str(), b.rel_path.as_str()))
    });
    files.dedup_by(|a, b| a.root_id == b.root_id && a.rel_path == b.rel_path);
    Ok(files)
}

fn collect_project_files(roots: &[RootEntry]) -> Result<Vec<WorkspaceFile>, rmcp::ErrorData> {
    let mut files = Vec::new();
    for root in roots {
        for entry in WalkDir::new(&root.path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                if !entry.file_type().is_dir() {
                    return true;
                }
                let name = entry.file_name().to_string_lossy();
                name != ".git" && name != "target"
            })
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if entry.file_type().is_symlink() {
                continue;
            }
            if !path.is_file() {
                continue;
            }
            if !bsl_backend::system::fs_utils::is_bsl_file(path) {
                continue;
            }
            let rel_path = match path.strip_prefix(&root.path) {
                Ok(rel) => rel,
                Err(_) => continue,
            };
            let rel_path = normalize_path_components(rel_path);
            files.push(WorkspaceFile {
                root_id: root.root_id.clone(),
                root_path: root.path.clone(),
                rel_path,
                abs_path: path.to_path_buf(),
            });
        }
    }
    files.sort_by(|a, b| {
        (a.root_id.as_str(), a.rel_path.as_str()).cmp(&(b.root_id.as_str(), b.rel_path.as_str()))
    });
    files.dedup_by(|a, b| a.root_id == b.root_id && a.rel_path == b.rel_path);
    Ok(files)
}

fn normalize_path_components(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        if let Component::Normal(value) = component {
            parts.push(value.to_string_lossy().to_string());
        }
    }
    parts.join("/")
}

fn load_document_snapshot(
    file: &WorkspaceFile,
    overlays: &HashMap<DocumentKey, DocumentOverlay>,
) -> Result<Option<DocumentSnapshot>, rmcp::ErrorData> {
    let key = DocumentKey {
        root_id: file.root_id.clone(),
        path: file.rel_path.clone(),
    };

    if let Some(overlay) = overlays.get(&key) {
        return Ok(Some(DocumentSnapshot {
            file: DocumentRefDto {
                root_id: key.root_id,
                path: key.path,
            },
            abs_path: file.abs_path.clone(),
            text: overlay.text.clone(),
            version: overlay_version_i32(overlay.version),
        }));
    }

    let text = load_disk_text_with_limits(&file.root_path, &file.abs_path)?;
    Ok(text.map(|text| DocumentSnapshot {
        file: DocumentRefDto {
            root_id: key.root_id,
            path: key.path,
        },
        abs_path: file.abs_path.clone(),
        text,
        version: 0,
    }))
}

fn load_disk_text_with_limits(
    root_path: &Path,
    path: &Path,
) -> Result<Option<String>, rmcp::ErrorData> {
    let canonical = match std::fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => return Ok(None),
    };
    if !canonical.starts_with(root_path) {
        return Err(rmcp::ErrorData::invalid_params("path escapes roots", None));
    }

    let metadata = match std::fs::metadata(&canonical) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(None),
    };
    if metadata.len() > MAX_DISK_FILE_BYTES {
        return Err(rmcp::ErrorData::invalid_params(
            format!(
                "file too large: {} ({} bytes)",
                canonical.display(),
                metadata.len()
            ),
            None,
        ));
    }
    bsl_backend::system::fs_utils::read_bsl_file(&canonical)
        .map(Some)
        .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))
}

fn overlay_version_i32(version: u64) -> i32 {
    if version > i32::MAX as u64 {
        i32::MAX
    } else {
        version as i32
    }
}

fn document_key_from_ref(
    roots: &[RootEntry],
    doc: &DocumentRef,
) -> Result<DocumentKey, rmcp::ErrorData> {
    if !roots.iter().any(|root| root.root_id == doc.root_id) {
        return Err(rmcp::ErrorData::invalid_params("unknown root_id", None));
    }
    Ok(DocumentKey {
        root_id: doc.root_id.clone(),
        path: normalize_relative_path(&doc.path)?,
    })
}

fn resolve_doc_path(
    roots: &[RootEntry],
    key: &DocumentKey,
) -> Result<(PathBuf, PathBuf, DocumentRefDto), rmcp::ErrorData> {
    let root = roots
        .iter()
        .find(|root| root.root_id == key.root_id)
        .ok_or_else(|| rmcp::ErrorData::invalid_params("unknown root_id", None))?;
    let abs = root.path.join(PathBuf::from(&key.path));
    Ok((
        root.path.clone(),
        abs,
        DocumentRefDto {
            root_id: key.root_id.clone(),
            path: key.path.clone(),
        },
    ))
}

fn select_effective_text(
    file: &FileRef,
    key: &DocumentKey,
    overlays: &HashMap<DocumentKey, DocumentOverlay>,
    root_path: &Path,
    abs_path: &Path,
) -> Result<String, rmcp::ErrorData> {
    if let Some(text) = &file.text {
        if file.version.is_none() {
            return Err(rmcp::ErrorData::invalid_params(
                "version is required when text is provided",
                None,
            ));
        }
        if text.len() > MAX_OVERLAY_BYTES {
            return Err(rmcp::ErrorData::invalid_params(
                format!("overlay text exceeds MAX_OVERLAY_BYTES={MAX_OVERLAY_BYTES}"),
                None,
            ));
        }
        return Ok(text.clone());
    }

    if let Some(overlay) = overlays.get(key) {
        return Ok(overlay.text.clone());
    }

    load_disk_text_with_limits(root_path, abs_path)?
        .ok_or_else(|| rmcp::ErrorData::invalid_params("file not found", None))
}

fn select_effective_version(
    file: &FileRef,
    key: &DocumentKey,
    overlays: &HashMap<DocumentKey, DocumentOverlay>,
) -> i32 {
    if let Some(version) = file.version {
        return overlay_version_i32(version);
    }
    overlays
        .get(key)
        .map(|overlay| overlay_version_i32(overlay.version))
        .unwrap_or(0)
}

fn get_type_at_position(
    ir_program: &bsl_shared::ir::SemanticProgram,
    resolver: &TypeResolver,
    line: u32,
    column: u32,
) -> Option<bsl_shared::domain::types::TypeResolution> {
    if let Some((var_name, _type_hint, scope_id)) =
        ir_program.find_variable_with_scope(line, column)
    {
        return Some(resolver.resolve_variable_with_context(
            &var_name,
            &ir_program.symbols,
            scope_id,
        ));
    }

    let node = ir_program.find_node_at_position(line, column)?;
    match &node.kind {
        bsl_shared::ir::SemanticNodeKind::VariableDeclaration {
            type_hint: Some(resolution),
            ..
        } => Some(resolver.resolve_expression_sync(&resolution.type_name())),
        bsl_shared::ir::SemanticNodeKind::FunctionCall {
            object_type: Some(type_resolution),
            ..
        } => Some(resolver.resolve_expression_sync(&type_resolution.type_name())),
        bsl_shared::ir::SemanticNodeKind::MemberAccess { object_type, .. } => {
            Some(resolver.resolve_expression_sync(&object_type.type_name()))
        }
        bsl_shared::ir::SemanticNodeKind::NewExpression { type_name, .. } => {
            Some(resolver.resolve_expression_sync(type_name))
        }
        _ => None,
    }
}

fn span_to_range(span: bsl_shared::ir::Span) -> RangeDto {
    RangeDto {
        start: PositionDto {
            line: span.start_line,
            character: span.start_column,
        },
        end: PositionDto {
            line: span.end_line,
            character: span.end_column,
        },
    }
}

fn map_path_to_root(roots: &[RootEntry], target: &Path) -> Option<(String, String)> {
    for root in roots {
        let Ok(rel) = target.strip_prefix(&root.path) else {
            continue;
        };
        let rel_path = normalize_path_components(rel);
        return Some((root.root_id.clone(), rel_path));
    }
    None
}

fn sort_symbols(symbols: &mut [SymbolDto]) {
    symbols.sort_by(|a, b| {
        (
            a.file.root_id.as_str(),
            a.file.path.as_str(),
            sort::range_sort_key(&a.range),
            a.kind.as_str(),
            a.name.as_str(),
            a.symbol_id.as_str(),
        )
            .cmp(&(
                b.file.root_id.as_str(),
                b.file.path.as_str(),
                sort::range_sort_key(&b.range),
                b.kind.as_str(),
                b.name.as_str(),
                b.symbol_id.as_str(),
            ))
    });
}

fn sort_references(references: &mut [ReferenceDto]) {
    references.sort_by(|a, b| {
        (
            a.file.root_id.as_str(),
            a.file.path.as_str(),
            sort::range_sort_key(&a.range),
        )
            .cmp(&(
                b.file.root_id.as_str(),
                b.file.path.as_str(),
                sort::range_sort_key(&b.range),
            ))
    });
}

fn sort_pack_items(items: &mut [ContextPackItemDto]) {
    items.sort_by(|a, b| {
        let (a_root, a_path) = a
            .file
            .as_ref()
            .map(|file| (file.root_id.as_str(), file.path.as_str()))
            .unwrap_or(("", ""));
        let (b_root, b_path) = b
            .file
            .as_ref()
            .map(|file| (file.root_id.as_str(), file.path.as_str()))
            .unwrap_or(("", ""));
        let a_range = a
            .range
            .as_ref()
            .map(sort::range_sort_key)
            .unwrap_or((0, 0, 0, 0));
        let b_range = b
            .range
            .as_ref()
            .map(sort::range_sort_key)
            .unwrap_or((0, 0, 0, 0));
        (a.kind.as_str(), a_root, a_path, a_range, a.item_id.as_str()).cmp(&(
            b.kind.as_str(),
            b_root,
            b_path,
            b_range,
            b.item_id.as_str(),
        ))
    });
}

struct SnippetRender {
    text: String,
    range: RangeDto,
    truncated: bool,
}

fn render_snippet(text: &str, center_line: u32, context_lines: u32) -> SnippetRender {
    let lines: Vec<&str> = text.lines().collect();
    let total_lines = lines.len() as u32;

    let start_line = center_line.saturating_sub(context_lines);
    let end_line = if total_lines == 0 {
        0
    } else {
        center_line
            .saturating_add(context_lines)
            .min(total_lines.saturating_sub(1))
    };

    let mut rendered = String::new();
    for line_idx in start_line..=end_line {
        let raw = lines
            .get(line_idx as usize)
            .copied()
            .unwrap_or_default()
            .trim_end_matches('\r');
        let marker = if line_idx == center_line { ">" } else { " " };
        rendered.push_str(&format!(
            "{marker}{:>5} | {raw}\n",
            line_idx.saturating_add(1)
        ));
    }

    SnippetRender {
        text: rendered,
        range: RangeDto {
            start: PositionDto {
                line: start_line,
                character: 0,
            },
            end: PositionDto {
                line: end_line,
                character: 0,
            },
        },
        truncated: false,
    }
}

struct TextBudget {
    budget_chars: usize,
    used_chars: usize,
    text: String,
    truncated: bool,
}

impl TextBudget {
    fn new(budget_chars: usize) -> Self {
        Self {
            budget_chars,
            used_chars: 0,
            text: String::new(),
            truncated: false,
        }
    }

    fn push_str(&mut self, value: &str) {
        if self.truncated || self.budget_chars == 0 {
            if !value.is_empty() {
                self.truncated = true;
            }
            return;
        }

        let mut chars = value.chars();
        while self.used_chars < self.budget_chars {
            let Some(ch) = chars.next() else {
                return;
            };
            self.text.push(ch);
            self.used_chars += 1;
        }

        if chars.next().is_some() {
            self.truncated = true;
        }
    }

    fn push_line(&mut self, value: &str) {
        self.push_str(value);
        self.push_str("\n");
    }

    fn finish(self) -> String {
        self.text
    }
}

fn compute_budget_chars(budget_chars: Option<u32>, budget_tokens: Option<u32>) -> usize {
    if let Some(chars) = budget_chars {
        return chars as usize;
    }
    if let Some(tokens) = budget_tokens {
        return (tokens as usize).saturating_mul(CHARS_PER_TOKEN);
    }
    DEFAULT_BUDGET_CHARS
}

fn include_fingerprint(include: &crate::server::types::ContextInclude) -> String {
    format!(
        "snippets={};diagnostics={};types={};members={};references={};symbols={}",
        if include.snippets { 1 } else { 0 },
        if include.diagnostics { 1 } else { 0 },
        if include.types { 1 } else { 0 },
        if include.members { 1 } else { 0 },
        if include.references { 1 } else { 0 },
        if include.symbols { 1 } else { 0 },
    )
}

fn scope_key_for_pack(
    roots: &[RootEntry],
    scope: &WorkspaceScope,
) -> Result<String, rmcp::ErrorData> {
    match scope {
        WorkspaceScope::Project => Ok("project".to_string()),
        WorkspaceScope::Hot => Ok("hot".to_string()),
        WorkspaceScope::File { document } => {
            let key = document_key_from_ref(roots, document)?;
            let document_id = ids::document_id(&key.root_id, &key.path);
            Ok(format!("file:{document_id}"))
        }
    }
}

fn focus_key_for_pack(
    roots: &[RootEntry],
    focus: &ContextFocus,
) -> Result<String, rmcp::ErrorData> {
    match focus {
        ContextFocus::Diagnostic { diagnostic_id } => Ok(format!("diagnostic:{diagnostic_id}")),
        ContextFocus::Symbol { symbol_id } => Ok(format!("symbol:{symbol_id}")),
        ContextFocus::Query { query } => Ok(format!("query:{}", query.trim())),
        ContextFocus::Position { file, position } => {
            let key = document_key_from_ref(roots, &file.doc)?;
            let document_id = ids::document_id(&key.root_id, &key.path);
            Ok(format!(
                "position:{document_id}:{}:{}",
                position.line, position.character
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::JobManager;
    use crate::server::types::{
        ContextExpandParams, ContextFocus, ContextInclude, ContextPackParams, DocumentRef, FileRef,
        Position, WorkspaceOpenParams, WorkspaceScope,
    };
    use crate::types::JobStateDto;
    use std::sync::Arc;

    async fn wait_startup(job_manager: &JobManager, open: &WorkspaceOpenResponse) {
        let job_id = open
            .startup_job_id
            .as_deref()
            .expect("startup_job_id missing");
        loop {
            let status = job_manager.wait(job_id, 60_000).await.expect("job_wait");
            match status.state {
                JobStateDto::Succeeded => break,
                JobStateDto::Queued | JobStateDto::Running => continue,
                other => panic!("startup job ended unexpectedly: {}", other.as_str()),
            }
        }
    }

    #[tokio::test]
    async fn documents_set_and_clear_bump_revision_only_on_change() {
        let temp = tempfile::TempDir::new().expect("tempdir");

        let job_manager = Arc::new(JobManager::new());
        let manager = Arc::new(SessionManager::new());
        let open = manager
            .open(
                WorkspaceOpenParams {
                    roots: vec![temp.path().to_string_lossy().to_string()],
                    platform_docs_archive: None,
                    platform_version: None,
                    configuration_path: None,
                    mode: None,
                },
                Arc::clone(&job_manager),
            )
            .await
            .expect("open");
        assert_eq!(open.analysis_revision, 0);

        let root_id = open.roots[0].root_id.clone();
        let file = FileRef {
            doc: DocumentRef {
                root_id: root_id.clone(),
                path: "src/CommonModules/Foo/Module.bsl".to_string(),
            },
            text: Some("x".to_string()),
            version: Some(1),
        };

        let set = manager
            .documents_set(&open.session_id, std::slice::from_ref(&file), true)
            .await
            .expect("set");
        assert_eq!(set.analysis_revision, 1);

        let set_again = manager
            .documents_set(&open.session_id, std::slice::from_ref(&file), true)
            .await
            .expect("set again");
        assert_eq!(set_again.analysis_revision, 1);

        let clear = manager
            .documents_clear(
                &open.session_id,
                &[DocumentRef {
                    root_id,
                    path: "src/CommonModules/Foo/Module.bsl".to_string(),
                }],
                true,
            )
            .await
            .expect("clear");
        assert_eq!(clear.analysis_revision, 2);
    }

    #[tokio::test]
    async fn context_pack_and_expand_are_deterministic_and_budgeted() {
        let temp = tempfile::TempDir::new().expect("tempdir");

        let job_manager = Arc::new(JobManager::new());
        let manager = Arc::new(SessionManager::new());
        let open = manager
            .open(
                WorkspaceOpenParams {
                    roots: vec![temp.path().to_string_lossy().to_string()],
                    platform_docs_archive: None,
                    platform_version: None,
                    configuration_path: None,
                    mode: None,
                },
                Arc::clone(&job_manager),
            )
            .await
            .expect("open");
        wait_startup(job_manager.as_ref(), &open).await;

        let session_id = open.session_id.clone();
        let root_id = open.roots[0].root_id.clone();

        let overlay_file = FileRef {
            doc: DocumentRef {
                root_id: root_id.clone(),
                path: "src/CommonModules/Foo/Module.bsl".to_string(),
            },
            text: Some("Procedure Test()\n    A = 1;\nEndProcedure\n".to_string()),
            version: Some(1),
        };
        manager
            .documents_set(&session_id, std::slice::from_ref(&overlay_file), true)
            .await
            .expect("documents_set");

        let focus_file = FileRef {
            doc: overlay_file.doc.clone(),
            text: None,
            version: None,
        };

        let params = ContextPackParams {
            session_id: session_id.clone(),
            goal: Some("Test pack".to_string()),
            focus: Some(ContextFocus::Position {
                file: focus_file,
                position: Position {
                    line: 1,
                    character: 4,
                },
            }),
            scope: Some(WorkspaceScope::Hot),
            budget_chars: Some(900),
            budget_tokens: None,
            include: ContextInclude::default(),
        };

        let pack1 = manager.context_pack(params.clone()).await.expect("pack1");
        let pack2 = manager.context_pack(params).await.expect("pack2");

        assert_eq!(pack1.analysis_revision, pack2.analysis_revision);
        assert_eq!(pack1.pack_id, pack2.pack_id);
        assert_eq!(pack1.text, pack2.text);
        assert_eq!(pack1.items.len(), pack2.items.len());
        for (left, right) in pack1.items.iter().zip(pack2.items.iter()) {
            assert_eq!(left.kind, right.kind);
            assert_eq!(left.summary, right.summary);
            assert_eq!(left.file, right.file);
            assert_eq!(left.range, right.range);
            assert_eq!(left.item_id, right.item_id);
        }

        assert!(pack1.text.chars().count() <= 900);

        let mut snapshot_value = serde_json::to_value(&pack1).expect("json");
        if let Some(pack_id) = snapshot_value.get_mut("pack_id") {
            *pack_id = serde_json::Value::String("<pack_id>".to_string());
        }
        if let Some(items) = snapshot_value
            .get_mut("items")
            .and_then(|v| v.as_array_mut())
        {
            for item in items {
                if let Some(item_id) = item.get_mut("item_id") {
                    *item_id = serde_json::Value::String("<item_id>".to_string());
                }
                if let Some(file) = item.get_mut("file").and_then(|v| v.as_object_mut()) {
                    if let Some(root) = file.get_mut("root_id") {
                        *root = serde_json::Value::String("<root_id>".to_string());
                    }
                }
            }
        }

        insta::assert_json_snapshot!("context_pack_position", snapshot_value);

        let item_id = pack1.items[0].item_id.clone();
        let expand_params = ContextExpandParams {
            session_id,
            pack_id: pack1.pack_id,
            item_id,
            budget_chars: Some(400),
            budget_tokens: None,
        };
        let expand1 = manager
            .context_expand(expand_params.clone())
            .await
            .expect("expand1");
        let expand2 = manager
            .context_expand(expand_params)
            .await
            .expect("expand2");

        assert_eq!(expand1.analysis_revision, expand2.analysis_revision);
        assert_eq!(expand1.text, expand2.text);
        assert!(expand1.text.chars().count() <= 400);

        insta::assert_json_snapshot!(
            "context_expand_snippet",
            serde_json::to_value(expand1).expect("json")
        );
    }
}
