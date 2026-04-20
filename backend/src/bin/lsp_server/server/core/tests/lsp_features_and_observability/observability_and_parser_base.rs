#[tokio::test]
async fn p22_get_observability_metrics_exposes_unified_stage_contract() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

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

    let execute = Request::build("workspace/executeCommand")
        .id(2)
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

    let value = serde_json::to_value(&execute_response).expect("serialize response");
    let result = value.get("result").cloned().expect("result field");
    assert_unified_intellisense_v2_stage_contract(&result);

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_syntax_diagnostics_mode_drilldown() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
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

    let uri = Url::parse("file:///syntax_mode_metrics_fixture.bsl").expect("fixture uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Процедура Тест(\nКонецПроцедуры\n".to_string(),
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
    let diagnostics = wait_lsp_publish_diagnostics(&mut published_rx, &uri).await;
    assert!(
        !diagnostics.is_empty(),
        "syntax fixture must publish diagnostics before metrics snapshot"
    );

    let execute = Request::build("workspace/executeCommand")
        .id(2)
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
    let value = serde_json::to_value(&execute_response).expect("serialize response");
    let result = value.get("result").cloned().expect("result field");
    let metrics = result.get("metrics").expect("metrics field");
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    let drilldown_counter = counters
        .get(
            "intellisense_v2_drilldown_stage_total_origin_lsp_mode_full_operation_diagnostics_stage_syntax_diagnostics_query",
        )
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let aggregate_counter = counters
        .get("intellisense_v2_syntax_diagnostics_query_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let drilldown_hist_count = histograms
        .get(
            "intellisense_v2_drilldown_stage_latency_ms_origin_lsp_mode_full_operation_diagnostics_stage_syntax_diagnostics_query",
        )
        .and_then(|value| value.get("count"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let aggregate_hist_count = histograms
        .get("intellisense_v2_syntax_diagnostics_query_ms")
        .and_then(|value| value.get("count"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0);

    assert!(
        drilldown_counter > 0,
        "metrics endpoint must expose syntax_diagnostics stage drilldown by parse mode"
    );
    assert_eq!(
        aggregate_counter, drilldown_counter,
        "legacy aggregate total must remain in parity with syntax_diagnostics mode drilldown"
    );
    assert!(
        drilldown_hist_count > 0,
        "metrics endpoint must expose syntax_diagnostics latency histogram by parse mode"
    );
    assert_eq!(
        aggregate_hist_count, drilldown_hist_count,
        "legacy aggregate latency must remain in parity with syntax_diagnostics mode drilldown"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_sidebar_shape_filters_unrelated_histograms() {
    let coordinator = Arc::new(SystemCoordinator::new());
    coordinator.record_completion_latency(Duration::from_millis(33));
    coordinator.record_intellisense_v2_ir_query_latency("completion", Duration::from_millis(21));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let execute = Request::build("workspace/executeCommand")
        .id(2203)
        .params(serde_json::json!({
            "command": "bsl.getObservabilityMetrics",
            "arguments": [{
                "shape": "sidebar",
            }],
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
    let metrics = result.get("metrics").expect("metrics field");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    assert!(
        histograms.contains_key("intellisense_v2_ir_query_completion_ms"),
        "sidebar shape must keep key completion IR histogram"
    );
    assert!(
        !histograms.contains_key("completion_duration_ms"),
        "sidebar shape must omit unrelated full histograms"
    );
    assert!(
        result.get("didChangeParseSnapshotEvidence").is_none(),
        "sidebar shape must omit didChange parse-snapshot evidence payload"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_did_change_parse_snapshot_evidence() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let uri =
        Url::parse("file:///did_change_parse_snapshot_evidence_fixture.bsl").expect("fixture uri");
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 1,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position::new(0, 0),
                end: Position::new(0, 0),
            }),
            range_length: None,
            text: "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n".to_string(),
        }],
    };
    let did_change_req = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
        .finish();
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req)
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    let evidence = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let execute = Request::build("workspace/executeCommand")
                .id(2216)
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
            let value = serde_json::to_value(&execute_response).expect("serialize response");
            let result = value.get("result").cloned().expect("result field");
            let evidence = result
                .get("didChangeParseSnapshotEvidence")
                .and_then(|value| value.get("entries"))
                .and_then(|value| value.as_array())
                .and_then(|entries| {
                    entries.iter().find(|entry| {
                        entry.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && entry
                                .get("requestedVersion")
                                .and_then(|value| value.as_i64())
                                == Some(1)
                    })
                })
                .cloned();
            if let Some(entry) = evidence {
                break (result, entry);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("didChange parse-snapshot evidence must appear in observability metrics response");

    let (result, entry) = evidence;
    assert_eq!(
        result
            .get("didChangeParseSnapshotEvidence")
            .and_then(|value| value.get("version"))
            .and_then(|value| value.as_u64()),
        Some(crate::server::DID_CHANGE_PARSE_SNAPSHOT_EVIDENCE_VERSION as u64)
    );
    assert_eq!(
        entry.get("parseMode").and_then(|value| value.as_str()),
        Some("full")
    );
    assert_eq!(
        entry.get("baseTextSource").and_then(|value| value.as_str()),
        Some("analysis_snapshot")
    );
    assert_eq!(
        entry.get("changeShape").and_then(|value| value.as_str()),
        Some("ranged")
    );
    assert_eq!(
        entry
            .get("contentChangesCount")
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        entry.get("replayOrder").and_then(|value| value.as_str()),
        Some("receive_order")
    );
    assert_eq!(
        entry
            .get("baseDocumentVersion")
            .and_then(|value| value.as_i64()),
        None
    );
    assert_eq!(
        entry
            .get("changedRangesCount")
            .and_then(|value| value.as_u64()),
        Some(0)
    );
    assert_eq!(
        entry.get("fallbackReason").and_then(|value| value.as_str()),
        Some("no_previous_tree")
    );
    assert_eq!(
        entry
            .get("parserBaseRootCause")
            .and_then(|value| value.as_str()),
        None
    );
    assert_eq!(
        entry
            .get("shadowDocumentVersion")
            .and_then(|value| value.as_i64()),
        None
    );
    assert_eq!(
        entry
            .get("latestReadyDocumentVersion")
            .and_then(|value| value.as_i64()),
        None
    );

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_input_edit_conversion_failure_reason() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///did_change_parse_snapshot_input_edit_failure_fixture.bsl")
        .expect("fixture uri");
    let seeded_text = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n".to_string();
    let seeded_did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 1,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position::new(0, 0),
                end: Position::new(0, 0),
            }),
            range_length: None,
            text: seeded_text,
        }],
    };
    let seeded_did_change_req = Request::build("textDocument/didChange")
        .params(serde_json::to_value(seeded_did_change).expect("DidChangeTextDocumentParams"))
        .finish();
    let seeded_did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(seeded_did_change_req)
        .await
        .expect("didChange notification");
    assert!(
        seeded_did_change_response.is_none(),
        "didChange is a notification"
    );

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let execute = Request::build("workspace/executeCommand")
                .id(2217)
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
            let value = serde_json::to_value(&execute_response).expect("serialize response");
            let result = value.get("result").cloned().expect("result field");
            let has_seed_entry = result
                .get("didChangeParseSnapshotEvidence")
                .and_then(|value| value.get("entries"))
                .and_then(|value| value.as_array())
                .is_some_and(|entries| {
                    entries.iter().any(|entry| {
                        entry.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && entry
                                .get("requestedVersion")
                                .and_then(|value| value.as_i64())
                                == Some(1)
                    })
                });
            if has_seed_entry {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("seed didChange parse-snapshot evidence must appear before forcing failure");

    let failing_did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position::new(99, 0),
                end: Position::new(99, 0),
            }),
            range_length: None,
            text: "2".to_string(),
        }],
    };
    let failing_did_change_req = Request::build("textDocument/didChange")
        .params(serde_json::to_value(failing_did_change).expect("DidChangeTextDocumentParams"))
        .finish();
    let failing_did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(failing_did_change_req)
        .await
        .expect("didChange notification");
    assert!(
        failing_did_change_response.is_none(),
        "didChange is a notification"
    );

    let evidence = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let execute = Request::build("workspace/executeCommand")
                .id(2218)
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
            let value = serde_json::to_value(&execute_response).expect("serialize response");
            let result = value.get("result").cloned().expect("result field");
            let evidence = result
                .get("didChangeParseSnapshotEvidence")
                .and_then(|value| value.get("entries"))
                .and_then(|value| value.as_array())
                .and_then(|entries| {
                    entries.iter().find(|entry| {
                        entry.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && entry
                                .get("requestedVersion")
                                .and_then(|value| value.as_i64())
                                == Some(2)
                    })
                })
                .cloned();
            if let Some(entry) = evidence {
                break (result, entry);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("invalid input-edit conversion failure must appear in observability metrics response");

    let (result, entry) = evidence;
    assert_eq!(
        result
            .get("didChangeParseSnapshotEvidence")
            .and_then(|value| value.get("version"))
            .and_then(|value| value.as_u64()),
        Some(crate::server::DID_CHANGE_PARSE_SNAPSHOT_EVIDENCE_VERSION as u64)
    );
    assert_eq!(
        entry.get("parseMode").and_then(|value| value.as_str()),
        Some("full")
    );
    assert_eq!(
        entry.get("baseTextSource").and_then(|value| value.as_str()),
        Some("shadow_state")
    );
    assert_eq!(
        entry.get("changeShape").and_then(|value| value.as_str()),
        Some("ranged")
    );
    assert_eq!(
        entry
            .get("contentChangesCount")
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        entry.get("replayOrder").and_then(|value| value.as_str()),
        Some("receive_order")
    );
    assert_eq!(
        entry
            .get("baseDocumentVersion")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(
        entry
            .get("changedRangesCount")
            .and_then(|value| value.as_u64()),
        Some(0)
    );
    assert_eq!(
        entry.get("fallbackReason").and_then(|value| value.as_str()),
        Some("input_edit_conversion_failed")
    );

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_incremental_mode_for_valid_multi_range_did_change() {
    fn utf16_range_for_line_fragment(line: &str, line_number: u32, needle: &str) -> Range {
        let start_byte = line
            .find(needle)
            .unwrap_or_else(|| panic!("needle not found: {needle}"));
        let end_byte = start_byte + needle.len();
        Range {
            start: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, start_byte),
            ),
            end: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, end_byte),
            ),
        }
    }

    let base_text = "Процедура Тест()\n    Сообщить(\"один два\");\nКонецПроцедуры\n";
    let base_line = "    Сообщить(\"один два\");";
    let after_first_line = "    Сообщить(\"оченьдлинно два\");";
    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        base_text,
        "file:///did_change_parse_snapshot_multi_range_fixture.bsl",
    )
    .await;

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize same-version ready parse snapshot before multi-range didChange");

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![
            TextDocumentContentChangeEvent {
                range: Some(utf16_range_for_line_fragment(base_line, 1, "один")),
                range_length: None,
                text: "оченьдлинно".to_string(),
            },
            TextDocumentContentChangeEvent {
                range: Some(utf16_range_for_line_fragment(after_first_line, 1, "два")),
                range_length: None,
                text: "три".to_string(),
            },
        ],
    };
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    let evidence = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let execute = Request::build("workspace/executeCommand")
                .id(2219)
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
            let value = serde_json::to_value(&execute_response).expect("serialize response");
            let result = value.get("result").cloned().expect("result field");
            let evidence = result
                .get("didChangeParseSnapshotEvidence")
                .and_then(|value| value.get("entries"))
                .and_then(|value| value.as_array())
                .and_then(|entries| {
                    entries.iter().find(|entry| {
                        entry.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && entry
                                .get("requestedVersion")
                                .and_then(|value| value.as_i64())
                                == Some(2)
                    })
                })
                .cloned();
            if let Some(entry) = evidence {
                break (result, entry);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("valid multi-range didChange evidence must appear in observability metrics response");

    let (result, entry) = evidence;
    assert_eq!(
        result
            .get("didChangeParseSnapshotEvidence")
            .and_then(|value| value.get("version"))
            .and_then(|value| value.as_u64()),
        Some(crate::server::DID_CHANGE_PARSE_SNAPSHOT_EVIDENCE_VERSION as u64)
    );
    assert_eq!(
        entry.get("parseMode").and_then(|value| value.as_str()),
        Some("incremental")
    );
    assert_eq!(
        entry.get("baseTextSource").and_then(|value| value.as_str()),
        Some("shadow_state")
    );
    assert_eq!(
        entry.get("changeShape").and_then(|value| value.as_str()),
        Some("ranged")
    );
    assert_eq!(
        entry
            .get("contentChangesCount")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    assert_eq!(
        entry.get("replayOrder").and_then(|value| value.as_str()),
        Some("receive_order")
    );
    assert_eq!(
        entry
            .get("baseDocumentVersion")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(
        entry
            .get("changedRangesCount")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    assert_eq!(
        entry.get("fallbackReason").and_then(|value| value.as_str()),
        None,
        "valid multi-range didChange must not false-fallback to edits_do_not_match_new_content"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_incremental_mode_for_valid_bom_crlf_receive_order_did_change(
) {
    fn utf16_range_for_line_fragment(line: &str, line_number: u32, needle: &str) -> Range {
        let start_byte = line
            .find(needle)
            .unwrap_or_else(|| panic!("needle not found: {needle}"));
        let end_byte = start_byte + needle.len();
        Range {
            start: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, start_byte),
            ),
            end: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, end_byte),
            ),
        }
    }

    const BASE_TEXT: &str =
        "\u{feff}Процедура Тест()\r\n    Сообщить(\"один два\");\r\nКонецПроцедуры\r\n";
    const BASE_LINE: &str = "    Сообщить(\"один два\");";
    const AFTER_FIRST_TEXT: &str =
        "\u{feff}Процедура Тест()\r\n    Сообщить(\"оченьдлинно два\");\r\nКонецПроцедуры\r\n";
    const AFTER_FIRST_LINE: &str = "    Сообщить(\"оченьдлинно два\");";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        BASE_TEXT,
        "file:///did_change_parse_snapshot_bom_crlf_receive_order_fixture.bsl",
    )
    .await;

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize same-version ready parse snapshot before BOM+CRLF didChange");

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![
            TextDocumentContentChangeEvent {
                range: Some(utf16_range_for_line_fragment(BASE_LINE, 1, "один")),
                range_length: None,
                text: "оченьдлинно".to_string(),
            },
            TextDocumentContentChangeEvent {
                range: Some(utf16_range_for_line_fragment(AFTER_FIRST_LINE, 1, "два")),
                range_length: None,
                text: "три".to_string(),
            },
        ],
    };
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    let evidence = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let execute = Request::build("workspace/executeCommand")
                .id(2220)
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
            let value = serde_json::to_value(&execute_response).expect("serialize response");
            let result = value.get("result").cloned().expect("result field");
            let evidence = result
                .get("didChangeParseSnapshotEvidence")
                .and_then(|value| value.get("entries"))
                .and_then(|value| value.as_array())
                .and_then(|entries| {
                    entries.iter().find(|entry| {
                        entry.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && entry
                                .get("requestedVersion")
                                .and_then(|value| value.as_i64())
                                == Some(2)
                    })
                })
                .cloned();
            if let Some(entry) = evidence {
                break (result, entry);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("valid BOM+CRLF didChange evidence must appear in observability metrics response");

    let (result, entry) = evidence;
    assert_eq!(
        result
            .get("didChangeParseSnapshotEvidence")
            .and_then(|value| value.get("version"))
            .and_then(|value| value.as_u64()),
        Some(crate::server::DID_CHANGE_PARSE_SNAPSHOT_EVIDENCE_VERSION as u64)
    );
    assert_eq!(
        entry.get("parseMode").and_then(|value| value.as_str()),
        Some("incremental")
    );
    assert_eq!(
        entry.get("baseTextSource").and_then(|value| value.as_str()),
        Some("shadow_state")
    );
    assert_eq!(
        entry.get("changeShape").and_then(|value| value.as_str()),
        Some("ranged")
    );
    assert_eq!(
        entry
            .get("contentChangesCount")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    assert_eq!(
        entry.get("replayOrder").and_then(|value| value.as_str()),
        Some("receive_order")
    );
    assert_eq!(
        entry
            .get("baseDocumentVersion")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(
        entry
            .get("changedRangesCount")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    assert_eq!(
        entry.get("fallbackReason").and_then(|value| value.as_str()),
        None,
        "valid BOM+CRLF didChange must not false-fallback to edits_do_not_match_new_content"
    );

    let analysis = server.analysis_v2.snapshot().await;
    let observed_text = analysis
        .file_text(file_id)
        .expect("file_text query")
        .expect("file text after BOM+CRLF didChange");
    assert_eq!(
        observed_text.as_ref(),
        "\u{feff}Процедура Тест()\r\n    Сообщить(\"оченьдлинно три\");\r\nКонецПроцедуры\r\n"
    );

    let _ = AFTER_FIRST_TEXT;
    drain_task.abort();
}

#[tokio::test]
async fn p22_did_change_reseeds_stale_parser_tree_cache_from_matching_ready_snapshot() {
    fn utf16_range_for_line_fragment(line: &str, line_number: u32, needle: &str) -> Range {
        let start_byte = line
            .find(needle)
            .unwrap_or_else(|| panic!("needle not found: {needle}"));
        let end_byte = start_byte + needle.len();
        Range {
            start: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, start_byte),
            ),
            end: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, end_byte),
            ),
        }
    }

    const V1_TEXT: &str = "Процедура Тест()\n    Сообщить(\"один\");\nКонецПроцедуры\n";
    const V2_TEXT: &str = "Процедура Тест()\n    Сообщить(\"два\");\nКонецПроцедуры\n";
    const V2_LINE: &str = "    Сообщить(\"два\");";

    let _env_lock = lock_test_env().await;
    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        V1_TEXT,
        "file:///did_change_parse_snapshot_stale_parser_base_fixture.bsl",
    )
    .await;

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize same-version ready parse snapshot before stale-base didChange");

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, V2_TEXT).await;

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("version 2 must materialize ready parse snapshot before poisoning parser tree cache");

    let parser = server
        .coordinator
        .parser_coordinator()
        .expect("parser coordinator");
    let file_path = uri.to_file_path().expect("file path");
    let poisoned_report = parser
        .parse_incremental_with_report(file_path.clone(), V1_TEXT.to_string(), Vec::new())
        .expect("poison stale parser tree cache with version 1");
    assert!(
        !poisoned_report.incremental,
        "poisoning the parser tree cache should force a full parse of version 1"
    );

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 3,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: Some(utf16_range_for_line_fragment(V2_LINE, 1, "два")),
            range_length: None,
            text: "три".to_string(),
        }],
    };
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    let evidence = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let execute = Request::build("workspace/executeCommand")
                .id(2221)
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
            let value = serde_json::to_value(&execute_response).expect("serialize response");
            let result = value.get("result").cloned().expect("result field");
            let evidence = result
                .get("didChangeParseSnapshotEvidence")
                .and_then(|value| value.get("entries"))
                .and_then(|value| value.as_array())
                .and_then(|entries| {
                    entries.iter().find(|entry| {
                        entry.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && entry
                                .get("requestedVersion")
                                .and_then(|value| value.as_i64())
                                == Some(3)
                    })
                })
                .cloned();
            if let Some(entry) = evidence {
                break entry;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("stale parser base didChange evidence must appear in observability metrics response");

    assert_eq!(
        evidence.get("parseMode").and_then(|value| value.as_str()),
        Some("incremental")
    );
    assert_eq!(
        evidence
            .get("baseTextSource")
            .and_then(|value| value.as_str()),
        Some("shadow_state")
    );
    assert_eq!(
        evidence
            .get("baseDocumentVersion")
            .and_then(|value| value.as_i64()),
        Some(2)
    );
    assert_eq!(
        evidence
            .get("changedRangesCount")
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        evidence.get("fallbackReason").and_then(|value| value.as_str()),
        None,
        "matching ready snapshot must reseed stale parser tree cache instead of falling back to edits_do_not_match_new_content"
    );
    assert_eq!(
        evidence
            .get("parserBaseRootCause")
            .and_then(|value| value.as_str()),
        None,
        "successful reseed must not emit stale parser-base root-cause attribution"
    );

    let analysis = server.analysis_v2.snapshot().await;
    let observed_text = analysis
        .file_text(file_id)
        .expect("file_text query")
        .expect("file text after stale-base didChange");
    assert_eq!(
        observed_text.as_ref(),
        "Процедура Тест()\n    Сообщить(\"три\");\nКонецПроцедуры\n"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p25_did_change_recovers_lagging_ready_snapshot_shadow_base_before_full_fallback() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    fn utf16_range_for_line_fragment(line: &str, line_number: u32, needle: &str) -> Range {
        let start_byte = line
            .find(needle)
            .unwrap_or_else(|| panic!("needle not found: {needle}"));
        let end_byte = start_byte + needle.len();
        Range {
            start: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, start_byte),
            ),
            end: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, end_byte),
            ),
        }
    }

    const V1_TEXT: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V2_TEXT: &str = "Процедура Тест()\n    Возврат 20;\nКонецПроцедуры\n";
    const V3_TEXT: &str = "Процедура Тест()\n    Возврат 300;\nКонецПроцедуры\n";
    const V3_LINE: &str = "    Возврат 300;";

    let _env_lock = lock_test_env().await;
    let _blocking_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "1500");

    let (mut service, drain_task, server, uri, file_id) =
        open_lsp_fixture_with_snapshot(V1_TEXT, "file:///did_change_stale_parser_base_fixture.bsl")
            .await;

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize version 1 before churn");

    for (version, text) in [(2, V2_TEXT), (3, V3_TEXT)] {
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(
                Request::build("textDocument/didChange")
                    .params(
                        serde_json::to_value(DidChangeTextDocumentParams {
                            text_document: VersionedTextDocumentIdentifier {
                                uri: uri.clone(),
                                version,
                            },
                            content_changes: vec![TextDocumentContentChangeEvent {
                                range: None,
                                range_length: None,
                                text: text.to_string(),
                            }],
                        })
                        .expect("DidChangeTextDocumentParams"),
                    )
                    .finish(),
            )
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");
    }

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let shadow = server
                .latest_document_shadow_state_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if shadow.as_ref().is_some_and(|state| state.version == 3) && ready_version == Some(1) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("shadow state must advance to version 3 while ready snapshot still lags at version 1");
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(
                    serde_json::to_value(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: uri.clone(),
                            version: 4,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: Some(utf16_range_for_line_fragment(V3_LINE, 1, "300")),
                            range_length: None,
                            text: "4000".to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    let evidence = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let execute = Request::build("workspace/executeCommand")
                .id(2222)
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
            let value = serde_json::to_value(&execute_response).expect("serialize response");
            let result = value.get("result").cloned().expect("result field");
            let evidence = result
                .get("didChangeParseSnapshotEvidence")
                .and_then(|value| value.get("entries"))
                .and_then(|value| value.as_array())
                .and_then(|entries| {
                    entries.iter().find(|entry| {
                        entry.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && entry
                                .get("requestedVersion")
                                .and_then(|value| value.as_i64())
                                == Some(4)
                    })
                })
                .cloned();
            if let Some(entry) = evidence {
                break entry;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("stale parser base didChange evidence must appear in observability metrics response");

    assert_eq!(
        evidence.get("parseMode").and_then(|value| value.as_str()),
        Some("incremental")
    );
    assert_eq!(
        evidence
            .get("baseTextSource")
            .and_then(|value| value.as_str()),
        Some("shadow_state")
    );
    assert_eq!(
        evidence.get("changeShape").and_then(|value| value.as_str()),
        Some("ranged")
    );
    assert_eq!(
        evidence
            .get("contentChangesCount")
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        evidence
            .get("baseDocumentVersion")
            .and_then(|value| value.as_i64()),
        Some(3)
    );
    assert_eq!(
        evidence
            .get("changedRangesCount")
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        evidence.get("fallbackReason").and_then(|value| value.as_str()),
        None,
        "lagging ready snapshot root cause must first recover a parser base instead of paying immediate stale fallback"
    );
    assert_eq!(
        evidence
            .get("parserBaseRootCause")
            .and_then(|value| value.as_str()),
        None,
        "successful bounded recovery must clear stale parser-base attribution"
    );
    assert_eq!(
        evidence
            .get("shadowDocumentVersion")
            .and_then(|value| value.as_i64()),
        None
    );
    assert_eq!(
        evidence
            .get("latestReadyDocumentVersion")
            .and_then(|value| value.as_i64()),
        None
    );
    assert_eq!(
        evidence
            .get("matchingReadySnapshotForShadowState")
            .and_then(|value| value.as_bool()),
        None
    );
    assert_eq!(
        evidence
            .get("readySnapshotPrimeAttempted")
            .and_then(|value| value.as_bool()),
        None
    );
    assert_eq!(
        evidence
            .get("treeCacheMatchesShadowTextAfterPrime")
            .and_then(|value| value.as_bool()),
        None
    );
    let analysis = server.analysis_v2.snapshot().await;
    let observed_text = analysis
        .file_text(file_id)
        .expect("file_text query")
        .expect("file text after recovered lagging-base didChange");
    assert_eq!(
        observed_text.as_ref(),
        "Процедура Тест()\n    Возврат 4000;\nКонецПроцедуры\n"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p25_did_change_preserves_truthful_stale_fallback_when_lagging_shadow_recovery_fails() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    fn utf16_range_for_line_fragment(line: &str, line_number: u32, needle: &str) -> Range {
        let start_byte = line
            .find(needle)
            .unwrap_or_else(|| panic!("needle not found: {needle}"));
        let end_byte = start_byte + needle.len();
        Range {
            start: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, start_byte),
            ),
            end: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, end_byte),
            ),
        }
    }

    const V1_TEXT: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V2_TEXT: &str = "Процедура Тест()\n    Возврат 20;\nКонецПроцедуры\n";
    const V3_TEXT: &str = "Процедура Тест()\n    Возврат 300;\nКонецПроцедуры\n";
    const V3_LINE: &str = "    Возврат 300;";

    let _env_lock = lock_test_env().await;
    let _blocking_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "1500");
    let _poison_after_recovery_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_POISON_TREE_CACHE_AFTER_RECOVERY", "1");

    let (mut service, drain_task, server, uri, file_id) =
        open_lsp_fixture_with_snapshot(V1_TEXT, "file:///did_change_stale_parser_base_fixture.bsl")
            .await;

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize version 1 before churn");

    for (version, text) in [(2, V2_TEXT), (3, V3_TEXT)] {
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(
                Request::build("textDocument/didChange")
                    .params(
                        serde_json::to_value(DidChangeTextDocumentParams {
                            text_document: VersionedTextDocumentIdentifier {
                                uri: uri.clone(),
                                version,
                            },
                            content_changes: vec![TextDocumentContentChangeEvent {
                                range: None,
                                range_length: None,
                                text: text.to_string(),
                            }],
                        })
                        .expect("DidChangeTextDocumentParams"),
                    )
                    .finish(),
            )
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");
    }

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let shadow = server
                .latest_document_shadow_state_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if shadow.as_ref().is_some_and(|state| state.version == 3) && ready_version == Some(1) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("shadow state must advance to version 3 while ready snapshot still lags at version 1");
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(
                    serde_json::to_value(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: uri.clone(),
                            version: 4,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: Some(utf16_range_for_line_fragment(V3_LINE, 1, "300")),
                            range_length: None,
                            text: "4000".to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    let evidence = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let execute = Request::build("workspace/executeCommand")
                .id(22225)
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
            let value = serde_json::to_value(&execute_response).expect("serialize response");
            let result = value.get("result").cloned().expect("result field");
            let evidence = result
                .get("didChangeParseSnapshotEvidence")
                .and_then(|value| value.get("entries"))
                .and_then(|value| value.as_array())
                .and_then(|entries| {
                    entries.iter().find(|entry| {
                        entry.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && entry
                                .get("requestedVersion")
                                .and_then(|value| value.as_i64())
                                == Some(4)
                    })
                })
                .cloned();
            if let Some(entry) = evidence {
                break entry;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("stale parser base fallback evidence must appear in observability metrics response");

    assert_eq!(
        evidence.get("parseMode").and_then(|value| value.as_str()),
        Some("full")
    );
    assert_eq!(
        evidence
            .get("fallbackReason")
            .and_then(|value| value.as_str()),
        Some("stale_parser_base"),
        "failed lagging-shadow recovery must preserve truthful stale fallback attribution"
    );
    assert_eq!(
        evidence
            .get("parserBaseRootCause")
            .and_then(|value| value.as_str()),
        Some("ready_snapshot_lags_shadow_state")
    );
    assert_eq!(
        evidence
            .get("shadowDocumentVersion")
            .and_then(|value| value.as_i64()),
        Some(3)
    );
    assert_eq!(
        evidence
            .get("latestReadyDocumentVersion")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    drain_task.abort();
}

#[tokio::test]
async fn p22_did_change_stale_parser_base_distinguishes_missing_matching_ready_snapshot() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    fn utf16_range_for_line_fragment(line: &str, line_number: u32, needle: &str) -> Range {
        let start_byte = line
            .find(needle)
            .unwrap_or_else(|| panic!("needle not found: {needle}"));
        let end_byte = start_byte + needle.len();
        Range {
            start: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, start_byte),
            ),
            end: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, end_byte),
            ),
        }
    }

    const V1_TEXT: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V1_LINE: &str = "    Возврат 1;";
    const POISON_TEXT: &str = "Процедура Тест()\n    Возврат 0;\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let _blocking_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_OPEN_BLOCKING_PARSE_DELAY_MS", "4000");

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        V1_TEXT,
        "file:///did_change_no_matching_ready_snapshot_fixture.bsl",
    )
    .await;

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let shadow_version = server
                .latest_document_shadow_state_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.version);
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if shadow_version == Some(1) && ready_version.is_none() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didOpen delay must keep ready snapshot absent while shadow state is version 1");

    let parser = server
        .coordinator
        .parser_coordinator()
        .expect("parser coordinator");
    let file_path = uri.to_file_path().expect("file path");
    let poisoned_report = parser
        .parse_incremental_with_report(file_path, POISON_TEXT.to_string(), Vec::new())
        .expect("poison tree cache before no-matching-ready didChange");
    assert!(
        !poisoned_report.incremental,
        "poisoning tree cache should force a full parse for the mismatched base"
    );

    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(
                    serde_json::to_value(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: uri.clone(),
                            version: 2,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: Some(utf16_range_for_line_fragment(V1_LINE, 1, "1")),
                            range_length: None,
                            text: "20".to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    let evidence = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let execute = Request::build("workspace/executeCommand")
                .id(2223)
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
            let value = serde_json::to_value(&execute_response).expect("serialize response");
            let result = value.get("result").cloned().expect("result field");
            let evidence = result
                .get("didChangeParseSnapshotEvidence")
                .and_then(|value| value.get("entries"))
                .and_then(|value| value.as_array())
                .and_then(|entries| {
                    entries.iter().find(|entry| {
                        entry.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && entry
                                .get("requestedVersion")
                                .and_then(|value| value.as_i64())
                                == Some(2)
                    })
                })
                .cloned();
            if let Some(entry) = evidence {
                break entry;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("no-matching-ready didChange evidence must appear in observability metrics response");

    assert_eq!(
        evidence
            .get("fallbackReason")
            .and_then(|value| value.as_str()),
        Some("stale_parser_base")
    );
    assert_eq!(
        evidence
            .get("parserBaseRootCause")
            .and_then(|value| value.as_str()),
        Some("no_matching_ready_snapshot_for_shadow_state")
    );
    assert_eq!(
        evidence
            .get("shadowDocumentVersion")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(
        evidence
            .get("latestReadyDocumentVersion")
            .and_then(|value| value.as_i64()),
        None
    );
    assert_eq!(
        evidence
            .get("matchingReadySnapshotForShadowState")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        evidence
            .get("readySnapshotPrimeAttempted")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        evidence
            .get("treeCacheMatchesShadowTextAfterPrime")
            .and_then(|value| value.as_bool()),
        None
    );

    drain_task.abort();
}

#[tokio::test]
async fn p22_did_change_stale_parser_base_distinguishes_tree_cache_mismatch_after_prime() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    fn utf16_range_for_line_fragment(line: &str, line_number: u32, needle: &str) -> Range {
        let start_byte = line
            .find(needle)
            .unwrap_or_else(|| panic!("needle not found: {needle}"));
        let end_byte = start_byte + needle.len();
        Range {
            start: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, start_byte),
            ),
            end: Position::new(
                line_number,
                bsl_line_index::byte_offset_to_utf16(line, end_byte),
            ),
        }
    }

    const V1_TEXT: &str = "Процедура Тест()\n    Сообщить(\"один\");\nКонецПроцедуры\n";
    const V2_TEXT: &str = "Процедура Тест()\n    Сообщить(\"два\");\nКонецПроцедуры\n";
    const V2_LINE: &str = "    Сообщить(\"два\");";

    let _env_lock = lock_test_env().await;
    let _poison_after_prime_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_POISON_TREE_CACHE_AFTER_PRIME", "1");

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        V1_TEXT,
        "file:///did_change_tree_cache_mismatch_after_prime_fixture.bsl",
    )
    .await;

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize version 1 before post-prime mismatch test");

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, V2_TEXT).await;

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("version 2 must materialize ready parse snapshot before post-prime mismatch test");

    let parser = server
        .coordinator
        .parser_coordinator()
        .expect("parser coordinator");
    let file_path = uri.to_file_path().expect("file path");
    let poisoned_report = parser
        .parse_incremental_with_report(file_path, V1_TEXT.to_string(), Vec::new())
        .expect("poison stale parser tree cache with version 1");
    assert!(
        !poisoned_report.incremental,
        "poisoning the parser tree cache should force a full parse of version 1"
    );

    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(
                    serde_json::to_value(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: uri.clone(),
                            version: 3,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: Some(utf16_range_for_line_fragment(V2_LINE, 1, "два")),
                            range_length: None,
                            text: "три".to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    let evidence = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let execute = Request::build("workspace/executeCommand")
                .id(2224)
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
            let value = serde_json::to_value(&execute_response).expect("serialize response");
            let result = value.get("result").cloned().expect("result field");
            let evidence = result
                .get("didChangeParseSnapshotEvidence")
                .and_then(|value| value.get("entries"))
                .and_then(|value| value.as_array())
                .and_then(|entries| {
                    entries.iter().find(|entry| {
                        entry.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && entry
                                .get("requestedVersion")
                                .and_then(|value| value.as_i64())
                                == Some(3)
                    })
                })
                .cloned();
            if let Some(entry) = evidence {
                break entry;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("post-prime mismatch didChange evidence must appear in observability metrics response");

    assert_eq!(
        evidence
            .get("fallbackReason")
            .and_then(|value| value.as_str()),
        Some("stale_parser_base")
    );
    assert_eq!(
        evidence
            .get("parserBaseRootCause")
            .and_then(|value| value.as_str()),
        Some("tree_cache_mismatch_after_prime")
    );
    assert_eq!(
        evidence
            .get("shadowDocumentVersion")
            .and_then(|value| value.as_i64()),
        Some(2)
    );
    assert_eq!(
        evidence
            .get("latestReadyDocumentVersion")
            .and_then(|value| value.as_i64()),
        Some(2)
    );
    assert_eq!(
        evidence
            .get("matchingReadySnapshotForShadowState")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        evidence
            .get("readySnapshotPrimeAttempted")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        evidence
            .get("treeCacheMatchesShadowTextAfterPrime")
            .and_then(|value| value.as_bool()),
        Some(false)
    );

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_semantic_diagnostics_breakdown() {
    const SEMANTIC_FIXTURE: &str = "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.НесуществующийМетод();\nКонецПроцедуры\n";

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
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

    let uri = Url::parse("file:///semantic_breakdown_fixture.bsl").expect("fixture uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: SEMANTIC_FIXTURE.to_string(),
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
    let diagnostics = wait_lsp_publish_diagnostics(&mut published_rx, &uri).await;
    assert!(
        !diagnostics.is_empty(),
        "semantic fixture must publish diagnostics before metrics snapshot"
    );

    let execute = Request::build("workspace/executeCommand")
        .id(2204)
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
    let value = serde_json::to_value(&execute_response).expect("serialize response");
    let result = value.get("result").cloned().expect("result field");
    let metrics = result.get("metrics").expect("metrics field");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    for key in [
        "intellisense_v2_semantic_diagnostics_query_inputs_ms",
        "intellisense_v2_semantic_diagnostics_query_parse_result_ms",
        "intellisense_v2_semantic_diagnostics_query_ir_ms",
        "intellisense_v2_semantic_diagnostics_query_collect_ms",
    ] {
        let count = histograms
            .get(key)
            .and_then(|value| value.get("count"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        assert!(
            count > 0,
            "metrics endpoint must expose semantic diagnostics breakdown histogram {key}"
        );
    }

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_type_index_precompute_breakdown() {
    let coordinator = Arc::new(SystemCoordinator::new());
    for (stage, millis) in [
        ("type_index_precompute", 4800_u64),
        ("type_index_precompute_build", 14_u64),
        ("type_index_precompute_ir", 4700_u64),
        ("type_index_precompute_ast_to_ir", 1900_u64),
        ("type_index_precompute_semantic_facts", 2600_u64),
        (
            "type_index_precompute_semantic_facts_seed_module_context",
            120_u64,
        ),
        (
            "type_index_precompute_semantic_facts_local_function_summaries",
            2300_u64,
        ),
        (
            "type_index_precompute_semantic_facts_visit_statements",
            180_u64,
        ),
    ] {
        coordinator.record_intellisense_v2_runtime_exec_latency_with_origin(
            bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
            stage,
            Duration::from_millis(millis),
        );
    }

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let execute = Request::build("workspace/executeCommand")
        .id(2205)
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
    let value = serde_json::to_value(&execute_response).expect("serialize response");
    let result = value.get("result").cloned().expect("result field");
    let metrics = result.get("metrics").expect("metrics field");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    for key in [
        "intellisense_v2_runtime_type_index_precompute_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_build_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_ir_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms",
    ] {
        let count = histograms
            .get(key)
            .and_then(|value| value.get("count"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        assert!(
            count > 0,
            "metrics endpoint must expose type-index precompute breakdown histogram {key}"
        );
    }

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_completion_timeline_exposes_versioned_contract() {
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

    let execute = Request::build("workspace/executeCommand")
        .id(2)
        .params(serde_json::json!({
            "command": "bsl.getCompletionTimeline",
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
    assert_eq!(
        result
            .get("version")
            .and_then(|value| value.as_u64())
            .expect("version"),
        crate::server::COMPLETION_TIMELINE_VERSION as u64
    );
    assert!(
        result
            .get("traces")
            .and_then(|value| value.as_array())
            .is_some(),
        "result must contain traces array, got {result}"
    );

    drain_task.abort();
}
