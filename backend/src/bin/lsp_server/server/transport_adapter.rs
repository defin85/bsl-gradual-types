use std::fmt::{self, Display, Formatter};
use std::io::Error as IoError;
use std::num::ParseIntError;
use std::str::Utf8Error;
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

    async fn forward_response(self, mut responses_tx: mpsc::Sender<TransportMessage>) {
        let request_id = self.request_id;
        if let Some(response) = self.future.await {
            if responses_tx
                .send(TransportMessage::Response(response))
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
            }
        }
    }
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
                Ok(()) => return true,
                Err(TryEnqueueError::Closed) => return false,
                Err(TryEnqueueError::Full(request)) => {
                    scheduled_request = Some(request);
                    let notified = self.space_notify.notified();
                    notified.await;
                }
            }
        }
    }

    fn try_enqueue(&self, scheduled_request: ScheduledRequest) -> Result<(), TryEnqueueError> {
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
        if queue.len() < lane_capacity {
            queue.push_back(scheduled_request);
            self.item_notify.notify_waiters();
            Ok(())
        } else {
            Err(TryEnqueueError::Full(scheduled_request))
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

    async fn pop_next(&self) -> Option<ScheduledRequest> {
        let next = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state
                .control
                .pop_front()
                .or_else(|| state.completion.pop_front())
                .or_else(|| state.general.pop_front())
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

    #[cfg(test)]
    fn lane_depth(&self, lane: AdmissionLane) -> usize {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Self::queue_for_lane(&state, lane).len()
    }
}

enum TryEnqueueError {
    Closed,
    Full(ScheduledRequest),
}

pub(crate) async fn serve_with_completion_handoff<I, O, L, S>(
    stdin: I,
    stdout: O,
    socket: L,
    service: S,
    concurrency_level: usize,
) where
    I: AsyncRead + Unpin,
    O: AsyncWrite + Unpin,
    L: Loopback,
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
    I: AsyncRead + Unpin,
    O: AsyncWrite + Unpin,
    L: Loopback,
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
    I: AsyncRead + Unpin,
    O: AsyncWrite + Unpin,
    L: Loopback,
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

    let responses_tx_for_server_tasks = responses_tx.clone();
    let process_server_tasks = async move {
        let mut responses_tx = responses_tx_for_server_tasks;
        let mut server_tasks = server_tasks_rx.buffer_unordered(concurrency_level);

        while let Some(response) = server_tasks.next().await {
            let Some(response) = response else {
                continue;
            };
            if responses_tx
                .send(TransportMessage::Response(response))
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
    let process_scheduler = async move {
        let mut responses_tx = responses_tx_for_scheduler;
        loop {
            if !admission_queues_for_scheduler
                .wait_until_non_empty_or_closed()
                .await
            {
                break;
            }
            if let Err(err) = future::poll_fn(|cx| service.poll_ready(cx)).await {
                error!("{}", display_sources(err.into().as_ref()));
                break;
            }
            let Some(scheduled_request) = admission_queues_for_scheduler.pop_next().await else {
                continue;
            };

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
            let future = service
                .call(scheduled_request.request)
                .unwrap_or_else(|err| {
                    error!("{}", display_sources(err.into().as_ref()));
                    None
                })
                .boxed();

            if is_completion_handoff_barrier {
                if let Some(response) = future.await {
                    if responses_tx
                        .send(TransportMessage::Response(response))
                        .await
                        .is_err()
                    {
                        error!("failed to forward completion handoff barrier response: transport closed");
                        break;
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
        let outbound = stream::select(responses_rx, client_requests.map(TransportMessage::Request));
        pin_mut!(outbound);

        loop {
            tokio::select! {
                _ = transport_shutdown_for_output.notified() => break,
                maybe_message = outbound.next() => {
                    let Some(message) = maybe_message else {
                        break;
                    };
                    if let Err(err) = write_transport_message(&mut stdout, &message).await {
                        error!("failed to encode message: {err}");
                        break;
                    }
                }
            }
        }
    };

    let transport_shutdown_for_input = transport_shutdown.clone();
    let admission_queues_for_input = admission_queues.clone();
    let client_abort_for_input = client_abort;
    let read_input = async move {
        let mut stdin = BufReader::new(stdin);
        let completion_spillover_capacity =
            admission_queues_for_input.lane_capacity(AdmissionLane::Completion);
        let mut pending_completion_requests = std::collections::VecDeque::new();
        let mut pending_general_request = None;

        'read_input: loop {
            while let Some(staged_completion_request) = pending_completion_requests.pop_front() {
                match admission_queues_for_input.try_enqueue(staged_completion_request) {
                    Ok(()) => {}
                    Err(TryEnqueueError::Closed) => {
                        error!("transport admission queue closed unexpectedly");
                        break 'read_input;
                    }
                    Err(TryEnqueueError::Full(request)) => {
                        pending_completion_requests.push_front(request);
                        break;
                    }
                }
            }

            if let Some(staged_general_request) = pending_general_request.take() {
                match admission_queues_for_input.try_enqueue(staged_general_request) {
                    Ok(()) => {}
                    Err(TryEnqueueError::Closed) => {
                        error!("transport admission queue closed unexpectedly");
                        break;
                    }
                    Err(TryEnqueueError::Full(request)) => {
                        pending_general_request = Some(request);
                    }
                }
            }

            let read_result = tokio::select! {
                _ = transport_shutdown_for_input.notified() => break,
                lane_has_space = admission_queues_for_input.wait_for_space_in_lane_or_closed(AdmissionLane::Completion), if !pending_completion_requests.is_empty() => {
                    if !lane_has_space {
                        error!("transport admission queue closed unexpectedly");
                        break;
                    }
                    continue;
                }
                lane_has_space = admission_queues_for_input.wait_for_space_in_lane_or_closed(AdmissionLane::General), if pending_general_request.is_some() => {
                    if !lane_has_space {
                        error!("transport admission queue closed unexpectedly");
                        break;
                    }
                    continue;
                }
                read_result = read_transport_message(&mut stdin) => read_result,
            };
            match read_result {
                Ok(Some(TransportMessage::Request(request))) => {
                    let adapter_read_at_ms = super::unix_timestamp_ms();
                    let request_id = request.id().map(ToString::to_string);
                    if let Some(request_id) = request_id.as_deref() {
                        super::request_context::record_pending_completion_adapter_read_at_ms(
                            &request,
                            request_id,
                            Some(adapter_read_at_ms),
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
                            Ok(()) => {}
                            Err(TryEnqueueError::Closed) => {
                                error!("transport admission queue closed unexpectedly");
                                break;
                            }
                            Err(TryEnqueueError::Full(request)) => {
                                pending_general_request = Some(request);
                            }
                        }
                    } else if matches!(scheduled_request.lane, AdmissionLane::Completion) {
                        match admission_queues_for_input.try_enqueue(scheduled_request) {
                            Ok(()) => {}
                            Err(TryEnqueueError::Closed) => {
                                error!("transport admission queue closed unexpectedly");
                                break;
                            }
                            Err(TryEnqueueError::Full(request)) => {
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
                            }
                        }
                    } else if !admission_queues_for_input.enqueue(scheduled_request).await {
                        error!("transport admission queue closed unexpectedly");
                        break;
                    }
                }
                Ok(Some(TransportMessage::Response(response))) => {
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
                        .send(TransportMessage::Response(response))
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

    futures::join!(
        print_output,
        read_input,
        process_scheduler,
        process_server_tasks,
        process_completion_tasks
    );
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
    responses_tx: &mut mpsc::Sender<TransportMessage>,
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
        .send(TransportMessage::Response(response))
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
    responses_tx: &mut mpsc::Sender<TransportMessage>,
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
        .send(TransportMessage::Response(response))
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
    responses_tx: &mut mpsc::Sender<TransportMessage>,
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
    responses_tx: &mut mpsc::Sender<TransportMessage>,
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
    responses_tx: &mut mpsc::Sender<TransportMessage>,
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
        .send(TransportMessage::Response(response))
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
) -> Result<Option<TransportMessage>, TransportCodecError>
where
    I: AsyncRead + Unpin,
{
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
    Ok(Some(message))
}

async fn write_transport_message<O>(
    writer: &mut BufWriter<O>,
    message: &TransportMessage,
) -> Result<(), TransportCodecError>
where
    O: AsyncWrite + Unpin,
{
    let body = serde_json::to_vec(message).map_err(TransportCodecError::Json)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;
    Ok(())
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
                            .map(|value| {
                                value
                                    .as_i64()
                                    .map(|id| id.to_string())
                                    .or_else(|| value.as_str().map(ToString::to_string))
                            })
                            .flatten()
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
                let pending = cancel_state
                    .pending_cancellations
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if pending.contains_key("1") {
                    break;
                }
                drop(pending);
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
            cancelled_response.get("id").and_then(|value| value.as_i64()),
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
            &[DID_CHANGE_METHOD.to_string(), CANCEL_REQUEST_METHOD.to_string()],
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
}
