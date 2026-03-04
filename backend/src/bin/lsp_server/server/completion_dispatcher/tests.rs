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
fn queue_rotates_oldest_cancel_when_saturated_by_cancel_events() {
    let queue = CompletionEventQueue::new(2);
    let first = queue.try_enqueue(make_event(
        1,
        1,
        CompletionEventPayload::Cancel {
            request_id: "r1".to_string(),
        },
    ));
    assert_eq!(first, QueueEnqueueOutcome::Enqueued);
    let second = queue.try_enqueue(make_event(
        2,
        1,
        CompletionEventPayload::Cancel {
            request_id: "r2".to_string(),
        },
    ));
    assert_eq!(second, QueueEnqueueOutcome::Enqueued);

    let third = queue.try_enqueue(make_event(
        3,
        1,
        CompletionEventPayload::Cancel {
            request_id: "r3".to_string(),
        },
    ));
    assert_eq!(third, QueueEnqueueOutcome::EvictedCancelForCancel);

    let payloads = queue.debug_payloads();
    assert_eq!(payloads.len(), 2);
    assert!(payloads.iter().any(|payload| matches!(
        payload,
        CompletionEventPayload::Cancel { request_id } if request_id == "r2"
    )));
    assert!(payloads.iter().any(|payload| matches!(
        payload,
        CompletionEventPayload::Cancel { request_id } if request_id == "r3"
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

#[test]
fn queue_report_lists_dropped_completion_file_seq_on_latest_wins_eviction() {
    let queue = CompletionEventQueue::new(4);
    let (first_outcome, first_dropped) = queue.try_enqueue_with_report(make_event(
        1,
        1,
        CompletionEventPayload::CompletionRequest {
            request_id: Some("r1".to_string()),
            version_hint: Some(1),
            trigger_mode: "invoked".to_string(),
        },
    ));
    assert_eq!(first_outcome, QueueEnqueueOutcome::Enqueued);
    assert!(first_dropped.is_empty());

    let (second_outcome, second_dropped) = queue.try_enqueue_with_report(make_event(
        2,
        2,
        CompletionEventPayload::CompletionRequest {
            request_id: Some("r2".to_string()),
            version_hint: Some(2),
            trigger_mode: "invoked".to_string(),
        },
    ));
    assert_eq!(second_outcome, QueueEnqueueOutcome::EvictedStaleCompletion);
    assert_eq!(
        second_dropped,
        vec![1],
        "dropped report must include stale completion file_seq"
    );
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
        .emit_completion_request_with_turn(file_id, None, Some(2), "invoked".to_string())
        .await;
    assert_eq!(completion.ticket.file_seq, 3);
    assert_eq!(completion.ticket.request_epoch, 1);
    let completion_outcome = tokio::time::timeout(
        Duration::from_millis(200),
        completion
            .turn_waiter
            .expect("turn waiter must be present")
            .wait(),
    )
    .await
    .expect("turn waiter timeout");
    assert_eq!(completion_outcome, CompletionTurnOutcome::Ready);

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

#[tokio::test(flavor = "current_thread")]
async fn newer_turn_request_supersedes_pending_stale_request_before_start() {
    let registry = CompletionDispatcherRegistry::new(8);
    let file_id = V2FileId(77);

    let first = registry
        .emit_completion_request_with_turn(
            file_id,
            Some("r1".to_string()),
            Some(1),
            "invoked".to_string(),
        )
        .await;
    let second = registry
        .emit_completion_request_with_turn(
            file_id,
            Some("r2".to_string()),
            Some(2),
            "invoked".to_string(),
        )
        .await;

    assert_eq!(first.ticket.request_epoch, 1);
    assert_eq!(second.ticket.request_epoch, 2);

    let first_outcome = tokio::time::timeout(
        Duration::from_millis(200),
        first.turn_waiter.expect("first waiter").wait(),
    )
    .await
    .expect("first waiter timeout");
    assert_eq!(first_outcome, CompletionTurnOutcome::SupersededBeforeStart);

    let second_outcome = tokio::time::timeout(
        Duration::from_millis(200),
        second.turn_waiter.expect("second waiter").wait(),
    )
    .await
    .expect("second waiter timeout");
    assert_eq!(second_outcome, CompletionTurnOutcome::Ready);

    let close = registry.close_file_dispatcher(file_id).await;
    assert!(close.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn turn_dispatch_returns_queue_rejected_when_queue_saturated_by_cancel_only() {
    let registry = CompletionDispatcherRegistry::new(1);
    let file_id = V2FileId(99);

    let _ = registry.emit_cancel(file_id, "occupy".to_string()).await;
    let dispatch = registry
        .emit_completion_request_with_turn(
            file_id,
            Some("r1".to_string()),
            Some(1),
            "invoked".to_string(),
        )
        .await;

    assert_eq!(dispatch.ticket.request_epoch, 1);
    assert_eq!(dispatch.ticket.queue_outcome, QueueEnqueueOutcome::Full);
    assert!(
        dispatch.turn_waiter.is_none(),
        "queue rejected dispatch must not expose waiter"
    );

    let close = registry.close_file_dispatcher(file_id).await;
    assert!(close.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn queue_capacity_update_applies_to_existing_dispatchers() {
    let registry = CompletionDispatcherRegistry::new(1);
    let file_id = V2FileId(123);

    let _ = registry.emit_cancel(file_id, "occupy".to_string()).await;
    registry.set_queue_capacity(2).await;

    let dispatch = registry
        .emit_completion_request_with_turn(
            file_id,
            Some("r1".to_string()),
            Some(1),
            "invoked".to_string(),
        )
        .await;

    assert_ne!(
        dispatch.ticket.queue_outcome,
        QueueEnqueueOutcome::Full,
        "updated capacity must unblock completion enqueue"
    );

    let close = registry.close_file_dispatcher(file_id).await;
    assert!(close.is_some());
}
