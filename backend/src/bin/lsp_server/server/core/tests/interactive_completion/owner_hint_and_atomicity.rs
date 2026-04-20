#[tokio::test]
async fn p7_completion_owner_hint_type_lookup_is_serve_only_even_when_flow_sensitive_enabled() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let settings = DidChangeConfigurationParams {
        settings: serde_json::json!({
            "bsl": {
                "hover": {
                    "detailLevel": "full",
                    "maxMethods": 10,
                    "maxProperties": 5,
                    "showCertainty": true
                },
                "diagnostics": {
                    "detailLevel": "standard",
                    "showHints": true
                },
                "formatting": {
                    "enabled": false,
                    "indentSize": 4
                },
                "typeHints": {
                    "enabled": true,
                    "showVariableTypes": true,
                    "showReturnTypes": true,
                    "showUnionDetails": true,
                    "minCertainty": 0.5
                },
                "codeActions": {
                    "enabled": false
                },
                "enableFlowSensitive": true
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_response = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_response.is_none(),
        "didChangeConfiguration is a notification"
    );

    let fixture =
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_owner_hint_serve_only.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.to_string(),
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

    let completion_character = "    ЛокМассив."
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;
    let completion = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/completion")
                .id(9001)
                .params(
                    serde_json::to_value(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: Position::new(2, completion_character),
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                        context: Some(CompletionContext {
                            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                            trigger_character: Some(".".to_string()),
                        }),
                    })
                    .expect("CompletionParams"),
                )
                .finish(),
        )
        .await
        .expect("completion request");
    assert!(completion.is_some(), "completion must return a response");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");

    let lookup_path_direct_total = read_u64_metric(
        counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_direct"),
    );
    let lookup_path_flow_only_total = read_u64_metric(
        counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_flow_only"),
    );
    let lookup_path_flow_plus_fallback_total = read_u64_metric(
        counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_flow_plus_fallback"),
    );
    assert!(
        lookup_path_direct_total > 0,
        "owner-hint type lookup must stay on direct serve-only path under flow-sensitive mode"
    );
    assert_eq!(
        lookup_path_flow_only_total, 0,
        "flow-only owner-hint lookup path must not run in strict serve-only mode"
    );
    assert_eq!(
        lookup_path_flow_plus_fallback_total, 0,
        "flow+fallback owner-hint lookup path must not run in strict serve-only mode"
    );
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_drilldown_stage_total_origin_lsp_mode_event_driven_operation_completion_stage_parse_result_query",
        )),
        0,
        "strict serve-only completion path must not execute parse_result query stage"
    );

    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_type_index_total",
        ),),
        0,
        "serve-only owner-hint path must not execute type_index compute in request path"
    );
    assert_eq!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_parse_result_total",
            ),
        ),
        0,
        "serve-only owner-hint path must not execute parse_result compute in request path"
    );
    assert_eq!(
        read_u64_metric(
            counters
                .get("intellisense_v2_completion_owner_hint_index_fetch_block_on_type_index_total"),
        ),
        0,
        "serve-only owner-hint path must not block on type_index compute"
    );
    assert_eq!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_completion_owner_hint_index_fetch_block_on_parse_result_total",
            ),
        ),
        0,
        "serve-only owner-hint path must not block on parse_result compute"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_interactive_default_paths_emit_shared_type_index_reasons_on_exact_lookups() {
    const TYPE_INDEX_REASON_PREFIX: &str = "intellisense_v2_type_index_reason_total_reason_";
    const INTERACTIVE_REASON_COUNTER_KEYS: &[&str] = &[
        "intellisense_v2_type_index_reason_total_reason_type_index_exact_hit",
        "intellisense_v2_type_index_reason_total_reason_type_index_fallback_unavailable",
        "intellisense_v2_type_index_reason_total_reason_other",
    ];
    const ALLOWED_TYPE_INDEX_REASON_SUFFIXES: &[&str] = &[
        "type_index_exact_hit",
        "type_index_fallback_unavailable",
        "type_index_precompute_exact_stored",
        "type_index_precompute_superseded",
        "type_index_precompute_cancelled",
        "type_index_precompute_missing_file",
        "type_index_precompute_queue_saturated",
        "type_index_artifact_invalidated_deps",
        "type_index_artifact_invalidated_settings",
        "type_index_artifact_evicted_global_guard",
        "type_index_artifact_evicted_per_file_window",
        "other",
    ];

    let interactive_reason_total = |coordinator: &Arc<SystemCoordinator>| -> u64 {
        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        INTERACTIVE_REASON_COUNTER_KEYS
            .iter()
            .map(|key| read_u64_metric(counters.get(*key)))
            .sum()
    };

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура Тест()\n\
МассивДляCompletion = Новый Массив;\n\
МассивДляCompletion.\n\
\n\
МассивДляHover = Новый Массив;\n\
ЗначДляHover = МассивДляHover.Количество();\n\
\n\
МассивДляSignature = Новый Массив;\n\
МассивДляSignature.Количество();\n\
\n\
МассивДляDefinition = Новый Массив;\n\
МассивДляDefinition.Количество();\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_interactive_type_index_reasons.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.to_string(),
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

    let completion_position = find_utf16_position_after_marker(fixture, "МассивДляCompletion.");
    let hover_position = find_utf16_position_after_marker(fixture, "ЗначДляHover = МассивДляHover");
    let signature_position =
        find_utf16_position_after_marker(fixture, "МассивДляSignature.Количество(");
    let definition_position =
        find_utf16_position_after_marker(fixture, "МассивДляDefinition.Количество");

    let before_completion = interactive_reason_total(&coordinator);
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/completion")
                .id(9101)
                .params(
                    serde_json::to_value(CompletionParams {
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
                    })
                    .expect("CompletionParams"),
                )
                .finish(),
        )
        .await
        .expect("completion request");
    assert!(
        completion_response.is_some(),
        "completion must return a response"
    );
    let after_completion = interactive_reason_total(&coordinator);
    assert!(
        after_completion == before_completion,
        "completion must reuse current semantic state without extra type_index serve reason"
    );

    let before_hover = interactive_reason_total(&coordinator);
    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/hover")
                .id(9102)
                .params(
                    serde_json::to_value(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: hover_position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                    })
                    .expect("HoverParams"),
                )
                .finish(),
        )
        .await
        .expect("hover request");
    assert!(
        hover_response.is_some(),
        "hover must return a response envelope"
    );
    let after_hover = interactive_reason_total(&coordinator);
    assert!(
        after_hover > before_hover,
        "hover must emit at least one type_index serve reason"
    );

    let before_signature = interactive_reason_total(&coordinator);
    let signature_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/signatureHelp")
                .id(9103)
                .params(
                    serde_json::to_value(tower_lsp::lsp_types::SignatureHelpParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: signature_position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        context: None,
                    })
                    .expect("SignatureHelpParams"),
                )
                .finish(),
        )
        .await
        .expect("signatureHelp request");
    assert!(
        signature_response.is_some(),
        "signatureHelp must return a response envelope"
    );
    let after_signature = interactive_reason_total(&coordinator);
    assert!(
        after_signature > before_signature,
        "signatureHelp must emit at least one type_index serve reason on the shared default path"
    );

    let before_definition = interactive_reason_total(&coordinator);
    let definition_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/definition")
                .id(9104)
                .params(
                    serde_json::to_value(GotoDefinitionParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: definition_position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    })
                    .expect("GotoDefinitionParams"),
                )
                .finish(),
        )
        .await
        .expect("definition request");
    assert!(
        definition_response.is_some(),
        "definition must return a response envelope"
    );
    let after_definition = interactive_reason_total(&coordinator);
    assert!(
        after_definition > before_definition,
        "definition must emit at least one type_index serve reason on the shared default path"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let allowed_suffixes: BTreeSet<&str> =
        ALLOWED_TYPE_INDEX_REASON_SUFFIXES.iter().copied().collect();
    for key in counters.keys() {
        if let Some(suffix) = key.strip_prefix(TYPE_INDEX_REASON_PREFIX) {
            assert!(
                allowed_suffixes.contains(suffix),
                "type_index reason escaped bounded taxonomy: key={key}, suffix={suffix}"
            );
        }
    }
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_observability_contract_violation_reason_unknown_type_index_reason",
        )),
        0,
        "interactive serve reasons must stay in known taxonomy and not trigger unknown reason violation"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p8_deps_update_is_atomic_and_completion_uses_runtime_index_snapshot() {
    fn make_index_snapshot(id: &str, type_name: &str) -> IndexSnapshot {
        let mut snapshot = IndexSnapshot::empty(IndexSnapshotId::from_hash(id.to_string()));
        Arc::make_mut(&mut snapshot.type_index).insert(
            type_name.to_string(),
            Arc::new(IndexItem::new(
                type_name.to_string(),
                IndexItemKind::Type(TypeKind::Generic),
                IndexKind::Type,
            )),
        );
        snapshot
    }

    fn extract_completion_labels(
        response: tower_lsp::lsp_types::CompletionResponse,
    ) -> Vec<String> {
        match response {
            tower_lsp::lsp_types::CompletionResponse::Array(items) => {
                items.into_iter().map(|item| item.label).collect()
            }
            tower_lsp::lsp_types::CompletionResponse::List(list) => {
                list.items.into_iter().map(|item| item.label).collect()
            }
        }
    }

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().unwrap() = Some(server.clone());
            server
        }
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
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

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

    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be created");

    let snapshot_a = make_index_snapshot("p8_a", "P8TypeA");
    let snapshot_b = make_index_snapshot("p8_b", "P8TypeB");

    coordinator
        .intellisense_index()
        .replace_snapshot(snapshot_a.clone());
    let expected_deps_id_a = build_deps_bundle_v2(coordinator.as_ref(), None, None)
        .expect("bundle A")
        .deps_id;

    coordinator
        .intellisense_index()
        .replace_snapshot(snapshot_b.clone());
    let expected_deps_id_b = build_deps_bundle_v2(coordinator.as_ref(), None, None)
        .expect("bundle B")
        .deps_id;

    coordinator
        .intellisense_index()
        .replace_snapshot(snapshot_a.clone());
    server.deps_update_v2("p8_test_initial", None, None).await;

    let uri = Url::parse("file:///test_p8.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Procedure Test()\n\t// P8\nEndProcedure".to_string(),
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
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 1,
                character: 6,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None,
    };

    let completion_a = server
        .completion(completion_params.clone())
        .await
        .expect("completion")
        .expect("completion response");
    let labels_a = extract_completion_labels(completion_a);
    assert!(
        labels_a.iter().any(|label| label == "P8TypeA"),
        "expected completion to contain P8TypeA, got {:?}",
        labels_a
    );
    assert!(
        labels_a.iter().all(|label| label != "P8TypeB"),
        "unexpected P8TypeB in completion A: {:?}",
        labels_a
    );

    let update_task = tokio::spawn({
        let coordinator = coordinator.clone();
        let server = server.clone();
        let snapshot_b = snapshot_b.clone();
        async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            coordinator
                .intellisense_index()
                .replace_snapshot(snapshot_b);
            server.deps_update_v2("p8_test_update", None, None).await;
        }
    });

    for _ in 0..200 {
        let (_analysis, index_snapshot, deps_id) = server.analysis_v2.snapshot_with_deps().await;
        match index_snapshot.id.as_str() {
            "p8_a" => assert_eq!(deps_id.as_str(), expected_deps_id_a.as_str()),
            "p8_b" => assert_eq!(deps_id.as_str(), expected_deps_id_b.as_str()),
            other => panic!("unexpected index snapshot id: {}", other),
        }
    }

    update_task.await.expect("update task join");

    let completion_b = server
        .completion(completion_params)
        .await
        .expect("completion")
        .expect("completion response");
    let labels_b = extract_completion_labels(completion_b);
    assert!(
        labels_b.iter().any(|label| label == "P8TypeB"),
        "expected completion to contain P8TypeB, got {:?}",
        labels_b
    );
    assert!(
        labels_b.iter().all(|label| label != "P8TypeA"),
        "unexpected P8TypeA in completion B: {:?}",
        labels_b
    );

    drain_task.abort();
}
