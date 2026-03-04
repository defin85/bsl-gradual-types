use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Condvar, OnceLock};
use std::time::{Duration, Instant};

use super::policy::{
    completion_fastpath_preconditions, interactive_freshness_knobs, should_query_parse_result,
    InteractiveFreshnessKnobs,
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
            | SemanticOperation::Definition => RuntimeQueuePriority::Interactive,
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
    pub completion_churn_fastpath_active: bool,
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
    interactive_tx: std::sync::mpsc::Sender<Command>,
    background_tx: std::sync::mpsc::Sender<Command>,
    #[cfg(test)]
    join_handle: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
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
        reply: oneshot::Sender<(AnalysisV2, Arc<IndexSnapshot>, DepsSnapshotId)>,
    },
    WaitForFileVersion {
        origin: ObservabilityOrigin,
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

const INTERACTIVE_BURST_QUOTA: usize = 8;

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
    reply: oneshot::Sender<bool>,
    started_waiting_at: Instant,
    origin: ObservabilityOrigin,
    priority: RuntimeQueuePriority,
}

#[cfg(test)]
#[path = "facade/tests.rs"]
mod tests;
