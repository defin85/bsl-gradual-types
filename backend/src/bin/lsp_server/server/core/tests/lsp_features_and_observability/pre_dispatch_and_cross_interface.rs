#[test]
fn pre_dispatch_cancelled_completion_trace_is_server_centric_and_has_no_fabricated_post_dispatch_fields(
) {
    let trace = super::build_pre_dispatch_terminal_completion_trace(
        crate::server::request_context::PreDispatchCompletionTerminalTraceInput {
            request_id: "req-pre-dispatch-cancel".to_string(),
            uri: "file:///pre-dispatch-cancel.bsl".to_string(),
            trigger_mode: "trigger_character".to_string(),
            client_probe_id: Some("probe-pre-dispatch-cancel".to_string()),
            adapter_read_at_ms: Some(1_700_000_000_100),
            resolved_at_ms: 1_700_000_000_148,
            outcome: "cancelled".to_string(),
        },
        "completion-trace-test".to_string(),
    );

    assert_eq!(trace.trace_id, "completion-trace-test");
    assert_eq!(trace.request_id.as_deref(), Some("req-pre-dispatch-cancel"));
    assert_eq!(
        trace.client_probe_id.as_deref(),
        Some("probe-pre-dispatch-cancel")
    );
    assert_eq!(trace.uri, "file:///pre-dispatch-cancel.bsl");
    assert_eq!(trace.trigger_mode, "trigger_character");
    assert_eq!(trace.outcome, "cancelled");
    assert_eq!(trace.started_at_ms, 1_700_000_000_100);
    assert_eq!(trace.total_duration_ms, 48);
    assert_eq!(
        trace.dominant_stage.as_deref(),
        Some("queued_before_dispatch")
    );
    assert!(
        trace.server_edge_details.is_none(),
        "pre-dispatch cancelled trace must not fabricate post-dispatch server_edge_details, trace={trace:?}"
    );
    assert!(trace.turn_attribution.is_none());
    assert!(trace.prepare_details.is_none());
    assert_eq!(trace.stages.len(), 2);
    assert_eq!(trace.stages[0].name, "queued_before_dispatch");
    assert_eq!(trace.stages[0].status, "cancelled");
    assert_eq!(trace.stages[0].started_offset_ms, 0);
    assert_eq!(trace.stages[0].duration_ms, 48);
    assert_eq!(trace.stages[1].name, "terminal");
    assert_eq!(trace.stages[1].status, "cancelled");
    assert_eq!(trace.stages[1].started_offset_ms, 48);
    assert_eq!(trace.stages[1].duration_ms, 0);
}

#[test]
fn pre_dispatch_queue_rejected_completion_trace_is_fail_closed_and_has_no_fabricated_post_dispatch_fields(
) {
    let trace = super::build_pre_dispatch_terminal_completion_trace(
        crate::server::request_context::PreDispatchCompletionTerminalTraceInput {
            request_id: "req-pre-dispatch-rejected".to_string(),
            uri: "file:///pre-dispatch-rejected.bsl".to_string(),
            trigger_mode: "invoked".to_string(),
            client_probe_id: Some("probe-pre-dispatch-rejected".to_string()),
            adapter_read_at_ms: Some(1_700_000_000_200),
            resolved_at_ms: 1_700_000_000_239,
            outcome: "queue_rejected".to_string(),
        },
        "completion-trace-test-rejected".to_string(),
    );

    assert_eq!(trace.trace_id, "completion-trace-test-rejected");
    assert_eq!(
        trace.request_id.as_deref(),
        Some("req-pre-dispatch-rejected")
    );
    assert_eq!(
        trace.client_probe_id.as_deref(),
        Some("probe-pre-dispatch-rejected")
    );
    assert_eq!(trace.uri, "file:///pre-dispatch-rejected.bsl");
    assert_eq!(trace.trigger_mode, "invoked");
    assert_eq!(trace.outcome, "queue_rejected");
    assert_eq!(trace.started_at_ms, 1_700_000_000_200);
    assert_eq!(trace.total_duration_ms, 39);
    assert_eq!(
        trace.dominant_stage.as_deref(),
        Some("queued_before_dispatch")
    );
    assert!(
        trace.server_edge_details.is_none(),
        "pre-dispatch queue-rejected trace must not fabricate post-dispatch server_edge_details, trace={trace:?}"
    );
    assert!(trace.turn_attribution.is_none());
    assert!(trace.prepare_details.is_none());
    assert_eq!(trace.stages.len(), 2);
    assert_eq!(trace.stages[0].name, "queued_before_dispatch");
    assert_eq!(trace.stages[0].status, "failed");
    assert_eq!(trace.stages[0].started_offset_ms, 0);
    assert_eq!(trace.stages[0].duration_ms, 39);
    assert_eq!(trace.stages[1].name, "terminal");
    assert_eq!(trace.stages[1].status, "failed");
    assert_eq!(trace.stages[1].started_offset_ms, 39);
    assert_eq!(trace.stages[1].duration_ms, 0);
}

#[tokio::test]
async fn p22_get_completion_timeline_contains_completion_trace() {
    const FIXTURE: &str =
        "Процедура Тест()\n    ДляCompletion = (Новый Массив()).\nКонецПроцедуры\n";

    let coordinator = Arc::new(SystemCoordinator::new());

    let (service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let mut service = crate::server::request_context::DispatchContextService::new(
        crate::server::request_context::RequestContextService::new(service),
    );

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let initialize_response = service
        .ready()
        .await
        .unwrap()
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
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri = Url::parse("file:///completion_timeline_fixture.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let completion_req = Request::build("textDocument/completion")
        .id(2)
        .params(serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "position": { "line": 2, "character": 13 },
            "context": {
                "triggerKind": CompletionTriggerKind::TRIGGER_CHARACTER,
                "triggerCharacter": "."
            },
            "bslProbeId": "probe-p22"
        }))
        .finish();
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(completion_req)
        .await
        .expect("completion request");
    assert!(
        completion_response.is_some(),
        "completion should return a response"
    );

    let execute = Request::build("workspace/executeCommand")
        .id(3)
        .params(serde_json::json!({
            "command": "bsl.getCompletionTimeline",
            "arguments": [{ "limit": 10 }],
        }))
        .finish();
    let execute_response = service
        .ready()
        .await
        .unwrap()
        .call(execute)
        .await
        .expect("workspace/executeCommand request")
        .expect("workspace/executeCommand response");
    let value = serde_json::to_value(&execute_response).expect("serialize response");
    let result = value.get("result").cloned().expect("result field");
    let traces = result
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("traces array");
    assert!(
        !traces.is_empty(),
        "expected non-empty completion timeline traces after completion request"
    );
    let trace = traces
        .last()
        .and_then(|value| value.as_object())
        .expect("trace");
    assert_eq!(
        trace
            .get("client_probe_id")
            .and_then(|value| value.as_str()),
        Some("probe-p22")
    );
    for field in [
        "trace_id",
        "request_id",
        "uri",
        "trigger_mode",
        "outcome",
        "started_at_ms",
        "total_duration_ms",
        "dominant_stage",
        "prepare_details",
        "server_edge_details",
        "turn_attribution",
        "stages",
    ] {
        assert!(
            trace.contains_key(field),
            "missing field `{field}` in trace"
        );
    }
    let prepare_details = trace
        .get("prepare_details")
        .and_then(|value| value.as_object())
        .expect("prepare_details object");
    let server_edge_details = trace
        .get("server_edge_details")
        .and_then(|value| value.as_object())
        .expect("server_edge_details object");
    assert!(
        prepare_details.contains_key("wait_budget_ms"),
        "missing field `wait_budget_ms` in prepare_details"
    );
    assert!(
        prepare_details.contains_key("route"),
        "prepare_details must expose bounded route field even when absent"
    );
    assert!(
        prepare_details.contains_key("fail_closed_cause"),
        "prepare_details must expose split fail-closed cause field even when absent"
    );
    assert!(
        prepare_details.contains_key("guard_outcome"),
        "missing field `guard_outcome` in prepare_details"
    );
    assert!(
        prepare_details.contains_key("min_file_version"),
        "missing field `min_file_version` in prepare_details"
    );
    assert!(
        prepare_details.contains_key("progress"),
        "prepare_details must expose prepare progress details in v5 contract"
    );
    assert!(
        prepare_details.contains_key("wait_for_file_version_runtime"),
        "prepare_details must expose wait_for_file_version runtime drilldown in v5 contract"
    );
    assert!(
        prepare_details.contains_key("snapshot_with_deps_runtime"),
        "prepare_details must expose snapshot_with_deps runtime drilldown in v5 contract"
    );
    assert!(
        prepare_details.contains_key("exact_wait"),
        "prepare_details must expose exact wait details in v5 contract"
    );
    assert!(
        prepare_details.contains_key("timeout_attribution"),
        "prepare_details must expose timeout attribution details in v6 contract"
    );
    for field in [
        "adapter_read_at_ms",
        "adapter_to_dispatch_wait_ms",
        "transport_received_at_ms",
        "transport_received_at_ms_provenance",
        "pre_method_attribution_provenance",
        "handler_entered_at_ms",
        "response_sent_at_ms",
        "transport_to_method_wait_ms",
        "method_prelude_exec_ms",
        "transport_to_handler_wait_ms",
        "server_handler_exec_ms",
    ] {
        assert!(
            server_edge_details.contains_key(field),
            "missing field `{field}` in server_edge_details"
        );
    }
    let transport_received_at_ms = server_edge_details
        .get("transport_received_at_ms")
        .and_then(|value| value.as_u64())
        .expect("transport_received_at_ms");
    let adapter_read_at_ms = server_edge_details
        .get("adapter_read_at_ms")
        .and_then(|value| value.as_u64())
        .expect("adapter_read_at_ms");
    let adapter_to_dispatch_wait_ms = server_edge_details
        .get("adapter_to_dispatch_wait_ms")
        .and_then(|value| value.as_u64())
        .expect("adapter_to_dispatch_wait_ms");
    let transport_received_at_ms_provenance = server_edge_details
        .get("transport_received_at_ms_provenance")
        .and_then(|value| value.as_str())
        .expect("transport_received_at_ms_provenance");
    let pre_method_attribution_provenance = server_edge_details
        .get("pre_method_attribution_provenance")
        .and_then(|value| value.as_str())
        .expect("pre_method_attribution_provenance");
    let handler_entered_at_ms = server_edge_details
        .get("handler_entered_at_ms")
        .and_then(|value| value.as_u64())
        .expect("handler_entered_at_ms");
    let method_entered_at_ms = server_edge_details
        .get("method_entered_at_ms")
        .and_then(|value| value.as_u64());
    let response_sent_at_ms = server_edge_details
        .get("response_sent_at_ms")
        .and_then(|value| value.as_u64())
        .expect("response_sent_at_ms");
    let transport_slot_released_at_ms = server_edge_details
        .get("transport_slot_released_at_ms")
        .and_then(|value| value.as_u64());
    let transport_to_method_wait_ms = server_edge_details
        .get("transport_to_method_wait_ms")
        .and_then(|value| value.as_u64())
        .expect("transport_to_method_wait_ms");
    let method_prelude_exec_ms = server_edge_details
        .get("method_prelude_exec_ms")
        .and_then(|value| value.as_u64())
        .expect("method_prelude_exec_ms");
    let transport_to_handler_wait_ms = server_edge_details
        .get("transport_to_handler_wait_ms")
        .and_then(|value| value.as_u64())
        .expect("transport_to_handler_wait_ms");
    let server_handler_exec_ms = server_edge_details
        .get("server_handler_exec_ms")
        .and_then(|value| value.as_u64())
        .expect("server_handler_exec_ms");
    assert!(
        matches!(
            transport_received_at_ms_provenance,
            "request_context_call_entry" | "jsonrpc_dispatch_received"
        ),
        "unexpected transport_received_at_ms_provenance={transport_received_at_ms_provenance}"
    );
    assert!(
        matches!(
            pre_method_attribution_provenance,
            "same_request_authoritative" | "best_effort_fallback" | "unavailable"
        ),
        "unexpected pre_method_attribution_provenance={pre_method_attribution_provenance}"
    );
    assert!(
        adapter_read_at_ms <= transport_received_at_ms,
        "adapter_read_at_ms must not exceed transport_received_at_ms"
    );
    assert!(
        transport_received_at_ms <= handler_entered_at_ms,
        "transport_received_at_ms must not exceed handler_entered_at_ms"
    );
    if let Some(method_entered_at_ms) = method_entered_at_ms {
        assert!(
            transport_received_at_ms <= method_entered_at_ms,
            "transport_received_at_ms must not exceed method_entered_at_ms"
        );
        assert!(
            method_entered_at_ms <= handler_entered_at_ms,
            "method_entered_at_ms must not exceed handler_entered_at_ms"
        );
        assert_eq!(
            transport_to_method_wait_ms,
            method_entered_at_ms.saturating_sub(transport_received_at_ms),
            "transport_to_method_wait_ms must match timestamp delta"
        );
        if let Some(jsonrpc_dispatch_received_at_ms) = server_edge_details
            .get("jsonrpc_dispatch_received_at_ms")
            .and_then(|value| value.as_u64())
        {
            let dispatch_to_request_context_wait_ms = server_edge_details
                .get("dispatch_to_request_context_wait_ms")
                .and_then(|value| value.as_u64())
                .expect("dispatch_to_request_context_wait_ms");
            assert_eq!(
                transport_received_at_ms_provenance, "jsonrpc_dispatch_received",
                "jsonrpc dispatch timestamp must align with provenance"
            );
            assert_eq!(
                transport_received_at_ms, jsonrpc_dispatch_received_at_ms,
                "transport_received_at_ms must equal jsonrpc dispatch timestamp when provenance is jsonrpc_dispatch_received"
            );
            assert!(
                transport_received_at_ms <= method_entered_at_ms,
                "jsonrpc dispatch timestamp must not exceed method_entered_at_ms"
            );
            assert_eq!(
                adapter_to_dispatch_wait_ms,
                jsonrpc_dispatch_received_at_ms.saturating_sub(adapter_read_at_ms),
                "adapter_to_dispatch_wait_ms must match adapter-read to dispatch delta"
            );
            assert!(
                dispatch_to_request_context_wait_ms <= transport_to_method_wait_ms,
                "dispatch_to_request_context_wait_ms must not exceed transport_to_method_wait_ms"
            );
        } else {
            assert_eq!(
                transport_received_at_ms_provenance, "request_context_call_entry",
                "missing jsonrpc dispatch timestamp must fall back to request_context_call_entry provenance"
            );
            assert!(
                !server_edge_details.contains_key("dispatch_to_request_context_wait_ms"),
                "dispatch_to_request_context_wait_ms must not be fabricated when jsonrpc_dispatch_received_at_ms is absent"
            );
        }
        assert_eq!(
            method_prelude_exec_ms,
            handler_entered_at_ms.saturating_sub(method_entered_at_ms),
            "method_prelude_exec_ms must match timestamp delta"
        );
        if let Some(service_scope_entered_at_ms) = server_edge_details
            .get("service_scope_entered_at_ms")
            .and_then(|value| value.as_u64())
        {
            let service_future_created_at_ms = server_edge_details
                .get("service_future_created_at_ms")
                .and_then(|value| value.as_u64())
                .expect("service_future_created_at_ms");
            let transport_to_service_future_wait_ms = server_edge_details
                .get("transport_to_service_future_wait_ms")
                .and_then(|value| value.as_u64())
                .expect("transport_to_service_future_wait_ms");
            let service_future_to_scope_wait_ms = server_edge_details
                .get("service_future_to_scope_wait_ms")
                .and_then(|value| value.as_u64())
                .expect("service_future_to_scope_wait_ms");
            let service_future_first_poll_entered_at_ms = server_edge_details
                .get("service_future_first_poll_entered_at_ms")
                .and_then(|value| value.as_u64());
            let transport_to_service_scope_wait_ms = server_edge_details
                .get("transport_to_service_scope_wait_ms")
                .and_then(|value| value.as_u64())
                .expect("transport_to_service_scope_wait_ms");
            let service_scope_to_method_wait_ms = server_edge_details
                .get("service_scope_to_method_wait_ms")
                .and_then(|value| value.as_u64())
                .expect("service_scope_to_method_wait_ms");
            assert!(
                transport_received_at_ms <= service_scope_entered_at_ms,
                "transport_received_at_ms must not exceed service_scope_entered_at_ms"
            );
            assert!(
                transport_received_at_ms <= service_future_created_at_ms,
                "transport_received_at_ms must not exceed service_future_created_at_ms"
            );
            assert!(
                service_future_created_at_ms <= service_scope_entered_at_ms,
                "service_future_created_at_ms must not exceed service_scope_entered_at_ms"
            );
            assert!(
                service_scope_entered_at_ms <= method_entered_at_ms,
                "service_scope_entered_at_ms must not exceed method_entered_at_ms"
            );
            assert_eq!(
                transport_to_service_future_wait_ms,
                service_future_created_at_ms.saturating_sub(transport_received_at_ms),
                "transport_to_service_future_wait_ms must match timestamp delta"
            );
            assert_eq!(
                service_future_to_scope_wait_ms,
                service_scope_entered_at_ms.saturating_sub(service_future_created_at_ms),
                "service_future_to_scope_wait_ms must match timestamp delta"
            );
            if let Some(service_future_first_poll_entered_at_ms) =
                service_future_first_poll_entered_at_ms
            {
                let service_future_to_first_poll_wait_ms = server_edge_details
                    .get("service_future_to_first_poll_wait_ms")
                    .and_then(|value| value.as_u64())
                    .expect("service_future_to_first_poll_wait_ms");
                let service_future_first_poll_outcome = server_edge_details
                    .get("service_future_first_poll_outcome")
                    .and_then(|value| value.as_str())
                    .expect("service_future_first_poll_outcome");
                let first_poll_contention_attribution = server_edge_details
                    .get("first_poll_contention_attribution")
                    .and_then(|value| value.as_object())
                    .expect("first_poll_contention_attribution");
                assert!(
                    service_scope_entered_at_ms <= service_future_first_poll_entered_at_ms,
                    "service_scope_entered_at_ms must not exceed service_future_first_poll_entered_at_ms"
                );
                assert!(
                    service_future_created_at_ms <= service_future_first_poll_entered_at_ms,
                    "service_future_created_at_ms must not exceed service_future_first_poll_entered_at_ms"
                );
                assert!(
                    matches!(service_future_first_poll_outcome, "ready" | "pending"),
                    "unexpected service_future_first_poll_outcome={service_future_first_poll_outcome}"
                );
                assert_eq!(
                    service_future_to_first_poll_wait_ms,
                    service_future_first_poll_entered_at_ms
                        .saturating_sub(service_future_created_at_ms),
                    "service_future_to_first_poll_wait_ms must match timestamp delta"
                );
                let contender_class = first_poll_contention_attribution
                    .get("contender_class")
                    .and_then(|value| value.as_str())
                    .expect("first_poll_contention_attribution.contender_class");
                assert!(
                    matches!(
                        contender_class,
                        "document_sync"
                            | "completion"
                            | "other_request"
                            | "other_notification"
                            | "mixed"
                            | "none_visible"
                            | "unavailable"
                    ),
                    "unexpected first_poll_contention_attribution.contender_class={contender_class}"
                );
                let uri_scope = first_poll_contention_attribution
                    .get("uri_scope")
                    .and_then(|value| value.as_str())
                    .expect("first_poll_contention_attribution.uri_scope");
                assert!(
                    matches!(
                        uri_scope,
                        "same_uri" | "other_uri" | "mixed" | "unavailable"
                    ),
                    "unexpected first_poll_contention_attribution.uri_scope={uri_scope}"
                );
                let inflight_count = first_poll_contention_attribution
                    .get("inflight_count")
                    .and_then(|value| value.as_u64())
                    .expect("first_poll_contention_attribution.inflight_count");
                let concurrency_level = first_poll_contention_attribution
                    .get("concurrency_level")
                    .and_then(|value| value.as_u64())
                    .expect("first_poll_contention_attribution.concurrency_level");
                assert!(
                    concurrency_level > 0,
                    "first_poll_contention_attribution.concurrency_level must stay positive"
                );
                if let Some(oldest_inflight_age_ms) = first_poll_contention_attribution
                    .get("oldest_inflight_age_ms")
                    .and_then(|value| value.as_u64())
                {
                    assert!(
                        inflight_count > 0,
                        "oldest_inflight_age_ms must not be emitted when inflight_count=0"
                    );
                    assert!(
                        oldest_inflight_age_ms <= service_future_to_first_poll_wait_ms,
                        "oldest_inflight_age_ms must stay bounded by the same trace first-poll gap"
                    );
                } else {
                    assert!(
                        inflight_count == 0 || contender_class == "unavailable",
                        "oldest_inflight_age_ms may be absent only for inflight_count=0 or unavailable snapshots"
                    );
                }
                if let Some(service_future_first_wake_scheduled_at_ms) = server_edge_details
                    .get("service_future_first_wake_scheduled_at_ms")
                    .and_then(|value| value.as_u64())
                {
                    let first_poll_to_first_wake_wait_ms = server_edge_details
                        .get("first_poll_to_first_wake_wait_ms")
                        .and_then(|value| value.as_u64())
                        .expect("first_poll_to_first_wake_wait_ms");
                    assert_eq!(
                        service_future_first_poll_outcome, "pending",
                        "first wake split must only exist for pending-first-poll traces"
                    );
                    assert!(
                        service_future_first_poll_entered_at_ms
                            <= service_future_first_wake_scheduled_at_ms,
                        "service_future_first_poll_entered_at_ms must not exceed service_future_first_wake_scheduled_at_ms"
                    );
                    assert_eq!(
                        first_poll_to_first_wake_wait_ms,
                        service_future_first_wake_scheduled_at_ms
                            .saturating_sub(service_future_first_poll_entered_at_ms),
                        "first_poll_to_first_wake_wait_ms must match timestamp delta"
                    );
                } else {
                    assert!(
                        !server_edge_details.contains_key("first_poll_to_first_wake_wait_ms"),
                        "first_poll_to_first_wake_wait_ms must not be fabricated when service_future_first_wake_scheduled_at_ms is absent"
                    );
                }
            } else {
                assert!(
                    !server_edge_details.contains_key("service_future_to_first_poll_wait_ms"),
                    "service_future_to_first_poll_wait_ms must not be fabricated when service_future_first_poll_entered_at_ms is absent"
                );
                assert!(
                    !server_edge_details.contains_key("service_future_first_poll_outcome"),
                    "service_future_first_poll_outcome must not be fabricated when service_future_first_poll_entered_at_ms is absent"
                );
                assert!(
                    !server_edge_details.contains_key("service_future_first_wake_scheduled_at_ms"),
                    "service_future_first_wake_scheduled_at_ms must not be fabricated when service_future_first_poll_entered_at_ms is absent"
                );
                assert!(
                    !server_edge_details.contains_key("first_poll_to_first_wake_wait_ms"),
                    "first_poll_to_first_wake_wait_ms must not be fabricated when service_future_first_poll_entered_at_ms is absent"
                );
                assert!(
                    !server_edge_details.contains_key("first_poll_contention_attribution"),
                    "first_poll_contention_attribution must not be fabricated when service_future_first_poll_entered_at_ms is absent"
                );
            }
            assert_eq!(
                transport_to_service_scope_wait_ms,
                service_scope_entered_at_ms.saturating_sub(transport_received_at_ms),
                "transport_to_service_scope_wait_ms must match timestamp delta"
            );
            assert_eq!(
                service_scope_to_method_wait_ms,
                method_entered_at_ms.saturating_sub(service_scope_entered_at_ms),
                "service_scope_to_method_wait_ms must match timestamp delta"
            );
        } else {
            assert!(
                !server_edge_details.contains_key("service_future_created_at_ms"),
                "service_future_created_at_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("transport_to_service_future_wait_ms"),
                "transport_to_service_future_wait_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("service_future_to_scope_wait_ms"),
                "service_future_to_scope_wait_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("transport_to_service_scope_wait_ms"),
                "transport_to_service_scope_wait_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("service_scope_to_method_wait_ms"),
                "service_scope_to_method_wait_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("service_future_first_poll_entered_at_ms"),
                "service_future_first_poll_entered_at_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("service_future_to_first_poll_wait_ms"),
                "service_future_to_first_poll_wait_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("service_future_first_poll_outcome"),
                "service_future_first_poll_outcome must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("service_future_first_wake_scheduled_at_ms"),
                "service_future_first_wake_scheduled_at_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("first_poll_to_first_wake_wait_ms"),
                "first_poll_to_first_wake_wait_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("first_poll_contention_attribution"),
                "first_poll_contention_attribution must not be fabricated when service_scope_entered_at_ms is absent"
            );
        }
    } else {
        assert!(
            !server_edge_details.contains_key("service_future_created_at_ms"),
            "service_future_created_at_ms must not be present when method_entered_at_ms is absent"
        );
        assert!(
            !server_edge_details.contains_key("transport_to_service_future_wait_ms"),
            "transport_to_service_future_wait_ms must not be present when method_entered_at_ms is absent"
        );
        assert!(
            !server_edge_details.contains_key("service_future_to_scope_wait_ms"),
            "service_future_to_scope_wait_ms must not be present when method_entered_at_ms is absent"
        );
        assert!(
            !server_edge_details.contains_key("service_scope_entered_at_ms"),
            "service_scope_entered_at_ms must not be present when method_entered_at_ms is absent"
        );
        assert!(
            !server_edge_details.contains_key("transport_to_service_scope_wait_ms"),
            "transport_to_service_scope_wait_ms must not be present when method_entered_at_ms is absent"
        );
        assert!(
            !server_edge_details.contains_key("service_scope_to_method_wait_ms"),
            "service_scope_to_method_wait_ms must not be present when method_entered_at_ms is absent"
        );
    }
    assert!(
        handler_entered_at_ms <= response_sent_at_ms,
        "handler_entered_at_ms must not exceed response_sent_at_ms"
    );
    assert_eq!(
        transport_to_handler_wait_ms,
        handler_entered_at_ms.saturating_sub(transport_received_at_ms),
        "transport_to_handler_wait_ms must match timestamp delta"
    );
    if let Some(transport_slot_released_at_ms) = transport_slot_released_at_ms {
        let transport_to_slot_release_wait_ms = server_edge_details
            .get("transport_to_slot_release_wait_ms")
            .and_then(|value| value.as_u64())
            .expect("transport_to_slot_release_wait_ms");
        let slot_release_to_handler_wait_ms = server_edge_details
            .get("slot_release_to_handler_wait_ms")
            .and_then(|value| value.as_u64())
            .expect("slot_release_to_handler_wait_ms");
        let slot_release_to_response_wait_ms = server_edge_details
            .get("slot_release_to_response_wait_ms")
            .and_then(|value| value.as_u64())
            .expect("slot_release_to_response_wait_ms");
        assert!(
            transport_received_at_ms <= transport_slot_released_at_ms,
            "transport_slot_released_at_ms must not precede transport_received_at_ms"
        );
        assert!(
            transport_slot_released_at_ms <= handler_entered_at_ms,
            "transport_slot_released_at_ms must not exceed handler_entered_at_ms"
        );
        assert_eq!(
            transport_to_slot_release_wait_ms,
            transport_slot_released_at_ms.saturating_sub(transport_received_at_ms),
            "transport_to_slot_release_wait_ms must match timestamp delta"
        );
        assert_eq!(
            slot_release_to_handler_wait_ms,
            handler_entered_at_ms.saturating_sub(transport_slot_released_at_ms),
            "slot_release_to_handler_wait_ms must match timestamp delta"
        );
        assert_eq!(
            slot_release_to_response_wait_ms,
            response_sent_at_ms.saturating_sub(transport_slot_released_at_ms),
            "slot_release_to_response_wait_ms must match timestamp delta"
        );
    } else {
        assert!(
            !server_edge_details.contains_key("transport_to_slot_release_wait_ms"),
            "transport_to_slot_release_wait_ms must not be fabricated when transport_slot_released_at_ms is absent"
        );
        assert!(
            !server_edge_details.contains_key("slot_release_to_handler_wait_ms"),
            "slot_release_to_handler_wait_ms must not be fabricated when transport_slot_released_at_ms is absent"
        );
        assert!(
            !server_edge_details.contains_key("slot_release_to_response_wait_ms"),
            "slot_release_to_response_wait_ms must not be fabricated when transport_slot_released_at_ms is absent"
        );
    }
    assert_eq!(
        server_handler_exec_ms,
        response_sent_at_ms.saturating_sub(handler_entered_at_ms),
        "server_handler_exec_ms must match timestamp delta"
    );
    let stages = trace
        .get("stages")
        .and_then(|value| value.as_array())
        .expect("trace stages array");
    assert!(!stages.is_empty(), "trace stages must not be empty");
    for stage in stages {
        let stage = stage.as_object().expect("stage object");
        for field in ["name", "status", "started_offset_ms", "duration_ms"] {
            assert!(
                stage.contains_key(field),
                "missing field `{field}` in stage"
            );
        }
    }
    let turn_attribution = trace
        .get("turn_attribution")
        .and_then(|value| value.as_object())
        .expect("turn_attribution object");
    for field in [
        "request_file_seq",
        "request_epoch",
        "queue_outcome",
        "queue_capacity",
        "queue_depth_before_enqueue",
        "queue_depth_after_enqueue",
        "queued_completion_ahead_count",
        "did_change_ahead_count",
        "active_completion_count",
        "dropped_completion_file_seq",
    ] {
        assert!(
            turn_attribution.contains_key(field),
            "missing field `{field}` in turn_attribution"
        );
    }
    assert!(
        turn_attribution.contains_key("dispatcher_resolution_latency_ms"),
        "missing field `dispatcher_resolution_latency_ms` in turn_attribution"
    );
    if let (Some(transport_slot_released_at_ms), Some(turn_wait_entered_at_ms)) = (
        transport_slot_released_at_ms,
        turn_attribution
            .get("turn_wait_entered_at_ms")
            .and_then(|value| value.as_u64()),
    ) {
        assert!(
            transport_slot_released_at_ms <= turn_wait_entered_at_ms,
            "transport_slot_released_at_ms must not exceed turn_wait_entered_at_ms"
        );
    }

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_runtime_knobs_config() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let execute = Request::build("workspace/executeCommand")
        .id(2)
        .params(serde_json::json!({
            "command": "bsl.getObservabilityMetrics",
            "arguments": [{}],
        }))
        .finish();
    let execute_response = service
        .ready()
        .await
        .unwrap()
        .call(execute)
        .await
        .expect("workspace/executeCommand request")
        .expect("workspace/executeCommand response");

    let value = serde_json::to_value(&execute_response).expect("serialize response");
    let result = value.get("result").cloned().expect("result field");
    let metrics = result
        .get("metrics")
        .and_then(|value| value.as_object())
        .expect("metrics object");
    let config = metrics
        .get("config")
        .and_then(|value| value.as_object())
        .expect("config object");
    let interactive_wait_budget = config
        .get("BSL_INTELLISENSE_V2_INTERACTIVE_WAIT_BUDGET_MS")
        .and_then(|value| value.as_object())
        .expect("interactive wait budget config entry");
    assert_eq!(
        interactive_wait_budget
            .get("effective")
            .and_then(|value| value.as_u64()),
        Some(120),
        "metrics config must expose effective interactive wait budget"
    );
    assert!(
        interactive_wait_budget
            .get("source")
            .and_then(|value| value.as_str())
            .is_some(),
        "metrics config must expose value source"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p23_cross_interface_semantic_parity_lsp_web_mcp_diagnostics() {
    const PARITY_FIXTURE: &str = "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.НесуществующийМетод();\nКонецПроцедуры\n";

    let lsp_coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) = LspService::build({
        let coordinator = lsp_coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let (published_tx, mut published_rx) =
        tokio::sync::mpsc::unbounded_channel::<PublishDiagnosticsParams>();
    let drain_task = tokio::spawn(async move {
        while let Some(req) = socket.next().await {
            if req.method() != "textDocument/publishDiagnostics" {
                continue;
            }
            let Some(params) = req.params().cloned() else {
                continue;
            };
            let Ok(parsed) =
                serde_json::from_value::<tower_lsp::lsp_types::PublishDiagnosticsParams>(params)
            else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;
    let lsp_uri = Url::parse("file:///parity_fixture.bsl").expect("lsp uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: lsp_uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: PARITY_FIXTURE.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");
    let lsp_diagnostics = wait_lsp_publish_diagnostics(&mut published_rx, &lsp_uri).await;
    let lsp_normalized = normalize_lsp_semantic_diagnostics(&lsp_diagnostics);
    assert!(
        !lsp_normalized.is_empty(),
        "expected non-empty LSP diagnostics"
    );
    drain_task.abort();

    let app = create_router(build_web_test_state(), "backend/static", true);
    let web_response = app
        .oneshot(
            AxumRequest::post("/api/diagnostics")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::json!({ "code": PARITY_FIXTURE }).to_string(),
                ))
                .expect("web diagnostics request"),
        )
        .await
        .expect("web diagnostics response");
    assert!(
        web_response.status().is_success(),
        "unexpected web status: {}",
        web_response.status()
    );
    let web_body = axum::body::to_bytes(web_response.into_body(), usize::MAX)
        .await
        .expect("web body");
    let web_payload: serde_json::Value =
        serde_json::from_slice(&web_body).expect("web diagnostics payload");
    let web_normalized = normalize_web_semantic_diagnostics(&web_payload);
    assert!(
        !web_normalized.is_empty(),
        "expected non-empty Web diagnostics, payload={web_payload}"
    );

    let temp = tempfile::TempDir::new().expect("tempdir");
    let module_path = temp.path().join("Module.bsl");
    std::fs::write(&module_path, PARITY_FIXTURE).expect("write module");
    let mcp_manager = Arc::new(SessionManager::new());
    let mcp_job_manager = Arc::new(JobManager::new());
    let open = mcp_manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            mcp_job_manager.clone(),
        )
        .await
        .expect("mcp workspace open");
    wait_mcp_startup(mcp_job_manager.as_ref(), open.startup_job_id.as_deref()).await;
    let mcp_diagnostics = mcp_manager
        .bsl_diagnostics(BslDiagnosticsParams {
            session_id: open.session_id,
            scope: WorkspaceScope::Tagged(WorkspaceScopeTagged::Project),
            limit: 200,
            include_impact: false,
            include_coverage: false,
            include_flow_sensitive: false,
        })
        .await
        .expect("mcp diagnostics");
    let mcp_normalized = normalize_mcp_semantic_diagnostics(&mcp_diagnostics.diagnostics);
    assert!(
        !mcp_normalized.is_empty(),
        "expected non-empty MCP diagnostics"
    );

    assert_eq!(
        lsp_normalized, web_normalized,
        "LSP/Web semantic diagnostics drift detected"
    );
    assert_eq!(
        lsp_normalized, mcp_normalized,
        "LSP/MCP semantic diagnostics drift detected"
    );
}

#[tokio::test]
async fn p24_real_scenario_observability_stage_parity_lsp_vs_mcp() {
    const OBSERVABILITY_FIXTURE: &str =
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";

    let lsp_coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) = LspService::build({
        let coordinator = lsp_coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let (published_tx, mut published_rx) =
        tokio::sync::mpsc::unbounded_channel::<PublishDiagnosticsParams>();
    let drain_task = tokio::spawn(async move {
        while let Some(req) = socket.next().await {
            if req.method() != "textDocument/publishDiagnostics" {
                continue;
            }
            let Some(params) = req.params().cloned() else {
                continue;
            };
            let Ok(parsed) =
                serde_json::from_value::<tower_lsp::lsp_types::PublishDiagnosticsParams>(params)
            else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///observability_fixture.bsl").expect("lsp uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: OBSERVABILITY_FIXTURE.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");
    let _ = wait_lsp_publish_diagnostics(&mut published_rx, &uri).await;

    let completion = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position::new(2, 13),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None,
    };
    let completion_req = Request::build("textDocument/completion")
        .id(2)
        .params(serde_json::to_value(completion).expect("CompletionParams"))
        .finish();
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(completion_req)
        .await
        .expect("completion request");
    assert!(
        completion_response.is_some(),
        "completion should return a response"
    );

    let execute = Request::build("workspace/executeCommand")
        .id(3)
        .params(serde_json::json!({
            "command": "bsl.getObservabilityMetrics",
            "arguments": [],
        }))
        .finish();
    let execute_response = service
        .ready()
        .await
        .unwrap()
        .call(execute)
        .await
        .expect("workspace/executeCommand request")
        .expect("workspace/executeCommand response");
    let lsp_metrics_payload =
        serde_json::to_value(&execute_response).expect("serialize execute response");
    let lsp_metrics_payload = lsp_metrics_payload
        .get("result")
        .cloned()
        .expect("execute result field");
    drain_task.abort();

    let temp = tempfile::TempDir::new().expect("tempdir");
    let module_path = temp.path().join("Module.bsl");
    std::fs::write(&module_path, OBSERVABILITY_FIXTURE).expect("write module");
    let mcp_manager = Arc::new(SessionManager::new());
    let mcp_job_manager = Arc::new(JobManager::new());
    let open = mcp_manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            mcp_job_manager.clone(),
        )
        .await
        .expect("mcp workspace open");
    wait_mcp_startup(mcp_job_manager.as_ref(), open.startup_job_id.as_deref()).await;

    let _diagnostics = mcp_manager
        .bsl_diagnostics(BslDiagnosticsParams {
            session_id: open.session_id.clone(),
            scope: WorkspaceScope::Tagged(WorkspaceScopeTagged::Project),
            limit: 200,
            include_impact: false,
            include_coverage: false,
            include_flow_sensitive: false,
        })
        .await
        .expect("mcp diagnostics");
    let _members = mcp_manager
        .bsl_members(BslMembersParams {
            session_id: open.session_id.clone(),
            file: McpFileRef {
                doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            },
            position: McpPosition {
                line: 2,
                character: 13,
            },
            limit: 50,
            include_flow_sensitive: false,
        })
        .await
        .expect("mcp members");
    let mcp_metrics_payload = mcp_manager
        .observability_metrics_get(&open.session_id)
        .await
        .expect("mcp observability")
        .metrics;

    let lsp_stages = collect_observed_stages(&lsp_metrics_payload);
    let mcp_stages = collect_observed_stages(&mcp_metrics_payload);
    let required_shared_stages = [
        "runtime_snapshot_with_deps",
        "semantic_diagnostics_query",
        "ir_query",
    ];

    for stage in required_shared_stages {
        assert!(
            lsp_stages.contains(stage),
            "LSP metrics missing required stage {stage}, stages={lsp_stages:?}"
        );
        assert!(
            mcp_stages.contains(stage),
            "MCP metrics missing required stage {stage}, stages={mcp_stages:?}"
        );
        assert!(
            has_positive_counter_for_stage(&lsp_metrics_payload, stage),
            "LSP stage {stage} has no positive counters"
        );
        assert!(
            has_positive_counter_for_stage(&mcp_metrics_payload, stage),
            "MCP stage {stage} has no positive counters"
        );
    }
    assert!(
        mcp_stages.contains("parse_result_query"),
        "MCP metrics missing parse_result_query stage, stages={mcp_stages:?}"
    );
    assert!(
        !has_positive_counter_for_stage(&mcp_metrics_payload, "parse_result_query"),
        "MCP semantic scenario must not execute parse_result_query stage"
    );

    assert_drilldown_stage_metrics_for_origin(&lsp_metrics_payload, "lsp");
    assert_drilldown_stage_metrics_for_origin(&mcp_metrics_payload, "agent");
}

#[tokio::test]
async fn p25_web_hover_endpoints_use_shared_ephemeral_exact_artifact() {
    let code = "Процедура Тест()\n\
    Arr = Новый Массив;\n\
    Arr.Добавить(1);\n\
    ДляHover = Arr;\n\
КонецПроцедуры\n";
    let position = find_utf16_position_at_marker_tail(code, "ДляHover = Arr");

    let hover_text = web_hover_text_for_code(code, position).await;
    assert!(
        hover_text.contains("Массив"),
        "web hover must expose warmed exact type for ephemeral semantic snapshot, hover={hover_text}"
    );

    let enhanced_hover_text = web_enhanced_hover_text_for_code(code, position).await;
    assert!(
        enhanced_hover_text.contains("Массив"),
        "web enhanced hover must expose warmed exact type for ephemeral semantic snapshot, hover={enhanced_hover_text}"
    );
}

#[tokio::test]
async fn p25_cross_interface_semantic_parity_lsp_web_mcp_core_tools() {
    const TOOLS_FIXTURE: &str = "Процедура Foo() Экспорт\nКонецПроцедуры\n\nПроцедура Bar()\n    Arr = Новый Массив;\n    Arr.Добавить(1);\n    Foo();\nКонецПроцедуры\n";
    const TARGET_SYMBOL: &str = "Foo";
    const TYPE_LINE: u32 = 5;
    const TYPE_CHARACTER: u32 = 5;
    const MEMBERS_LINE: u32 = 5;
    const MEMBERS_CHARACTER: u32 = 7;
    const SYMBOL_CALL_LINE: u32 = 6;
    const SYMBOL_CALL_CHARACTER: u32 = 5;

    let lsp_coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) = LspService::build({
        let coordinator = lsp_coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let lsp_uri = Url::parse("file:///Module.bsl").expect("lsp uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: lsp_uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: TOOLS_FIXTURE.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: lsp_uri.clone(),
            },
            position: Position {
                line: MEMBERS_LINE,
                character: MEMBERS_CHARACTER,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None,
    };
    let completion_req = Request::build("textDocument/completion")
        .id(2)
        .params(serde_json::to_value(completion_params).expect("CompletionParams"))
        .finish();
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(completion_req)
        .await
        .expect("completion request")
        .expect("completion response");
    let completion_value = serde_json::to_value(&completion_response).expect("serialize response");
    let completion_result = completion_value
        .get("result")
        .cloned()
        .expect("result field");
    let lsp_completion: Option<CompletionResponse> =
        serde_json::from_value(completion_result).expect("parse completion result");
    let lsp_completion = lsp_completion.expect("completion result present");
    let lsp_members = normalize_lsp_member_labels(&lsp_completion);

    let symbol_req = Request::build("workspace/symbol")
        .id(3)
        .params(
            serde_json::to_value(WorkspaceSymbolParams {
                query: TARGET_SYMBOL.to_string(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .expect("WorkspaceSymbolParams"),
        )
        .finish();
    let symbol_response = service
        .ready()
        .await
        .unwrap()
        .call(symbol_req)
        .await
        .expect("workspace/symbol request")
        .expect("workspace/symbol response");
    let symbol_value = serde_json::to_value(&symbol_response).expect("serialize response");
    let symbol_result = symbol_value.get("result").cloned().expect("result field");
    let lsp_symbols: Option<Vec<SymbolInformation>> =
        serde_json::from_value(symbol_result).expect("parse symbol result");
    let lsp_symbols = lsp_symbols.expect("symbol result present");
    let lsp_symbols = normalize_lsp_workspace_symbols(&lsp_symbols);
    assert!(
        !lsp_symbols.is_empty(),
        "expected non-empty LSP symbol_search result"
    );

    let definition_req = Request::build("textDocument/definition")
        .id(4)
        .params(
            serde_json::to_value(GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: lsp_uri.clone(),
                    },
                    position: Position {
                        line: SYMBOL_CALL_LINE,
                        character: SYMBOL_CALL_CHARACTER,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .expect("GotoDefinitionParams"),
        )
        .finish();
    let definition_response = service
        .ready()
        .await
        .unwrap()
        .call(definition_req)
        .await
        .expect("textDocument/definition request")
        .expect("textDocument/definition response");
    let definition_value = serde_json::to_value(&definition_response).expect("serialize response");
    let definition_result = definition_value
        .get("result")
        .cloned()
        .expect("result field");
    let lsp_definition: Option<GotoDefinitionResponse> =
        serde_json::from_value(definition_result).expect("parse definition result");
    let lsp_definition = normalize_lsp_definition(lsp_definition);
    assert!(
        !lsp_definition.is_empty(),
        "expected non-empty LSP definition result"
    );

    let references_req = Request::build("textDocument/references")
        .id(5)
        .params(
            serde_json::to_value(ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: lsp_uri.clone(),
                    },
                    position: Position {
                        line: SYMBOL_CALL_LINE,
                        character: SYMBOL_CALL_CHARACTER,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: ReferenceContext {
                    include_declaration: false,
                },
            })
            .expect("ReferenceParams"),
        )
        .finish();
    let references_response = service
        .ready()
        .await
        .unwrap()
        .call(references_req)
        .await
        .expect("textDocument/references request")
        .expect("textDocument/references response");
    let references_value = serde_json::to_value(&references_response).expect("serialize response");
    let references_result = references_value
        .get("result")
        .cloned()
        .expect("result field");
    let lsp_references: Option<Vec<Location>> =
        serde_json::from_value(references_result).expect("parse references result");
    let lsp_references = normalize_lsp_locations(&lsp_references.unwrap_or_default());
    assert!(
        !lsp_references.is_empty(),
        "expected non-empty LSP references result"
    );

    let hover_req = Request::build("textDocument/hover")
        .id(6)
        .params(
            serde_json::to_value(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: lsp_uri.clone(),
                    },
                    position: Position {
                        line: TYPE_LINE,
                        character: TYPE_CHARACTER,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .expect("HoverParams"),
        )
        .finish();
    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(hover_req)
        .await
        .expect("textDocument/hover request")
        .expect("textDocument/hover response");
    let hover_value = serde_json::to_value(&hover_response).expect("serialize response");
    let hover_result = hover_value.get("result").cloned().expect("result field");
    let lsp_hover: Option<Hover> = serde_json::from_value(hover_result).expect("parse hover");
    let lsp_hover_text = lsp_hover
        .and_then(extract_hover_text)
        .unwrap_or_else(|| String::from(""));
    assert!(
        !lsp_hover_text.is_empty(),
        "expected non-empty LSP hover response at type position"
    );
    drain_task.abort();

    // Web currently exposes hover/diagnostics for semantic parity, while MCP-only tools below
    // are validated via LSP/MCP pairs.
    let app = create_router(build_web_test_state(), "backend/static", true);
    let web_hover_response = app
        .clone()
        .oneshot(
            AxumRequest::post("/api/hover")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::json!({
                        "code": TOOLS_FIXTURE,
                        "line": TYPE_LINE,
                        "column": TYPE_CHARACTER
                    })
                    .to_string(),
                ))
                .expect("web hover request"),
        )
        .await
        .expect("web hover response");
    assert!(
        web_hover_response.status().is_success(),
        "unexpected web hover status: {}",
        web_hover_response.status()
    );
    let web_hover_body = axum::body::to_bytes(web_hover_response.into_body(), usize::MAX)
        .await
        .expect("web hover body");
    let web_hover_payload: serde_json::Value =
        serde_json::from_slice(&web_hover_body).expect("web hover payload");
    let web_hover_text = web_hover_payload
        .get("hover")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    assert!(
        !web_hover_text.is_empty(),
        "expected non-empty Web hover text, payload={web_hover_payload}"
    );
    let web_enhanced_hover_response = app
        .oneshot(
            AxumRequest::post("/api/hover/enhanced")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::json!({
                        "code": TOOLS_FIXTURE,
                        "line": TYPE_LINE,
                        "column": TYPE_CHARACTER
                    })
                    .to_string(),
                ))
                .expect("web enhanced hover request"),
        )
        .await
        .expect("web enhanced hover response");
    assert!(
        web_enhanced_hover_response.status().is_success(),
        "unexpected web enhanced hover status: {}",
        web_enhanced_hover_response.status()
    );
    let web_enhanced_hover_body =
        axum::body::to_bytes(web_enhanced_hover_response.into_body(), usize::MAX)
            .await
            .expect("web enhanced hover body");
    let web_enhanced_hover_payload: serde_json::Value =
        serde_json::from_slice(&web_enhanced_hover_body).expect("web enhanced hover payload");
    let web_enhanced_hover_text = web_enhanced_hover_payload
        .get("hoverText")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    assert!(
        !web_enhanced_hover_text.is_empty()
            && web_enhanced_hover_text != "No information available",
        "expected non-empty Web enhanced hover text, payload={web_enhanced_hover_payload}"
    );

    let temp = tempfile::TempDir::new().expect("tempdir");
    let module_path = temp.path().join("Module.bsl");
    std::fs::write(&module_path, TOOLS_FIXTURE).expect("write module");
    let mcp_manager = Arc::new(SessionManager::new());
    let mcp_job_manager = Arc::new(JobManager::new());
    let open = mcp_manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            mcp_job_manager.clone(),
        )
        .await
        .expect("mcp workspace open");
    wait_mcp_startup(mcp_job_manager.as_ref(), open.startup_job_id.as_deref()).await;

    let mcp_type = mcp_manager
        .bsl_type_at_position(BslTypeAtPositionParams {
            session_id: open.session_id.clone(),
            file: McpFileRef {
                doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            },
            position: McpPosition {
                line: TYPE_LINE,
                character: TYPE_CHARACTER,
            },
            include_flow_sensitive: false,
        })
        .await
        .expect("mcp type_at_position");
    assert!(
        mcp_type.warnings.is_empty(),
        "mcp type_at_position returned warnings: {:?}",
        mcp_type.warnings
    );
    let mcp_type_name = mcp_type
        .type_info
        .as_ref()
        .map(|type_info| type_info.name.clone())
        .expect("mcp type_at_position type_info");

    let mcp_members = mcp_manager
        .bsl_members(BslMembersParams {
            session_id: open.session_id.clone(),
            file: McpFileRef {
                doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            },
            position: McpPosition {
                line: MEMBERS_LINE,
                character: MEMBERS_CHARACTER,
            },
            limit: 100,
            include_flow_sensitive: false,
        })
        .await
        .expect("mcp members");
    let mcp_members = normalize_mcp_member_labels(&mcp_members.members);

    let mcp_symbol_search = mcp_manager
        .bsl_symbol_search(BslSymbolSearchParams {
            session_id: open.session_id.clone(),
            query: TARGET_SYMBOL.to_string(),
            limit: 20,
        })
        .await
        .expect("mcp symbol_search");
    let mcp_symbols = normalize_mcp_workspace_symbols(&mcp_symbol_search.symbols);
    assert!(
        !mcp_symbols.is_empty(),
        "expected non-empty MCP symbol_search result"
    );
    let mcp_target_symbol_id = mcp_symbol_search
        .symbols
        .iter()
        .find(|symbol| symbol.name == TARGET_SYMBOL)
        .map(|symbol| symbol.symbol_id.clone())
        .expect("mcp target symbol id");

    let mcp_references = mcp_manager
        .bsl_references(BslReferencesParams {
            session_id: open.session_id.clone(),
            symbol_id: mcp_target_symbol_id,
            limit: 50,
            include_snippets: false,
        })
        .await
        .expect("mcp references");
    let mcp_references = normalize_mcp_references(&mcp_references.references);
    assert!(
        !mcp_references.is_empty(),
        "expected non-empty MCP references result"
    );

    let mcp_definition = mcp_manager
        .bsl_definition(BslDefinitionParams {
            session_id: open.session_id,
            symbol_id: None,
            file: Some(McpFileRef {
                doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            }),
            position: Some(McpPosition {
                line: SYMBOL_CALL_LINE,
                character: SYMBOL_CALL_CHARACTER,
            }),
        })
        .await
        .expect("mcp definition");
    let mcp_definition = normalize_mcp_definition(mcp_definition.location.as_ref());
    assert!(
        !mcp_definition.is_empty(),
        "expected non-empty MCP definition result"
    );

    assert_eq!(lsp_members, mcp_members, "LSP/MCP members drift detected");
    assert_eq!(
        lsp_symbols, mcp_symbols,
        "LSP/MCP symbol_search drift detected"
    );
    assert_eq!(
        lsp_references, mcp_references,
        "LSP/MCP references drift detected"
    );
    assert_eq!(
        lsp_definition, mcp_definition,
        "LSP/MCP definition drift detected"
    );

    assert!(
        lsp_hover_text.contains(&mcp_type_name),
        "LSP hover/type_at_position drift detected: expected '{mcp_type_name}' in hover text, got '{lsp_hover_text}'"
    );
    assert!(
        web_hover_text.contains(&mcp_type_name),
        "Web hover/type_at_position drift detected: expected '{mcp_type_name}' in hover text, got '{web_hover_text}'"
    );
}
