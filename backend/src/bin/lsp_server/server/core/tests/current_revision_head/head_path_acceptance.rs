#[tokio::test]
async fn p33_completion_uses_current_revision_head_path_without_exact_artifact() {
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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = "Процедура Тест()\n    Результат = (Новый Массив()).\nКонецПроцедуры\n";
    let uri =
        Url::parse("file:///test_p33_completion_head_without_exact_artifact.bsl").expect("uri");
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

    server.sync_v2_globals().await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let completion_position = find_utf16_position_after_marker(fixture, "(Новый Массив()).");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        !completion_labels.is_empty(),
        "member-access completion should use current-revision head path even when exact artifact is missing, labels={completion_labels:?}"
    );
    assert!(
        completion_labels.iter().any(|label| label == "Количество"),
        "head-path completion should surface canonical members for current-revision explicit receiver, labels={completion_labels:?}"
    );
    let timeline = lsp_get_completion_timeline(&mut service, 401, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces.last().expect("head-path completion timeline trace");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "head-path completion trace must expose bounded route in prepare_details, trace={trace:?}"
    );
    assert!(
        trace
            .get("prepare_details")
            .and_then(|value| value.as_object())
            .is_some_and(|details| details.contains_key("fail_closed_cause")),
        "head-path completion trace must keep fail_closed_cause field present even when route succeeds, trace={trace:?}"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert_eq!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"
            )
        ),
        0,
        "head-path completion should not rely on exact wait deadline, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get("intellisense_v2_completion_fallback_unavailable_total")),
        0,
        "head-path completion should not record fallback_unavailable for explicit current-revision receiver, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_head_hit")) > 0,
        "head-path completion must record head-hit route, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_exact_hit")),
        0,
        "head-path completion must not record exact-hit route, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_form_module_object_completion_uses_current_revision_head_path_without_exact_artifact()
{
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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = "Процедура Тест()\n    ДляCompletion = Объект.\nКонецПроцедуры\n";
    let uri = Url::parse("file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl")
        .expect("form module uri");
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

    server.sync_v2_globals().await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let completion_position = find_utf16_position_after_marker(fixture, "ДляCompletion = Объект.");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        completion_labels.iter().any(|label| label == "Ссылка"),
        "head-path completion for FormModule.Объект must include form-data property Ссылка, labels={completion_labels:?}"
    );
    assert!(
        completion_labels
            .iter()
            .any(|label| label == "ПометкаУдаления"),
        "head-path completion for FormModule.Объект must include form-data property ПометкаУдаления, labels={completion_labels:?}"
    );
    assert!(
        !completion_labels
            .iter()
            .any(|label| label == "ПолучитьСсылкуНового"),
        "head-path completion for FormModule.Объект must not leak object-facet method ПолучитьСсылкуНового, labels={completion_labels:?}"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 403, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("form-module head-path completion timeline trace");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "form-module head-path completion trace must expose bounded route in prepare_details, trace={trace:?}"
    );
    assert!(
        trace
            .get("prepare_details")
            .and_then(|value| value.as_object())
            .is_some_and(|details| details.contains_key("fail_closed_cause")),
        "form-module head-path completion trace must keep fail_closed_cause field present even when route succeeds, trace={trace:?}"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert_eq!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"
            )
        ),
        0,
        "form-module head-path completion should not rely on exact wait deadline, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get("intellisense_v2_completion_fallback_unavailable_total")),
        0,
        "form-module head-path completion should not record fallback_unavailable, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_head_hit")) > 0,
        "form-module head-path completion must record head-hit route, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_exact_hit")),
        0,
        "form-module head-path completion must not record exact-hit route, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_form_module_head_path_skips_ir_query_delay_when_owner_hints_are_ready() {
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

    let _env_lock = lock_test_env_blocking();
    let _ir_delay_guard = EnvVarGuard::set("BSL_TEST_COMPLETION_IR_QUERY_DELAY_MS", "400");

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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = "Процедура Тест()\n    ДляCompletion = Объект.\nКонецПроцедуры\n";
    let uri = Url::parse("file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl")
        .expect("form module uri");
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

    server.sync_v2_globals().await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let metrics_before = coordinator.observability_metrics();
    let counters_before = metrics_before
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics_before.counters object");

    let completion_position = find_utf16_position_after_marker(fixture, "ДляCompletion = Объект.");
    let started = Instant::now();
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    let elapsed = started.elapsed();
    assert!(
        completion_labels.iter().any(|label| label == "Ссылка"),
        "head-path completion for FormModule.Объект must remain non-empty even when IR query delay is injected, labels={completion_labels:?}"
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "head-path completion must stay bounded and skip IR delay (elapsed={elapsed:?})"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 404, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("form-module head-path completion timeline trace with ir delay");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "form-module head-path completion with injected IR delay must still expose head route, trace={trace:?}"
    );
    assert!(
        completion_timeline_query_bundle_total_ms(trace).unwrap_or(u64::MAX) < 250,
        "head-path query_bundle must not inherit injected IR delay, trace={trace:?}"
    );

    let metrics_after = coordinator.observability_metrics();
    let counters_after = metrics_after
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics_after.counters object");
    let ir_query_delta =
        read_u64_metric(counters_after.get("intellisense_v2_ir_query_completion_total"))
            .saturating_sub(read_u64_metric(
                counters_before.get("intellisense_v2_ir_query_completion_total"),
            ));
    assert_eq!(
        ir_query_delta, 0,
        "head-path completion must not execute completion IR query when owner hints are already ready, counters_before={counters_before:?}, counters_after={counters_after:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_non_member_form_completion_ages_out_of_shadow_empty_success_window() {
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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = "Процедура ПриСозданииНаСервере()\n    Этот\nКонецПроцедуры\n";
    let uri = Url::parse("file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl")
        .expect("form module uri");
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

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: fixture.to_string(),
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

    tokio::time::sleep(Duration::from_millis(
        wait_budget_ms.saturating_add(150).max(250),
    ))
    .await;

    let completion_position = find_utf16_position_after_marker(fixture, "    Этот");
    let completion_labels: Vec<String> = lsp_completion_items_with_request(
        &mut service,
        12_001,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await
    .into_iter()
    .map(|item| item.label)
    .collect();
    assert!(
        completion_labels.iter().any(|label| label == "ЭтотОбъект"),
        "aged non-member completion must leave shadow-only ok_empty path and return form-module candidates, labels={completion_labels:?}"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 40_433, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces.last().expect("non-member form completion trace");
    assert_eq!(
        trace.get("outcome").and_then(|value| value.as_str()),
        Some("ok_non_empty"),
        "aged non-member completion must not stay on synthetic ok_empty success path, trace={trace:?}"
    );
    assert_ne!(
        completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
        Some("exact_deadline"),
        "aged non-member completion must not regress into exact_deadline while current-revision exact precompute is still delayed, trace={trace:?}"
    );
    assert_eq!(
        completion_timeline_trace_stage_duration_ms(trace, "wait_exact_type_index"),
        None,
        "aged non-member completion must not block on wait_exact_type_index once bounded no-IR current-revision fallback is available, trace={trace:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_aged_non_member_completion_skips_blocking_current_revision_snapshot_reprobe() {
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

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = wait_budget_ms.saturating_add(500).max(400);
    let snapshot_delay_ms = precompute_delay_ms.saturating_add(300).max(800);
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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = "Процедура ПриСозданииНаСервере()\n    Этот\nКонецПроцедуры\n";
    let uri = Url::parse("file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl")
        .expect("form module uri");
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

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: fixture.to_string(),
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

    tokio::time::sleep(Duration::from_millis(
        wait_budget_ms.saturating_add(150).max(250),
    ))
    .await;

    let _snapshot_delay_guard = EnvVarGuard::set(
        "BSL_TEST_AGED_NON_MEMBER_EXACT_REPROBE_DELAY_MS",
        &snapshot_delay_ms.to_string(),
    );

    let completion_position = find_utf16_position_after_marker(fixture, "    Этот");
    let completion_started = Instant::now();
    let completion_labels: Vec<String> = lsp_completion_items_with_request(
        &mut service,
        12_101,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await
    .into_iter()
    .map(|item| item.label)
    .collect();
    let completion_elapsed_ms = completion_started.elapsed().as_millis() as u64;
    assert!(
        completion_labels.iter().any(|label| label == "ЭтотОбъект"),
        "aged non-member completion must keep returning current-revision no-IR candidates even when current-revision snapshot reacquisition is delayed, labels={completion_labels:?}"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 40_434, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("aged non-member completion trace with snapshot delay");
    let uncovered_gap_ms = completion_timeline_uncovered_gap_ms(trace).unwrap_or(u64::MAX);
    let max_stage_end_ms = completion_timeline_max_stage_end_ms(trace).unwrap_or(0);
    let total_duration_ms = trace
        .get("total_duration_ms")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert_eq!(
        trace.get("outcome").and_then(|value| value.as_str()),
        Some("ok_non_empty"),
        "aged non-member completion must stay successful under injected snapshot delay, trace={trace:?}"
    );
    assert_eq!(
        completion_timeline_trace_stage_duration_ms(trace, "wait_exact_type_index"),
        None,
        "aged non-member completion must not re-enter exact wait under injected snapshot delay, trace={trace:?}"
    );
    assert!(
        completion_elapsed_ms < snapshot_delay_ms / 2,
        "aged non-member completion must not inherit injected current-revision snapshot delay once post-window path is lightweight, elapsed={}ms, snapshot_delay={}ms, trace={trace:?}",
        completion_elapsed_ms,
        snapshot_delay_ms,
    );
    assert!(
        uncovered_gap_ms < snapshot_delay_ms / 2,
        "aged non-member completion must not leave snapshot-delay-sized uncovered handler gap, uncovered_gap={}ms, max_stage_end={}ms, total_duration_ms={}ms, trace={trace:?}",
        uncovered_gap_ms,
        max_stage_end_ms,
        total_duration_ms,
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_completion_service_first_poll_ignores_blocking_did_change_parse_delay() {
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

    const FIXTURE: &str = "Процедура Тест()\n    ДляCompletion = Объект.\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let _blocking_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "1500");
    let _did_save_parse_delay_guard = EnvVarGuard::set("BSL_TEST_DID_SAVE_PARSE_DELAY_MS", "1500");

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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let uri = Url::parse("file:///test_p33_completion_service_first_poll_blocking_parse.bsl")
        .expect("form module uri");
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
    let file_id = server.get_or_create_file_id_v2(&uri).await;
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
    let did_change_server = server.clone();
    let did_change_handle = tokio::spawn(async move {
        did_change_server.did_change(did_change).await;
    });

    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(2)
                && server
                    .analysis_v2
                    .file_revision_state(file_id)
                    .await
                    .map(|state| state.version)
                    == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange must publish current revision before blocking parse");

    let completion_position = find_utf16_position_after_marker(FIXTURE, "ДляCompletion = Объект.");
    let started = Instant::now();
    let completion_labels = lsp_completion_labels_with_request(
        &mut service,
        406,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;
    let elapsed = started.elapsed();
    let timeline = lsp_get_completion_timeline(&mut service, 4060, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("completion trace after blocking didChange parse");
    assert!(
        elapsed < Duration::from_millis(250),
        "completion must not inherit blocking didChange parse delay before first poll (elapsed={elapsed:?}, labels={completion_labels:?}, trace={trace:?})"
    );
    assert!(
        did_change_handle.is_finished(),
        "didChange must already return while blocking parse continues in background"
    );

    did_change_handle.await.expect("didChange join");

    let service_future_to_first_poll_wait_ms = trace
        .get("server_edge_details")
        .and_then(|value| value.as_object())
        .and_then(|details| details.get("service_future_to_first_poll_wait_ms"))
        .and_then(|value| value.as_u64())
        .expect("service_future_to_first_poll_wait_ms");
    assert!(
        service_future_to_first_poll_wait_ms < 250,
        "service future first poll must not inherit blocking didChange parse delay, trace={trace:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_completion_transport_first_poll_stays_short_under_completion_burst() {
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

    const FIXTURE: &str = "Процедура Тест()\n    ДляCompletion = Объект.\nКонецПроцедуры\n";
    const SATURATING_REQUESTS: i64 = 4;
    const FIFTH_REQUEST_ID: i64 = 40_654;
    const COMPLETION_DELAY_MS: u64 = 350;
    const FIRST_POLL_BUDGET_MS: u64 = 150;

    let _env_lock = lock_test_env().await;
    let _completion_delay_guard = EnvVarGuard::set(
        "BSL_TEST_COMPLETION_DELAY_MS",
        &COMPLETION_DELAY_MS.to_string(),
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_completion_transport_burst.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
        },
    };
    server.did_open(did_open).await;
    server.sync_v2_globals().await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "ДляCompletion = Объект.");
    let completion_request = |request_id: i64| {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "textDocument/completion",
            "params": CompletionParams {
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
            },
        })
    };

    for request_id in 0..SATURATING_REQUESTS {
        harness
            .write_message(&completion_request(40_650 + request_id))
            .await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;

    let completion_response = harness
        .send_request(
            FIFTH_REQUEST_ID,
            "textDocument/completion",
            CompletionParams {
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
            },
        )
        .await;
    assert!(
        completion_response.get("result").is_some(),
        "completion request under burst must still complete"
    );

    let trace = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let timeline = live_transport_get_completion_timeline(&mut harness, 40_699, 16).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("completion timeline traces array");
            if let Some(trace) = traces.iter().find(|trace| {
                trace.get("request_id").and_then(|value| value.as_str())
                    == Some(&FIFTH_REQUEST_ID.to_string())
            }) {
                break trace.clone();
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("fifth completion trace must appear in timeline");

    let service_future_to_first_poll_wait_ms =
        completion_timeline_server_edge_u64(&trace, "service_future_to_first_poll_wait_ms")
            .expect("service_future_to_first_poll_wait_ms");
    assert!(
        service_future_to_first_poll_wait_ms <= FIRST_POLL_BUDGET_MS,
        "transport slot backlog must not delay fifth completion before first poll under a short completion burst, trace={trace:?}"
    );

    harness.shutdown().await;
}

#[tokio::test]
async fn p33_completion_waiter_registration_bypasses_unrelated_interactive_apply_backlog() {
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

    const TARGET_V1_FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\n    ДляCompletion = S.\nКонецПроцедуры\n";
    const TARGET_V2_FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Описание\", \"x\");\n    ДляCompletion = S.\nКонецПроцедуры\n";
    const BACKLOG_FILE_COUNT: usize = 6;
    const BACKLOG_APPLY_DELAY_MS: u64 = 120;
    const COMPLETION_BUDGET_MS: u64 = 500;
    const REGISTRATION_BUDGET_MS: u64 = 40;

    let _env_lock = lock_test_env().await;

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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let target_uri =
        Url::parse("file:///test_p33_completion_waiter_registration_target.bsl").expect("uri");
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didOpen")
                .params(
                    serde_json::to_value(DidOpenTextDocumentParams {
                        text_document: TextDocumentItem {
                            uri: target_uri.clone(),
                            language_id: "bsl".to_string(),
                            version: 1,
                            text: TARGET_V1_FIXTURE.to_string(),
                        },
                    })
                    .expect("DidOpenTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    server.sync_v2_globals().await;

    let target_file_id = server.get_or_create_file_id_v2(&target_uri).await;
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(
                    serde_json::to_value(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: target_uri.clone(),
                            version: 2,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: None,
                            range_length: None,
                            text: TARGET_V2_FIXTURE.to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("target didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&target_file_id)
                .copied()
                == Some(2)
                && server
                    .analysis_v2
                    .file_revision_state(target_file_id)
                    .await
                    .map(|state| state.version)
                    == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("target didChange must publish current revision before unrelated apply backlog");
    force_current_revision_without_exact_type_index(
        &server,
        target_file_id,
        &target_uri,
        TARGET_V2_FIXTURE,
        2,
    )
    .await;

    let _apply_delay_guard = EnvVarGuard::set(
        "BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS",
        &BACKLOG_APPLY_DELAY_MS.to_string(),
    );
    for index in 0..BACKLOG_FILE_COUNT {
        let backlog_uri = Url::parse(&format!(
            "file:///test_p33_completion_waiter_registration_backlog_{index}.bsl"
        ))
        .expect("backlog uri");
        let backlog_file_id = server.get_or_create_file_id_v2(&backlog_uri).await;
        let backlog_text: Arc<str> = Arc::from(format!(
            "Процедура Фон{index}()\n    Сообщить(\"{index}\");\nКонецПроцедуры\n"
        ));
        let backlog_path: Arc<str> = Arc::from(
            backlog_uri
                .to_file_path()
                .expect("backlog file path")
                .to_string_lossy()
                .to_string(),
        );
        server.analysis_v2.apply_changes_interactive(
            bsl_runtime::application::ObservabilityOrigin::Lsp,
            vec![bsl_analysis_v2::Change::SetFileWithSnapshot {
                file_id: backlog_file_id,
                text: backlog_text.clone(),
                version: 2,
                path: backlog_path,
                parse_snapshot: parse_snapshot_for_test(
                    backlog_file_id,
                    2,
                    backlog_text.as_ref(),
                    vec![],
                    true,
                    None,
                ),
            }],
        );
    }

    let completion_position =
        find_utf16_position_after_marker(TARGET_V2_FIXTURE, "ДляCompletion = S.");
    let request_id = "50621";
    crate::server::request_context::record_completion_request_id_for_testing(
        &target_uri,
        completion_position,
        request_id,
    );
    let started = Instant::now();
    let completion_response = tokio::time::timeout(
        Duration::from_millis(COMPLETION_BUDGET_MS),
        server.completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: target_uri.clone(),
                },
                position: completion_position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
        }),
    )
    .await
    .expect("completion must not inherit unrelated interactive apply backlog before waiter registration");
    let completion_response = completion_response
        .expect("completion request")
        .expect("completion response");
    let completion_labels: Vec<String> = match completion_response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    };
    let elapsed = started.elapsed();
    assert!(
        completion_labels.iter().any(|label| label == "Описание"),
        "target completion must still observe the already applied revision while unrelated apply backlog is queued, labels={completion_labels:?}"
    );
    assert!(
        elapsed <= Duration::from_millis(COMPLETION_BUDGET_MS),
        "completion must stay bounded under unrelated interactive apply backlog (elapsed={elapsed:?})"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 50_622, 20).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .iter()
        .rev()
        .find(|trace| trace.get("request_id").and_then(|value| value.as_str()) == Some(request_id))
        .expect("target completion trace under unrelated apply backlog");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "target completion must stay on the current-revision head fast path while unrelated apply backlog is queued, trace={trace:?}"
    );
    assert_ne!(
        completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
        Some("prepare_timeout"),
        "completion must not regress into prepare_timeout@wait_for_file_version when only waiter registration used to be blocked, trace={trace:?}"
    );
    let wait_runtime = trace
        .get("prepare_details")
        .and_then(|value| value.get("wait_for_file_version_runtime"));
    let registration_queue_wait_ms = wait_runtime
        .and_then(|value| value.get("queue_wait_ms"))
        .and_then(|value| value.as_u64());
    let head_ready_before_wait = trace
        .get("prepare_details")
        .and_then(|value| value.get("exact_wait"))
        .and_then(|value| value.get("head_ready_before_wait"))
        .and_then(|value| value.as_bool());
    let exact_ready_before_wait = trace
        .get("prepare_details")
        .and_then(|value| value.get("exact_wait"))
        .and_then(|value| value.get("exact_ready_before_wait"))
        .and_then(|value| value.as_bool());
    assert!(
        registration_queue_wait_ms.is_some()
            || head_ready_before_wait == Some(true)
            || exact_ready_before_wait == Some(true),
        "completion must either expose wait_for_file_version runtime registration latency or prove it bypassed the wait via ready current-revision artifacts, trace={trace:?}"
    );
    if let Some(registration_queue_wait_ms) = registration_queue_wait_ms {
        assert!(
            registration_queue_wait_ms <= REGISTRATION_BUDGET_MS,
            "completion waiter registration must stay bounded even while unrelated interactive apply backlog is queued, queue_wait_ms={}ms > {}ms, trace={trace:?}",
            registration_queue_wait_ms,
            REGISTRATION_BUDGET_MS,
        );
    }

    drain_task.abort();
}
