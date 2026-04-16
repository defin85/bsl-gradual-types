//! Command-specific handlers for BslLanguageServer
//!
//! Contains implementation of helper methods for command handling.

use bsl_backend::system::fs_utils::read_bsl_file;
use bsl_line_index::LineIndex;
use bsl_syntax::ast::ParseResult;
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::{MessageType, Url};
use tracing::{info, warn};

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use bsl_analysis_v2::FileId as V2FileId;
use tokio::sync::Notify;

use crate::commands::{
    handle_incremental_update, handle_parse_configuration, ParseConfigurationParams,
};
use crate::handlers::{find_containing_function_in_parse_result, CurrentContextResponse};
use crate::types::{
    AutoReindexCommandParams, AutoReindexStateResponse, BuildIndexParams, BuildIndexResponse,
    CompletionTimelineRequest, CompletionTimelineResponse, DiagnosticsSaveTimelineRequest,
    DiagnosticsSaveTimelineResponse, GetCurrentContextParams, GetIndexStateParams,
    GetIndexStateResponse, GetSnapshotStatusRequest, IncrementalUpdateParams,
    IncrementalUpdateResponse, ObservabilityMetricsRequest, ObservabilityMetricsResponse,
    WorkspaceStatsResponse,
};
use bsl_shared::api::dtos::SnapshotReadinessDto;

use super::{BslLanguageServer, FullIndexOperationKind, FullIndexStateKind};

const ATTACHED_MESSAGE: &str = "already running (attached)";
const CURRENT_CONTEXT_LATEST_GENERATIONS_MAX_SESSIONS: usize = 256;
const CURRENT_CONTEXT_READY_SNAPSHOT_WAIT_BUDGET_MS: u64 = 100;
const CURRENT_CONTEXT_PARSE_BROKER_WAIT_BUDGET_MS: u64 = 2_000;
const CURRENT_CONTEXT_PARSE_BROKER_WAIT_POLL_MS: u64 = 25;

#[cfg(test)]
fn maybe_inject_get_current_context_parse_delay_for_test() {
    if let Some(delay_ms) = std::env::var("BSL_TEST_GET_CURRENT_CONTEXT_PARSE_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
    {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
}

#[cfg(not(test))]
fn maybe_inject_get_current_context_parse_delay_for_test() {}

#[cfg(test)]
static GET_CURRENT_CONTEXT_PARSE_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
static GET_CURRENT_CONTEXT_PARSE_CANCELLATIONS: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
fn record_get_current_context_parse_attempt_for_test() {
    GET_CURRENT_CONTEXT_PARSE_ATTEMPTS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(not(test))]
fn record_get_current_context_parse_attempt_for_test() {}

#[cfg(test)]
fn record_get_current_context_parse_cancellation_for_test() {
    GET_CURRENT_CONTEXT_PARSE_CANCELLATIONS.fetch_add(1, AtomicOrdering::SeqCst);
}

#[cfg(not(test))]
fn record_get_current_context_parse_cancellation_for_test() {}

#[cfg(test)]
fn current_context_ready_snapshot_wait_budget() -> Duration {
    std::env::var("BSL_TEST_GET_CURRENT_CONTEXT_READY_SNAPSHOT_WAIT_BUDGET_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_millis(CURRENT_CONTEXT_READY_SNAPSHOT_WAIT_BUDGET_MS))
}

#[cfg(not(test))]
fn current_context_ready_snapshot_wait_budget() -> Duration {
    Duration::from_millis(CURRENT_CONTEXT_READY_SNAPSHOT_WAIT_BUDGET_MS)
}

fn current_context_ready_snapshot_latest_only_stabilization_budget() -> Duration {
    current_context_ready_snapshot_wait_budget().saturating_add(Duration::from_millis(
        CURRENT_CONTEXT_PARSE_BROKER_WAIT_POLL_MS,
    ))
}

#[cfg(test)]
fn current_context_parse_broker_wait_budget() -> Duration {
    std::env::var("BSL_TEST_GET_CURRENT_CONTEXT_BROKER_WAIT_BUDGET_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_millis(CURRENT_CONTEXT_PARSE_BROKER_WAIT_BUDGET_MS))
}

#[cfg(not(test))]
fn current_context_parse_broker_wait_budget() -> Duration {
    Duration::from_millis(CURRENT_CONTEXT_PARSE_BROKER_WAIT_BUDGET_MS)
}

#[cfg(test)]
pub(crate) fn reset_get_current_context_parse_attempts_for_test() {
    GET_CURRENT_CONTEXT_PARSE_ATTEMPTS.store(0, AtomicOrdering::SeqCst);
    GET_CURRENT_CONTEXT_PARSE_CANCELLATIONS.store(0, AtomicOrdering::SeqCst);
}

#[cfg(test)]
pub(crate) fn get_current_context_parse_attempts_for_test() -> usize {
    GET_CURRENT_CONTEXT_PARSE_ATTEMPTS.load(AtomicOrdering::SeqCst)
}

#[cfg(test)]
pub(crate) fn get_current_context_parse_cancellations_for_test() -> usize {
    GET_CURRENT_CONTEXT_PARSE_CANCELLATIONS.load(AtomicOrdering::SeqCst)
}

#[derive(Debug, Clone)]
pub(crate) enum BeginFullIndexOutcome {
    Started {
        operation_id: String,
    },
    AlreadyRunning {
        active_operation: Option<FullIndexOperationKind>,
        operation_id: Option<String>,
    },
}

#[derive(Debug, Clone)]
struct CurrentContextSupersessionKey {
    editor_session_id: String,
    request_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CurrentContextParseBrokerKey {
    file_id: V2FileId,
    file_version: Option<i32>,
    text_hash: [u8; 32],
}

#[derive(Debug, Clone)]
pub(crate) struct CurrentContextLatestGenerationState {
    request_generation: u64,
    broker_key: CurrentContextParseBrokerKey,
    active_parse_cancellation_flag: Option<std::sync::Weak<std::sync::atomic::AtomicBool>>,
}

pub(crate) type CurrentContextLatestGenerationRegistry =
    std::sync::Mutex<std::collections::HashMap<String, CurrentContextLatestGenerationState>>;

#[derive(Debug, Clone, Copy)]
struct CurrentContextParseObservability {
    parse_source: &'static str,
    parse_elapsed: Duration,
}

#[derive(Debug, Clone)]
enum CurrentContextParseSharedResult {
    Parsed {
        parse_result: Arc<ParseResult>,
        line_index: Arc<LineIndex>,
        parse_observability: CurrentContextParseObservability,
    },
    ParseUnavailable {
        parse_observability: CurrentContextParseObservability,
    },
    Superseded,
}

#[derive(Debug)]
pub(crate) struct CurrentContextParseBrokerEntry {
    result: std::sync::Mutex<Option<CurrentContextParseSharedResult>>,
    notify: Notify,
}

pub(crate) type CurrentContextParseBroker = std::sync::Mutex<
    std::collections::HashMap<CurrentContextParseBrokerKey, Arc<CurrentContextParseBrokerEntry>>,
>;

#[derive(Debug, Clone, Copy)]
enum CurrentContextRoute {
    ReadySnapshot,
    BrokerLeader,
    BrokerFollower,
}

#[derive(Debug, Clone, Copy)]
enum CurrentContextTerminalOutcome {
    Resolved,
    ParseUnavailable,
    Superseded,
    BudgetExhausted,
}

enum CurrentContextParseBrokerAcquireOutcome {
    Leader(Arc<CurrentContextParseBrokerEntry>),
    Follower(Arc<CurrentContextParseBrokerEntry>),
}

enum CurrentContextParseBrokerWaitOutcome {
    Resolved(CurrentContextParseSharedResult),
    Superseded,
    BudgetExhausted,
}

impl CurrentContextSupersessionKey {
    fn from_params(params: &GetCurrentContextParams) -> Option<Self> {
        Some(Self {
            editor_session_id: params.editor_session_id.clone()?,
            request_generation: params.request_generation?,
        })
    }
}

impl CurrentContextParseBrokerKey {
    fn new(file_id: V2FileId, file_version: Option<i32>, text: &str) -> Self {
        Self {
            file_id,
            file_version,
            text_hash: *blake3::hash(text.as_bytes()).as_bytes(),
        }
    }
}

impl CurrentContextParseBrokerEntry {
    fn new() -> Self {
        Self {
            result: std::sync::Mutex::new(None),
            notify: Notify::new(),
        }
    }

    fn take_shared_result(&self) -> Option<CurrentContextParseSharedResult> {
        self.result
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn publish_shared_result(&self, result: CurrentContextParseSharedResult) {
        *self
            .result
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(result);
        self.notify.notify_waiters();
    }
}

impl CurrentContextRoute {
    fn as_str(self) -> &'static str {
        match self {
            Self::ReadySnapshot => "ready_snapshot",
            Self::BrokerLeader => "broker_leader",
            Self::BrokerFollower => "broker_follower",
        }
    }
}

impl CurrentContextTerminalOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Resolved => "resolved",
            Self::ParseUnavailable => "parse_unavailable",
            Self::Superseded => "superseded",
            Self::BudgetExhausted => "budget_exhausted",
        }
    }
}

fn acquire_current_context_parse_broker_entry(
    broker: &CurrentContextParseBroker,
    key: CurrentContextParseBrokerKey,
) -> CurrentContextParseBrokerAcquireOutcome {
    let mut broker = broker
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(entry) = broker.get(&key) {
        return CurrentContextParseBrokerAcquireOutcome::Follower(Arc::clone(entry));
    }
    let entry = Arc::new(CurrentContextParseBrokerEntry::new());
    broker.insert(key, Arc::clone(&entry));
    CurrentContextParseBrokerAcquireOutcome::Leader(entry)
}

fn release_current_context_parse_broker_entry(
    broker: &CurrentContextParseBroker,
    key: &CurrentContextParseBrokerKey,
    expected_entry: &Arc<CurrentContextParseBrokerEntry>,
) {
    let mut broker = broker
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if broker
        .get(key)
        .is_some_and(|registered| Arc::ptr_eq(registered, expected_entry))
    {
        broker.remove(key);
    }
}

async fn wait_for_current_context_parse_broker_result(
    entry: &Arc<CurrentContextParseBrokerEntry>,
    latest_generations: &CurrentContextLatestGenerationRegistry,
    supersession_key: Option<&CurrentContextSupersessionKey>,
    wait_budget: Duration,
) -> CurrentContextParseBrokerWaitOutcome {
    let deadline = tokio::time::Instant::now() + wait_budget;
    loop {
        if let Some(result) = entry.take_shared_result() {
            return CurrentContextParseBrokerWaitOutcome::Resolved(result);
        }
        if let Some(supersession_key) = supersession_key {
            if !is_latest_current_context_generation(latest_generations, supersession_key) {
                return CurrentContextParseBrokerWaitOutcome::Superseded;
            }
        }

        let now = tokio::time::Instant::now();
        if now >= deadline {
            return CurrentContextParseBrokerWaitOutcome::BudgetExhausted;
        }

        let notified = entry.notify.notified();
        if let Some(result) = entry.take_shared_result() {
            return CurrentContextParseBrokerWaitOutcome::Resolved(result);
        }

        tokio::select! {
            _ = notified => {}
            _ = tokio::time::sleep(
                (deadline - now).min(Duration::from_millis(CURRENT_CONTEXT_PARSE_BROKER_WAIT_POLL_MS))
            ) => {}
        }
    }
}

fn resolve_current_context_from_parse(
    parse_result: &ParseResult,
    file_text: &str,
    line_index: &LineIndex,
    line: u32,
    character: u32,
) -> CurrentContextResponse {
    match find_containing_function_in_parse_result(
        parse_result,
        file_text,
        line_index,
        line,
        character,
    ) {
        Some((name, kind, params_list, return_type)) => CurrentContextResponse {
            function_name: Some(name),
            function_kind: kind,
            params: Some(params_list),
            return_type,
        },
        None => CurrentContextResponse::empty(),
    }
}

fn record_current_context_request_observability(
    coordinator: &bsl_backend::system::SystemCoordinator,
    route: Option<CurrentContextRoute>,
    terminal_outcome: CurrentContextTerminalOutcome,
    parse_observability: Option<CurrentContextParseObservability>,
    wall_elapsed: Duration,
) {
    if let Some(parse_observability) = parse_observability {
        coordinator
            .record_intellisense_v2_current_context_parse_source(parse_observability.parse_source);
        coordinator.record_intellisense_v2_current_context_parse_latency(
            parse_observability.parse_source,
            parse_observability.parse_elapsed,
        );
        coordinator.record_intellisense_v2_current_context_wall_latency(
            parse_observability.parse_source,
            wall_elapsed,
        );
    }
    if let Some(route) = route {
        coordinator.record_intellisense_v2_current_context_role(route.as_str());
        coordinator.record_intellisense_v2_current_context_wall_latency_by_role(
            route.as_str(),
            wall_elapsed,
        );
        if let Some(parse_observability) = parse_observability {
            coordinator.record_intellisense_v2_current_context_parse_latency_by_role(
                route.as_str(),
                parse_observability.parse_elapsed,
            );
        }
    }
    coordinator.record_intellisense_v2_current_context_terminal_outcome(terminal_outcome.as_str());
}

fn register_current_context_generation(
    latest_generations: &CurrentContextLatestGenerationRegistry,
    supersession_key: &CurrentContextSupersessionKey,
    broker_key: &CurrentContextParseBrokerKey,
) -> bool {
    let mut latest_generations = latest_generations
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let accepted = match latest_generations.get_mut(&supersession_key.editor_session_id) {
        Some(latest_generation)
            if latest_generation.request_generation > supersession_key.request_generation =>
        {
            false
        }
        Some(latest_generation) => {
            let carry_cancellation_flag = if latest_generation.broker_key == *broker_key {
                latest_generation.active_parse_cancellation_flag.clone()
            } else {
                None
            };
            if latest_generation.request_generation < supersession_key.request_generation
                && latest_generation.broker_key != *broker_key
            {
                if let Some(cancellation_flag) = latest_generation
                    .active_parse_cancellation_flag
                    .as_ref()
                    .and_then(std::sync::Weak::upgrade)
                {
                    cancellation_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
            *latest_generation = CurrentContextLatestGenerationState {
                request_generation: supersession_key.request_generation,
                broker_key: broker_key.clone(),
                active_parse_cancellation_flag: carry_cancellation_flag,
            };
            true
        }
        None => {
            latest_generations.insert(
                supersession_key.editor_session_id.clone(),
                CurrentContextLatestGenerationState {
                    request_generation: supersession_key.request_generation,
                    broker_key: broker_key.clone(),
                    active_parse_cancellation_flag: None,
                },
            );
            true
        }
    };
    if !accepted {
        return false;
    }
    while latest_generations.len() > CURRENT_CONTEXT_LATEST_GENERATIONS_MAX_SESSIONS {
        let Some(oldest_session_id) = latest_generations
            .iter()
            .min_by_key(|(_, generation)| generation.request_generation)
            .map(|(session_id, _)| session_id.clone())
        else {
            break;
        };
        latest_generations.remove(&oldest_session_id);
    }
    true
}

fn is_latest_current_context_generation(
    latest_generations: &CurrentContextLatestGenerationRegistry,
    supersession_key: &CurrentContextSupersessionKey,
) -> bool {
    latest_generations
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&supersession_key.editor_session_id)
        .is_some_and(|state| state.request_generation == supersession_key.request_generation)
}

fn current_context_generation_allows_equivalent_parse_reuse(
    latest_generations: &CurrentContextLatestGenerationRegistry,
    supersession_key: &CurrentContextSupersessionKey,
    broker_key: &CurrentContextParseBrokerKey,
) -> bool {
    let latest_generations = latest_generations
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(state) = latest_generations.get(&supersession_key.editor_session_id) else {
        return true;
    };
    if state.request_generation <= supersession_key.request_generation {
        return true;
    }
    state.broker_key == *broker_key
}

fn attach_current_context_generation_parse_cancellation_flag(
    latest_generations: &CurrentContextLatestGenerationRegistry,
    supersession_key: &CurrentContextSupersessionKey,
    broker_key: &CurrentContextParseBrokerKey,
    cancellation_flag: &Arc<std::sync::atomic::AtomicBool>,
) {
    let mut latest_generations = latest_generations
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(state) = latest_generations.get_mut(&supersession_key.editor_session_id) else {
        return;
    };
    if state.broker_key != *broker_key
        || state.request_generation < supersession_key.request_generation
    {
        return;
    }
    state.active_parse_cancellation_flag = Some(Arc::downgrade(cancellation_flag));
}

async fn wait_for_current_context_ready_snapshot_latest_only_stabilization(
    latest_generations: &CurrentContextLatestGenerationRegistry,
    generation_notify: &Notify,
    supersession_key: Option<&CurrentContextSupersessionKey>,
) -> bool {
    let Some(supersession_key) = supersession_key else {
        return true;
    };
    let stabilization_budget = current_context_ready_snapshot_latest_only_stabilization_budget();
    if stabilization_budget.is_zero() {
        return is_latest_current_context_generation(latest_generations, supersession_key);
    }

    let deadline = tokio::time::Instant::now() + stabilization_budget;
    loop {
        if !is_latest_current_context_generation(latest_generations, supersession_key) {
            return false;
        }

        let now = tokio::time::Instant::now();
        if now >= deadline {
            return true;
        }

        let notified = generation_notify.notified();
        tokio::pin!(notified);
        tokio::select! {
            _ = &mut notified => {}
            _ = tokio::time::sleep(
                (deadline - now).min(Duration::from_millis(CURRENT_CONTEXT_PARSE_BROKER_WAIT_POLL_MS))
            ) => {}
        }
    }
}

impl BslLanguageServer {
    pub(crate) async fn handle_get_index_state(
        &self,
        _params: GetIndexStateParams,
    ) -> JsonRpcResult<GetIndexStateResponse> {
        Ok(self.current_index_state().await)
    }

    pub(crate) async fn current_index_state(&self) -> GetIndexStateResponse {
        let state = self.full_index_state.lock().await;
        state.to_response()
    }

    pub(crate) async fn begin_full_index_operation(
        &self,
        kind: FullIndexOperationKind,
        message: impl Into<String>,
    ) -> BeginFullIndexOutcome {
        let message = message.into();
        let operation_id = format!(
            "{}-{}",
            kind.as_str(),
            self.next_full_index_operation_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );

        {
            let mut state = self.full_index_state.lock().await;
            if state.state == FullIndexStateKind::Running {
                return BeginFullIndexOutcome::AlreadyRunning {
                    active_operation: state.active_operation,
                    operation_id: state.operation_id.clone(),
                };
            }

            state.state = FullIndexStateKind::Running;
            state.active_operation = Some(kind);
            state.operation_id = Some(operation_id.clone());
            state.message = Some(message);
            state.updated_at_ms = crate::server::unix_timestamp_ms();
        }

        self.spawn_full_index_watchdog(operation_id.clone(), self.full_index_watchdog_timeout);

        BeginFullIndexOutcome::Started { operation_id }
    }

    pub(crate) fn spawn_full_index_watchdog(&self, operation_id: String, timeout: Duration) {
        let state_holder = self.full_index_state.clone();
        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;

            let mut state = state_holder.lock().await;
            if state.state != FullIndexStateKind::Running {
                return;
            }
            if state.operation_id.as_deref() != Some(operation_id.as_str()) {
                return;
            }

            state.state = FullIndexStateKind::Failed;
            state.active_operation = None;
            state.operation_id = None;
            state.message = Some(format!(
                "full-index timeout after {}ms",
                timeout.as_millis()
            ));
            state.updated_at_ms = crate::server::unix_timestamp_ms();
        });
    }

    pub(crate) async fn finish_full_index_operation_success(
        &self,
        operation_id: &str,
        message: impl Into<String>,
    ) {
        self.finish_full_index_operation(operation_id, FullIndexStateKind::Ready, message)
            .await;
    }

    pub(crate) async fn finish_full_index_operation_failed(
        &self,
        operation_id: &str,
        message: impl Into<String>,
    ) {
        self.finish_full_index_operation(operation_id, FullIndexStateKind::Failed, message)
            .await;
    }

    async fn finish_full_index_operation(
        &self,
        operation_id: &str,
        final_state: FullIndexStateKind,
        message: impl Into<String>,
    ) {
        let mut state = self.full_index_state.lock().await;
        if state.operation_id.as_deref() != Some(operation_id) {
            return;
        }

        state.state = final_state;
        state.active_operation = None;
        state.operation_id = None;
        state.message = Some(message.into());
        state.updated_at_ms = crate::server::unix_timestamp_ms();
    }

    fn attached_build_index_response(
        active_operation: Option<FullIndexOperationKind>,
        operation_id: Option<String>,
    ) -> BuildIndexResponse {
        let active = active_operation.map(|op| op.as_str()).unwrap_or("unknown");
        let suffix = operation_id
            .as_ref()
            .map(|id| format!(" (operation_id={id})"))
            .unwrap_or_default();
        BuildIndexResponse {
            success: true,
            types_count: 0,
            message: format!("{ATTACHED_MESSAGE}: active_operation={active}{suffix}"),
        }
    }

    /// Handle bsl.getCurrentContext command
    pub(crate) async fn handle_get_current_context(
        &self,
        params: GetCurrentContextParams,
    ) -> JsonRpcResult<CurrentContextResponse> {
        info!(
            "Custom command: bsl.getCurrentContext - {}:{}:{}",
            params.uri, params.line, params.character
        );

        let uri = Url::parse(&params.uri).map_err(|e| {
            tower_lsp::jsonrpc::Error::invalid_params(format!("Invalid URI: {}", e))
        })?;
        let supersession_key = CurrentContextSupersessionKey::from_params(&params);

        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let path = match uri.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => uri.to_string(),
        };
        let shadow_state = self
            .latest_document_shadow_state_v2
            .read()
            .await
            .get(&file_id)
            .cloned();
        let file_text = shadow_state
            .as_ref()
            .map(|state| state.text.clone())
            .or_else(|| {
                uri.to_file_path()
                    .ok()
                    .and_then(|path| read_bsl_file(&path).ok().map(Arc::from))
            });
        let Some(file_text) = file_text else {
            warn!(
                uri = %uri,
                file_id = file_id.0,
                "getCurrentContext: document text is unavailable"
            );
            record_current_context_request_observability(
                self.coordinator.as_ref(),
                None,
                CurrentContextTerminalOutcome::ParseUnavailable,
                None,
                Duration::ZERO,
            );
            return Ok(CurrentContextResponse::empty());
        };
        let ready_parse_snapshot = if let Some(shadow_state) = shadow_state.as_ref() {
            self.latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .filter(|state| {
                    state.parse_snapshot.file_version == shadow_state.version
                        && state.text.as_ref() == shadow_state.text.as_ref()
                })
                .map(|state| {
                    (
                        Arc::clone(&state.parse_snapshot.parse_result),
                        Arc::clone(&state.parse_snapshot.line_index),
                    )
                })
        } else {
            None
        };
        let line = params.line;
        let character = params.character;
        let current_context_started = Instant::now();
        let broker_key = CurrentContextParseBrokerKey::new(
            file_id,
            shadow_state.as_ref().map(|state| state.version),
            file_text.as_ref(),
        );

        if let Some(supersession_key) = supersession_key.as_ref() {
            if !register_current_context_generation(
                self.current_context_latest_generations.as_ref(),
                supersession_key,
                &broker_key,
            ) {
                record_current_context_request_observability(
                    self.coordinator.as_ref(),
                    None,
                    CurrentContextTerminalOutcome::Superseded,
                    None,
                    Duration::ZERO,
                );
                return Ok(CurrentContextResponse::empty());
            }
            self.current_context_generation_notify.notify_waiters();
        }

        let mut exact_task_wait_superseded = false;
        let ready_parse_snapshot = if ready_parse_snapshot.is_some() {
            ready_parse_snapshot
        } else if let Some(shadow_state) = shadow_state.as_ref() {
            let wait_started = Instant::now();
            let wait_budget = current_context_ready_snapshot_wait_budget();
            let expected_text_hash = Some(broker_key.text_hash);
            loop {
                let ready = self
                    .latest_ready_parse_snapshots_v2
                    .read()
                    .await
                    .get(&file_id)
                    .filter(|state| {
                        state.parse_snapshot.file_version == shadow_state.version
                            && state.text.as_ref() == shadow_state.text.as_ref()
                    })
                    .map(|state| {
                        (
                            Arc::clone(&state.parse_snapshot.parse_result),
                            Arc::clone(&state.parse_snapshot.line_index),
                        )
                    });
                if ready.is_some() {
                    break ready;
                }
                if supersession_key.as_ref().is_some_and(|supersession_key| {
                    !is_latest_current_context_generation(
                        self.current_context_latest_generations.as_ref(),
                        supersession_key,
                    )
                }) {
                    exact_task_wait_superseded = true;
                    break None;
                }
                let remaining = wait_budget.saturating_sub(wait_started.elapsed());
                if remaining == Duration::ZERO {
                    break None;
                }
                let Some(task_control) = self
                    .matching_background_parse_snapshot_task_control_v2(
                        file_id,
                        shadow_state.version,
                        expected_text_hash,
                    )
                    .await
                else {
                    break None;
                };
                let materialized = task_control.materialized_notify.notified();
                let control = task_control.control_notify.notified();
                tokio::pin!(materialized);
                tokio::pin!(control);
                tokio::select! {
                    _ = tokio::time::sleep(remaining.min(Duration::from_millis(CURRENT_CONTEXT_PARSE_BROKER_WAIT_POLL_MS))) => {}
                    _ = &mut materialized => {}
                    _ = &mut control => {}
                }
            }
        } else {
            None
        };

        if exact_task_wait_superseded {
            record_current_context_request_observability(
                self.coordinator.as_ref(),
                None,
                CurrentContextTerminalOutcome::Superseded,
                None,
                current_context_started.elapsed(),
            );
            return Ok(CurrentContextResponse::empty());
        }

        if let Some((parse_result, line_index)) = ready_parse_snapshot.as_ref() {
            let latest_generation_stable =
                wait_for_current_context_ready_snapshot_latest_only_stabilization(
                    self.current_context_latest_generations.as_ref(),
                    self.current_context_generation_notify.as_ref(),
                    supersession_key.as_ref(),
                )
                .await;
            let wall_elapsed = current_context_started.elapsed();
            let parse_observability = CurrentContextParseObservability {
                parse_source: "ready_snapshot",
                parse_elapsed: Duration::ZERO,
            };
            let terminal_outcome = if latest_generation_stable {
                CurrentContextTerminalOutcome::Resolved
            } else {
                CurrentContextTerminalOutcome::Superseded
            };
            record_current_context_request_observability(
                self.coordinator.as_ref(),
                Some(CurrentContextRoute::ReadySnapshot),
                terminal_outcome,
                Some(parse_observability),
                wall_elapsed,
            );
            return if matches!(terminal_outcome, CurrentContextTerminalOutcome::Resolved) {
                Ok(resolve_current_context_from_parse(
                    parse_result.as_ref(),
                    file_text.as_ref(),
                    line_index.as_ref(),
                    line,
                    character,
                ))
            } else {
                Ok(CurrentContextResponse::empty())
            };
        }

        let route_and_shared_result = match acquire_current_context_parse_broker_entry(
            self.current_context_parse_broker.as_ref(),
            broker_key.clone(),
        ) {
            CurrentContextParseBrokerAcquireOutcome::Leader(entry) => {
                if supersession_key.as_ref().is_some_and(|supersession_key| {
                    !current_context_generation_allows_equivalent_parse_reuse(
                        self.current_context_latest_generations.as_ref(),
                        supersession_key,
                        &broker_key,
                    )
                }) {
                    let shared_result = CurrentContextParseSharedResult::Superseded;
                    entry.publish_shared_result(shared_result.clone());
                    release_current_context_parse_broker_entry(
                        self.current_context_parse_broker.as_ref(),
                        &broker_key,
                        &entry,
                    );
                    (CurrentContextRoute::BrokerLeader, shared_result)
                } else {
                    let coordinator = self.coordinator.clone();
                    let latest_generations = self.current_context_latest_generations.clone();
                    let path_for_parse = PathBuf::from(path.as_str());
                    let file_text_for_parse = file_text.clone();
                    let broker_key_for_parse = broker_key.clone();
                    let supersession_key_for_parse = supersession_key.clone();
                    let cancellation_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
                    if let Some(supersession_key) = supersession_key.as_ref() {
                        attach_current_context_generation_parse_cancellation_flag(
                            self.current_context_latest_generations.as_ref(),
                            supersession_key,
                            &broker_key,
                            &cancellation_flag,
                        );
                    }
                    let cancellation_flag_for_parse = Arc::clone(&cancellation_flag);
                    let shared_result = match bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                        bsl_runtime::application::CpuWorkClass::Background,
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        Some(self.coordinator.as_ref()),
                        move || {
                            let parse_started = Instant::now();
                            maybe_inject_get_current_context_parse_delay_for_test();
                            if supersession_key_for_parse.as_ref().is_some_and(|supersession_key| {
                                !current_context_generation_allows_equivalent_parse_reuse(
                                    latest_generations.as_ref(),
                                    supersession_key,
                                    &broker_key_for_parse,
                                )
                            }) {
                                cancellation_flag_for_parse
                                    .store(true, std::sync::atomic::Ordering::SeqCst);
                                return CurrentContextParseSharedResult::Superseded;
                            }
                            record_get_current_context_parse_attempt_for_test();
                            let parser_result = coordinator.parser_coordinator().and_then(|parser| {
                                parser
                                    .parse_current_context_with_cancellation(
                                        path_for_parse.clone(),
                                        file_text_for_parse.to_string(),
                                        cancellation_flag_for_parse.as_ref(),
                                    )
                                    .ok()
                            });
                            let parse_elapsed = parse_started.elapsed();
                            if let Some(report) = parser_result {
                                CurrentContextParseSharedResult::Parsed {
                                    parse_result: Arc::new(report.parse_result),
                                    line_index: report.line_index,
                                    parse_observability: CurrentContextParseObservability {
                                        parse_source: "parser_coordinator",
                                        parse_elapsed,
                                    },
                                }
                            } else if cancellation_flag_for_parse
                                .load(std::sync::atomic::Ordering::SeqCst)
                            {
                                record_get_current_context_parse_cancellation_for_test();
                                CurrentContextParseSharedResult::Superseded
                            } else if supersession_key_for_parse.as_ref().is_some_and(|supersession_key| {
                                !current_context_generation_allows_equivalent_parse_reuse(
                                    latest_generations.as_ref(),
                                    supersession_key,
                                    &broker_key_for_parse,
                                )
                            }) {
                                CurrentContextParseSharedResult::Superseded
                            } else if let Ok(parse_result) = bsl_syntax::parse(
                                file_text_for_parse.as_ref(),
                                &bsl_syntax::ParseOptions::default(),
                            ) {
                                CurrentContextParseSharedResult::Parsed {
                                    parse_result: Arc::new(parse_result),
                                    line_index: Arc::new(LineIndex::new(file_text_for_parse.as_ref())),
                                    parse_observability: CurrentContextParseObservability {
                                        parse_source: "syntax_fallback",
                                        parse_elapsed,
                                    },
                                }
                            } else {
                                CurrentContextParseSharedResult::ParseUnavailable {
                                    parse_observability: CurrentContextParseObservability {
                                        parse_source: "parse_unavailable",
                                        parse_elapsed,
                                    },
                                }
                            }
                        },
                    )
                    .await {
                        Ok(shared_result) => shared_result,
                        Err(err) => {
                            release_current_context_parse_broker_entry(
                                self.current_context_parse_broker.as_ref(),
                                &broker_key,
                                &entry,
                            );
                            warn!(
                                uri = %uri,
                                file_id = file_id.0,
                                error = %err,
                                "getCurrentContext: auxiliary parse task failed"
                            );
                            record_current_context_request_observability(
                                self.coordinator.as_ref(),
                                Some(CurrentContextRoute::BrokerLeader),
                                CurrentContextTerminalOutcome::ParseUnavailable,
                                None,
                                current_context_started.elapsed(),
                            );
                            return Ok(CurrentContextResponse::empty());
                        }
                    };
                    entry.publish_shared_result(shared_result.clone());
                    release_current_context_parse_broker_entry(
                        self.current_context_parse_broker.as_ref(),
                        &broker_key,
                        &entry,
                    );
                    (CurrentContextRoute::BrokerLeader, shared_result)
                }
            }
            CurrentContextParseBrokerAcquireOutcome::Follower(entry) => {
                match wait_for_current_context_parse_broker_result(
                    &entry,
                    self.current_context_latest_generations.as_ref(),
                    supersession_key.as_ref(),
                    current_context_parse_broker_wait_budget(),
                )
                .await
                {
                    CurrentContextParseBrokerWaitOutcome::Resolved(shared_result) => {
                        (CurrentContextRoute::BrokerFollower, shared_result)
                    }
                    CurrentContextParseBrokerWaitOutcome::Superseded => {
                        record_current_context_request_observability(
                            self.coordinator.as_ref(),
                            Some(CurrentContextRoute::BrokerFollower),
                            CurrentContextTerminalOutcome::Superseded,
                            None,
                            current_context_started.elapsed(),
                        );
                        return Ok(CurrentContextResponse::empty());
                    }
                    CurrentContextParseBrokerWaitOutcome::BudgetExhausted => {
                        record_current_context_request_observability(
                            self.coordinator.as_ref(),
                            Some(CurrentContextRoute::BrokerFollower),
                            CurrentContextTerminalOutcome::BudgetExhausted,
                            None,
                            current_context_started.elapsed(),
                        );
                        return Ok(CurrentContextResponse::empty());
                    }
                }
            }
        };

        let (route, shared_result) = route_and_shared_result;
        let wall_elapsed = current_context_started.elapsed();
        match shared_result {
            CurrentContextParseSharedResult::Parsed {
                parse_result,
                line_index,
                parse_observability,
            } => {
                let terminal_outcome =
                    if supersession_key.as_ref().is_some_and(|supersession_key| {
                        !is_latest_current_context_generation(
                            self.current_context_latest_generations.as_ref(),
                            supersession_key,
                        )
                    }) {
                        CurrentContextTerminalOutcome::Superseded
                    } else {
                        CurrentContextTerminalOutcome::Resolved
                    };
                record_current_context_request_observability(
                    self.coordinator.as_ref(),
                    Some(route),
                    terminal_outcome,
                    Some(parse_observability),
                    wall_elapsed,
                );
                if !matches!(terminal_outcome, CurrentContextTerminalOutcome::Resolved) {
                    return Ok(CurrentContextResponse::empty());
                }
                Ok(resolve_current_context_from_parse(
                    parse_result.as_ref(),
                    file_text.as_ref(),
                    line_index.as_ref(),
                    line,
                    character,
                ))
            }
            CurrentContextParseSharedResult::ParseUnavailable {
                parse_observability,
            } => {
                let terminal_outcome =
                    if supersession_key.as_ref().is_some_and(|supersession_key| {
                        !is_latest_current_context_generation(
                            self.current_context_latest_generations.as_ref(),
                            supersession_key,
                        )
                    }) {
                        CurrentContextTerminalOutcome::Superseded
                    } else {
                        CurrentContextTerminalOutcome::ParseUnavailable
                    };
                record_current_context_request_observability(
                    self.coordinator.as_ref(),
                    Some(route),
                    terminal_outcome,
                    Some(parse_observability),
                    wall_elapsed,
                );
                if matches!(
                    terminal_outcome,
                    CurrentContextTerminalOutcome::ParseUnavailable
                ) {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        "getCurrentContext: parse snapshot is unavailable"
                    );
                }
                Ok(CurrentContextResponse::empty())
            }
            CurrentContextParseSharedResult::Superseded => {
                record_current_context_request_observability(
                    self.coordinator.as_ref(),
                    Some(route),
                    CurrentContextTerminalOutcome::Superseded,
                    None,
                    wall_elapsed,
                );
                Ok(CurrentContextResponse::empty())
            }
        }
    }

    /// Custom request: bsl/buildIndex
    ///
    /// MVP: переиспользуем pipeline parseConfiguration (сервер — источник истины, прогресс через $/progress).
    pub(crate) async fn handle_build_index(
        &self,
        _params: BuildIndexParams,
    ) -> JsonRpcResult<BuildIndexResponse> {
        let operation_id = match self
            .begin_full_index_operation(FullIndexOperationKind::BuildIndex, "Building BSL index")
            .await
        {
            BeginFullIndexOutcome::Started { operation_id } => operation_id,
            BeginFullIndexOutcome::AlreadyRunning {
                active_operation,
                operation_id,
            } => {
                return Ok(Self::attached_build_index_response(
                    active_operation,
                    operation_id,
                ));
            }
        };

        let cfg = self.config.read().await.clone();
        let Some(cfg) = cfg else {
            let message =
                "LSP config not available (initializationOptions not received)".to_string();
            self.finish_full_index_operation_failed(&operation_id, message.clone())
                .await;
            return Ok(BuildIndexResponse {
                success: false,
                types_count: 0,
                message,
            });
        };

        let platform_docs_root = cfg.platform_docs_archive.as_deref().map(PathBuf::from);

        let Some(config_path) = cfg.configuration_path else {
            let message = "configurationPath is not configured".to_string();
            self.finish_full_index_operation_failed(&operation_id, message.clone())
                .await;
            return Ok(BuildIndexResponse {
                success: false,
                types_count: 0,
                message,
            });
        };

        let config_root = PathBuf::from(&config_path);

        let resp = handle_parse_configuration(
            ParseConfigurationParams { config_path },
            self.coordinator.get_domain_bundle(),
            self.client.clone(),
            "bsl-build-index",
            "Building BSL index",
            Some(self.coordinator.clone()),
        )
        .await;

        if resp.success {
            self.deps_update_v2("bsl/buildIndex", platform_docs_root, Some(config_root))
                .await;
            self.sync_v2_globals().await;
            self.finish_full_index_operation_success(&operation_id, "Index build completed")
                .await;
        } else {
            self.finish_full_index_operation_failed(
                &operation_id,
                resp.message
                    .clone()
                    .unwrap_or_else(|| "Index build failed".to_string()),
            )
            .await;
        }

        Ok(BuildIndexResponse {
            success: resp.success,
            types_count: resp.loaded_types,
            message: resp
                .message
                .unwrap_or_else(|| "Index build completed".to_string()),
        })
    }

    /// Custom request: bsl/incrementalUpdate
    ///
    /// MVP: сейчас это честная переиндексация конфигурации без перезапуска LSP.
    pub(crate) async fn handle_incremental_update(
        &self,
        params: IncrementalUpdateParams,
    ) -> JsonRpcResult<IncrementalUpdateResponse> {
        if params.is_auto {
            let paused = *self.auto_reindex_paused.read().await;
            if paused {
                warn!("Auto reindex skipped: paused");
                self.client
                    .log_message(
                        MessageType::INFO,
                        "Auto reindex is paused; incrementalUpdate skipped.",
                    )
                    .await;
                return Ok(IncrementalUpdateResponse {
                    success: false,
                    message: "Auto reindex paused".to_string(),
                });
            }
        }

        let platform_docs_root = {
            let config = self.config.read().await;
            config
                .as_ref()
                .and_then(|cfg| cfg.platform_docs_archive.as_deref())
                .map(PathBuf::from)
        };
        let config_root = PathBuf::from(&params.config_path);

        let resp =
            handle_incremental_update(params, self.coordinator.clone(), self.client.clone()).await;

        if resp.success {
            self.deps_update_v2(
                "bsl/incrementalUpdate",
                platform_docs_root,
                Some(config_root),
            )
            .await;
            self.sync_v2_globals().await;
        }

        Ok(IncrementalUpdateResponse {
            success: resp.success,
            message: resp.message,
        })
    }

    /// Custom request: bsl/pauseAutoReindex
    pub(crate) async fn handle_pause_auto_reindex(
        &self,
        _params: AutoReindexCommandParams,
    ) -> JsonRpcResult<AutoReindexStateResponse> {
        let mut paused = self.auto_reindex_paused.write().await;
        if !*paused {
            *paused = true;
            info!("Auto reindex paused via LSP");
        }

        self.client
            .log_message(MessageType::INFO, "Auto reindex paused.")
            .await;

        Ok(AutoReindexStateResponse {
            success: true,
            paused: true,
            message: "Auto reindex paused".to_string(),
        })
    }

    /// Custom request: bsl/resumeAutoReindex
    pub(crate) async fn handle_resume_auto_reindex(
        &self,
        _params: AutoReindexCommandParams,
    ) -> JsonRpcResult<AutoReindexStateResponse> {
        let mut paused = self.auto_reindex_paused.write().await;
        if *paused {
            *paused = false;
            info!("Auto reindex resumed via LSP");
        }

        self.client
            .log_message(MessageType::INFO, "Auto reindex resumed.")
            .await;

        Ok(AutoReindexStateResponse {
            success: true,
            paused: false,
            message: "Auto reindex resumed".to_string(),
        })
    }

    /// Custom request: bsl/getWorkspaceStats
    pub(crate) async fn handle_get_workspace_stats(&self) -> JsonRpcResult<WorkspaceStatsResponse> {
        let config = self.config.read().await.clone();
        let root = resolve_workspace_root(config);
        let bsl_files = root.as_deref().map(count_bsl_files).unwrap_or(0);

        let diagnostics = {
            let counts = self.diagnostics_counts.read().await;
            counts.values().sum()
        };

        Ok(WorkspaceStatsResponse {
            bsl_files,
            diagnostics,
        })
    }

    /// Custom request: bsl/getSnapshotStatus
    pub(crate) async fn handle_get_snapshot_status(
        &self,
        request: GetSnapshotStatusRequest,
    ) -> JsonRpcResult<SnapshotReadinessDto> {
        let uri = Url::parse(request.uri.as_str()).map_err(|err| {
            tower_lsp::jsonrpc::Error::invalid_params(format!("Invalid uri: {err}"))
        })?;
        Ok(self.snapshot_status_for_uri_v2(&uri).await)
    }

    /// Custom request: bsl/getObservabilityMetrics
    pub(crate) async fn handle_get_observability_metrics(
        &self,
        request: ObservabilityMetricsRequest,
    ) -> JsonRpcResult<ObservabilityMetricsResponse> {
        let metrics = if request.shape.as_deref() == Some("sidebar") {
            self.coordinator.observability_metrics_sidebar()
        } else {
            self.coordinator.observability_metrics()
        };
        let did_change_parse_snapshot_evidence = if request.shape.as_deref() == Some("sidebar") {
            None
        } else {
            Some(crate::types::DidChangeParseSnapshotEvidenceResponse {
                version: super::DID_CHANGE_PARSE_SNAPSHOT_EVIDENCE_VERSION,
                entries: self.snapshot_did_change_parse_snapshot_evidence(
                    super::DID_CHANGE_PARSE_SNAPSHOT_EVIDENCE_MAX_ENTRIES,
                ),
            })
        };
        Ok(ObservabilityMetricsResponse {
            metrics,
            did_change_parse_snapshot_evidence,
        })
    }

    pub(crate) async fn handle_get_completion_timeline(
        &self,
        params: CompletionTimelineRequest,
    ) -> JsonRpcResult<CompletionTimelineResponse> {
        let default_limit = super::COMPLETION_TIMELINE_MAX_ENTRIES;
        let limit = params
            .limit
            .unwrap_or(default_limit)
            .clamp(1, super::COMPLETION_TIMELINE_MAX_ENTRIES);
        let request_id_filter = params.request_id.as_deref();

        let traces_guard = self
            .completion_timeline_traces
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let traces = traces_guard
            .iter()
            .rev()
            .filter(|trace| match request_id_filter {
                Some(request_id) => trace.request_id.as_deref() == Some(request_id),
                None => true,
            })
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();

        Ok(CompletionTimelineResponse {
            version: super::COMPLETION_TIMELINE_VERSION,
            traces: traces.into_iter().rev().collect(),
        })
    }

    pub(crate) async fn handle_get_diagnostics_save_timeline(
        &self,
        params: DiagnosticsSaveTimelineRequest,
    ) -> JsonRpcResult<DiagnosticsSaveTimelineResponse> {
        let limit = params
            .limit
            .unwrap_or(super::DIAGNOSTICS_SAVE_TIMELINE_MAX_ENTRIES)
            .clamp(1, super::DIAGNOSTICS_SAVE_TIMELINE_MAX_ENTRIES);

        Ok(DiagnosticsSaveTimelineResponse {
            version: super::DIAGNOSTICS_SAVE_TIMELINE_VERSION,
            traces: self.snapshot_diagnostics_save_timeline_traces(limit),
        })
    }
}

fn resolve_workspace_root(config: Option<crate::config::LspConfig>) -> Option<PathBuf> {
    let config_path = config.and_then(|cfg| cfg.configuration_path);
    let path = config_path.map(PathBuf::from)?;
    if path.is_dir() {
        return Some(path);
    }

    if path.file_name().and_then(|name| name.to_str()) == Some("Configuration.xml") {
        return path.parent().map(|parent| parent.to_path_buf());
    }

    None
}

fn count_bsl_files(root: &Path) -> usize {
    let mut count = 0usize;
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.')
                        || name == "target"
                        || name == "node_modules"
                        || name == ".bsl_cache"
                    {
                        continue;
                    }
                }
                stack.push(path);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("bsl") {
                count += 1;
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_backend::system::SystemCoordinator;
    use futures::StreamExt;
    use std::sync::{Arc, Mutex as StdMutex};
    use tower::Service;
    use tower::ServiceExt;
    use tower_lsp::jsonrpc::Request;
    use tower_lsp::lsp_types::{ClientCapabilities, InitializeParams, InitializedParams};
    use tower_lsp::LspService;

    fn create_test_server() -> BslLanguageServer {
        let coordinator = Arc::new(SystemCoordinator::new());
        let holder: Arc<StdMutex<Option<BslLanguageServer>>> = Arc::new(StdMutex::new(None));

        let (_service, _socket) = LspService::build({
            let coordinator = coordinator.clone();
            let holder = holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *holder.lock().expect("test server holder lock") = Some(server.clone());
                server
            }
        })
        .finish();

        let server = holder
            .lock()
            .expect("test server holder lock")
            .clone()
            .expect("test server must be captured");
        server
    }

    fn create_custom_service() -> (
        LspService<BslLanguageServer>,
        tokio::task::JoinHandle<()>,
        BslLanguageServer,
    ) {
        let coordinator = Arc::new(SystemCoordinator::new());
        let holder: Arc<StdMutex<Option<BslLanguageServer>>> = Arc::new(StdMutex::new(None));
        let (service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            let holder = holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *holder.lock().expect("test server holder lock") = Some(server.clone());
                server
            }
        })
        .custom_method("bsl/buildIndex", BslLanguageServer::handle_build_index)
        .custom_method(
            "bsl/getIndexState",
            BslLanguageServer::handle_get_index_state,
        )
        .custom_method(
            "bsl/getSnapshotStatus",
            BslLanguageServer::handle_get_snapshot_status,
        )
        .finish();

        let drain_task =
            tokio::spawn(async move { while let Some(_request) = socket.next().await {} });
        let server = holder
            .lock()
            .expect("test server holder lock")
            .clone()
            .expect("test server must be captured");
        (service, drain_task, server)
    }

    async fn initialize_custom_service(service: &mut LspService<BslLanguageServer>) {
        let initialize = Request::build("initialize")
            .id(100)
            .params(
                serde_json::to_value(InitializeParams {
                    capabilities: ClientCapabilities::default(),
                    ..Default::default()
                })
                .expect("InitializeParams"),
            )
            .finish();
        let initialize_response = service
            .ready()
            .await
            .expect("service ready")
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(
            initialize_response.is_some(),
            "initialize should return a response"
        );

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .expect("service ready")
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );
    }

    #[tokio::test]
    async fn build_index_attaches_when_startup_operation_is_running() {
        let server = create_test_server();
        let startup_operation_id = match server
            .begin_full_index_operation(FullIndexOperationKind::Startup, "startup")
            .await
        {
            BeginFullIndexOutcome::Started { operation_id } => operation_id,
            BeginFullIndexOutcome::AlreadyRunning { .. } => {
                panic!("startup operation unexpectedly already running")
            }
        };

        let response = server
            .handle_build_index(BuildIndexParams {
                workspace_path: String::new(),
            })
            .await
            .expect("build index response");

        assert!(response.success, "attached response must be successful");
        assert_eq!(response.types_count, 0);
        assert!(
            response.message.contains("already running (attached)"),
            "unexpected message: {}",
            response.message
        );

        let state = server.current_index_state().await;
        assert_eq!(state.state, "running");
        assert_eq!(state.active_operation.as_deref(), Some("startup"));
        assert_eq!(
            state.operation_id.as_deref(),
            Some(startup_operation_id.as_str())
        );

        server
            .finish_full_index_operation_failed(&startup_operation_id, "cleanup")
            .await;
    }

    #[tokio::test]
    async fn get_index_state_reports_ready_after_successful_finish() {
        let server = create_test_server();
        let operation_id = match server
            .begin_full_index_operation(FullIndexOperationKind::BuildIndex, "build")
            .await
        {
            BeginFullIndexOutcome::Started { operation_id } => operation_id,
            BeginFullIndexOutcome::AlreadyRunning { .. } => {
                panic!("operation unexpectedly already running")
            }
        };

        server
            .finish_full_index_operation_success(&operation_id, "done")
            .await;

        let state = server
            .handle_get_index_state(GetIndexStateParams::default())
            .await
            .expect("index state response");

        assert_eq!(state.version, 1);
        assert_eq!(state.state, "ready");
        assert!(state.ready);
        assert!(state.active_operation.is_none());
        assert!(state.operation_id.is_none());
        assert!(state.updated_at_ms > 0);
    }

    #[tokio::test]
    async fn watchdog_timeout_transitions_running_operation_to_failed() {
        let server = create_test_server();
        let operation_id = match server
            .begin_full_index_operation(FullIndexOperationKind::BuildIndex, "build")
            .await
        {
            BeginFullIndexOutcome::Started { operation_id } => operation_id,
            BeginFullIndexOutcome::AlreadyRunning { .. } => {
                panic!("operation unexpectedly already running")
            }
        };

        server.spawn_full_index_watchdog(operation_id, Duration::from_millis(10));
        tokio::time::sleep(Duration::from_millis(30)).await;

        let state = server.current_index_state().await;
        assert_eq!(state.state, "failed");
        assert!(!state.ready);
        assert!(state.active_operation.is_none());
        assert!(state.operation_id.is_none());
        assert!(
            state
                .message
                .as_deref()
                .is_some_and(|message| message.contains("timeout")),
            "timeout message must be present"
        );
    }

    #[tokio::test]
    async fn get_index_state_rpc_returns_nullable_fields_as_explicit_nulls() {
        let (mut service, drain_task, _server) = create_custom_service();
        initialize_custom_service(&mut service).await;
        let request = Request::build("bsl/getIndexState")
            .id(1)
            .params(serde_json::json!({}))
            .finish();

        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(request)
            .await
            .expect("bsl/getIndexState request")
            .expect("bsl/getIndexState response");

        let value = serde_json::to_value(response).expect("serialize response");
        let result = value.get("result").expect("result field");
        let object = result.as_object().expect("result object");

        for field in ["active_operation", "operation_id", "message"] {
            assert!(
                object.contains_key(field),
                "field `{field}` must be present in response"
            );
            assert!(
                object.get(field).is_some_and(|value| value.is_null()),
                "field `{field}` must be null for idle state"
            );
        }

        assert_eq!(
            object
                .get("version")
                .and_then(|value| value.as_u64())
                .expect("version"),
            1
        );
        assert_eq!(
            object
                .get("state")
                .and_then(|value| value.as_str())
                .expect("state"),
            "idle"
        );
        assert!(!object
            .get("ready")
            .and_then(|value| value.as_bool())
            .expect("ready"));

        drain_task.abort();
    }

    #[tokio::test]
    async fn build_index_rpc_attaches_to_running_startup_operation() {
        let (mut service, drain_task, server) = create_custom_service();
        initialize_custom_service(&mut service).await;
        let startup_operation_id = match server
            .begin_full_index_operation(FullIndexOperationKind::Startup, "startup")
            .await
        {
            BeginFullIndexOutcome::Started { operation_id } => operation_id,
            BeginFullIndexOutcome::AlreadyRunning { .. } => {
                panic!("startup operation unexpectedly already running")
            }
        };

        let request = Request::build("bsl/buildIndex")
            .id(2)
            .params(serde_json::json!({ "workspace_path": "/tmp/workspace" }))
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(request)
            .await
            .expect("bsl/buildIndex request")
            .expect("bsl/buildIndex response");
        let value = serde_json::to_value(response).expect("serialize response");
        let result = value.get("result").expect("result field");
        let object = result.as_object().expect("result object");
        assert!(object
            .get("success")
            .and_then(|value| value.as_bool())
            .expect("success"));
        let message = object
            .get("message")
            .and_then(|value| value.as_str())
            .expect("message");
        assert!(
            message.contains("already running (attached)"),
            "unexpected attached message: {}",
            message
        );

        let state = server.current_index_state().await;
        assert_eq!(state.state, "running");
        assert_eq!(state.active_operation.as_deref(), Some("startup"));
        assert_eq!(
            state.operation_id.as_deref(),
            Some(startup_operation_id.as_str())
        );

        server
            .finish_full_index_operation_failed(&startup_operation_id, "cleanup")
            .await;
        drain_task.abort();
    }

    fn sample_stage(
        name: &str,
        status: &str,
        started_offset_ms: u64,
        duration_ms: u64,
    ) -> crate::types::CompletionTimelineStageTrace {
        crate::types::CompletionTimelineStageTrace {
            name: name.to_string(),
            status: status.to_string(),
            started_offset_ms,
            duration_ms,
        }
    }

    fn sample_trace(
        trace_id: &str,
        request_id: Option<&str>,
        outcome: &str,
        total_duration_ms: u64,
        stages: Vec<crate::types::CompletionTimelineStageTrace>,
    ) -> crate::types::CompletionTimelineTrace {
        crate::types::CompletionTimelineTrace {
            trace_id: trace_id.to_string(),
            request_id: request_id.map(ToString::to_string),
            client_probe_id: None,
            uri: "file:///timeline.bsl".to_string(),
            trigger_mode: "trigger_character".to_string(),
            outcome: outcome.to_string(),
            started_at_ms: 1_700_000_000_000,
            total_duration_ms,
            dominant_stage: stages
                .iter()
                .max_by_key(|stage| stage.duration_ms)
                .map(|stage| stage.name.clone()),
            prepare_details: None,
            collect_breakdown: None,
            server_edge_details: None,
            turn_attribution: None,
            stages,
        }
    }

    fn sample_diagnostics_save_trace(
        trace_id: &str,
        requested_version: i32,
    ) -> crate::types::DiagnosticsSaveTimelineTrace {
        crate::types::DiagnosticsSaveTimelineTrace {
            trace_id: trace_id.to_string(),
            uri: "file:///timeline.bsl".to_string(),
            requested_version,
            save_cycle_sequence: requested_version as u64,
            diagnostics_generation: requested_version as u64,
            trigger: "did_save".to_string(),
            started_at_ms: 1_700_000_000_000 + requested_version as u64,
            first_publish: Some(crate::types::DiagnosticsSaveTimelinePublishTrace {
                profile: "save_fastlane".to_string(),
                publish_kind: "syntax_only".to_string(),
                outcome: "published".to_string(),
                elapsed_ms: 42,
                syntax_work_mode: Some("recomputed".to_string()),
                semantic_path: None,
                semantic_parse_source: None,
                semantic_ir_source: None,
                runtime_queue_wait_ms: None,
                apply_lag_ms: None,
                blocking_queue_wait_ms: Some(7),
                wait_for_file_version_ms: None,
                snapshot_with_deps_ms: None,
                syntax_diagnostics_query_ms: Some(9),
                semantic_diagnostics_query_ms: None,
                semantic_diagnostics_inputs_ms: None,
                semantic_diagnostics_parse_result_ms: None,
                semantic_diagnostics_ir_ms: None,
                semantic_diagnostics_collect_ms: None,
                semantic_diagnostics_flow_sensitive_ms: None,
                semantic_diagnostics_ir_ast_to_ir_convert_ms: None,
                semantic_diagnostics_ir_semantic_facts_materialize_ms: None,
                semantic_diagnostics_ir_semantic_facts_seed_module_context_ms: None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms: None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_prep_ms: None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_ms:
                    None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_snapshot_build_ms:
                    None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_body_infer_ms:
                    None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_function_count:
                    None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_scc_count: None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_iteration_count:
                    None,
                semantic_diagnostics_ir_semantic_facts_visit_statements_ms: None,
                semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms: None,
                semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms: None,
                semantic_diagnostics_ir_semantic_facts_visit_callable_body_count: None,
                semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_count: None,
                semantic_diagnostics_ir_semantic_facts_statement_count: None,
                semantic_diagnostics_ir_semantic_facts_local_function_summary_count: None,
                semantic_diagnostics_ir_semantic_facts_index_entry_count: None,
                publish_wait_ms: Some(1),
            }),
            followup_publish: None,
            save_fastlane_outcome: Some("published".to_string()),
            idle_heavy_outcome: Some("published".to_string()),
            followup_syntax_work_mode: Some("recomputed".to_string()),
            followup_semantic_path: Some("generic_pipeline".to_string()),
            followup_semantic_parse_source: Some("salsa".to_string()),
            followup_semantic_ir_source: Some("salsa".to_string()),
            followup_ready_snapshot_zero_probe: Some("not_ready".to_string()),
            followup_ready_snapshot_wait_probe: Some("timeout".to_string()),
            followup_ready_snapshot_task_state: Some("in_flight_same_version".to_string()),
            followup_ready_snapshot_timeout_phase: Some("parse_exec".to_string()),
            followup_ready_snapshot_timeout_phase_elapsed_ms: Some(3500),
            followup_ready_snapshot_timeout_leaf: Some("parser_tree_build".to_string()),
            followup_ready_snapshot_timeout_leaf_elapsed_ms: Some(3500),
            followup_ready_snapshot_parse_exec_ms: Some(3500),
            followup_ready_snapshot_parse_exec_timeout_subphase: Some(
                "core_parse_build".to_string(),
            ),
            followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms: Some(3500),
            followup_ready_snapshot_parse_exec_core_parse_build_ms: Some(3500),
            followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint: Some(
                "parser_tree_build".to_string(),
            ),
            followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms: Some(3500),
            followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms: Some(3500),
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms: None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint:
                None,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms:
                None,
            followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms: None,
            followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms: None,
            followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint: Some(
                "parser_tree_build".to_string(),
            ),
            followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms: Some(3500),
            followup_ready_snapshot_parse_exec_dominant_subphase: Some(
                "core_parse_build".to_string(),
            ),
            followup_ready_snapshot_parse_exec_dominant_subphase_ms: Some(3500),
            followup_ready_snapshot_post_parse_pre_materialization_ms: None,
            followup_ready_snapshot_ready_install_ms: None,
            followup_ready_snapshot_document_symbol_side_work_ms: None,
            followup_ready_snapshot_dominant_phase: Some("parse_exec".to_string()),
            followup_ready_snapshot_dominant_phase_ms: Some(3500),
            followup_ready_snapshot_relief_valve_outcome: Some("engaged_timed_out".to_string()),
            followup_ready_snapshot_relief_valve_budget_ms: Some(500),
            followup_ready_snapshot_relief_valve_elapsed_ms: Some(500),
            followup_shadow_state_available: Some(false),
            followup_wait_reason: None,
            followup_blocker_reason: None,
            followup_runtime_queue_wait_ms: None,
            followup_apply_lag_ms: None,
            followup_wait_for_file_version_ms: None,
            followup_snapshot_with_deps_ms: None,
            terminal_outcome: Some("published".to_string()),
        }
    }

    #[tokio::test]
    async fn completion_timeline_retention_evicts_oldest_first() {
        let server = create_test_server();
        for idx in 0..205_u64 {
            let trace = sample_trace(
                &format!("trace-{idx}"),
                Some(&format!("req-{idx}")),
                "ok_non_empty",
                10 + idx,
                vec![sample_stage("prepare_stateful", "completed", 0, 10 + idx)],
            );
            server.record_completion_timeline_trace(trace);
        }

        let response = server
            .handle_get_completion_timeline(crate::types::CompletionTimelineRequest::default())
            .await
            .expect("timeline response");
        assert_eq!(response.version, crate::server::COMPLETION_TIMELINE_VERSION);
        assert_eq!(response.traces.len(), 200);
        assert_eq!(
            response.traces.first().map(|trace| trace.trace_id.as_str()),
            Some("trace-5")
        );
        assert_eq!(
            response.traces.last().map(|trace| trace.trace_id.as_str()),
            Some("trace-204")
        );
    }

    #[tokio::test]
    async fn completion_timeline_can_filter_by_request_id() {
        let server = create_test_server();
        server.record_completion_timeline_trace(sample_trace(
            "trace-a",
            Some("req-a"),
            "ok_non_empty",
            30,
            vec![sample_stage("query_bundle", "completed", 0, 30)],
        ));
        server.record_completion_timeline_trace(sample_trace(
            "trace-b",
            Some("req-b"),
            "cancelled",
            5,
            vec![sample_stage("query_bundle", "cancelled", 0, 5)],
        ));

        let response = server
            .handle_get_completion_timeline(crate::types::CompletionTimelineRequest {
                limit: Some(10),
                request_id: Some("req-b".to_string()),
            })
            .await
            .expect("timeline response");

        assert_eq!(response.traces.len(), 1);
        let trace = &response.traces[0];
        assert_eq!(trace.trace_id, "trace-b");
        assert_eq!(trace.request_id.as_deref(), Some("req-b"));
        assert_eq!(trace.outcome, "cancelled");
        assert!(trace.server_edge_details.is_none());
        assert_eq!(trace.stages.len(), 1);
        assert_eq!(trace.stages[0].status, "cancelled");
    }

    #[tokio::test]
    async fn diagnostics_save_timeline_retention_evicts_oldest_first() {
        let server = create_test_server();
        {
            let mut store = server
                .diagnostics_save_timeline_store
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for idx in 0..205_i32 {
                store.traces.push_back(sample_diagnostics_save_trace(
                    &format!("save-trace-{idx}"),
                    idx,
                ));
            }
        }

        let response = server
            .handle_get_diagnostics_save_timeline(
                crate::types::DiagnosticsSaveTimelineRequest::default(),
            )
            .await
            .expect("timeline response");
        assert_eq!(
            response.version,
            crate::server::DIAGNOSTICS_SAVE_TIMELINE_VERSION
        );
        assert_eq!(response.traces.len(), 200);
        assert_eq!(
            response.traces.first().map(|trace| trace.trace_id.as_str()),
            Some("save-trace-5")
        );
        assert_eq!(
            response.traces.last().map(|trace| trace.trace_id.as_str()),
            Some("save-trace-204")
        );
    }

    #[tokio::test]
    async fn diagnostics_save_timeline_late_terminal_result_does_not_resurrect_duplicate_trace() {
        let server = create_test_server();
        let uri = Url::parse("file:///timeline-duplicate.bsl").expect("uri");
        let key = crate::server::DiagnosticsSaveTimelineCycleKey {
            file_id: bsl_analysis_v2::FileId(71),
            diagnostics_generation: 11,
            save_cycle_sequence: 4,
            requested_version: 9,
        };

        server.begin_diagnostics_save_timeline_cycle(&uri, key);
        server.record_diagnostics_save_timeline_profile_result(
            &uri,
            key,
            crate::server::DiagnosticsSaveTimelineProfileResult {
                profile: bsl_runtime::application::DiagnosticsProfile::SaveFastlane,
                disposition: bsl_runtime::application::DiagnosticsDisposition::Published,
                publish: Some(crate::types::DiagnosticsSaveTimelinePublishTrace {
                    profile: "save_fastlane".to_string(),
                    publish_kind: "syntax_only".to_string(),
                    outcome: "published".to_string(),
                    elapsed_ms: 33,
                    syntax_work_mode: Some("recomputed".to_string()),
                    semantic_path: None,
                    semantic_parse_source: None,
                    semantic_ir_source: None,
                    runtime_queue_wait_ms: None,
                    apply_lag_ms: None,
                    blocking_queue_wait_ms: Some(9),
                    wait_for_file_version_ms: None,
                    snapshot_with_deps_ms: None,
                    syntax_diagnostics_query_ms: Some(12),
                    semantic_diagnostics_query_ms: None,
                    semantic_diagnostics_inputs_ms: None,
                    semantic_diagnostics_parse_result_ms: None,
                    semantic_diagnostics_ir_ms: None,
                    semantic_diagnostics_collect_ms: None,
                    semantic_diagnostics_flow_sensitive_ms: None,
                    semantic_diagnostics_ir_ast_to_ir_convert_ms: None,
                    semantic_diagnostics_ir_semantic_facts_materialize_ms: None,
                    semantic_diagnostics_ir_semantic_facts_seed_module_context_ms: None,
                    semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms: None,
                    semantic_diagnostics_ir_semantic_facts_local_function_summaries_prep_ms:
                        None,
                    semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_ms:
                        None,
                    semantic_diagnostics_ir_semantic_facts_local_function_summaries_snapshot_build_ms:
                        None,
                    semantic_diagnostics_ir_semantic_facts_local_function_summaries_body_infer_ms:
                        None,
                    semantic_diagnostics_ir_semantic_facts_local_function_summaries_function_count:
                        None,
                    semantic_diagnostics_ir_semantic_facts_local_function_summaries_scc_count:
                        None,
                    semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_iteration_count:
                        None,
                    semantic_diagnostics_ir_semantic_facts_visit_statements_ms: None,
                    semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms: None,
                    semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms: None,
                    semantic_diagnostics_ir_semantic_facts_visit_callable_body_count: None,
                    semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_count: None,
                    semantic_diagnostics_ir_semantic_facts_statement_count: None,
                    semantic_diagnostics_ir_semantic_facts_local_function_summary_count: None,
                    semantic_diagnostics_ir_semantic_facts_index_entry_count: None,
                    publish_wait_ms: Some(1),
                }),
            },
        );
        server.record_diagnostics_save_timeline_profile_disposition(
            &uri,
            key,
            bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
            bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration,
        );
        server.record_diagnostics_save_timeline_profile_disposition(
            &uri,
            key,
            bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
            bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration,
        );

        let response = server
            .handle_get_diagnostics_save_timeline(crate::types::DiagnosticsSaveTimelineRequest {
                limit: Some(10),
            })
            .await
            .expect("timeline response");
        let traces = response
            .traces
            .into_iter()
            .filter(|trace| {
                trace.uri == uri.as_str()
                    && trace.requested_version == key.requested_version
                    && trace.save_cycle_sequence == key.save_cycle_sequence
                    && trace.diagnostics_generation == key.diagnostics_generation
            })
            .collect::<Vec<_>>();
        assert_eq!(
            traces.len(),
            1,
            "late terminal result must not resurrect duplicate diagnostics save trace"
        );
        assert_eq!(
            traces[0].terminal_outcome.as_deref(),
            Some("superseded_generation")
        );
        assert_eq!(traces[0].trace_id, "diagnostics-save-trace-1");
    }

    #[tokio::test]
    async fn diagnostics_save_timeline_begin_after_terminal_does_not_resurrect_duplicate_trace() {
        let server = create_test_server();
        let uri = Url::parse("file:///timeline-begin-after-terminal.bsl").expect("uri");
        let key = crate::server::DiagnosticsSaveTimelineCycleKey {
            file_id: bsl_analysis_v2::FileId(72),
            diagnostics_generation: 14,
            save_cycle_sequence: 5,
            requested_version: 11,
        };

        server.begin_diagnostics_save_timeline_cycle(&uri, key);
        server.record_diagnostics_save_timeline_profile_disposition(
            &uri,
            key,
            bsl_runtime::application::DiagnosticsProfile::SaveFastlane,
            bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration,
        );
        server.record_diagnostics_save_timeline_profile_disposition(
            &uri,
            key,
            bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
            bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration,
        );

        server.begin_diagnostics_save_timeline_cycle(&uri, key);

        let response = server
            .handle_get_diagnostics_save_timeline(crate::types::DiagnosticsSaveTimelineRequest {
                limit: Some(10),
            })
            .await
            .expect("timeline response");
        let traces = response
            .traces
            .into_iter()
            .filter(|trace| {
                trace.uri == uri.as_str()
                    && trace.requested_version == key.requested_version
                    && trace.save_cycle_sequence == key.save_cycle_sequence
                    && trace.diagnostics_generation == key.diagnostics_generation
            })
            .collect::<Vec<_>>();
        assert_eq!(
            traces.len(),
            1,
            "begin after terminal archive must not resurrect duplicate diagnostics save trace"
        );
        assert_eq!(
            traces[0].terminal_outcome.as_deref(),
            Some("superseded_generation")
        );
        assert_eq!(traces[0].trace_id, "diagnostics-save-trace-1");
    }

    #[tokio::test]
    async fn completion_head_observation_invalidation_tracks_latest_file_version() {
        let server = create_test_server();
        let file_id = bsl_analysis_v2::FileId(41);
        let deps_id = bsl_analysis_v2::DepsSnapshotId::from_hash("deps-head-version");
        let settings_id = Some(bsl_analysis_v2::SettingsId::from_hash(
            "settings-head-version",
        ));

        server
            .record_completion_head_hit_v2(file_id, 7, deps_id.clone(), settings_id.clone(), false)
            .await;
        server
            .record_completion_head_hit_v2(file_id, 8, deps_id.clone(), settings_id.clone(), false)
            .await;

        let observations = server.completion_head_serve_observations_v2.read().await;
        let latest = observations
            .get(&file_id)
            .expect("latest head observation after version bump");
        assert_eq!(latest.file_version, 8);
        drop(observations);

        assert!(
            !server
                .record_completion_head_to_exact_upgrade_if_pending_v2(
                    file_id,
                    7,
                    &deps_id,
                    settings_id.as_ref(),
                )
                .await,
            "stale exact upgrade for previous file_version must not match pending head observation"
        );

        let observations = server.completion_head_serve_observations_v2.read().await;
        let still_latest = observations
            .get(&file_id)
            .expect("latest head observation must stay pending after stale version mismatch");
        assert_eq!(still_latest.file_version, 8);
        drop(observations);

        assert!(
            server
                .record_completion_head_to_exact_upgrade_if_pending_v2(
                    file_id,
                    8,
                    &deps_id,
                    settings_id.as_ref(),
                )
                .await,
            "latest matching file_version must still upgrade pending head observation"
        );
        assert!(
            server
                .completion_head_serve_observations_v2
                .read()
                .await
                .get(&file_id)
                .is_none(),
            "matching upgrade must clear pending head observation"
        );
    }

    #[tokio::test]
    async fn completion_head_observation_invalidation_rejects_deps_mismatch() {
        let server = create_test_server();
        let file_id = bsl_analysis_v2::FileId(42);
        let deps_a = bsl_analysis_v2::DepsSnapshotId::from_hash("deps-head-a");
        let deps_b = bsl_analysis_v2::DepsSnapshotId::from_hash("deps-head-b");
        let settings_id = Some(bsl_analysis_v2::SettingsId::from_hash("settings-head-deps"));

        server
            .record_completion_head_hit_v2(file_id, 3, deps_a.clone(), settings_id.clone(), false)
            .await;

        assert!(
            !server
                .record_completion_head_to_exact_upgrade_if_pending_v2(
                    file_id,
                    3,
                    &deps_b,
                    settings_id.as_ref(),
                )
                .await,
            "exact upgrade with mismatched deps_id must not consume pending head observation"
        );

        let observations = server.completion_head_serve_observations_v2.read().await;
        let pending = observations
            .get(&file_id)
            .expect("head observation must stay pending after deps mismatch");
        assert_eq!(pending.file_version, 3);
        assert_eq!(
            pending.deps_id,
            bsl_analysis_v2::DepsSnapshotId::from_hash("deps-head-a")
        );
        drop(observations);

        assert!(
            server
                .record_completion_head_to_exact_upgrade_if_pending_v2(
                    file_id,
                    3,
                    &deps_a,
                    settings_id.as_ref(),
                )
                .await,
            "matching deps_id must still upgrade pending head observation"
        );
    }

    #[tokio::test]
    async fn completion_head_observation_invalidation_rejects_settings_mismatch() {
        let server = create_test_server();
        let file_id = bsl_analysis_v2::FileId(43);
        let deps_id = bsl_analysis_v2::DepsSnapshotId::from_hash("deps-head-settings");
        let settings_a = Some(bsl_analysis_v2::SettingsId::from_hash("settings-head-a"));
        let settings_b = Some(bsl_analysis_v2::SettingsId::from_hash("settings-head-b"));

        server
            .record_completion_head_hit_v2(file_id, 5, deps_id.clone(), settings_a.clone(), false)
            .await;

        assert!(
            !server
                .record_completion_head_to_exact_upgrade_if_pending_v2(
                    file_id,
                    5,
                    &deps_id,
                    settings_b.as_ref(),
                )
                .await,
            "exact upgrade with mismatched settings_id must not consume pending head observation"
        );

        let observations = server.completion_head_serve_observations_v2.read().await;
        let pending = observations
            .get(&file_id)
            .expect("head observation must stay pending after settings mismatch");
        assert_eq!(pending.file_version, 5);
        assert_eq!(pending.settings_id, settings_a);
        drop(observations);

        assert!(
            server
                .record_completion_head_to_exact_upgrade_if_pending_v2(
                    file_id,
                    5,
                    &deps_id,
                    settings_a.as_ref(),
                )
                .await,
            "matching settings_id must still upgrade pending head observation"
        );
    }

    #[test]
    fn current_context_generation_registry_rejects_stale_generation() {
        let latest_generations = CurrentContextLatestGenerationRegistry::default();
        let latest_key = CurrentContextSupersessionKey {
            editor_session_id: "file:///session-1.bsl::1".to_string(),
            request_generation: 5,
        };
        let latest_broker_key = CurrentContextParseBrokerKey {
            file_id: V2FileId(1),
            file_version: Some(5),
            text_hash: [5; 32],
        };
        assert!(
            register_current_context_generation(
                &latest_generations,
                &latest_key,
                &latest_broker_key
            ),
            "first generation for a session must be accepted"
        );

        let stale_key = CurrentContextSupersessionKey {
            editor_session_id: latest_key.editor_session_id.clone(),
            request_generation: 4,
        };
        let stale_broker_key = CurrentContextParseBrokerKey {
            file_id: V2FileId(1),
            file_version: Some(4),
            text_hash: [4; 32],
        };
        assert!(
            !register_current_context_generation(
                &latest_generations,
                &stale_key,
                &stale_broker_key
            ),
            "older generation for the same session must be rejected"
        );
        assert!(
            is_latest_current_context_generation(&latest_generations, &latest_key),
            "latest generation must remain unchanged after stale registration attempt"
        );
    }

    #[test]
    fn current_context_generation_registry_prunes_oldest_sessions_when_capacity_is_exceeded() {
        let latest_generations = CurrentContextLatestGenerationRegistry::default();

        for generation in 1..=(CURRENT_CONTEXT_LATEST_GENERATIONS_MAX_SESSIONS as u64 + 1) {
            let session_id = format!("file:///session-{generation}.bsl::1");
            assert!(
                register_current_context_generation(
                    &latest_generations,
                    &CurrentContextSupersessionKey {
                        editor_session_id: session_id.clone(),
                        request_generation: generation,
                    },
                    &CurrentContextParseBrokerKey {
                        file_id: V2FileId(generation as u32),
                        file_version: Some(generation as i32),
                        text_hash: [generation as u8; 32],
                    },
                ),
                "generation {generation} must register successfully"
            );
        }

        let latest_generations = latest_generations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert_eq!(
            latest_generations.len(),
            CURRENT_CONTEXT_LATEST_GENERATIONS_MAX_SESSIONS,
            "registry must stay bounded after capacity overflow"
        );
        assert!(
            !latest_generations.contains_key("file:///session-1.bsl::1"),
            "oldest session must be evicted first once registry exceeds capacity"
        );
        assert_eq!(
            latest_generations
                .get("file:///session-257.bsl::1")
                .map(|state| state.request_generation),
            Some(257),
            "newest session must remain tracked after opportunistic pruning"
        );
    }

    #[test]
    fn current_context_generation_registry_allows_equivalent_newer_work_to_reuse_inflight_parse() {
        let latest_generations = CurrentContextLatestGenerationRegistry::default();
        let session_id = "file:///session-1.bsl::1".to_string();
        let older_key = CurrentContextSupersessionKey {
            editor_session_id: session_id.clone(),
            request_generation: 1,
        };
        let equivalent_broker_key = CurrentContextParseBrokerKey {
            file_id: V2FileId(1),
            file_version: Some(2),
            text_hash: [7; 32],
        };
        assert!(register_current_context_generation(
            &latest_generations,
            &older_key,
            &equivalent_broker_key,
        ));
        assert!(register_current_context_generation(
            &latest_generations,
            &CurrentContextSupersessionKey {
                editor_session_id: session_id.clone(),
                request_generation: 2,
            },
            &equivalent_broker_key,
        ));
        assert!(
            current_context_generation_allows_equivalent_parse_reuse(
                &latest_generations,
                &older_key,
                &equivalent_broker_key,
            ),
            "same-key newer generation must keep the older leader parse reusable"
        );
        let non_equivalent_broker_key = CurrentContextParseBrokerKey {
            file_id: V2FileId(1),
            file_version: Some(3),
            text_hash: [9; 32],
        };
        assert!(register_current_context_generation(
            &latest_generations,
            &CurrentContextSupersessionKey {
                editor_session_id: session_id,
                request_generation: 3,
            },
            &non_equivalent_broker_key,
        ));
        assert!(
            !current_context_generation_allows_equivalent_parse_reuse(
                &latest_generations,
                &older_key,
                &equivalent_broker_key,
            ),
            "non-equivalent newer generation must supersede obsolete in-flight parse work"
        );
    }
}
