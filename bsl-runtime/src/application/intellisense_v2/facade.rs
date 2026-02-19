use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Condvar, OnceLock};
use std::time::{Duration, Instant};

use super::policy::{
    interactive_freshness_knobs, should_query_parse_result, InteractiveFreshnessKnobs,
};
use tokio::sync::oneshot;
use tracing::warn;

use crate::system::{IndexSnapshot, IndexSnapshotId, SystemCoordinator};
use bsl_analysis_v2::{
    AnalysisHostV2, AnalysisV2, Change, DepsSnapshotId, FileId, SemanticDeps, SettingsId,
};
use bsl_shared::domain::types::ParseError;
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::SemanticProgram;

/// Canonical semantic operations expected from IntelliSense v2 adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticOperation {
    Completion,
    Hover,
    SignatureHelp,
    Definition,
    DocumentSymbol,
    Rename,
    Diagnostics,
    Members,
    TypeAtPosition,
    SymbolSearch,
    References,
}

impl SemanticOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            SemanticOperation::Completion => "completion",
            SemanticOperation::Hover => "hover",
            SemanticOperation::SignatureHelp => "signature_help",
            SemanticOperation::Definition => "definition",
            SemanticOperation::DocumentSymbol => "document_symbol",
            SemanticOperation::Rename => "rename",
            SemanticOperation::Diagnostics => "diagnostics",
            SemanticOperation::Members => "members",
            SemanticOperation::TypeAtPosition => "type_at_position",
            SemanticOperation::SymbolSearch => "symbol_search",
            SemanticOperation::References => "references",
        }
    }
}

fn runtime_work_class_for_operation(operation: SemanticOperation) -> &'static str {
    match operation {
        SemanticOperation::Completion
        | SemanticOperation::Hover
        | SemanticOperation::SignatureHelp => "interactive",
        _ => "background",
    }
}

/// Canonical observability origin across adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservabilityOrigin {
    Lsp,
    Web,
    Agent,
    Runtime,
}

impl ObservabilityOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            ObservabilityOrigin::Lsp => "lsp",
            ObservabilityOrigin::Web => "web",
            ObservabilityOrigin::Agent => "agent",
            ObservabilityOrigin::Runtime => "runtime",
        }
    }
}

/// Shared execution context passed by adapters into semantic operations.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub origin: ObservabilityOrigin,
    pub operation: SemanticOperation,
    pub file_id: FileId,
    /// If set, facade should wait until runtime reaches this file version.
    pub min_file_version: Option<i32>,
    /// If set, facade should ensure the request executes against this deps snapshot.
    pub expected_deps_id: Option<DepsSnapshotId>,
    pub flow_sensitive: bool,
    pub settings: ExecutionSettings,
    pub cancellation: CancellationPolicy,
}

#[derive(Debug, Clone)]
pub struct ExecutionSettings {
    pub settings_id: SettingsId,
    pub diagnostics_detail_level: DetailLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationPolicy {
    RespectClientAbort,
    BestEffort,
    Ignore,
}

/// Canonical stage contract for semantic observability across LSP/Web/MCP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservabilityStage {
    RuntimeQueueWait,
    RuntimeWaitForFileVersion,
    RuntimeSnapshotWithDeps,
    IrQuery,
    SyntaxDiagnosticsQuery,
    SemanticDiagnosticsQuery,
    ParseResultQuery,
}

impl ObservabilityStage {
    pub fn as_str(self) -> &'static str {
        match self {
            ObservabilityStage::RuntimeQueueWait => "runtime_queue_wait",
            ObservabilityStage::RuntimeWaitForFileVersion => "runtime_wait_for_file_version",
            ObservabilityStage::RuntimeSnapshotWithDeps => "runtime_snapshot_with_deps",
            ObservabilityStage::IrQuery => "ir_query",
            ObservabilityStage::SyntaxDiagnosticsQuery => "syntax_diagnostics_query",
            ObservabilityStage::SemanticDiagnosticsQuery => "semantic_diagnostics_query",
            ObservabilityStage::ParseResultQuery => "parse_result_query",
        }
    }
}

/// Unified outcome labels for semantic stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticOutcome {
    Success,
    Empty,
    Cancelled,
    Error,
    StaleVersion,
    MissingDeps,
}

impl SemanticOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            SemanticOutcome::Success => "success",
            SemanticOutcome::Empty => "empty",
            SemanticOutcome::Cancelled => "cancelled",
            SemanticOutcome::Error => "error",
            SemanticOutcome::StaleVersion => "stale_version",
            SemanticOutcome::MissingDeps => "missing_deps",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservabilityMetricKind {
    Counter,
    HistogramMs,
}

/// Shared snapshot payload consumed by semantic operations.
pub struct SemanticSnapshot {
    pub analysis: AnalysisV2,
    pub index_snapshot: Arc<IndexSnapshot>,
    pub deps_id: DepsSnapshotId,
}

/// Prepared operation state after canonical wait/snapshot sequencing.
pub struct PreparedOperationSnapshot {
    pub snapshot: SemanticSnapshot,
    pub wait_elapsed: Option<Duration>,
    pub snapshot_elapsed: Duration,
    pub wait_budget_exhausted: bool,
    pub stale_served: bool,
    pub observed_file_version: Option<i32>,
}

#[derive(Debug, Clone, Copy)]
pub struct FileRevisionState {
    pub version: i32,
    pub updated_at: Instant,
}

#[derive(Clone)]
pub struct IntellisenseV2Facade {
    inner: Arc<Inner>,
}

struct Inner {
    tx: std::sync::mpsc::Sender<Command>,
    #[cfg(test)]
    join_handle: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

enum Command {
    ApplyChanges {
        changes: Vec<Change>,
    },
    ApplyDepsBundle {
        deps_id: DepsSnapshotId,
        deps: Arc<SemanticDeps>,
        index_snapshot: Arc<IndexSnapshot>,
        reply: oneshot::Sender<bool>,
    },
    GetSnapshot {
        reply: oneshot::Sender<AnalysisV2>,
    },
    GetSnapshotWithDeps {
        enqueued_at: Instant,
        reply: oneshot::Sender<(AnalysisV2, Arc<IndexSnapshot>, DepsSnapshotId)>,
    },
    WaitForFileVersion {
        enqueued_at: Instant,
        file_id: FileId,
        min_version: i32,
        reply: oneshot::Sender<bool>,
    },
    GetFileRevisionState {
        file_id: FileId,
        reply: oneshot::Sender<Option<FileRevisionState>>,
    },
    #[cfg(test)]
    Shutdown {
        ack: oneshot::Sender<()>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SingleflightQueryKind {
    ParseResult,
    SyntaxDiagnostics,
    Ir,
}

impl SingleflightQueryKind {
    fn as_str(self) -> &'static str {
        match self {
            SingleflightQueryKind::ParseResult => "parse_result",
            SingleflightQueryKind::SyntaxDiagnostics => "syntax_diagnostics",
            SingleflightQueryKind::Ir => "ir",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SingleflightRevisionKey {
    file_id: FileId,
    file_version: i32,
    file_signature: String,
    deps_id: Option<DepsSnapshotId>,
    settings_id: Option<SettingsId>,
    query_kind: SingleflightQueryKind,
}

struct SingleflightFlight<T> {
    state: std::sync::Mutex<SingleflightFlightState<T>>,
    cv: Condvar,
}

impl<T> SingleflightFlight<T> {
    fn new() -> Self {
        Self {
            state: std::sync::Mutex::new(SingleflightFlightState {
                in_progress: true,
                terminal_outcome: None,
            }),
            cv: Condvar::new(),
        }
    }
}

struct SingleflightFlightState<T> {
    in_progress: bool,
    terminal_outcome: Option<SingleflightTerminalOutcome<T>>,
}

#[derive(Clone)]
enum SingleflightTerminalOutcome<T> {
    Success(Option<T>),
    Error(SingleflightQueryError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingleflightQueryError {
    Cancelled,
}

type SingleflightMap<T> =
    std::sync::Mutex<HashMap<SingleflightRevisionKey, Arc<SingleflightFlight<T>>>>;

static PARSE_RESULT_FLIGHTS: OnceLock<SingleflightMap<Arc<bsl_syntax::ast::ParseResult>>> =
    OnceLock::new();
static SYNTAX_DIAGNOSTICS_FLIGHTS: OnceLock<SingleflightMap<Arc<Vec<ParseError>>>> =
    OnceLock::new();
static IR_FLIGHTS: OnceLock<SingleflightMap<Arc<SemanticProgram>>> = OnceLock::new();

impl IntellisenseV2Facade {
    pub fn new(
        initial_host: AnalysisHostV2,
        initial_index_snapshot: Arc<IndexSnapshot>,
        observability: Option<Arc<SystemCoordinator>>,
    ) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<Command>();

        let join_handle = std::thread::Builder::new()
            .name("analysis-v2-writer".to_string())
            .spawn(move || {
                let mut host = initial_host;
                let mut current_deps_id = host.deps_id();
                let mut index_snapshot = initial_index_snapshot;
                let mut applied_file_revisions: HashMap<FileId, FileRevisionState> =
                    HashMap::new();
                let mut waiters: HashMap<FileId, Vec<PendingWaiter>> = HashMap::new();

                let wake_waiters_for_file =
                    |file_id: FileId,
                     current_version: Option<i32>,
                     waiters: &mut HashMap<FileId, Vec<PendingWaiter>>,
                     observability: &Option<Arc<SystemCoordinator>>| {
                        let Some(pending) = waiters.remove(&file_id) else {
                            return;
                        };

                        let mut still_waiting = Vec::new();
                        for waiter in pending {
                            match current_version {
                                None => {
                                    let exec_elapsed = waiter.started_waiting_at.elapsed();
                                    if let Some(coordinator) = observability {
                                        coordinator.record_intellisense_v2_runtime_exec_latency(
                                            "wait_for_file_version",
                                            exec_elapsed,
                                        );
                                    }
                                    let _ = waiter.reply.send(false);
                                }
                                Some(version) if version >= waiter.min_version => {
                                    let exec_elapsed = waiter.started_waiting_at.elapsed();
                                    if let Some(coordinator) = observability {
                                        coordinator.record_intellisense_v2_runtime_exec_latency(
                                            "wait_for_file_version",
                                            exec_elapsed,
                                        );
                                    }
                                    let _ = waiter.reply.send(true);
                                }
                                Some(_) => still_waiting.push(waiter),
                            }
                        }

                        if !still_waiting.is_empty() {
                            waiters.insert(file_id, still_waiting);
                        }
                    };

                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        Command::ApplyChanges { changes } => {
                            let mut changed_files = Vec::new();

                            for change in changes {
                                match &change {
                                    Change::SetFile { file_id, version, .. } => {
                                        applied_file_revisions.insert(
                                            *file_id,
                                            FileRevisionState {
                                                version: *version,
                                                updated_at: Instant::now(),
                                            },
                                        );
                                        changed_files.push(*file_id);
                                    }
                                    Change::RemoveFile { file_id } => {
                                        applied_file_revisions.remove(file_id);
                                        changed_files.push(*file_id);
                                    }
                                    Change::SetDepsSnapshot { .. } => {
                                        warn!("analysis_v2_runtime: ignoring SetDepsSnapshot in ApplyChanges; use ApplyDepsBundle to keep index_snapshot in sync");
                                        continue;
                                    }
                                    Change::SetSettingsSnapshot { .. } => {}
                                }

                                host.apply_change(change);
                            }

                            for file_id in changed_files {
                                let version = applied_file_revisions
                                    .get(&file_id)
                                    .map(|state| state.version);
                                wake_waiters_for_file(
                                    file_id,
                                    version,
                                    &mut waiters,
                                    &observability,
                                );
                            }
                        }
                        Command::ApplyDepsBundle {
                            deps_id,
                            deps,
                            index_snapshot: new_index_snapshot,
                            reply,
                        } => {
                            current_deps_id = deps_id.clone();
                            index_snapshot = new_index_snapshot;
                            host.apply_change(Change::SetDepsSnapshot { deps_id, deps });
                            let _ = reply.send(true);
                        }
                        Command::GetSnapshot { reply } => {
                            let _ = reply.send(host.snapshot());
                        }
                        Command::GetSnapshotWithDeps { enqueued_at, reply } => {
                            let queue_wait_elapsed = enqueued_at.elapsed();
                            if let Some(coordinator) = &observability {
                                coordinator.record_intellisense_v2_runtime_queue_wait_latency(
                                    "snapshot_with_deps",
                                    queue_wait_elapsed,
                                );
                            }

                            let exec_started = Instant::now();
                            let response = (
                                host.snapshot(),
                                index_snapshot.clone(),
                                current_deps_id.clone(),
                            );
                            let exec_elapsed = exec_started.elapsed();
                            if let Some(coordinator) = &observability {
                                coordinator.record_intellisense_v2_runtime_exec_latency(
                                    "snapshot_with_deps",
                                    exec_elapsed,
                                );
                            }
                            let _ = reply.send(response);
                        }
                        Command::WaitForFileVersion {
                            enqueued_at,
                            file_id,
                            min_version,
                            reply,
                        } => {
                            let queue_wait_elapsed = enqueued_at.elapsed();
                            if let Some(coordinator) = &observability {
                                coordinator.record_intellisense_v2_runtime_queue_wait_latency(
                                    "wait_for_file_version",
                                    queue_wait_elapsed,
                                );
                            }

                            match applied_file_revisions.get(&file_id).map(|state| state.version) {
                                Some(version) if version >= min_version => {
                                    let exec_started = Instant::now();
                                    let _ = reply.send(true);
                                    let exec_elapsed = exec_started.elapsed();
                                    if let Some(coordinator) = &observability {
                                        coordinator.record_intellisense_v2_runtime_exec_latency(
                                            "wait_for_file_version",
                                            exec_elapsed,
                                        );
                                    }
                                }
                                _ => {
                                    waiters.entry(file_id).or_default().push(PendingWaiter {
                                        min_version,
                                        reply,
                                        started_waiting_at: Instant::now(),
                                    });
                                }
                            }
                        }
                        Command::GetFileRevisionState { file_id, reply } => {
                            let _ = reply.send(applied_file_revisions.get(&file_id).copied());
                        }
                        #[cfg(test)]
                        Command::Shutdown { ack } => {
                            for (_file_id, pending) in waiters.drain() {
                                for waiter in pending {
                                    let exec_elapsed = waiter.started_waiting_at.elapsed();
                                    if let Some(coordinator) = &observability {
                                        coordinator.record_intellisense_v2_runtime_exec_latency(
                                            "wait_for_file_version",
                                            exec_elapsed,
                                        );
                                    }
                                    let _ = waiter.reply.send(false);
                                }
                            }
                            let _ = ack.send(());
                            break;
                        }
                    }
                }
            })
            .expect("failed to spawn analysis-v2 writer thread");

        #[cfg(not(test))]
        let _ = join_handle;

        Self {
            inner: Arc::new(Inner {
                tx,
                #[cfg(test)]
                join_handle: std::sync::Mutex::new(Some(join_handle)),
            }),
        }
    }

    pub fn apply_changes(&self, changes: Vec<Change>) {
        if changes.is_empty() {
            return;
        }
        if self
            .inner
            .tx
            .send(Command::ApplyChanges { changes })
            .is_err()
        {
            warn!("analysis_v2_runtime: failed to send ApplyChanges (writer thread is gone)");
        }
    }

    pub async fn apply_deps_bundle(
        &self,
        deps_id: DepsSnapshotId,
        deps: Arc<SemanticDeps>,
        index_snapshot: Arc<IndexSnapshot>,
    ) -> bool {
        let (reply, rx) = oneshot::channel::<bool>();
        if self
            .inner
            .tx
            .send(Command::ApplyDepsBundle {
                deps_id,
                deps,
                index_snapshot,
                reply,
            })
            .is_err()
        {
            warn!("analysis_v2_runtime: failed to send ApplyDepsBundle (writer thread is gone)");
            return false;
        }
        rx.await.unwrap_or(false)
    }

    pub async fn snapshot(&self) -> AnalysisV2 {
        let (reply, rx) = oneshot::channel::<AnalysisV2>();
        if self.inner.tx.send(Command::GetSnapshot { reply }).is_err() {
            warn!("analysis_v2_runtime: failed to send GetSnapshot (writer thread is gone)");
            return AnalysisHostV2::default().snapshot();
        }
        match rx.await {
            Ok(snapshot) => snapshot,
            Err(_) => {
                warn!("analysis_v2_runtime: GetSnapshot response cancelled");
                AnalysisHostV2::default().snapshot()
            }
        }
    }

    pub async fn snapshot_with_deps(&self) -> (AnalysisV2, Arc<IndexSnapshot>, DepsSnapshotId) {
        let (reply, rx) = oneshot::channel::<(AnalysisV2, Arc<IndexSnapshot>, DepsSnapshotId)>();
        if self
            .inner
            .tx
            .send(Command::GetSnapshotWithDeps {
                enqueued_at: Instant::now(),
                reply,
            })
            .is_err()
        {
            warn!(
                "analysis_v2_runtime: failed to send GetSnapshotWithDeps (writer thread is gone)"
            );
            return (
                AnalysisHostV2::default().snapshot(),
                Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(""))),
                DepsSnapshotId::from_hash(""),
            );
        }

        match rx.await {
            Ok(tuple) => tuple,
            Err(_) => {
                warn!("analysis_v2_runtime: GetSnapshotWithDeps response cancelled");
                (
                    AnalysisHostV2::default().snapshot(),
                    Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(""))),
                    DepsSnapshotId::from_hash(""),
                )
            }
        }
    }

    /// Returns a consistent analysis/index/deps snapshot for a semantic operation.
    /// Operation kind is part of the canonical facade contract and is reserved for
    /// shared policy/observability branching in subsequent migration steps.
    pub async fn snapshot_for_operation(&self, _operation: SemanticOperation) -> SemanticSnapshot {
        let (analysis, index_snapshot, deps_id) = self.snapshot_with_deps().await;
        SemanticSnapshot {
            analysis,
            index_snapshot,
            deps_id,
        }
    }

    /// Canonical stateful operation preparation for adapters:
    /// wait-for-version -> snapshot-with-deps -> deps guard check.
    pub async fn prepare_stateful_operation(
        &self,
        context: &ExecutionContext,
        observability: Option<&SystemCoordinator>,
    ) -> Result<PreparedOperationSnapshot, SemanticOutcome> {
        let interactive_knobs = interactive_freshness_knobs(context.operation, observability);
        let mut wait_budget_exhausted = false;
        let mut stale_served = false;

        let wait_elapsed = if let Some(min_file_version) = context.min_file_version {
            let started = Instant::now();
            let wait_ok = if let Some(knobs) = interactive_knobs {
                match tokio::time::timeout(
                    knobs.wait_budget,
                    self.wait_for_file_version(context.file_id, min_file_version),
                )
                .await
                {
                    Ok(wait_ok) => wait_ok,
                    Err(_) => {
                        wait_budget_exhausted = true;
                        if let Some(coordinator) = observability {
                            coordinator.record_intellisense_v2_interactive_wait_budget_exhausted();
                        }
                        true
                    }
                }
            } else {
                self.wait_for_file_version(context.file_id, min_file_version)
                    .await
            };
            let elapsed = started.elapsed();
            if let Some(coordinator) = observability {
                coordinator.record_intellisense_v2_wait_for_file_version_with_origin(
                    context.origin.as_str(),
                    context.operation.as_str(),
                    elapsed,
                );
            }
            if !wait_ok {
                return Err(SemanticOutcome::StaleVersion);
            }
            Some(elapsed)
        } else {
            None
        };

        let snapshot_started = Instant::now();
        let (analysis, index_snapshot, deps_id) = self.snapshot_with_deps().await;
        let snapshot_elapsed = snapshot_started.elapsed();
        if let Some(coordinator) = observability {
            coordinator.record_intellisense_v2_snapshot_latency_with_origin(
                context.origin.as_str(),
                context.operation.as_str(),
                snapshot_elapsed,
            );
            coordinator.record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
                context.origin.as_str(),
                runtime_work_class_for_operation(context.operation),
                wait_elapsed.unwrap_or(Duration::ZERO),
            );
            coordinator.record_intellisense_v2_runtime_exec_class_latency_with_origin(
                context.origin.as_str(),
                runtime_work_class_for_operation(context.operation),
                snapshot_elapsed,
            );
        }

        if let Some(expected_deps_id) = context.expected_deps_id.as_ref() {
            if expected_deps_id != &deps_id {
                return Err(SemanticOutcome::MissingDeps);
            }
        }

        let observed_file_version = analysis.file_version(context.file_id).ok().flatten();
        let observed_settings_id = analysis.settings_id().ok();
        if let (Some(min_file_version), Some(knobs)) = (context.min_file_version, interactive_knobs)
        {
            if wait_budget_exhausted {
                let completion_fallback_metric_enabled =
                    matches!(context.operation, SemanticOperation::Completion);
                let record_completion_fallback_unavailable = || {
                    if completion_fallback_metric_enabled {
                        if let Some(coordinator) = observability {
                            coordinator.record_intellisense_v2_completion_fallback_unavailable();
                        }
                    }
                };

                let Some(expected_deps_id) = context.expected_deps_id.as_ref() else {
                    record_completion_fallback_unavailable();
                    return Err(SemanticOutcome::StaleVersion);
                };
                if expected_deps_id != &deps_id {
                    record_completion_fallback_unavailable();
                    return Err(SemanticOutcome::StaleVersion);
                }
                if observed_settings_id.as_ref() != Some(&context.settings.settings_id) {
                    record_completion_fallback_unavailable();
                    return Err(SemanticOutcome::StaleVersion);
                }
                if let Some(observed_version) = observed_file_version {
                    if observed_version < min_file_version {
                        let lag_versions = min_file_version.saturating_sub(observed_version);
                        if let Some(coordinator) = observability {
                            coordinator.record_intellisense_v2_revision_lag(lag_versions);
                        }

                        if let Err(outcome) = self
                            .validate_stale_fallback(
                                context.file_id,
                                min_file_version,
                                observed_version,
                                knobs,
                            )
                            .await
                        {
                            record_completion_fallback_unavailable();
                            return Err(outcome);
                        }
                        stale_served = true;
                        if let Some(coordinator) = observability {
                            coordinator.record_intellisense_v2_interactive_stale_served();
                            if completion_fallback_metric_enabled {
                                coordinator.record_intellisense_v2_completion_stale_fallback();
                            }
                        }
                    }
                } else {
                    record_completion_fallback_unavailable();
                    return Err(SemanticOutcome::StaleVersion);
                }
            } else if observed_file_version.is_some_and(|version| version < min_file_version) {
                return Err(SemanticOutcome::StaleVersion);
            }
        }

        Ok(PreparedOperationSnapshot {
            snapshot: SemanticSnapshot {
                analysis,
                index_snapshot,
                deps_id,
            },
            wait_elapsed,
            snapshot_elapsed,
            wait_budget_exhausted,
            stale_served,
            observed_file_version,
        })
    }

    async fn validate_stale_fallback(
        &self,
        file_id: FileId,
        requested_version: i32,
        observed_version: i32,
        knobs: InteractiveFreshnessKnobs,
    ) -> Result<(), SemanticOutcome> {
        let version_gap = requested_version.saturating_sub(observed_version);
        if version_gap > knobs.max_stale_version_gap {
            return Err(SemanticOutcome::StaleVersion);
        }

        let Some(revision_state) = self.file_revision_state(file_id).await else {
            return Err(SemanticOutcome::StaleVersion);
        };
        if revision_state.version != observed_version {
            return Err(SemanticOutcome::StaleVersion);
        }
        if revision_state.updated_at.elapsed() > knobs.max_stale_age {
            return Err(SemanticOutcome::StaleVersion);
        }

        Ok(())
    }

    /// Canonical ephemeral operation preparation for one-shot adapters:
    /// snapshot build -> deps guard check.
    pub fn prepare_ephemeral_operation(
        context: &ExecutionContext,
        deps_id: DepsSnapshotId,
        deps: Arc<SemanticDeps>,
        index_snapshot: Arc<IndexSnapshot>,
        file_text: Arc<str>,
        file_version: i32,
        file_path: Arc<str>,
        observability: Option<&SystemCoordinator>,
    ) -> Result<PreparedOperationSnapshot, SemanticOutcome> {
        let snapshot_started = Instant::now();
        let snapshot = Self::ephemeral_snapshot(
            deps_id,
            deps,
            index_snapshot,
            context.settings.clone(),
            context.file_id,
            file_text,
            file_version,
            file_path,
        );
        let snapshot_elapsed = snapshot_started.elapsed();
        if let Some(coordinator) = observability {
            coordinator.record_intellisense_v2_snapshot_latency_with_origin(
                context.origin.as_str(),
                context.operation.as_str(),
                snapshot_elapsed,
            );
        }

        if let Some(expected_deps_id) = context.expected_deps_id.as_ref() {
            if expected_deps_id != &snapshot.deps_id {
                return Err(SemanticOutcome::MissingDeps);
            }
        }

        Ok(PreparedOperationSnapshot {
            snapshot,
            wait_elapsed: None,
            snapshot_elapsed,
            wait_budget_exhausted: false,
            stale_served: false,
            observed_file_version: Some(file_version),
        })
    }

    /// Run an optional semantic query with shared stage-level observability hooks.
    pub fn run_optional_query<T, E, F>(
        context: &ExecutionContext,
        stage: ObservabilityStage,
        analysis: &AnalysisV2,
        observability: Option<&SystemCoordinator>,
        query: F,
    ) -> Result<Option<T>, E>
    where
        F: FnOnce(&AnalysisV2) -> Result<Option<T>, E>,
    {
        let started = Instant::now();
        let raw_result = query(analysis);
        let elapsed = started.elapsed();
        let query_cancelled = raw_result.is_err();
        let report_cancelled =
            query_cancelled && !matches!(context.cancellation, CancellationPolicy::Ignore);
        let result = match raw_result {
            Ok(value) => Ok(value),
            Err(err) => match context.cancellation {
                CancellationPolicy::RespectClientAbort => Err(err),
                CancellationPolicy::BestEffort | CancellationPolicy::Ignore => Ok(None),
            },
        };

        if let Some(coordinator) = observability {
            match stage {
                ObservabilityStage::IrQuery => {
                    coordinator.record_intellisense_v2_ir_query_latency_with_origin(
                        context.origin.as_str(),
                        context.operation.as_str(),
                        elapsed,
                    );
                    if report_cancelled {
                        coordinator.record_intellisense_v2_ir_query_cancelled_with_origin(
                            context.origin.as_str(),
                            context.operation.as_str(),
                        );
                    }
                }
                ObservabilityStage::SyntaxDiagnosticsQuery => {
                    coordinator
                        .record_intellisense_v2_syntax_diagnostics_query_latency_with_origin(
                            context.origin.as_str(),
                            elapsed,
                        );
                    if report_cancelled {
                        coordinator.record_intellisense_v2_query_cancelled_with_origin(
                            context.origin.as_str(),
                            "syntax",
                        );
                    }
                }
                ObservabilityStage::SemanticDiagnosticsQuery => {
                    coordinator
                        .record_intellisense_v2_semantic_diagnostics_query_latency_with_origin(
                            context.origin.as_str(),
                            elapsed,
                        );
                    if report_cancelled {
                        coordinator.record_intellisense_v2_query_cancelled_with_origin(
                            context.origin.as_str(),
                            "semantic",
                        );
                    }
                }
                ObservabilityStage::ParseResultQuery => {
                    coordinator.record_intellisense_v2_parse_result_query_latency_with_origin(
                        context.origin.as_str(),
                        elapsed,
                    );
                    if report_cancelled {
                        coordinator.record_intellisense_v2_query_cancelled_with_origin(
                            context.origin.as_str(),
                            "other",
                        );
                    }
                }
                ObservabilityStage::RuntimeQueueWait
                | ObservabilityStage::RuntimeWaitForFileVersion
                | ObservabilityStage::RuntimeSnapshotWithDeps => {}
            }
        }

        result
    }

    /// Run parse_result according to centralized lazy policy and shared stage hooks.
    pub fn run_parse_result_query<T, E, F>(
        context: &ExecutionContext,
        analysis: &AnalysisV2,
        ir_available: bool,
        observability: Option<&SystemCoordinator>,
        query: F,
    ) -> Result<Option<T>, E>
    where
        F: FnOnce(&AnalysisV2) -> Result<Option<T>, E>,
    {
        if !should_query_parse_result(context.operation, ir_available) {
            return Ok(None);
        }
        Self::run_optional_query(
            context,
            ObservabilityStage::ParseResultQuery,
            analysis,
            observability,
            query,
        )
    }

    pub fn run_ir_query_singleflight(
        context: &ExecutionContext,
        analysis: &AnalysisV2,
        observability: Option<&SystemCoordinator>,
        file_id: FileId,
    ) -> Result<Option<Arc<SemanticProgram>>, SingleflightQueryError> {
        let key = Self::singleflight_revision_key(analysis, file_id, SingleflightQueryKind::Ir);
        Self::run_optional_query(
            context,
            ObservabilityStage::IrQuery,
            analysis,
            observability,
            |_analysis| {
                if let Some(key) = key {
                    Self::run_singleflight_query(
                        &IR_FLIGHTS,
                        key,
                        context.origin,
                        SingleflightQueryKind::Ir,
                        observability,
                        || {
                            analysis
                                .ir(file_id)
                                .map_err(|_| SingleflightQueryError::Cancelled)
                        },
                    )
                } else {
                    if let Some(coordinator) = observability {
                        coordinator
                            .record_intellisense_v2_singleflight_key_unavailable_with_origin(
                                context.origin.as_str(),
                                SingleflightQueryKind::Ir.as_str(),
                            );
                    }
                    analysis
                        .ir(file_id)
                        .map_err(|_| SingleflightQueryError::Cancelled)
                }
            },
        )
    }

    pub fn run_parse_result_query_singleflight(
        context: &ExecutionContext,
        analysis: &AnalysisV2,
        ir_available: bool,
        observability: Option<&SystemCoordinator>,
        file_id: FileId,
    ) -> Result<Option<Arc<bsl_syntax::ast::ParseResult>>, SingleflightQueryError> {
        if !should_query_parse_result(context.operation, ir_available) {
            return Ok(None);
        }
        let key =
            Self::singleflight_revision_key(analysis, file_id, SingleflightQueryKind::ParseResult);
        Self::run_optional_query(
            context,
            ObservabilityStage::ParseResultQuery,
            analysis,
            observability,
            |_analysis| {
                if let Some(key) = key {
                    Self::run_singleflight_query(
                        &PARSE_RESULT_FLIGHTS,
                        key,
                        context.origin,
                        SingleflightQueryKind::ParseResult,
                        observability,
                        || {
                            analysis
                                .parse_result(file_id)
                                .map_err(|_| SingleflightQueryError::Cancelled)
                        },
                    )
                } else {
                    if let Some(coordinator) = observability {
                        coordinator
                            .record_intellisense_v2_singleflight_key_unavailable_with_origin(
                                context.origin.as_str(),
                                SingleflightQueryKind::ParseResult.as_str(),
                            );
                    }
                    analysis
                        .parse_result(file_id)
                        .map_err(|_| SingleflightQueryError::Cancelled)
                }
            },
        )
    }

    pub fn run_syntax_diagnostics_query_singleflight(
        context: &ExecutionContext,
        analysis: &AnalysisV2,
        observability: Option<&SystemCoordinator>,
        file_id: FileId,
    ) -> Result<Option<Arc<Vec<ParseError>>>, SingleflightQueryError> {
        let key = Self::singleflight_revision_key(
            analysis,
            file_id,
            SingleflightQueryKind::SyntaxDiagnostics,
        );
        Self::run_optional_query(
            context,
            ObservabilityStage::SyntaxDiagnosticsQuery,
            analysis,
            observability,
            |_analysis| {
                if let Some(key) = key {
                    Self::run_singleflight_query(
                        &SYNTAX_DIAGNOSTICS_FLIGHTS,
                        key,
                        context.origin,
                        SingleflightQueryKind::SyntaxDiagnostics,
                        observability,
                        || {
                            analysis
                                .syntax_diagnostics(file_id)
                                .map_err(|_| SingleflightQueryError::Cancelled)
                        },
                    )
                } else {
                    if let Some(coordinator) = observability {
                        coordinator
                            .record_intellisense_v2_singleflight_key_unavailable_with_origin(
                                context.origin.as_str(),
                                SingleflightQueryKind::SyntaxDiagnostics.as_str(),
                            );
                    }
                    analysis
                        .syntax_diagnostics(file_id)
                        .map_err(|_| SingleflightQueryError::Cancelled)
                }
            },
        )
    }

    fn singleflight_revision_key(
        analysis: &AnalysisV2,
        file_id: FileId,
        query_kind: SingleflightQueryKind,
    ) -> Option<SingleflightRevisionKey> {
        let file_version = analysis.file_version(file_id).ok().flatten()?;
        let file_signature = Self::singleflight_file_signature(analysis, file_id)?;
        let (deps_id, settings_id) = if Self::singleflight_requires_snapshot_identity(query_kind) {
            (
                Some(analysis.deps_id().ok()?),
                Some(analysis.settings_id().ok()?),
            )
        } else {
            (None, None)
        };
        Some(SingleflightRevisionKey {
            file_id,
            file_version,
            file_signature,
            deps_id,
            settings_id,
            query_kind,
        })
    }

    fn singleflight_requires_snapshot_identity(query_kind: SingleflightQueryKind) -> bool {
        matches!(query_kind, SingleflightQueryKind::Ir)
    }

    fn singleflight_file_signature(analysis: &AnalysisV2, file_id: FileId) -> Option<String> {
        if let Some(path) = analysis.file_path(file_id).ok().flatten() {
            return Some(format!("path:{path}"));
        }
        let text = analysis.file_text(file_id).ok().flatten()?;
        Some(format!("text:{}", blake3::hash(text.as_bytes()).to_hex()))
    }

    fn run_singleflight_query<T>(
        flights: &OnceLock<SingleflightMap<T>>,
        key: SingleflightRevisionKey,
        origin: ObservabilityOrigin,
        query_kind: SingleflightQueryKind,
        observability: Option<&SystemCoordinator>,
        query: impl FnOnce() -> Result<Option<T>, SingleflightQueryError>,
    ) -> Result<Option<T>, SingleflightQueryError>
    where
        T: Clone,
    {
        let flights = flights.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
        let (flight, is_leader) = {
            let mut guard = flights
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(existing) = guard.get(&key) {
                (existing.clone(), false)
            } else {
                let created = Arc::new(SingleflightFlight::new());
                guard.insert(key.clone(), created.clone());
                (created, true)
            }
        };

        if is_leader {
            if let Some(coordinator) = observability {
                coordinator.record_intellisense_v2_singleflight_leader_with_origin(
                    origin.as_str(),
                    query_kind.as_str(),
                );
            }
            let result = match catch_unwind(AssertUnwindSafe(query)) {
                Ok(result) => result,
                Err(_panic_payload) => Err(SingleflightQueryError::Cancelled),
            };
            {
                let mut state = flight
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.terminal_outcome = Some(match &result {
                    Ok(value) => SingleflightTerminalOutcome::Success(value.clone()),
                    Err(err) => SingleflightTerminalOutcome::Error(*err),
                });
                state.in_progress = false;
            }
            flight.cv.notify_all();
            let mut guard = flights
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.remove(&key);
            result
        } else {
            if let Some(coordinator) = observability {
                coordinator.record_intellisense_v2_singleflight_shared_with_origin(
                    origin.as_str(),
                    query_kind.as_str(),
                );
            }
            let wait_started = Instant::now();
            let mut state = flight
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            while state.in_progress {
                state = flight
                    .cv
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            if let Some(coordinator) = observability {
                coordinator.record_intellisense_v2_singleflight_wait_latency_with_origin(
                    origin.as_str(),
                    query_kind.as_str(),
                    wait_started.elapsed(),
                );
            }
            match state.terminal_outcome.clone() {
                Some(SingleflightTerminalOutcome::Success(shared)) => Ok(shared),
                Some(SingleflightTerminalOutcome::Error(err)) => Err(err),
                None => Err(SingleflightQueryError::Cancelled),
            }
        }
    }

    /// One-shot helper for ephemeral adapters (e.g. web handlers).
    /// Builds a semantic snapshot without creating a long-lived writer-thread runtime.
    pub fn ephemeral_snapshot(
        deps_id: DepsSnapshotId,
        deps: Arc<SemanticDeps>,
        index_snapshot: Arc<IndexSnapshot>,
        settings: ExecutionSettings,
        file_id: FileId,
        file_text: Arc<str>,
        file_version: i32,
        file_path: Arc<str>,
    ) -> SemanticSnapshot {
        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id.clone(),
            deps,
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: settings.settings_id,
            diagnostics_detail_level: settings.diagnostics_detail_level,
        });
        host.apply_change(Change::SetFile {
            file_id,
            text: file_text,
            version: file_version,
            path: file_path,
        });

        SemanticSnapshot {
            analysis: host.snapshot(),
            index_snapshot,
            deps_id,
        }
    }

    pub async fn wait_for_file_version(&self, file_id: FileId, min_version: i32) -> bool {
        let (reply, rx) = oneshot::channel::<bool>();
        if self
            .inner
            .tx
            .send(Command::WaitForFileVersion {
                enqueued_at: Instant::now(),
                file_id,
                min_version,
                reply,
            })
            .is_err()
        {
            warn!("analysis_v2_runtime: failed to send WaitForFileVersion (writer thread is gone)");
            return false;
        }
        rx.await.unwrap_or(false)
    }

    pub async fn file_revision_state(&self, file_id: FileId) -> Option<FileRevisionState> {
        let (reply, rx) = oneshot::channel::<Option<FileRevisionState>>();
        if self
            .inner
            .tx
            .send(Command::GetFileRevisionState { file_id, reply })
            .is_err()
        {
            warn!(
                "analysis_v2_runtime: failed to send GetFileRevisionState (writer thread is gone)"
            );
            return None;
        }
        match rx.await {
            Ok(state) => state,
            Err(_) => {
                warn!("analysis_v2_runtime: GetFileRevisionState response cancelled");
                None
            }
        }
    }

    #[cfg(test)]
    pub(crate) async fn shutdown_for_test(&self) {
        let (ack, rx) = oneshot::channel::<()>();
        let _ = self.inner.tx.send(Command::Shutdown { ack });
        let _ = rx.await;

        let join_handle = self.inner.join_handle.lock().unwrap().take();
        if let Some(handle) = join_handle {
            let _ = handle.join();
        }
    }
}

struct PendingWaiter {
    min_version: i32,
    reply: oneshot::Sender<bool>,
    started_waiting_at: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::resolver::TypeResolver;

    #[tokio::test]
    async fn p7_apply_changes_and_wait_for_version_works() {
        let runtime = IntellisenseV2Facade::new(
            AnalysisHostV2::default(),
            Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
            None,
        );
        let file_id = FileId(1);

        runtime.apply_changes(vec![Change::SetFile {
            file_id,
            text: Arc::from("abc"),
            version: 7,
            path: Arc::from("test.bsl"),
        }]);

        let ok = timeout(
            Duration::from_secs(1),
            runtime.wait_for_file_version(file_id, 7),
        )
        .await
        .expect("wait_for_file_version timeout");
        assert!(ok, "expected wait_for_file_version to succeed");

        let analysis = runtime.snapshot().await;
        assert_eq!(analysis.file_version(file_id).unwrap(), Some(7));

        runtime.shutdown_for_test().await;
    }

    #[tokio::test]
    async fn p7_waiters_are_released_on_shutdown() {
        let runtime = IntellisenseV2Facade::new(
            AnalysisHostV2::default(),
            Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
            None,
        );
        let file_id = FileId(1);

        let wait_task = tokio::spawn({
            let runtime = runtime.clone();
            async move { runtime.wait_for_file_version(file_id, 42).await }
        });

        runtime.shutdown_for_test().await;

        let ok = timeout(Duration::from_secs(1), wait_task)
            .await
            .expect("wait task timeout")
            .expect("wait task join");
        assert!(!ok, "expected waiter to return false on shutdown");
    }

    fn make_deps() -> Arc<SemanticDeps> {
        let repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
        let signature_index = repository.get_signature_index_clone();
        let resolver = Some(Arc::new(TypeResolver::new(repository.clone())));
        let platform_signatures_loaded = repository.platform_docs_loaded();
        Arc::new(SemanticDeps {
            repository,
            signature_index,
            resolver,
            platform_signatures_loaded,
        })
    }

    fn make_index_snapshot(raw_id: &str) -> Arc<IndexSnapshot> {
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(raw_id)))
    }

    #[tokio::test]
    async fn p8_snapshot_with_deps_is_atomic() {
        let mut host = AnalysisHostV2::default();

        let deps_old = make_deps();
        let deps_id_old = DepsSnapshotId::from_hash("deps_old");
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id_old.clone(),
            deps: deps_old,
        });

        let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index_old"), None);

        {
            let (analysis, index_snapshot, deps_id) = runtime.snapshot_with_deps().await;
            assert_eq!(deps_id.as_str(), "deps_old");
            assert_eq!(index_snapshot.id.as_str(), "index_old");
            assert_eq!(analysis.deps_id().unwrap().as_str(), "deps_old");
        }

        let deps_new = make_deps();
        let deps_id_new = DepsSnapshotId::from_hash("deps_new");
        let index_new = make_index_snapshot("index_new");

        let apply_task = tokio::spawn({
            let runtime = runtime.clone();
            let deps_new = deps_new.clone();
            let deps_id_new = deps_id_new.clone();
            let index_new = index_new.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                let ok = runtime
                    .apply_deps_bundle(deps_id_new, deps_new, index_new)
                    .await;
                assert!(ok, "apply_deps_bundle should succeed");
            }
        });

        let watch_task = tokio::spawn({
            let runtime = runtime.clone();
            async move {
                for _ in 0..200 {
                    let (_analysis, index_snapshot, deps_id) = runtime.snapshot_with_deps().await;
                    match deps_id.as_str() {
                        "deps_old" => assert_eq!(index_snapshot.id.as_str(), "index_old"),
                        "deps_new" => assert_eq!(index_snapshot.id.as_str(), "index_new"),
                        other => panic!("unexpected deps_id: {}", other),
                    }
                }
            }
        });

        apply_task.await.expect("apply task join");
        watch_task.await.expect("watch task join");

        let (analysis, index_snapshot, deps_id) = runtime.snapshot_with_deps().await;
        assert_eq!(deps_id.as_str(), "deps_new");
        assert_eq!(index_snapshot.id.as_str(), "index_new");
        assert_eq!(analysis.deps_id().unwrap().as_str(), "deps_new");

        runtime.shutdown_for_test().await;
    }

    #[tokio::test]
    async fn p8_apply_changes_ignores_set_deps_snapshot() {
        let mut host = AnalysisHostV2::default();

        let deps_old = make_deps();
        let deps_id_old = DepsSnapshotId::from_hash("deps_old");
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id_old.clone(),
            deps: deps_old,
        });

        let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index_old"), None);

        let deps_new = make_deps();
        let deps_id_new = DepsSnapshotId::from_hash("deps_new");
        runtime.apply_changes(vec![Change::SetDepsSnapshot {
            deps_id: deps_id_new,
            deps: deps_new,
        }]);

        let (analysis, _index_snapshot, deps_id) = runtime.snapshot_with_deps().await;
        assert_eq!(deps_id.as_str(), "deps_old");
        assert_eq!(analysis.deps_id().unwrap().as_str(), "deps_old");

        runtime.shutdown_for_test().await;
    }

    #[test]
    fn ephemeral_snapshot_sets_contract_inputs() {
        let deps = make_deps();
        let deps_id = DepsSnapshotId::from_hash("deps_ephemeral");
        let settings = ExecutionSettings {
            settings_id: SettingsId::from_hash("settings_ephemeral"),
            diagnostics_detail_level: DetailLevel::Full,
        };
        let snapshot = IntellisenseV2Facade::ephemeral_snapshot(
            deps_id.clone(),
            deps,
            make_index_snapshot("index_ephemeral"),
            settings.clone(),
            FileId(7),
            Arc::from("Перем х;"),
            42,
            Arc::from("<ephemeral>"),
        );

        assert_eq!(
            snapshot.analysis.file_version(FileId(7)).unwrap(),
            Some(42),
            "ephemeral snapshot should carry file version"
        );
        assert_eq!(
            snapshot.analysis.deps_id().unwrap().as_str(),
            deps_id.as_str(),
            "ephemeral snapshot should carry deps id"
        );
        assert_eq!(
            snapshot.analysis.settings_id().unwrap().as_str(),
            settings.settings_id.as_str(),
            "ephemeral snapshot should carry settings id"
        );
        assert_eq!(snapshot.index_snapshot.id.as_str(), "index_ephemeral");
    }

    #[test]
    fn semantic_operation_contract_values_are_stable() {
        assert_eq!(SemanticOperation::Completion.as_str(), "completion");
        assert_eq!(SemanticOperation::Hover.as_str(), "hover");
        assert_eq!(SemanticOperation::SignatureHelp.as_str(), "signature_help");
        assert_eq!(SemanticOperation::Definition.as_str(), "definition");
        assert_eq!(
            SemanticOperation::DocumentSymbol.as_str(),
            "document_symbol"
        );
        assert_eq!(SemanticOperation::Rename.as_str(), "rename");
        assert_eq!(SemanticOperation::Diagnostics.as_str(), "diagnostics");
        assert_eq!(SemanticOperation::Members.as_str(), "members");
        assert_eq!(
            SemanticOperation::TypeAtPosition.as_str(),
            "type_at_position"
        );
        assert_eq!(SemanticOperation::SymbolSearch.as_str(), "symbol_search");
        assert_eq!(SemanticOperation::References.as_str(), "references");
    }

    #[test]
    fn observability_contract_values_are_stable() {
        assert_eq!(
            ObservabilityStage::RuntimeQueueWait.as_str(),
            "runtime_queue_wait"
        );
        assert_eq!(
            ObservabilityStage::RuntimeWaitForFileVersion.as_str(),
            "runtime_wait_for_file_version"
        );
        assert_eq!(
            ObservabilityStage::RuntimeSnapshotWithDeps.as_str(),
            "runtime_snapshot_with_deps"
        );
        assert_eq!(ObservabilityStage::IrQuery.as_str(), "ir_query");
        assert_eq!(
            ObservabilityStage::SyntaxDiagnosticsQuery.as_str(),
            "syntax_diagnostics_query"
        );
        assert_eq!(
            ObservabilityStage::SemanticDiagnosticsQuery.as_str(),
            "semantic_diagnostics_query"
        );
        assert_eq!(
            ObservabilityStage::ParseResultQuery.as_str(),
            "parse_result_query"
        );
        assert_eq!(SemanticOutcome::Success.as_str(), "success");
        assert_eq!(SemanticOutcome::Empty.as_str(), "empty");
        assert_eq!(SemanticOutcome::Cancelled.as_str(), "cancelled");
        assert_eq!(SemanticOutcome::Error.as_str(), "error");
        assert_eq!(SemanticOutcome::StaleVersion.as_str(), "stale_version");
        assert_eq!(SemanticOutcome::MissingDeps.as_str(), "missing_deps");
    }

    #[tokio::test]
    async fn stateful_prepare_operation_returns_missing_deps_on_mismatch() {
        let mut host = AnalysisHostV2::default();
        let deps_old = make_deps();
        let deps_id_old = DepsSnapshotId::from_hash("deps_old");
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id_old,
            deps: deps_old,
        });
        let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index"), None);

        let context = ExecutionContext {
            origin: ObservabilityOrigin::Lsp,
            operation: SemanticOperation::Hover,
            file_id: FileId(1),
            min_file_version: None,
            expected_deps_id: Some(DepsSnapshotId::from_hash("deps_expected")),
            flow_sensitive: false,
            settings: ExecutionSettings {
                settings_id: SettingsId::from_hash("settings"),
                diagnostics_detail_level: DetailLevel::Full,
            },
            cancellation: CancellationPolicy::BestEffort,
        };

        let result = runtime.prepare_stateful_operation(&context, None).await;
        assert!(matches!(result, Err(SemanticOutcome::MissingDeps)));

        runtime.shutdown_for_test().await;
    }

    #[tokio::test]
    async fn interactive_prepare_timeout_serves_stale_when_gap_within_default() {
        let coordinator = SystemCoordinator::new();
        let file_id = FileId(10);
        let deps_id = DepsSnapshotId::from_hash("deps_stale_ok");
        let settings_id = SettingsId::from_hash("settings");

        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id.clone(),
            deps: make_deps(),
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: settings_id.clone(),
            diagnostics_detail_level: DetailLevel::Full,
        });
        let runtime = IntellisenseV2Facade::new(
            host,
            Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
            None,
        );
        runtime.apply_changes(vec![Change::SetFile {
            file_id,
            text: Arc::from("x = 1;"),
            version: 4,
            path: Arc::from("stale_ok.bsl"),
        }]);
        let _ = runtime.snapshot().await;

        let context = ExecutionContext {
            origin: ObservabilityOrigin::Lsp,
            operation: SemanticOperation::Completion,
            file_id,
            min_file_version: Some(5),
            expected_deps_id: Some(deps_id),
            flow_sensitive: false,
            settings: ExecutionSettings {
                settings_id: settings_id.clone(),
                diagnostics_detail_level: DetailLevel::Full,
            },
            cancellation: CancellationPolicy::BestEffort,
        };

        let prepared = runtime
            .prepare_stateful_operation(&context, Some(&coordinator))
            .await
            .expect("interactive fallback should serve stale snapshot");
        assert!(
            prepared.wait_budget_exhausted,
            "expected bounded wait timeout for interactive path"
        );
        assert!(
            prepared.stale_served,
            "expected stale fallback to be served"
        );
        assert_eq!(prepared.observed_file_version, Some(4));
        assert!(
            prepared
                .wait_elapsed
                .is_some_and(|elapsed| elapsed >= Duration::from_millis(90)),
            "wait elapsed should reflect bounded wait timeout"
        );

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");
        assert!(
            counters.contains_key("intellisense_v2_interactive_wait_budget_exhausted_total"),
            "wait budget exhausted metric should be recorded"
        );
        assert!(
            counters.contains_key("intellisense_v2_interactive_stale_served_total"),
            "stale served metric should be recorded"
        );
        assert!(
            counters.contains_key("intellisense_v2_runtime_queue_wait_interactive_total"),
            "interactive queue-class counter should be recorded"
        );
        assert!(
            counters.contains_key("intellisense_v2_runtime_exec_interactive_total"),
            "interactive exec-class counter should be recorded"
        );
        assert!(
            counters.contains_key("intellisense_v2_completion_stale_fallback_total"),
            "completion stale-fallback counter should be recorded"
        );
        assert!(
            counters.contains_key("intellisense_v2_revision_lag_sample_total"),
            "revision lag counter should be recorded"
        );
        assert!(
            histograms.contains_key("intellisense_v2_runtime_queue_wait_interactive_ms"),
            "interactive queue-class histogram should be recorded"
        );
        assert!(
            histograms.contains_key("intellisense_v2_runtime_exec_interactive_ms"),
            "interactive exec-class histogram should be recorded"
        );
        assert!(
            histograms.contains_key("intellisense_v2_revision_lag_versions"),
            "revision lag histogram should be recorded"
        );

        runtime.shutdown_for_test().await;
    }

    #[tokio::test]
    async fn interactive_prepare_timeout_rejects_stale_when_gap_exceeds_default() {
        let coordinator = SystemCoordinator::new();
        let file_id = FileId(11);
        let deps_id = DepsSnapshotId::from_hash("deps_stale_reject");
        let settings_id = SettingsId::from_hash("settings");

        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id.clone(),
            deps: make_deps(),
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: settings_id.clone(),
            diagnostics_detail_level: DetailLevel::Full,
        });
        let runtime = IntellisenseV2Facade::new(
            host,
            Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
            None,
        );
        runtime.apply_changes(vec![Change::SetFile {
            file_id,
            text: Arc::from("x = 1;"),
            version: 2,
            path: Arc::from("stale_reject.bsl"),
        }]);
        let _ = runtime.snapshot().await;

        let context = ExecutionContext {
            origin: ObservabilityOrigin::Lsp,
            operation: SemanticOperation::Completion,
            file_id,
            min_file_version: Some(5),
            expected_deps_id: Some(deps_id),
            flow_sensitive: false,
            settings: ExecutionSettings {
                settings_id: settings_id.clone(),
                diagnostics_detail_level: DetailLevel::Full,
            },
            cancellation: CancellationPolicy::BestEffort,
        };

        let wait_budget_ms = crate::system::global_runtime_config()
            .get_u64(crate::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
            .unwrap_or(120);
        let started = Instant::now();
        let result = runtime
            .prepare_stateful_operation(&context, Some(&coordinator))
            .await;
        let elapsed = started.elapsed();
        assert!(
            matches!(result, Err(SemanticOutcome::StaleVersion)),
            "gap > 1 should reject stale fallback under default policy"
        );
        let min_expected = Duration::from_millis(wait_budget_ms.saturating_sub(30));
        let max_expected = Duration::from_millis(wait_budget_ms.saturating_add(300));
        assert!(
            elapsed >= min_expected,
            "stale reject should spend wait budget before fail (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );
        assert!(
            elapsed <= max_expected,
            "stale reject should stay bounded near wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");
        assert!(
            counters.contains_key("intellisense_v2_completion_fallback_unavailable_total"),
            "completion fallback-unavailable counter should be recorded"
        );
        assert!(
            counters.contains_key("intellisense_v2_revision_lag_sample_total"),
            "revision lag counter should be recorded"
        );
        assert!(
            histograms.contains_key("intellisense_v2_revision_lag_versions"),
            "revision lag histogram should be recorded"
        );

        runtime.shutdown_for_test().await;
    }

    #[tokio::test]
    async fn interactive_prepare_timeout_rejects_stale_on_settings_mismatch() {
        let file_id = FileId(12);
        let deps_id = DepsSnapshotId::from_hash("deps_stale_mismatch");
        let stale_settings_id = SettingsId::from_hash("settings_old");
        let requested_settings_id = SettingsId::from_hash("settings_new");

        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id.clone(),
            deps: make_deps(),
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: stale_settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        });
        let runtime = IntellisenseV2Facade::new(
            host,
            Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
            None,
        );
        runtime.apply_changes(vec![Change::SetFile {
            file_id,
            text: Arc::from("x = 1;"),
            version: 4,
            path: Arc::from("stale_mismatch.bsl"),
        }]);
        let _ = runtime.snapshot().await;

        let context = ExecutionContext {
            origin: ObservabilityOrigin::Lsp,
            operation: SemanticOperation::SignatureHelp,
            file_id,
            min_file_version: Some(5),
            expected_deps_id: Some(deps_id),
            flow_sensitive: false,
            settings: ExecutionSettings {
                settings_id: requested_settings_id,
                diagnostics_detail_level: DetailLevel::Full,
            },
            cancellation: CancellationPolicy::BestEffort,
        };

        let wait_budget_ms = crate::system::global_runtime_config()
            .get_u64(crate::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
            .unwrap_or(120);
        let started = Instant::now();
        let result = runtime.prepare_stateful_operation(&context, None).await;
        let elapsed = started.elapsed();
        assert!(
            matches!(result, Err(SemanticOutcome::StaleVersion)),
            "settings mismatch must reject stale fallback"
        );
        let min_expected = Duration::from_millis(wait_budget_ms.saturating_sub(30));
        let max_expected = Duration::from_millis(wait_budget_ms.saturating_add(300));
        assert!(
            elapsed >= min_expected,
            "settings-mismatch reject should spend wait budget before fail (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );
        assert!(
            elapsed <= max_expected,
            "settings-mismatch reject should stay bounded near wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );

        runtime.shutdown_for_test().await;
    }

    #[tokio::test]
    async fn interactive_prepare_timeout_rejects_stale_without_expected_deps() {
        let runtime = IntellisenseV2Facade::new(
            AnalysisHostV2::default(),
            Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
            None,
        );
        let file_id = FileId(13);
        let settings_id = SettingsId::from_hash("settings");

        runtime.apply_changes(vec![
            Change::SetSettingsSnapshot {
                settings_id: settings_id.clone(),
                diagnostics_detail_level: DetailLevel::Full,
            },
            Change::SetFile {
                file_id,
                text: Arc::from("x = 1;"),
                version: 4,
                path: Arc::from("stale_no_expected_deps.bsl"),
            },
        ]);

        let context = ExecutionContext {
            origin: ObservabilityOrigin::Lsp,
            operation: SemanticOperation::Completion,
            file_id,
            min_file_version: Some(5),
            expected_deps_id: None,
            flow_sensitive: false,
            settings: ExecutionSettings {
                settings_id,
                diagnostics_detail_level: DetailLevel::Full,
            },
            cancellation: CancellationPolicy::BestEffort,
        };

        let result = runtime.prepare_stateful_operation(&context, None).await;
        assert!(
            matches!(result, Err(SemanticOutcome::StaleVersion)),
            "stale fallback must be rejected when expected deps snapshot is unknown"
        );

        runtime.shutdown_for_test().await;
    }

    #[test]
    fn run_parse_result_query_skips_when_policy_disallows_it() {
        let analysis = AnalysisHostV2::default().snapshot();
        let context = ExecutionContext {
            origin: ObservabilityOrigin::Lsp,
            operation: SemanticOperation::Hover,
            file_id: FileId(1),
            min_file_version: None,
            expected_deps_id: None,
            flow_sensitive: false,
            settings: ExecutionSettings {
                settings_id: SettingsId::from_hash("settings"),
                diagnostics_detail_level: DetailLevel::Full,
            },
            cancellation: CancellationPolicy::BestEffort,
        };

        let mut called = false;
        let result = IntellisenseV2Facade::run_parse_result_query(
            &context,
            &analysis,
            false,
            None,
            |_analysis| {
                called = true;
                Ok::<Option<()>, ()>(None)
            },
        )
        .expect("query should not fail");

        assert!(result.is_none(), "parse_result should be skipped by policy");
        assert!(
            !called,
            "query closure must not be called when policy skips"
        );
    }

    #[test]
    fn run_optional_query_records_ir_metrics() {
        let coordinator = SystemCoordinator::new();
        let analysis = AnalysisHostV2::default().snapshot();
        let context = ExecutionContext {
            origin: ObservabilityOrigin::Lsp,
            operation: SemanticOperation::Completion,
            file_id: FileId(1),
            min_file_version: None,
            expected_deps_id: None,
            flow_sensitive: false,
            settings: ExecutionSettings {
                settings_id: SettingsId::from_hash("settings"),
                diagnostics_detail_level: DetailLevel::Full,
            },
            cancellation: CancellationPolicy::BestEffort,
        };

        let _ = IntellisenseV2Facade::run_optional_query(
            &context,
            ObservabilityStage::IrQuery,
            &analysis,
            Some(&coordinator),
            |_analysis| Ok::<Option<()>, ()>(None),
        )
        .expect("query should succeed");

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");

        assert!(
            counters.contains_key("intellisense_v2_ir_query_completion_total"),
            "IR counter should be recorded for completion"
        );
        assert!(
            histograms.contains_key("intellisense_v2_ir_query_completion_ms"),
            "IR histogram should be recorded for completion"
        );
    }

    #[test]
    fn run_optional_query_best_effort_downgrades_cancellation_to_empty() {
        let coordinator = SystemCoordinator::new();
        let analysis = AnalysisHostV2::default().snapshot();
        let context = ExecutionContext {
            origin: ObservabilityOrigin::Lsp,
            operation: SemanticOperation::Members,
            file_id: FileId(1),
            min_file_version: None,
            expected_deps_id: None,
            flow_sensitive: false,
            settings: ExecutionSettings {
                settings_id: SettingsId::from_hash("settings"),
                diagnostics_detail_level: DetailLevel::Full,
            },
            cancellation: CancellationPolicy::BestEffort,
        };

        let result = IntellisenseV2Facade::run_optional_query(
            &context,
            ObservabilityStage::IrQuery,
            &analysis,
            Some(&coordinator),
            |_analysis| Err::<Option<()>, ()>(()),
        )
        .expect("best effort should downgrade cancellation");
        assert!(
            result.is_none(),
            "best effort cancellation must return empty"
        );

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let cancelled = counters
            .get("intellisense_v2_ir_query_cancelled_total_other")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        assert!(
            cancelled > 0,
            "best effort should still expose cancelled counters"
        );
    }

    #[test]
    fn run_optional_query_ignore_drops_cancellation_counters() {
        let coordinator = SystemCoordinator::new();
        let analysis = AnalysisHostV2::default().snapshot();
        let context = ExecutionContext {
            origin: ObservabilityOrigin::Lsp,
            operation: SemanticOperation::Members,
            file_id: FileId(1),
            min_file_version: None,
            expected_deps_id: None,
            flow_sensitive: false,
            settings: ExecutionSettings {
                settings_id: SettingsId::from_hash("settings"),
                diagnostics_detail_level: DetailLevel::Full,
            },
            cancellation: CancellationPolicy::Ignore,
        };

        let result = IntellisenseV2Facade::run_optional_query(
            &context,
            ObservabilityStage::IrQuery,
            &analysis,
            Some(&coordinator),
            |_analysis| Err::<Option<()>, ()>(()),
        )
        .expect("ignore policy should drop cancellation error");
        assert!(result.is_none(), "ignore policy must return empty result");

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let cancelled = counters
            .get("intellisense_v2_ir_query_cancelled_total_other")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        assert_eq!(
            cancelled, 0,
            "ignore policy should suppress cancelled counters"
        );
    }

    #[test]
    fn singleflight_scope_is_bound_only_for_ir() {
        assert!(
            IntellisenseV2Facade::singleflight_requires_snapshot_identity(
                SingleflightQueryKind::Ir
            ),
            "IR should remain tied to deps/settings snapshots"
        );
        assert!(
            !IntellisenseV2Facade::singleflight_requires_snapshot_identity(
                SingleflightQueryKind::ParseResult
            ),
            "parse_result should not be tied to deps/settings snapshots"
        );
        assert!(
            !IntellisenseV2Facade::singleflight_requires_snapshot_identity(
                SingleflightQueryKind::SyntaxDiagnostics
            ),
            "syntax_diagnostics should not be tied to deps/settings snapshots"
        );
    }

    #[test]
    fn singleflight_runs_leader_once_and_shares_result() {
        static TEST_FLIGHTS: OnceLock<SingleflightMap<Arc<String>>> = OnceLock::new();
        let key = SingleflightRevisionKey {
            file_id: FileId(777),
            file_version: 10,
            file_signature: "path:test://singleflight/777.bsl".to_string(),
            deps_id: Some(DepsSnapshotId::from_hash("deps")),
            settings_id: Some(SettingsId::from_hash("settings")),
            query_kind: SingleflightQueryKind::Ir,
        };
        let calls = Arc::new(AtomicUsize::new(0));

        let first_calls = calls.clone();
        let first_key = key.clone();
        let first = std::thread::spawn(move || {
            IntellisenseV2Facade::run_singleflight_query(
                &TEST_FLIGHTS,
                first_key,
                ObservabilityOrigin::Runtime,
                SingleflightQueryKind::Ir,
                None,
                || {
                    first_calls.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_millis(60));
                    Ok(Some(Arc::new(String::from("shared"))))
                },
            )
        });

        std::thread::sleep(std::time::Duration::from_millis(5));

        let second_calls = calls.clone();
        let second = std::thread::spawn(move || {
            IntellisenseV2Facade::run_singleflight_query(
                &TEST_FLIGHTS,
                key,
                ObservabilityOrigin::Runtime,
                SingleflightQueryKind::Ir,
                None,
                || {
                    second_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(Some(Arc::new(String::from("second"))))
                },
            )
        });

        let first_result = first.join().expect("first thread join").expect("first ok");
        let second_result = second
            .join()
            .expect("second thread join")
            .expect("second ok");

        assert_eq!(
            first_result.as_ref().map(|value| value.as_str()),
            Some("shared")
        );
        assert_eq!(
            second_result.as_ref().map(|value| value.as_str()),
            Some("shared")
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn singleflight_propagates_leader_cancel_without_retry_and_cleans_up() {
        static TEST_FLIGHTS: OnceLock<SingleflightMap<Arc<String>>> = OnceLock::new();
        let key = SingleflightRevisionKey {
            file_id: FileId(778),
            file_version: 10,
            file_signature: "path:test://singleflight/778.bsl".to_string(),
            deps_id: None,
            settings_id: None,
            query_kind: SingleflightQueryKind::ParseResult,
        };
        let calls = Arc::new(AtomicUsize::new(0));

        let first_calls = calls.clone();
        let first_key = key.clone();
        let first = std::thread::spawn(move || {
            IntellisenseV2Facade::run_singleflight_query(
                &TEST_FLIGHTS,
                first_key,
                ObservabilityOrigin::Runtime,
                SingleflightQueryKind::ParseResult,
                None,
                || {
                    first_calls.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_millis(60));
                    Err(SingleflightQueryError::Cancelled)
                },
            )
        });

        std::thread::sleep(std::time::Duration::from_millis(5));

        let second_calls = calls.clone();
        let second_key = key.clone();
        let second = std::thread::spawn(move || {
            IntellisenseV2Facade::run_singleflight_query(
                &TEST_FLIGHTS,
                second_key,
                ObservabilityOrigin::Runtime,
                SingleflightQueryKind::ParseResult,
                None,
                || {
                    second_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(Some(Arc::new(String::from("unexpected-retry"))))
                },
            )
        });

        let first_result = first.join().expect("first thread join");
        let second_result = second.join().expect("second thread join");
        assert!(first_result.is_err(), "leader must fail");
        assert!(
            second_result.is_err(),
            "follower must receive leader cancel"
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "follower must not trigger retry inside the same flight"
        );

        let map = TEST_FLIGHTS
            .get()
            .expect("test singleflight map should be initialized");
        let inflight_len = map
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len();
        assert_eq!(inflight_len, 0, "flight entry must be cleaned up");

        let rerun_calls = calls.clone();
        let rerun = IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::ParseResult,
            None,
            || {
                rerun_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Some(Arc::new(String::from("after-cleanup"))))
            },
        )
        .expect("new request after cleanup should run as new leader");
        assert_eq!(rerun.as_deref().map(String::as_str), Some("after-cleanup"));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn singleflight_leader_panic_is_downgraded_and_cleans_up() {
        static TEST_FLIGHTS: OnceLock<SingleflightMap<Arc<String>>> = OnceLock::new();
        let key = SingleflightRevisionKey {
            file_id: FileId(780),
            file_version: 10,
            file_signature: "path:test://singleflight/780.bsl".to_string(),
            deps_id: None,
            settings_id: None,
            query_kind: SingleflightQueryKind::SyntaxDiagnostics,
        };

        let first_key = key.clone();
        let first = std::thread::spawn(move || {
            IntellisenseV2Facade::run_singleflight_query(
                &TEST_FLIGHTS,
                first_key,
                ObservabilityOrigin::Runtime,
                SingleflightQueryKind::SyntaxDiagnostics,
                None,
                || {
                    std::thread::sleep(std::time::Duration::from_millis(60));
                    panic!("leader panic must not leak in-flight entry")
                },
            )
        });

        std::thread::sleep(std::time::Duration::from_millis(5));

        let second = std::thread::spawn(move || {
            IntellisenseV2Facade::run_singleflight_query(
                &TEST_FLIGHTS,
                key,
                ObservabilityOrigin::Runtime,
                SingleflightQueryKind::SyntaxDiagnostics,
                None,
                || Ok(Some(Arc::new(String::from("unexpected-after-panic")))),
            )
        });

        let first_result = first.join().expect("first thread join");
        let second_result = second.join().expect("second thread join");
        assert!(
            matches!(first_result, Err(SingleflightQueryError::Cancelled)),
            "leader panic must be exposed as cancelled outcome"
        );
        assert!(
            matches!(second_result, Err(SingleflightQueryError::Cancelled)),
            "follower must receive terminal leader outcome when panic happens"
        );

        let map = TEST_FLIGHTS
            .get()
            .expect("test singleflight map should be initialized");
        let inflight_len = map
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len();
        assert_eq!(
            inflight_len, 0,
            "singleflight key must be cleaned up after panic"
        );
    }

    #[test]
    fn singleflight_records_leader_shared_and_wait_metrics() {
        static TEST_FLIGHTS: OnceLock<SingleflightMap<Arc<String>>> = OnceLock::new();
        let key = SingleflightRevisionKey {
            file_id: FileId(779),
            file_version: 10,
            file_signature: "path:test://singleflight/779.bsl".to_string(),
            deps_id: None,
            settings_id: None,
            query_kind: SingleflightQueryKind::SyntaxDiagnostics,
        };
        let coordinator = Arc::new(SystemCoordinator::new());

        let first_coordinator = coordinator.clone();
        let first_key = key.clone();
        let first = std::thread::spawn(move || {
            IntellisenseV2Facade::run_singleflight_query(
                &TEST_FLIGHTS,
                first_key,
                ObservabilityOrigin::Runtime,
                SingleflightQueryKind::SyntaxDiagnostics,
                Some(first_coordinator.as_ref()),
                || {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    Ok(Some(Arc::new(String::from("shared"))))
                },
            )
        });

        std::thread::sleep(std::time::Duration::from_millis(5));

        let second_coordinator = coordinator.clone();
        let second = std::thread::spawn(move || {
            IntellisenseV2Facade::run_singleflight_query(
                &TEST_FLIGHTS,
                key,
                ObservabilityOrigin::Runtime,
                SingleflightQueryKind::SyntaxDiagnostics,
                Some(second_coordinator.as_ref()),
                || Ok(Some(Arc::new(String::from("second")))),
            )
        });

        let _ = first.join().expect("first thread join").expect("first ok");
        let _ = second
            .join()
            .expect("second thread join")
            .expect("second ok");

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");

        assert!(
            counters.contains_key("intellisense_v2_singleflight_leader_total"),
            "singleflight leader counter should be recorded"
        );
        assert!(
            counters.contains_key("intellisense_v2_singleflight_shared_total"),
            "singleflight shared counter should be recorded"
        );
        assert!(
            histograms.contains_key("intellisense_v2_singleflight_wait_ms"),
            "singleflight wait histogram should be recorded"
        );
    }

    #[tokio::test]
    async fn parity_stateful_and_ephemeral_diagnostics_are_equal() {
        let deps = make_deps();
        let deps_id = DepsSnapshotId::from_hash("deps_parity");
        let settings_id = SettingsId::from_hash("settings_parity");
        let settings = ExecutionSettings {
            settings_id: settings_id.clone(),
            diagnostics_detail_level: DetailLevel::Full,
        };
        let file_id = FileId(11);
        let code: Arc<str> =
            Arc::from("Процедура Тест()\n\tМассив1.Добавить(1);\nКонецПроцедуры\n");
        let path: Arc<str> = Arc::from("parity_test.bsl");

        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id.clone(),
            deps: deps.clone(),
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: settings_id.clone(),
            diagnostics_detail_level: DetailLevel::Full,
        });
        host.apply_change(Change::SetFile {
            file_id,
            text: code.clone(),
            version: 1,
            path: path.clone(),
        });
        let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index_parity"), None);
        let stateful = runtime.snapshot().await;

        let ephemeral = IntellisenseV2Facade::ephemeral_snapshot(
            deps_id,
            deps,
            make_index_snapshot("index_parity"),
            settings,
            file_id,
            code,
            1,
            path,
        )
        .analysis;

        let stateful_syntax = stateful
            .syntax_diagnostics(file_id)
            .expect("stateful syntax query")
            .unwrap_or_else(|| Arc::new(Vec::new()));
        let ephemeral_syntax = ephemeral
            .syntax_diagnostics(file_id)
            .expect("ephemeral syntax query")
            .unwrap_or_else(|| Arc::new(Vec::new()));

        let stateful_semantic = stateful
            .semantic_diagnostics(file_id)
            .expect("stateful semantic query")
            .unwrap_or_else(|| Arc::new(Vec::new()));
        let ephemeral_semantic = ephemeral
            .semantic_diagnostics(file_id)
            .expect("ephemeral semantic query")
            .unwrap_or_else(|| Arc::new(Vec::new()));

        let syntax_key = |d: &bsl_shared::domain::types::ParseError| {
            (d.message.clone(), d.span.start, d.span.end)
        };
        let semantic_key = |d: &bsl_shared::domain::types::TypeDiagnostic| {
            (
                d.message.clone(),
                d.span.start,
                d.span.end,
                format!("{:?}", d.severity),
            )
        };

        let mut left_syntax: Vec<_> = stateful_syntax.iter().map(syntax_key).collect();
        let mut right_syntax: Vec<_> = ephemeral_syntax.iter().map(syntax_key).collect();
        left_syntax.sort();
        right_syntax.sort();
        assert_eq!(
            left_syntax, right_syntax,
            "syntax diagnostics parity mismatch"
        );

        let mut left_semantic: Vec<_> = stateful_semantic.iter().map(semantic_key).collect();
        let mut right_semantic: Vec<_> = ephemeral_semantic.iter().map(semantic_key).collect();
        left_semantic.sort();
        right_semantic.sort();
        assert_eq!(
            left_semantic, right_semantic,
            "semantic diagnostics parity mismatch"
        );

        runtime.shutdown_for_test().await;
    }
}
