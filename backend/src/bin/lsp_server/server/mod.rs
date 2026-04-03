//! BSL Language Server implementation
//!
//! Contains the main server struct and LanguageServer trait implementation.
//!
//! This module is split into submodules:
//! - `core`: Constructor and helper methods
//! - `language_server`: Full LanguageServer trait implementation
//! - `command_handlers`: Command-specific handlers

mod analysis_v2_runtime;
mod command_handlers;
mod completion_cancellation;
mod completion_dispatcher;
mod core;
mod language_server;
pub(crate) mod request_context;
pub(crate) mod transport_adapter;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tower_lsp::Client;

use bsl_analysis_v2::{DepsSnapshotId, FileId as V2FileId, SettingsId};
use bsl_backend::system::SystemCoordinator;

use crate::config::{BslSettings, LspConfig};
use crate::types::GetIndexStateResponse;

// Re-export Url for use in submodules
pub use tower_lsp::lsp_types::Url;
pub(crate) use transport_adapter::serve_with_completion_handoff;

use self::analysis_v2_runtime::AnalysisV2Runtime;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum V2FileKey {
    /// Preferred key: filesystem path derived from `Url::to_file_path()`.
    Path(PathBuf),
    /// Fallback key for non-file documents.
    Url(String),
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FormattingCapabilityState {
    pub dynamic_document_formatting: bool,
    pub dynamic_range_formatting: bool,
    pub registered: bool,
    pub in_flight: bool,
    pub desired_enabled: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct InlayHintsCapabilityState {
    pub dynamic_registration: bool,
    pub registered: bool,
    pub in_flight: bool,
    pub desired_enabled: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CodeActionsCapabilityState {
    pub dynamic_registration: bool,
    pub registered: bool,
    pub in_flight: bool,
    pub desired_enabled: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct DocumentShadowStateV2 {
    pub version: i32,
    pub text: Arc<str>,
}

#[derive(Debug, Clone)]
pub(crate) struct ScaleAwareChurnStateV2 {
    pub window_started_at: Instant,
    pub changes_in_window: u32,
    pub large_churn_active: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CompletionParityStateV2 {
    pub trigger_character_non_empty: Option<bool>,
    pub invoked_non_empty: Option<bool>,
    pub trigger_character_labels: Option<Vec<String>>,
    pub invoked_labels: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionHeadServeObservationV2 {
    pub file_version: i32,
    pub deps_id: DepsSnapshotId,
    pub settings_id: Option<SettingsId>,
    pub served_at: Instant,
}

#[derive(Debug, Clone)]
pub(crate) struct DocumentSymbolReadyStateV2 {
    pub file_version: i32,
    pub response: tower_lsp::lsp_types::DocumentSymbolResponse,
}

pub(crate) type CompletionParityKeyV2 = (V2FileId, i32, u32, u32);
pub(crate) type CompletionParityStoreV2 =
    Arc<RwLock<HashMap<CompletionParityKeyV2, CompletionParityStateV2>>>;

pub(crate) const COMPLETION_TIMELINE_VERSION: u32 = 20;
pub(crate) const COMPLETION_TIMELINE_MAX_ENTRIES: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FullIndexStateKind {
    Idle,
    Running,
    Ready,
    Failed,
}

impl FullIndexStateKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FullIndexOperationKind {
    Startup,
    BuildIndex,
}

impl FullIndexOperationKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::BuildIndex => "buildIndex",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FullIndexRuntimeState {
    pub state: FullIndexStateKind,
    pub active_operation: Option<FullIndexOperationKind>,
    pub operation_id: Option<String>,
    pub message: Option<String>,
    pub updated_at_ms: u64,
}

impl Default for FullIndexRuntimeState {
    fn default() -> Self {
        Self {
            state: FullIndexStateKind::Idle,
            active_operation: None,
            operation_id: None,
            message: None,
            updated_at_ms: unix_timestamp_ms(),
        }
    }
}

impl FullIndexRuntimeState {
    pub(crate) fn to_response(&self) -> GetIndexStateResponse {
        GetIndexStateResponse {
            version: 1,
            state: self.state.as_str().to_string(),
            ready: self.state == FullIndexStateKind::Ready,
            active_operation: self.active_operation.map(|op| op.as_str().to_string()),
            operation_id: self.operation_id.clone(),
            message: self.message.clone(),
            updated_at_ms: self.updated_at_ms,
        }
    }
}

pub(crate) fn unix_timestamp_ms() -> u64 {
    let now = SystemTime::now();
    let duration = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_millis(0));
    duration.as_millis().min(u64::MAX as u128) as u64
}

/// BSL Language Server backend - CLEAN ARCHITECTURE
#[derive(Clone)]
pub struct BslLanguageServer {
    pub(crate) client: Client,
    pub(crate) diagnostics_counts: Arc<RwLock<HashMap<Url, usize>>>,
    pub(crate) config: Arc<RwLock<Option<LspConfig>>>,
    pub(crate) settings: Arc<RwLock<BslSettings>>,
    pub(crate) completion_snippet_support: Arc<RwLock<bool>>,
    pub(crate) auto_reindex_paused: Arc<RwLock<bool>>,
    pub(crate) coordinator: Arc<SystemCoordinator>,
    pub(crate) formatting_capability: Arc<RwLock<FormattingCapabilityState>>,
    pub(crate) inlay_hints_capability: Arc<RwLock<InlayHintsCapabilityState>>,
    pub(crate) code_actions_capability: Arc<RwLock<CodeActionsCapabilityState>>,

    pub(crate) analysis_v2: AnalysisV2Runtime,
    /// Serializes `didOpen/didChange/didClose` updates so that incremental changes are applied
    /// against a consistent base text (source of truth lives in `analysis-v2` inputs).
    pub(crate) text_sync_v2: Arc<Mutex<()>>,
    /// Session-stable mapping: once a `FileId` is assigned for a key, it is not revoked for the
    /// lifetime of the server process (even if the document is closed and re-opened).
    pub(crate) file_key_to_file_id_v2: Arc<RwLock<HashMap<V2FileKey, V2FileId>>>,
    pub(crate) next_file_id_v2: Arc<AtomicU32>,
    pub(crate) diagnostics_tasks_v2: Arc<Mutex<DiagnosticsTasksV2>>,
    pub(crate) type_index_precompute_tasks_v2: Arc<Mutex<TypeIndexPrecomputeTasksV2>>,
    pub(crate) current_revision_head_precompute_tasks_v2:
        Arc<Mutex<CurrentRevisionHeadPrecomputeTasksV2>>,
    pub(crate) background_parse_snapshot_apply_tasks_v2:
        Arc<Mutex<BackgroundParseSnapshotApplyTasksV2>>,
    pub(crate) document_symbol_bootstrap_tasks_v2: Arc<Mutex<DocumentSymbolBootstrapTasksV2>>,
    pub(crate) diagnostics_generation_v2: Arc<RwLock<HashMap<V2FileId, u64>>>,
    pub(crate) latest_received_file_versions_v2: Arc<RwLock<HashMap<V2FileId, i32>>>,
    pub(crate) latest_document_shadow_state_v2:
        Arc<RwLock<HashMap<V2FileId, DocumentShadowStateV2>>>,
    pub(crate) latest_apply_enqueued_at_v2: Arc<RwLock<HashMap<V2FileId, Instant>>>,
    pub(crate) scale_aware_churn_state_v2: Arc<RwLock<HashMap<V2FileId, ScaleAwareChurnStateV2>>>,
    pub(crate) document_symbol_ready_cache_v2:
        Arc<RwLock<HashMap<V2FileId, DocumentSymbolReadyStateV2>>>,
    pub(crate) document_symbol_request_epochs_v2: Arc<RwLock<HashMap<V2FileId, u64>>>,
    pub(crate) completion_seen_files_v2: Arc<RwLock<HashSet<V2FileId>>>,
    pub(crate) completion_parity_state_v2: CompletionParityStoreV2,
    pub(crate) completion_head_serve_observations_v2:
        Arc<RwLock<HashMap<V2FileId, CompletionHeadServeObservationV2>>>,
    pub(crate) completion_dispatcher_v2: Arc<completion_dispatcher::CompletionDispatcherRegistry>,
    pub(crate) completion_cancellation_registry_v2:
        Arc<completion_cancellation::CompletionCancellationRegistry>,
    pub(crate) last_deps_id_v2: Arc<RwLock<Option<DepsSnapshotId>>>,
    pub(crate) last_settings_id_v2: Arc<RwLock<Option<SettingsId>>>,
    pub(crate) full_index_state: Arc<Mutex<FullIndexRuntimeState>>,
    pub(crate) next_full_index_operation_id: Arc<AtomicU64>,
    pub(crate) full_index_watchdog_timeout: Duration,
    pub(crate) completion_timeline_traces:
        Arc<Mutex<VecDeque<crate::types::CompletionTimelineTrace>>>,
    pub(crate) next_completion_timeline_trace_id: Arc<AtomicU64>,
    pub(crate) next_document_symbol_request_epoch_v2: Arc<AtomicU64>,
    pub(crate) next_type_index_precompute_task_id: Arc<AtomicU64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct DiagnosticsTaskKeyV2 {
    pub file_id: V2FileId,
    pub profile: bsl_runtime::application::DiagnosticsProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct DiagnosticsSupersessionKeyV2 {
    pub file_id: V2FileId,
    pub profile: bsl_runtime::application::DiagnosticsProfile,
    pub diagnostics_generation: u64,
    pub requested_version: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticsCancellationReasonV2 {
    SupersededGeneration = 1,
    SupersededVersion = 2,
    ClientCancel = 3,
    OtherCancel = 4,
}

impl DiagnosticsCancellationReasonV2 {
    fn from_code(code: u8) -> Self {
        match code {
            x if x == Self::SupersededGeneration as u8 => Self::SupersededGeneration,
            x if x == Self::SupersededVersion as u8 => Self::SupersededVersion,
            x if x == Self::ClientCancel as u8 => Self::ClientCancel,
            _ => Self::OtherCancel,
        }
    }

    pub(crate) fn for_supersession(
        previous: DiagnosticsSupersessionKeyV2,
        next_generation: u64,
        next_version: i32,
    ) -> Self {
        if next_generation != previous.diagnostics_generation {
            Self::SupersededGeneration
        } else if next_version != previous.requested_version {
            Self::SupersededVersion
        } else {
            Self::OtherCancel
        }
    }

    pub(crate) fn to_disposition(self) -> bsl_runtime::application::DiagnosticsDisposition {
        match self {
            Self::SupersededGeneration => {
                bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration
            }
            Self::SupersededVersion => {
                bsl_runtime::application::DiagnosticsDisposition::SupersededVersion
            }
            Self::ClientCancel => bsl_runtime::application::DiagnosticsDisposition::ClientCancel,
            Self::OtherCancel => bsl_runtime::application::DiagnosticsDisposition::OtherCancel,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DiagnosticsCancellationTokenV2 {
    cancelled: Arc<AtomicBool>,
    reason_code: Arc<AtomicU8>,
}

impl DiagnosticsCancellationTokenV2 {
    pub(crate) fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            reason_code: Arc::new(AtomicU8::new(
                DiagnosticsCancellationReasonV2::OtherCancel as u8,
            )),
        }
    }

    pub(crate) fn cancel(&self, reason: DiagnosticsCancellationReasonV2) {
        self.reason_code.store(reason as u8, Ordering::SeqCst);
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub(crate) fn reason(&self) -> DiagnosticsCancellationReasonV2 {
        DiagnosticsCancellationReasonV2::from_code(self.reason_code.load(Ordering::SeqCst))
    }

    pub(crate) fn same_inner(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cancelled, &other.cancelled)
    }
}

pub(crate) struct DiagnosticsTaskV2 {
    pub supersession_key: DiagnosticsSupersessionKeyV2,
    pub cancel_token: DiagnosticsCancellationTokenV2,
    pub trigger: bsl_runtime::application::DiagnosticsTrigger,
    pub debounce: bool,
    pub handle: JoinHandle<()>,
}

type DiagnosticsTasksV2 = HashMap<DiagnosticsTaskKeyV2, DiagnosticsTaskV2>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TypeIndexPrecomputeSupersessionKeyV2 {
    pub file_id: V2FileId,
    pub requested_version: i32,
}

pub(crate) struct TypeIndexPrecomputeTaskV2 {
    pub task_id: u64,
    pub supersession_key: TypeIndexPrecomputeSupersessionKeyV2,
    pub work_class: bsl_runtime::application::CpuWorkClass,
    pub phase: Arc<AtomicU8>,
    pub active_requested_version: Arc<std::sync::atomic::AtomicI32>,
    pub scheduled_at: Instant,
    pub handle: JoinHandle<()>,
}

type TypeIndexPrecomputeTasksV2 = HashMap<V2FileId, TypeIndexPrecomputeTaskV2>;

pub(crate) struct CurrentRevisionHeadPrecomputeTaskV2 {
    pub requested_version: Arc<AtomicI32>,
    pub handle: JoinHandle<()>,
}

type CurrentRevisionHeadPrecomputeTasksV2 = HashMap<V2FileId, CurrentRevisionHeadPrecomputeTaskV2>;

pub(crate) struct BackgroundParseSnapshotApplyTaskV2 {
    pub requested_version: Arc<AtomicI32>,
    pub handle: JoinHandle<()>,
}

type BackgroundParseSnapshotApplyTasksV2 = HashMap<V2FileId, BackgroundParseSnapshotApplyTaskV2>;

pub(crate) struct DocumentSymbolBootstrapTaskV2 {
    pub requested_version: Arc<AtomicI32>,
    pub handle: JoinHandle<()>,
}

type DocumentSymbolBootstrapTasksV2 = HashMap<V2FileId, DocumentSymbolBootstrapTaskV2>;

pub(crate) fn intellisense_v2_slow_wait_warn_threshold() -> Option<Duration> {
    bsl_runtime::application::RuntimePerfKnobs::from_runtime_config().slow_wait_warn_threshold
}

pub(crate) fn intellisense_v2_slow_snapshot_warn_threshold() -> Option<Duration> {
    bsl_runtime::application::RuntimePerfKnobs::from_runtime_config().slow_snapshot_warn_threshold
}

pub(crate) fn intellisense_v2_slow_query_warn_threshold() -> Option<Duration> {
    bsl_runtime::application::RuntimePerfKnobs::from_runtime_config().slow_query_warn_threshold
}

pub(crate) fn intellisense_v2_slow_client_log_threshold() -> Option<Duration> {
    bsl_runtime::application::RuntimePerfKnobs::from_runtime_config().slow_client_log_threshold
}
