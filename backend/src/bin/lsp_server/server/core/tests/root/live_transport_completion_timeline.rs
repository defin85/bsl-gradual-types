#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]

fn create_diagnostics_save_timeline_test_server() -> BslLanguageServer {
    let coordinator = Arc::new(SystemCoordinator::new());
    let holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (_service, _socket) = LspService::build({
        let coordinator = coordinator.clone();
        let holder = holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let server = holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");
    server
}

async fn diagnostics_save_timeline_trace_for_test(
    server: &BslLanguageServer,
    uri: &Url,
    key: crate::server::DiagnosticsSaveTimelineCycleKey,
) -> crate::types::DiagnosticsSaveTimelineTrace {
    server
        .handle_get_diagnostics_save_timeline(crate::types::DiagnosticsSaveTimelineRequest {
            limit: Some(16),
        })
        .await
        .expect("diagnostics save timeline response")
        .traces
        .into_iter()
        .find(|trace| {
            trace.uri == uri.to_string() && trace.requested_version == key.requested_version
        })
        .expect("matching diagnostics save timeline trace")
}

#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[cfg(unix)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn p24_live_transport_completion_timeline_exposes_handoff_aware_server_edge_split() {
    const FIXTURE: &str =
        "Процедура Тест()\n    ДляCompletion = (Новый Массив()).\nКонецПроцедуры\n";
    const REQUEST_ID: i64 = 50_521;

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///completion_timeline_flush_split_fixture.bsl").expect("uri");
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: FIXTURE.to_string(),
            },
        })
        .await;
    server.sync_v2_globals().await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "(Новый Массив()).");
    let completion_response = harness
        .send_request(
            REQUEST_ID,
            "textDocument/completion",
            CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: completion_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                    trigger_character: Some(".".to_string()),
                }),
            },
        )
        .await;
    assert!(
        completion_response.get("result").is_some(),
        "completion should return a response over live transport"
    );

    let timeline = live_transport_get_completion_timeline(&mut harness, 50_522, 16).await;
    assert_eq!(
        timeline.get("version").and_then(|value| value.as_u64()),
        Some(crate::server::COMPLETION_TIMELINE_VERSION as u64),
        "live transport completion timeline must expose the current payload version"
    );
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .iter()
        .find(|trace| {
            trace.get("request_id").and_then(|value| value.as_str())
                == Some(&REQUEST_ID.to_string())
        })
        .cloned()
        .expect("live completion trace must be immediately visible in timeline after response");

    let response_sent_at_ms = completion_timeline_server_edge_u64(&trace, "response_sent_at_ms")
        .expect("response_sent_at_ms");
    let response_output_handoff_started_at_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_handoff_started_at_ms")
            .expect("response_output_handoff_started_at_ms");
    let response_output_handoff_enqueued_at_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_handoff_enqueued_at_ms")
            .expect("response_output_handoff_enqueued_at_ms");
    let response_output_enqueue_completed_at_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_enqueue_completed_at_ms")
            .expect("response_output_enqueue_completed_at_ms");
    let response_output_encode_started_at_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_encode_started_at_ms")
            .expect("response_output_encode_started_at_ms");
    let response_output_write_started_at_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_write_started_at_ms")
            .expect("response_output_write_started_at_ms");
    let response_output_encode_completed_at_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_encode_completed_at_ms")
            .expect("response_output_encode_completed_at_ms");
    let response_flush_completed_at_ms =
        completion_timeline_server_edge_u64(&trace, "response_flush_completed_at_ms")
            .expect("response_flush_completed_at_ms");
    let response_ready_to_output_enqueue_wait_ms =
        completion_timeline_server_edge_u64(&trace, "response_ready_to_output_enqueue_wait_ms")
            .expect("response_ready_to_output_enqueue_wait_ms");
    let response_ready_to_output_handoff_wait_ms =
        completion_timeline_server_edge_u64(&trace, "response_ready_to_output_handoff_wait_ms")
            .expect("response_ready_to_output_handoff_wait_ms");
    let response_output_handoff_send_wait_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_handoff_send_wait_ms")
            .expect("response_output_handoff_send_wait_ms");
    let response_output_handoff_to_writer_wait_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_handoff_to_writer_wait_ms")
            .expect("response_output_handoff_to_writer_wait_ms");
    let response_output_queue_wait_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_queue_wait_ms")
            .expect("response_output_queue_wait_ms");
    let response_output_encode_exec_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_encode_exec_ms")
            .expect("response_output_encode_exec_ms");
    let response_output_write_and_flush_exec_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_write_and_flush_exec_ms")
            .expect("response_output_write_and_flush_exec_ms");
    let response_ready_to_flush_wait_ms =
        completion_timeline_server_edge_u64(&trace, "response_ready_to_flush_wait_ms")
            .expect("response_ready_to_flush_wait_ms");
    assert!(
        response_sent_at_ms <= response_output_handoff_started_at_ms,
        "handoff start must not precede handler-ready boundary, trace={trace:?}"
    );
    assert!(
        response_output_handoff_started_at_ms <= response_output_handoff_enqueued_at_ms,
        "handoff enqueue acceptance must not precede handoff start, trace={trace:?}"
    );
    assert!(
        response_output_handoff_enqueued_at_ms <= response_flush_completed_at_ms,
        "handoff acceptance must not outlive flush completion, trace={trace:?}"
    );
    assert!(
        response_sent_at_ms <= response_output_handoff_enqueued_at_ms,
        "handoff acceptance must not precede handler-ready boundary, trace={trace:?}"
    );
    // `response_output_enqueue_completed_at_ms` remains a legacy writer-selection compatibility
    // seam on v24 payloads. It is not a truthful acceptance boundary and can precede the
    // separately recorded handoff acceptance timestamp on live transport.
    assert!(
        response_sent_at_ms <= response_output_enqueue_completed_at_ms,
        "egress enqueue must not precede handler-ready boundary, trace={trace:?}"
    );
    assert!(
        response_output_enqueue_completed_at_ms <= response_output_encode_started_at_ms,
        "egress encode-start must not precede enqueue completion, trace={trace:?}"
    );
    assert!(
        response_output_encode_started_at_ms <= response_output_encode_completed_at_ms,
        "egress encode completion must not precede encode-start boundary, trace={trace:?}"
    );
    assert!(
        response_output_encode_completed_at_ms <= response_output_write_started_at_ms,
        "egress write-start must not precede encode completion, trace={trace:?}"
    );
    assert!(
        response_output_write_started_at_ms <= response_flush_completed_at_ms,
        "flush completion must not precede literal write-start boundary, trace={trace:?}"
    );
    assert_eq!(
        response_ready_to_output_handoff_wait_ms,
        response_output_handoff_started_at_ms.saturating_sub(response_sent_at_ms),
        "response_ready_to_output_handoff_wait_ms must match handler-ready to handoff-start delta, trace={trace:?}"
    );
    assert_eq!(
        response_output_handoff_send_wait_ms,
        response_output_handoff_enqueued_at_ms
            .saturating_sub(response_output_handoff_started_at_ms),
        "response_output_handoff_send_wait_ms must match handoff-start to handoff-accept delta, trace={trace:?}"
    );
    assert_eq!(
        response_output_handoff_to_writer_wait_ms,
        response_output_enqueue_completed_at_ms
            .saturating_sub(response_output_handoff_enqueued_at_ms),
        "response_output_handoff_to_writer_wait_ms must match handoff-accept to writer-selection delta, trace={trace:?}"
    );
    assert_eq!(
        response_ready_to_output_enqueue_wait_ms,
        response_output_enqueue_completed_at_ms.saturating_sub(response_sent_at_ms),
        "response_ready_to_output_enqueue_wait_ms must match handler-ready to outbound-path delta, trace={trace:?}"
    );
    assert_eq!(
        response_output_queue_wait_ms,
        response_output_encode_started_at_ms
            .saturating_sub(response_output_enqueue_completed_at_ms),
        "response_output_queue_wait_ms must match outbound enqueue to encode-start delta, trace={trace:?}"
    );
    assert_eq!(
        response_output_encode_exec_ms,
        response_output_encode_completed_at_ms.saturating_sub(response_output_encode_started_at_ms),
        "response_output_encode_exec_ms must match encode-start to encode-complete delta, trace={trace:?}"
    );
    assert_eq!(
        response_output_write_and_flush_exec_ms,
        response_flush_completed_at_ms.saturating_sub(response_output_write_started_at_ms),
        "response_output_write_and_flush_exec_ms must match write-start to flush-complete delta, trace={trace:?}"
    );
    assert_eq!(
        response_ready_to_flush_wait_ms,
        response_flush_completed_at_ms.saturating_sub(response_sent_at_ms),
        "response_ready_to_flush_wait_ms must match handler-ready to flush delta, trace={trace:?}"
    );

    let metrics = live_transport_get_observability_metrics(&mut harness, 50_523).await;
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    for key in [
        "completion_stage_response_ready_to_output_handoff_wait_ms",
        "completion_stage_response_output_handoff_send_wait_ms",
        "completion_stage_response_output_handoff_to_writer_wait_ms",
        "completion_stage_response_ready_to_output_enqueue_wait_ms",
        "completion_stage_response_output_queue_wait_ms",
        "completion_stage_response_output_encode_exec_ms",
        "completion_stage_response_output_write_and_flush_exec_ms",
        "completion_stage_response_ready_to_flush_wait_ms",
    ] {
        let count = histograms
            .get(key)
            .and_then(|value| value.get("count"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        assert!(
            count > 0,
            "{key} must be exported after live completion response"
        );
    }

    live_transport_close_document(&mut harness, &uri).await;
    harness.shutdown().await;
}
