use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use tower::Service;
use tower_lsp::jsonrpc::{Id, Request};
use tower_lsp::lsp_types::{CancelParams, CompletionParams, NumberOrString, Position, Url};

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

fn cancel_request_hook_cell() -> &'static Mutex<Option<CancelRequestHook>> {
    static CELL: std::sync::OnceLock<Mutex<Option<CancelRequestHook>>> = std::sync::OnceLock::new();
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
    cancelled_before_take: bool,
    jsonrpc_dispatch_received_at_ms: Option<u64>,
    request_received_at_ms: Option<u64>,
    service_future_created_at_ms: Option<u64>,
    service_future_first_poll_entered_at_ms: Option<u64>,
    service_future_first_poll_outcome: Option<String>,
    service_future_first_wake_scheduled_at_ms: Option<u64>,
    service_scope_entered_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingCompletionRequestContext {
    pub(crate) request_id: String,
    pub(crate) cancelled_before_take: bool,
    pub(crate) jsonrpc_dispatch_received_at_ms: Option<u64>,
    pub(crate) request_received_at_ms: Option<u64>,
    pub(crate) service_future_created_at_ms: Option<u64>,
    pub(crate) service_future_first_poll_entered_at_ms: Option<u64>,
    pub(crate) service_future_first_poll_outcome: Option<String>,
    pub(crate) service_future_first_wake_scheduled_at_ms: Option<u64>,
    pub(crate) service_scope_entered_at_ms: Option<u64>,
}

#[derive(Debug, Default, Clone)]
struct ServiceFuturePollObservationSnapshot {
    first_poll_entered_at_ms: Option<u64>,
    first_poll_outcome: Option<String>,
    first_wake_scheduled_at_ms: Option<u64>,
}

#[derive(Debug, Clone)]
struct ServiceFuturePollObservationState {
    request_id: Option<String>,
    snapshot: Arc<Mutex<ServiceFuturePollObservationSnapshot>>,
}

impl ServiceFuturePollObservationState {
    fn new(request_id: Option<String>) -> Self {
        Self {
            request_id,
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

    fn record_first_poll_entered_at_ms(&self, first_poll_entered_at_ms: u64) {
        let should_record_pending = {
            let mut snapshot = self
                .snapshot
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if snapshot.first_poll_entered_at_ms.is_none() {
                snapshot.first_poll_entered_at_ms = Some(first_poll_entered_at_ms);
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
}

impl<F> InstrumentedServiceFuture<F> {
    fn new(inner: F, observation: ServiceFuturePollObservationState) -> Self {
        Self {
            inner: Box::pin(inner),
            first_poll_observed: false,
            observation,
        }
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
            return this.inner.as_mut().poll(cx);
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

fn completion_request_key(params: &CompletionParams) -> CompletionRequestKey {
    let text_document_position = &params.text_document_position;
    CompletionRequestKey {
        uri: text_document_position.text_document.uri.to_string(),
        line: text_document_position.position.line,
        character: text_document_position.position.character,
    }
}

fn completion_request_key_from_request(request: &Request) -> Option<CompletionRequestKey> {
    if request.method() != "textDocument/completion" {
        return None;
    }
    let params = request.params()?.clone();
    let completion_params = serde_json::from_value::<CompletionParams>(params).ok()?;
    Some(completion_request_key(&completion_params))
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
            entry.cancelled_before_take = false;
            entry.request_received_at_ms = request_received_at_ms;
        }
    } else {
        pending.by_request_id.insert(
            request_id.clone(),
            PendingCompletionRequestEntry {
                key: key.clone(),
                cancelled_before_take: false,
                jsonrpc_dispatch_received_at_ms: None,
                request_received_at_ms,
                service_future_created_at_ms: None,
                service_future_first_poll_entered_at_ms: None,
                service_future_first_poll_outcome: None,
                service_future_first_wake_scheduled_at_ms: None,
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
            entry.cancelled_before_take = false;
            entry.jsonrpc_dispatch_received_at_ms = jsonrpc_dispatch_received_at_ms;
        }
    } else {
        pending.by_request_id.insert(
            request_id.clone(),
            PendingCompletionRequestEntry {
                key: key.clone(),
                cancelled_before_take: false,
                jsonrpc_dispatch_received_at_ms,
                request_received_at_ms: None,
                service_future_created_at_ms: None,
                service_future_first_poll_entered_at_ms: None,
                service_future_first_poll_outcome: None,
                service_future_first_wake_scheduled_at_ms: None,
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
            cancelled_before_take: false,
            jsonrpc_dispatch_received_at_ms: None,
            request_received_at_ms: None,
            service_future_created_at_ms: None,
            service_future_first_poll_entered_at_ms: None,
            service_future_first_poll_outcome: None,
            service_future_first_wake_scheduled_at_ms: None,
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
        cancelled_before_take: entry.cancelled_before_take,
        jsonrpc_dispatch_received_at_ms: entry.jsonrpc_dispatch_received_at_ms,
        request_received_at_ms: entry.request_received_at_ms,
        service_future_created_at_ms: entry.service_future_created_at_ms,
        service_future_first_poll_entered_at_ms: entry.service_future_first_poll_entered_at_ms,
        service_future_first_poll_outcome: entry.service_future_first_poll_outcome,
        service_future_first_wake_scheduled_at_ms: entry.service_future_first_wake_scheduled_at_ms,
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
                cancelled_before_take: entry.cancelled_before_take,
                jsonrpc_dispatch_received_at_ms: entry.jsonrpc_dispatch_received_at_ms,
                request_received_at_ms: entry.request_received_at_ms,
                service_future_created_at_ms: entry.service_future_created_at_ms,
                service_future_first_poll_entered_at_ms: entry
                    .service_future_first_poll_entered_at_ms,
                service_future_first_poll_outcome: entry.service_future_first_poll_outcome,
                service_future_first_wake_scheduled_at_ms: entry
                    .service_future_first_wake_scheduled_at_ms,
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

pub(crate) fn set_cancel_request_hook(hook: Option<CancelRequestHook>) {
    let mut slot = cancel_request_hook_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = hook;
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
            record_pending_completion_jsonrpc_dispatch_received_at_ms(
                &request,
                &request_id,
                Some(super::unix_timestamp_ms()),
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
        let service_future_poll_observation =
            ServiceFuturePollObservationState::new(request_id.clone());
        let future = InstrumentedServiceFuture::new(
            self.inner.call(request),
            service_future_poll_observation.clone(),
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
