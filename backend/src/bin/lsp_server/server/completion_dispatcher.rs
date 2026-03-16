use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bsl_analysis_v2::FileId as V2FileId;
use tokio::sync::{oneshot, Mutex as TokioMutex, Notify};
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub(crate) struct CompletionRequestMetadata {
    pub request_id: Option<String>,
    pub version_hint: Option<i32>,
    pub trigger_mode: String,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionEventEnvelope {
    pub file_id: V2FileId,
    pub file_seq: u64,
    pub request_epoch: u64,
    pub received_at: Instant,
    pub payload: CompletionEventPayload,
}

#[derive(Debug, Clone)]
pub(crate) enum CompletionEventPayload {
    DidOpen {
        version: i32,
    },
    DidChange {
        version: i32,
    },
    CompletionRequest {
        request_id: Option<String>,
        version_hint: Option<i32>,
        trigger_mode: String,
    },
    Cancel {
        request_id: String,
    },
    DidClose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QueueEnqueueOutcome {
    Enqueued,
    CoalescedDidChange,
    CoalescedCancel,
    EvictedStaleCompletion,
    EvictedNonCancelForCancel,
    EvictedCancelForCancel,
    Full,
    Closed,
}

impl QueueEnqueueOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Enqueued => "enqueued",
            Self::CoalescedDidChange => "coalesced_did_change",
            Self::CoalescedCancel => "coalesced_cancel",
            Self::EvictedStaleCompletion => "evicted_stale_completion",
            Self::EvictedNonCancelForCancel => "evicted_non_cancel_for_cancel",
            Self::EvictedCancelForCancel => "evicted_cancel_for_cancel",
            Self::Full => "full",
            Self::Closed => "closed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DispatchTicket {
    pub file_seq: u64,
    pub request_epoch: u64,
    pub queue_outcome: QueueEnqueueOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionTurnOutcome {
    Ready,
    SupersededBeforeStart,
    QueueRejected,
}

impl CompletionTurnOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::SupersededBeforeStart => "superseded_before_start",
            Self::QueueRejected => "queue_rejected",
        }
    }
}

#[derive(Debug)]
pub(crate) struct CompletionTurnWaiter {
    receiver: oneshot::Receiver<CompletionTurnOutcome>,
}

impl CompletionTurnWaiter {
    pub(crate) async fn wait(self) -> CompletionTurnOutcome {
        self.receiver
            .await
            .unwrap_or(CompletionTurnOutcome::QueueRejected)
    }
}

#[derive(Debug)]
pub(crate) struct CompletionRequestDispatch {
    pub ticket: DispatchTicket,
    pub turn_waiter: Option<CompletionTurnWaiter>,
    pub attribution: CompletionDispatchAttributionSnapshot,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionTurnHolderSnapshot {
    pub request_id: Option<String>,
    pub file_seq: u64,
    pub request_epoch: u64,
    pub trigger_mode: String,
    pub version_hint: Option<i32>,
    pub age: Duration,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionDispatchAttributionSnapshot {
    pub request_file_seq: u64,
    pub request_epoch: u64,
    pub queue_outcome: QueueEnqueueOutcome,
    pub queue_capacity: usize,
    pub queue_depth_before_enqueue: usize,
    pub queue_depth_after_enqueue: usize,
    pub queued_completion_ahead_count: usize,
    pub did_change_ahead_count: usize,
    pub active_completion_count: usize,
    pub dropped_completion_file_seq: Vec<u64>,
    pub active_holder: Option<CompletionTurnHolderSnapshot>,
    pub queued_completion_ahead: Option<CompletionTurnHolderSnapshot>,
}

#[derive(Debug, Default)]
struct CompletionEventQueueState {
    entries: VecDeque<CompletionEventEnvelope>,
    closed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionEventQueue {
    capacity: Arc<AtomicUsize>,
    state: Arc<Mutex<CompletionEventQueueState>>,
    notify: Arc<Notify>,
}

impl CompletionEventQueue {
    fn new(capacity: usize) -> Self {
        Self {
            capacity: Arc::new(AtomicUsize::new(capacity.max(1))),
            state: Arc::new(Mutex::new(CompletionEventQueueState::default())),
            notify: Arc::new(Notify::new()),
        }
    }

    fn capacity(&self) -> usize {
        self.capacity.load(Ordering::SeqCst).max(1)
    }

    fn set_capacity(&self, capacity: usize) {
        self.capacity.store(capacity.max(1), Ordering::SeqCst);
    }

    #[cfg(test)]
    fn try_enqueue(&self, event: CompletionEventEnvelope) -> QueueEnqueueOutcome {
        self.try_enqueue_with_report(event).0
    }

    fn try_enqueue_with_report(
        &self,
        event: CompletionEventEnvelope,
    ) -> (QueueEnqueueOutcome, Vec<u64>) {
        let capacity = self.capacity();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.closed {
            return (QueueEnqueueOutcome::Closed, Vec::new());
        }

        let before_completion_file_seq: HashSet<u64> = state
            .entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.payload,
                    CompletionEventPayload::CompletionRequest { .. }
                )
            })
            .map(|entry| entry.file_seq)
            .collect();

        let mut outcome = Self::apply_pre_enqueue_policy(&mut state.entries, &event);
        if matches!(outcome, QueueEnqueueOutcome::CoalescedCancel) {
            let dropped =
                Self::dropped_completion_file_seq(&before_completion_file_seq, &state.entries);
            return (outcome, dropped);
        }

        if state.entries.len() >= capacity {
            let overflow_outcome = Self::apply_overflow_policy(&mut state.entries, &event);
            let Some(overflow_outcome) = overflow_outcome else {
                let dropped =
                    Self::dropped_completion_file_seq(&before_completion_file_seq, &state.entries);
                return (QueueEnqueueOutcome::Full, dropped);
            };
            outcome = overflow_outcome;
        }

        if state.entries.len() >= capacity {
            let dropped =
                Self::dropped_completion_file_seq(&before_completion_file_seq, &state.entries);
            return (QueueEnqueueOutcome::Full, dropped);
        }

        state.entries.push_back(event);
        let dropped =
            Self::dropped_completion_file_seq(&before_completion_file_seq, &state.entries);
        drop(state);
        self.notify.notify_one();
        (outcome, dropped)
    }

    async fn recv(&self) -> Option<CompletionEventEnvelope> {
        loop {
            let notified = {
                let mut state = self
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if let Some(event) = state.entries.pop_front() {
                    return Some(event);
                }
                if state.closed {
                    return None;
                }
                self.notify.notified()
            };
            notified.await;
        }
    }

    fn close(&self) {
        {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.closed = true;
        }
        self.notify.notify_waiters();
    }

    #[cfg(test)]
    fn debug_payloads(&self) -> Vec<CompletionEventPayload> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state
            .entries
            .iter()
            .map(|entry| entry.payload.clone())
            .collect()
    }

    #[cfg(test)]
    fn debug_envelopes(&self) -> Vec<CompletionEventEnvelope> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.entries.iter().cloned().collect()
    }

    fn summary(&self, now: Instant) -> CompletionEventQueueSummary {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut completion_request_count = 0usize;
        let mut did_change_count = 0usize;
        let mut first_completion = None;
        for entry in state.entries.iter() {
            match &entry.payload {
                CompletionEventPayload::CompletionRequest {
                    request_id,
                    version_hint,
                    trigger_mode,
                } => {
                    completion_request_count = completion_request_count.saturating_add(1);
                    if first_completion.is_none() {
                        first_completion = Some(CompletionTurnHolderSnapshot {
                            request_id: request_id.clone(),
                            file_seq: entry.file_seq,
                            request_epoch: entry.request_epoch,
                            trigger_mode: trigger_mode.clone(),
                            version_hint: *version_hint,
                            age: now.saturating_duration_since(entry.received_at),
                        });
                    }
                }
                CompletionEventPayload::DidChange { .. } => {
                    did_change_count = did_change_count.saturating_add(1);
                }
                CompletionEventPayload::DidOpen { .. }
                | CompletionEventPayload::Cancel { .. }
                | CompletionEventPayload::DidClose => {}
            }
        }
        CompletionEventQueueSummary {
            depth: state.entries.len(),
            completion_request_count,
            did_change_count,
            first_completion,
        }
    }

    fn apply_pre_enqueue_policy(
        entries: &mut VecDeque<CompletionEventEnvelope>,
        event: &CompletionEventEnvelope,
    ) -> QueueEnqueueOutcome {
        match &event.payload {
            CompletionEventPayload::DidChange { .. } => {
                let removed = Self::remove_matching(entries, |queued| {
                    matches!(queued.payload, CompletionEventPayload::DidChange { .. })
                });
                if removed {
                    QueueEnqueueOutcome::CoalescedDidChange
                } else {
                    QueueEnqueueOutcome::Enqueued
                }
            }
            CompletionEventPayload::CompletionRequest { .. } => {
                let incoming_epoch = event.request_epoch;
                let stale_request_ids: HashSet<String> = entries
                    .iter()
                    .filter_map(|queued| match &queued.payload {
                        CompletionEventPayload::CompletionRequest {
                            request_id,
                            version_hint: _,
                            trigger_mode: _,
                        } if queued.request_epoch < incoming_epoch => request_id.clone(),
                        _ => None,
                    })
                    .collect();

                let removed = Self::remove_matching(entries, |queued| match &queued.payload {
                    CompletionEventPayload::CompletionRequest { .. } => {
                        queued.request_epoch < incoming_epoch
                    }
                    CompletionEventPayload::Cancel { request_id } => {
                        stale_request_ids.contains(request_id)
                    }
                    _ => false,
                });
                if removed {
                    QueueEnqueueOutcome::EvictedStaleCompletion
                } else {
                    QueueEnqueueOutcome::Enqueued
                }
            }
            CompletionEventPayload::Cancel { request_id } => {
                let duplicated = entries.iter().any(|queued| {
                    matches!(
                        &queued.payload,
                        CompletionEventPayload::Cancel {
                            request_id: queued_request_id,
                        } if queued_request_id == request_id
                    )
                });
                if duplicated {
                    QueueEnqueueOutcome::CoalescedCancel
                } else {
                    QueueEnqueueOutcome::Enqueued
                }
            }
            CompletionEventPayload::DidOpen { .. } | CompletionEventPayload::DidClose => {
                QueueEnqueueOutcome::Enqueued
            }
        }
    }

    fn apply_overflow_policy(
        entries: &mut VecDeque<CompletionEventEnvelope>,
        event: &CompletionEventEnvelope,
    ) -> Option<QueueEnqueueOutcome> {
        match &event.payload {
            CompletionEventPayload::Cancel { .. } => {
                if Self::remove_oldest_non_cancel(entries).is_some() {
                    Some(QueueEnqueueOutcome::EvictedNonCancelForCancel)
                } else if Self::remove_oldest_cancel(entries) {
                    // Queue is saturated by cancel events only. Keep latest cancellation intent
                    // by rotating out the oldest cancel event.
                    Some(QueueEnqueueOutcome::EvictedCancelForCancel)
                } else {
                    None
                }
            }
            CompletionEventPayload::CompletionRequest { .. } => {
                if Self::remove_oldest_stale_completion(entries, event.request_epoch) {
                    Some(QueueEnqueueOutcome::EvictedStaleCompletion)
                } else if Self::remove_oldest_did_change(entries) {
                    Some(QueueEnqueueOutcome::CoalescedDidChange)
                } else {
                    None
                }
            }
            CompletionEventPayload::DidChange { .. } => {
                if Self::remove_oldest_did_change(entries) {
                    Some(QueueEnqueueOutcome::CoalescedDidChange)
                } else if Self::remove_oldest_completion(entries) {
                    Some(QueueEnqueueOutcome::EvictedStaleCompletion)
                } else {
                    None
                }
            }
            CompletionEventPayload::DidOpen { .. } | CompletionEventPayload::DidClose => {
                if Self::remove_oldest_did_change(entries) {
                    Some(QueueEnqueueOutcome::CoalescedDidChange)
                } else if Self::remove_oldest_completion(entries) {
                    Some(QueueEnqueueOutcome::EvictedStaleCompletion)
                } else if Self::remove_oldest_non_cancel(entries).is_some() {
                    Some(QueueEnqueueOutcome::EvictedNonCancelForCancel)
                } else {
                    None
                }
            }
        }
    }

    fn remove_oldest_stale_completion(
        entries: &mut VecDeque<CompletionEventEnvelope>,
        incoming_epoch: u64,
    ) -> bool {
        let index = entries.iter().position(|queued| {
            matches!(
                queued.payload,
                CompletionEventPayload::CompletionRequest { .. }
            ) && queued.request_epoch < incoming_epoch
        });
        if let Some(index) = index {
            Self::remove_completion_at(entries, index);
            true
        } else {
            false
        }
    }

    fn remove_oldest_completion(entries: &mut VecDeque<CompletionEventEnvelope>) -> bool {
        let index = entries.iter().position(|queued| {
            matches!(
                queued.payload,
                CompletionEventPayload::CompletionRequest { .. }
            )
        });
        if let Some(index) = index {
            Self::remove_completion_at(entries, index);
            true
        } else {
            false
        }
    }

    fn remove_oldest_did_change(entries: &mut VecDeque<CompletionEventEnvelope>) -> bool {
        let index = entries
            .iter()
            .position(|queued| matches!(queued.payload, CompletionEventPayload::DidChange { .. }));
        if let Some(index) = index {
            entries.remove(index);
            true
        } else {
            false
        }
    }

    fn remove_oldest_non_cancel(
        entries: &mut VecDeque<CompletionEventEnvelope>,
    ) -> Option<CompletionEventEnvelope> {
        let index = entries
            .iter()
            .position(|queued| !matches!(queued.payload, CompletionEventPayload::Cancel { .. }))?;
        entries.remove(index)
    }

    fn remove_oldest_cancel(entries: &mut VecDeque<CompletionEventEnvelope>) -> bool {
        let index = entries
            .iter()
            .position(|queued| matches!(queued.payload, CompletionEventPayload::Cancel { .. }));
        if let Some(index) = index {
            entries.remove(index);
            true
        } else {
            false
        }
    }

    fn remove_completion_at(entries: &mut VecDeque<CompletionEventEnvelope>, index: usize) {
        let removed = entries.remove(index);
        let request_id = removed.and_then(|event| match event.payload {
            CompletionEventPayload::CompletionRequest {
                request_id,
                version_hint: _,
                trigger_mode: _,
            } => request_id,
            _ => None,
        });
        if let Some(request_id) = request_id {
            let _ = Self::remove_matching(entries, |queued| {
                matches!(
                    &queued.payload,
                    CompletionEventPayload::Cancel {
                        request_id: queued_request_id,
                    } if queued_request_id == &request_id
                )
            });
        }
    }

    fn remove_matching<F>(entries: &mut VecDeque<CompletionEventEnvelope>, mut predicate: F) -> bool
    where
        F: FnMut(&CompletionEventEnvelope) -> bool,
    {
        let mut removed = false;
        let mut kept = VecDeque::with_capacity(entries.len());
        for queued in entries.drain(..) {
            if predicate(&queued) {
                removed = true;
            } else {
                kept.push_back(queued);
            }
        }
        *entries = kept;
        removed
    }

    fn dropped_completion_file_seq(
        before_completion_file_seq: &HashSet<u64>,
        entries: &VecDeque<CompletionEventEnvelope>,
    ) -> Vec<u64> {
        let after_completion_file_seq: HashSet<u64> = entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.payload,
                    CompletionEventPayload::CompletionRequest { .. }
                )
            })
            .map(|entry| entry.file_seq)
            .collect();
        before_completion_file_seq
            .iter()
            .filter(|file_seq| !after_completion_file_seq.contains(file_seq))
            .copied()
            .collect()
    }
}

#[derive(Debug)]
struct CompletionTurnState {
    waiters: HashMap<u64, oneshot::Sender<CompletionTurnOutcome>>,
}

impl CompletionTurnState {
    fn register(&mut self, file_seq: u64, sender: oneshot::Sender<CompletionTurnOutcome>) {
        self.waiters.insert(file_seq, sender);
    }

    fn resolve(&mut self, file_seq: u64, outcome: CompletionTurnOutcome) -> bool {
        let Some(sender) = self.waiters.remove(&file_seq) else {
            return false;
        };
        let _ = sender.send(outcome);
        true
    }

    fn resolve_many(&mut self, file_seq: &[u64], outcome: CompletionTurnOutcome) -> usize {
        let mut resolved = 0usize;
        for seq in file_seq {
            if self.resolve(*seq, outcome) {
                resolved = resolved.saturating_add(1);
            }
        }
        resolved
    }

    fn resolve_all(&mut self, outcome: CompletionTurnOutcome) {
        let pending = std::mem::take(&mut self.waiters);
        for (_, sender) in pending {
            let _ = sender.send(outcome);
        }
    }
}

#[derive(Debug, Clone)]
struct CompletionEventQueueSummary {
    depth: usize,
    completion_request_count: usize,
    did_change_count: usize,
    first_completion: Option<CompletionTurnHolderSnapshot>,
}

#[derive(Debug, Clone)]
struct ActiveCompletionEntry {
    request_id: Option<String>,
    file_seq: u64,
    request_epoch: u64,
    trigger_mode: String,
    version_hint: Option<i32>,
    started_at: Instant,
}

#[derive(Debug)]
struct PerFileDispatcher {
    queue: CompletionEventQueue,
    next_file_seq: u64,
    latest_request_epoch: u64,
    latest_request_epoch_shared: Arc<AtomicU64>,
    turn_state: Arc<Mutex<CompletionTurnState>>,
    active_completions: Arc<Mutex<HashMap<u64, ActiveCompletionEntry>>>,
    drain_task: JoinHandle<()>,
}

impl PerFileDispatcher {
    fn new(capacity: usize) -> Self {
        let queue = CompletionEventQueue::new(capacity);
        let drain_queue = queue.clone();
        let latest_request_epoch_shared = Arc::new(AtomicU64::new(0));
        let turn_state = Arc::new(Mutex::new(CompletionTurnState {
            waiters: HashMap::new(),
        }));
        let active_completions = Arc::new(Mutex::new(HashMap::new()));
        let turn_state_for_drain = Arc::clone(&turn_state);
        let latest_epoch_for_drain = Arc::clone(&latest_request_epoch_shared);
        let drain_task = tokio::spawn(async move {
            while let Some(event) = drain_queue.recv().await {
                if matches!(
                    event.payload,
                    CompletionEventPayload::CompletionRequest { .. }
                ) {
                    let latest_epoch = latest_epoch_for_drain.load(Ordering::SeqCst);
                    let turn_outcome = if event.request_epoch < latest_epoch {
                        CompletionTurnOutcome::SupersededBeforeStart
                    } else {
                        CompletionTurnOutcome::Ready
                    };
                    let mut turn_state = turn_state_for_drain
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    let _ = turn_state.resolve(event.file_seq, turn_outcome);
                }
                consume_event(event);
            }
            let mut turn_state = turn_state_for_drain
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            turn_state.resolve_all(CompletionTurnOutcome::QueueRejected);
        });
        Self {
            queue,
            next_file_seq: 0,
            latest_request_epoch: 0,
            latest_request_epoch_shared,
            turn_state,
            active_completions,
            drain_task,
        }
    }

    fn next_ticket(&mut self, payload: &CompletionEventPayload) -> DispatchTicket {
        self.next_file_seq = self.next_file_seq.saturating_add(1);
        if matches!(payload, CompletionEventPayload::CompletionRequest { .. }) {
            self.latest_request_epoch = self.latest_request_epoch.saturating_add(1);
            self.latest_request_epoch_shared
                .store(self.latest_request_epoch, Ordering::SeqCst);
        }
        DispatchTicket {
            file_seq: self.next_file_seq,
            request_epoch: self.latest_request_epoch,
            queue_outcome: QueueEnqueueOutcome::Enqueued,
        }
    }

    fn register_turn_waiter(&self, file_seq: u64, sender: oneshot::Sender<CompletionTurnOutcome>) {
        let mut turn_state = self
            .turn_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        turn_state.register(file_seq, sender);
    }

    fn resolve_turn_waiter(&self, file_seq: u64, outcome: CompletionTurnOutcome) -> bool {
        let mut turn_state = self
            .turn_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        turn_state.resolve(file_seq, outcome)
    }

    fn resolve_turn_waiters(&self, file_seq: &[u64], outcome: CompletionTurnOutcome) -> usize {
        let mut turn_state = self
            .turn_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        turn_state.resolve_many(file_seq, outcome)
    }

    fn reject_all_turn_waiters(&self) {
        let mut turn_state = self
            .turn_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        turn_state.resolve_all(CompletionTurnOutcome::QueueRejected);
    }

    fn register_active_completion(
        &self,
        ticket: DispatchTicket,
        metadata: CompletionRequestMetadata,
    ) {
        let mut active = self
            .active_completions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        active.insert(
            ticket.file_seq,
            ActiveCompletionEntry {
                request_id: metadata.request_id,
                file_seq: ticket.file_seq,
                request_epoch: ticket.request_epoch,
                trigger_mode: metadata.trigger_mode,
                version_hint: metadata.version_hint,
                started_at: Instant::now(),
            },
        );
    }

    fn unregister_active_completion(&self, file_seq: u64) {
        let mut active = self
            .active_completions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        active.remove(&file_seq);
    }

    fn active_snapshot(&self, now: Instant) -> (usize, Option<CompletionTurnHolderSnapshot>) {
        let active = self
            .active_completions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let holder = active
            .values()
            .min_by_key(|entry| entry.started_at)
            .map(|entry| CompletionTurnHolderSnapshot {
                request_id: entry.request_id.clone(),
                file_seq: entry.file_seq,
                request_epoch: entry.request_epoch,
                trigger_mode: entry.trigger_mode.clone(),
                version_hint: entry.version_hint,
                age: now.saturating_duration_since(entry.started_at),
            });
        (active.len(), holder)
    }
}

fn consume_event(event: CompletionEventEnvelope) {
    let CompletionEventEnvelope {
        file_id,
        file_seq,
        request_epoch,
        received_at,
        payload,
    } = event;
    let _ = (file_id, file_seq, request_epoch, received_at);
    match payload {
        CompletionEventPayload::DidOpen { version } => {
            let _ = version;
        }
        CompletionEventPayload::DidChange { version } => {
            let _ = version;
        }
        CompletionEventPayload::CompletionRequest {
            request_id,
            version_hint,
            trigger_mode,
        } => {
            let _ = (request_id, version_hint, trigger_mode);
        }
        CompletionEventPayload::Cancel { request_id } => {
            let _ = request_id;
        }
        CompletionEventPayload::DidClose => {}
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionDispatcherRegistry {
    queue_capacity: Arc<AtomicUsize>,
    per_file: Arc<TokioMutex<HashMap<V2FileId, PerFileDispatcher>>>,
}

impl CompletionDispatcherRegistry {
    pub(crate) fn new(queue_capacity: usize) -> Self {
        Self {
            queue_capacity: Arc::new(AtomicUsize::new(queue_capacity.max(1))),
            per_file: Arc::new(TokioMutex::new(HashMap::new())),
        }
    }

    fn queue_capacity(&self) -> usize {
        self.queue_capacity.load(Ordering::SeqCst).max(1)
    }

    pub(crate) async fn set_queue_capacity(&self, queue_capacity: usize) {
        let normalized = queue_capacity.max(1);
        self.queue_capacity.store(normalized, Ordering::SeqCst);
        let per_file = self.per_file.lock().await;
        for dispatcher in per_file.values() {
            dispatcher.queue.set_capacity(normalized);
        }
    }

    pub(crate) async fn emit_did_open(&self, file_id: V2FileId, version: i32) -> DispatchTicket {
        self.emit(file_id, CompletionEventPayload::DidOpen { version })
            .await
    }

    pub(crate) async fn emit_did_change(&self, file_id: V2FileId, version: i32) -> DispatchTicket {
        self.emit(file_id, CompletionEventPayload::DidChange { version })
            .await
    }

    pub(crate) async fn emit_completion_request_with_turn(
        &self,
        file_id: V2FileId,
        request_id: Option<String>,
        version_hint: Option<i32>,
        trigger_mode: String,
    ) -> CompletionRequestDispatch {
        let mut per_file = self.per_file.lock().await;
        let dispatcher = per_file
            .entry(file_id)
            .or_insert_with(|| PerFileDispatcher::new(self.queue_capacity()));
        let observed_at = Instant::now();
        let queue_before = dispatcher.queue.summary(observed_at);
        let (active_completion_count, active_holder) = dispatcher.active_snapshot(observed_at);
        let payload = CompletionEventPayload::CompletionRequest {
            request_id,
            version_hint,
            trigger_mode,
        };
        let mut ticket = dispatcher.next_ticket(&payload);
        let event = CompletionEventEnvelope {
            file_id,
            file_seq: ticket.file_seq,
            request_epoch: ticket.request_epoch,
            received_at: Instant::now(),
            payload,
        };
        let (sender, receiver) = oneshot::channel();
        dispatcher.register_turn_waiter(ticket.file_seq, sender);

        let (queue_outcome, dropped_completion_file_seq) =
            dispatcher.queue.try_enqueue_with_report(event);
        let queue_after = dispatcher.queue.summary(observed_at);
        ticket.queue_outcome = queue_outcome;
        let attribution = CompletionDispatchAttributionSnapshot {
            request_file_seq: ticket.file_seq,
            request_epoch: ticket.request_epoch,
            queue_outcome,
            queue_capacity: dispatcher.queue.capacity(),
            queue_depth_before_enqueue: queue_before.depth,
            queue_depth_after_enqueue: queue_after.depth,
            queued_completion_ahead_count: queue_before.completion_request_count,
            did_change_ahead_count: queue_before.did_change_count,
            active_completion_count,
            dropped_completion_file_seq: dropped_completion_file_seq.clone(),
            active_holder,
            queued_completion_ahead: queue_before.first_completion,
        };
        let _ = dispatcher.resolve_turn_waiters(
            &dropped_completion_file_seq,
            CompletionTurnOutcome::SupersededBeforeStart,
        );
        if matches!(
            queue_outcome,
            QueueEnqueueOutcome::Full | QueueEnqueueOutcome::Closed
        ) {
            let _ = dispatcher
                .resolve_turn_waiter(ticket.file_seq, CompletionTurnOutcome::QueueRejected);
            return CompletionRequestDispatch {
                ticket,
                turn_waiter: None,
                attribution,
            };
        }

        CompletionRequestDispatch {
            ticket,
            turn_waiter: Some(CompletionTurnWaiter { receiver }),
            attribution,
        }
    }

    pub(crate) async fn emit_cancel(
        &self,
        file_id: V2FileId,
        request_id: String,
    ) -> DispatchTicket {
        self.emit(file_id, CompletionEventPayload::Cancel { request_id })
            .await
    }

    pub(crate) async fn emit_did_close(&self, file_id: V2FileId) -> DispatchTicket {
        self.emit(file_id, CompletionEventPayload::DidClose).await
    }

    pub(crate) async fn latest_request_epoch(&self, file_id: V2FileId) -> Option<u64> {
        let per_file = self.per_file.lock().await;
        per_file
            .get(&file_id)
            .map(|dispatcher| dispatcher.latest_request_epoch)
    }

    #[cfg(test)]
    pub(crate) async fn debug_state(&self, file_id: V2FileId) -> Option<(u64, u64)> {
        let per_file = self.per_file.lock().await;
        per_file
            .get(&file_id)
            .map(|dispatcher| (dispatcher.next_file_seq, dispatcher.latest_request_epoch))
    }

    pub(crate) async fn close_file_dispatcher(&self, file_id: V2FileId) -> Option<DispatchTicket> {
        let has_dispatcher = {
            let per_file = self.per_file.lock().await;
            per_file.contains_key(&file_id)
        };
        if !has_dispatcher {
            return None;
        }

        let ticket = self.emit_did_close(file_id).await;
        let dispatcher = {
            let mut per_file = self.per_file.lock().await;
            per_file.remove(&file_id)
        };
        if let Some(dispatcher) = dispatcher {
            dispatcher.reject_all_turn_waiters();
            dispatcher.queue.close();
            let _ = tokio::time::timeout(Duration::from_millis(50), dispatcher.drain_task).await;
        }
        Some(ticket)
    }

    pub(crate) async fn mark_completion_active(
        &self,
        file_id: V2FileId,
        ticket: DispatchTicket,
        metadata: CompletionRequestMetadata,
    ) -> bool {
        let per_file = self.per_file.lock().await;
        let Some(dispatcher) = per_file.get(&file_id) else {
            return false;
        };
        dispatcher.register_active_completion(ticket, metadata);
        true
    }

    pub(crate) async fn mark_completion_inactive(&self, file_id: V2FileId, file_seq: u64) -> bool {
        let per_file = self.per_file.lock().await;
        let Some(dispatcher) = per_file.get(&file_id) else {
            return false;
        };
        dispatcher.unregister_active_completion(file_seq);
        true
    }

    async fn emit(&self, file_id: V2FileId, payload: CompletionEventPayload) -> DispatchTicket {
        let mut per_file = self.per_file.lock().await;
        let dispatcher = per_file
            .entry(file_id)
            .or_insert_with(|| PerFileDispatcher::new(self.queue_capacity()));
        let mut ticket = dispatcher.next_ticket(&payload);
        let event = CompletionEventEnvelope {
            file_id,
            file_seq: ticket.file_seq,
            request_epoch: ticket.request_epoch,
            received_at: Instant::now(),
            payload,
        };
        let (queue_outcome, dropped_completion_file_seq) =
            dispatcher.queue.try_enqueue_with_report(event);
        ticket.queue_outcome = queue_outcome;
        let _ = dispatcher.resolve_turn_waiters(
            &dropped_completion_file_seq,
            CompletionTurnOutcome::SupersededBeforeStart,
        );
        ticket
    }
}

#[cfg(test)]
#[path = "completion_dispatcher/tests.rs"]
mod tests;
