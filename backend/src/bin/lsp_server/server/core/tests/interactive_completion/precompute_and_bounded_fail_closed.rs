#[tokio::test]
async fn p7_trigger_character_and_invoked_member_access_keep_semantic_parity() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;
    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p7_trigger_parity.bsl").expect("test uri");
    let text = concat!(
        "Процедура Тест()\n",
        "    ДляCompletion = (Новый Массив()).\n",
        "КонецПроцедуры\n"
    );
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
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

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: text.to_string(),
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
    let completion_position = find_utf16_position_after_marker(text, "(Новый Массив()).");
    let dot_response = server
        .completion(CompletionParams {
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
        .await
        .expect("dot completion request")
        .expect("dot completion response");

    let invoked_response = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: completion_position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
        })
        .await
        .expect("invoked completion request")
        .expect("invoked completion response");

    let extract_labels = |response: &CompletionResponse| -> Vec<String> {
        match response {
            CompletionResponse::Array(items) => {
                items.iter().map(|item| item.label.clone()).collect()
            }
            CompletionResponse::List(list) => {
                list.items.iter().map(|item| item.label.clone()).collect()
            }
        }
    };
    let dot_members = extract_labels(&dot_response);
    let invoked_members = extract_labels(&invoked_response);
    let metrics = coordinator.observability_metrics();
    assert!(
        !dot_members.is_empty(),
        "trigger-character completion must return candidates"
    );
    assert!(
        !invoked_members.is_empty(),
        "invoked completion must return candidates"
    );
    assert!(
        dot_members
            .iter()
            .any(|label| invoked_members.contains(label)),
        "trigger-character and invoked completion must have semantic overlap: dot={:?} invoked={:?}",
        dot_members,
        invoked_members
    );

    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let trigger_char_total = counters
        .get("intellisense_v2_completion_trigger_mode_total_mode_trigger_character")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let invoked_total = counters
        .get("intellisense_v2_completion_trigger_mode_total_mode_invoked")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let overlap_metric_recorded = counters.iter().any(|(key, value)| {
        key.starts_with("intellisense_v2_completion_parity_overlap_total_mode_invoked_bucket_")
            && value.as_u64().unwrap_or(0) > 0
    });
    assert!(
        trigger_char_total > 0,
        "trigger-character completion metric must be recorded"
    );
    assert!(
        invoked_total > 0,
        "invoked completion metric must be recorded"
    );
    assert!(
        overlap_metric_recorded,
        "semantic-overlap parity metric must be recorded"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_completion_context_modes_are_supported() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///test_p7_completion_context_modes.bsl").expect("test uri");
    let text = "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
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

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    let member_character = "    ЛокМассив."
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;
    let base_params = TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        position: Position::new(2, member_character),
    };

    let contexts = [
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS,
            trigger_character: None,
        }),
        None,
    ];

    for context in contexts {
        let response = server
            .completion(CompletionParams {
                text_document_position: base_params.clone(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context,
            })
            .await
            .expect("completion request");
        assert!(response.is_some(), "completion response must be present");
    }

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        counters
            .get("intellisense_v2_completion_trigger_mode_total_mode_trigger_character")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0
    );
    assert!(
        counters
            .get("intellisense_v2_completion_trigger_mode_total_mode_invoked")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0
    );
    assert!(
        counters
            .get("intellisense_v2_completion_trigger_mode_total_mode_trigger_for_incomplete")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0
    );
    assert!(
        counters
            .get("intellisense_v2_completion_trigger_mode_total_mode_none")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_first_completion_after_received_advance_fails_closed_empty() {
    const STALE_FIXTURE: &str =
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///test_p7_stale_completion.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: STALE_FIXTURE.to_string(),
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

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: STALE_FIXTURE.to_string(),
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

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    server
        .deps_update_v2("p7_stale_completion_setup", None, None)
        .await;
    server.sync_v2_globals().await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    {
        let mut versions = server.latest_received_file_versions_v2.write().await;
        // Simulate the window right after the next didChange was received but before runtime apply.
        versions.insert(file_id, 3);
    }

    let completion = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(2, 13),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        })
        .await
        .expect("completion request")
        .expect("completion response");

    match completion {
        CompletionResponse::List(list) => {
            assert!(
                list.items.is_empty(),
                "first completion after received-version advance must fail closed with empty payload"
            );
        }
        CompletionResponse::Array(items) => {
            assert!(
                items.is_empty(),
                "first completion after received-version advance must fail closed with empty payload"
            );
        }
    }

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let stale_fallback_total = counters
        .get("intellisense_v2_completion_stale_fallback_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let fallback_unavailable_total = counters
        .get("intellisense_v2_completion_fallback_unavailable_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let fail_closed_reason_total = counters
        .iter()
        .filter(|(key, _)| {
            key.starts_with(
                "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_completion_",
            )
        })
        .map(|(_, value)| value.as_u64().unwrap_or(0))
        .sum::<u64>();
    assert!(
        stale_fallback_total == 0,
        "stale fallback counter must stay zero under fail-closed contract"
    );
    assert!(
        fallback_unavailable_total > 0,
        "fail-closed completion miss must record fallback_unavailable"
    );
    assert!(
        fail_closed_reason_total > 0,
        "fail-closed completion miss must emit bounded public reason metrics"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_large_churn_budget_timeout_returns_fail_closed_empty_response() {
    const FILLER_LINES: usize = 2200;
    let mut fixture = String::new();
    fixture.push_str("Процедура Тест()\n");
    fixture.push_str("    ЛокМассив = Новый Массив;\n");
    for idx in 0..FILLER_LINES {
        fixture.push_str(&format!("    // filler {idx}\n"));
    }
    fixture.push_str("    ЛокМассив.\n");
    fixture.push_str("КонецПроцедуры\n");

    let completion_line = (2 + FILLER_LINES) as u32;
    let completion_character = "    ЛокМассив."
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///test_p7_large_churn_stale_fastpath.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.clone(),
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

    for version in 2..=7_i32 {
        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: fixture.clone(),
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
    }

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    server
        .deps_update_v2("p7_large_churn_stale_fastpath_setup", None, None)
        .await;
    server.sync_v2_globals().await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let large_churn_active = server
        .scale_aware_churn_state_v2
        .read()
        .await
        .get(&file_id)
        .is_some_and(|state| state.large_churn_active);
    assert!(
        large_churn_active,
        "expected large+churn state to be active after burst didChange on large document"
    );

    let warm_completion = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(completion_line, completion_character),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(".".to_string()),
            }),
        })
        .await
        .expect("warm completion request")
        .expect("warm completion response");
    let _ = warm_completion;

    {
        let mut versions = server.latest_received_file_versions_v2.write().await;
        versions.insert(file_id, 8);
    }

    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let started = Instant::now();
    let stale_completion = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(completion_line, completion_character),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(".".to_string()),
            }),
        })
        .await
        .expect("stale completion request")
        .expect("stale completion response");
    let elapsed = started.elapsed();

    match stale_completion {
        CompletionResponse::List(list) => {
            assert!(
                list.items.is_empty(),
                "large-churn timeout must fail closed with empty payload even when cache is warm"
            );
        }
        CompletionResponse::Array(items) => {
            assert!(
                items.is_empty(),
                "large-churn timeout must fail closed with empty payload even when cache is warm"
            );
        }
    }
    assert!(
        elapsed <= Duration::from_millis(wait_budget_ms.saturating_add(300)),
        "fail-closed completion should remain bounded near wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let stale_fallback_total = counters
        .get("intellisense_v2_completion_stale_fallback_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let fallback_unavailable_total = counters
        .get("intellisense_v2_completion_fallback_unavailable_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let fail_closed_reason_total = counters
        .iter()
        .filter(|(key, _)| {
            key.starts_with(
                "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_completion_",
            )
        })
        .map(|(_, value)| value.as_u64().unwrap_or(0))
        .sum::<u64>();
    assert!(
        stale_fallback_total == 0,
        "stale fallback counter must stay zero under fail-closed churn path"
    );
    assert!(
        fallback_unavailable_total > 0,
        "fail-closed churn path must record fallback_unavailable"
    );
    assert!(
        fail_closed_reason_total > 0,
        "fail-closed churn path must emit bounded public reason metrics"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_member_access_completion_does_not_backfill_from_runtime_index_snapshot_when_owner_unresolved(
) {
    const OWNER_UNRESOLVED_REASON_KEY: &str = "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_completion_reason_owner_unresolved";
    const MISSING_INDEX_REASON_KEY: &str = "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_completion_reason_missing_semantic_index";

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

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура Тест()\n\
НеизвестныйЛокал.\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_completion_no_search_rescue.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.to_string(),
        },
    };
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didOpen")
                .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    coordinator
        .intellisense_index()
        .replace_snapshot(make_index_snapshot(
            "p7_completion_no_search_rescue",
            "SearchOnlyType",
        ));
    coordinator.intellisense_index().replace_symbols_for_uri(
        uri.as_str(),
        vec![IndexItem::new(
            "SearchOnlySymbol".to_string(),
            IndexItemKind::Symbol(bsl_backend::system::SymbolKind::Function),
            IndexKind::Symbol,
        )],
    );
    coordinator.intellisense_index().replace_modules_for_key(
        "p7_completion_no_search_rescue_module",
        vec![IndexItem::new(
            "SearchOnlyModule".to_string(),
            IndexItemKind::Symbol(bsl_backend::system::SymbolKind::Procedure),
            IndexKind::Module,
        )],
    );

    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be captured");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let completion_position = find_utf16_position_after_marker(fixture, "НеизвестныйЛокал.");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        completion_labels.is_empty(),
        "member-access unresolved owner must stay fail-closed when only runtime discovery/search index is populated, labels={completion_labels:?}"
    );
    assert!(
        completion_labels
            .iter()
            .all(|label| label != "SearchOnlyType"
                && label != "SearchOnlySymbol"
                && label != "SearchOnlyModule"),
        "member-access unresolved owner must not backfill from runtime discovery/search index, labels={completion_labels:?}"
    );
    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    let fail_closed_reason_total = read_u64_metric(counters.get(OWNER_UNRESOLVED_REASON_KEY));
    assert!(
        fail_closed_reason_total > 0,
        "member-access unresolved owner must emit owner_unresolved bounded public reason metrics, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get(MISSING_INDEX_REASON_KEY)),
        0,
        "member-access unresolved owner must not be counted as missing_semantic_index, counters={counters:?}"
    );
    let ir_query_count = histograms
        .get("intellisense_v2_ir_query_completion_ms")
        .and_then(|value| value.as_object())
        .map(|histogram| read_u64_metric(histogram.get("count")))
        .unwrap_or(0);
    assert_eq!(
        ir_query_count, 0,
        "member-access fail-closed path must not build canonical IR when exact type index is unavailable, histograms={histograms:?}"
    );
    let query_bundle_count = histograms
        .get("completion_stage_query_bundle_ms")
        .and_then(|value| value.as_object())
        .map(|histogram| read_u64_metric(histogram.get("count")))
        .unwrap_or(0);
    assert_eq!(
        query_bundle_count, 0,
        "member-access fail-closed path must short-circuit before query_bundle, histograms={histograms:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_type_index_precompute_slot_coalesces_rapid_versions_without_respawn_or_cancel_fanout() {
    const CANCELLED_REASON_KEY: &str =
        "intellisense_v2_type_index_reason_total_reason_type_index_precompute_cancelled";

    let cancelled_reason_total = |coordinator: &Arc<SystemCoordinator>| -> u64 {
        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        read_u64_metric(counters.get(CANCELLED_REASON_KEY))
    };

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (_service, socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move {
        let mut socket = socket;
        while let Some(_req) = socket.next().await {}
    });

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");

    let before_cancelled = cancelled_reason_total(&coordinator);
    let file_id = bsl_analysis_v2::FileId(777);
    let initial_version = 4;
    let latest_version = 8_i32;
    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, latest_version);

    let initial_task_id = 321_u64;
    let initial_handle = tokio::spawn(std::future::pending::<()>());
    let initial_phase = Arc::new(std::sync::atomic::AtomicU8::new(
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::WaitingCpuPermit
            .as_u8(),
    ));
    {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        tasks.insert(
            file_id,
            crate::server::TypeIndexPrecomputeTaskV2 {
                task_id: initial_task_id,
                supersession_key: crate::server::TypeIndexPrecomputeSupersessionKeyV2 {
                    file_id,
                    requested_version: initial_version,
                },
                work_class: bsl_runtime::application::CpuWorkClass::Background,
                phase: Arc::clone(&initial_phase),
                active_requested_version: Arc::new(std::sync::atomic::AtomicI32::new(0)),
                scheduled_at: Instant::now(),
                handle: initial_handle,
            },
        );
    }

    for version in (initial_version + 1)..=latest_version {
        server
            .schedule_type_index_precompute_v2(file_id, version)
            .await;
    }

    {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        let task = tasks
            .get(&file_id)
            .expect("coalesced type-index precompute task must remain registered");
        assert_eq!(
            task.task_id, initial_task_id,
            "rapid reschedule burst must keep the same coordinated precompute slot instead of abort+respawn"
        );
        assert_eq!(
            task.supersession_key.requested_version, latest_version,
            "coalesced precompute slot must track the latest requested version"
        );
        assert_eq!(
            task.work_class,
            bsl_runtime::application::CpuWorkClass::Background,
            "didChange reschedule burst must keep background work class until an interactive waiter explicitly promotes it"
        );
        task.handle.abort();
        let _ = tasks.remove(&file_id);
    }
    let after_cancelled = cancelled_reason_total(&coordinator);
    assert_eq!(
        after_cancelled, before_cancelled,
        "rapid reschedule burst must not emit type_index_precompute_cancelled fanout when the slot can coalesce in place"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_non_member_completion_keeps_local_variables_when_exact_index_not_ready() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура Тест()\n\
    ТаблЗнач = Новый ТаблицаЗначений;\n\
    Целевой = ТаблЗна\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_non_member_local_variable_completion.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.to_string(),
        },
    };
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didOpen")
                .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be captured");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;
    server
        .latest_apply_enqueued_at_v2
        .write()
        .await
        .insert(file_id, Instant::now() - Duration::from_secs(1));

    let completion_position = find_utf16_position_after_marker(fixture, "Целевой = ТаблЗна");
    let completion_items = lsp_completion_items_with_request(
        &mut service,
        12002,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;
    let timeline = lsp_get_completion_timeline(&mut service, 12003, 10).await;
    assert!(
        completion_items.iter().any(|item| {
            item.label == "ТаблЗнач" && item.kind == Some(CompletionItemKind::VARIABLE)
        }),
        "non-member completion must keep local variable candidates when exact index is not ready, items={completion_items:?}, timeline={timeline:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_local_constructor_member_completion_returns_children_from_current_revision_head() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = "Процедура Тест()\n\
    Лок = Новый ТаблицаЗначений;\n\
    Лок.\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_local_constructor_member_children.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.to_string(),
        },
    };
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didOpen")
                .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let completion_position = find_utf16_position_after_marker(fixture, "Лок.");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        completion_labels.iter().any(|label| label == "Колонки"),
        "member completion must return ТаблицаЗначений property children for local constructor owner, labels={completion_labels:?}"
    );
    assert!(
        completion_labels.iter().any(|label| label == "ВыгрузитьКолонку"),
        "member completion must return ТаблицаЗначений method children for local constructor owner, labels={completion_labels:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_real_advance_report_tablznach_member_completion_returns_children_from_current_revision_head(
) {
    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = include_str!(
        "../../../../../../../../examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl"
    )
    .replace("\r\n", "\n");
    let assignment = "\tТаблЗнач = Новый ТаблицаЗначений;\n";
    let insertion = "\tТаблЗнач.\n";
    let insert_at = fixture
        .find(assignment)
        .map(|idx| idx + assignment.len())
        .expect("ТаблЗнач constructor assignment in fixture");
    let mut content = String::with_capacity(fixture.len() + insertion.len());
    content.push_str(&fixture[..insert_at]);
    content.push_str(insertion);
    content.push_str(&fixture[insert_at..]);

    let uri =
        Url::parse("file:///test_p7_real_advance_report_tablznach_member_children.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: content.clone(),
        },
    };
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didOpen")
                .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, &content, 2).await;

    let completion_position = find_utf16_position_after_marker(&content, insertion);
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        completion_labels.iter().any(|label| label == "Колонки"),
        "real fixture member completion must return ТаблицаЗначений property children for ТаблЗнач, labels={completion_labels:?}"
    );
    assert!(
        completion_labels.iter().any(|label| label == "ВыгрузитьКолонку"),
        "real fixture member completion must return ТаблицаЗначений method children for ТаблЗнач, labels={completion_labels:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_member_access_owner_miss_with_ready_head_reports_owner_unresolved() {
    const OWNER_UNRESOLVED_REASON_KEY: &str = "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_completion_reason_owner_unresolved";
    const MISSING_INDEX_REASON_KEY: &str = "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_completion_reason_missing_semantic_index";

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура Тест()\n\
    НеизвестныйЛокал.\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_owner_unresolved_member_access.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.to_string(),
        },
    };
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didOpen")
                .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be captured");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let completion_position = find_utf16_position_after_marker(fixture, "НеизвестныйЛокал.");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        completion_labels.is_empty(),
        "unresolved owner must fail closed without synthetic member children, labels={completion_labels:?}"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 12004, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces.last().expect("owner-unresolved completion trace");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "outcome"),
        Some("owner_unresolved"),
        "owner-hint miss with ready head must not be reported as wait_not_ready, trace={trace:?}"
    );
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
        Some("owner_unresolved"),
        "owner-hint miss with ready head must expose bounded owner_unresolved cause, trace={trace:?}"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(counters.get(OWNER_UNRESOLVED_REASON_KEY)) > 0,
        "owner-unresolved completion must emit a distinct public fail-closed reason, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get(MISSING_INDEX_REASON_KEY)),
        0,
        "owner-unresolved completion must not be counted as missing_semantic_index"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_waiting_completion_promotes_matching_type_index_precompute_to_interactive() {
    const FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let (background_started_tx, background_started_rx) = tokio::sync::oneshot::channel::<()>();
    let background_blocker = tokio::spawn(async move {
        bsl_runtime::application::spawn_bounded_blocking_with_class(
            bsl_runtime::application::CpuWorkClass::Background,
            move || {
                let _ = background_started_tx.send(());
                std::thread::sleep(Duration::from_millis(400));
            },
        )
        .await
        .expect("background blocker join");
    });
    background_started_rx
        .await
        .expect("background blocker should start");

    let uri = Url::parse("file:///test_p7_waiter_promotes_exact_precompute.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
        },
    };
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didOpen")
                .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    server.sync_v2_globals().await;

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
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
    server.sync_v2_globals().await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "ДляCompletion = S.");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let promotion_total = read_u64_metric(
        counters.get("intellisense_v2_completion_exact_type_index_wait_promotion_total"),
    );
    let join_total = read_u64_metric(
        counters.get("intellisense_v2_completion_exact_type_index_wait_join_total"),
    );
    assert!(
        completion_labels.iter().any(|label| label == "Количество"),
        "waiting completion should recover typed-structure members once matching exact precompute is promoted or joined, labels={completion_labels:?}, counters={counters:?}"
    );
    assert!(
        promotion_total + join_total > 0,
        "waiting completion must observe waiter-aware exact precompute orchestration (join or promotion), counters={counters:?}"
    );
    assert!(
        read_u64_metric(
            counters.get("intellisense_v2_completion_exact_type_index_wait_ready_after_wait_total")
        ) > 0,
        "waiting completion must report ready_after_wait after joining or promoting the matching precompute, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"
            )
        ),
        0,
        "promotion path must avoid exact wait deadline miss in this scenario, counters={counters:?}"
    );

    background_blocker.await.expect("background blocker task");
    drain_task.abort();
}

#[tokio::test]
async fn p7_promote_type_index_precompute_replaces_matching_background_waiter() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (_service, socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move {
        let mut socket = socket;
        while let Some(_req) = socket.next().await {}
    });

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");

    let file_id = bsl_analysis_v2::FileId(4242);
    let requested_version = 7;
    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, requested_version);

    let previous_task_id = 999_u64;
    let previous_handle = tokio::spawn(std::future::pending::<()>());
    let previous_phase = Arc::new(std::sync::atomic::AtomicU8::new(
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::WaitingCpuPermit
            .as_u8(),
    ));
    {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        tasks.insert(
            file_id,
            crate::server::TypeIndexPrecomputeTaskV2 {
                task_id: previous_task_id,
                supersession_key: crate::server::TypeIndexPrecomputeSupersessionKeyV2 {
                    file_id,
                    requested_version,
                },
                work_class: bsl_runtime::application::CpuWorkClass::Background,
                phase: Arc::clone(&previous_phase),
                active_requested_version: Arc::new(std::sync::atomic::AtomicI32::new(0)),
                scheduled_at: Instant::now(),
                handle: previous_handle,
            },
        );
    }

    let action = server
        .promote_type_index_precompute_for_waiter_v2(file_id, Some(requested_version))
        .await;
    assert_eq!(
        action,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputeWaiterActionV2::Promoted,
        "matching background task in pre-compute wait phase must be promoted for interactive waiter"
    );

    let promoted_task = {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        let task = tasks
            .remove(&file_id)
            .expect("promoted task must remain registered");
        task.handle.abort();
        task
    };
    assert_eq!(
        promoted_task.supersession_key.requested_version, requested_version,
        "promotion must preserve requested version for the matching waiter"
    );
    assert_eq!(
        promoted_task.work_class,
        bsl_runtime::application::CpuWorkClass::Interactive,
        "promotion must respawn the task in interactive work class"
    );
    assert_ne!(
        promoted_task.task_id, previous_task_id,
        "promotion must replace the previous task instance instead of mutating it in place"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_promote_type_index_precompute_restarts_matching_background_compute_for_waiter() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (_service, socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move {
        let mut socket = socket;
        while let Some(_req) = socket.next().await {}
    });

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");

    let file_id = bsl_analysis_v2::FileId(4243);
    let requested_version = 7;
    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, requested_version);

    let previous_task_id = 1000_u64;
    let previous_handle = tokio::spawn(std::future::pending::<()>());
    let previous_phase = Arc::new(std::sync::atomic::AtomicU8::new(
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing.as_u8(),
    ));
    {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        tasks.insert(
            file_id,
            crate::server::TypeIndexPrecomputeTaskV2 {
                task_id: previous_task_id,
                supersession_key: crate::server::TypeIndexPrecomputeSupersessionKeyV2 {
                    file_id,
                    requested_version,
                },
                work_class: bsl_runtime::application::CpuWorkClass::Background,
                phase: Arc::clone(&previous_phase),
                active_requested_version: Arc::new(std::sync::atomic::AtomicI32::new(
                    requested_version,
                )),
                scheduled_at: Instant::now(),
                handle: previous_handle,
            },
        );
    }

    let action = server
        .promote_type_index_precompute_for_waiter_v2(file_id, Some(requested_version))
        .await;
    assert_eq!(
        action,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputeWaiterActionV2::Promoted,
        "matching background task already in compute phase must still be restarted for an interactive waiter"
    );

    let promoted_task = {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        let task = tasks
            .remove(&file_id)
            .expect("promoted task must remain registered");
        task.handle.abort();
        task
    };
    assert_eq!(
        promoted_task.supersession_key.requested_version, requested_version,
        "restart must preserve the requested version for the matching waiter"
    );
    assert_eq!(
        promoted_task.work_class,
        bsl_runtime::application::CpuWorkClass::Interactive,
        "restart must respawn the matching compute task in interactive work class"
    );
    assert_ne!(
        promoted_task.task_id, previous_task_id,
        "restart must replace the previous compute task instance instead of keeping the background worker"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_promote_type_index_precompute_restarts_completed_retained_task_without_exact_ready() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (_service, socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move {
        let mut socket = socket;
        while let Some(_req) = socket.next().await {}
    });

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");

    let file_id = bsl_analysis_v2::FileId(4244);
    let requested_version = 7;
    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, requested_version);
    server.analysis_v2.apply_changes_interactive(
        bsl_runtime::application::ObservabilityOrigin::Lsp,
        vec![bsl_analysis_v2::Change::SetFile {
            file_id,
            text: Arc::from("Процедура Тест()\nКонецПроцедуры\n"),
            version: requested_version,
            path: Arc::from("file:///test_p7_completed_retained_without_exact_ready.bsl"),
        }],
    );

    let previous_task_id = 1001_u64;
    let previous_handle = tokio::spawn(async {});
    tokio::task::yield_now().await;
    assert!(
        previous_handle.is_finished(),
        "test setup must create a retained completed precompute handle"
    );
    let previous_phase = Arc::new(std::sync::atomic::AtomicU8::new(
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Completed.as_u8(),
    ));
    {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        tasks.insert(
            file_id,
            crate::server::TypeIndexPrecomputeTaskV2 {
                task_id: previous_task_id,
                supersession_key: crate::server::TypeIndexPrecomputeSupersessionKeyV2 {
                    file_id,
                    requested_version,
                },
                work_class: bsl_runtime::application::CpuWorkClass::Background,
                phase: Arc::clone(&previous_phase),
                active_requested_version: Arc::new(std::sync::atomic::AtomicI32::new(0)),
                scheduled_at: Instant::now(),
                handle: previous_handle,
            },
        );
    }
    assert!(
        !server
            .analysis_v2
            .snapshot()
            .await
            .current_type_index_serve_only_ready(file_id)
            .expect("current_type_index_serve_only_ready before retained-task restart"),
        "test setup must keep exact readiness cold while the retained completed task is present"
    );

    let action = server
        .promote_type_index_precompute_for_waiter_v2(file_id, Some(requested_version))
        .await;
    assert_eq!(
        action,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputeWaiterActionV2::Promoted,
        "completed retained same-version task without exact readiness must be restarted for interactive waiter"
    );

    let promoted_task = {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        let task = tasks
            .remove(&file_id)
            .expect("restarted task must remain registered");
        task.handle.abort();
        task
    };
    assert_eq!(
        promoted_task.supersession_key.requested_version, requested_version,
        "restart must preserve the requested version for the waiter"
    );
    assert_eq!(
        promoted_task.work_class,
        bsl_runtime::application::CpuWorkClass::Interactive,
        "completed retained task must be respawned in interactive work class"
    );
    assert_ne!(
        promoted_task.task_id, previous_task_id,
        "restart must replace the retained completed task instead of reusing it"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_promote_type_index_precompute_restarts_completed_phase_task_without_exact_ready() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (_service, socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move {
        let mut socket = socket;
        while let Some(_req) = socket.next().await {}
    });

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");

    let file_id = bsl_analysis_v2::FileId(4245);
    let requested_version = 7;
    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, requested_version);
    server.analysis_v2.apply_changes_interactive(
        bsl_runtime::application::ObservabilityOrigin::Lsp,
        vec![bsl_analysis_v2::Change::SetFile {
            file_id,
            text: Arc::from("Процедура Тест()\nКонецПроцедуры\n"),
            version: requested_version,
            path: Arc::from("file:///test_p7_completed_phase_without_exact_ready.bsl"),
        }],
    );

    let previous_task_id = 1002_u64;
    let previous_handle = tokio::spawn(std::future::pending::<()>());
    let previous_phase = Arc::new(std::sync::atomic::AtomicU8::new(
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Completed.as_u8(),
    ));
    {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        tasks.insert(
            file_id,
            crate::server::TypeIndexPrecomputeTaskV2 {
                task_id: previous_task_id,
                supersession_key: crate::server::TypeIndexPrecomputeSupersessionKeyV2 {
                    file_id,
                    requested_version,
                },
                work_class: bsl_runtime::application::CpuWorkClass::Background,
                phase: Arc::clone(&previous_phase),
                active_requested_version: Arc::new(std::sync::atomic::AtomicI32::new(0)),
                scheduled_at: Instant::now(),
                handle: previous_handle,
            },
        );
    }
    assert!(
        !server
            .analysis_v2
            .snapshot()
            .await
            .current_type_index_serve_only_ready(file_id)
            .expect("current_type_index_serve_only_ready before completed-phase restart"),
        "test setup must keep exact readiness cold while the completed-phase task is still attached"
    );

    let action = server
        .promote_type_index_precompute_for_waiter_v2(file_id, Some(requested_version))
        .await;
    assert_eq!(
        action,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputeWaiterActionV2::Promoted,
        "completed-phase same-version task without exact readiness must be restarted for interactive waiter"
    );

    let promoted_task = {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        let task = tasks
            .remove(&file_id)
            .expect("restarted task must remain registered");
        task.handle.abort();
        task
    };
    assert_eq!(
        promoted_task.supersession_key.requested_version, requested_version,
        "restart must preserve the requested version for the waiter"
    );
    assert_eq!(
        promoted_task.work_class,
        bsl_runtime::application::CpuWorkClass::Interactive,
        "completed-phase task must be respawned in interactive work class"
    );
    assert_ne!(
        promoted_task.task_id, previous_task_id,
        "restart must replace the completed-phase task instead of joining it"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_hover_and_definition_do_not_backfill_from_runtime_index_snapshot() {
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

    const HOVER_REASON_KEY: &str = "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_hover_reason_missing_semantic_index";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = wait_budget_ms.saturating_add(500).max(400);
    let _precompute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS",
        &precompute_delay_ms.to_string(),
    );

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

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура МойМетод() Экспорт\n\
КонецПроцедуры\n\
\n\
Процедура Тест()\n\
    ЭтотОбъект.МойМетод();\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_hover_definition_no_search_rescue.bsl").expect("uri");

    coordinator
        .intellisense_index()
        .replace_snapshot(make_index_snapshot(
            "p7_hover_definition_no_search_rescue",
            "SearchOnlyType",
        ));
    coordinator.intellisense_index().replace_symbols_for_uri(
        uri.as_str(),
        vec![IndexItem::new(
            "SearchOnlyMethod".to_string(),
            IndexItemKind::Symbol(bsl_backend::system::SymbolKind::Procedure),
            IndexKind::Symbol,
        )],
    );
    coordinator.intellisense_index().replace_modules_for_key(
        "p7_hover_definition_no_search_rescue_module",
        vec![IndexItem::new(
            "SearchOnlyModule".to_string(),
            IndexItemKind::Symbol(bsl_backend::system::SymbolKind::Procedure),
            IndexKind::Module,
        )],
    );

    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be captured");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 1).await;

    let member_position = find_utf16_position_after_marker(fixture, "ЭтотОбъект.МойМетод");

    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/hover")
                .id(12012)
                .params(
                    serde_json::to_value(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: member_position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                    })
                    .expect("HoverParams"),
                )
                .finish(),
        )
        .await
        .expect("hover request")
        .expect("hover response");
    let hover_value = serde_json::to_value(&hover_response).expect("serialize hover response");
    let hover_result = hover_value
        .get("result")
        .cloned()
        .expect("hover result field");
    let hover: Option<Hover> = serde_json::from_value(hover_result).expect("parse hover result");
    assert!(
        hover.is_none(),
        "hover must stay fail-closed when only runtime discovery/search index is populated: {hover_value:?}"
    );

    let definition_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/definition")
                .id(12013)
                .params(
                    serde_json::to_value(GotoDefinitionParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: member_position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    })
                    .expect("GotoDefinitionParams"),
                )
                .finish(),
        )
        .await
        .expect("definition request")
        .expect("definition response");
    let definition_value =
        serde_json::to_value(&definition_response).expect("serialize definition response");
    let definition_result = definition_value
        .get("result")
        .cloned()
        .expect("definition result field");
    let definition: Option<GotoDefinitionResponse> =
        serde_json::from_value(definition_result).expect("parse definition result");
    assert!(
        normalize_lsp_definition(definition).is_empty(),
        "definition must stay fail-closed when only runtime discovery/search index is populated: {definition_value:?}"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(counters.get(HOVER_REASON_KEY)) > 0,
        "hover cache miss must emit missing_semantic_index bounded public reason metrics, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_large_churn_budget_timeout_without_prior_completion_returns_fail_closed_empty_response()
{
    const FILLER_LINES: usize = 2200;
    let mut fixture = String::new();
    fixture.push_str("Процедура Тест()\n");
    fixture.push_str("    ЛокМассив = Новый Массив;\n");
    for idx in 0..FILLER_LINES {
        fixture.push_str(&format!("    // filler {idx}\n"));
    }
    fixture.push_str("    ЛокМассив.\n");
    fixture.push_str("КонецПроцедуры\n");

    let completion_line = (2 + FILLER_LINES) as u32;
    let completion_character = "    ЛокМассив."
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///test_p7_large_churn_stale_cache_miss.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.clone(),
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

    for version in 2..=7_i32 {
        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: fixture.clone(),
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
    }

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    server
        .deps_update_v2("p7_large_churn_stale_cache_miss_setup", None, None)
        .await;
    server.sync_v2_globals().await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let large_churn_active = server
        .scale_aware_churn_state_v2
        .read()
        .await
        .get(&file_id)
        .is_some_and(|state| state.large_churn_active);
    assert!(
        large_churn_active,
        "expected large+churn state to be active after burst didChange on large document"
    );

    {
        let mut versions = server.latest_received_file_versions_v2.write().await;
        versions.insert(file_id, 8);
    }

    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let started = Instant::now();
    let stale_completion = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(completion_line, completion_character),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(".".to_string()),
            }),
        })
        .await
        .expect("stale completion request")
        .expect("stale completion response");
    let elapsed = started.elapsed();

    match stale_completion {
        CompletionResponse::List(list) => {
            assert!(
                list.items.is_empty(),
                "cache miss under large-churn timeout must fail closed with empty payload"
            );
        }
        CompletionResponse::Array(items) => {
            assert!(
                items.is_empty(),
                "cache miss under large-churn timeout must fail closed with empty payload"
            );
        }
    }
    assert!(
        elapsed <= Duration::from_millis(wait_budget_ms.saturating_add(300)),
        "fail-closed cache miss should remain bounded near wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    let stale_fallback_total = counters
        .get("intellisense_v2_completion_stale_fallback_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert!(
        stale_fallback_total == 0,
        "stale fallback counter must stay zero on fail-closed cache miss"
    );
    let fallback_unavailable_total = counters
        .get("intellisense_v2_completion_fallback_unavailable_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert!(
        fallback_unavailable_total > 0,
        "expected fallback_unavailable counter to increment for stale cache-miss"
    );
    let prepare_apply_age_start_count = histograms
        .get("completion_stage_prepare_apply_age_at_start_ms")
        .and_then(|value| value.as_object())
        .and_then(|histogram| histogram.get("count"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let prepare_apply_age_terminal_count = histograms
        .get("completion_stage_prepare_apply_age_at_terminal_ms")
        .and_then(|value| value.as_object())
        .and_then(|histogram| histogram.get("count"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert!(
        prepare_apply_age_start_count > 0 && prepare_apply_age_terminal_count > 0,
        "prepare fail-closed path must expose apply-age histograms, histograms={histograms:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_completion_prepare_timeout_stays_bounded_when_disk_fallback_blocks() {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let temp = tempfile::TempDir::new().expect("tempdir");
    let fifo_path = temp.path().join("blocking_prepare_fallback.bsl");
    let mkfifo_status = std::process::Command::new("mkfifo")
        .arg(&fifo_path)
        .status()
        .expect("mkfifo status");
    assert!(mkfifo_status.success(), "mkfifo must succeed");

    let writer_path = fifo_path.clone();
    let writer = std::thread::spawn(move || {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .mode(0o644)
            .open(&writer_path)
            .expect("open fifo writer");
        std::thread::sleep(Duration::from_millis(350));
        let fixture =
            "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
        file.write_all(fixture.as_bytes())
            .expect("write fifo contents");
    });

    let uri = Url::from_file_path(&fifo_path).expect("fifo uri");
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);

    let started = Instant::now();
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/completion")
                .id(17701)
                .params(
                    serde_json::to_value(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: Position::new(2, 13),
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
        .expect("completion request")
        .expect("completion response");
    let elapsed = started.elapsed();

    let completion_value =
        serde_json::to_value(&completion_response).expect("serialize completion response");
    let result = completion_value
        .get("result")
        .cloned()
        .expect("result field");
    let response: Option<CompletionResponse> =
        serde_json::from_value(result).expect("parse completion result");
    match response.expect("completion payload") {
        CompletionResponse::List(list) => {
            assert!(
                list.items.is_empty(),
                "blocking disk fallback must still fail closed under prepare timeout"
            );
        }
        CompletionResponse::Array(items) => {
            assert!(
                items.is_empty(),
                "blocking disk fallback must still fail closed under prepare timeout"
            );
        }
    }
    assert!(
        elapsed <= Duration::from_millis(wait_budget_ms.saturating_add(200)),
        "blocking disk fallback must stay bounded by prepare wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(
            counters
                .get("intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout")
        ) > 0,
        "blocking disk fallback must attribute fail-closed completion to prepare-timeout, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline")
        ),
        0,
        "blocking disk fallback must not attribute fail-closed completion to exact-deadline, counters={counters:?}"
    );

    writer.join().expect("writer thread");
    drain_task.abort();
}

#[tokio::test]
async fn p7_large_churn_did_change_skips_inline_parse_snapshot() {
    const FILLER_LINES: usize = 2200;
    const LATEST_VERSION: i32 = 7;
    let mut fixture = String::new();
    fixture.push_str("Процедура Тест()\n");
    fixture.push_str("    ЛокМассив = Новый Массив;\n");
    for idx in 0..FILLER_LINES {
        fixture.push_str(&format!("    // filler {idx}\n"));
    }
    fixture.push_str("    ЛокМассив.\n");
    fixture.push_str("КонецПроцедуры\n");

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///test_p7_large_churn_skip_parse_snapshot.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.clone(),
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

    for version in 2..=LATEST_VERSION {
        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: fixture.clone(),
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
    }

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    server.sync_v2_globals().await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let large_churn_active = server
        .scale_aware_churn_state_v2
        .read()
        .await
        .get(&file_id)
        .is_some_and(|state| state.large_churn_active);
    assert!(
        large_churn_active,
        "expected large+churn state to be active after burst didChange on large document"
    );
    assert!(
        server
            .analysis_v2
            .wait_for_file_version(file_id, LATEST_VERSION)
            .await,
        "analysis runtime must catch up to the latest didChange version"
    );

    let analysis = server.analysis_v2.snapshot().await;
    let parse_mode = analysis
        .syntax_diagnostics_observability_mode(file_id)
        .expect("syntax diagnostics mode query")
        .expect("syntax diagnostics mode for active file");
    assert_eq!(
        parse_mode, "other",
        "large+churn didChange must skip inline parse snapshot and leave syntax diagnostics on fallback mode"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(counters.get("intellisense_v2_parse_snapshot_total_origin_lsp_mode_other"))
            > 0,
        "skipped large+churn parse snapshot must be visible via mode=other counters"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_hover_cache_miss_emits_bounded_fail_closed_reason() {
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

    const FALLBACK_REASON_KEY: &str = "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_hover_reason_missing_semantic_index";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = wait_budget_ms.saturating_add(500).max(400);
    let _precompute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS",
        &precompute_delay_ms.to_string(),
    );

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

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура Тест()\n\
МассивДляHover = Новый Массив;\n\
ЗначДляHover = МассивДляHover.Количество();\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_hover_type_index_fallback_unavailable.bsl").expect("uri");
    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be captured");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 1).await;

    let fallback_reason_total = |coordinator: &Arc<SystemCoordinator>| -> u64 {
        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        read_u64_metric(counters.get(FALLBACK_REASON_KEY))
    };

    let before_hover = fallback_reason_total(&coordinator);
    let hover_position = find_utf16_position_after_marker(fixture, "ЗначДляHover = МассивДляHover");
    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/hover")
                .id(9107)
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
        "hover should return response envelope"
    );
    let hover_value = serde_json::to_value(&hover_response).expect("serialize response");
    let hover_result = hover_value.get("result").cloned().expect("result field");
    let hover: Option<Hover> = serde_json::from_value(hover_result).expect("parse hover");
    assert!(
        hover.is_none(),
        "hover cache miss must stay fail-closed and return empty semantic payload: {hover_value:?}"
    );
    let after_hover = fallback_reason_total(&coordinator);
    assert!(
        after_hover > before_hover,
        "hover cache miss must emit bounded public fail-closed reason"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_hover_bootstraps_exact_type_index_without_did_save_when_precompute_fits_budget() {
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

    const FALLBACK_REASON_KEY: &str = "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_hover_reason_missing_semantic_index";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = (wait_budget_ms / 4).max(20);
    let _precompute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS",
        &precompute_delay_ms.to_string(),
    );

    let fixture = "Процедура Тест()\n\
    S = Новый Структура;\n\
    S.Вставить(\"Идентификатор\", \"A-01\");\n\
    ДляHover = S.Идентификатор;\n\
КонецПроцедуры\n";
    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture,
        "file:///test_p7_hover_bootstraps_exact_without_did_save.bsl",
    )
    .await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let before_metrics = server.coordinator.observability_metrics();
    let before_counters = before_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let before_fail_closed = read_u64_metric(before_counters.get(FALLBACK_REASON_KEY));

    let hover_position = find_utf16_position_at_marker_tail(fixture, "ДляHover = S.Идентификатор");
    let started = Instant::now();
    let hover_text = lsp_hover_text_optional_at(&mut service, &uri, hover_position).await;
    let elapsed = started.elapsed();

    let hover_text = hover_text.expect("hover should bootstrap exact type index without didSave");
    assert!(
        hover_text.contains("Идентификатор") && hover_text.contains("Строка"),
        "hover must expose typed-structure field info after same-version bootstrap, hover={hover_text}"
    );
    assert!(
        elapsed <= std::time::Duration::from_millis(wait_budget_ms.saturating_add(250).max(250)),
        "hover bootstrap should stay bounded by the interactive wait budget, elapsed={elapsed:?}, wait_budget_ms={wait_budget_ms}"
    );

    let after_metrics = server.coordinator.observability_metrics();
    let after_counters = after_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let after_fail_closed = read_u64_metric(after_counters.get(FALLBACK_REASON_KEY));
    assert_eq!(
        after_fail_closed, before_fail_closed,
        "successful hover bootstrap must not emit missing_semantic_index fail-closed attribution"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_hover_timeout_still_seeds_exact_type_index_without_did_save() {
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

    const FALLBACK_REASON_KEY: &str = "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_hover_reason_missing_semantic_index";
    const WAIT_BUDGET_EXHAUSTED_KEY: &str =
        "intellisense_v2_interactive_wait_budget_exhausted_total";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = wait_budget_ms.saturating_add(500).max(400);
    let _precompute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS",
        &precompute_delay_ms.to_string(),
    );

    let fixture = "Процедура Тест()\n\
    S = Новый Структура;\n\
    S.Вставить(\"Идентификатор\", \"A-01\");\n\
    ДляHover = S.Идентификатор;\n\
КонецПроцедуры\n";
    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture,
        "file:///test_p7_hover_timeout_still_seeds_exact_without_did_save.bsl",
    )
    .await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let before_metrics = server.coordinator.observability_metrics();
    let before_counters = before_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let before_fail_closed = read_u64_metric(before_counters.get(FALLBACK_REASON_KEY));
    let before_wait_budget_exhausted =
        read_u64_metric(before_counters.get(WAIT_BUDGET_EXHAUSTED_KEY));

    let hover_position = find_utf16_position_at_marker_tail(fixture, "ДляHover = S.Идентификатор");
    let started = Instant::now();
    let first_hover_text = lsp_hover_text_optional_at(&mut service, &uri, hover_position).await;
    let first_elapsed = started.elapsed();
    assert!(
        first_hover_text.is_none(),
        "hover must remain fail-closed on the first request when same-version exact precompute exceeds the interactive budget"
    );
    assert!(
        first_elapsed
            <= std::time::Duration::from_millis(wait_budget_ms.saturating_add(250).max(250)),
        "hover timeout must stay bounded by the interactive wait budget, elapsed={first_elapsed:?}, wait_budget_ms={wait_budget_ms}"
    );

    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;
    wait_for_type_index_precompute_completion(&server, file_id).await;

    let second_hover_text = lsp_hover_text_optional_at(&mut service, &uri, hover_position).await;
    let second_hover_text = second_hover_text.expect(
        "hover should succeed after same-version exact precompute finishes without didSave",
    );
    assert!(
        second_hover_text.contains("Идентификатор") && second_hover_text.contains("Строка"),
        "hover must expose typed-structure field info once same-version exact precompute finishes, hover={second_hover_text}"
    );

    let after_metrics = server.coordinator.observability_metrics();
    let after_counters = after_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let after_fail_closed = read_u64_metric(after_counters.get(FALLBACK_REASON_KEY));
    let after_wait_budget_exhausted =
        read_u64_metric(after_counters.get(WAIT_BUDGET_EXHAUSTED_KEY));
    assert!(
        after_fail_closed > before_fail_closed,
        "timed-out hover must still expose bounded missing_semantic_index attribution"
    );
    assert!(
        after_wait_budget_exhausted > before_wait_budget_exhausted,
        "timed-out hover bootstrap must attribute the bounded wait budget exhaustion"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_diagnostics_only_query_keeps_exact_isolation_before_hover_and_definition_recovery() {
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

    const HOVER_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_hover_reason_missing_semantic_index";
    const DEFINITION_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_definition_reason_missing_semantic_index";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = (wait_budget_ms / 4).max(20);
    let _precompute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS",
        &precompute_delay_ms.to_string(),
    );

    let fixture = concat!(
        "Процедура Целевой()\n",
        "КонецПроцедуры\n",
        "\n",
        "Процедура Тест()\n",
        "    S = Новый Структура;\n",
        "    S.Вставить(\"Идентификатор\", \"A-01\");\n",
        "    ДляHover = S.Идентификатор;\n",
        "    Целевой();\n",
        "КонецПроцедуры\n"
    );
    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture,
        "file:///test_p7_diagnostics_only_exact_recovery_after_runtime_query.bsl",
    )
    .await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let before_metrics = server.coordinator.observability_metrics();
    let before_counters = before_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let before_hover_fail_closed = read_u64_metric(before_counters.get(HOVER_REASON_KEY));
    let before_definition_fail_closed = read_u64_metric(before_counters.get(DEFINITION_REASON_KEY));

    let exact_ready_before_diagnostics = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready before diagnostics-only query");
    assert!(
        !exact_ready_before_diagnostics,
        "test setup must start with current-revision exact type index unpublished"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics.is_empty(),
        "diagnostics-only query should stay semantically clean for the exact-recovery fixture, diagnostics={diagnostics:?}"
    );

    let exact_ready_after_diagnostics = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready after diagnostics-only query");
    assert!(
        !exact_ready_after_diagnostics,
        "diagnostics-only query must not publish the exact type index before later exact consumers recover"
    );

    let hover_position = find_utf16_position_at_marker_tail(fixture, "ДляHover = S.Идентификатор");
    let hover_text = lsp_hover_text_optional_at(&mut service, &uri, hover_position)
        .await
        .expect("hover should recover canonical exact semantics after diagnostics-only query");
    assert!(
        hover_text.contains("Идентификатор") && hover_text.contains("Строка"),
        "hover must recover the typed structure field after diagnostics-only query, hover={hover_text}"
    );

    let exact_ready_after_hover = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready after hover recovery");
    assert!(
        exact_ready_after_hover,
        "later hover recovery must publish the current exact type index instead of treating diagnostics-only artifacts as exact truth"
    );

    let definition_position =
        find_utf16_position_after_marker(fixture, "ДляHover = S.Идентификатор;\n    Целевой");
    let definition_points = lsp_definition_points_at(&mut service, &uri, definition_position).await;
    let direct_definition_points =
        snapshot_definition_points_at(&server, file_id, &uri, definition_position).await;
    assert!(
        !definition_points.is_empty(),
        "definition must recover canonical exact target after diagnostics-only query, points={definition_points:?}, direct_points={direct_definition_points:?}"
    );

    let after_metrics = server.coordinator.observability_metrics();
    let after_counters = after_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let after_hover_fail_closed = read_u64_metric(after_counters.get(HOVER_REASON_KEY));
    let after_definition_fail_closed = read_u64_metric(after_counters.get(DEFINITION_REASON_KEY));
    assert_eq!(
        after_hover_fail_closed, before_hover_fail_closed,
        "successful hover recovery after diagnostics-only query must not emit missing_semantic_index fail-closed attribution"
    );
    assert_eq!(
        after_definition_fail_closed, before_definition_fail_closed,
        "successful definition recovery after diagnostics-only query must not emit missing_semantic_index fail-closed attribution"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_definition_bootstraps_exact_type_index_without_did_save_when_precompute_fits_budget() {
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

    const FALLBACK_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_definition_reason_missing_semantic_index";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = (wait_budget_ms / 4).max(20);
    let _precompute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS",
        &precompute_delay_ms.to_string(),
    );

    let fixture = "Процедура Целевой()\n\
КонецПроцедуры\n\
\n\
Процедура Тест()\n\
    Целевой();\n\
КонецПроцедуры\n";
    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture,
        "file:///test_p7_definition_bootstraps_exact_without_did_save.bsl",
    )
    .await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let before_metrics = server.coordinator.observability_metrics();
    let before_counters = before_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let before_fail_closed = read_u64_metric(before_counters.get(FALLBACK_REASON_KEY));

    let definition_position =
        find_utf16_position_after_marker(fixture, "Процедура Тест()\nЦелевой");
    let started = Instant::now();
    let definition_points = lsp_definition_points_at(&mut service, &uri, definition_position).await;
    let elapsed = started.elapsed();
    let direct_definition_points =
        snapshot_definition_points_at(&server, file_id, &uri, definition_position).await;
    let exact_ready = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready after definition bootstrap");

    assert!(
        !definition_points.is_empty(),
        "definition should bootstrap exact type index without didSave once same-version precompute fits the wait budget, points={definition_points:?}, direct_points={direct_definition_points:?}, exact_ready={exact_ready}"
    );
    assert!(
        elapsed <= std::time::Duration::from_millis(wait_budget_ms.saturating_add(250).max(250)),
        "definition bootstrap should stay bounded by the interactive wait budget, elapsed={elapsed:?}, wait_budget_ms={wait_budget_ms}"
    );

    let after_metrics = server.coordinator.observability_metrics();
    let after_counters = after_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let after_fail_closed = read_u64_metric(after_counters.get(FALLBACK_REASON_KEY));
    assert_eq!(
        after_fail_closed, before_fail_closed,
        "successful definition bootstrap must not emit missing_semantic_index fail-closed attribution"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_definition_timeout_still_seeds_exact_type_index_without_did_save() {
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

    const FALLBACK_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_definition_reason_missing_semantic_index";
    const WAIT_BUDGET_EXHAUSTED_KEY: &str =
        "intellisense_v2_interactive_wait_budget_exhausted_total";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = wait_budget_ms.saturating_add(500).max(400);
    let _precompute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS",
        &precompute_delay_ms.to_string(),
    );

    let fixture = "Процедура Целевой()\n\
КонецПроцедуры\n\
\n\
Процедура Тест()\n\
    Целевой();\n\
КонецПроцедуры\n";
    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture,
        "file:///test_p7_definition_timeout_still_seeds_exact_without_did_save.bsl",
    )
    .await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let before_metrics = server.coordinator.observability_metrics();
    let before_counters = before_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let before_fail_closed = read_u64_metric(before_counters.get(FALLBACK_REASON_KEY));
    let before_wait_budget_exhausted =
        read_u64_metric(before_counters.get(WAIT_BUDGET_EXHAUSTED_KEY));

    let definition_position =
        find_utf16_position_after_marker(fixture, "Процедура Тест()\nЦелевой");
    let started = Instant::now();
    let first_definition_points =
        lsp_definition_points_at(&mut service, &uri, definition_position).await;
    let first_elapsed = started.elapsed();
    assert!(
        first_definition_points.is_empty(),
        "definition must remain fail-closed on the first request when same-version exact precompute exceeds the interactive budget, points={first_definition_points:?}"
    );
    assert!(
        first_elapsed
            <= std::time::Duration::from_millis(wait_budget_ms.saturating_add(250).max(250)),
        "definition timeout must stay bounded by the interactive wait budget, elapsed={first_elapsed:?}, wait_budget_ms={wait_budget_ms}"
    );

    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;
    wait_for_type_index_precompute_completion(&server, file_id).await;

    let second_definition_points =
        lsp_definition_points_at(&mut service, &uri, definition_position).await;
    let second_direct_definition_points =
        snapshot_definition_points_at(&server, file_id, &uri, definition_position).await;
    let exact_ready_after_wait = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready after timeout definition wait");
    assert!(
        !second_definition_points.is_empty(),
        "definition should succeed after same-version exact precompute finishes without didSave, points={second_definition_points:?}, direct_points={second_direct_definition_points:?}, exact_ready={exact_ready_after_wait}"
    );

    let after_metrics = server.coordinator.observability_metrics();
    let after_counters = after_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let after_fail_closed = read_u64_metric(after_counters.get(FALLBACK_REASON_KEY));
    let after_wait_budget_exhausted =
        read_u64_metric(after_counters.get(WAIT_BUDGET_EXHAUSTED_KEY));
    assert!(
        after_fail_closed > before_fail_closed,
        "timed-out definition must still expose bounded missing_semantic_index attribution"
    );
    assert!(
        after_wait_budget_exhausted > before_wait_budget_exhausted,
        "timed-out definition bootstrap must attribute the bounded wait budget exhaustion"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_signature_help_bootstraps_exact_type_index_without_did_save_when_precompute_fits_budget(
) {
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

    const FALLBACK_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_signature_help_reason_missing_semantic_index";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = (wait_budget_ms / 4).max(20);
    let _precompute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS",
        &precompute_delay_ms.to_string(),
    );

    let fixture = concat!(
        "Процедура Тест()\n",
        "    МойМассив = Новый Массив();\n",
        "    МойМассив.Добавить(1, 2);\n",
        "КонецПроцедуры\n"
    );
    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture,
        "file:///test_p7_signature_help_bootstraps_exact_without_did_save.bsl",
    )
    .await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let before_metrics = server.coordinator.observability_metrics();
    let before_counters = before_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let before_fail_closed = read_u64_metric(before_counters.get(FALLBACK_REASON_KEY));

    let signature_position = find_utf16_position_after_marker(fixture, "МойМассив.Добавить(1, ");
    let started = Instant::now();
    let signature_help = lsp_signature_help_at(&mut service, &uri, signature_position)
        .await
        .expect("signatureHelp should bootstrap exact type index without didSave");
    let elapsed = started.elapsed();
    let signature_label = signature_help
        .signatures
        .first()
        .map(|signature| signature.label.as_str())
        .unwrap_or("");
    assert!(
        signature_label.contains("Добавить("),
        "signatureHelp must expose exact method signature after same-version bootstrap, label={signature_label}"
    );
    assert_eq!(signature_help.active_parameter, Some(1));
    assert!(
        elapsed <= std::time::Duration::from_millis(wait_budget_ms.saturating_add(250).max(250)),
        "signatureHelp bootstrap should stay bounded by the interactive wait budget, elapsed={elapsed:?}, wait_budget_ms={wait_budget_ms}"
    );

    let after_metrics = server.coordinator.observability_metrics();
    let after_counters = after_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let after_fail_closed = read_u64_metric(after_counters.get(FALLBACK_REASON_KEY));
    assert_eq!(
        after_fail_closed, before_fail_closed,
        "successful signatureHelp bootstrap must not emit missing_semantic_index fail-closed attribution"
    );

    drain_task.abort();
}
