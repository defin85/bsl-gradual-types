use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use tower::Service;
use tower_lsp::jsonrpc::{Id, Request};
use tower_lsp::lsp_types::{
    CancelParams, CompletionParams, CompletionTriggerKind, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    NumberOrString, Position, Url,
};

tokio::task_local! {
    static LSP_REQUEST_ID: Option<String>;
}

tokio::task_local! {
    static LSP_REQUEST_RECEIVED_AT_MS: Option<u64>;
}

tokio::task_local! {
    static LSP_REQUEST_JSONRPC_DISPATCH_RECEIVED_AT_MS: Option<u64>;
}

tokio::task_local! {
    static LSP_REQUEST_SERVICE_FUTURE_CREATED_AT_MS: Option<u64>;
}

tokio::task_local! {
    static LSP_REQUEST_SERVICE_SCOPE_ENTERED_AT_MS: Option<u64>;
}

tokio::task_local! {
    static LSP_REQUEST_SERVICE_FUTURE_POLL_OBSERVATION: Option<ServiceFuturePollObservationState>;
}

type CancelRequestHook = Arc<dyn Fn(String) + Send + Sync + 'static>;
type PreDispatchCompletionTerminalHook =
    Arc<dyn Fn(PreDispatchCompletionTerminalTraceInput) + Send + Sync + 'static>;
type CompletionResponseEgressHook =
    Arc<dyn Fn(CompletionResponseEgressTracePatch) + Send + Sync + 'static>;

fn cancel_request_hook_cell() -> &'static Mutex<Option<CancelRequestHook>> {
    static CELL: std::sync::OnceLock<Mutex<Option<CancelRequestHook>>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| Mutex::new(None))
}

fn pre_dispatch_completion_terminal_hook_cell()
-> &'static Mutex<Option<PreDispatchCompletionTerminalHook>> {
    static CELL: std::sync::OnceLock<Mutex<Option<PreDispatchCompletionTerminalHook>>> =
        std::sync::OnceLock::new();
    CELL.get_or_init(|| Mutex::new(None))
}

fn completion_response_egress_hook_cell() -> &'static Mutex<Option<CompletionResponseEgressHook>> {
    static CELL: std::sync::OnceLock<Mutex<Option<CompletionResponseEgressHook>>> =
        std::sync::OnceLock::new();
    CELL.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CompletionRequestKey {
    uri: String,
    line: u32,
    character: u32,
}

#[derive(Debug, Default)]
struct PendingCompletionRequestIds {
    by_key: HashMap<CompletionRequestKey, VecDeque<String>>,
    by_request_id: HashMap<String, PendingCompletionRequestEntry>,
}

#[derive(Debug)]
struct PendingCompletionRequestEntry {
    key: CompletionRequestKey,
    uri: String,
    trigger_mode: String,
    cancelled_before_take: bool,
    client_probe_id: Option<String>,
    adapter_read_at_ms: Option<u64>,
    jsonrpc_dispatch_received_at_ms: Option<u64>,
    request_received_at_ms: Option<u64>,
    transport_slot_released_at_ms: Option<u64>,
    service_future_created_at_ms: Option<u64>,
    service_future_first_poll_entered_at_ms: Option<u64>,
    service_future_first_poll_outcome: Option<String>,
    service_future_first_wake_scheduled_at_ms: Option<u64>,
    first_poll_contention_attribution:
        Option<crate::types::CompletionTimelineFirstPollContentionAttributionTrace>,
    first_poll_contention_contenders:
        Option<Vec<crate::types::CompletionTimelineFirstPollContentionContenderTrace>>,
    service_scope_entered_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingCompletionRequestContext {
    pub(crate) request_id: String,
    pub(crate) uri: String,
    pub(crate) trigger_mode: String,
    pub(crate) cancelled_before_take: bool,
    pub(crate) client_probe_id: Option<String>,
    pub(crate) adapter_read_at_ms: Option<u64>,
    pub(crate) jsonrpc_dispatch_received_at_ms: Option<u64>,
    pub(crate) request_received_at_ms: Option<u64>,
    pub(crate) transport_slot_released_at_ms: Option<u64>,
    pub(crate) service_future_created_at_ms: Option<u64>,
    pub(crate) service_future_first_poll_entered_at_ms: Option<u64>,
    pub(crate) service_future_first_poll_outcome: Option<String>,
    pub(crate) service_future_first_wake_scheduled_at_ms: Option<u64>,
    pub(crate) first_poll_contention_attribution:
        Option<crate::types::CompletionTimelineFirstPollContentionAttributionTrace>,
    pub(crate) first_poll_contention_contenders:
        Option<Vec<crate::types::CompletionTimelineFirstPollContentionContenderTrace>>,
    pub(crate) service_scope_entered_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreDispatchCompletionTerminalTraceInput {
    pub(crate) request_id: String,
    pub(crate) uri: String,
    pub(crate) trigger_mode: String,
    pub(crate) client_probe_id: Option<String>,
    pub(crate) adapter_read_at_ms: Option<u64>,
    pub(crate) resolved_at_ms: u64,
    pub(crate) outcome: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionResponseEgressTracePatch {
    pub(crate) request_id: String,
    pub(crate) response_output_handoff_started_at_ms: u64,
    pub(crate) response_output_handoff_enqueued_at_ms: u64,
    pub(crate) response_output_enqueue_completed_at_ms: u64,
    pub(crate) response_output_encode_started_at_ms: u64,
    pub(crate) response_output_write_started_at_ms: u64,
    pub(crate) response_output_encode_completed_at_ms: u64,
    pub(crate) response_flush_completed_at_ms: u64,
}

#[derive(Debug, Default, Clone)]
struct ServiceFuturePollObservationSnapshot {
    first_poll_entered_at_ms: Option<u64>,
    first_poll_outcome: Option<String>,
    first_wake_scheduled_at_ms: Option<u64>,
    first_poll_contention_attribution:
        Option<crate::types::CompletionTimelineFirstPollContentionAttributionTrace>,
    first_poll_contention_contenders:
        Option<Vec<crate::types::CompletionTimelineFirstPollContentionContenderTrace>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum InflightRequestClass {
    DocumentSync,
    Completion,
    OtherRequest,
    OtherNotification,
}

impl InflightRequestClass {
    fn as_contract_str(self) -> &'static str {
        match self {
            Self::DocumentSync => "document_sync",
            Self::Completion => "completion",
            Self::OtherRequest => "other_request",
            Self::OtherNotification => "other_notification",
        }
    }
}

#[derive(Debug, Clone)]
struct InflightRequestMetadata {
    entry_id: u64,
    class: InflightRequestClass,
    uri: Option<String>,
}

#[derive(Debug, Clone)]
struct InflightRequestEntry {
    class: InflightRequestClass,
    method: String,
    command: Option<String>,
    phase: Option<String>,
    uri: Option<String>,
    started_at_ms: u64,
}

#[derive(Debug, Clone)]
struct FirstPollContentionSnapshot {
    attribution: crate::types::CompletionTimelineFirstPollContentionAttributionTrace,
    contenders: Option<Vec<crate::types::CompletionTimelineFirstPollContentionContenderTrace>>,
}

#[derive(Debug, Default)]
struct InflightRequestRegistry {
    next_entry_id: u64,
    by_entry_id: HashMap<u64, InflightRequestEntry>,
}

#[derive(Debug, Clone)]
struct ServiceFuturePollObservationState {
    request_id: Option<String>,
    inflight_request: Option<InflightRequestMetadata>,
    snapshot: Arc<Mutex<ServiceFuturePollObservationSnapshot>>,
}

impl ServiceFuturePollObservationState {
    fn new(request_id: Option<String>, inflight_request: Option<InflightRequestMetadata>) -> Self {
        Self {
            request_id,
            inflight_request,
            snapshot: Arc::new(Mutex::new(ServiceFuturePollObservationSnapshot::default())),
        }
    }

    fn first_poll_entered_at_ms(&self) -> Option<u64> {
        self.snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .first_poll_entered_at_ms
    }

    fn first_poll_outcome(&self) -> Option<String> {
        self.snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .first_poll_outcome
            .clone()
    }

    fn first_wake_scheduled_at_ms(&self) -> Option<u64> {
        self.snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .first_wake_scheduled_at_ms
    }

    fn first_poll_contention_attribution(
        &self,
    ) -> Option<crate::types::CompletionTimelineFirstPollContentionAttributionTrace> {
        self.snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .first_poll_contention_attribution
            .clone()
    }

    fn first_poll_contention_contenders(
        &self,
    ) -> Option<Vec<crate::types::CompletionTimelineFirstPollContentionContenderTrace>> {
        self.snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .first_poll_contention_contenders
            .clone()
    }

    fn set_inflight_phase(&self, phase: Option<&str>) {
        let Some(entry_id) = self
            .inflight_request
            .as_ref()
            .map(|request| request.entry_id)
        else {
            return;
        };
        set_inflight_request_phase(entry_id, phase);
    }

    fn record_first_poll_entered_at_ms(&self, first_poll_entered_at_ms: u64) {
        let first_poll_contention_snapshot = self.inflight_request.as_ref().and_then(|current| {
            first_poll_contention_snapshot_for_request(current, first_poll_entered_at_ms)
        });
        let first_poll_contention_attribution = first_poll_contention_snapshot
            .as_ref()
            .map(|snapshot| snapshot.attribution.clone());
        let first_poll_contention_contenders = first_poll_contention_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.contenders.clone());
        let should_record_pending = {
            let mut snapshot = self
                .snapshot
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if snapshot.first_poll_entered_at_ms.is_none() {
                snapshot.first_poll_entered_at_ms = Some(first_poll_entered_at_ms);
                snapshot.first_poll_contention_attribution =
                    first_poll_contention_attribution.clone();
                snapshot.first_poll_contention_contenders =
                    first_poll_contention_contenders.clone();
                true
            } else {
                false
            }
        };
        if should_record_pending {
            if let Some(request_id) = self.request_id.as_deref() {
                record_pending_completion_service_future_first_poll_entered_at_ms(
                    request_id,
                    first_poll_entered_at_ms,
                );
                if let Some(first_poll_contention_attribution) =
                    first_poll_contention_attribution.clone()
                {
                    record_pending_completion_first_poll_contention_attribution(
                        request_id,
                        first_poll_contention_attribution,
                    );
                }
                if let Some(first_poll_contention_contenders) =
                    first_poll_contention_contenders.clone()
                {
                    record_pending_completion_first_poll_contention_contenders(
                        request_id,
                        first_poll_contention_contenders,
                    );
                }
            }
        }
    }

    fn record_first_poll_outcome(&self, first_poll_outcome: &'static str) {
        let should_record_pending = {
            let mut snapshot = self
                .snapshot
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if snapshot.first_poll_outcome.is_none() {
                snapshot.first_poll_outcome = Some(first_poll_outcome.to_string());
                true
            } else {
                false
            }
        };
        if should_record_pending {
            if let Some(request_id) = self.request_id.as_deref() {
                record_pending_completion_service_future_first_poll_outcome(
                    request_id,
                    first_poll_outcome,
                );
            }
        }
    }

    fn record_first_wake_scheduled_at_ms(&self, first_wake_scheduled_at_ms: u64) {
        let should_record_pending = {
            let mut snapshot = self
                .snapshot
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if snapshot.first_wake_scheduled_at_ms.is_none() {
                snapshot.first_wake_scheduled_at_ms = Some(first_wake_scheduled_at_ms);
                true
            } else {
                false
            }
        };
        if should_record_pending {
            if let Some(request_id) = self.request_id.as_deref() {
                record_pending_completion_service_future_first_wake_scheduled_at_ms(
                    request_id,
                    first_wake_scheduled_at_ms,
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FirstWakeTrackingMode {
    Unknown,
    Armed,
    Disabled,
    Committed,
}

#[derive(Debug)]
struct FirstWakeTrackingState {
    mode: FirstWakeTrackingMode,
    candidate_at_ms: Option<u64>,
}

impl Default for FirstWakeTrackingState {
    fn default() -> Self {
        Self {
            mode: FirstWakeTrackingMode::Unknown,
            candidate_at_ms: None,
        }
    }
}

#[derive(Debug, Clone)]
struct FirstWakeTracker {
    observation: ServiceFuturePollObservationState,
    state: Arc<Mutex<FirstWakeTrackingState>>,
}

impl FirstWakeTracker {
    fn new(observation: ServiceFuturePollObservationState) -> Self {
        Self {
            observation,
            state: Arc::new(Mutex::new(FirstWakeTrackingState::default())),
        }
    }

    fn observe_wake(&self) {
        let wake_to_record = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match state.mode {
                FirstWakeTrackingMode::Unknown => {
                    state
                        .candidate_at_ms
                        .get_or_insert_with(super::unix_timestamp_ms);
                    None
                }
                FirstWakeTrackingMode::Armed => {
                    let wake_at_ms = state
                        .candidate_at_ms
                        .take()
                        .unwrap_or_else(super::unix_timestamp_ms);
                    state.mode = FirstWakeTrackingMode::Committed;
                    Some(wake_at_ms)
                }
                FirstWakeTrackingMode::Disabled | FirstWakeTrackingMode::Committed => None,
            }
        };
        if let Some(wake_at_ms) = wake_to_record {
            self.observation
                .record_first_wake_scheduled_at_ms(wake_at_ms);
        }
    }

    fn arm_after_first_pending_poll(&self) {
        let wake_to_record = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(wake_at_ms) = state.candidate_at_ms.take() {
                state.mode = FirstWakeTrackingMode::Committed;
                Some(wake_at_ms)
            } else {
                state.mode = FirstWakeTrackingMode::Armed;
                None
            }
        };
        if let Some(wake_at_ms) = wake_to_record {
            self.observation
                .record_first_wake_scheduled_at_ms(wake_at_ms);
        }
    }

    fn disable(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.mode = FirstWakeTrackingMode::Disabled;
        state.candidate_at_ms = None;
    }
}

#[derive(Debug)]
struct FirstWakeTrackingWaker {
    inner: Waker,
    tracker: FirstWakeTracker,
}

impl Wake for FirstWakeTrackingWaker {
    fn wake(self: Arc<Self>) {
        self.tracker.observe_wake();
        self.inner.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.tracker.observe_wake();
        self.inner.wake_by_ref();
    }
}

#[derive(Debug)]
struct InstrumentedServiceFuture<F> {
    inner: Pin<Box<F>>,
    first_poll_observed: bool,
    observation: ServiceFuturePollObservationState,
    inflight_request_entry_id: Option<u64>,
}

impl<F> InstrumentedServiceFuture<F> {
    fn new(
        inner: F,
        observation: ServiceFuturePollObservationState,
        inflight_request: Option<&InflightRequestMetadata>,
    ) -> Self {
        Self {
            inner: Box::pin(inner),
            first_poll_observed: false,
            observation,
            inflight_request_entry_id: inflight_request.map(|request| request.entry_id),
        }
    }

    fn clear_inflight_request_entry(&mut self) {
        if let Some(entry_id) = self.inflight_request_entry_id.take() {
            remove_inflight_request_entry(entry_id);
        }
    }
}

impl<F> Drop for InstrumentedServiceFuture<F> {
    fn drop(&mut self) {
        self.clear_inflight_request_entry();
    }
}

impl<F> Future for InstrumentedServiceFuture<F>
where
    F: Future,
{
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();
        if this.first_poll_observed {
            return match this.inner.as_mut().poll(cx) {
                Poll::Ready(output) => {
                    this.clear_inflight_request_entry();
                    Poll::Ready(output)
                }
                Poll::Pending => Poll::Pending,
            };
        }

        this.first_poll_observed = true;
        this.observation
            .record_first_poll_entered_at_ms(super::unix_timestamp_ms());
        let first_wake_tracker = FirstWakeTracker::new(this.observation.clone());
        let wrapped_waker = Waker::from(Arc::new(FirstWakeTrackingWaker {
            inner: cx.waker().clone(),
            tracker: first_wake_tracker.clone(),
        }));
        let mut wrapped_cx = Context::from_waker(&wrapped_waker);
        match this.inner.as_mut().poll(&mut wrapped_cx) {
            Poll::Ready(output) => {
                this.observation.record_first_poll_outcome("ready");
                first_wake_tracker.disable();
                this.clear_inflight_request_entry();
                Poll::Ready(output)
            }
            Poll::Pending => {
                this.observation.record_first_poll_outcome("pending");
                first_wake_tracker.arm_after_first_pending_poll();
                Poll::Pending
            }
        }
    }
}

fn pending_completion_request_ids_cell() -> &'static Mutex<PendingCompletionRequestIds> {
    static CELL: std::sync::OnceLock<Mutex<PendingCompletionRequestIds>> =
        std::sync::OnceLock::new();
    CELL.get_or_init(|| Mutex::new(PendingCompletionRequestIds::default()))
}

fn inflight_request_registry_cell() -> &'static Mutex<InflightRequestRegistry> {
    static CELL: std::sync::OnceLock<Mutex<InflightRequestRegistry>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| Mutex::new(InflightRequestRegistry::default()))
}

fn completion_request_key(params: &CompletionParams) -> CompletionRequestKey {
    let text_document_position = &params.text_document_position;
    CompletionRequestKey {
        uri: text_document_position.text_document.uri.to_string(),
        line: text_document_position.position.line,
        character: text_document_position.position.character,
    }
}

fn completion_trigger_mode_from_params(params: &CompletionParams) -> String {
    match params.context.as_ref().map(|context| context.trigger_kind) {
        Some(CompletionTriggerKind::TRIGGER_CHARACTER) => "trigger_character",
        Some(CompletionTriggerKind::INVOKED) => "invoked",
        Some(CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS) => "trigger_for_incomplete",
        Some(_) => "other",
        None => "none",
    }
    .to_string()
}

fn completion_request_params_from_request(request: &Request) -> Option<CompletionParams> {
    if request.method() != "textDocument/completion" {
        return None;
    }
    let params = request.params()?.clone();
    serde_json::from_value::<CompletionParams>(params).ok()
}

fn completion_request_key_from_request(request: &Request) -> Option<CompletionRequestKey> {
    let completion_params = completion_request_params_from_request(request)?;
    Some(completion_request_key(&completion_params))
}

fn completion_request_uri_and_trigger_mode_from_request(
    request: &Request,
) -> Option<(String, String)> {
    let completion_params = completion_request_params_from_request(request)?;
    Some((
        completion_params
            .text_document_position
            .text_document
            .uri
            .to_string(),
        completion_trigger_mode_from_params(&completion_params),
    ))
}

fn completion_probe_id_from_request(request: &Request) -> Option<String> {
    if request.method() != "textDocument/completion" {
        return None;
    }
    request
        .params()?
        .get("bslProbeId")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn inflight_request_class_for_request(request: &Request) -> InflightRequestClass {
    match request.method() {
        "textDocument/completion" => InflightRequestClass::Completion,
        "textDocument/didOpen"
        | "textDocument/didChange"
        | "textDocument/didSave"
        | "textDocument/didClose"
        | "textDocument/willSave"
        | "textDocument/willSaveWaitUntil" => InflightRequestClass::DocumentSync,
        _ if request.id().is_some() => InflightRequestClass::OtherRequest,
        _ => InflightRequestClass::OtherNotification,
    }
}

fn request_uri_from_value(value: &serde_json::Value) -> Option<String> {
    let object = value.as_object()?;

    if let Some(uri) = object
        .get("textDocument")
        .and_then(|value| value.as_object())
        .and_then(|text_document| text_document.get("uri"))
        .and_then(|value| value.as_str())
    {
        return Some(uri.to_string());
    }

    if let Some(uri) = object
        .get("textDocumentPosition")
        .and_then(|value| value.as_object())
        .and_then(|text_document_position| text_document_position.get("textDocument"))
        .and_then(|value| value.as_object())
        .and_then(|text_document| text_document.get("uri"))
        .and_then(|value| value.as_str())
    {
        return Some(uri.to_string());
    }

    object
        .get("textDocumentPositionParams")
        .and_then(|value| value.as_object())
        .and_then(|text_document_position| text_document_position.get("textDocument"))
        .and_then(|value| value.as_object())
        .and_then(|text_document| text_document.get("uri"))
        .and_then(|value| value.as_str())
        .map(|uri| uri.to_string())
}

fn request_uri_from_request(request: &Request) -> Option<String> {
    let params = request.params()?.clone();
    match request.method() {
        "textDocument/completion" => {
            serde_json::from_value::<CompletionParams>(params)
                .ok()
                .map(|completion| {
                    completion
                        .text_document_position
                        .text_document
                        .uri
                        .to_string()
                })
        }
        "textDocument/didOpen" => serde_json::from_value::<DidOpenTextDocumentParams>(params)
            .ok()
            .map(|did_open| did_open.text_document.uri.to_string()),
        "textDocument/didChange" => serde_json::from_value::<DidChangeTextDocumentParams>(params)
            .ok()
            .map(|did_change| did_change.text_document.uri.to_string()),
        "textDocument/didSave" => serde_json::from_value::<DidSaveTextDocumentParams>(params)
            .ok()
            .map(|did_save| did_save.text_document.uri.to_string()),
        "textDocument/didClose" => serde_json::from_value::<DidCloseTextDocumentParams>(params)
            .ok()
            .map(|did_close| did_close.text_document.uri.to_string()),
        _ => request_uri_from_value(&params),
    }
}

fn request_execute_command_name_from_request(request: &Request) -> Option<String> {
    if request.method() != "workspace/executeCommand" {
        return None;
    }
    request
        .params()?
        .get("command")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn unavailable_first_poll_contention_attribution(
    concurrency_level: u64,
) -> crate::types::CompletionTimelineFirstPollContentionAttributionTrace {
    crate::types::CompletionTimelineFirstPollContentionAttributionTrace {
        contender_class: "unavailable".to_string(),
        uri_scope: "unavailable".to_string(),
        inflight_count: 0,
        oldest_inflight_age_ms: None,
        concurrency_level,
    }
}

const MAX_FIRST_POLL_CONTENTION_CONTENDERS: usize = 5;

fn register_inflight_request(
    request: &Request,
    started_at_ms: u64,
) -> Option<InflightRequestMetadata> {
    let class = inflight_request_class_for_request(request);
    let method = request.method().to_string();
    let command = request_execute_command_name_from_request(request);
    let uri = request_uri_from_request(request);
    let mut registry = inflight_request_registry_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    registry.next_entry_id = registry.next_entry_id.saturating_add(1);
    let entry_id = registry.next_entry_id;
    registry.by_entry_id.insert(
        entry_id,
        InflightRequestEntry {
            class,
            method: method.clone(),
            command,
            phase: None,
            uri: uri.clone(),
            started_at_ms,
        },
    );
    Some(InflightRequestMetadata {
        entry_id,
        class,
        uri,
    })
}

fn remove_inflight_request_entry(entry_id: u64) {
    let mut registry = inflight_request_registry_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    registry.by_entry_id.remove(&entry_id);
}

fn set_inflight_request_phase(entry_id: u64, phase: Option<&str>) {
    let mut registry = inflight_request_registry_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(entry) = registry.by_entry_id.get_mut(&entry_id) else {
        return;
    };
    entry.phase = phase.map(str::to_string);
}

#[cfg(test)]
fn clear_inflight_request_registry_for_testing() {
    let mut registry = inflight_request_registry_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    registry.by_entry_id.clear();
    registry.next_entry_id = 0;
}

fn first_poll_contention_snapshot_for_request(
    current: &InflightRequestMetadata,
    first_poll_entered_at_ms: u64,
) -> Option<FirstPollContentionSnapshot> {
    let concurrency_level = crate::DEFAULT_LSP_TRANSPORT_CONCURRENCY_LEVEL as u64;
    if current.class != InflightRequestClass::Completion {
        return Some(FirstPollContentionSnapshot {
            attribution: unavailable_first_poll_contention_attribution(concurrency_level),
            contenders: None,
        });
    }

    let registry = inflight_request_registry_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(current_entry) = registry.by_entry_id.get(&current.entry_id) else {
        return Some(FirstPollContentionSnapshot {
            attribution: unavailable_first_poll_contention_attribution(concurrency_level),
            contenders: None,
        });
    };

    let contenders: Vec<&InflightRequestEntry> = registry
        .by_entry_id
        .iter()
        .filter_map(|(entry_id, entry)| {
            if *entry_id == current.entry_id {
                None
            } else {
                Some(entry)
            }
        })
        .collect();

    if contenders.is_empty() {
        return Some(FirstPollContentionSnapshot {
            attribution: crate::types::CompletionTimelineFirstPollContentionAttributionTrace {
                contender_class: "none_visible".to_string(),
                uri_scope: "unavailable".to_string(),
                inflight_count: 0,
                oldest_inflight_age_ms: None,
                concurrency_level,
            },
            contenders: Some(Vec::new()),
        });
    }

    let class_set: HashSet<InflightRequestClass> =
        contenders.iter().map(|entry| entry.class).collect();
    let contender_class = if class_set.len() == 1 {
        class_set
            .iter()
            .next()
            .map(|class| class.as_contract_str().to_string())
            .unwrap_or_else(|| "none_visible".to_string())
    } else {
        "mixed".to_string()
    };

    let current_uri = current_entry.uri.as_deref().or(current.uri.as_deref());
    let mut saw_same_uri = false;
    let mut saw_other_uri = false;
    let mut saw_unknown_uri = current_uri.is_none();
    for contender in &contenders {
        match (current_uri, contender.uri.as_deref()) {
            (Some(current_uri), Some(contender_uri)) if contender_uri == current_uri => {
                saw_same_uri = true;
            }
            (Some(_), Some(_)) => saw_other_uri = true,
            _ => saw_unknown_uri = true,
        }
    }
    let uri_scope = if saw_same_uri && saw_other_uri {
        "mixed"
    } else if saw_unknown_uri {
        "unavailable"
    } else if saw_same_uri {
        "same_uri"
    } else if saw_other_uri {
        "other_uri"
    } else {
        "unavailable"
    };

    let oldest_inflight_age_ms = contenders
        .iter()
        .map(|entry| first_poll_entered_at_ms.saturating_sub(entry.started_at_ms))
        .max();

    let mut contender_entries = contenders
        .iter()
        .map(
            |entry| crate::types::CompletionTimelineFirstPollContentionContenderTrace {
                request_class: entry.class.as_contract_str().to_string(),
                method: entry.method.clone(),
                command: entry.command.clone(),
                phase: entry.phase.clone(),
                uri: entry.uri.clone(),
                age_ms: first_poll_entered_at_ms.saturating_sub(entry.started_at_ms),
            },
        )
        .collect::<Vec<_>>();
    contender_entries.sort_by(|left, right| {
        right
            .age_ms
            .cmp(&left.age_ms)
            .then_with(|| left.request_class.cmp(&right.request_class))
            .then_with(|| left.method.cmp(&right.method))
            .then_with(|| left.command.cmp(&right.command))
            .then_with(|| left.phase.cmp(&right.phase))
            .then_with(|| left.uri.cmp(&right.uri))
    });
    contender_entries.truncate(MAX_FIRST_POLL_CONTENTION_CONTENDERS);

    Some(FirstPollContentionSnapshot {
        attribution: crate::types::CompletionTimelineFirstPollContentionAttributionTrace {
            contender_class,
            uri_scope: uri_scope.to_string(),
            inflight_count: contenders.len() as u64,
            oldest_inflight_age_ms,
            concurrency_level,
        },
        contenders: Some(contender_entries),
    })
}

#[cfg(test)]
fn first_poll_contention_attribution_for_request(
    current: &InflightRequestMetadata,
    first_poll_entered_at_ms: u64,
) -> Option<crate::types::CompletionTimelineFirstPollContentionAttributionTrace> {
    first_poll_contention_snapshot_for_request(current, first_poll_entered_at_ms)
        .map(|snapshot| snapshot.attribution)
}

fn remove_request_id_from_key_queue(
    pending: &mut PendingCompletionRequestIds,
    key: &CompletionRequestKey,
    request_id: &str,
) {
    if let Some(queue) = pending.by_key.get_mut(key) {
        queue.retain(|queued| queued != request_id);
        if queue.is_empty() {
            pending.by_key.remove(key);
        }
    }
}

fn ensure_request_id_enqueued(
    pending: &mut PendingCompletionRequestIds,
    key: &CompletionRequestKey,
    request_id: &str,
) {
    let queue = pending.by_key.entry(key.clone()).or_default();
    if !queue.iter().any(|queued| queued == request_id) {
        queue.push_back(request_id.to_string());
    }
}

fn record_pending_completion_request_id(
    request: &Request,
    request_id: &str,
    request_received_at_ms: Option<u64>,
) {
    let Some(key) = completion_request_key_from_request(request) else {
        return;
    };
    let Some((uri, trigger_mode)) = completion_request_uri_and_trigger_mode_from_request(request)
    else {
        return;
    };
    let client_probe_id = completion_probe_id_from_request(request);
    let request_id = request_id.to_string();
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(old_key) = pending
        .by_request_id
        .get(&request_id)
        .map(|entry| entry.key.clone())
    {
        if old_key != key {
            remove_request_id_from_key_queue(&mut pending, &old_key, &request_id);
        }
        if let Some(entry) = pending.by_request_id.get_mut(&request_id) {
            entry.key = key.clone();
            entry.uri = uri.clone();
            entry.trigger_mode = trigger_mode.clone();
            entry.cancelled_before_take = false;
            entry.client_probe_id = client_probe_id.clone();
            entry.request_received_at_ms = request_received_at_ms;
        }
    } else {
        pending.by_request_id.insert(
            request_id.clone(),
            PendingCompletionRequestEntry {
                key: key.clone(),
                uri,
                trigger_mode,
                cancelled_before_take: false,
                client_probe_id,
                adapter_read_at_ms: None,
                jsonrpc_dispatch_received_at_ms: None,
                request_received_at_ms,
                transport_slot_released_at_ms: None,
                service_future_created_at_ms: None,
                service_future_first_poll_entered_at_ms: None,
                service_future_first_poll_outcome: None,
                service_future_first_wake_scheduled_at_ms: None,
                first_poll_contention_attribution: None,
                first_poll_contention_contenders: None,
                service_scope_entered_at_ms: None,
            },
        );
    }
    ensure_request_id_enqueued(&mut pending, &key, &request_id);
}

pub(crate) fn record_pending_completion_adapter_read_at_ms(
    request: &Request,
    request_id: &str,
    adapter_read_at_ms: Option<u64>,
) {
    let Some(key) = completion_request_key_from_request(request) else {
        return;
    };
    let Some((uri, trigger_mode)) = completion_request_uri_and_trigger_mode_from_request(request)
    else {
        return;
    };
    let client_probe_id = completion_probe_id_from_request(request);
    let request_id = request_id.to_string();
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(old_key) = pending
        .by_request_id
        .get(&request_id)
        .map(|entry| entry.key.clone())
    {
        if old_key != key {
            remove_request_id_from_key_queue(&mut pending, &old_key, &request_id);
        }
        if let Some(entry) = pending.by_request_id.get_mut(&request_id) {
            entry.key = key.clone();
            entry.uri = uri.clone();
            entry.trigger_mode = trigger_mode.clone();
            entry.cancelled_before_take = false;
            entry.client_probe_id = client_probe_id.clone();
            if entry.adapter_read_at_ms.is_none() {
                entry.adapter_read_at_ms = adapter_read_at_ms;
            }
        }
    } else {
        pending.by_request_id.insert(
            request_id.clone(),
            PendingCompletionRequestEntry {
                key: key.clone(),
                uri,
                trigger_mode,
                cancelled_before_take: false,
                client_probe_id,
                adapter_read_at_ms,
                jsonrpc_dispatch_received_at_ms: None,
                request_received_at_ms: None,
                transport_slot_released_at_ms: None,
                service_future_created_at_ms: None,
                service_future_first_poll_entered_at_ms: None,
                service_future_first_poll_outcome: None,
                service_future_first_wake_scheduled_at_ms: None,
                first_poll_contention_attribution: None,
                first_poll_contention_contenders: None,
                service_scope_entered_at_ms: None,
            },
        );
    }
    ensure_request_id_enqueued(&mut pending, &key, &request_id);
}

fn record_pending_completion_jsonrpc_dispatch_received_at_ms(
    request: &Request,
    request_id: &str,
    jsonrpc_dispatch_received_at_ms: Option<u64>,
) {
    let Some(key) = completion_request_key_from_request(request) else {
        return;
    };
    let Some((uri, trigger_mode)) = completion_request_uri_and_trigger_mode_from_request(request)
    else {
        return;
    };
    let client_probe_id = completion_probe_id_from_request(request);
    let request_id = request_id.to_string();
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(old_key) = pending
        .by_request_id
        .get(&request_id)
        .map(|entry| entry.key.clone())
    {
        if old_key != key {
            remove_request_id_from_key_queue(&mut pending, &old_key, &request_id);
        }
        if let Some(entry) = pending.by_request_id.get_mut(&request_id) {
            entry.key = key.clone();
            entry.uri = uri.clone();
            entry.trigger_mode = trigger_mode.clone();
            entry.cancelled_before_take = false;
            entry.client_probe_id = client_probe_id.clone();
            entry.jsonrpc_dispatch_received_at_ms = jsonrpc_dispatch_received_at_ms;
        }
    } else {
        pending.by_request_id.insert(
            request_id.clone(),
            PendingCompletionRequestEntry {
                key: key.clone(),
                uri,
                trigger_mode,
                cancelled_before_take: false,
                client_probe_id,
                adapter_read_at_ms: None,
                jsonrpc_dispatch_received_at_ms,
                request_received_at_ms: None,
                transport_slot_released_at_ms: None,
                service_future_created_at_ms: None,
                service_future_first_poll_entered_at_ms: None,
                service_future_first_poll_outcome: None,
                service_future_first_wake_scheduled_at_ms: None,
                first_poll_contention_attribution: None,
                first_poll_contention_contenders: None,
                service_scope_entered_at_ms: None,
            },
        );
    }
    ensure_request_id_enqueued(&mut pending, &key, &request_id);
}

fn mark_pending_completion_request_cancelled_before_take(request_id: &str) {
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(key) = pending
        .by_request_id
        .get(request_id)
        .map(|entry| entry.key.clone())
    else {
        return;
    };
    if let Some(entry) = pending.by_request_id.get_mut(request_id) {
        entry.cancelled_before_take = true;
    }
    remove_request_id_from_key_queue(&mut pending, &key, request_id);
}

pub(crate) fn record_pending_completion_transport_slot_released_at_ms(
    request_id: &str,
    transport_slot_released_at_ms: u64,
) {
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(entry) = pending.by_request_id.get_mut(request_id) {
        entry.transport_slot_released_at_ms = Some(transport_slot_released_at_ms);
    }
}

fn record_pending_completion_service_future_created_at_ms(
    request_id: &str,
    service_future_created_at_ms: u64,
) {
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(entry) = pending.by_request_id.get_mut(request_id) {
        entry.service_future_created_at_ms = Some(service_future_created_at_ms);
    }
}

fn record_pending_completion_service_future_first_poll_entered_at_ms(
    request_id: &str,
    service_future_first_poll_entered_at_ms: u64,
) {
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(entry) = pending.by_request_id.get_mut(request_id) {
        entry.service_future_first_poll_entered_at_ms =
            Some(service_future_first_poll_entered_at_ms);
    }
}

fn record_pending_completion_service_future_first_poll_outcome(
    request_id: &str,
    service_future_first_poll_outcome: &str,
) {
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(entry) = pending.by_request_id.get_mut(request_id) {
        entry.service_future_first_poll_outcome =
            Some(service_future_first_poll_outcome.to_string());
    }
}

fn record_pending_completion_service_future_first_wake_scheduled_at_ms(
    request_id: &str,
    service_future_first_wake_scheduled_at_ms: u64,
) {
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(entry) = pending.by_request_id.get_mut(request_id) {
        entry.service_future_first_wake_scheduled_at_ms =
            Some(service_future_first_wake_scheduled_at_ms);
    }
}

fn record_pending_completion_first_poll_contention_attribution(
    request_id: &str,
    first_poll_contention_attribution: crate::types::CompletionTimelineFirstPollContentionAttributionTrace,
) {
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(entry) = pending.by_request_id.get_mut(request_id) {
        entry.first_poll_contention_attribution = Some(first_poll_contention_attribution);
    }
}

fn record_pending_completion_first_poll_contention_contenders(
    request_id: &str,
    first_poll_contention_contenders: Vec<
        crate::types::CompletionTimelineFirstPollContentionContenderTrace,
    >,
) {
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(entry) = pending.by_request_id.get_mut(request_id) {
        entry.first_poll_contention_contenders = Some(first_poll_contention_contenders);
    }
}

fn record_pending_completion_service_scope_entered_at_ms(
    request_id: &str,
    service_scope_entered_at_ms: u64,
) {
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(entry) = pending.by_request_id.get_mut(request_id) {
        entry.service_scope_entered_at_ms = Some(service_scope_entered_at_ms);
    }
}

#[cfg(test)]
fn remove_pending_completion_request_id(request_id: &str) {
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(entry) = pending.by_request_id.remove(request_id) else {
        return;
    };
    remove_request_id_from_key_queue(&mut pending, &entry.key, request_id);
}

#[cfg(test)]
pub(crate) fn record_completion_request_id_for_testing(
    uri: &Url,
    position: Position,
    request_id: &str,
) {
    let key = CompletionRequestKey {
        uri: uri.to_string(),
        line: position.line,
        character: position.character,
    };
    let request_id = request_id.to_string();
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(old_entry) = pending.by_request_id.insert(
        request_id.clone(),
        PendingCompletionRequestEntry {
            key: key.clone(),
            uri: uri.to_string(),
            trigger_mode: "none".to_string(),
            cancelled_before_take: false,
            client_probe_id: None,
            adapter_read_at_ms: None,
            jsonrpc_dispatch_received_at_ms: None,
            request_received_at_ms: None,
            transport_slot_released_at_ms: None,
            service_future_created_at_ms: None,
            service_future_first_poll_entered_at_ms: None,
            service_future_first_poll_outcome: None,
            service_future_first_wake_scheduled_at_ms: None,
            first_poll_contention_attribution: None,
            first_poll_contention_contenders: None,
            service_scope_entered_at_ms: None,
        },
    ) {
        remove_request_id_from_key_queue(&mut pending, &old_entry.key, &request_id);
    }
    ensure_request_id_enqueued(&mut pending, &key, &request_id);
}

#[cfg(test)]
pub(crate) fn take_completion_request_id(uri: &Url, position: Position) -> Option<String> {
    take_completion_request_context(uri, position).map(|context| context.request_id)
}

pub(crate) fn take_completion_request_context_by_request_id(
    request_id: &str,
) -> Option<PendingCompletionRequestContext> {
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let entry = pending.by_request_id.remove(request_id)?;
    if let Some(queue) = pending.by_key.get_mut(&entry.key) {
        queue.retain(|queued| queued != request_id);
        if queue.is_empty() {
            pending.by_key.remove(&entry.key);
        }
    }
    Some(PendingCompletionRequestContext {
        request_id: request_id.to_string(),
        uri: entry.uri,
        trigger_mode: entry.trigger_mode,
        cancelled_before_take: entry.cancelled_before_take,
        client_probe_id: entry.client_probe_id,
        adapter_read_at_ms: entry.adapter_read_at_ms,
        jsonrpc_dispatch_received_at_ms: entry.jsonrpc_dispatch_received_at_ms,
        request_received_at_ms: entry.request_received_at_ms,
        transport_slot_released_at_ms: entry.transport_slot_released_at_ms,
        service_future_created_at_ms: entry.service_future_created_at_ms,
        service_future_first_poll_entered_at_ms: entry.service_future_first_poll_entered_at_ms,
        service_future_first_poll_outcome: entry.service_future_first_poll_outcome,
        service_future_first_wake_scheduled_at_ms: entry.service_future_first_wake_scheduled_at_ms,
        first_poll_contention_attribution: entry.first_poll_contention_attribution,
        first_poll_contention_contenders: entry.first_poll_contention_contenders,
        service_scope_entered_at_ms: entry.service_scope_entered_at_ms,
    })
}

pub(crate) fn take_completion_request_context(
    uri: &Url,
    position: Position,
) -> Option<PendingCompletionRequestContext> {
    let key = CompletionRequestKey {
        uri: uri.to_string(),
        line: position.line,
        character: position.character,
    };
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    loop {
        let request_id = {
            let queue = pending.by_key.get_mut(&key)?;
            queue.pop_front()
        };
        let Some(request_id) = request_id else {
            pending.by_key.remove(&key);
            return None;
        };
        if let Some(entry) = pending.by_request_id.remove(&request_id) {
            let empty = pending
                .by_key
                .get(&key)
                .is_some_and(|queue| queue.is_empty());
            if empty {
                pending.by_key.remove(&key);
            }
            if entry.cancelled_before_take {
                continue;
            }
            return Some(PendingCompletionRequestContext {
                request_id,
                uri: entry.uri,
                trigger_mode: entry.trigger_mode,
                cancelled_before_take: entry.cancelled_before_take,
                client_probe_id: entry.client_probe_id,
                adapter_read_at_ms: entry.adapter_read_at_ms,
                jsonrpc_dispatch_received_at_ms: entry.jsonrpc_dispatch_received_at_ms,
                request_received_at_ms: entry.request_received_at_ms,
                transport_slot_released_at_ms: entry.transport_slot_released_at_ms,
                service_future_created_at_ms: entry.service_future_created_at_ms,
                service_future_first_poll_entered_at_ms: entry
                    .service_future_first_poll_entered_at_ms,
                service_future_first_poll_outcome: entry.service_future_first_poll_outcome,
                service_future_first_wake_scheduled_at_ms: entry
                    .service_future_first_wake_scheduled_at_ms,
                first_poll_contention_attribution: entry.first_poll_contention_attribution,
                first_poll_contention_contenders: entry.first_poll_contention_contenders,
                service_scope_entered_at_ms: entry.service_scope_entered_at_ms,
            });
        }
    }
}

pub(crate) fn current_request_id() -> Option<String> {
    LSP_REQUEST_ID.try_with(Clone::clone).ok().flatten()
}

pub(crate) fn current_request_received_at_ms() -> Option<u64> {
    LSP_REQUEST_RECEIVED_AT_MS
        .try_with(Clone::clone)
        .ok()
        .flatten()
}

pub(crate) fn current_request_jsonrpc_dispatch_received_at_ms() -> Option<u64> {
    LSP_REQUEST_JSONRPC_DISPATCH_RECEIVED_AT_MS
        .try_with(Clone::clone)
        .ok()
        .flatten()
}

pub(crate) fn current_request_service_future_created_at_ms() -> Option<u64> {
    LSP_REQUEST_SERVICE_FUTURE_CREATED_AT_MS
        .try_with(Clone::clone)
        .ok()
        .flatten()
}

pub(crate) fn current_request_service_scope_entered_at_ms() -> Option<u64> {
    LSP_REQUEST_SERVICE_SCOPE_ENTERED_AT_MS
        .try_with(Clone::clone)
        .ok()
        .flatten()
}

pub(crate) fn current_request_service_future_first_poll_entered_at_ms() -> Option<u64> {
    LSP_REQUEST_SERVICE_FUTURE_POLL_OBSERVATION
        .try_with(|state| {
            state
                .as_ref()
                .and_then(|observation| observation.first_poll_entered_at_ms())
        })
        .ok()
        .flatten()
}

pub(crate) fn current_request_service_future_first_poll_outcome() -> Option<String> {
    LSP_REQUEST_SERVICE_FUTURE_POLL_OBSERVATION
        .try_with(|state| {
            state
                .as_ref()
                .and_then(|observation| observation.first_poll_outcome())
        })
        .ok()
        .flatten()
}

pub(crate) fn current_request_service_future_first_wake_scheduled_at_ms() -> Option<u64> {
    LSP_REQUEST_SERVICE_FUTURE_POLL_OBSERVATION
        .try_with(|state| {
            state
                .as_ref()
                .and_then(|observation| observation.first_wake_scheduled_at_ms())
        })
        .ok()
        .flatten()
}

pub(crate) fn current_request_service_future_first_poll_contention_attribution()
-> Option<crate::types::CompletionTimelineFirstPollContentionAttributionTrace> {
    LSP_REQUEST_SERVICE_FUTURE_POLL_OBSERVATION
        .try_with(|state| {
            state
                .as_ref()
                .and_then(|observation| observation.first_poll_contention_attribution())
        })
        .ok()
        .flatten()
}

pub(crate) fn current_request_service_future_first_poll_contention_contenders()
-> Option<Vec<crate::types::CompletionTimelineFirstPollContentionContenderTrace>> {
    LSP_REQUEST_SERVICE_FUTURE_POLL_OBSERVATION
        .try_with(|state| {
            state
                .as_ref()
                .and_then(|observation| observation.first_poll_contention_contenders())
        })
        .ok()
        .flatten()
}

pub(crate) fn set_current_request_inflight_phase(phase: &str) {
    let _ = LSP_REQUEST_SERVICE_FUTURE_POLL_OBSERVATION.try_with(|state| {
        if let Some(observation) = state.as_ref() {
            observation.set_inflight_phase(Some(phase));
        }
    });
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct TestRequestServerEdgeTrace {
    pub(crate) request_id: String,
    pub(crate) method: String,
    pub(crate) uri: String,
    pub(crate) server_edge_details: crate::types::CompletionTimelineServerEdgeDetailsTrace,
}

#[cfg(test)]
fn test_request_server_edge_traces_cell() -> &'static Mutex<VecDeque<TestRequestServerEdgeTrace>> {
    static CELL: std::sync::OnceLock<Mutex<VecDeque<TestRequestServerEdgeTrace>>> =
        std::sync::OnceLock::new();
    CELL.get_or_init(|| Mutex::new(VecDeque::new()))
}

#[cfg(test)]
pub(crate) fn record_request_server_edge_trace_for_testing(
    request_id: Option<&str>,
    method: &str,
    uri: &Url,
    server_edge_details: crate::types::CompletionTimelineServerEdgeDetailsTrace,
) {
    const MAX_STORED_TRACES: usize = 256;

    let Some(request_id) = request_id else {
        return;
    };

    let mut traces = test_request_server_edge_traces_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    traces.push_back(TestRequestServerEdgeTrace {
        request_id: request_id.to_string(),
        method: method.to_string(),
        uri: uri.to_string(),
        server_edge_details,
    });
    while traces.len() > MAX_STORED_TRACES {
        let _ = traces.pop_front();
    }
}

#[cfg(test)]
pub(crate) fn take_request_server_edge_trace_for_testing(
    request_id: &str,
) -> Option<TestRequestServerEdgeTrace> {
    let mut traces = test_request_server_edge_traces_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let position = traces
        .iter()
        .rposition(|trace| trace.request_id == request_id)?;
    traces.remove(position)
}

pub(crate) fn set_cancel_request_hook(hook: Option<CancelRequestHook>) {
    let mut slot = cancel_request_hook_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = hook;
}

pub(crate) fn set_pre_dispatch_completion_terminal_hook(
    hook: Option<PreDispatchCompletionTerminalHook>,
) {
    let mut slot = pre_dispatch_completion_terminal_hook_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = hook;
}

pub(crate) fn set_completion_response_egress_hook(hook: Option<CompletionResponseEgressHook>) {
    let mut slot = completion_response_egress_hook_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = hook;
}

pub(crate) fn notify_pre_dispatch_completion_terminal_outcome(
    context: PendingCompletionRequestContext,
    resolved_at_ms: u64,
    outcome: &'static str,
) {
    let hook = {
        let slot = pre_dispatch_completion_terminal_hook_cell()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        slot.clone()
    };
    let Some(hook) = hook else {
        return;
    };
    hook(PreDispatchCompletionTerminalTraceInput {
        request_id: context.request_id,
        uri: context.uri,
        trigger_mode: context.trigger_mode,
        client_probe_id: context.client_probe_id,
        adapter_read_at_ms: context.adapter_read_at_ms,
        resolved_at_ms,
        outcome: outcome.to_string(),
    });
}

pub(crate) fn notify_pre_dispatch_completion_cancelled(
    context: PendingCompletionRequestContext,
    cancelled_at_ms: u64,
) {
    notify_pre_dispatch_completion_terminal_outcome(context, cancelled_at_ms, "cancelled");
}

async fn with_request_context<F, T>(
    request_id: Option<String>,
    request_received_at_ms: Option<u64>,
    jsonrpc_dispatch_received_at_ms: Option<u64>,
    service_future_created_at_ms: Option<u64>,
    service_scope_entered_at_ms: Option<u64>,
    service_future_poll_observation: Option<ServiceFuturePollObservationState>,
    future: F,
) -> T
where
    F: Future<Output = T>,
{
    LSP_REQUEST_ID
        .scope(request_id, async move {
            LSP_REQUEST_RECEIVED_AT_MS
                .scope(request_received_at_ms, async move {
                    LSP_REQUEST_JSONRPC_DISPATCH_RECEIVED_AT_MS
                        .scope(jsonrpc_dispatch_received_at_ms, async move {
                            LSP_REQUEST_SERVICE_FUTURE_CREATED_AT_MS
                                .scope(service_future_created_at_ms, async move {
                                    LSP_REQUEST_SERVICE_SCOPE_ENTERED_AT_MS
                                        .scope(service_scope_entered_at_ms, async move {
                                            LSP_REQUEST_SERVICE_FUTURE_POLL_OBSERVATION
                                                .scope(service_future_poll_observation, future)
                                                .await
                                        })
                                        .await
                                })
                                .await
                        })
                        .await
                })
                .await
        })
        .await
}

fn pending_completion_jsonrpc_dispatch_received_at_ms(request_id: &str) -> Option<u64> {
    pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .by_request_id
        .get(request_id)
        .and_then(|entry| entry.jsonrpc_dispatch_received_at_ms)
}

fn request_id_from_jsonrpc_id(id: &Id) -> Option<String> {
    match id {
        Id::Number(value) => Some(value.to_string()),
        Id::String(value) => Some(value.clone()),
        Id::Null => None,
    }
}

pub(crate) fn notify_completion_response_egress(patch: CompletionResponseEgressTracePatch) {
    let hook = completion_response_egress_hook_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    if let Some(hook) = hook {
        hook(patch);
    }
}

fn request_id_from_request(request: &Request) -> Option<String> {
    request.id().and_then(request_id_from_jsonrpc_id)
}

fn cancelled_request_id_from_request(request: &Request) -> Option<String> {
    if request.method() != "$/cancelRequest" {
        return None;
    }
    let params = request.params()?.clone();
    let cancel_params: CancelParams = serde_json::from_value(params).ok()?;
    match cancel_params.id {
        NumberOrString::Number(value) => Some(value.to_string()),
        NumberOrString::String(value) => Some(value),
    }
}

fn notify_cancel_request_hook(request_id: String) {
    let hook = {
        let slot = cancel_request_hook_cell()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        slot.clone()
    };
    if let Some(hook) = hook {
        hook(request_id);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DispatchContextService<S> {
    inner: S,
}

impl<S> DispatchContextService<S> {
    pub(crate) fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S> Service<Request> for DispatchContextService<S>
where
    S: Service<Request> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        if let Some(request_id) = request_id_from_request(&request) {
            let dispatch_received_at_ms = super::unix_timestamp_ms();
            record_pending_completion_adapter_read_at_ms(
                &request,
                &request_id,
                Some(dispatch_received_at_ms),
            );
            record_pending_completion_jsonrpc_dispatch_received_at_ms(
                &request,
                &request_id,
                Some(dispatch_received_at_ms),
            );
        }
        self.inner.call(request)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RequestContextService<S> {
    inner: S,
}

impl<S> RequestContextService<S> {
    pub(crate) fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S> Service<Request> for RequestContextService<S>
where
    S: Service<Request> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        if let Some(request_id) = cancelled_request_id_from_request(&request) {
            mark_pending_completion_request_cancelled_before_take(&request_id);
            notify_cancel_request_hook(request_id);
        }
        let request_id = request_id_from_request(&request);
        let jsonrpc_dispatch_received_at_ms = request_id
            .as_deref()
            .and_then(pending_completion_jsonrpc_dispatch_received_at_ms);
        let request_received_at_ms = Some(super::unix_timestamp_ms());
        if let Some(request_id) = request_id.as_deref() {
            record_pending_completion_request_id(&request, request_id, request_received_at_ms);
        }
        let inflight_request = request_received_at_ms.and_then(|request_received_at_ms| {
            register_inflight_request(&request, request_received_at_ms)
        });
        let service_future_poll_observation =
            ServiceFuturePollObservationState::new(request_id.clone(), inflight_request.clone());
        let future = InstrumentedServiceFuture::new(
            self.inner.call(request),
            service_future_poll_observation.clone(),
            inflight_request.as_ref(),
        );
        let service_future_created_at_ms = Some(super::unix_timestamp_ms());
        if let (Some(request_id), Some(service_future_created_at_ms)) =
            (request_id.as_deref(), service_future_created_at_ms)
        {
            record_pending_completion_service_future_created_at_ms(
                request_id,
                service_future_created_at_ms,
            );
        }
        Box::pin(async move {
            let service_scope_entered_at_ms = Some(super::unix_timestamp_ms());
            if let (Some(request_id), Some(service_scope_entered_at_ms)) =
                (request_id.as_deref(), service_scope_entered_at_ms)
            {
                record_pending_completion_service_scope_entered_at_ms(
                    request_id,
                    service_scope_entered_at_ms,
                );
            }
            with_request_context(
                request_id,
                request_received_at_ms,
                jsonrpc_dispatch_received_at_ms,
                service_future_created_at_ms,
                service_scope_entered_at_ms,
                Some(service_future_poll_observation),
                future,
            )
            .await
        })
    }
}

#[cfg(test)]
#[path = "request_context/tests.rs"]
mod tests;
