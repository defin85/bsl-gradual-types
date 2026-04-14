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
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, Notify, RwLock};
use tokio::task::JoinHandle;
use tower_lsp::Client;

use bsl_analysis_v2::{DepsSnapshotId, FileId as V2FileId, SettingsId};
use bsl_backend::system::SystemCoordinator;
use bsl_shared::api::dtos::SnapshotReadinessDto;

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

pub(crate) const COMPLETION_TIMELINE_VERSION: u32 = 25;
pub(crate) const COMPLETION_TIMELINE_MAX_ENTRIES: usize = 200;
pub(crate) const DIAGNOSTICS_SAVE_TIMELINE_VERSION: u32 = 12;
pub(crate) const DIAGNOSTICS_SAVE_TIMELINE_MAX_ENTRIES: usize = 200;
pub(crate) const DID_CHANGE_PARSE_SNAPSHOT_EVIDENCE_VERSION: u32 = 3;
pub(crate) const DID_CHANGE_PARSE_SNAPSHOT_EVIDENCE_MAX_ENTRIES: usize = 200;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CompletionResponseEgressTraceInputs {
    pub response_sent_at_ms: u64,
    pub response_output_handoff_started_at_ms: Option<u64>,
    pub response_output_handoff_enqueued_at_ms: Option<u64>,
    pub response_output_enqueue_completed_at_ms: Option<u64>,
    pub response_output_encode_started_at_ms: Option<u64>,
    pub response_output_write_started_at_ms: Option<u64>,
    pub response_output_encode_completed_at_ms: Option<u64>,
    pub response_flush_completed_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CompletionResponseEgressDerivedTrace {
    pub response_ready_to_output_handoff_wait_ms: Option<u64>,
    pub response_output_handoff_send_wait_ms: Option<u64>,
    pub response_output_handoff_to_writer_wait_ms: Option<u64>,
    pub response_ready_to_output_enqueue_wait_ms: Option<u64>,
    pub response_output_queue_wait_ms: Option<u64>,
    pub response_output_encode_exec_ms: Option<u64>,
    pub response_output_write_and_flush_exec_ms: Option<u64>,
    pub response_ready_to_flush_wait_ms: Option<u64>,
}

pub(crate) fn derive_completion_response_egress_trace(
    inputs: CompletionResponseEgressTraceInputs,
) -> CompletionResponseEgressDerivedTrace {
    CompletionResponseEgressDerivedTrace {
        response_ready_to_output_handoff_wait_ms: inputs.response_output_handoff_started_at_ms.map(
            |handoff_started_at_ms| {
                handoff_started_at_ms.saturating_sub(inputs.response_sent_at_ms)
            },
        ),
        response_output_handoff_send_wait_ms: inputs
            .response_output_handoff_started_at_ms
            .zip(inputs.response_output_handoff_enqueued_at_ms)
            .map(|(handoff_started_at_ms, handoff_enqueued_at_ms)| {
                handoff_enqueued_at_ms.saturating_sub(handoff_started_at_ms)
            }),
        response_output_handoff_to_writer_wait_ms: inputs
            .response_output_handoff_enqueued_at_ms
            .zip(inputs.response_output_enqueue_completed_at_ms)
            .map(|(handoff_enqueued_at_ms, enqueue_completed_at_ms)| {
                enqueue_completed_at_ms.saturating_sub(handoff_enqueued_at_ms)
            }),
        response_ready_to_output_enqueue_wait_ms: inputs
            .response_output_enqueue_completed_at_ms
            .map(|enqueue_completed_at_ms| {
                enqueue_completed_at_ms.saturating_sub(inputs.response_sent_at_ms)
            }),
        response_output_queue_wait_ms: inputs
            .response_output_enqueue_completed_at_ms
            .zip(inputs.response_output_encode_started_at_ms)
            .map(|(enqueue_completed_at_ms, encode_started_at_ms)| {
                encode_started_at_ms.saturating_sub(enqueue_completed_at_ms)
            }),
        response_output_encode_exec_ms: inputs
            .response_output_encode_started_at_ms
            .zip(inputs.response_output_encode_completed_at_ms)
            .map(|(encode_started_at_ms, encode_completed_at_ms)| {
                encode_completed_at_ms.saturating_sub(encode_started_at_ms)
            }),
        response_output_write_and_flush_exec_ms: inputs
            .response_output_write_started_at_ms
            .zip(inputs.response_flush_completed_at_ms)
            .map(|(write_started_at_ms, flush_completed_at_ms)| {
                flush_completed_at_ms.saturating_sub(write_started_at_ms)
            }),
        response_ready_to_flush_wait_ms: inputs.response_flush_completed_at_ms.map(
            |flush_completed_at_ms| {
                flush_completed_at_ms.saturating_sub(inputs.response_sent_at_ms)
            },
        ),
    }
}

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
    pub(crate) file_id_to_uri_v2: Arc<RwLock<HashMap<V2FileId, Url>>>,
    pub(crate) next_file_id_v2: Arc<AtomicU32>,
    pub(crate) diagnostics_tasks_v2: Arc<Mutex<DiagnosticsTasksV2>>,
    pub(crate) type_index_precompute_tasks_v2: Arc<Mutex<TypeIndexPrecomputeTasksV2>>,
    pub(crate) current_revision_head_precompute_tasks_v2:
        Arc<Mutex<CurrentRevisionHeadPrecomputeTasksV2>>,
    pub(crate) background_parse_snapshot_apply_tasks_v2:
        Arc<Mutex<BackgroundParseSnapshotApplyTasksV2>>,
    pub(crate) document_symbol_bootstrap_tasks_v2: Arc<Mutex<DocumentSymbolBootstrapTasksV2>>,
    pub(crate) diagnostics_generation_v2: Arc<RwLock<HashMap<V2FileId, u64>>>,
    pub(crate) diagnostics_save_cycle_sequence_v2: Arc<RwLock<HashMap<V2FileId, u64>>>,
    pub(crate) latest_received_file_versions_v2: Arc<RwLock<HashMap<V2FileId, i32>>>,
    /// Tracks the freshest file version whose current-revision handoff has already been pushed
    /// into the runtime writer queue. This is stricter than `latest_received_file_versions_v2`,
    /// which may advance before the runtime handoff is actually enqueued.
    pub(crate) latest_current_revision_handoff_versions_v2: Arc<RwLock<HashMap<V2FileId, i32>>>,
    pub(crate) latest_document_shadow_state_v2:
        Arc<RwLock<HashMap<V2FileId, DocumentShadowStateV2>>>,
    pub(crate) latest_ready_parse_snapshots_v2:
        Arc<RwLock<HashMap<V2FileId, ReadyParseSnapshotStateV2>>>,
    pub(crate) latest_snapshot_failures_v2:
        Arc<RwLock<HashMap<V2FileId, SnapshotBuildFailureStateV2>>>,
    pub(crate) latest_snapshot_status_v2: Arc<RwLock<HashMap<V2FileId, SnapshotReadinessDto>>>,
    pub(crate) latest_save_fastlane_syntax_artifacts_v2:
        Arc<RwLock<HashMap<V2FileId, SaveFastlaneSyntaxArtifactsV2>>>,
    pub(crate) latest_apply_enqueued_at_v2: Arc<RwLock<HashMap<V2FileId, Instant>>>,
    pub(crate) latest_diagnostics_publish_state_v2:
        Arc<RwLock<HashMap<V2FileId, DiagnosticsPublishedStateV2>>>,
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
    pub(crate) current_context_latest_generations:
        Arc<command_handlers::CurrentContextLatestGenerationRegistry>,
    pub(crate) current_context_generation_notify: Arc<Notify>,
    pub(crate) current_context_parse_broker: Arc<command_handlers::CurrentContextParseBroker>,
    pub(crate) completion_timeline_traces:
        Arc<StdMutex<VecDeque<crate::types::CompletionTimelineTrace>>>,
    pub(crate) next_completion_timeline_trace_id: Arc<AtomicU64>,
    pub(crate) diagnostics_save_timeline_store: Arc<StdMutex<DiagnosticsSaveTimelineStore>>,
    pub(crate) did_change_parse_snapshot_evidence_store:
        Arc<StdMutex<DidChangeParseSnapshotEvidenceStore>>,
    pub(crate) diagnostics_did_save_followup_lane_v2:
        Arc<StdMutex<DiagnosticsDidSaveFollowupLaneStateV2>>,
    pub(crate) diagnostics_did_save_followup_lane_notify_v2: Arc<Notify>,
    pub(crate) next_diagnostics_save_timeline_trace_id: Arc<AtomicU64>,
    pub(crate) next_did_change_parse_snapshot_evidence_id: Arc<AtomicU64>,
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
    pub save_cycle_sequence: Option<u64>,
    pub requested_version: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DiagnosticsPublishedStateV2 {
    pub requested_version: i32,
    pub diagnostics_generation: u64,
    pub publish_rank: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct DiagnosticsSaveTimelineCycleKey {
    pub file_id: V2FileId,
    pub diagnostics_generation: u64,
    pub save_cycle_sequence: u64,
    pub requested_version: i32,
}

#[derive(Debug, Default)]
pub(crate) struct DiagnosticsSaveTimelineTerminalKeyStore {
    pub keys: HashSet<DiagnosticsSaveTimelineCycleKey>,
    pub order: VecDeque<DiagnosticsSaveTimelineCycleKey>,
}

#[derive(Debug, Default)]
pub(crate) struct DiagnosticsSaveTimelineStore {
    pub active_cycles:
        HashMap<DiagnosticsSaveTimelineCycleKey, crate::types::DiagnosticsSaveTimelineTrace>,
    pub terminal_keys: DiagnosticsSaveTimelineTerminalKeyStore,
    pub traces: VecDeque<crate::types::DiagnosticsSaveTimelineTrace>,
}

#[derive(Debug, Default)]
pub(crate) struct DiagnosticsDidSaveFollowupLaneStateV2 {
    pub active_slots: usize,
    pub queued_files: VecDeque<V2FileId>,
    pub queued_set: HashSet<V2FileId>,
}

#[derive(Debug, Clone)]
pub(crate) struct DiagnosticsSaveTimelineProfileResult {
    pub profile: bsl_runtime::application::DiagnosticsProfile,
    pub disposition: bsl_runtime::application::DiagnosticsDisposition,
    pub publish: Option<crate::types::DiagnosticsSaveTimelinePublishTrace>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadyParseSnapshotAttributionPhaseV2 {
    Waiting = 1,
    ParseExec = 2,
    PostParsePreMaterialization = 3,
    ReadyInstall = 4,
    DocumentSymbolSideWork = 5,
}

impl ReadyParseSnapshotAttributionPhaseV2 {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::ParseExec => "parse_exec",
            Self::PostParsePreMaterialization => "post_parse_pre_materialization",
            Self::ReadyInstall => "ready_install",
            Self::DocumentSymbolSideWork => "document_symbol_side_work",
        }
    }

    fn tracks_latency(self) -> bool {
        !matches!(self, Self::Waiting)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ReadyParseSnapshotPhaseAttributionV2 {
    pub parse_exec_ms: Option<u64>,
    pub post_parse_pre_materialization_ms: Option<u64>,
    pub ready_install_ms: Option<u64>,
    pub document_symbol_side_work_ms: Option<u64>,
}

impl ReadyParseSnapshotPhaseAttributionV2 {
    pub(crate) fn dominant_phase(self: &Self) -> Option<(&'static str, u64)> {
        [
            ("parse_exec", self.parse_exec_ms),
            (
                "post_parse_pre_materialization",
                self.post_parse_pre_materialization_ms,
            ),
            ("ready_install", self.ready_install_ms),
            (
                "document_symbol_side_work",
                self.document_symbol_side_work_ms,
            ),
        ]
        .into_iter()
        .filter_map(|(phase, duration_ms)| duration_ms.map(|value| (phase, value)))
        .max_by_key(|(_, duration_ms)| *duration_ms)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ReadyParseSnapshotPhaseAttributionSnapshotV2 {
    pub current_phase: Option<ReadyParseSnapshotAttributionPhaseV2>,
    pub current_phase_elapsed_ms: Option<u64>,
    pub completed: ReadyParseSnapshotPhaseAttributionV2,
}

impl ReadyParseSnapshotPhaseAttributionSnapshotV2 {
    pub(crate) fn dominant_phase(self: &Self) -> Option<(&'static str, u64)> {
        let completed = self.completed.dominant_phase();
        let current = self
            .current_phase
            .zip(self.current_phase_elapsed_ms)
            .filter(|(phase, _)| phase.tracks_latency())
            .map(|(phase, duration_ms)| (phase.as_str(), duration_ms));
        match (completed, current) {
            (Some(left), Some(right)) => Some(if left.1 >= right.1 { left } else { right }),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (None, None) => None,
        }
    }
}

#[derive(Debug, Default)]
struct ReadyParseSnapshotPhaseAttributionStateV2 {
    current_phase: Option<ReadyParseSnapshotAttributionPhaseV2>,
    current_phase_started_at: Option<Instant>,
    completed: ReadyParseSnapshotPhaseAttributionV2,
}

impl ReadyParseSnapshotPhaseAttributionStateV2 {
    fn reset(&mut self) {
        self.current_phase = None;
        self.current_phase_started_at = None;
        self.completed = ReadyParseSnapshotPhaseAttributionV2::default();
    }

    fn transition_to(&mut self, phase: ReadyParseSnapshotAttributionPhaseV2, now: Instant) {
        self.finish_current(now);
        self.current_phase = Some(phase);
        self.current_phase_started_at = Some(now);
    }

    fn finish_current(&mut self, now: Instant) {
        let Some(phase) = self.current_phase.take() else {
            self.current_phase_started_at = None;
            return;
        };
        let elapsed_ms = self
            .current_phase_started_at
            .take()
            .map(|started_at| duration_to_u64_ms(now.saturating_duration_since(started_at)));
        match phase {
            ReadyParseSnapshotAttributionPhaseV2::Waiting => {}
            ReadyParseSnapshotAttributionPhaseV2::ParseExec => {
                self.completed.parse_exec_ms = elapsed_ms;
            }
            ReadyParseSnapshotAttributionPhaseV2::PostParsePreMaterialization => {
                self.completed.post_parse_pre_materialization_ms = elapsed_ms;
            }
            ReadyParseSnapshotAttributionPhaseV2::ReadyInstall => {
                self.completed.ready_install_ms = elapsed_ms;
            }
            ReadyParseSnapshotAttributionPhaseV2::DocumentSymbolSideWork => {
                self.completed.document_symbol_side_work_ms = elapsed_ms;
            }
        }
    }

    fn snapshot(&self, now: Instant) -> ReadyParseSnapshotPhaseAttributionSnapshotV2 {
        ReadyParseSnapshotPhaseAttributionSnapshotV2 {
            current_phase: self.current_phase,
            current_phase_elapsed_ms: self
                .current_phase_started_at
                .map(|started_at| duration_to_u64_ms(now.saturating_duration_since(started_at))),
            completed: self.completed.clone(),
        }
    }
}

fn duration_to_u64_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

#[derive(Debug, Clone)]
pub(crate) struct ReadyParseSnapshotStateV2 {
    pub text: Arc<str>,
    pub parse_snapshot: bsl_analysis_v2::ParseSnapshot,
    pub source: BackgroundParseSnapshotApplyTaskSourceV2,
    pub phase_attribution: ReadyParseSnapshotPhaseAttributionV2,
}

#[derive(Debug, Clone)]
pub(crate) struct SnapshotBuildFailureStateV2 {
    pub requested_version: i32,
    pub reason: Arc<str>,
}

#[derive(Debug, Clone)]
pub(crate) struct SaveFastlaneSyntaxArtifactsV2 {
    pub version: i32,
    pub syntax_errors: Arc<Vec<bsl_shared::domain::types::ParseError>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct DidChangeParseSnapshotEvidenceKey {
    pub file_id: V2FileId,
    pub requested_version: i32,
}

#[derive(Debug, Default)]
pub(crate) struct DidChangeParseSnapshotEvidenceStore {
    pub entries: HashMap<
        DidChangeParseSnapshotEvidenceKey,
        crate::types::DidChangeParseSnapshotEvidenceTrace,
    >,
    pub order: VecDeque<DidChangeParseSnapshotEvidenceKey>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundParseSnapshotApplyTaskSourceV2 {
    DidOpen,
    DidChange,
    DidSave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParseSnapshotAsyncDelayMode {
    None,
    DidChangeTestOnly,
    DidSaveTestOnly,
}

#[derive(Debug, Clone)]
pub(crate) struct DidChangeStaleParserBaseAttributionV2 {
    pub root_cause: &'static str,
    pub shadow_document_version: i32,
    pub latest_ready_document_version: Option<i32>,
    pub matching_ready_snapshot_for_shadow_state: bool,
    pub ready_snapshot_prime_attempted: bool,
    pub tree_cache_matches_shadow_text_after_prime: Option<bool>,
}

#[derive(Debug, Clone)]
pub(crate) struct DidChangeParseSnapshotAttributionV2 {
    pub uri: Url,
    pub base_text_source: &'static str,
    pub change_shape: &'static str,
    pub content_changes_count: usize,
    pub replay_order: &'static str,
    pub base_document_version: Option<i32>,
    pub stale_parser_base: Option<DidChangeStaleParserBaseAttributionV2>,
}

#[derive(Debug, Clone)]
pub(crate) struct BackgroundParseSnapshotApplyTargetV2 {
    pub requested_version: i32,
    pub text_hash: [u8; 32],
    pub source: BackgroundParseSnapshotApplyTaskSourceV2,
    pub path: Arc<str>,
    pub text: Arc<str>,
    pub parser_base_recovery_text: Option<Arc<str>>,
    pub parser_edits: Vec<bsl_runtime::system::parser_coordinator::TextEdit>,
    pub forced_full_parse_reason: Option<&'static str>,
    pub async_delay_mode: ParseSnapshotAsyncDelayMode,
    pub blocking_delay_env_key: Option<&'static str>,
    pub did_change_attribution: Option<DidChangeParseSnapshotAttributionV2>,
    pub epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundParseSnapshotApplyTaskPhaseV2 {
    Waiting = 1,
    Parsing = 2,
    Materializing = 3,
}

impl BackgroundParseSnapshotApplyTaskPhaseV2 {
    pub(crate) fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            x if x == Self::Waiting as u8 => Some(Self::Waiting),
            x if x == Self::Parsing as u8 => Some(Self::Parsing),
            x if x == Self::Materializing as u8 => Some(Self::Materializing),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct BackgroundParseSnapshotApplyTaskControlV2 {
    pub cancel_requested: AtomicBool,
    pub retarget_requested: AtomicBool,
    pub promotion_requested: AtomicBool,
    pub materialized: AtomicBool,
    pub phase: AtomicU8,
    phase_attribution: StdMutex<ReadyParseSnapshotPhaseAttributionStateV2>,
    pub control_notify: Notify,
    pub materialized_notify: Notify,
}

impl BackgroundParseSnapshotApplyTaskControlV2 {
    pub(crate) fn new() -> Self {
        Self {
            cancel_requested: AtomicBool::new(false),
            retarget_requested: AtomicBool::new(false),
            promotion_requested: AtomicBool::new(false),
            materialized: AtomicBool::new(false),
            phase: AtomicU8::new(0),
            phase_attribution: StdMutex::new(ReadyParseSnapshotPhaseAttributionStateV2::default()),
            control_notify: Notify::new(),
            materialized_notify: Notify::new(),
        }
    }

    pub(crate) fn reset_phase_attribution(&self) {
        self.phase_attribution
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .reset();
    }

    pub(crate) fn transition_phase_attribution(&self, phase: ReadyParseSnapshotAttributionPhaseV2) {
        self.phase_attribution
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .transition_to(phase, Instant::now());
    }

    pub(crate) fn finish_phase_attribution(&self) -> ReadyParseSnapshotPhaseAttributionSnapshotV2 {
        let mut guard = self
            .phase_attribution
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = Instant::now();
        guard.finish_current(now);
        guard.snapshot(now)
    }

    pub(crate) fn phase_attribution_snapshot(
        &self,
    ) -> ReadyParseSnapshotPhaseAttributionSnapshotV2 {
        self.phase_attribution
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .snapshot(Instant::now())
    }
}

pub(crate) struct BackgroundParseSnapshotApplyTaskV2 {
    pub target_epoch: Arc<AtomicU64>,
    pub target: Arc<StdMutex<BackgroundParseSnapshotApplyTargetV2>>,
    pub control: Arc<BackgroundParseSnapshotApplyTaskControlV2>,
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
