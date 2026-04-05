use arc_swap::ArcSwap;
use std::collections::{HashMap, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Condvar, OnceLock};
use std::time::{Duration, Instant};

use super::policy::{
    completion_fastpath_preconditions, interactive_freshness_knobs, should_query_parse_result,
};
use tokio::sync::oneshot;
use tracing::warn;

use crate::system::{IndexSnapshot, IndexSnapshotId, SystemCoordinator};
use bsl_analysis_v2::{
    AnalysisHostV2, AnalysisV2, Change, DepsSnapshotId, FileId, SemanticDeps, SettingsId,
};
use bsl_shared::domain::types::ParseError;
use bsl_shared::domain::types::TypeResolution;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeQueuePriority {
    Interactive,
    Background,
}

impl RuntimeQueuePriority {
    fn for_operation(operation: SemanticOperation) -> Self {
        match operation {
            SemanticOperation::Completion
            | SemanticOperation::Hover
            | SemanticOperation::SignatureHelp
            | SemanticOperation::Definition
            | SemanticOperation::Members
            | SemanticOperation::TypeAtPosition => RuntimeQueuePriority::Interactive,
            _ => RuntimeQueuePriority::Background,
        }
    }

    fn as_work_class(self) -> &'static str {
        match self {
            RuntimeQueuePriority::Interactive => "interactive",
            RuntimeQueuePriority::Background => "background",
        }
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
    /// Optional completion routing mode for mode-aware drilldown metrics.
    pub completion_mode: Option<&'static str>,
    /// True when scale-aware policy marks this file as `large + churn`.
    pub completion_large_churn_active: bool,
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

#[derive(Debug, Clone, Default)]
struct PrepareStatefulProgressState {
    phase: Option<&'static str>,
    phase_started_offset: Option<Duration>,
    wait_completed_offset: Option<Duration>,
    snapshot_completed_offset: Option<Duration>,
    snapshot_with_deps_timeout_runtime: Option<SnapshotWithDepsTimeoutRuntimeProgressState>,
}

#[derive(Debug, Clone, Default)]
pub struct PrepareStatefulProgressSnapshot {
    pub phase: Option<&'static str>,
    pub phase_started_offset: Option<Duration>,
    pub wait_completed_offset: Option<Duration>,
    pub snapshot_completed_offset: Option<Duration>,
    pub snapshot_with_deps_timeout_runtime: Option<SnapshotWithDepsTimeoutRuntimeTrace>,
}

#[derive(Debug, Clone)]
pub struct PrepareStatefulProgress {
    started_at: Instant,
    inner: Arc<std::sync::Mutex<PrepareStatefulProgressState>>,
}

impl Default for PrepareStatefulProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl PrepareStatefulProgress {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            inner: Arc::new(std::sync::Mutex::new(
                PrepareStatefulProgressState::default(),
            )),
        }
    }

    pub fn mark_phase(&self, phase: &'static str) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.phase = Some(phase);
        state.phase_started_offset = Some(self.started_at.elapsed());
        if phase != "snapshot_with_deps" {
            state.snapshot_with_deps_timeout_runtime = None;
        }
    }

    pub fn mark_wait_completed(&self) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.wait_completed_offset = Some(self.started_at.elapsed());
    }

    pub fn mark_snapshot_completed(&self) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.snapshot_completed_offset = Some(self.started_at.elapsed());
    }

    pub fn mark_snapshot_with_deps_queue_wait(&self) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.snapshot_with_deps_timeout_runtime =
            Some(SnapshotWithDepsTimeoutRuntimeProgressState::queue_wait());
    }

    pub fn mark_snapshot_with_deps_exec_started(&self, queue_wait_elapsed: Duration) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.snapshot_with_deps_timeout_runtime =
            Some(SnapshotWithDepsTimeoutRuntimeProgressState::exec(
                queue_wait_elapsed,
                self.started_at.elapsed(),
            ));
    }

    pub fn mark_snapshot_with_deps_wake_wait(
        &self,
        queue_wait_elapsed: Duration,
        exec_elapsed: Duration,
    ) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.snapshot_with_deps_timeout_runtime =
            Some(SnapshotWithDepsTimeoutRuntimeProgressState::wake_wait(
                queue_wait_elapsed,
                exec_elapsed,
                self.started_at.elapsed(),
            ));
    }

    pub fn mark_snapshot_with_deps_timeout_runtime_unavailable(&self) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.snapshot_with_deps_timeout_runtime =
            Some(SnapshotWithDepsTimeoutRuntimeProgressState::unavailable());
    }

    pub fn snapshot(&self) -> PrepareStatefulProgressSnapshot {
        let state = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        PrepareStatefulProgressSnapshot {
            phase: state.phase,
            phase_started_offset: state.phase_started_offset,
            wait_completed_offset: state.wait_completed_offset,
            snapshot_completed_offset: state.snapshot_completed_offset,
            snapshot_with_deps_timeout_runtime: state
                .snapshot_with_deps_timeout_runtime
                .map(|runtime| runtime.to_trace(self.started_at, &state)),
        }
    }
}

/// Shared snapshot payload consumed by semantic operations.
pub struct SemanticSnapshot {
    pub analysis: AnalysisV2,
    pub deps_id: DepsSnapshotId,
}

/// Request-scoped snapshot payload for completion first-response routing.
pub struct CompletionCurrentRevisionSnapshot {
    pub analysis: AnalysisV2,
    pub deps_id: DepsSnapshotId,
    pub index_snapshot: Arc<IndexSnapshot>,
}

/// Lightweight immutable bundle for completion routes that only need deps/index truth.
pub struct CompletionSupportBundle {
    pub deps: Arc<SemanticDeps>,
    pub deps_id: DepsSnapshotId,
    pub index_snapshot: Arc<IndexSnapshot>,
}

/// Narrow immutable payload for completion first-response routing.
pub struct CompletionFirstResponseSupport {
    pub deps: Option<Arc<SemanticDeps>>,
    pub deps_id: DepsSnapshotId,
    pub index_snapshot: Arc<IndexSnapshot>,
    pub settings_id: Option<SettingsId>,
    pub file_content: Option<Arc<str>>,
    pub file_path: Option<Arc<str>>,
    pub head_owner_type_hints: Vec<TypeResolution>,
    pub head_ready: bool,
    pub exact_ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionFirstResponseReadiness {
    HeadReady,
    ExactReady,
    NotReady,
}

/// Prepared current-revision state for completion first response.
pub struct PreparedCompletionFirstResponse {
    pub support: CompletionFirstResponseSupport,
    pub wait_elapsed: Option<Duration>,
    pub snapshot_elapsed: Duration,
    pub wait_for_file_version_runtime: Option<WaitForFileVersionRuntimeTrace>,
    pub timeout_attribution: Option<PrepareTimeoutAttributionTrace>,
    pub wait_budget_exhausted: bool,
    pub observed_file_version: Option<i32>,
    pub readiness: CompletionFirstResponseReadiness,
}

/// Prepared operation state after canonical wait/snapshot sequencing.
pub struct PreparedOperationSnapshot {
    pub snapshot: SemanticSnapshot,
    pub index_snapshot: Arc<IndexSnapshot>,
    pub wait_elapsed: Option<Duration>,
    pub snapshot_elapsed: Duration,
    pub wait_for_file_version_runtime: Option<WaitForFileVersionRuntimeTrace>,
    pub snapshot_with_deps_runtime: SnapshotWithDepsRuntimeTrace,
    pub timeout_attribution: Option<PrepareTimeoutAttributionTrace>,
    pub wait_budget_exhausted: bool,
    pub stale_served: bool,
    pub completion_churn_fastpath_active: bool,
    pub observed_file_version: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrepareTimeoutSourceKind {
    PrepareGuard,
    InteractiveWaitBudget,
}

impl PrepareTimeoutSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PrepareGuard => "prepare_guard",
            Self::InteractiveWaitBudget => "interactive_wait_budget",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PrepareTimeoutAttributionTrace {
    pub source: PrepareTimeoutSourceKind,
    pub phase: &'static str,
    pub budget: Duration,
    pub elapsed: Duration,
    pub overshoot: Duration,
}

impl PrepareTimeoutAttributionTrace {
    pub fn new(
        source: PrepareTimeoutSourceKind,
        phase: &'static str,
        budget: Duration,
        elapsed: Duration,
    ) -> Self {
        Self {
            source,
            phase,
            budget,
            elapsed,
            overshoot: elapsed.saturating_sub(budget),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitForFileVersionResolutionKind {
    Immediate,
    Waiter,
}

impl WaitForFileVersionResolutionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Immediate => "immediate",
            Self::Waiter => "waiter",
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WaitForFileVersionRuntimeTrace {
    pub queue_wait_elapsed: Option<Duration>,
    pub exec_elapsed: Option<Duration>,
    pub wake_wait_elapsed: Option<Duration>,
    pub resolution: Option<WaitForFileVersionResolutionKind>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SnapshotWithDepsRuntimeTrace {
    pub queue_wait_elapsed: Option<Duration>,
    pub exec_elapsed: Option<Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotWithDepsTimeoutResolutionKind {
    QueueWait,
    Exec,
    WakeWait,
    Unavailable,
}

impl SnapshotWithDepsTimeoutResolutionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::QueueWait => "queue_wait",
            Self::Exec => "exec",
            Self::WakeWait => "wake_wait",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SnapshotWithDepsTimeoutRuntimeTrace {
    pub queue_wait_elapsed: Option<Duration>,
    pub exec_elapsed: Option<Duration>,
    pub wake_wait_elapsed: Option<Duration>,
    pub resolution: SnapshotWithDepsTimeoutResolutionKind,
}

#[derive(Debug, Clone, Copy)]
struct SnapshotWithDepsTimeoutRuntimeProgressState {
    resolution: SnapshotWithDepsTimeoutResolutionKind,
    queue_wait_elapsed: Option<Duration>,
    exec_started_offset: Option<Duration>,
    exec_elapsed: Option<Duration>,
    wake_wait_started_offset: Option<Duration>,
}

impl SnapshotWithDepsTimeoutRuntimeProgressState {
    fn queue_wait() -> Self {
        Self {
            resolution: SnapshotWithDepsTimeoutResolutionKind::QueueWait,
            queue_wait_elapsed: None,
            exec_started_offset: None,
            exec_elapsed: None,
            wake_wait_started_offset: None,
        }
    }

    fn exec(queue_wait_elapsed: Duration, exec_started_offset: Duration) -> Self {
        Self {
            resolution: SnapshotWithDepsTimeoutResolutionKind::Exec,
            queue_wait_elapsed: Some(queue_wait_elapsed),
            exec_started_offset: Some(exec_started_offset),
            exec_elapsed: None,
            wake_wait_started_offset: None,
        }
    }

    fn wake_wait(
        queue_wait_elapsed: Duration,
        exec_elapsed: Duration,
        wake_wait_started_offset: Duration,
    ) -> Self {
        Self {
            resolution: SnapshotWithDepsTimeoutResolutionKind::WakeWait,
            queue_wait_elapsed: Some(queue_wait_elapsed),
            exec_started_offset: None,
            exec_elapsed: Some(exec_elapsed),
            wake_wait_started_offset: Some(wake_wait_started_offset),
        }
    }

    fn unavailable() -> Self {
        Self {
            resolution: SnapshotWithDepsTimeoutResolutionKind::Unavailable,
            queue_wait_elapsed: None,
            exec_started_offset: None,
            exec_elapsed: None,
            wake_wait_started_offset: None,
        }
    }

    fn to_trace(
        self,
        started_at: Instant,
        state: &PrepareStatefulProgressState,
    ) -> SnapshotWithDepsTimeoutRuntimeTrace {
        let now_offset = started_at.elapsed();
        let queue_wait_elapsed = match (self.queue_wait_elapsed, state.phase_started_offset) {
            (Some(value), _) => Some(value),
            (None, Some(phase_started_offset))
                if self.resolution == SnapshotWithDepsTimeoutResolutionKind::QueueWait =>
            {
                Some(now_offset.saturating_sub(phase_started_offset))
            }
            _ => None,
        };
        let exec_elapsed = match (self.exec_elapsed, self.exec_started_offset) {
            (Some(value), _) => Some(value),
            (None, Some(exec_started_offset))
                if self.resolution == SnapshotWithDepsTimeoutResolutionKind::Exec =>
            {
                Some(now_offset.saturating_sub(exec_started_offset))
            }
            _ => None,
        };
        let wake_wait_elapsed = match self.wake_wait_started_offset {
            Some(wake_wait_started_offset)
                if self.resolution == SnapshotWithDepsTimeoutResolutionKind::WakeWait =>
            {
                Some(now_offset.saturating_sub(wake_wait_started_offset))
            }
            _ => None,
        };
        SnapshotWithDepsTimeoutRuntimeTrace {
            queue_wait_elapsed,
            exec_elapsed,
            wake_wait_elapsed,
            resolution: self.resolution,
        }
    }
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
    interactive_tx: std::sync::mpsc::Sender<Command>,
    background_tx: std::sync::mpsc::Sender<Command>,
    completion_deps_index_snapshot: Arc<ArcSwap<CompletionDepsIndexSnapshot>>,
    #[cfg(test)]
    join_handle: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

#[derive(Clone)]
struct CompletionDepsIndexSnapshot {
    deps: Arc<SemanticDeps>,
    deps_id: DepsSnapshotId,
    index_snapshot: Arc<IndexSnapshot>,
}

enum Command {
    ApplyChanges {
        origin: ObservabilityOrigin,
        enqueued_at: Instant,
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
        origin: ObservabilityOrigin,
        enqueued_at: Instant,
        progress: Option<PrepareStatefulProgress>,
        reply: oneshot::Sender<GetSnapshotWithDepsReply>,
    },
    WaitForFileVersion {
        origin: ObservabilityOrigin,
        enqueued_at: Instant,
        file_id: FileId,
        min_version: i32,
        reply: oneshot::Sender<WaitForFileVersionReply>,
    },
    GetFileRevisionState {
        file_id: FileId,
        reply: oneshot::Sender<Option<FileRevisionState>>,
    },
    #[cfg(test)]
    TestSleep {
        duration: Duration,
        ack: oneshot::Sender<()>,
    },
    #[cfg(test)]
    TestNoop {
        ack: oneshot::Sender<()>,
    },
    #[cfg(test)]
    Shutdown {
        ack: oneshot::Sender<()>,
    },
}

struct CurrentRevisionApplyCommand {
    origin: ObservabilityOrigin,
    enqueued_at: Instant,
    file_id: FileId,
    version: i32,
    text: Arc<str>,
    path: Arc<str>,
    reuse_previous_version: Option<i32>,
}

impl CurrentRevisionApplyCommand {
    fn try_from_command(command: Command) -> Result<Self, Command> {
        let Command::ApplyChanges {
            origin,
            enqueued_at,
            changes,
        } = command
        else {
            return Err(command);
        };
        if origin != ObservabilityOrigin::Lsp {
            return Err(Command::ApplyChanges {
                origin,
                enqueued_at,
                changes,
            });
        }
        let parsed = match changes.as_slice() {
            [Change::SetFile {
                file_id,
                text,
                version,
                path,
            }] => Some(Self {
                origin,
                enqueued_at,
                file_id: *file_id,
                version: *version,
                text: text.clone(),
                path: path.clone(),
                reuse_previous_version: None,
            }),
            [Change::SetFile {
                file_id,
                text,
                version,
                path,
            }, Change::ReuseCompletionHeadFromPreviousVersion {
                file_id: reuse_file_id,
                expected_version,
                previous_version,
            }] if file_id == reuse_file_id && version == expected_version => Some(Self {
                origin,
                enqueued_at,
                file_id: *file_id,
                version: *version,
                text: text.clone(),
                path: path.clone(),
                reuse_previous_version: Some(*previous_version),
            }),
            _ => None,
        };
        parsed.ok_or(Command::ApplyChanges {
            origin,
            enqueued_at,
            changes,
        })
    }

    fn can_supersede(&self, newer: &Self) -> bool {
        self.file_id == newer.file_id && newer.version > self.version
    }

    fn supersede_with(&mut self, newer: Self) {
        let reuse_previous_version = match newer.reuse_previous_version {
            Some(newer_previous_version) if newer_previous_version == self.version => {
                self.reuse_previous_version.or(Some(newer_previous_version))
            }
            Some(newer_previous_version) => Some(newer_previous_version),
            None => None,
        };
        self.enqueued_at = self.enqueued_at.min(newer.enqueued_at);
        self.version = newer.version;
        self.text = newer.text;
        self.path = newer.path;
        // Preserve the earliest reusable base across a coalesced whitespace-append chain so
        // the latest current-revision SetFile can still publish a completion head immediately.
        self.reuse_previous_version = reuse_previous_version;
    }

    fn into_command(self) -> Command {
        let mut changes = vec![Change::SetFile {
            file_id: self.file_id,
            text: self.text,
            version: self.version,
            path: self.path,
        }];
        if let Some(previous_version) = self.reuse_previous_version {
            changes.push(Change::ReuseCompletionHeadFromPreviousVersion {
                file_id: self.file_id,
                expected_version: self.version,
                previous_version,
            });
        }
        Command::ApplyChanges {
            origin: self.origin,
            enqueued_at: self.enqueued_at,
            changes,
        }
    }
}

const INTERACTIVE_BURST_QUOTA: usize = 8;
const CURRENT_REVISION_COALESCE_WINDOW: Duration = Duration::from_millis(4);

fn try_recv_next_writer_command_nonblocking(
    interactive_rx: &std::sync::mpsc::Receiver<Command>,
    background_rx: &std::sync::mpsc::Receiver<Command>,
    interactive_streak: &mut usize,
    interactive_closed: &mut bool,
    background_closed: &mut bool,
) -> Option<(RuntimeQueuePriority, Command)> {
    use std::sync::mpsc::TryRecvError;

    if *interactive_streak >= INTERACTIVE_BURST_QUOTA {
        *interactive_streak = 0;
        if !*background_closed {
            match background_rx.try_recv() {
                Ok(command) => return Some((RuntimeQueuePriority::Background, command)),
                Err(TryRecvError::Disconnected) => *background_closed = true,
                Err(TryRecvError::Empty) => {}
            }
        }
    }

    if !*interactive_closed {
        match interactive_rx.try_recv() {
            Ok(command) => {
                *interactive_streak = interactive_streak.saturating_add(1);
                return Some((RuntimeQueuePriority::Interactive, command));
            }
            Err(TryRecvError::Disconnected) => *interactive_closed = true,
            Err(TryRecvError::Empty) => {}
        }
    }

    if !*background_closed {
        match background_rx.try_recv() {
            Ok(command) => {
                *interactive_streak = 0;
                return Some((RuntimeQueuePriority::Background, command));
            }
            Err(TryRecvError::Disconnected) => *background_closed = true,
            Err(TryRecvError::Empty) => {}
        }
    }

    None
}

fn recv_next_writer_command(
    interactive_rx: &std::sync::mpsc::Receiver<Command>,
    background_rx: &std::sync::mpsc::Receiver<Command>,
    interactive_streak: &mut usize,
    interactive_closed: &mut bool,
    background_closed: &mut bool,
) -> Option<(RuntimeQueuePriority, Command)> {
    use std::sync::mpsc::{RecvTimeoutError, TryRecvError};

    loop {
        if *interactive_streak >= INTERACTIVE_BURST_QUOTA {
            *interactive_streak = 0;
            if !*background_closed {
                match background_rx.try_recv() {
                    Ok(command) => return Some((RuntimeQueuePriority::Background, command)),
                    Err(TryRecvError::Disconnected) => *background_closed = true,
                    Err(TryRecvError::Empty) => {}
                }
            }
        }

        if !*interactive_closed {
            match interactive_rx.try_recv() {
                Ok(command) => {
                    *interactive_streak = interactive_streak.saturating_add(1);
                    return Some((RuntimeQueuePriority::Interactive, command));
                }
                Err(TryRecvError::Disconnected) => *interactive_closed = true,
                Err(TryRecvError::Empty) => {}
            }
        }

        if !*background_closed {
            match background_rx.try_recv() {
                Ok(command) => {
                    *interactive_streak = 0;
                    return Some((RuntimeQueuePriority::Background, command));
                }
                Err(TryRecvError::Disconnected) => *background_closed = true,
                Err(TryRecvError::Empty) => {}
            }
        }

        if *interactive_closed && *background_closed {
            return None;
        }

        if !*interactive_closed {
            match interactive_rx.recv_timeout(Duration::from_millis(2)) {
                Ok(command) => {
                    *interactive_streak = interactive_streak.saturating_add(1);
                    return Some((RuntimeQueuePriority::Interactive, command));
                }
                Err(RecvTimeoutError::Disconnected) => *interactive_closed = true,
                Err(RecvTimeoutError::Timeout) => {}
            }
        }

        if !*background_closed {
            match background_rx.recv_timeout(Duration::from_millis(2)) {
                Ok(command) => {
                    *interactive_streak = 0;
                    return Some((RuntimeQueuePriority::Background, command));
                }
                Err(RecvTimeoutError::Disconnected) => *background_closed = true,
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }
}

fn coalesce_interactive_current_revision_apply_command(
    interactive_rx: &std::sync::mpsc::Receiver<Command>,
    command: Command,
    pending_interactive_commands: &mut VecDeque<Command>,
) -> Command {
    use std::sync::mpsc::{RecvTimeoutError, TryRecvError};

    let mut current = match CurrentRevisionApplyCommand::try_from_command(command) {
        Ok(current) => current,
        Err(command) => return command,
    };
    let coalesce_deadline = Instant::now() + CURRENT_REVISION_COALESCE_WINDOW;

    loop {
        let next_command = match interactive_rx.try_recv() {
            Ok(next_command) => Some(next_command),
            Err(TryRecvError::Empty) => {
                let remaining = coalesce_deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    None
                } else {
                    match interactive_rx.recv_timeout(remaining) {
                        Ok(next_command) => Some(next_command),
                        Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => None,
                    }
                }
            }
            Err(TryRecvError::Disconnected) => None,
        };

        match next_command {
            Some(next_command) => {
                match CurrentRevisionApplyCommand::try_from_command(next_command) {
                    Ok(next) if current.can_supersede(&next) => {
                        current.supersede_with(next);
                    }
                    Ok(next) => {
                        pending_interactive_commands.push_back(next.into_command());
                    }
                    Err(next_command) => {
                        pending_interactive_commands.push_back(next_command);
                    }
                }
            }
            None => break,
        }
    }

    current.into_command()
}

fn promote_interactive_current_revision_apply_command(
    interactive_rx: &std::sync::mpsc::Receiver<Command>,
    pending_interactive_commands: &mut VecDeque<Command>,
) -> Option<Command> {
    use std::sync::mpsc::TryRecvError;

    let mut promoted: Option<CurrentRevisionApplyCommand> = None;
    let mut fresh_pending = VecDeque::new();
    loop {
        match interactive_rx.try_recv() {
            Ok(next_command) => match CurrentRevisionApplyCommand::try_from_command(next_command) {
                Ok(next) => match &mut promoted {
                    Some(current) if current.can_supersede(&next) => {
                        current.supersede_with(next);
                    }
                    Some(_) => fresh_pending.push_back(next.into_command()),
                    None => promoted = Some(next),
                },
                Err(next_command) => fresh_pending.push_back(next_command),
            },
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
        }
    }

    while let Some(command) = fresh_pending.pop_back() {
        pending_interactive_commands.push_front(command);
    }

    promoted.map(CurrentRevisionApplyCommand::into_command)
}

fn promote_pending_current_revision_apply_command(
    pending_interactive_commands: &mut VecDeque<Command>,
) -> Option<Command> {
    for idx in (0..pending_interactive_commands.len()).rev() {
        let Some(command) = pending_interactive_commands.remove(idx) else {
            continue;
        };
        match CurrentRevisionApplyCommand::try_from_command(command) {
            Ok(current) => return Some(current.into_command()),
            Err(command) => {
                pending_interactive_commands.insert(idx, command);
            }
        }
    }
    None
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

#[path = "facade/operations.rs"]
mod operations;
#[path = "facade/runtime.rs"]
mod runtime;

struct PendingWaiter {
    min_version: i32,
    reply: oneshot::Sender<WaitForFileVersionReply>,
    queue_wait_elapsed: Duration,
    started_waiting_at: Instant,
    origin: ObservabilityOrigin,
    priority: RuntimeQueuePriority,
}

struct GetSnapshotWithDepsReply {
    analysis: AnalysisV2,
    index_snapshot: Arc<IndexSnapshot>,
    deps_id: DepsSnapshotId,
    trace: SnapshotWithDepsRuntimeTrace,
}

#[derive(Debug, Clone, Copy)]
struct WaitForFileVersionReply {
    ready: bool,
    trace: WaitForFileVersionRuntimeTrace,
}

#[cfg(test)]
#[path = "facade/tests.rs"]
mod tests;
