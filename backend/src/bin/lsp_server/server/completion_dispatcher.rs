use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bsl_analysis_v2::FileId as V2FileId;
use tokio::sync::{Mutex as TokioMutex, Notify};
use tokio::task::JoinHandle;

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
    Full,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DispatchTicket {
    pub file_seq: u64,
    pub request_epoch: u64,
    pub queue_outcome: QueueEnqueueOutcome,
}

#[derive(Debug, Default)]
struct CompletionEventQueueState {
    entries: VecDeque<CompletionEventEnvelope>,
    closed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionEventQueue {
    capacity: usize,
    state: Arc<Mutex<CompletionEventQueueState>>,
    notify: Arc<Notify>,
}

impl CompletionEventQueue {
    fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            state: Arc::new(Mutex::new(CompletionEventQueueState::default())),
            notify: Arc::new(Notify::new()),
        }
    }

    fn try_enqueue(&self, event: CompletionEventEnvelope) -> QueueEnqueueOutcome {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.closed {
            return QueueEnqueueOutcome::Closed;
        }

        let mut outcome = Self::apply_pre_enqueue_policy(&mut state.entries, &event);
        if matches!(outcome, QueueEnqueueOutcome::CoalescedCancel) {
            return outcome;
        }

        if state.entries.len() >= self.capacity {
            let overflow_outcome = Self::apply_overflow_policy(&mut state.entries, &event);
            let Some(overflow_outcome) = overflow_outcome else {
                return QueueEnqueueOutcome::Full;
            };
            outcome = overflow_outcome;
        }

        if state.entries.len() >= self.capacity {
            return QueueEnqueueOutcome::Full;
        }

        state.entries.push_back(event);
        drop(state);
        self.notify.notify_one();
        outcome
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
                } else if entries.pop_front().is_some() {
                    Some(QueueEnqueueOutcome::EvictedNonCancelForCancel)
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
}

#[derive(Debug)]
struct PerFileDispatcher {
    queue: CompletionEventQueue,
    next_file_seq: u64,
    latest_request_epoch: u64,
    drain_task: JoinHandle<()>,
}

impl PerFileDispatcher {
    fn new(capacity: usize) -> Self {
        let queue = CompletionEventQueue::new(capacity);
        let drain_queue = queue.clone();
        let drain_task = tokio::spawn(async move {
            while let Some(event) = drain_queue.recv().await {
                consume_event(event);
            }
        });
        Self {
            queue,
            next_file_seq: 0,
            latest_request_epoch: 0,
            drain_task,
        }
    }

    fn next_ticket(&mut self, payload: &CompletionEventPayload) -> DispatchTicket {
        self.next_file_seq = self.next_file_seq.saturating_add(1);
        if matches!(payload, CompletionEventPayload::CompletionRequest { .. }) {
            self.latest_request_epoch = self.latest_request_epoch.saturating_add(1);
        }
        DispatchTicket {
            file_seq: self.next_file_seq,
            request_epoch: self.latest_request_epoch,
            queue_outcome: QueueEnqueueOutcome::Enqueued,
        }
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
    queue_capacity: usize,
    per_file: Arc<TokioMutex<HashMap<V2FileId, PerFileDispatcher>>>,
}

impl CompletionDispatcherRegistry {
    pub(crate) fn new(queue_capacity: usize) -> Self {
        Self {
            queue_capacity: queue_capacity.max(1),
            per_file: Arc::new(TokioMutex::new(HashMap::new())),
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

    pub(crate) async fn emit_completion_request(
        &self,
        file_id: V2FileId,
        request_id: Option<String>,
        version_hint: Option<i32>,
        trigger_mode: String,
    ) -> DispatchTicket {
        self.emit(
            file_id,
            CompletionEventPayload::CompletionRequest {
                request_id,
                version_hint,
                trigger_mode,
            },
        )
        .await
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
            dispatcher.queue.close();
            let _ = tokio::time::timeout(Duration::from_millis(50), dispatcher.drain_task).await;
        }
        Some(ticket)
    }

    async fn emit(&self, file_id: V2FileId, payload: CompletionEventPayload) -> DispatchTicket {
        let mut per_file = self.per_file.lock().await;
        let dispatcher = per_file
            .entry(file_id)
            .or_insert_with(|| PerFileDispatcher::new(self.queue_capacity));
        let mut ticket = dispatcher.next_ticket(&payload);
        let event = CompletionEventEnvelope {
            file_id,
            file_seq: ticket.file_seq,
            request_epoch: ticket.request_epoch,
            received_at: Instant::now(),
            payload,
        };
        ticket.queue_outcome = dispatcher.queue.try_enqueue(event);
        ticket
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(
        file_seq: u64,
        request_epoch: u64,
        payload: CompletionEventPayload,
    ) -> CompletionEventEnvelope {
        CompletionEventEnvelope {
            file_id: V2FileId(1),
            file_seq,
            request_epoch,
            received_at: Instant::now(),
            payload,
        }
    }

    #[test]
    fn queue_coalesces_did_change_to_latest_revision() {
        let queue = CompletionEventQueue::new(4);
        let first = queue.try_enqueue(make_event(
            1,
            0,
            CompletionEventPayload::DidChange { version: 1 },
        ));
        assert_eq!(first, QueueEnqueueOutcome::Enqueued);

        let second = queue.try_enqueue(make_event(
            2,
            0,
            CompletionEventPayload::DidChange { version: 2 },
        ));
        assert_eq!(second, QueueEnqueueOutcome::CoalescedDidChange);

        let payloads = queue.debug_payloads();
        assert_eq!(payloads.len(), 1);
        assert!(matches!(
            payloads[0],
            CompletionEventPayload::DidChange { version: 2 }
        ));
    }

    #[test]
    fn queue_evicts_stale_completion_on_overflow() {
        let queue = CompletionEventQueue::new(2);

        let first = queue.try_enqueue(make_event(
            1,
            1,
            CompletionEventPayload::CompletionRequest {
                request_id: Some("r1".to_string()),
                version_hint: Some(1),
                trigger_mode: "invoked".to_string(),
            },
        ));
        assert_eq!(first, QueueEnqueueOutcome::Enqueued);

        let second = queue.try_enqueue(make_event(
            2,
            2,
            CompletionEventPayload::CompletionRequest {
                request_id: Some("r2".to_string()),
                version_hint: Some(2),
                trigger_mode: "invoked".to_string(),
            },
        ));
        assert_eq!(second, QueueEnqueueOutcome::EvictedStaleCompletion);

        let third = queue.try_enqueue(make_event(
            3,
            3,
            CompletionEventPayload::CompletionRequest {
                request_id: Some("r3".to_string()),
                version_hint: Some(3),
                trigger_mode: "invoked".to_string(),
            },
        ));
        assert_eq!(third, QueueEnqueueOutcome::EvictedStaleCompletion);

        let payloads = queue.debug_payloads();
        assert_eq!(payloads.len(), 1);
        assert!(matches!(
            payloads[0],
            CompletionEventPayload::CompletionRequest { .. }
        ));
    }

    #[test]
    fn queue_prioritizes_cancel_when_full() {
        let queue = CompletionEventQueue::new(2);
        let _ = queue.try_enqueue(make_event(
            1,
            1,
            CompletionEventPayload::CompletionRequest {
                request_id: Some("r1".to_string()),
                version_hint: Some(1),
                trigger_mode: "invoked".to_string(),
            },
        ));
        let _ = queue.try_enqueue(make_event(
            2,
            1,
            CompletionEventPayload::DidChange { version: 7 },
        ));

        let cancel = queue.try_enqueue(make_event(
            3,
            1,
            CompletionEventPayload::Cancel {
                request_id: "r1".to_string(),
            },
        ));
        assert_eq!(cancel, QueueEnqueueOutcome::EvictedNonCancelForCancel);

        let payloads = queue.debug_payloads();
        assert_eq!(payloads.len(), 2);
        assert!(!payloads
            .iter()
            .any(|payload| matches!(payload, CompletionEventPayload::CompletionRequest { .. })));
        assert!(payloads
            .iter()
            .any(|payload| matches!(payload, CompletionEventPayload::DidChange { .. })));
        assert!(payloads.iter().any(|payload| matches!(
            payload,
            CompletionEventPayload::Cancel { request_id } if request_id == "r1"
        )));
    }

    #[test]
    fn queue_coalesces_duplicate_cancel() {
        let queue = CompletionEventQueue::new(3);
        let _ = queue.try_enqueue(make_event(
            1,
            1,
            CompletionEventPayload::CompletionRequest {
                request_id: Some("r1".to_string()),
                version_hint: Some(1),
                trigger_mode: "invoked".to_string(),
            },
        ));
        let _ = queue.try_enqueue(make_event(
            2,
            1,
            CompletionEventPayload::Cancel {
                request_id: "r1".to_string(),
            },
        ));
        let duplicate = queue.try_enqueue(make_event(
            3,
            1,
            CompletionEventPayload::Cancel {
                request_id: "r1".to_string(),
            },
        ));

        assert_eq!(duplicate, QueueEnqueueOutcome::CoalescedCancel);
        assert_eq!(queue.debug_payloads().len(), 2);
    }

    #[test]
    fn queue_burst_latest_wins_keeps_latest_did_change_and_completion() {
        let queue = CompletionEventQueue::new(8);

        for version in 1..=5_i32 {
            let did_change = queue.try_enqueue(make_event(
                (version as u64) * 10,
                0,
                CompletionEventPayload::DidChange { version },
            ));
            assert!(matches!(
                did_change,
                QueueEnqueueOutcome::Enqueued | QueueEnqueueOutcome::CoalescedDidChange
            ));

            let request_id = format!("r{version}");
            let completion = queue.try_enqueue(make_event(
                (version as u64) * 10 + 1,
                version as u64,
                CompletionEventPayload::CompletionRequest {
                    request_id: Some(request_id),
                    version_hint: Some(version),
                    trigger_mode: "invoked".to_string(),
                },
            ));
            assert!(matches!(
                completion,
                QueueEnqueueOutcome::Enqueued | QueueEnqueueOutcome::EvictedStaleCompletion
            ));
        }

        let envelopes = queue.debug_envelopes();
        assert_eq!(envelopes.len(), 2, "burst should keep bounded latest state");

        let file_seq: Vec<u64> = envelopes.iter().map(|entry| entry.file_seq).collect();
        assert!(
            file_seq.windows(2).all(|pair| pair[0] < pair[1]),
            "queued envelopes must stay ordered by file_seq, got {file_seq:?}"
        );

        assert!(matches!(
            envelopes[0].payload,
            CompletionEventPayload::DidChange { version: 5 }
        ));
        assert!(matches!(
            &envelopes[1].payload,
            CompletionEventPayload::CompletionRequest { request_id, .. }
                if request_id.as_deref() == Some("r5")
        ));
        assert_eq!(
            envelopes[1].request_epoch, 5,
            "latest completion epoch should survive burst coalescing"
        );
    }

    #[test]
    fn newer_completion_evicts_stale_cancel_for_previous_request() {
        let queue = CompletionEventQueue::new(4);

        let first = queue.try_enqueue(make_event(
            1,
            1,
            CompletionEventPayload::CompletionRequest {
                request_id: Some("r1".to_string()),
                version_hint: Some(1),
                trigger_mode: "invoked".to_string(),
            },
        ));
        assert_eq!(first, QueueEnqueueOutcome::Enqueued);

        let cancel = queue.try_enqueue(make_event(
            2,
            1,
            CompletionEventPayload::Cancel {
                request_id: "r1".to_string(),
            },
        ));
        assert_eq!(cancel, QueueEnqueueOutcome::Enqueued);

        let second = queue.try_enqueue(make_event(
            3,
            2,
            CompletionEventPayload::CompletionRequest {
                request_id: Some("r2".to_string()),
                version_hint: Some(2),
                trigger_mode: "invoked".to_string(),
            },
        ));
        assert_eq!(second, QueueEnqueueOutcome::EvictedStaleCompletion);

        let payloads = queue.debug_payloads();
        assert_eq!(payloads.len(), 1);
        assert!(matches!(
            &payloads[0],
            CompletionEventPayload::CompletionRequest { request_id, .. }
                if request_id.as_deref() == Some("r2")
        ));
    }

    #[tokio::test]
    async fn request_epoch_advances_only_for_completion_requests() {
        let registry = CompletionDispatcherRegistry::new(8);
        let file_id = V2FileId(7);

        let open = registry.emit_did_open(file_id, 1).await;
        assert_eq!(open.file_seq, 1);
        assert_eq!(open.request_epoch, 0);
        assert_eq!(open.queue_outcome, QueueEnqueueOutcome::Enqueued);

        let change = registry.emit_did_change(file_id, 2).await;
        assert_eq!(change.file_seq, 2);
        assert_eq!(change.request_epoch, 0);

        let completion = registry
            .emit_completion_request(file_id, None, Some(2), "invoked".to_string())
            .await;
        assert_eq!(completion.file_seq, 3);
        assert_eq!(completion.request_epoch, 1);

        let cancel = registry.emit_cancel(file_id, "42".to_string()).await;
        assert_eq!(cancel.file_seq, 4);
        assert_eq!(cancel.request_epoch, 1);

        let close = registry.emit_did_close(file_id).await;
        assert_eq!(close.file_seq, 5);
        assert_eq!(close.request_epoch, 1);
    }

    #[tokio::test]
    async fn removing_dispatcher_resets_file_sequence() {
        let registry = CompletionDispatcherRegistry::new(8);
        let file_id = V2FileId(11);

        let first = registry.emit_did_open(file_id, 1).await;
        assert_eq!(first.file_seq, 1);
        let close = registry.close_file_dispatcher(file_id).await;
        assert!(close.is_some());
        let second = registry.emit_did_open(file_id, 2).await;
        assert_eq!(second.file_seq, 1);
        assert_eq!(second.request_epoch, 0);
    }

    #[tokio::test]
    async fn close_file_dispatcher_is_noop_when_dispatcher_absent() {
        let registry = CompletionDispatcherRegistry::new(8);
        let file_id = V2FileId(42);

        let close = registry.close_file_dispatcher(file_id).await;
        assert!(close.is_none());

        let open = registry.emit_did_open(file_id, 1).await;
        assert_eq!(open.file_seq, 1);
    }
}
