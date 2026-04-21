use std::fmt::{self, Display, Formatter};
use std::io::Error as IoError;
use std::num::ParseIntError;
use std::str::Utf8Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures::channel::mpsc;
use futures::future::BoxFuture;
use futures::{future, pin_mut, stream, FutureExt, Sink, SinkExt, StreamExt, TryFutureExt};
use tokio::io::{
    AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
};
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tower::Service;
use tower_lsp::jsonrpc::{Error, ErrorCode, Id, Request, Response};
use tower_lsp::Loopback;
use tracing::{error, warn};

const MESSAGE_QUEUE_SIZE: usize = 100;
const CONTROL_QUEUE_SIZE: usize = 16;
const COMPLETION_METHOD: &str = "textDocument/completion";
const DID_OPEN_METHOD: &str = "textDocument/didOpen";
const DID_CHANGE_METHOD: &str = "textDocument/didChange";
const DID_SAVE_METHOD: &str = "textDocument/didSave";
const DID_CLOSE_METHOD: &str = "textDocument/didClose";
const CANCEL_REQUEST_METHOD: &str = "$/cancelRequest";
const SHUTDOWN_METHOD: &str = "shutdown";
const EXIT_METHOD: &str = "exit";
const GENERAL_BACKPRESSURE_ERROR_CODE: i64 = -32001;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
enum TransportMessage {
    Response(Response),
    Request(Request),
}

#[derive(Debug)]
enum TransportCodecError {
    Json(serde_json::Error),
    Io(IoError),
    InvalidContentLength(ParseIntError),
    InvalidContentType,
    MissingContentLength,
    UnexpectedEof,
    Utf8(Utf8Error),
}

impl Display for TransportCodecError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(err) => write!(f, "unable to parse JSON body: {err}"),
            Self::Io(err) => write!(f, "failed to process transport frame: {err}"),
            Self::InvalidContentLength(err) => {
                write!(f, "unable to parse content length: {err}")
            }
            Self::InvalidContentType => write!(f, "unable to parse content type"),
            Self::MissingContentLength => write!(f, "missing required `Content-Length` header"),
            Self::UnexpectedEof => write!(f, "unexpected EOF while reading transport message"),
            Self::Utf8(err) => write!(f, "request contains invalid UTF8: {err}"),
        }
    }
}

impl std::error::Error for TransportCodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(err) => Some(err),
            Self::Io(err) => Some(err),
            Self::InvalidContentLength(err) => Some(err),
            Self::Utf8(err) => Some(err),
            Self::InvalidContentType | Self::MissingContentLength | Self::UnexpectedEof => None,
        }
    }
}

impl From<IoError> for TransportCodecError {
    fn from(error: IoError) -> Self {
        Self::Io(error)
    }
}

impl From<ParseIntError> for TransportCodecError {
    fn from(error: ParseIntError) -> Self {
        Self::InvalidContentLength(error)
    }
}

impl From<Utf8Error> for TransportCodecError {
    fn from(error: Utf8Error) -> Self {
        Self::Utf8(error)
    }
}

struct CompletionHandoffTask {
    request_id: Option<String>,
    future: BoxFuture<'static, Option<Response>>,
}

impl CompletionHandoffTask {
    fn new(request_id: Option<String>, future: BoxFuture<'static, Option<Response>>) -> Self {
        Self { request_id, future }
    }

    async fn forward_response(self, mut responses_tx: mpsc::Sender<QueuedTransportMessage>) {
        let request_id = self.request_id;
        if let Some(response) = self.future.await {
            let response_output_handoff_started_at_ms = super::unix_timestamp_ms();
            let response_output_handoff_enqueued_at_ms = Arc::new(AtomicU64::new(0));
            if responses_tx
                .send(QueuedTransportMessage::completion_response(
                    response,
                    request_id.clone(),
                    response_output_handoff_started_at_ms,
                    response_output_handoff_enqueued_at_ms.clone(),
                ))
                .await
                .is_err()
            {
                match request_id.as_deref() {
                    Some(request_id) => error!(
                        "failed to forward deferred completion response for request {request_id}: transport closed"
                    ),
                    None => error!(
                        "failed to forward deferred completion response without request id: transport closed"
                    ),
                }
            } else {
                response_output_handoff_enqueued_at_ms
                    .store(super::unix_timestamp_ms(), Ordering::Relaxed);
            }
        }
    }
}

enum CompletionBarrierFirstPoll {
    Ready(Option<Response>),
    Pending,
}

#[derive(Debug, Clone)]
struct CompletionBarrierOwnerMetadata {
    generation: u64,
    method: String,
    uri: Option<String>,
    version: Option<i32>,
    first_poll_exec_ms: u64,
    first_poll_outcome: String,
}

#[derive(Debug, Clone)]
struct CompletionBarrierSnapshot {
    generation: u64,
    owner_method: String,
    owner_uri: Option<String>,
    owner_version: Option<i32>,
    doc_sync_first_poll_exec_ms: u64,
    doc_sync_first_poll_outcome: String,
}

#[derive(Debug, Clone, Copy)]
struct CompletionBarrierTicket {
    entry_id: u64,
}

#[derive(Debug, Default)]
struct CompletionBarrierGateState {
    next_entry_id: u64,
    next_generation: u64,
    active: std::collections::VecDeque<(CompletionBarrierTicket, CompletionBarrierOwnerMetadata)>,
}

#[derive(Debug, Clone)]
struct CompletionBarrierGate {
    state: Arc<Mutex<CompletionBarrierGateState>>,
    released: Arc<Notify>,
}

impl CompletionBarrierGate {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(CompletionBarrierGateState::default())),
            released: Arc::new(Notify::new()),
        }
    }

    fn begin(&self, mut owner: CompletionBarrierOwnerMetadata) -> CompletionBarrierTicket {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.next_entry_id = state.next_entry_id.saturating_add(1);
        state.next_generation = state.next_generation.saturating_add(1);
        owner.generation = state.next_generation;
        let ticket = CompletionBarrierTicket {
            entry_id: state.next_entry_id,
        };
        state.active.push_back((ticket, owner));
        ticket
    }

    fn release(&self, ticket: CompletionBarrierTicket) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let position = state
            .active
            .iter()
            .position(|(active_ticket, _)| active_ticket.entry_id == ticket.entry_id);
        debug_assert!(
            position.is_some(),
            "completion barrier inflight count underflow"
        );
        if let Some(position) = position {
            let _ = state.active.remove(position);
        }
        self.released.notify_waiters();
    }

    fn is_active(&self) -> bool {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        !state.active.is_empty()
    }

    fn snapshot(&self) -> Option<CompletionBarrierSnapshot> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let (_, owner) = state.active.front()?;
        Some(CompletionBarrierSnapshot {
            generation: owner.generation,
            owner_method: owner.method.clone(),
            owner_uri: owner.uri.clone(),
            owner_version: owner.version,
            doc_sync_first_poll_exec_ms: owner.first_poll_exec_ms,
            doc_sync_first_poll_outcome: owner.first_poll_outcome.clone(),
        })
    }

    async fn wait_for_release(&self) {
        if !self.is_active() {
            return;
        }
        self.released.notified().await;
    }
}

#[derive(Debug)]
struct QueuedTransportMessage {
    message: TransportMessage,
    completion_request_id: Option<String>,
    completion_response_handoff_started_at_ms: Option<u64>,
    completion_response_handoff_enqueued_at_ms: Option<Arc<AtomicU64>>,
}

impl QueuedTransportMessage {
    fn request(request: Request) -> Self {
        Self {
            message: TransportMessage::Request(request),
            completion_request_id: None,
            completion_response_handoff_started_at_ms: None,
            completion_response_handoff_enqueued_at_ms: None,
        }
    }

    fn response(response: Response) -> Self {
        Self {
            message: TransportMessage::Response(response),
            completion_request_id: None,
            completion_response_handoff_started_at_ms: None,
            completion_response_handoff_enqueued_at_ms: None,
        }
    }

    fn completion_response(
        response: Response,
        completion_request_id: Option<String>,
        completion_response_handoff_started_at_ms: u64,
        completion_response_handoff_enqueued_at_ms: Arc<AtomicU64>,
    ) -> Self {
        Self {
            message: TransportMessage::Response(response),
            completion_request_id,
            completion_response_handoff_started_at_ms: Some(
                completion_response_handoff_started_at_ms,
            ),
            completion_response_handoff_enqueued_at_ms: Some(
                completion_response_handoff_enqueued_at_ms,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TransportMessageWriteMilestones {
    encode_started_at_ms: u64,
    write_started_at_ms: u64,
    encode_completed_at_ms: u64,
    flush_completed_at_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct TransportMessageReadMilestones {
    read_started_at_ms: u64,
    parse_completed_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdmissionLane {
    Control,
    Completion,
    General,
}

#[derive(Debug)]
struct ScheduledRequest {
    lane: AdmissionLane,
    request_id: Option<String>,
    request: Request,
}

#[derive(Debug, Clone, Copy)]
struct AdmissionEnqueueMetadata {
    lane_depth_before: usize,
    lane_depth_after: usize,
}

#[derive(Debug, Clone, Copy)]
struct ReadLoopWaitObservation {
    reason: &'static str,
    started_at_ms: u64,
    pending_completion_spillover_depth: u64,
    pending_general_request_staged: bool,
}

#[derive(Debug, Clone, Copy)]
struct AdmissionQueueCapacities {
    control: usize,
    completion: usize,
    general: usize,
}

impl Default for AdmissionQueueCapacities {
    fn default() -> Self {
        Self {
            control: CONTROL_QUEUE_SIZE.max(1),
            completion: MESSAGE_QUEUE_SIZE.max(1),
            general: MESSAGE_QUEUE_SIZE.max(1),
        }
    }
}

#[derive(Debug, Default)]
struct AdmissionQueuesState {
    control: std::collections::VecDeque<ScheduledRequest>,
    completion: std::collections::VecDeque<ScheduledRequest>,
    general: std::collections::VecDeque<ScheduledRequest>,
    closed: bool,
}

#[derive(Debug, Clone)]
struct AdmissionQueues {
    capacities: AdmissionQueueCapacities,
    state: Arc<Mutex<AdmissionQueuesState>>,
    item_notify: Arc<Notify>,
    space_notify: Arc<Notify>,
}

impl AdmissionQueues {
    fn new(capacities: AdmissionQueueCapacities) -> Self {
        Self {
            capacities,
            state: Arc::new(Mutex::new(AdmissionQueuesState::default())),
            item_notify: Arc::new(Notify::new()),
            space_notify: Arc::new(Notify::new()),
        }
    }

    fn lane_capacity(&self, lane: AdmissionLane) -> usize {
        match lane {
            AdmissionLane::Control => self.capacities.control,
            AdmissionLane::Completion => self.capacities.completion,
            AdmissionLane::General => self.capacities.general,
        }
    }

    fn queue_for_lane_mut(
        state: &mut AdmissionQueuesState,
        lane: AdmissionLane,
    ) -> &mut std::collections::VecDeque<ScheduledRequest> {
        match lane {
            AdmissionLane::Control => &mut state.control,
            AdmissionLane::Completion => &mut state.completion,
            AdmissionLane::General => &mut state.general,
        }
    }

    fn has_any(state: &AdmissionQueuesState) -> bool {
        !(state.control.is_empty() && state.completion.is_empty() && state.general.is_empty())
    }

    fn queue_for_lane(
        state: &AdmissionQueuesState,
        lane: AdmissionLane,
    ) -> &std::collections::VecDeque<ScheduledRequest> {
        match lane {
            AdmissionLane::Control => &state.control,
            AdmissionLane::Completion => &state.completion,
            AdmissionLane::General => &state.general,
        }
    }

    async fn enqueue(&self, scheduled_request: ScheduledRequest) -> bool {
        let mut scheduled_request = Some(scheduled_request);
        loop {
            match self.try_enqueue(scheduled_request.take().expect("scheduled request")) {
                Ok(_) => return true,
                Err(TryEnqueueError::Closed) => return false,
                Err(TryEnqueueError::Full {
                    request,
                    lane_depth_before: _,
                }) => {
                    scheduled_request = Some(request);
                    let notified = self.space_notify.notified();
                    notified.await;
                }
            }
        }
    }

    fn try_enqueue(
        &self,
        scheduled_request: ScheduledRequest,
    ) -> Result<AdmissionEnqueueMetadata, TryEnqueueError> {
        let lane = scheduled_request.lane;
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.closed {
            return Err(TryEnqueueError::Closed);
        }
        let lane_capacity = self.lane_capacity(lane);
        let queue = Self::queue_for_lane_mut(&mut state, lane);
        let lane_depth_before = queue.len();
        if lane_depth_before < lane_capacity {
            if matches!(lane, AdmissionLane::Completion) {
                if let Some(position) = completion_queue_insert_position(queue, &scheduled_request)
                {
                    queue.insert(position, scheduled_request);
                } else {
                    queue.push_back(scheduled_request);
                }
            } else {
                queue.push_back(scheduled_request);
            }
            self.item_notify.notify_waiters();
            Ok(AdmissionEnqueueMetadata {
                lane_depth_before,
                lane_depth_after: queue.len(),
            })
        } else {
            Err(TryEnqueueError::Full {
                request: scheduled_request,
                lane_depth_before,
            })
        }
    }

    async fn wait_for_space_in_lane_or_closed(&self, lane: AdmissionLane) -> bool {
        loop {
            let notified = {
                let state = self
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if state.closed {
                    return false;
                }
                if Self::queue_for_lane(&state, lane).len() < self.lane_capacity(lane) {
                    return true;
                }
                self.space_notify.notified()
            };
            notified.await;
        }
    }

    async fn wait_until_non_empty_or_closed(&self) -> bool {
        loop {
            let notified = {
                let state = self
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if Self::has_any(&state) {
                    return true;
                }
                if state.closed {
                    return false;
                }
                self.item_notify.notified()
            };
            notified.await;
        }
    }

    async fn pop_next(&self, completion_barrier_active: bool) -> Option<ScheduledRequest> {
        let next = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(control) = state.control.pop_front() {
                Some(control)
            } else {
                let completion_dispatchable = !completion_barrier_active
                    || state.completion.front().is_some_and(|scheduled| {
                        is_completion_supporting_document_sync_notification(&scheduled.request)
                    });
                if completion_dispatchable {
                    if !completion_barrier_active {
                        if let Some(position) = token_ready_completion_position(&state.completion) {
                            state
                                .completion
                                .remove(position)
                                .or_else(|| state.general.pop_front())
                        } else {
                            state
                                .completion
                                .pop_front()
                                .or_else(|| state.general.pop_front())
                        }
                    } else {
                        state
                            .completion
                            .pop_front()
                            .or_else(|| state.general.pop_front())
                    }
                } else {
                    state.general.pop_front()
                }
            }
        };
        if next.is_some() {
            self.space_notify.notify_waiters();
        }
        next
    }

    async fn remove_queued_completion_by_request_id(
        &self,
        request_id: &str,
    ) -> Option<ScheduledRequest> {
        let removed = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let position = state
                .completion
                .iter()
                .position(|scheduled| scheduled.request_id.as_deref() == Some(request_id))?;
            state.completion.remove(position)
        };
        if removed.is_some() {
            self.space_notify.notify_waiters();
        }
        removed
    }

    fn close(&self) {
        {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.closed = true;
        }
        self.item_notify.notify_waiters();
        self.space_notify.notify_waiters();
    }

    fn lane_depth(&self, lane: AdmissionLane) -> usize {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Self::queue_for_lane(&state, lane).len()
    }

    fn queued_completion_request_ids(&self) -> Vec<String> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state
            .completion
            .iter()
            .filter_map(|scheduled| scheduled.request_id.clone())
            .collect()
    }
}

enum TryEnqueueError {
    Closed,
    Full {
        request: ScheduledRequest,
        lane_depth_before: usize,
    },
}

pub(crate) async fn serve_with_completion_handoff<I, O, L, S>(
    stdin: I,
    stdout: O,
    socket: L,
    service: S,
    concurrency_level: usize,
) where
    I: AsyncRead + Send + Unpin + 'static,
    O: AsyncWrite + Send + Unpin + 'static,
    L: Loopback + Send + 'static,
    L::RequestStream: Send + 'static,
    L::ResponseSink: Send + 'static,
    <L::ResponseSink as Sink<Response>>::Error: std::error::Error + Send + Sync + 'static,
    S: Service<Request, Response = Option<Response>> + Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: Send + 'static,
{
    serve_with_completion_handoff_with_capacities(
        stdin,
        stdout,
        socket,
        service,
        concurrency_level,
        AdmissionQueueCapacities::default(),
    )
    .await;
}

async fn serve_with_completion_handoff_with_capacities<I, O, L, S>(
    stdin: I,
    stdout: O,
    socket: L,
    service: S,
    concurrency_level: usize,
    admission_queue_capacities: AdmissionQueueCapacities,
) where
    I: AsyncRead + Send + Unpin + 'static,
    O: AsyncWrite + Send + Unpin + 'static,
    L: Loopback + Send + 'static,
    L::RequestStream: Send + 'static,
    L::ResponseSink: Send + 'static,
    <L::ResponseSink as Sink<Response>>::Error: std::error::Error + Send + Sync + 'static,
    S: Service<Request, Response = Option<Response>> + Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: Send + 'static,
{
    let admission_queues = AdmissionQueues::new(admission_queue_capacities);
    serve_with_completion_handoff_with_admission_queues(
        stdin,
        stdout,
        socket,
        service,
        concurrency_level,
        admission_queues,
    )
    .await;
}

async fn serve_with_completion_handoff_with_admission_queues<I, O, L, S>(
    stdin: I,
    stdout: O,
    socket: L,
    mut service: S,
    concurrency_level: usize,
    admission_queues: AdmissionQueues,
) where
    I: AsyncRead + Send + Unpin + 'static,
    O: AsyncWrite + Send + Unpin + 'static,
    L: Loopback + Send + 'static,
    L::RequestStream: Send + 'static,
    L::ResponseSink: Send + 'static,
    <L::ResponseSink as Sink<Response>>::Error: std::error::Error + Send + Sync + 'static,
    S: Service<Request, Response = Option<Response>> + Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: Send + 'static,
{
    let (client_requests, mut client_responses) = socket.split();
    let (client_requests, client_abort) = stream::abortable(client_requests);
    let (mut responses_tx, responses_rx) = mpsc::channel(0);
    let (mut server_tasks_tx, server_tasks_rx) =
        mpsc::channel::<BoxFuture<'static, Option<Response>>>(MESSAGE_QUEUE_SIZE);
    let (mut completion_tasks_tx, completion_tasks_rx) =
        mpsc::channel::<CompletionHandoffTask>(MESSAGE_QUEUE_SIZE);
    let transport_shutdown = std::sync::Arc::new(Notify::new());
    let completion_barrier_gate = CompletionBarrierGate::new();
    let transport_shutdown_for_supervisor = transport_shutdown.clone();
    let admission_queues_for_supervisor = admission_queues.clone();
    let client_abort_for_supervisor = client_abort.clone();

    let responses_tx_for_server_tasks = responses_tx.clone();
    let process_server_tasks = async move {
        let mut responses_tx = responses_tx_for_server_tasks;
        let mut server_tasks = server_tasks_rx.buffer_unordered(concurrency_level);

        while let Some(response) = server_tasks.next().await {
            let Some(response) = response else {
                continue;
            };
            if responses_tx
                .send(QueuedTransportMessage::response(response))
                .await
                .is_err()
            {
                break;
            }
        }
    };

    let transport_shutdown_for_scheduler = transport_shutdown.clone();
    let admission_queues_for_scheduler = admission_queues.clone();
    let client_abort_for_scheduler = client_abort.clone();
    let responses_tx_for_scheduler = responses_tx.clone();
    let completion_barrier_gate_for_scheduler = completion_barrier_gate.clone();
    let process_scheduler = async move {
        let mut responses_tx = responses_tx_for_scheduler;
        loop {
            if !admission_queues_for_scheduler
                .wait_until_non_empty_or_closed()
                .await
            {
                break;
            }
            let scheduler_woke_at_ms = super::unix_timestamp_ms();
            let scheduler_poll_ready_entered_at_ms = super::unix_timestamp_ms();
            if let Err(err) = future::poll_fn(|cx| service.poll_ready(cx)).await {
                error!("{}", display_sources(err.into().as_ref()));
                break;
            }
            let scheduler_poll_ready_resolved_at_ms = super::unix_timestamp_ms();
            let completion_barrier_active = completion_barrier_gate_for_scheduler.is_active();
            let Some(scheduled_request) = admission_queues_for_scheduler
                .pop_next(completion_barrier_active)
                .await
            else {
                if completion_barrier_active {
                    let barrier_wait_started_at_ms = super::unix_timestamp_ms();
                    if let Some(snapshot) = completion_barrier_gate_for_scheduler.snapshot() {
                        for request_id in
                            admission_queues_for_scheduler.queued_completion_request_ids()
                        {
                            patch_completion_pre_dispatch_barrier_snapshot(
                                &request_id,
                                &snapshot,
                                Some(barrier_wait_started_at_ms),
                            );
                        }
                    }
                    completion_barrier_gate_for_scheduler
                        .wait_for_release()
                        .await;
                }
                continue;
            };
            let scheduler_dequeued_at_ms = super::unix_timestamp_ms();
            if let Some(request_id) = scheduled_request.request_id.as_deref() {
                super::request_context::patch_pending_completion_pre_dispatch_trace(
                    request_id,
                    super::request_context::CompletionPreDispatchTracePatch {
                        scheduler_woke_at_ms: Some(scheduler_woke_at_ms),
                        scheduler_poll_ready_entered_at_ms: Some(
                            scheduler_poll_ready_entered_at_ms,
                        ),
                        scheduler_poll_ready_resolved_at_ms: Some(
                            scheduler_poll_ready_resolved_at_ms,
                        ),
                        scheduler_dequeued_at_ms: Some(scheduler_dequeued_at_ms),
                        completion_barrier_active_at_dequeue: Some(completion_barrier_active),
                        ..Default::default()
                    },
                );
            }

            if let Some(cancelled_request_id) =
                cancelled_request_id_from_request(&scheduled_request.request)
            {
                if cancel_queued_completion_before_dispatch(
                    &admission_queues_for_scheduler,
                    &cancelled_request_id,
                    &mut responses_tx,
                )
                .await
                .is_err()
                {
                    break;
                }
            }

            let request_id = scheduled_request.request_id.clone();
            let is_completion = matches!(scheduled_request.lane, AdmissionLane::Completion);
            let is_completion_handoff_barrier =
                is_completion_supporting_document_sync_notification(&scheduled_request.request);
            let completion_barrier_owner_method = is_completion_handoff_barrier
                .then(|| scheduled_request.request.method().to_string());
            let completion_barrier_owner_uri = is_completion_handoff_barrier
                .then(|| {
                    request_text_document_uri(&scheduled_request.request).map(ToString::to_string)
                })
                .flatten();
            let completion_barrier_owner_version = is_completion_handoff_barrier
                .then(|| request_text_document_version(&scheduled_request.request))
                .flatten();
            let scheduler_service_call_started_at_ms = super::unix_timestamp_ms();
            let future = service
                .call(scheduled_request.request)
                .unwrap_or_else(|err| {
                    error!("{}", display_sources(err.into().as_ref()));
                    None
                })
                .boxed();
            let scheduler_service_call_returned_at_ms = super::unix_timestamp_ms();
            if let Some(request_id) = request_id.as_deref() {
                super::request_context::patch_pending_completion_pre_dispatch_trace(
                    request_id,
                    super::request_context::CompletionPreDispatchTracePatch {
                        scheduler_service_call_started_at_ms: Some(
                            scheduler_service_call_started_at_ms,
                        ),
                        scheduler_service_call_returned_at_ms: Some(
                            scheduler_service_call_returned_at_ms,
                        ),
                        ..Default::default()
                    },
                );
            }

            if is_completion_handoff_barrier {
                let mut future = future;
                let first_poll_started_at_ms = super::unix_timestamp_ms();
                let first_poll = future::poll_fn(|cx| match future.as_mut().poll(cx) {
                    std::task::Poll::Ready(response) => {
                        std::task::Poll::Ready(CompletionBarrierFirstPoll::Ready(response))
                    }
                    std::task::Poll::Pending => {
                        std::task::Poll::Ready(CompletionBarrierFirstPoll::Pending)
                    }
                })
                .await;
                let first_poll_completed_at_ms = super::unix_timestamp_ms();
                let first_poll_exec_ms =
                    first_poll_completed_at_ms.saturating_sub(first_poll_started_at_ms);
                match first_poll {
                    CompletionBarrierFirstPoll::Ready(response) => {
                        if let Some(response) = response {
                            if responses_tx
                                .send(QueuedTransportMessage::response(response))
                                .await
                                .is_err()
                            {
                                error!(
                                    "failed to forward completion handoff barrier response: transport closed"
                                );
                                break;
                            }
                        }
                    }
                    CompletionBarrierFirstPoll::Pending => {
                        let barrier_owner = completion_barrier_owner_metadata(
                            completion_barrier_owner_method
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string()),
                            completion_barrier_owner_uri.clone(),
                            completion_barrier_owner_version,
                            first_poll_exec_ms,
                            "pending",
                        );
                        let barrier_ticket =
                            completion_barrier_gate_for_scheduler.begin(barrier_owner);
                        let barrier_gate = completion_barrier_gate_for_scheduler.clone();
                        let barrier_future = async move {
                            let response = future.await;
                            barrier_gate.release(barrier_ticket);
                            response
                        }
                        .boxed();
                        if server_tasks_tx.send(barrier_future).await.is_err() {
                            completion_barrier_gate_for_scheduler.release(barrier_ticket);
                            error!("server task queue closed unexpectedly");
                            break;
                        }
                    }
                }
            } else if is_completion {
                let task = CompletionHandoffTask::new(request_id.clone(), future);
                if completion_tasks_tx.send(task).await.is_err() {
                    error!("completion handoff queue closed unexpectedly");
                    break;
                }
                if let Some(request_id) = request_id.as_deref() {
                    super::request_context::record_pending_completion_transport_slot_released_at_ms(
                        request_id,
                        super::unix_timestamp_ms(),
                    );
                }
            } else if server_tasks_tx.send(future).await.is_err() {
                error!("server task queue closed unexpectedly");
                break;
            }
        }

        admission_queues_for_scheduler.close();
        transport_shutdown_for_scheduler.notify_waiters();
        client_abort_for_scheduler.abort();
    };

    let responses_tx_for_completion = responses_tx.clone();
    let process_completion_tasks = async move {
        let mut completion_tasks = JoinSet::new();
        let mut completion_tasks_rx = completion_tasks_rx.fuse();
        let mut receiver_closed = false;

        loop {
            tokio::select! {
                maybe_task = completion_tasks_rx.next(), if !receiver_closed => {
                    match maybe_task {
                        Some(task) => {
                            let responses_tx = responses_tx_for_completion.clone();
                            completion_tasks.spawn(async move {
                                task.forward_response(responses_tx).await;
                            });
                        }
                        None => {
                            receiver_closed = true;
                            completion_tasks.abort_all();
                            if completion_tasks.is_empty() {
                                break;
                            }
                        }
                    }
                }
                join_result = completion_tasks.join_next(), if !completion_tasks.is_empty() => {
                    if let Some(Err(err)) = join_result {
                        if !err.is_cancelled() {
                            error!("completion handoff task failed: {err}");
                        }
                    }
                }
                else => {
                    if receiver_closed {
                        break;
                    }
                }
            }
        }
    };

    let transport_shutdown_for_output = transport_shutdown.clone();
    let print_output = async move {
        let mut stdout = BufWriter::new(stdout);
        let outbound = stream::select(
            responses_rx,
            client_requests.map(QueuedTransportMessage::request),
        );
        pin_mut!(outbound);

        loop {
            tokio::select! {
                _ = transport_shutdown_for_output.notified() => break,
                maybe_queued_message = outbound.next() => {
                    let Some(queued_message) = maybe_queued_message else {
                        break;
                    };
                    let response_output_enqueue_completed_at_ms = super::unix_timestamp_ms();
                    let write_milestones = match write_transport_message(&mut stdout, &queued_message.message).await {
                        Ok(milestones) => milestones,
                        Err(err) => {
                        error!("failed to encode message: {err}");
                        break;
                        }
                    };
                    if let Some(request_id) = queued_message.completion_request_id {
                        let response_output_handoff_enqueued_at_ms =
                            resolve_completion_handoff_enqueued_at_ms(
                                queued_message
                                    .completion_response_handoff_enqueued_at_ms
                                    .as_ref(),
                                response_output_enqueue_completed_at_ms,
                            )
                            .await;
                        super::request_context::notify_completion_response_egress(
                            super::request_context::CompletionResponseEgressTracePatch {
                                request_id,
                                response_output_handoff_started_at_ms: queued_message
                                    .completion_response_handoff_started_at_ms
                                    .unwrap_or(response_output_enqueue_completed_at_ms),
                                response_output_handoff_enqueued_at_ms,
                                response_output_enqueue_completed_at_ms,
                                response_output_encode_started_at_ms: write_milestones.encode_started_at_ms,
                                response_output_write_started_at_ms: write_milestones.write_started_at_ms,
                                response_output_encode_completed_at_ms: write_milestones.encode_completed_at_ms,
                                response_flush_completed_at_ms: write_milestones.flush_completed_at_ms,
                            },
                        );
                    }
                }
            }
        }
    };

    let transport_shutdown_for_input = transport_shutdown.clone();
    let admission_queues_for_input = admission_queues.clone();
    let client_abort_for_input = client_abort;
    let completion_barrier_gate_for_input = completion_barrier_gate.clone();
    let read_input = async move {
        let mut stdin = BufReader::new(stdin);
        let completion_spillover_capacity =
            admission_queues_for_input.lane_capacity(AdmissionLane::Completion);
        let mut pending_completion_requests = std::collections::VecDeque::<ScheduledRequest>::new();
        let mut pending_general_request = None;
        let mut next_read_wait_observation: Option<ReadLoopWaitObservation> = None;

        'read_input: loop {
            while let Some(staged_completion_request) = pending_completion_requests.pop_front() {
                let request_id = staged_completion_request.request_id.clone();
                match admission_queues_for_input.try_enqueue(staged_completion_request) {
                    Ok(metadata) => {
                        if let Some(request_id) = request_id.as_deref() {
                            super::request_context::patch_pending_completion_pre_dispatch_trace(
                                request_id,
                                super::request_context::CompletionPreDispatchTracePatch {
                                    admission_lane: Some("interactive_completion".to_string()),
                                    admission_lane_depth_before: Some(
                                        metadata.lane_depth_before as u64,
                                    ),
                                    admission_lane_depth_after: Some(
                                        metadata.lane_depth_after as u64,
                                    ),
                                    admission_enqueue_outcome: Some("enqueued".to_string()),
                                    admission_enqueued_at_ms: Some(super::unix_timestamp_ms()),
                                    ..Default::default()
                                },
                            );
                            if let Some(snapshot) = completion_barrier_gate_for_input.snapshot() {
                                patch_completion_pre_dispatch_barrier_snapshot(
                                    request_id,
                                    &snapshot,
                                    Some(super::unix_timestamp_ms()),
                                );
                            }
                        }
                    }
                    Err(TryEnqueueError::Closed) => {
                        error!("transport admission queue closed unexpectedly");
                        break 'read_input;
                    }
                    Err(TryEnqueueError::Full {
                        request,
                        lane_depth_before,
                    }) => {
                        if let Some(request_id) = request_id.as_deref() {
                            super::request_context::patch_pending_completion_pre_dispatch_trace(
                                request_id,
                                super::request_context::CompletionPreDispatchTracePatch {
                                    admission_lane: Some("interactive_completion".to_string()),
                                    admission_lane_depth_before: Some(lane_depth_before as u64),
                                    admission_lane_depth_after: Some(lane_depth_before as u64),
                                    admission_enqueue_outcome: Some(
                                        "lane_full_deferred".to_string(),
                                    ),
                                    ..Default::default()
                                },
                            );
                        }
                        pending_completion_requests.push_front(request);
                        break;
                    }
                }
            }

            if let Some(staged_general_request) = pending_general_request.take() {
                match admission_queues_for_input.try_enqueue(staged_general_request) {
                    Ok(_) => {}
                    Err(TryEnqueueError::Closed) => {
                        error!("transport admission queue closed unexpectedly");
                        break;
                    }
                    Err(TryEnqueueError::Full {
                        request,
                        lane_depth_before: _,
                    }) => {
                        pending_general_request = Some(request);
                    }
                }
            }

            let read_result = tokio::select! {
                _ = transport_shutdown_for_input.notified() => break,
                completion_wait = async {
                    let wait_started_at_ms = super::unix_timestamp_ms();
                    let lane_has_space = admission_queues_for_input
                        .wait_for_space_in_lane_or_closed(AdmissionLane::Completion)
                        .await;
                    (wait_started_at_ms, lane_has_space)
                }, if !pending_completion_requests.is_empty() => {
                    let (wait_started_at_ms, lane_has_space) = completion_wait;
                    if !lane_has_space {
                        error!("transport admission queue closed unexpectedly");
                        break;
                    }
                    next_read_wait_observation.get_or_insert(ReadLoopWaitObservation {
                        reason: "completion_lane_space",
                        started_at_ms: wait_started_at_ms,
                        pending_completion_spillover_depth: pending_completion_requests.len() as u64,
                        pending_general_request_staged: pending_general_request.is_some(),
                    });
                    continue;
                }
                general_wait = async {
                    let wait_started_at_ms = super::unix_timestamp_ms();
                    let lane_has_space = admission_queues_for_input
                        .wait_for_space_in_lane_or_closed(AdmissionLane::General)
                        .await;
                    (wait_started_at_ms, lane_has_space)
                }, if pending_general_request.is_some() => {
                    let (wait_started_at_ms, lane_has_space) = general_wait;
                    if !lane_has_space {
                        error!("transport admission queue closed unexpectedly");
                        break;
                    }
                    next_read_wait_observation.get_or_insert(ReadLoopWaitObservation {
                        reason: "general_lane_space",
                        started_at_ms: wait_started_at_ms,
                        pending_completion_spillover_depth: pending_completion_requests.len() as u64,
                        pending_general_request_staged: pending_general_request.is_some(),
                    });
                    continue;
                }
                read_result = read_transport_message(&mut stdin) => read_result,
            };
            let read_wait_observation = match &read_result {
                Ok(Some(_)) => next_read_wait_observation.take(),
                _ => None,
            };
            match read_result {
                Ok(Some((TransportMessage::Request(request), read_milestones))) => {
                    let adapter_read_at_ms = read_milestones.parse_completed_at_ms;
                    let request_id = request.id().map(ToString::to_string);
                    if let Some(request_id) = request_id.as_deref() {
                        super::request_context::record_pending_completion_adapter_read_at_ms(
                            &request,
                            request_id,
                            Some(adapter_read_at_ms),
                        );
                        let read_wait_patch = read_wait_observation
                            .map(|observation| {
                                super::request_context::CompletionPreDispatchTracePatch {
                                    adapter_read_started_at_ms: Some(
                                        read_milestones.read_started_at_ms,
                                    ),
                                    adapter_parse_completed_at_ms: Some(adapter_read_at_ms),
                                    read_loop_wait_reason: Some(observation.reason.to_string()),
                                    read_loop_wait_ms: Some(
                                        adapter_read_at_ms
                                            .saturating_sub(observation.started_at_ms),
                                    ),
                                    pending_completion_spillover_depth: Some(
                                        observation.pending_completion_spillover_depth,
                                    ),
                                    pending_general_request_staged: Some(
                                        observation.pending_general_request_staged,
                                    ),
                                    ..Default::default()
                                }
                            })
                            .unwrap_or_else(|| {
                                super::request_context::CompletionPreDispatchTracePatch {
                                    adapter_read_started_at_ms: Some(
                                        read_milestones.read_started_at_ms,
                                    ),
                                    adapter_parse_completed_at_ms: Some(adapter_read_at_ms),
                                    ..Default::default()
                                }
                            });
                        super::request_context::patch_pending_completion_pre_dispatch_trace(
                            request_id,
                            read_wait_patch,
                        );
                    }
                    let scheduled_request = ScheduledRequest {
                        lane: classify_admission_lane(&request),
                        request_id,
                        request,
                    };
                    if let Some(cancelled_request_id) =
                        cancelled_request_id_from_request(&scheduled_request.request)
                    {
                        if let Some(position) =
                            pending_completion_requests
                                .iter()
                                .position(|pending_request| {
                                    pending_request.request_id.as_deref()
                                        == Some(cancelled_request_id.as_str())
                                })
                        {
                            let cancelled_request = pending_completion_requests
                                .remove(position)
                                .expect("pending completion cancellation position");
                            if respond_to_pre_dispatch_cancelled_completion(
                                cancelled_request,
                                &cancelled_request_id,
                                &mut responses_tx,
                            )
                            .await
                            .is_err()
                            {
                                break;
                            }
                            continue;
                        }
                    }
                    if matches!(scheduled_request.lane, AdmissionLane::General) {
                        if pending_general_request.is_some() {
                            if reject_saturated_general_request(
                                &mut responses_tx,
                                scheduled_request,
                            )
                            .await
                            .is_err()
                            {
                                break;
                            }
                            continue;
                        }
                        match admission_queues_for_input.try_enqueue(scheduled_request) {
                            Ok(_) => {}
                            Err(TryEnqueueError::Closed) => {
                                error!("transport admission queue closed unexpectedly");
                                break;
                            }
                            Err(TryEnqueueError::Full {
                                request,
                                lane_depth_before: _,
                            }) => {
                                pending_general_request = Some(request);
                            }
                        }
                    } else if matches!(scheduled_request.lane, AdmissionLane::Completion) {
                        let request_id = scheduled_request.request_id.clone();
                        let admission_try_enqueue_at_ms = super::unix_timestamp_ms();
                        if let Some(request_id) = request_id.as_deref() {
                            super::request_context::patch_pending_completion_pre_dispatch_trace(
                                request_id,
                                super::request_context::CompletionPreDispatchTracePatch {
                                    admission_try_enqueue_at_ms: Some(admission_try_enqueue_at_ms),
                                    admission_lane: Some("interactive_completion".to_string()),
                                    ..Default::default()
                                },
                            );
                        }
                        match admission_queues_for_input.try_enqueue(scheduled_request) {
                            Ok(metadata) => {
                                if let Some(request_id) = request_id.as_deref() {
                                    super::request_context::patch_pending_completion_pre_dispatch_trace(
                                        request_id,
                                        super::request_context::CompletionPreDispatchTracePatch {
                                            admission_lane_depth_before: Some(
                                                metadata.lane_depth_before as u64,
                                            ),
                                            admission_lane_depth_after: Some(
                                                metadata.lane_depth_after as u64,
                                            ),
                                            admission_enqueue_outcome: Some("enqueued".to_string()),
                                            admission_enqueued_at_ms: Some(
                                                super::unix_timestamp_ms(),
                                            ),
                                            ..Default::default()
                                        },
                                    );
                                    if let Some(snapshot) =
                                        completion_barrier_gate_for_input.snapshot()
                                    {
                                        patch_completion_pre_dispatch_barrier_snapshot(
                                            request_id,
                                            &snapshot,
                                            Some(super::unix_timestamp_ms()),
                                        );
                                    }
                                }
                            }
                            Err(TryEnqueueError::Closed) => {
                                error!("transport admission queue closed unexpectedly");
                                break;
                            }
                            Err(TryEnqueueError::Full {
                                request,
                                lane_depth_before,
                            }) => {
                                let request_id = request.request_id.clone();
                                if stage_completion_request_with_overflow_policy(
                                    &mut pending_completion_requests,
                                    completion_spillover_capacity,
                                    request,
                                    &mut responses_tx,
                                )
                                .await
                                .is_err()
                                {
                                    break;
                                }
                                if let Some(request_id) = request_id.as_deref() {
                                    super::request_context::patch_pending_completion_pre_dispatch_trace(
                                        request_id,
                                        super::request_context::CompletionPreDispatchTracePatch {
                                            admission_lane_depth_before: Some(
                                                lane_depth_before as u64,
                                            ),
                                            admission_lane_depth_after: Some(
                                                lane_depth_before as u64,
                                            ),
                                            admission_enqueue_outcome: Some(
                                                "lane_full_deferred".to_string(),
                                            ),
                                            admission_spillover_outcome: Some(
                                                "staged_completion_spillover".to_string(),
                                            ),
                                            pending_completion_spillover_depth: Some(
                                                pending_completion_requests.len() as u64,
                                            ),
                                            ..Default::default()
                                        },
                                    );
                                }
                            }
                        }
                    } else if !admission_queues_for_input.enqueue(scheduled_request).await {
                        error!("transport admission queue closed unexpectedly");
                        break;
                    }
                }
                Ok(Some((TransportMessage::Response(response), _read_milestones))) => {
                    if let Err(err) = client_responses.send(response).await {
                        error!("{}", display_sources(&err));
                        break;
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    error!("failed to decode message: {err}");
                    let response = Response::from_error(Id::Null, to_jsonrpc_error(&err));
                    if responses_tx
                        .send(QueuedTransportMessage::response(response))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }

        admission_queues_for_input.close();
        transport_shutdown_for_input.notify_waiters();
        client_abort_for_input.abort();
    };

    let mut transport_tasks = JoinSet::new();
    transport_tasks.spawn(async move {
        process_server_tasks.await;
    });
    transport_tasks.spawn(async move {
        process_scheduler.await;
    });
    transport_tasks.spawn(async move {
        process_completion_tasks.await;
    });
    transport_tasks.spawn(async move {
        print_output.await;
    });
    transport_tasks.spawn(async move {
        read_input.await;
    });

    let mut abort_requested = false;
    while let Some(join_result) = transport_tasks.join_next().await {
        if let Err(err) = join_result {
            if !err.is_cancelled() {
                error!("transport runtime task failed: {err}");
            }
            if !abort_requested {
                abort_requested = true;
                admission_queues_for_supervisor.close();
                transport_shutdown_for_supervisor.notify_waiters();
                client_abort_for_supervisor.abort();
                transport_tasks.abort_all();
            }
        }
    }
}

fn classify_admission_lane(request: &Request) -> AdmissionLane {
    if is_control_request(request) {
        AdmissionLane::Control
    } else if is_completion_priority_request(request) {
        AdmissionLane::Completion
    } else {
        AdmissionLane::General
    }
}

fn is_completion_priority_request(request: &Request) -> bool {
    is_completion_request(request) || is_completion_supporting_document_sync_notification(request)
}

fn is_completion_request(request: &Request) -> bool {
    request.method() == COMPLETION_METHOD && request.id().is_some()
}

fn is_completion_supporting_document_sync_notification(request: &Request) -> bool {
    request.id().is_none()
        && matches!(
            request.method(),
            DID_OPEN_METHOD | DID_CHANGE_METHOD | DID_SAVE_METHOD | DID_CLOSE_METHOD
        )
}

fn is_control_request(request: &Request) -> bool {
    matches!(
        request.method(),
        CANCEL_REQUEST_METHOD | SHUTDOWN_METHOD | EXIT_METHOD
    )
}

fn cancelled_request_id_from_request(request: &Request) -> Option<String> {
    if request.method() != CANCEL_REQUEST_METHOD {
        return None;
    }
    request.params()?.get("id").and_then(|value| {
        value
            .as_i64()
            .map(|id| id.to_string())
            .or_else(|| value.as_str().map(ToString::to_string))
    })
}

fn request_text_document_uri(request: &Request) -> Option<&str> {
    request.params()?.get("textDocument")?.get("uri")?.as_str()
}

fn completion_queue_insert_position(
    queued_requests: &std::collections::VecDeque<ScheduledRequest>,
    scheduled_request: &ScheduledRequest,
) -> Option<usize> {
    if scheduled_request.request.method() != COMPLETION_METHOD {
        return None;
    }
    let uri = request_text_document_uri(&scheduled_request.request)?;
    if super::request_context::same_file_ingress_token_publication_for_uri(uri).is_none() {
        return None;
    }
    Some(
        queued_requests
            .iter()
            .rposition(|queued| request_text_document_uri(&queued.request) == Some(uri))
            .map_or(0, |position| position + 1),
    )
}

fn token_ready_completion_position(
    queued_requests: &std::collections::VecDeque<ScheduledRequest>,
) -> Option<usize> {
    queued_requests
        .iter()
        .enumerate()
        .find_map(|(position, scheduled)| {
            if scheduled.request.method() != COMPLETION_METHOD {
                return None;
            }
            let uri = request_text_document_uri(&scheduled.request)?;
            if super::request_context::same_file_ingress_token_publication_for_uri(uri).is_none() {
                return None;
            }
            let has_earlier_related_work = queued_requests
                .iter()
                .take(position)
                .any(|earlier| request_text_document_uri(&earlier.request) == Some(uri));
            if has_earlier_related_work {
                None
            } else {
                Some(position)
            }
        })
}

fn request_text_document_version(request: &Request) -> Option<i32> {
    request
        .params()?
        .get("textDocument")?
        .get("version")?
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
}

fn completion_barrier_owner_metadata(
    method: String,
    uri: Option<String>,
    version: Option<i32>,
    first_poll_exec_ms: u64,
    first_poll_outcome: &str,
) -> CompletionBarrierOwnerMetadata {
    CompletionBarrierOwnerMetadata {
        generation: 0,
        method,
        uri,
        version,
        first_poll_exec_ms,
        first_poll_outcome: first_poll_outcome.to_string(),
    }
}

fn patch_completion_pre_dispatch_barrier_snapshot(
    request_id: &str,
    snapshot: &CompletionBarrierSnapshot,
    completion_barrier_wait_started_at_ms: Option<u64>,
) {
    super::request_context::patch_pending_completion_pre_dispatch_trace(
        request_id,
        super::request_context::CompletionPreDispatchTracePatch {
            completion_barrier_generation: Some(snapshot.generation),
            completion_barrier_owner_method: Some(snapshot.owner_method.clone()),
            completion_barrier_owner_uri: snapshot.owner_uri.clone(),
            completion_barrier_owner_version: snapshot.owner_version,
            completion_barrier_wait_started_at_ms,
            doc_sync_first_poll_exec_ms: Some(snapshot.doc_sync_first_poll_exec_ms),
            doc_sync_first_poll_outcome: Some(snapshot.doc_sync_first_poll_outcome.clone()),
            doc_sync_first_poll_method: Some(snapshot.owner_method.clone()),
            doc_sync_first_poll_uri: snapshot.owner_uri.clone(),
            doc_sync_first_poll_version: snapshot.owner_version,
            ..Default::default()
        },
    );
}

fn oldest_pending_completion_position(
    pending_completion_requests: &std::collections::VecDeque<ScheduledRequest>,
) -> Option<usize> {
    pending_completion_requests
        .iter()
        .position(|scheduled| is_completion_request(&scheduled.request))
}

fn oldest_pending_notification_position_by_uri(
    pending_completion_requests: &std::collections::VecDeque<ScheduledRequest>,
    uri: &str,
) -> Option<usize> {
    pending_completion_requests.iter().position(|scheduled| {
        is_completion_supporting_document_sync_notification(&scheduled.request)
            && request_text_document_uri(&scheduled.request) == Some(uri)
    })
}

fn oldest_pending_notification_position_with_different_uri(
    pending_completion_requests: &std::collections::VecDeque<ScheduledRequest>,
    incoming_uri: Option<&str>,
) -> Option<usize> {
    pending_completion_requests.iter().position(|scheduled| {
        is_completion_supporting_document_sync_notification(&scheduled.request)
            && request_text_document_uri(&scheduled.request) != incoming_uri
    })
}

async fn respond_to_pre_dispatch_cancelled_completion(
    cancelled_request: ScheduledRequest,
    request_id: &str,
    responses_tx: &mut mpsc::Sender<QueuedTransportMessage>,
) -> Result<(), ()> {
    if let Some(context) =
        super::request_context::take_completion_request_context_by_request_id(request_id)
    {
        super::request_context::notify_pre_dispatch_completion_cancelled(
            context,
            super::unix_timestamp_ms(),
        );
    }

    let Some(response) = cancelled_request
        .request
        .id()
        .cloned()
        .map(|id| Response::from_error(id, Error::request_cancelled()))
    else {
        return Ok(());
    };

    if responses_tx
        .send(QueuedTransportMessage::response(response))
        .await
        .is_err()
    {
        error!("failed to forward queued pre-dispatch cancellation response: transport closed");
        return Err(());
    }
    Ok(())
}

async fn respond_to_pre_dispatch_rejected_completion(
    rejected_request: ScheduledRequest,
    request_id: &str,
    responses_tx: &mut mpsc::Sender<QueuedTransportMessage>,
) -> Result<(), ()> {
    if let Some(context) =
        super::request_context::take_completion_request_context_by_request_id(request_id)
    {
        super::request_context::notify_pre_dispatch_completion_terminal_outcome(
            context,
            super::unix_timestamp_ms(),
            "queue_rejected",
        );
    }

    let Some(response) = rejected_request.request.id().cloned().map(|id| {
        Response::from_ok(
            id,
            serde_json::json!({
                "isIncomplete": true,
                "items": [],
            }),
        )
    }) else {
        return Ok(());
    };

    if responses_tx
        .send(QueuedTransportMessage::response(response))
        .await
        .is_err()
    {
        error!("failed to forward queued pre-dispatch rejection response: transport closed");
        return Err(());
    }
    Ok(())
}

async fn stage_completion_request_with_overflow_policy(
    pending_completion_requests: &mut std::collections::VecDeque<ScheduledRequest>,
    completion_spillover_capacity: usize,
    incoming_request: ScheduledRequest,
    responses_tx: &mut mpsc::Sender<QueuedTransportMessage>,
) -> Result<(), ()> {
    if pending_completion_requests.len() < completion_spillover_capacity {
        pending_completion_requests.push_back(incoming_request);
        return Ok(());
    }

    let incoming_uri =
        request_text_document_uri(&incoming_request.request).map(ToString::to_string);
    if is_completion_supporting_document_sync_notification(&incoming_request.request) {
        if let Some(position) = incoming_uri.as_deref().and_then(|uri| {
            oldest_pending_notification_position_by_uri(pending_completion_requests, uri)
        }) {
            let _ = pending_completion_requests.remove(position);
            pending_completion_requests.push_back(incoming_request);
            return Ok(());
        }

        if let Some(position) = oldest_pending_completion_position(pending_completion_requests) {
            let evicted_request = pending_completion_requests
                .remove(position)
                .expect("pending completion request by position");
            if let Some(request_id) = evicted_request.request_id.clone() {
                respond_to_pre_dispatch_rejected_completion(
                    evicted_request,
                    &request_id,
                    responses_tx,
                )
                .await?;
            }
            pending_completion_requests.push_back(incoming_request);
            return Ok(());
        }

        if let Some(position) = oldest_pending_notification_position_with_different_uri(
            pending_completion_requests,
            incoming_uri.as_deref(),
        ) {
            let dropped_request = pending_completion_requests
                .remove(position)
                .expect("pending completion notification by position");
            warn!(
                method = dropped_request.request.method(),
                "dropping older completion-supporting notification to preserve bounded completion admission"
            );
            pending_completion_requests.push_back(incoming_request);
            return Ok(());
        }

        let dropped_request = pending_completion_requests
            .pop_front()
            .expect("pending completion notification head");
        warn!(
            method = dropped_request.request.method(),
            "coalescing saturated completion-supporting notification backlog to newest handoff"
        );
        pending_completion_requests.push_back(incoming_request);
        return Ok(());
    }

    if let Some(position) = oldest_pending_completion_position(pending_completion_requests) {
        let evicted_request = pending_completion_requests
            .remove(position)
            .expect("pending completion request by position");
        if let Some(request_id) = evicted_request.request_id.clone() {
            respond_to_pre_dispatch_rejected_completion(evicted_request, &request_id, responses_tx)
                .await?;
        }
        pending_completion_requests.push_back(incoming_request);
        return Ok(());
    }

    if let Some(position) = oldest_pending_notification_position_with_different_uri(
        pending_completion_requests,
        incoming_uri.as_deref(),
    ) {
        let dropped_request = pending_completion_requests
            .remove(position)
            .expect("pending completion notification by position");
        warn!(
            method = dropped_request.request.method(),
            "dropping unrelated completion-supporting notification to preserve latest interactive completion"
        );
        pending_completion_requests.push_back(incoming_request);
        return Ok(());
    }

    if let Some(request_id) = incoming_request.request_id.clone() {
        respond_to_pre_dispatch_rejected_completion(incoming_request, &request_id, responses_tx)
            .await?;
    } else {
        warn!(
            method = incoming_request.request.method(),
            "dropping completion-priority notification after bounded spillover saturation"
        );
    }
    Ok(())
}

async fn cancel_queued_completion_before_dispatch(
    admission_queues: &AdmissionQueues,
    request_id: &str,
    responses_tx: &mut mpsc::Sender<QueuedTransportMessage>,
) -> Result<(), ()> {
    let Some(cancelled_request) = admission_queues
        .remove_queued_completion_by_request_id(request_id)
        .await
    else {
        return Ok(());
    };
    respond_to_pre_dispatch_cancelled_completion(cancelled_request, request_id, responses_tx).await
}

fn general_backpressure_error(method: &str) -> Error {
    Error {
        code: ErrorCode::ServerError(GENERAL_BACKPRESSURE_ERROR_CODE),
        message: format!("General admission queue saturated before dispatch for {method}").into(),
        data: None,
    }
}

async fn reject_saturated_general_request(
    responses_tx: &mut mpsc::Sender<QueuedTransportMessage>,
    scheduled_request: ScheduledRequest,
) -> Result<(), ()> {
    let Some(id) = scheduled_request.request.id().cloned() else {
        warn!(
            "general admission queue saturated before dispatch for notification {}",
            scheduled_request.request.method()
        );
        return Ok(());
    };
    let response = Response::from_error(
        id,
        general_backpressure_error(scheduled_request.request.method()),
    );
    if responses_tx
        .send(QueuedTransportMessage::response(response))
        .await
        .is_err()
    {
        error!("failed to forward general backpressure response: transport closed");
        return Err(());
    }
    Ok(())
}

async fn read_transport_message<I>(
    reader: &mut BufReader<I>,
) -> Result<Option<(TransportMessage, TransportMessageReadMilestones)>, TransportCodecError>
where
    I: AsyncRead + Unpin,
{
    let read_started_at_ms = super::unix_timestamp_ms();
    let mut content_length = None;

    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line).await?;

        if bytes == 0 {
            return if content_length.is_none() {
                Ok(None)
            } else {
                Err(TransportCodecError::UnexpectedEof)
            };
        }

        if line == "\r\n" {
            break;
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some(raw_len) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(raw_len.trim().parse::<usize>()?);
            continue;
        }

        if let Some(raw_content_type) = trimmed.strip_prefix("Content-Type:") {
            validate_content_type(raw_content_type.trim())?;
        }
    }

    let body_len = content_length.ok_or(TransportCodecError::MissingContentLength)?;
    let mut body = vec![0; body_len];
    reader.read_exact(&mut body).await?;
    let message = serde_json::from_slice(&body).map_err(TransportCodecError::Json)?;
    Ok(Some((
        message,
        TransportMessageReadMilestones {
            read_started_at_ms,
            parse_completed_at_ms: super::unix_timestamp_ms(),
        },
    )))
}

async fn resolve_completion_handoff_enqueued_at_ms(
    handoff_enqueued_at_ms: Option<&Arc<AtomicU64>>,
    fallback_at_ms: u64,
) -> u64 {
    let Some(handoff_enqueued_at_ms) = handoff_enqueued_at_ms else {
        return fallback_at_ms;
    };

    for _ in 0..4 {
        let value = handoff_enqueued_at_ms.load(Ordering::Relaxed);
        if value > 0 {
            return value;
        }
        tokio::task::yield_now().await;
    }

    fallback_at_ms
}

async fn write_transport_message<O>(
    writer: &mut BufWriter<O>,
    message: &TransportMessage,
) -> Result<TransportMessageWriteMilestones, TransportCodecError>
where
    O: AsyncWrite + Unpin,
{
    let encode_started_at_ms = super::unix_timestamp_ms();
    let body = serde_json::to_vec(message).map_err(TransportCodecError::Json)?;
    let encode_completed_at_ms = super::unix_timestamp_ms();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let write_started_at_ms = super::unix_timestamp_ms();
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;
    Ok(TransportMessageWriteMilestones {
        encode_started_at_ms,
        write_started_at_ms,
        encode_completed_at_ms,
        flush_completed_at_ms: super::unix_timestamp_ms(),
    })
}

fn validate_content_type(content_type: &str) -> Result<(), TransportCodecError> {
    let charset = content_type
        .split(';')
        .skip(1)
        .map(str::trim)
        .find_map(|param| param.strip_prefix("charset="));

    match charset {
        Some("utf-8") | Some("utf8") | None => Ok(()),
        Some(_) => Err(TransportCodecError::InvalidContentType),
    }
}

fn display_sources(error: &dyn std::error::Error) -> String {
    if let Some(source) = error.source() {
        format!("{error}: {}", display_sources(source))
    } else {
        error.to_string()
    }
}

fn to_jsonrpc_error(err: &TransportCodecError) -> Error {
    match err {
        TransportCodecError::Json(err) if err.is_data() => Error::invalid_request(),
        TransportCodecError::Json(_)
        | TransportCodecError::Io(_)
        | TransportCodecError::InvalidContentLength(_)
        | TransportCodecError::InvalidContentType
        | TransportCodecError::MissingContentLength
        | TransportCodecError::UnexpectedEof
        | TransportCodecError::Utf8(_) => Error::parse_error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::sync::Notify;
    use tower_lsp::jsonrpc::Response as JsonRpcResponse;

    #[derive(Debug)]
    struct EchoService;

    impl Service<Request> for EchoService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let id = request.id().expect("request id").clone();
            Box::pin(async move {
                Ok(Some(JsonRpcResponse::from_ok(
                    id,
                    json!({ "capabilities": {} }),
                )))
            })
        }
    }

    #[derive(Debug, Clone)]
    struct BlockingCompletionService {
        completion_release: std::sync::Arc<Notify>,
    }

    impl Service<Request> for BlockingCompletionService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let request_id = request.id().expect("request id").clone();
            let method = request.method().to_string();
            let completion_release = self.completion_release.clone();
            Box::pin(async move {
                if method == "textDocument/completion" {
                    completion_release.notified().await;
                    return Ok(Some(JsonRpcResponse::from_ok(
                        request_id,
                        json!({ "items": [], "isIncomplete": false }),
                    )));
                }

                Ok(Some(JsonRpcResponse::from_ok(
                    request_id,
                    json!({ "capabilities": {} }),
                )))
            })
        }
    }

    #[derive(Debug, Default)]
    struct SameFileOverlapState {
        first_completion_release: Notify,
        completion_calls_by_key: Mutex<HashMap<String, usize>>,
    }

    #[derive(Debug, Clone)]
    struct SameFileOverlapService {
        state: Arc<SameFileOverlapState>,
    }

    impl Service<Request> for SameFileOverlapService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let request_id = request.id().expect("request id").clone();
            let method = request.method().to_string();
            let params = request.params().cloned();
            let state = self.state.clone();
            Box::pin(async move {
                if method != "textDocument/completion" {
                    return Ok(Some(JsonRpcResponse::from_ok(
                        request_id,
                        json!({ "capabilities": {} }),
                    )));
                }

                let overlap_key = params
                    .as_ref()
                    .and_then(|value| {
                        Some(format!(
                            "{}:{}:{}",
                            value.get("textDocument")?.get("uri")?.as_str()?,
                            value.get("position")?.get("line")?.as_u64()?,
                            value.get("position")?.get("character")?.as_u64()?,
                        ))
                    })
                    .expect("same-file completion params");
                let call_index = {
                    let mut calls = state
                        .completion_calls_by_key
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    let entry = calls.entry(overlap_key).or_insert(0);
                    *entry += 1;
                    *entry
                };

                if call_index == 1 {
                    state.first_completion_release.notified().await;
                    return Ok(Some(JsonRpcResponse::from_ok(
                        request_id,
                        json!({ "items": [{ "label": "older" }], "isIncomplete": false }),
                    )));
                }

                Ok(Some(JsonRpcResponse::from_ok(
                    request_id,
                    json!({ "items": [{ "label": "newer" }], "isIncomplete": false }),
                )))
            })
        }
    }

    #[derive(Debug, Default)]
    struct CancellableCompletionState {
        completion_release: Notify,
        registered: Notify,
        pending_cancellations: Mutex<HashMap<String, Arc<Notify>>>,
    }

    #[derive(Debug, Clone)]
    struct CancellableCompletionService {
        state: Arc<CancellableCompletionState>,
    }

    impl Service<Request> for CancellableCompletionService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let request_id = request.id().cloned();
            let method = request.method().to_string();
            let params = request.params().cloned();
            let state = self.state.clone();
            Box::pin(async move {
                match method.as_str() {
                    "textDocument/completion" => {
                        let request_id = request_id.expect("completion request id");
                        let request_id_text = request_id.to_string();
                        let cancel_notify = Arc::new(Notify::new());
                        {
                            let mut pending = state
                                .pending_cancellations
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                            pending.insert(request_id_text.clone(), cancel_notify.clone());
                        }
                        state.registered.notify_waiters();

                        let response = tokio::select! {
                            _ = cancel_notify.notified() => {
                                JsonRpcResponse::from_error(
                                    request_id.clone(),
                                    Error::request_cancelled(),
                                )
                            }
                            _ = state.completion_release.notified() => {
                                JsonRpcResponse::from_ok(
                                    request_id.clone(),
                                    json!({ "items": [{ "label": "released" }], "isIncomplete": false }),
                                )
                            }
                        };

                        let mut pending = state
                            .pending_cancellations
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        pending.remove(&request_id_text);

                        Ok(Some(response))
                    }
                    "$/cancelRequest" => {
                        let cancelled_request_id = params
                            .as_ref()
                            .and_then(|value| value.get("id"))
                            .and_then(|value| {
                                value
                                    .as_i64()
                                    .map(|id| id.to_string())
                                    .or_else(|| value.as_str().map(ToString::to_string))
                            })
                            .expect("cancel request id");
                        let notify = {
                            let pending = state
                                .pending_cancellations
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                            pending.get(&cancelled_request_id).cloned()
                        };
                        if let Some(notify) = notify {
                            notify.notify_waiters();
                        }
                        Ok(None)
                    }
                    _ => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("request id"),
                        json!({ "capabilities": {} }),
                    ))),
                }
            })
        }
    }

    struct NullLoopback;

    impl Loopback for NullLoopback {
        type RequestStream = futures::stream::Pending<Request>;
        type ResponseSink = futures::sink::Drain<Response>;

        fn split(self) -> (Self::RequestStream, Self::ResponseSink) {
            (futures::stream::pending(), futures::sink::drain())
        }
    }

    #[derive(Debug, Default)]
    struct PrioritySchedulerState {
        general_release: Notify,
        general_started: Notify,
        ready_waker: Mutex<Option<std::task::Waker>>,
        general_inflight: AtomicBool,
        completion_call_count: AtomicUsize,
    }

    #[derive(Debug, Clone)]
    struct PrioritySchedulerService {
        state: Arc<PrioritySchedulerState>,
    }

    impl Service<Request> for PrioritySchedulerService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            if self.state.general_inflight.load(Ordering::SeqCst) {
                let mut ready_waker = self
                    .state
                    .ready_waker
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *ready_waker = Some(cx.waker().clone());
                Poll::Pending
            } else {
                Poll::Ready(Ok(()))
            }
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let request_id = request.id().cloned();
            let method = request.method().to_string();
            let state = self.state.clone();
            Box::pin(async move {
                match method.as_str() {
                    "textDocument/documentSymbol" => {
                        state.general_inflight.store(true, Ordering::SeqCst);
                        state.general_started.notify_waiters();
                        state.general_release.notified().await;
                        state.general_inflight.store(false, Ordering::SeqCst);
                        if let Some(waker) = state
                            .ready_waker
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .take()
                        {
                            waker.wake();
                        }
                        Ok(Some(JsonRpcResponse::from_ok(
                            request_id.expect("documentSymbol request id"),
                            json!({ "kind": "general" }),
                        )))
                    }
                    "textDocument/completion" => {
                        state.completion_call_count.fetch_add(1, Ordering::SeqCst);
                        Ok(Some(JsonRpcResponse::from_ok(
                            request_id.expect("completion request id"),
                            json!({ "items": [{ "label": "priority" }], "isIncomplete": false }),
                        )))
                    }
                    CANCEL_REQUEST_METHOD => Ok(None),
                    _ => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("request id"),
                        json!({ "capabilities": {} }),
                    ))),
                }
            })
        }
    }

    #[derive(Debug, Default)]
    struct GeneralBackpressureState {
        ready_release: AtomicBool,
        ready_blocked: Notify,
        ready_waker: Mutex<Option<std::task::Waker>>,
        call_order: Mutex<Vec<i64>>,
    }

    #[derive(Debug, Clone)]
    struct GeneralBackpressureService {
        state: Arc<GeneralBackpressureState>,
    }

    impl Service<Request> for GeneralBackpressureService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            if self.state.ready_release.load(Ordering::SeqCst) {
                Poll::Ready(Ok(()))
            } else {
                let mut ready_waker = self
                    .state
                    .ready_waker
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *ready_waker = Some(cx.waker().clone());
                self.state.ready_blocked.notify_waiters();
                Poll::Pending
            }
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let request_id = request.id().cloned();
            let method = request.method().to_string();
            let state = self.state.clone();
            let numeric_request_id = request_id
                .as_ref()
                .and_then(|id| match id {
                    Id::Number(value) => Some(*value),
                    _ => None,
                })
                .expect("numeric request id");
            state
                .call_order
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(numeric_request_id);
            Box::pin(async move {
                match method.as_str() {
                    "textDocument/completion" => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("completion request id"),
                        json!({ "items": [{ "label": "priority" }], "isIncomplete": false }),
                    ))),
                    "textDocument/documentSymbol" => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("documentSymbol request id"),
                        json!({ "kind": "general" }),
                    ))),
                    _ => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("request id"),
                        json!({ "capabilities": {} }),
                    ))),
                }
            })
        }
    }

    #[derive(Debug, Default)]
    struct CompletionIngressBackpressureState {
        ready_release: AtomicBool,
        ready_blocked: Notify,
        ready_waker: Mutex<Option<std::task::Waker>>,
        call_order: Mutex<Vec<i64>>,
    }

    #[derive(Debug, Clone)]
    struct CompletionIngressBackpressureService {
        state: Arc<CompletionIngressBackpressureState>,
    }

    impl Service<Request> for CompletionIngressBackpressureService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            if self.state.ready_release.load(Ordering::SeqCst) {
                Poll::Ready(Ok(()))
            } else {
                let mut ready_waker = self
                    .state
                    .ready_waker
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *ready_waker = Some(cx.waker().clone());
                self.state.ready_blocked.notify_waiters();
                Poll::Pending
            }
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let request_id = request.id().cloned();
            let method = request.method().to_string();
            let state = self.state.clone();
            Box::pin(async move {
                match method.as_str() {
                    "textDocument/completion" => {
                        let numeric_request_id = request_id
                            .as_ref()
                            .and_then(|id| match id {
                                Id::Number(value) => Some(*value),
                                _ => None,
                            })
                            .expect("numeric completion request id");
                        state
                            .call_order
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .push(numeric_request_id);
                        Ok(Some(JsonRpcResponse::from_ok(
                            request_id.expect("completion request id"),
                            json!({ "items": [{ "label": "priority" }], "isIncomplete": false }),
                        )))
                    }
                    CANCEL_REQUEST_METHOD => Ok(None),
                    _ => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("request id"),
                        json!({ "capabilities": {} }),
                    ))),
                }
            })
        }
    }

    #[derive(Debug, Default)]
    struct DocumentSyncPriorityState {
        ready_release: AtomicBool,
        ready_blocked: Notify,
        ready_waker: Mutex<Option<std::task::Waker>>,
        latest_version: Mutex<i64>,
        call_order: Mutex<Vec<String>>,
    }

    #[derive(Debug, Clone)]
    struct DocumentSyncPriorityService {
        state: Arc<DocumentSyncPriorityState>,
    }

    impl Service<Request> for DocumentSyncPriorityService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            if self.state.ready_release.load(Ordering::SeqCst) {
                Poll::Ready(Ok(()))
            } else {
                let mut ready_waker = self
                    .state
                    .ready_waker
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *ready_waker = Some(cx.waker().clone());
                self.state.ready_blocked.notify_waiters();
                Poll::Pending
            }
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let request_id = request.id().cloned();
            let method = request.method().to_string();
            let params = request.params().cloned();
            let state = self.state.clone();
            Box::pin(async move {
                state
                    .call_order
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(method.clone());
                match method.as_str() {
                    "textDocument/documentSymbol" => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("documentSymbol request id"),
                        json!({ "kind": "general" }),
                    ))),
                    "textDocument/didChange" => {
                        let version = params
                            .as_ref()
                            .and_then(|value| value.get("textDocument"))
                            .and_then(|value| value.get("version"))
                            .and_then(|value| value.as_i64())
                            .expect("didChange version");
                        *state
                            .latest_version
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner()) = version;
                        Ok(None)
                    }
                    "textDocument/completion" => {
                        let latest_version = *state
                            .latest_version
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        Ok(Some(JsonRpcResponse::from_ok(
                            request_id.expect("completion request id"),
                            json!({
                                "items": [{ "label": format!("version-{latest_version}") }],
                                "isIncomplete": false,
                                "version": latest_version,
                            }),
                        )))
                    }
                    _ => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("request id"),
                        json!({ "capabilities": {} }),
                    ))),
                }
            })
        }
    }

    #[derive(Debug, Default)]
    struct SameFileTokenPriorityState {
        ready_release: AtomicBool,
        ready_blocked: Notify,
        ready_waker: Mutex<Option<std::task::Waker>>,
        did_change_started: AtomicBool,
        did_change_started_notify: Notify,
        did_change_release: AtomicBool,
        did_change_release_notify: Notify,
    }

    #[derive(Debug, Clone)]
    struct SameFileTokenPriorityService {
        state: Arc<SameFileTokenPriorityState>,
    }

    impl Service<Request> for SameFileTokenPriorityService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            if self.state.ready_release.load(Ordering::SeqCst) {
                Poll::Ready(Ok(()))
            } else {
                let mut ready_waker = self
                    .state
                    .ready_waker
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *ready_waker = Some(cx.waker().clone());
                self.state.ready_blocked.notify_waiters();
                Poll::Pending
            }
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let request_id = request.id().cloned();
            let method = request.method().to_string();
            let state = self.state.clone();
            Box::pin(async move {
                match method.as_str() {
                    "textDocument/didChange" => {
                        state.did_change_started.store(true, Ordering::SeqCst);
                        state.did_change_started_notify.notify_waiters();
                        while !state.did_change_release.load(Ordering::SeqCst) {
                            state.did_change_release_notify.notified().await;
                        }
                        Ok(None)
                    }
                    "textDocument/didSave" => Ok(None),
                    COMPLETION_METHOD => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("completion request id"),
                        json!({
                            "items": [{ "label": "token-ready" }],
                            "isIncomplete": false,
                        }),
                    ))),
                    _ => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("request id"),
                        json!({ "capabilities": {} }),
                    ))),
                }
            })
        }
    }

    #[derive(Debug, Default)]
    struct SchedulerIsolationState {
        scheduler_release: AtomicBool,
        scheduler_blocked: Notify,
        ready_waker: Mutex<Option<std::task::Waker>>,
        first_dispatch_completed: AtomicBool,
        completion_call_count: AtomicUsize,
    }

    #[derive(Debug, Clone)]
    struct SchedulerIsolationService {
        state: Arc<SchedulerIsolationState>,
    }

    impl Service<Request> for SchedulerIsolationService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            if self.state.first_dispatch_completed.load(Ordering::SeqCst)
                && !self.state.scheduler_release.load(Ordering::SeqCst)
            {
                std::thread::sleep(std::time::Duration::from_millis(120));
                let mut ready_waker = self
                    .state
                    .ready_waker
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *ready_waker = Some(cx.waker().clone());
                self.state.scheduler_blocked.notify_waiters();
                Poll::Pending
            } else {
                Poll::Ready(Ok(()))
            }
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let request_id = request.id().cloned();
            let method = request.method().to_string();
            let state = self.state.clone();
            if method == "textDocument/documentSymbol" {
                state.first_dispatch_completed.store(true, Ordering::SeqCst);
            }
            Box::pin(async move {
                match method.as_str() {
                    "textDocument/documentSymbol" => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("documentSymbol request id"),
                        json!({ "kind": "general" }),
                    ))),
                    "textDocument/completion" => {
                        state.completion_call_count.fetch_add(1, Ordering::SeqCst);
                        Ok(Some(JsonRpcResponse::from_ok(
                            request_id.expect("completion request id"),
                            json!({ "items": [{ "label": "unexpected" }], "isIncomplete": false }),
                        )))
                    }
                    CANCEL_REQUEST_METHOD => Ok(None),
                    _ => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("request id"),
                        json!({ "capabilities": {} }),
                    ))),
                }
            })
        }
    }

    #[derive(Debug, Default)]
    struct CompletionBarrierBackpressureState {
        ready_release: AtomicBool,
        ready_blocked: Notify,
        ready_waker: Mutex<Option<std::task::Waker>>,
        latest_version: Mutex<i64>,
        call_order: Mutex<Vec<String>>,
    }

    #[derive(Debug, Clone)]
    struct CompletionBarrierBackpressureService {
        state: Arc<CompletionBarrierBackpressureState>,
    }

    impl Service<Request> for CompletionBarrierBackpressureService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            if self.state.ready_release.load(Ordering::SeqCst) {
                Poll::Ready(Ok(()))
            } else {
                let mut ready_waker = self
                    .state
                    .ready_waker
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *ready_waker = Some(cx.waker().clone());
                self.state.ready_blocked.notify_waiters();
                Poll::Pending
            }
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let request_id = request.id().cloned();
            let method = request.method().to_string();
            let params = request.params().cloned();
            let state = self.state.clone();
            Box::pin(async move {
                state
                    .call_order
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(method.clone());
                match method.as_str() {
                    "textDocument/didChange" => {
                        let version = params
                            .as_ref()
                            .and_then(|value| value.get("textDocument"))
                            .and_then(|value| value.get("version"))
                            .and_then(|value| value.as_i64())
                            .expect("didChange version");
                        *state
                            .latest_version
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner()) = version;
                        Ok(None)
                    }
                    "textDocument/completion" => {
                        let latest_version = *state
                            .latest_version
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        Ok(Some(JsonRpcResponse::from_ok(
                            request_id.expect("completion request id"),
                            json!({
                                "items": [{ "label": format!("version-{latest_version}") }],
                                "isIncomplete": false,
                                "version": latest_version,
                            }),
                        )))
                    }
                    CANCEL_REQUEST_METHOD => Ok(None),
                    _ => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("request id"),
                        json!({ "capabilities": {} }),
                    ))),
                }
            })
        }
    }

    #[derive(Debug, Default)]
    struct DocumentSyncBarrierBypassState {
        did_open_started: Notify,
        did_open_release: Notify,
        latest_version: Mutex<i64>,
        call_order: Mutex<Vec<String>>,
    }

    #[derive(Debug, Clone)]
    struct DocumentSyncBarrierBypassService {
        state: Arc<DocumentSyncBarrierBypassState>,
    }

    impl Service<Request> for DocumentSyncBarrierBypassService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let request_id = request.id().cloned();
            let method = request.method().to_string();
            let params = request.params().cloned();
            let state = self.state.clone();
            Box::pin(async move {
                state
                    .call_order
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(method.clone());
                match method.as_str() {
                    DID_OPEN_METHOD => {
                        let version = params
                            .as_ref()
                            .and_then(|value| value.get("textDocument"))
                            .and_then(|value| value.get("version"))
                            .and_then(|value| value.as_i64())
                            .expect("didOpen version");
                        *state
                            .latest_version
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner()) = version;
                        state.did_open_started.notify_waiters();
                        state.did_open_release.notified().await;
                        Ok(None)
                    }
                    "textDocument/hover" => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("hover request id"),
                        json!({
                            "contents": {
                                "kind": "markdown",
                                "value": "hover-ready"
                            }
                        }),
                    ))),
                    "textDocument/completion" => {
                        let latest_version = *state
                            .latest_version
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        Ok(Some(JsonRpcResponse::from_ok(
                            request_id.expect("completion request id"),
                            json!({
                                "items": [{ "label": format!("version-{latest_version}") }],
                                "isIncomplete": false,
                                "version": latest_version,
                            }),
                        )))
                    }
                    _ => Ok(Some(JsonRpcResponse::from_ok(
                        request_id.expect("request id"),
                        json!({ "capabilities": {} }),
                    ))),
                }
            })
        }
    }

    async fn read_framed_message(
        reader: &mut BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    ) -> serde_json::Value {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            let bytes = reader
                .read_line(&mut line)
                .await
                .expect("read response header line");
            assert!(bytes > 0, "unexpected EOF while reading response header");
            if line == "\r\n" {
                break;
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if let Some(raw_len) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(
                    raw_len
                        .trim()
                        .parse::<usize>()
                        .expect("parse response content length"),
                );
            }
        }
        let body_len = content_length.expect("response content length");
        let mut body = vec![0; body_len];
        reader
            .read_exact(&mut body)
            .await
            .expect("read response body");
        serde_json::from_slice(&body).expect("parse response json")
    }

    #[tokio::test]
    async fn transport_adapter_forwards_jsonrpc_response_over_stdio() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);

        let server_task = tokio::spawn(async move {
            serve_with_completion_handoff(server_read, server_write, NullLoopback, EchoService, 2)
                .await;
        });

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let body = serde_json::to_vec(&request).expect("serialize request");
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        client_write
            .write_all(header.as_bytes())
            .await
            .expect("write request header");
        client_write
            .write_all(&body)
            .await
            .expect("write request body");
        client_write.flush().await.expect("flush request");

        let mut reader = BufReader::new(client_read);
        let response = read_framed_message(&mut reader).await;
        assert_eq!(response.get("id").and_then(|value| value.as_i64()), Some(1));
        assert_eq!(
            response
                .get("result")
                .and_then(|value| value.get("capabilities"))
                .and_then(|value| value.as_object())
                .map(|map| map.is_empty()),
            Some(true)
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_releases_ingress_slot_before_blocking_completion_wait() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let completion_release = std::sync::Arc::new(Notify::new());

        let server_task = tokio::spawn({
            let completion_release = completion_release.clone();
            async move {
                serve_with_completion_handoff(
                    server_read,
                    server_write,
                    NullLoopback,
                    BlockingCompletionService { completion_release },
                    1,
                )
                .await;
            }
        });

        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///test.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "initialize",
                "params": {}
            }),
        ] {
            let body = serde_json::to_vec(&request).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        let mut reader = BufReader::new(client_read);
        let first_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("initialize response must not wait for completion release");
        assert_eq!(
            first_response.get("id").and_then(|value| value.as_i64()),
            Some(2)
        );

        completion_release.notify_waiters();
        let second_response = read_framed_message(&mut reader).await;
        assert_eq!(
            second_response.get("id").and_then(|value| value.as_i64()),
            Some(1)
        );
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(50),
                read_framed_message(&mut reader),
            )
            .await
            .is_err(),
            "completion handoff must not emit duplicate terminal responses"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_keeps_same_file_overlap_off_transport_after_handoff() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let overlap_state = Arc::new(SameFileOverlapState::default());

        let server_task = tokio::spawn({
            let overlap_state = overlap_state.clone();
            async move {
                serve_with_completion_handoff(
                    server_read,
                    server_write,
                    NullLoopback,
                    SameFileOverlapService {
                        state: overlap_state,
                    },
                    1,
                )
                .await;
            }
        });

        for request_id in [1_i64, 2_i64] {
            let request = json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///same-file.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            });
            let body = serde_json::to_vec(&request).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        let mut reader = BufReader::new(client_read);
        let newer_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("newer same-file completion must not wait behind older handoff owner");
        assert_eq!(
            newer_response.get("id").and_then(|value| value.as_i64()),
            Some(2)
        );

        overlap_state.first_completion_release.notify_waiters();
        let older_response = read_framed_message(&mut reader).await;
        assert_eq!(
            older_response.get("id").and_then(|value| value.as_i64()),
            Some(1)
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_emits_single_terminal_response_for_handoff_cancel_race() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let cancel_state = Arc::new(CancellableCompletionState::default());

        let server_task = tokio::spawn({
            let cancel_state = cancel_state.clone();
            async move {
                serve_with_completion_handoff(
                    server_read,
                    server_write,
                    NullLoopback,
                    CancellableCompletionService {
                        state: cancel_state,
                    },
                    1,
                )
                .await;
            }
        });

        let completion_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": "file:///cancel-race.bsl" },
                "position": { "line": 0, "character": 0 }
            }
        });
        let completion_body =
            serde_json::to_vec(&completion_request).expect("serialize completion request");
        let completion_header = format!("Content-Length: {}\r\n\r\n", completion_body.len());
        client_write
            .write_all(completion_header.as_bytes())
            .await
            .expect("write completion request header");
        client_write
            .write_all(&completion_body)
            .await
            .expect("write completion request body");
        client_write
            .flush()
            .await
            .expect("flush completion request");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            loop {
                let is_registered = cancel_state
                    .pending_cancellations
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .contains_key("1");
                if is_registered {
                    break;
                }
                cancel_state.registered.notified().await;
            }
        })
        .await
        .expect("completion handoff owner must register cancellable request");

        let cancel_request = json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": { "id": 1 }
        });
        let cancel_body = serde_json::to_vec(&cancel_request).expect("serialize cancel request");
        let cancel_header = format!("Content-Length: {}\r\n\r\n", cancel_body.len());
        client_write
            .write_all(cancel_header.as_bytes())
            .await
            .expect("write cancel request header");
        client_write
            .write_all(&cancel_body)
            .await
            .expect("write cancel request body");
        client_write.flush().await.expect("flush cancel request");

        let mut reader = BufReader::new(client_read);
        let cancelled_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("cancelled completion must resolve promptly after handoff");
        assert_eq!(
            cancelled_response
                .get("id")
                .and_then(|value| value.as_i64()),
            Some(1)
        );
        assert_eq!(
            cancelled_response
                .get("error")
                .and_then(|value| value.get("code"))
                .and_then(|value| value.as_i64()),
            Some(-32800)
        );

        cancel_state.completion_release.notify_waiters();
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(50),
                read_framed_message(&mut reader),
            )
            .await
            .is_err(),
            "cancelled handoff owner must not emit a second terminal response after release"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_aborts_blocked_completion_handoff_on_transport_shutdown() {
        let (mut client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let completion_release = std::sync::Arc::new(Notify::new());

        let server_task = tokio::spawn({
            let completion_release = completion_release.clone();
            async move {
                serve_with_completion_handoff(
                    server_read,
                    server_write,
                    NullLoopback,
                    BlockingCompletionService { completion_release },
                    1,
                )
                .await;
            }
        });

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": "file:///test.bsl" },
                "position": { "line": 0, "character": 0 }
            }
        });
        let body = serde_json::to_vec(&request).expect("serialize request");
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        client_stream
            .write_all(header.as_bytes())
            .await
            .expect("write request header");
        client_stream
            .write_all(&body)
            .await
            .expect("write request body");
        client_stream.flush().await.expect("flush request");

        drop(client_stream);
        tokio::time::timeout(std::time::Duration::from_secs(1), server_task)
            .await
            .expect("transport shutdown must abort blocked completion handoff")
            .expect("server task must exit cleanly");
    }

    #[tokio::test]
    async fn transport_adapter_prioritises_completion_over_general_backlog_after_read() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let priority_state = Arc::new(PrioritySchedulerState::default());

        let server_task = tokio::spawn({
            let priority_state = priority_state.clone();
            async move {
                serve_with_completion_handoff(
                    server_read,
                    server_write,
                    NullLoopback,
                    PrioritySchedulerService {
                        state: priority_state,
                    },
                    1,
                )
                .await;
            }
        });

        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": "file:///priority-general-1.bsl" }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": "file:///priority-general-2.bsl" }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///priority-completion.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
        ] {
            let body = serde_json::to_vec(&request).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while !priority_state.general_inflight.load(Ordering::SeqCst) {
                priority_state.general_started.notified().await;
            }
        })
        .await
        .expect("first general request must hold service readiness");

        priority_state.general_release.notify_waiters();

        let mut reader = BufReader::new(client_read);
        let first_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("first priority response timeout");

        let second_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("second priority response timeout");

        let first_two_ids = [
            first_response.get("id").and_then(|value| value.as_i64()),
            second_response.get("id").and_then(|value| value.as_i64()),
        ];
        assert!(
            first_two_ids.contains(&Some(1)) && first_two_ids.contains(&Some(3)),
            "completion must outrun queued general backlog, first_response={first_response:?}, second_response={second_response:?}"
        );

        priority_state.general_release.notify_waiters();
        let third_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("second general response timeout");
        assert_eq!(
            third_response.get("id").and_then(|value| value.as_i64()),
            Some(2)
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_keeps_completion_observability_execute_commands_in_general_lane() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let priority_state = Arc::new(PrioritySchedulerState::default());

        let server_task = tokio::spawn({
            let priority_state = priority_state.clone();
            async move {
                serve_with_completion_handoff(
                    server_read,
                    server_write,
                    NullLoopback,
                    PrioritySchedulerService {
                        state: priority_state,
                    },
                    1,
                )
                .await;
            }
        });

        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": "file:///priority-general-1.bsl" }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///priority-completion.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "workspace/executeCommand",
                "params": {
                    "command": "bsl.getCompletionTimeline",
                    "arguments": [{ "limit": 10 }]
                }
            }),
        ] {
            let body = serde_json::to_vec(&request).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while !priority_state.general_inflight.load(Ordering::SeqCst) {
                priority_state.general_started.notified().await;
            }
        })
        .await
        .expect("first general request must hold service readiness");

        priority_state.general_release.notify_waiters();

        let mut reader = BufReader::new(client_read);
        let first_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("first response timeout");
        let second_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("second response timeout");
        let third_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("third response timeout");

        let first_two_ids = [
            first_response.get("id").and_then(|value| value.as_i64()),
            second_response.get("id").and_then(|value| value.as_i64()),
        ];
        assert!(
            first_two_ids.contains(&Some(1)) && first_two_ids.contains(&Some(2)),
            "completion must outrun queued general executeCommand backlog, first={first_response:?}, second={second_response:?}"
        );
        assert_eq!(
            third_response.get("id").and_then(|value| value.as_i64()),
            Some(3)
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_preserves_completion_progress_when_general_lane_is_saturated() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let backpressure_state = Arc::new(GeneralBackpressureState::default());
        let admission_queues = AdmissionQueues::new(AdmissionQueueCapacities {
            control: 2,
            completion: 2,
            general: 1,
        });

        let server_task = tokio::spawn({
            let backpressure_state = backpressure_state.clone();
            let admission_queues = admission_queues.clone();
            async move {
                serve_with_completion_handoff_with_admission_queues(
                    server_read,
                    server_write,
                    NullLoopback,
                    GeneralBackpressureService {
                        state: backpressure_state,
                    },
                    1,
                    admission_queues,
                )
                .await;
            }
        });

        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": "file:///saturated-general-1.bsl" }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": "file:///saturated-general-2.bsl" }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///saturated-completion.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": "file:///saturated-general-3.bsl" }
                }
            }),
        ] {
            let body = serde_json::to_vec(&request).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while backpressure_state
                .ready_waker
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_none()
            {
                backpressure_state.ready_blocked.notified().await;
            }
        })
        .await
        .expect("scheduler must block on general readiness gate");

        let mut reader = BufReader::new(client_read);
        let saturated_general_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect(
            "bounded general backpressure must respond without waiting for scheduler readiness",
        );
        assert_eq!(
            saturated_general_response
                .get("id")
                .and_then(|value| value.as_i64()),
            Some(3)
        );
        assert_eq!(
            saturated_general_response
                .get("error")
                .and_then(|value| value.get("code"))
                .and_then(|value| value.as_i64()),
            Some(GENERAL_BACKPRESSURE_ERROR_CODE)
        );

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            loop {
                if admission_queues.lane_depth(AdmissionLane::Completion) == 1
                    && admission_queues.lane_depth(AdmissionLane::General) == 1
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("completion request must be read and staged before releasing scheduler readiness");

        backpressure_state
            .ready_release
            .store(true, Ordering::SeqCst);
        if let Some(waker) = backpressure_state
            .ready_waker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            waker.wake();
        }

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            loop {
                if backpressure_state
                    .call_order
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .len()
                    >= 2
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("completion and oldest queued general request must both reach dispatch");

        let dispatch_order = backpressure_state
            .call_order
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        assert_eq!(
            dispatch_order.first().copied(),
            Some(4),
            "completion must reach dispatch before queued general backlog once readiness is released, dispatch_order={dispatch_order:?}"
        );

        let first_remaining_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("first post-release response timeout");
        let second_remaining_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("second post-release response timeout");
        let third_remaining_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("third post-release response timeout");
        let trailing_ids = [
            first_remaining_response
                .get("id")
                .and_then(|value| value.as_i64()),
            second_remaining_response
                .get("id")
                .and_then(|value| value.as_i64()),
            third_remaining_response
                .get("id")
                .and_then(|value| value.as_i64()),
        ];
        assert!(
            trailing_ids.contains(&Some(1))
                && trailing_ids.contains(&Some(2))
                && trailing_ids.contains(&Some(4)),
            "bounded backpressure and interactive completion must still yield terminal responses for ids 1, 2 and 4, responses={trailing_ids:?}"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_keeps_running_when_saturated_unrelated_general_notification_is_dropped(
    ) {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let backpressure_state = Arc::new(GeneralBackpressureState::default());
        let admission_queues = AdmissionQueues::new(AdmissionQueueCapacities {
            control: 2,
            completion: 2,
            general: 1,
        });

        let server_task = tokio::spawn({
            let backpressure_state = backpressure_state.clone();
            let admission_queues = admission_queues.clone();
            async move {
                serve_with_completion_handoff_with_admission_queues(
                    server_read,
                    server_write,
                    NullLoopback,
                    GeneralBackpressureService {
                        state: backpressure_state,
                    },
                    1,
                    admission_queues,
                )
                .await;
            }
        });

        for message in [
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": "file:///saturated-general-1.bsl" }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": "file:///saturated-general-2.bsl" }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///saturated-completion.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "workspace/didChangeConfiguration",
                "params": {
                    "settings": {
                        "bsl": { "serverTrace": "verbose" }
                    }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": "file:///saturated-general-3.bsl" }
                }
            }),
        ] {
            let body = serde_json::to_vec(&message).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while backpressure_state
                .ready_waker
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_none()
            {
                backpressure_state.ready_blocked.notified().await;
            }
        })
        .await
        .expect("scheduler must block on general readiness gate");

        let mut reader = BufReader::new(client_read);
        let overflow_request_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("transport must stay alive long enough to reject the next overflow request");
        assert_eq!(
            overflow_request_response
                .get("id")
                .and_then(|value| value.as_i64()),
            Some(5)
        );
        assert_eq!(
            overflow_request_response
                .get("error")
                .and_then(|value| value.get("code"))
                .and_then(|value| value.as_i64()),
            Some(GENERAL_BACKPRESSURE_ERROR_CODE)
        );

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            loop {
                if admission_queues.lane_depth(AdmissionLane::Completion) == 1
                    && admission_queues.lane_depth(AdmissionLane::General) == 1
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("completion request must remain queued after dropping saturated notification");

        backpressure_state
            .ready_release
            .store(true, Ordering::SeqCst);
        if let Some(waker) = backpressure_state
            .ready_waker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            waker.wake();
        }

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            loop {
                if backpressure_state
                    .call_order
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .len()
                    >= 2
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("completion and oldest queued general request must both reach dispatch");

        let dispatch_order = backpressure_state
            .call_order
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        assert_eq!(
            dispatch_order.first().copied(),
            Some(4),
            "completion must still reach dispatch before queued general backlog, dispatch_order={dispatch_order:?}"
        );

        let first_remaining_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("first post-release response timeout");
        let second_remaining_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("second post-release response timeout");
        let third_remaining_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("third post-release response timeout");
        let trailing_ids = [
            first_remaining_response
                .get("id")
                .and_then(|value| value.as_i64()),
            second_remaining_response
                .get("id")
                .and_then(|value| value.as_i64()),
            third_remaining_response
                .get("id")
                .and_then(|value| value.as_i64()),
        ];
        assert!(
            trailing_ids.contains(&Some(1))
                && trailing_ids.contains(&Some(2))
                && trailing_ids.contains(&Some(4)),
            "bounded notification drop must preserve terminal responses for ids 1, 2 and 4, responses={trailing_ids:?}"
        );

        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(50),
                read_framed_message(&mut reader),
            )
            .await
            .is_err(),
            "dropped general notifications must not fabricate transport responses"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_prioritises_did_change_handoff_before_completion_under_general_backlog(
    ) {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let priority_state = Arc::new(DocumentSyncPriorityState::default());
        *priority_state
            .latest_version
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = 1;

        let server_task = tokio::spawn({
            let priority_state = priority_state.clone();
            async move {
                serve_with_completion_handoff_with_admission_queues(
                    server_read,
                    server_write,
                    NullLoopback,
                    DocumentSyncPriorityService {
                        state: priority_state,
                    },
                    1,
                    AdmissionQueues::new(AdmissionQueueCapacities {
                        control: 2,
                        completion: 2,
                        general: 1,
                    }),
                )
                .await;
            }
        });

        for message in [
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": "file:///priority-general-1.bsl" }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": "file:///priority-general-2.bsl" }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": "file:///priority-completion.bsl", "version": 2 },
                    "contentChanges": [{ "text": "// v2" }]
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///priority-completion.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
        ] {
            let body = serde_json::to_vec(&message).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while priority_state
                .ready_waker
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_none()
            {
                priority_state.ready_blocked.notified().await;
            }
        })
        .await
        .expect("scheduler must block on the first queued general request");

        priority_state.ready_release.store(true, Ordering::SeqCst);
        if let Some(waker) = priority_state
            .ready_waker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            waker.wake();
        }

        let mut reader = BufReader::new(client_read);
        let first_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("first response timeout");
        let second_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("second response timeout");
        let third_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("third response timeout");

        let completion_response = [first_response, second_response, third_response]
            .into_iter()
            .find(|response| response.get("id").and_then(|value| value.as_i64()) == Some(4))
            .expect("completion response");
        assert_eq!(
            completion_response
                .get("result")
                .and_then(|value| value.get("version"))
                .and_then(|value| value.as_i64()),
            Some(2),
            "completion must observe the latest didChange handed off before it on the same saturated transport path, response={completion_response:?}"
        );

        let call_order = priority_state
            .call_order
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        assert!(
            call_order.len() >= 3,
            "service must observe general request, didChange and completion dispatches, call_order={call_order:?}"
        );
        let did_change_position = call_order
            .iter()
            .position(|method| method == "textDocument/didChange")
            .expect("didChange dispatch position");
        let completion_position = call_order
            .iter()
            .position(|method| method == "textDocument/completion")
            .expect("completion dispatch position");
        assert!(
            did_change_position < completion_position,
            "didChange handoff must dispatch before completion on the same backlog-affected transport path, call_order={call_order:?}"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_prefers_token_ready_completion_over_unrelated_same_priority_fifo() {
        let target_uri = "file:///token-ready-completion.bsl";
        let unrelated_uri = "file:///unrelated-same-priority-change.bsl";
        crate::server::request_context::clear_same_file_ingress_token_publication_for_uri(
            target_uri,
        );
        crate::server::request_context::record_same_file_ingress_token_publication_for_uri(
            target_uri,
            7,
            1_700_000_000_007,
            "did_change",
        );

        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let priority_state = Arc::new(SameFileTokenPriorityState::default());
        let admission_queues = AdmissionQueues::new(AdmissionQueueCapacities {
            control: 2,
            completion: 4,
            general: 1,
        });

        let server_task = tokio::spawn({
            let priority_state = priority_state.clone();
            let admission_queues = admission_queues.clone();
            async move {
                serve_with_completion_handoff_with_admission_queues(
                    server_read,
                    server_write,
                    NullLoopback,
                    SameFileTokenPriorityService {
                        state: priority_state,
                    },
                    1,
                    admission_queues,
                )
                .await;
            }
        });

        for message in [
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": unrelated_uri, "version": 3 },
                    "contentChanges": [{ "text": "// unrelated" }]
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": COMPLETION_METHOD,
                "params": {
                    "textDocument": { "uri": target_uri },
                    "position": { "line": 0, "character": 0 }
                }
            }),
        ] {
            let body = serde_json::to_vec(&message).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while priority_state
                .ready_waker
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_none()
            {
                priority_state.ready_blocked.notified().await;
            }
        })
        .await
        .expect("scheduler must block before dequeueing queued completion-priority work");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while admission_queues.lane_depth(AdmissionLane::Completion) < 2 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("both completion-priority items must be enqueued before the ready gate releases");

        let token_ready_position = {
            let state = admission_queues
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            token_ready_completion_position(&state.completion)
        };
        assert_eq!(
            token_ready_position,
            Some(0),
            "completion queue must surface the token-ready completion ahead of unrelated queued work before scheduler release"
        );

        priority_state.ready_release.store(true, Ordering::SeqCst);
        if let Some(waker) = priority_state
            .ready_waker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            waker.wake();
        }

        let mut reader = BufReader::new(client_read);
        let completion_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("completion response timeout");
        assert_eq!(
            completion_response.get("id").and_then(|value| value.as_i64()),
            Some(2),
            "token-ready completion must bypass unrelated queued same-priority work once its file token is already published"
        );

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while !priority_state.did_change_started.load(Ordering::SeqCst) {
                priority_state.did_change_started_notify.notified().await;
            }
        })
        .await
        .expect(
            "queued didChange should still begin polling after the bypassed completion responds",
        );

        priority_state
            .did_change_release
            .store(true, Ordering::SeqCst);
        priority_state.did_change_release_notify.notify_waiters();

        let completion_context =
            crate::server::request_context::take_completion_request_context_by_request_id("2")
                .expect("queued completion request context");
        assert_eq!(completion_context.uri, target_uri);
        assert_eq!(
            completion_context.same_file_ingress_token_required_version,
            Some(7)
        );
        assert_eq!(
            completion_context.same_file_ingress_token_published_at_ms,
            Some(1_700_000_000_007)
        );
        assert_eq!(
            completion_context.same_file_ingress_token_source.as_deref(),
            Some("did_change")
        );
        assert_eq!(completion_context.same_file_ingress_token_wait_ms, Some(0));

        crate::server::request_context::clear_same_file_ingress_token_publication_for_uri(
            target_uri,
        );
        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_adapter_task_isolation_keeps_ready_output_and_late_cancel_progress_while_scheduler_stalls(
    ) {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let isolation_state = Arc::new(SchedulerIsolationState::default());
        let admission_queues = AdmissionQueues::new(AdmissionQueueCapacities {
            control: 2,
            completion: 1,
            general: 1,
        });

        let server_task = tokio::spawn({
            let isolation_state = isolation_state.clone();
            let admission_queues = admission_queues.clone();
            async move {
                serve_with_completion_handoff_with_admission_queues(
                    server_read,
                    server_write,
                    NullLoopback,
                    SchedulerIsolationService {
                        state: isolation_state,
                    },
                    1,
                    admission_queues,
                )
                .await;
            }
        });

        let initial_general_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": { "uri": "file:///task-isolation-general.bsl" }
            }
        });
        let initial_general_body = serde_json::to_vec(&initial_general_request)
            .expect("serialize initial general request");
        let initial_general_header =
            format!("Content-Length: {}\r\n\r\n", initial_general_body.len());
        client_write
            .write_all(initial_general_header.as_bytes())
            .await
            .expect("write initial general request header");
        client_write
            .write_all(&initial_general_body)
            .await
            .expect("write initial general request body");
        client_write
            .flush()
            .await
            .expect("flush initial general request");

        tokio::time::timeout(std::time::Duration::from_millis(500), async {
            while !isolation_state
                .first_dispatch_completed
                .load(Ordering::SeqCst)
            {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("initial general request must dispatch before the scheduler stall scenario starts");

        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///task-isolation-queued-completion.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///task-isolation-spillover-completion.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "$/cancelRequest",
                "params": { "id": 3 }
            }),
        ] {
            let body = serde_json::to_vec(&request).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        tokio::time::timeout(std::time::Duration::from_millis(500), async {
            loop {
                if isolation_state
                    .ready_waker
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .is_some()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("scheduler must enter the blocking second poll_ready branch");

        let mut reader = BufReader::new(client_read);
        let first_response = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            read_framed_message(&mut reader),
        )
        .await
        .expect("ready output must flush while scheduler remains stalled");
        let second_response = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            read_framed_message(&mut reader),
        )
        .await
        .expect("late cancel must still be classified while scheduler remains stalled");
        let responses = [&first_response, &second_response];

        let general_response = responses
            .iter()
            .find(|response| response.get("id").and_then(|value| value.as_i64()) == Some(1))
            .expect("general response must flush before scheduler release");
        assert_eq!(
            general_response
                .get("result")
                .and_then(|value| value.get("kind"))
                .and_then(|value| value.as_str()),
            Some("general")
        );

        let cancel_response = responses
            .iter()
            .find(|response| response.get("id").and_then(|value| value.as_i64()) == Some(3))
            .expect("spillover completion cancel response");
        assert_eq!(
            cancel_response
                .get("error")
                .and_then(|value| value.get("code"))
                .and_then(|value| value.as_i64()),
            Some(ErrorCode::RequestCancelled.code()),
            "reader-side spillover cancel must still classify while the scheduler stall lives, responses={responses:?}"
        );
        assert_eq!(
            isolation_state.completion_call_count.load(Ordering::SeqCst),
            0,
            "scheduler stall must not force queued completion dispatch before the spillover cancel is classified"
        );

        isolation_state
            .scheduler_release
            .store(true, Ordering::SeqCst);
        if let Some(waker) = isolation_state
            .ready_waker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            waker.wake();
        }

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_allows_general_hover_to_bypass_inflight_did_open_barrier_while_completion_waits(
    ) {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let barrier_state = Arc::new(DocumentSyncBarrierBypassState::default());

        let server_task = tokio::spawn({
            let barrier_state = barrier_state.clone();
            async move {
                serve_with_completion_handoff_with_admission_queues(
                    server_read,
                    server_write,
                    NullLoopback,
                    DocumentSyncBarrierBypassService {
                        state: barrier_state,
                    },
                    2,
                    AdmissionQueues::new(AdmissionQueueCapacities {
                        control: 2,
                        completion: 2,
                        general: 2,
                    }),
                )
                .await;
            }
        });

        for message in [
            json!({
                "jsonrpc": "2.0",
                "method": DID_OPEN_METHOD,
                "params": {
                    "textDocument": {
                        "uri": "file:///hover-bypass-did-open.bsl",
                        "languageId": "bsl",
                        "version": 1,
                        "text": "Перем = 1;"
                    }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": "file:///hover-bypass-did-open.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": COMPLETION_METHOD,
                "params": {
                    "textDocument": { "uri": "file:///hover-bypass-did-open.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
        ] {
            let body = serde_json::to_vec(&message).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            barrier_state.did_open_started.notified().await;
        })
        .await
        .expect("didOpen barrier must reach service.call()");

        let mut reader = BufReader::new(client_read);
        let hover_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("general hover must bypass an inflight didOpen barrier");
        assert_eq!(
            hover_response.get("id").and_then(|value| value.as_i64()),
            Some(1),
            "hover must complete before the didOpen barrier is released, response={hover_response:?}"
        );
        assert_eq!(
            hover_response
                .get("result")
                .and_then(|value| value.get("contents"))
                .and_then(|value| value.get("value"))
                .and_then(|value| value.as_str()),
            Some("hover-ready")
        );

        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(50),
                read_framed_message(&mut reader),
            )
            .await
            .is_err(),
            "completion must stay gated until the inflight didOpen barrier releases"
        );

        barrier_state.did_open_release.notify_waiters();

        let completion_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("completion response timeout after releasing didOpen barrier");
        assert_eq!(
            completion_response
                .get("id")
                .and_then(|value| value.as_i64()),
            Some(2)
        );
        assert_eq!(
            completion_response
                .get("result")
                .and_then(|value| value.get("version"))
                .and_then(|value| value.as_i64()),
            Some(1),
            "completion must still observe the didOpen-applied version after the barrier releases, response={completion_response:?}"
        );
        let completion_context =
            crate::server::request_context::take_completion_request_context_by_request_id("2")
                .expect("queued completion request context");
        assert_eq!(
            completion_context
                .completion_barrier_owner_method
                .as_deref(),
            Some(DID_OPEN_METHOD)
        );
        assert_eq!(
            completion_context.completion_barrier_owner_uri.as_deref(),
            Some("file:///hover-bypass-did-open.bsl")
        );
        assert_eq!(completion_context.completion_barrier_owner_version, Some(1));
        assert!(
            completion_context
                .completion_barrier_wait_ms
                .is_some_and(|wait_ms| wait_ms > 0),
            "completion barrier wait must be attributed once completion was gated behind didOpen"
        );
        assert!(
            completion_context.doc_sync_first_poll_exec_ms.is_some(),
            "doc-sync first poll exec must be exported for the active barrier owner"
        );
        assert_eq!(
            completion_context.doc_sync_first_poll_outcome.as_deref(),
            Some("pending")
        );
        assert_eq!(
            completion_context.doc_sync_first_poll_method.as_deref(),
            Some(DID_OPEN_METHOD)
        );
        assert_eq!(
            completion_context.doc_sync_first_poll_uri.as_deref(),
            Some("file:///hover-bypass-did-open.bsl")
        );
        assert_eq!(completion_context.doc_sync_first_poll_version, Some(1));

        let call_order = barrier_state
            .call_order
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let hover_position = call_order
            .iter()
            .position(|method| method == "textDocument/hover")
            .expect("hover dispatch position");
        let completion_position = call_order
            .iter()
            .position(|method| method == COMPLETION_METHOD)
            .expect("completion dispatch position");
        assert!(
            hover_position < completion_position,
            "hover must be allowed through while completion remains gated behind the inflight didOpen handoff, call_order={call_order:?}"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_cancels_queued_completion_before_dispatch() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let priority_state = Arc::new(PrioritySchedulerState::default());

        let server_task = tokio::spawn({
            let priority_state = priority_state.clone();
            async move {
                serve_with_completion_handoff(
                    server_read,
                    server_write,
                    NullLoopback,
                    PrioritySchedulerService {
                        state: priority_state,
                    },
                    1,
                )
                .await;
            }
        });

        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": "file:///cancel-general.bsl" }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///queued-cancel.bsl" },
                    "position": { "line": 0, "character": 0 },
                    "context": {
                        "triggerKind": 2,
                        "triggerCharacter": "."
                    },
                    "bslProbeId": "probe-queued-cancel"
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "$/cancelRequest",
                "params": { "id": 2 }
            }),
        ] {
            let body = serde_json::to_vec(&request).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while !priority_state.general_inflight.load(Ordering::SeqCst) {
                priority_state.general_started.notified().await;
            }
        })
        .await
        .expect("general request must hold service readiness");

        priority_state.general_release.notify_waiters();

        let mut reader = BufReader::new(client_read);
        let first_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("first response timeout");

        let second_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("second response timeout");

        let (general_response, cancelled_response) =
            if first_response.get("id").and_then(|value| value.as_i64()) == Some(2) {
                (&second_response, &first_response)
            } else {
                (&first_response, &second_response)
            };
        assert_eq!(
            general_response.get("id").and_then(|value| value.as_i64()),
            Some(1),
            "general request must still finish exactly once, first_response={first_response:?}, second_response={second_response:?}"
        );
        assert_eq!(
            cancelled_response
                .get("id")
                .and_then(|value| value.as_i64()),
            Some(2),
            "queued completion cancel must publish exactly one terminal response, first_response={first_response:?}, second_response={second_response:?}"
        );
        assert_eq!(
            cancelled_response
                .get("error")
                .and_then(|value| value.get("code"))
                .and_then(|value| value.as_i64()),
            Some(-32800)
        );
        assert_eq!(
            priority_state.completion_call_count.load(Ordering::SeqCst),
            0,
            "queued pre-dispatch completion must not reach service.call()"
        );

        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(50),
                read_framed_message(&mut reader),
            )
            .await
            .is_err(),
            "queued pre-dispatch cancel must remain exactly-once"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_classifies_late_cancel_while_completion_lane_is_saturated() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let priority_state = Arc::new(PrioritySchedulerState::default());
        let admission_queues = AdmissionQueues::new(AdmissionQueueCapacities {
            control: 1,
            completion: 1,
            general: 1,
        });

        let server_task = tokio::spawn({
            let priority_state = priority_state.clone();
            let admission_queues = admission_queues.clone();
            async move {
                serve_with_completion_handoff_with_admission_queues(
                    server_read,
                    server_write,
                    NullLoopback,
                    PrioritySchedulerService {
                        state: priority_state,
                    },
                    1,
                    admission_queues,
                )
                .await;
            }
        });

        let general_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": { "uri": "file:///cancel-blocker-general.bsl" }
            }
        });
        let general_body = serde_json::to_vec(&general_request).expect("serialize general request");
        let general_header = format!("Content-Length: {}\r\n\r\n", general_body.len());
        client_write
            .write_all(general_header.as_bytes())
            .await
            .expect("write general request header");
        client_write
            .write_all(&general_body)
            .await
            .expect("write general request body");
        client_write.flush().await.expect("flush general request");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while !priority_state.general_inflight.load(Ordering::SeqCst) {
                priority_state.general_started.notified().await;
            }
        })
        .await
        .expect("general request must hold service readiness before completion saturation");

        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///cancel-saturated-1.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///cancel-saturated-2.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "$/cancelRequest",
                "params": { "id": 3 }
            }),
        ] {
            let body = serde_json::to_vec(&request).expect("serialize follow-up request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write follow-up request header");
            client_write
                .write_all(&body)
                .await
                .expect("write follow-up request body");
        }
        client_write
            .flush()
            .await
            .expect("flush follow-up requests");

        let mut reader = BufReader::new(client_read);
        let cancel_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("late cancel must be classified before releasing the general blocker");
        assert_eq!(
            cancel_response.get("id").and_then(|value| value.as_i64()),
            Some(3)
        );
        assert_eq!(
            cancel_response
                .get("error")
                .and_then(|value| value.get("code"))
                .and_then(|value| value.as_i64()),
            Some(-32800),
            "late cancel must still cancel the queued completion before dispatch, response={cancel_response:?}"
        );

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            loop {
                if admission_queues.lane_depth(AdmissionLane::Completion) == 1 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect(
            "the first completion must remain queued while the general blocker holds readiness",
        );

        priority_state.general_release.notify_waiters();

        let mut responses_by_id = HashMap::new();
        responses_by_id.insert(3, cancel_response);
        for _ in 0..2 {
            let response = tokio::time::timeout(
                std::time::Duration::from_millis(150),
                read_framed_message(&mut reader),
            )
            .await
            .expect("response timeout after releasing general blocker");
            let response_id = response
                .get("id")
                .and_then(|value| value.as_i64())
                .expect("response id");
            responses_by_id.insert(response_id, response);
        }

        assert_eq!(
            responses_by_id.len(),
            3,
            "general request, first completion and queued cancel must each resolve exactly once, responses={responses_by_id:?}"
        );
        assert_eq!(
            priority_state.completion_call_count.load(Ordering::SeqCst),
            1,
            "only the first queued completion may reach service.call() once late cancel is classified"
        );

        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(50),
                read_framed_message(&mut reader),
            )
            .await
            .is_err(),
            "late control classification must keep pre-dispatch cancellation exactly-once"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_reads_cancel_after_completion_lane_saturates() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let backpressure_state = Arc::new(CompletionIngressBackpressureState::default());

        let server_task = tokio::spawn({
            let backpressure_state = backpressure_state.clone();
            async move {
                serve_with_completion_handoff_with_admission_queues(
                    server_read,
                    server_write,
                    NullLoopback,
                    CompletionIngressBackpressureService {
                        state: backpressure_state,
                    },
                    1,
                    AdmissionQueues::new(AdmissionQueueCapacities {
                        control: 2,
                        completion: 1,
                        general: 1,
                    }),
                )
                .await;
            }
        });

        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///cancel-saturated-completion.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///cancel-saturated-followup.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "$/cancelRequest",
                "params": { "id": 2 }
            }),
        ] {
            let body = serde_json::to_vec(&request).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while backpressure_state
                .ready_waker
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_none()
            {
                backpressure_state.ready_blocked.notified().await;
            }
        })
        .await
        .expect("scheduler must block before dispatching any queued completion");

        backpressure_state
            .ready_release
            .store(true, Ordering::SeqCst);
        if let Some(waker) = backpressure_state
            .ready_waker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            waker.wake();
        }

        let mut reader = BufReader::new(client_read);
        let first_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("first response timeout");
        let second_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("second response timeout");
        let responses = [&first_response, &second_response];
        let cancelled_completion = responses
            .iter()
            .find(|response| response.get("id").and_then(|value| value.as_i64()) == Some(2))
            .expect("cancelled completion response");
        assert_eq!(
            cancelled_completion
                .get("error")
                .and_then(|value| value.get("code"))
                .and_then(|value| value.as_i64()),
            Some(ErrorCode::RequestCancelled.code()),
            "queued completion must still be cancelled even when a later completion saturated the lane, responses={responses:?}"
        );
        let followup_completion = responses
            .iter()
            .find(|response| response.get("id").and_then(|value| value.as_i64()) == Some(3))
            .expect("follow-up completion response");
        assert!(
            followup_completion.get("result").is_some(),
            "follow-up completion must still complete after the cancelled request, responses={responses:?}"
        );
        assert_eq!(
            backpressure_state
                .call_order
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .as_slice(),
            &[3],
            "only the follow-up completion may reach dispatch after the queued cancel, responses={responses:?}"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_reads_cancel_after_completion_spillover_overflows() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let backpressure_state = Arc::new(CompletionIngressBackpressureState::default());

        let server_task = tokio::spawn({
            let backpressure_state = backpressure_state.clone();
            async move {
                serve_with_completion_handoff_with_admission_queues(
                    server_read,
                    server_write,
                    NullLoopback,
                    CompletionIngressBackpressureService {
                        state: backpressure_state,
                    },
                    1,
                    AdmissionQueues::new(AdmissionQueueCapacities {
                        control: 2,
                        completion: 1,
                        general: 1,
                    }),
                )
                .await;
            }
        });

        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///cancel-overflow-queued.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///cancel-overflow-evicted.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///cancel-overflow-cancelled.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "$/cancelRequest",
                "params": { "id": 4 }
            }),
        ] {
            let body = serde_json::to_vec(&request).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while backpressure_state
                .ready_waker
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_none()
            {
                backpressure_state.ready_blocked.notified().await;
            }
        })
        .await
        .expect("scheduler must block before dispatching queued completion");

        let mut reader = BufReader::new(client_read);
        let first_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("first overflow response timeout");
        let second_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("second overflow response timeout");

        let responses = [&first_response, &second_response];
        let rejected_completion = responses
            .iter()
            .find(|response| response.get("id").and_then(|value| value.as_i64()) == Some(3))
            .expect("rejected overflow completion response");
        let rejected_items = rejected_completion
            .get("result")
            .and_then(|value| value.get("items"))
            .and_then(|value| value.as_array())
            .expect("overflow rejection completion items");
        assert!(
            rejected_items.is_empty(),
            "overflowed completion must fail closed with an empty completion response, responses={responses:?}"
        );
        assert_eq!(
            rejected_completion
                .get("result")
                .and_then(|value| value.get("isIncomplete"))
                .and_then(|value| value.as_bool()),
            Some(true),
            "overflowed completion must stay incomplete-empty to match queue_rejected semantics, responses={responses:?}"
        );

        let cancelled_completion = responses
            .iter()
            .find(|response| response.get("id").and_then(|value| value.as_i64()) == Some(4))
            .expect("cancelled overflow completion response");
        assert_eq!(
            cancelled_completion
                .get("error")
                .and_then(|value| value.get("code"))
                .and_then(|value| value.as_i64()),
            Some(ErrorCode::RequestCancelled.code()),
            "late cancel must still classify after spillover overflow, responses={responses:?}"
        );

        backpressure_state
            .ready_release
            .store(true, Ordering::SeqCst);
        if let Some(waker) = backpressure_state
            .ready_waker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            waker.wake();
        }

        let completion_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("queued completion response timeout");
        assert_eq!(
            completion_response
                .get("id")
                .and_then(|value| value.as_i64()),
            Some(2)
        );
        assert!(
            completion_response.get("result").is_some(),
            "oldest queued completion must still complete after releasing readiness, response={completion_response:?}"
        );
        assert_eq!(
            backpressure_state
                .call_order
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .as_slice(),
            &[2],
            "only the oldest queued completion may reach dispatch once overflow eviction and late cancel are applied"
        );

        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(50),
                read_framed_message(&mut reader),
            )
            .await
            .is_err(),
            "overflow rejection and late cancel must remain exactly-once"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_attributes_reader_backpressure_before_adapter_read_after_completion_spillover_wait(
    ) {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let backpressure_state = Arc::new(CompletionIngressBackpressureState::default());

        let server_task = tokio::spawn({
            let backpressure_state = backpressure_state.clone();
            async move {
                serve_with_completion_handoff_with_admission_queues(
                    server_read,
                    server_write,
                    NullLoopback,
                    CompletionIngressBackpressureService {
                        state: backpressure_state,
                    },
                    1,
                    AdmissionQueues::new(AdmissionQueueCapacities {
                        control: 2,
                        completion: 1,
                        general: 1,
                    }),
                )
                .await;
            }
        });

        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 42_122,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///reader-backpressure-queued.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 42_123,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///reader-backpressure-spillover.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
        ] {
            let body = serde_json::to_vec(&request).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write
            .flush()
            .await
            .expect("flush initial saturated completion requests");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while backpressure_state
                .ready_waker
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_none()
            {
                backpressure_state.ready_blocked.notified().await;
            }
        })
        .await
        .expect("scheduler must block before reader-side completion spillover wait is exercised");

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        backpressure_state
            .ready_release
            .store(true, Ordering::SeqCst);
        if let Some(waker) = backpressure_state
            .ready_waker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            waker.wake();
        }

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            loop {
                if backpressure_state
                    .call_order
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .contains(&42_122)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("queued completion must dispatch to release completion-lane space");

        let traced_request = json!({
            "jsonrpc": "2.0",
            "id": 42_124,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": "file:///reader-backpressure-traced.bsl" },
                "position": { "line": 0, "character": 0 }
            }
        });
        let traced_body = serde_json::to_vec(&traced_request).expect("serialize traced request");
        let traced_header = format!("Content-Length: {}\r\n\r\n", traced_body.len());
        client_write
            .write_all(traced_header.as_bytes())
            .await
            .expect("write traced request header");
        client_write
            .write_all(&traced_body)
            .await
            .expect("write traced request body");
        client_write.flush().await.expect("flush traced request");

        let mut reader = BufReader::new(client_read);
        let mut responses_by_id = HashMap::new();
        for _ in 0..3 {
            let response = tokio::time::timeout(
                std::time::Duration::from_millis(150),
                read_framed_message(&mut reader),
            )
            .await
            .expect("completion response timeout after releasing reader backpressure");
            let response_id = response
                .get("id")
                .and_then(|value| value.as_i64())
                .expect("response id");
            responses_by_id.insert(response_id, response);
        }
        assert_eq!(
            responses_by_id.len(),
            3,
            "queued, spillover and traced completions must each resolve exactly once, responses={responses_by_id:?}"
        );
        assert!(
            backpressure_state
                .call_order
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .contains(&42_124),
            "traced completion must still reach dispatch after the reader-side wait"
        );

        let traced_context =
            crate::server::request_context::take_completion_request_context_by_request_id("42124")
                .expect("traced completion request context");
        assert_eq!(
            traced_context.read_loop_wait_reason.as_deref(),
            Some("completion_lane_space")
        );
        assert!(
            traced_context.read_loop_wait_ms.unwrap_or(0) > 0,
            "reader-side spillover wait must be recorded as a positive server-side wait, context={traced_context:?}"
        );
        assert_eq!(traced_context.pending_completion_spillover_depth, Some(1));
        assert_eq!(traced_context.pending_general_request_staged, Some(false));
        assert!(
            traced_context.adapter_read_started_at_ms.is_some()
                && traced_context.adapter_read_at_ms.is_some()
                && traced_context.adapter_parse_completed_at_ms.is_some(),
            "traced completion must retain authoritative adapter-read timestamps after the local reader wait, context={traced_context:?}"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_reads_cancel_after_completion_barrier_saturates() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let barrier_state = Arc::new(CompletionBarrierBackpressureState::default());
        *barrier_state
            .latest_version
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = 1;

        let server_task = tokio::spawn({
            let barrier_state = barrier_state.clone();
            async move {
                serve_with_completion_handoff_with_admission_queues(
                    server_read,
                    server_write,
                    NullLoopback,
                    CompletionBarrierBackpressureService {
                        state: barrier_state,
                    },
                    1,
                    AdmissionQueues::new(AdmissionQueueCapacities {
                        control: 2,
                        completion: 1,
                        general: 1,
                    }),
                )
                .await;
            }
        });

        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///cancel-saturated-barrier.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": "file:///cancel-saturated-barrier.bsl", "version": 2 },
                    "contentChanges": [{ "text": "// v2" }]
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "$/cancelRequest",
                "params": { "id": 2 }
            }),
        ] {
            let body = serde_json::to_vec(&request).expect("serialize request");
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            client_write
                .write_all(header.as_bytes())
                .await
                .expect("write request header");
            client_write
                .write_all(&body)
                .await
                .expect("write request body");
        }
        client_write.flush().await.expect("flush requests");

        tokio::time::timeout(std::time::Duration::from_millis(150), async {
            while barrier_state
                .ready_waker
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_none()
            {
                barrier_state.ready_blocked.notified().await;
            }
        })
        .await
        .expect("scheduler must block before dispatching queued completion or didChange");

        barrier_state.ready_release.store(true, Ordering::SeqCst);
        if let Some(waker) = barrier_state
            .ready_waker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            waker.wake();
        }

        let mut reader = BufReader::new(client_read);
        let cancel_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("cancel response timeout");
        assert_eq!(
            cancel_response.get("id").and_then(|value| value.as_i64()),
            Some(2)
        );
        assert_eq!(
            cancel_response
                .get("error")
                .and_then(|value| value.get("code"))
                .and_then(|value| value.as_i64()),
            Some(ErrorCode::RequestCancelled.code()),
            "cancel must still beat queued completion dispatch even when didChange occupied the completion barrier path, response={cancel_response:?}"
        );
        assert_eq!(
            barrier_state
                .call_order
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .as_slice(),
            &[
                DID_CHANGE_METHOD.to_string(),
                CANCEL_REQUEST_METHOD.to_string()
            ],
            "didChange barrier and control request may dispatch, but cancelled completion must not reach service.call()"
        );

        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(50),
                read_framed_message(&mut reader),
            )
            .await
            .is_err(),
            "cancelled completion must remain exactly-once even after the didChange barrier dispatches"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn completion_handoff_acceptance_can_precede_legacy_writer_selection_seam() {
        let (responses_tx, mut responses_rx) = mpsc::channel(1);
        let handoff_task = CompletionHandoffTask::new(
            Some("req-handoff".to_string()),
            async {
                Some(JsonRpcResponse::from_ok(
                    Id::Number(7),
                    serde_json::json!({ "items": [] }),
                ))
            }
            .boxed(),
        );

        handoff_task.forward_response(responses_tx).await;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let writer_selected_at_ms = super::super::unix_timestamp_ms();
        let queued_message = responses_rx
            .next()
            .await
            .expect("queued completion response must be available");

        assert_eq!(
            queued_message.completion_request_id.as_deref(),
            Some("req-handoff")
        );
        let handoff_started_at_ms = queued_message
            .completion_response_handoff_started_at_ms
            .expect("handoff started milestone");
        let handoff_enqueued_at_ms = resolve_completion_handoff_enqueued_at_ms(
            queued_message
                .completion_response_handoff_enqueued_at_ms
                .as_ref(),
            writer_selected_at_ms,
        )
        .await;

        assert!(
            handoff_started_at_ms <= handoff_enqueued_at_ms,
            "send-side handoff acceptance must not precede handoff start"
        );
        assert!(
            handoff_enqueued_at_ms < writer_selected_at_ms,
            "channel acceptance must precede the delayed writer-selection seam"
        );
    }
}
