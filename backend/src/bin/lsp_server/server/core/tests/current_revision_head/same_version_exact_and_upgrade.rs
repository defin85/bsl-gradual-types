#[tokio::test]
async fn p33_changed_text_current_revision_head_stays_available_while_parse_snapshot_builds() {
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

    const V1_FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    S.Вставить(\"Описание\", \"x\");\n    ДляCompletion = S.\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let _blocking_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "1500");

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

    let uri = Url::parse("file:///test_p33_changed_text_current_revision_head.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: V1_FIXTURE.to_string(),
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
            text: V2_FIXTURE.to_string(),
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
    .expect(
        "didChange must publish changed current revision before blocking parse snapshot completes",
    );

    let did_save_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didSave")
                .params(
                    serde_json::to_value(DidSaveTextDocumentParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        text: None,
                    })
                    .expect("DidSaveTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didSave notification");
    assert!(did_save_response.is_none(), "didSave is a notification");
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !crate::server::language_server::did_save_inline_parse_delay_active_for_test(),
        "same-version didSave must not start a second delayed parse worker on the current-revision completion path"
    );

    let completion_position = find_utf16_position_after_marker(V2_FIXTURE, "ДляCompletion = S.");
    let started = Instant::now();
    let completion_response = server
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
        .expect("completion request")
        .expect("completion response");
    let elapsed = started.elapsed();
    let completion_labels: Vec<String> = match completion_response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    };
    assert!(
        completion_labels.iter().any(|label| label == "Описание"),
        "changed-text current-revision head must expose new member before parse snapshot completes, labels={completion_labels:?}"
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "changed-text current-revision head must stay bounded while parse snapshot builds in background (elapsed={elapsed:?})"
    );
    assert!(
        did_change_handle.is_finished(),
        "didChange must already return while changed-text parse snapshot build continues in background"
    );

    did_change_handle.await.expect("didChange join");

    let timeline = lsp_get_completion_timeline(&mut service, 4061, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("completion trace after changed-text didChange");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "changed-text completion must resolve through current-revision head route, trace={trace:?}"
    );
    assert_ne!(
        completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
        Some("exact_deadline"),
        "changed-text current-revision head must not regress into exact_deadline while parse snapshot is still building, trace={trace:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_changed_text_current_revision_head_waits_for_delayed_runtime_apply() {
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

    const V1_FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    S.Вставить(\"Описание\", \"x\");\n    ДляCompletion = S.\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "300");
    let _blocking_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "1500");

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

    let uri = Url::parse("file:///test_p33_changed_text_runtime_apply_delay.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: V1_FIXTURE.to_string(),
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
    let did_change_started = Instant::now();
    server
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: V2_FIXTURE.to_string(),
            }],
        })
        .await;
    let did_change_elapsed = did_change_started.elapsed();
    assert!(
        did_change_elapsed < Duration::from_millis(250),
        "didChange must return before delayed runtime apply completes (elapsed={did_change_elapsed:?})"
    );

    server.cancel_type_index_precompute_v2(file_id).await;

    tokio::time::timeout(Duration::from_millis(1200), async {
        loop {
            if server
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
    .expect("delayed runtime apply must eventually publish version 2");

    let completion_position = find_utf16_position_after_marker(V2_FIXTURE, "ДляCompletion = S.");
    let completion_response = server
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
        .expect("completion request")
        .expect("completion response");
    let completion_labels: Vec<String> = match completion_response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    };
    assert!(
        completion_labels.iter().any(|label| label == "Описание"),
        "current-revision head must survive delayed runtime apply and expose latest member on first response, labels={completion_labels:?}"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 4062, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("completion trace after delayed runtime apply");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "delayed runtime apply must still resolve through current-revision head route, trace={trace:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_changed_text_burst_supersedes_obsolete_current_revision_head_precompute() {
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

    let _env_lock = lock_test_env().await;

    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let head_precompute_delay_ms = (wait_budget_ms / 3).max(40);
    let _current_head_delay_guard = EnvVarGuard::set(
        "BSL_TEST_CURRENT_REVISION_HEAD_PRECOMPUTE_DELAY_MS",
        &head_precompute_delay_ms.to_string(),
    );
    let _async_parse_delay_guard = EnvVarGuard::set("BSL_TEST_DID_CHANGE_PARSE_DELAY_MS", "500");

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

    let uri = Url::parse("file:///test_p33_changed_text_burst_current_revision_supersession.bsl")
        .expect("uri");
    let mut current_text =
        "Процедура Тест()\n    S = Новый Структура;\n    ДляCompletion = S.\nКонецПроцедуры\n"
            .to_string();
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: current_text.clone(),
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

    let latest_version = 8_i32;
    for version in 2..=latest_version {
        let insert_line = format!("    S.Вставить(\"Поле{version}\", {version});\n");
        current_text = current_text.replacen(
            "    ДляCompletion = S.\n",
            &(insert_line + "    ДляCompletion = S.\n"),
            1,
        );
        server
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: current_text.clone(),
                }],
            })
            .await;
        server.cancel_type_index_precompute_v2(file_id).await;
    }

    let completion_position = find_utf16_position_after_marker(&current_text, "ДляCompletion = S.");
    let completion_started = Instant::now();
    let completion_response = server
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
        .expect("completion request")
        .expect("completion response");
    let completion_elapsed = completion_started.elapsed();
    let completion_labels: Vec<String> = match completion_response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    };
    assert!(
        completion_labels
            .iter()
            .any(|label| label == &format!("Поле{latest_version}")),
        "burst changed-text path must preserve latest current-revision head instead of burning CPU on obsolete versions, labels={completion_labels:?}"
    );
    assert!(
        completion_elapsed < Duration::from_millis(wait_budget_ms.saturating_mul(2)),
        "burst changed-text completion must stay bounded after current-revision head supersession (elapsed={completion_elapsed:?}, latest_version={latest_version}, labels={completion_labels:?})"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 4063, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("completion trace after changed-text burst supersession");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "burst changed-text completion must resolve through latest current-revision head route, trace={trace:?}"
    );
    assert_ne!(
        completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
        Some("exact_deadline"),
        "burst changed-text completion must not regress into exact_deadline when obsolete head precompute work is superseded, trace={trace:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_completion_head_hit_then_upgrade_after_precompute_finish() {
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

    const FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";

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

    let uri = Url::parse("file:///test_p33_completion_exact_wait_recovery.bsl").expect("uri");
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
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "ДляCompletion = S.");
    let first_started = Instant::now();
    let first_completion_labels =
        lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    let first_elapsed = first_started.elapsed();
    assert!(
        first_completion_labels
            .iter()
            .any(|label| label == "Количество"),
        "first member-access completion must serve typed-structure members from current-revision head while matching exact precompute is still computing, labels={first_completion_labels:?}"
    );
    assert!(
        first_elapsed < Duration::from_millis(250),
        "first head-path completion should stay bounded while exact precompute runs in background (elapsed={first_elapsed:?}, budget_ms={wait_budget_ms})"
    );

    wait_for_type_index_precompute_completion(&server, file_id).await;

    let second_completion_labels =
        lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        second_completion_labels
            .iter()
            .any(|label| label == "Количество"),
        "member-access completion must keep typed-structure members available after exact precompute finishes, labels={second_completion_labels:?}"
    );
    let timeline = lsp_get_completion_timeline(&mut service, 402, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let first_trace = traces
        .get(traces.len().saturating_sub(2))
        .expect("first head-hit completion trace");
    let second_trace = traces
        .last()
        .expect("second completion trace after exact precompute");
    assert_eq!(
        completion_timeline_prepare_detail_str(first_trace, "route"),
        Some("head_hit"),
        "first completion trace must expose current-revision head route while exact precompute is still computing, trace={first_trace:?}"
    );
    assert!(
        matches!(
            completion_timeline_prepare_detail_str(second_trace, "route"),
            Some("head_hit" | "exact_hit")
        ),
        "completion after exact precompute must stay on canonical head/exact route, trace={second_trace:?}"
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
        "typed-structure head path must not regress into exact-deadline fail-closed, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task"
        )),
        0,
        "typed-structure head path must not degrade to no_matching_task while exact precompute is present, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_task_present_wrong_version")
        ),
        0,
        "typed-structure head path must not report wrong_version while serving current revision, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline")
        ),
        0,
        "typed-structure head path must not attribute completion to exact-deadline once head artifact is available, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_head_hit")) > 0,
        "typed-structure head path must record head-hit route before exact upgrade, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_head_to_exact_upgrade_total")) > 0,
        "background exact precompute must still record head-to-exact upgrade for the same revision, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_same_version_exact_wait_keeps_completed_task_observable_until_cleanup() {
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

    const FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let _post_compute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_POST_COMPUTE_DELAY_MS",
        "250",
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

    let uri = Url::parse("file:///test_p33_same_version_completed_exact_wait.bsl").expect("uri");
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
    let requested_version = 2;
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: requested_version,
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
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Completed,
    )
    .await;

    tokio::time::sleep(Duration::from_millis(50)).await;
    {
        let tasks = server.type_index_precompute_tasks_v2.lock().await;
        let task = tasks
            .get(&file_id)
            .expect("completed same-version precompute task must remain observable during bounded cleanup window");
        assert_eq!(task.supersession_key.requested_version, requested_version);
        assert_eq!(
            crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::from_atomic(
                task.phase.load(std::sync::atomic::Ordering::Relaxed)
            ),
            crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Completed
        );
    }

    let exact_wait = server
        .wait_for_current_type_index_serve_only_ready_v2(
            file_id,
            Some(requested_version),
            Duration::from_millis(40),
        )
        .await;
    assert_ne!(
        exact_wait.outcome,
        crate::server::core::deps_and_precompute::ExactTypeIndexWaitOutcomeV2::NoMatchingTask,
        "same-version exact wait must not regress into no_matching_task while completed producer is still inside bounded cleanup window: trace={exact_wait:?}"
    );
    assert_eq!(
        exact_wait.matching_task_state,
        Some(crate::server::core::deps_and_precompute::ExactTypeIndexMatchingTaskStateV2::Matching),
        "same-version exact wait must keep observing the completed producer entry during cleanup window: trace={exact_wait:?}"
    );

    wait_for_type_index_precompute_completion(&server, file_id).await;
    assert!(
        !server
            .type_index_precompute_tasks_v2
            .lock()
            .await
            .contains_key(&file_id),
        "completed same-version task must clean up after exact-ready becomes observable"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_shutdown_cleans_retained_same_version_exact_task_entry() {
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

    const FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let _post_compute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_POST_COMPUTE_DELAY_MS",
        "250",
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

    let uri = Url::parse("file:///test_p33_shutdown_completed_exact_wait.bsl").expect("uri");
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
    let requested_version = 2;
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: requested_version,
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
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Completed,
    )
    .await;

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        server
            .type_index_precompute_tasks_v2
            .lock()
            .await
            .contains_key(&file_id),
        "completed same-version precompute task must still be retained before shutdown cleanup"
    );

    let shutdown_response = service
        .ready()
        .await
        .unwrap()
        .call(Request::build("shutdown").id(9001).finish())
        .await
        .expect("shutdown request");
    assert!(
        shutdown_response.is_some(),
        "shutdown should return a response"
    );

    assert!(
        !server
            .type_index_precompute_tasks_v2
            .lock()
            .await
            .contains_key(&file_id),
        "shutdown must clean retained same-version exact-task entries"
    );

    let exit_response = service
        .ready()
        .await
        .unwrap()
        .call(Request::build("exit").finish())
        .await
        .expect("exit notification");
    assert!(exit_response.is_none(), "exit is a notification");

    drain_task.abort();
}

#[tokio::test]
async fn p33_same_version_invoked_completion_keeps_completed_task_visible_on_default_path() {
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

    const FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let _post_compute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_POST_COMPUTE_DELAY_MS",
        "250",
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

    let uri = Url::parse("file:///test_p33_same_version_invoked_default_path.bsl").expect("uri");
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
    let requested_version = 2;
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: requested_version,
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
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Completed,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "ДляCompletion = S.");
    let labels = lsp_completion_labels_with_request(
        &mut service,
        9002,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;
    assert!(
        labels.iter().any(|label| label == "Количество"),
        "same-version invoked member completion on the default LSP path must preserve current-revision semantics while the completed exact task is still retained: labels={labels:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_completion_head_hit_emits_exact_upgrade_when_background_exact_finishes() {
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

    const FIXTURE: &str = "Процедура Тест()\n    Результат = (Новый Массив()).\nКонецПроцедуры\n";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let _precompute_delay_guard =
        EnvVarGuard::set("BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS", "200");
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

    let uri = Url::parse("file:///test_p33_completion_head_to_exact_upgrade.bsl").expect("uri");
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
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "(Новый Массив()).");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        !completion_labels.is_empty(),
        "head route must still provide current-revision completion while exact precompute computes in background, labels={completion_labels:?}"
    );

    wait_for_type_index_precompute_completion(&server, file_id).await;

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_head_hit")) > 0,
        "head route must be recorded before upgrade, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_head_to_exact_upgrade_total")) > 0,
        "background exact precompute must record head-to-exact upgrade for same revision, counters={counters:?}"
    );
    assert!(
        histograms
            .get("intellisense_v2_completion_head_to_exact_upgrade_ms")
            .and_then(|value| value.get("count"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0,
        "head-to-exact upgrade latency histogram must be emitted, histograms={histograms:?}"
    );

    drain_task.abort();
}
