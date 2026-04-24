#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn p33_completion_current_revision_head_ignores_did_change_inline_parse_delay() {
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

    const FIXTURE: &str = "Процедура Тест()\n    ДляCompletion = Объект.\nКонецПроцедуры\n";

    let _env_lock = lock_test_env_blocking();
    let _parse_delay_guard = EnvVarGuard::set("BSL_TEST_DID_CHANGE_PARSE_DELAY_MS", "1500");

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

    let uri = Url::parse("file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl")
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
            if super::super::language_server::did_change_inline_parse_delay_active_for_test()
                && server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange must enter delayed inline parse window");
    assert_eq!(
        server
            .analysis_v2
            .file_revision_state(file_id)
            .await
            .map(|state| state.version),
        Some(2),
        "current-revision apply must reach analysis runtime before delayed inline parse completes"
    );

    let completion_position = find_utf16_position_after_marker(FIXTURE, "ДляCompletion = Объект.");
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
    let completion_labels = extract_completion_labels(completion_response);
    assert!(
        completion_labels.iter().any(|label| label == "Ссылка"),
        "current-revision head path must stay available while didChange inline parse is delayed, labels={completion_labels:?}"
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "head-path completion must not wait for delayed didChange inline parse to finish (elapsed={elapsed:?})"
    );
    assert!(
        did_change_handle.is_finished(),
        "didChange must already return while delayed parse continues in background"
    );

    did_change_handle.await.expect("didChange join");

    let timeline = lsp_get_completion_timeline(&mut service, 405, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("completion trace after delayed didChange inline parse");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "current-revision completion must still expose head_hit route while didChange parse remains in-flight, trace={trace:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_same_file_completion_supersession_releases_active_turn_during_response_build() {
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

    const FIRST_REQUEST_ID: i64 = 40_710;
    const SECOND_REQUEST_ID: i64 = 40_711;
    const FIRST_POLL_BUDGET_MS: u64 = 150;

    fn completion_response_incomplete_empty(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::List(list) => list.is_incomplete && list.items.is_empty(),
            CompletionResponse::Array(items) => items.is_empty(),
        }
    }

    fn completion_response_empty(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::List(list) => list.items.is_empty(),
            CompletionResponse::Array(items) => items.is_empty(),
        }
    }

    let _env_lock = lock_test_env().await;
    let _response_build_delay_guard =
        EnvVarGuard::set("BSL_TEST_COMPLETION_RESPONSE_BUILD_DELAY_MS", "300");

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let fixture_path = workspace_root.join("examples").join("test_lsp.bsl");
    let fixture_text =
        std::fs::read_to_string(&fixture_path).expect("read examples/test_lsp.bsl fixture");
    let uri = Url::from_file_path(&fixture_path).expect("fixture uri");
    harness
        .send_notification(
            "textDocument/didOpen",
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: fixture_text.clone(),
                },
            },
        )
        .await;
    server.sync_v2_globals().await;

    let position = find_utf16_position_after_marker(&fixture_text, "Arr.");
    live_transport_write_completion_request(
        &mut harness,
        FIRST_REQUEST_ID,
        &uri,
        position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if server
                .completion_cancellation_registry_v2
                .get(&FIRST_REQUEST_ID.to_string())
                .is_some()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("first completion request must register before supersession");
    tokio::time::sleep(Duration::from_millis(50)).await;

    live_transport_write_completion_request(
        &mut harness,
        SECOND_REQUEST_ID,
        &uri,
        position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;

    let (first_response, second_response) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut first_response = None;
        let mut second_response = None;
        loop {
            let response = harness.read_message().await;
            match response.get("id").and_then(|value| value.as_i64()) {
                Some(FIRST_REQUEST_ID) => first_response = Some(response),
                Some(SECOND_REQUEST_ID) => second_response = Some(response),
                _ => {}
            }
            if first_response.is_some() && second_response.is_some() {
                break (
                    first_response.take().expect("first completion response"),
                    second_response.take().expect("second completion response"),
                );
            }
        }
    })
    .await
    .expect("both completion responses must arrive");

    for _ in 0..40 {
        if server
            .completion_cancellation_registry_v2
            .get(&FIRST_REQUEST_ID.to_string())
            .is_none()
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        server
            .completion_cancellation_registry_v2
            .get(&FIRST_REQUEST_ID.to_string())
            .is_none(),
        "newer same-file completion must proactively cancel the older active request"
    );

    let parse_completion_response = |response: &serde_json::Value| {
        let result = response
            .get("result")
            .cloned()
            .expect("completion result field");
        serde_json::from_value::<Option<CompletionResponse>>(result)
            .expect("parse completion response")
    };
    let first_completion =
        parse_completion_response(&first_response).expect("first completion result present");
    assert!(
        completion_response_incomplete_empty(&first_completion)
            || completion_response_empty(&first_completion),
        "older superseded completion must resolve to bounded empty response, response={first_response:?}"
    );

    assert!(
        parse_completion_response(&second_response).is_some(),
        "newer same-file completion must still produce a bounded response"
    );

    let traces = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let timeline = live_transport_get_completion_timeline(&mut harness, 40_799, 32).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("completion timeline traces array");
            let first_trace = traces.iter().find(|trace| {
                trace.get("request_id").and_then(|value| value.as_str())
                    == Some(&FIRST_REQUEST_ID.to_string())
            });
            let second_trace = traces.iter().find(|trace| {
                trace.get("request_id").and_then(|value| value.as_str())
                    == Some(&SECOND_REQUEST_ID.to_string())
            });
            if let (Some(first_trace), Some(second_trace)) = (first_trace, second_trace) {
                break (first_trace.clone(), second_trace.clone());
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("completion overlap traces must appear in timeline");
    let (first_trace, second_trace) = traces;

    let second_first_poll_wait_ms =
        completion_timeline_server_edge_u64(&second_trace, "service_future_to_first_poll_wait_ms")
            .expect("second request service_future_to_first_poll_wait_ms");
    assert!(
        second_first_poll_wait_ms <= FIRST_POLL_BUDGET_MS,
        "newer same-file completion must reach first poll within budget while stale response_build is cancelled, trace={second_trace:?}"
    );
    assert!(
        matches!(
            first_trace.get("outcome").and_then(|value| value.as_str()),
            Some("cancelled" | "superseded")
        ),
        "older overlap trace must terminate with cancelled/superseded outcome, trace={first_trace:?}"
    );

    live_transport_close_document(&mut harness, &uri).await;
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn p47_same_file_ingress_token_waits_for_handoff_registration_before_republishing() {
    const FIXTURE: &str = "Процедура Тест()\nКонецПроцедуры\n";

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

    let uri = Url::parse("file:///Documents/Док1/Forms/Форма1/Ext/Form/SameFileIngressToken.bsl")
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
    let open_token = server
        .same_file_ingress_token_for_test(file_id)
        .await
        .expect("didOpen must publish same-file ingress token");
    assert_eq!(open_token.file_version, 1);
    assert_eq!(open_token.source.as_contract_str(), "did_open");

    let text_sync_guard = server.text_sync_v2.lock().await;
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

    tokio::time::sleep(Duration::from_millis(100)).await;

    let token_before_handoff = server
        .same_file_ingress_token_for_test(file_id)
        .await
        .expect("pre-handoff token state must still be available");
    assert_eq!(
        token_before_handoff.file_version, 1,
        "same-file token must not republish to version 2 before current-revision handoff registration"
    );
    assert_eq!(token_before_handoff.source.as_contract_str(), "did_open");

    drop(text_sync_guard);
    did_change_handle.await.expect("didChange join");

    let token_after_handoff = server
        .same_file_ingress_token_for_test(file_id)
        .await
        .expect("post-handoff token state");
    assert_eq!(token_after_handoff.file_version, 2);
    assert_eq!(token_after_handoff.source.as_contract_str(), "did_change");

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn p47_did_save_republishes_same_file_ingress_token_after_existing_handoff() {
    const FIXTURE: &str = "Процедура Тест()\nКонецПроцедуры\n";

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

    let uri =
        Url::parse("file:///Documents/Док1/Forms/Форма1/Ext/Form/SameFileIngressTokenSave.bsl")
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
    let token_after_open = server
        .same_file_ingress_token_for_test(file_id)
        .await
        .expect("didOpen must publish same-file ingress token");
    assert_eq!(token_after_open.file_version, 1);
    assert_eq!(token_after_open.source.as_contract_str(), "did_open");

    server
        .did_save(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            text: None,
        })
        .await;

    let token_after_save = server
        .same_file_ingress_token_for_test(file_id)
        .await
        .expect("didSave must republish same-file ingress token");
    assert_eq!(token_after_save.file_version, 1);
    assert_eq!(token_after_save.source.as_contract_str(), "did_save");
    assert!(
        token_after_save.published_at_ms >= token_after_open.published_at_ms,
        "didSave token publication must not move backwards in time"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p48_out_of_order_did_change_cannot_republish_stale_same_file_token() {
    const V1_FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\nКонецПроцедуры\n";
    const V3_FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\nКонецПроцедуры\n";
    const V2_FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Описание\", \"x\");\nКонецПроцедуры\n";

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

    let uri =
        Url::parse("file:///Documents/Док1/Forms/Форма1/Ext/Form/OutOfOrderSameFileToken.bsl")
            .expect("form module uri");
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didOpen")
                .params(
                    serde_json::to_value(DidOpenTextDocumentParams {
                        text_document: TextDocumentItem {
                            uri: uri.clone(),
                            language_id: "bsl".to_string(),
                            version: 1,
                            text: V1_FIXTURE.to_string(),
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
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let did_change_v3_response = service
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
                            range: None,
                            range_length: None,
                            text: V3_FIXTURE.to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("version 3 didChange notification");
    assert!(did_change_v3_response.is_none(), "didChange is a notification");

    let token_after_v3 = server
        .same_file_ingress_token_for_test(file_id)
        .await
        .expect("version 3 didChange must publish token");
    assert_eq!(token_after_v3.file_version, 3);
    assert_eq!(token_after_v3.source.as_contract_str(), "did_change");

    let did_change_v2_response = service
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
                            range: None,
                            range_length: None,
                            text: V2_FIXTURE.to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("out-of-order didChange notification");
    assert!(did_change_v2_response.is_none(), "didChange is a notification");

    assert_eq!(
        server
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied(),
        Some(3),
        "older didChange must not overwrite latest-received version after a newer revision already won"
    );
    assert_eq!(
        server
            .latest_current_revision_handoff_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied(),
        Some(3),
        "older didChange must not re-register a stale current-revision handoff"
    );
    let shadow_state_after_v2 = server
        .latest_document_shadow_state_v2
        .read()
        .await
        .get(&file_id)
        .cloned()
        .expect("shadow state after newer didChange");
    assert_eq!(shadow_state_after_v2.version, 3);
    assert_eq!(shadow_state_after_v2.text.as_ref(), V3_FIXTURE);
    assert_eq!(
        server
            .analysis_v2
            .file_revision_state(file_id)
            .await
            .map(|state| state.version),
        Some(3),
        "older didChange must not overwrite analysis runtime after a newer revision already won"
    );
    let token_after_v2 = server
        .same_file_ingress_token_for_test(file_id)
        .await
        .expect("same-file token after out-of-order didChange");
    assert_eq!(token_after_v2.file_version, 3);
    assert_eq!(token_after_v2.source.as_contract_str(), "did_change");

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_same_file_completion_supersession_releases_pre_active_turn_wait_before_active_registration(
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

    const FIRST_REQUEST_ID: i64 = 40_714;
    const SECOND_REQUEST_ID: i64 = 40_715;
    const FIRST_POLL_BUDGET_MS: u64 = 150;

    fn completion_response_incomplete_empty(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::List(list) => list.is_incomplete && list.items.is_empty(),
            CompletionResponse::Array(items) => items.is_empty(),
        }
    }

    fn completion_response_empty(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::List(list) => list.items.is_empty(),
            CompletionResponse::Array(items) => items.is_empty(),
        }
    }

    let _env_lock = lock_test_env().await;
    super::super::language_server::reset_completion_checkpoint_hits_for_test();
    let checkpoint_delay_guard = EnvVarGuard::set(
        "BSL_TEST_COMPLETION_CHECKPOINT_DELAYS",
        "before_active_turn_registration=1000",
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let fixture_path = workspace_root.join("examples").join("test_lsp.bsl");
    let fixture_text =
        std::fs::read_to_string(&fixture_path).expect("read examples/test_lsp.bsl fixture");
    let uri = Url::from_file_path(&fixture_path).expect("fixture uri");
    harness
        .send_notification(
            "textDocument/didOpen",
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: fixture_text.clone(),
                },
            },
        )
        .await;
    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;

    let position = find_utf16_position_after_marker(&fixture_text, "Arr.");
    live_transport_write_completion_request(
        &mut harness,
        FIRST_REQUEST_ID,
        &uri,
        position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if server
                .completion_cancellation_registry_v2
                .get(&FIRST_REQUEST_ID.to_string())
                .is_some()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("first completion request must register before pre-active overlap");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if super::super::language_server::completion_checkpoint_hits_for_test(
                "before_active_turn_registration",
            ) >= 1
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("first completion request must reach pre-active turn_wait checkpoint");
    drop(checkpoint_delay_guard);

    live_transport_write_completion_request(
        &mut harness,
        SECOND_REQUEST_ID,
        &uri,
        position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;

    let second_request_active = tokio::time::timeout(Duration::from_millis(400), async {
        loop {
            if server
                .completion_dispatcher_v2
                .debug_active_holder_request_id(file_id)
                .await
                .as_deref()
                == Some(&SECOND_REQUEST_ID.to_string())
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(
        second_request_active.is_ok(),
        "newer same-file completion must become the active holder while stale predecessor is still in pre-active turn_wait window"
    );

    let (first_response, second_response) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut first_response = None;
        let mut second_response = None;
        loop {
            let response = harness.read_message().await;
            match response.get("id").and_then(|value| value.as_i64()) {
                Some(FIRST_REQUEST_ID) => first_response = Some(response),
                Some(SECOND_REQUEST_ID) => second_response = Some(response),
                _ => {}
            }
            if first_response.is_some() && second_response.is_some() {
                break (
                    first_response.take().expect("first completion response"),
                    second_response.take().expect("second completion response"),
                );
            }
        }
    })
    .await
    .expect("both pre-active overlap responses must arrive");

    let parse_completion_response = |response: &serde_json::Value| {
        let result = response
            .get("result")
            .cloned()
            .expect("completion result field");
        serde_json::from_value::<Option<CompletionResponse>>(result)
            .expect("parse completion response")
    };
    let first_completion =
        parse_completion_response(&first_response).expect("first completion result present");
    assert!(
        completion_response_incomplete_empty(&first_completion)
            || completion_response_empty(&first_completion),
        "older pre-active completion must resolve to bounded empty response, response={first_response:?}"
    );
    assert!(
        parse_completion_response(&second_response).is_some(),
        "newer same-file completion must still produce a bounded response"
    );

    let traces = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let timeline = live_transport_get_completion_timeline(&mut harness, 40_816, 32).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("completion timeline traces array");
            let first_trace = traces.iter().find(|trace| {
                trace.get("request_id").and_then(|value| value.as_str())
                    == Some(&FIRST_REQUEST_ID.to_string())
            });
            let second_trace = traces.iter().find(|trace| {
                trace.get("request_id").and_then(|value| value.as_str())
                    == Some(&SECOND_REQUEST_ID.to_string())
            });
            if let (Some(first_trace), Some(second_trace)) = (first_trace, second_trace) {
                break (first_trace.clone(), second_trace.clone());
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("pre-active overlap traces must appear in timeline");
    let (first_trace, second_trace) = traces;

    let second_first_poll_wait_ms =
        completion_timeline_server_edge_u64(&second_trace, "service_future_to_first_poll_wait_ms")
            .expect("second request service_future_to_first_poll_wait_ms");
    assert!(
        second_first_poll_wait_ms <= FIRST_POLL_BUDGET_MS,
        "newer same-file completion must reach first poll within budget while stale predecessor is stopped pre-active, trace={second_trace:?}"
    );
    assert!(
        first_trace
            .get("turn_attribution")
            .and_then(|value| value.get("turn_wait_outcome"))
            .and_then(|value| value.as_str())
            == Some("ready"),
        "pre-active overlap trace must prove that the stale predecessor had already exited queue, trace={first_trace:?}"
    );
    assert!(
        first_trace
            .get("turn_attribution")
            .and_then(|value| value.get("turn_wait_resolved_at_ms"))
            .and_then(|value| value.as_u64())
            .is_some(),
        "pre-active overlap trace must expose absolute turn_wait resolution timestamp, trace={first_trace:?}"
    );
    assert!(
        matches!(
            first_trace.get("outcome").and_then(|value| value.as_str()),
            Some("cancelled" | "superseded")
        ),
        "older pre-active overlap trace must terminate with cancelled/superseded outcome, trace={first_trace:?}"
    );

    live_transport_close_document(&mut harness, &uri).await;
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_same_file_completion_burst_does_not_strand_superseded_pre_active_turn_wait_requests() {
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

    const REQUEST_COUNT: usize = 24;
    const FIRST_REQUEST_ID: i64 = 40_717;
    const LAST_REQUEST_ID: i64 = FIRST_REQUEST_ID + REQUEST_COUNT as i64 - 1;
    const FIRST_POLL_BUDGET_MS: u64 = 450;
    const STRANDED_PRE_ACTIVE_TURN_WAIT_AGE_BUDGET_MS: u64 = 500;

    let _env_lock = lock_test_env().await;
    super::super::language_server::reset_completion_checkpoint_hits_for_test();
    let _checkpoint_delay_guard = EnvVarGuard::set(
        "BSL_TEST_COMPLETION_CHECKPOINT_DELAYS",
        "before_active_turn_registration=500",
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let fixture_path = workspace_root.join("examples").join("test_lsp.bsl");
    let fixture_text =
        std::fs::read_to_string(&fixture_path).expect("read examples/test_lsp.bsl fixture");
    let uri = Url::from_file_path(&fixture_path).expect("fixture uri");
    harness
        .send_notification(
            "textDocument/didOpen",
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: fixture_text.clone(),
                },
            },
        )
        .await;
    server.sync_v2_globals().await;

    let position = find_utf16_position_after_marker(&fixture_text, "Arr.");
    let completion_context = Some(CompletionContext {
        trigger_kind: CompletionTriggerKind::INVOKED,
        trigger_character: None,
    });
    let warmup_labels = live_transport_completion_labels_with_request(
        &mut harness,
        40_716,
        &uri,
        position,
        completion_context.clone(),
    )
    .await;
    assert!(
        !warmup_labels.is_empty(),
        "warmup completion must prime event-driven path before burst overlap regression"
    );

    for request_id in FIRST_REQUEST_ID..=LAST_REQUEST_ID {
        live_transport_write_completion_request(
            &mut harness,
            request_id,
            &uri,
            position,
            completion_context.clone(),
        )
        .await;
    }

    let responses = tokio::time::timeout(Duration::from_secs(20), async {
        let mut responses = std::collections::BTreeMap::new();
        while responses.len() < REQUEST_COUNT {
            let response = harness.read_message().await;
            let Some(request_id) = response.get("id").and_then(|value| value.as_i64()) else {
                continue;
            };
            if (FIRST_REQUEST_ID..=LAST_REQUEST_ID).contains(&request_id) {
                responses.insert(request_id, response);
            }
        }
        responses
    })
    .await
    .expect("burst overlap responses must arrive");

    let last_response = responses
        .get(&LAST_REQUEST_ID)
        .expect("last burst completion response");
    let last_completion = last_response
        .get("result")
        .cloned()
        .and_then(|result| {
            serde_json::from_value::<Option<CompletionResponse>>(result)
                .expect("parse last completion result")
        })
        .expect("last burst completion result present");
    let last_labels = normalize_lsp_member_labels(&last_completion);
    assert!(
        !last_labels.is_empty(),
        "latest burst completion must still return bounded non-empty labels, response={last_response:?}"
    );

    let timeline = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let timeline = live_transport_get_completion_timeline(&mut harness, 40_817, 160).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("completion timeline traces array");
            let last_trace = traces.iter().find(|trace| {
                trace.get("request_id").and_then(|value| value.as_str())
                    == Some(&LAST_REQUEST_ID.to_string())
            });
            if let Some(last_trace) = last_trace {
                break last_trace.clone();
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("burst overlap last trace must appear in completion timeline");

    let last_first_poll_wait_ms =
        completion_timeline_server_edge_u64(&timeline, "service_future_to_first_poll_wait_ms")
            .expect("last request service_future_to_first_poll_wait_ms");
    assert!(
        last_first_poll_wait_ms <= FIRST_POLL_BUDGET_MS,
        "latest same-file burst completion must reach first poll within budget even while superseded predecessors are stopping pre-active, trace={timeline:?}"
    );

    let stranded_pre_active_turn_wait_contender_age_ms = timeline
        .get("server_edge_details")
        .and_then(|details| details.get("first_poll_contention_contenders"))
        .and_then(|value| value.as_array())
        .and_then(|contenders| {
            contenders.iter().find_map(|contender| {
                let request_class = contender
                    .get("request_class")
                    .and_then(|value| value.as_str());
                let phase = contender.get("phase").and_then(|value| value.as_str());
                if request_class == Some("completion") && phase == Some("turn_wait") {
                    contender.get("age_ms").and_then(|value| value.as_u64())
                } else {
                    None
                }
            })
        })
        .unwrap_or(0);
    assert!(
        stranded_pre_active_turn_wait_contender_age_ms
            <= STRANDED_PRE_ACTIVE_TURN_WAIT_AGE_BUDGET_MS,
        "latest same-file burst completion must not observe long-lived superseded pre-active turn_wait contender, trace={timeline:?}"
    );

    live_transport_close_document(&mut harness, &uri).await;
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p28_cancel_request_releases_pre_active_turn_wait_before_active_registration() {
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

    const FIRST_REQUEST_ID: i64 = 40_716;

    fn completion_response_incomplete_empty(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::List(list) => list.is_incomplete && list.items.is_empty(),
            CompletionResponse::Array(items) => items.is_empty(),
        }
    }

    let _env_lock = lock_test_env().await;
    super::super::language_server::reset_completion_checkpoint_hits_for_test();
    let _checkpoint_delay_guard = EnvVarGuard::set(
        "BSL_TEST_COMPLETION_CHECKPOINT_DELAYS",
        "before_active_turn_registration=1000",
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let fixture_path = workspace_root.join("examples").join("test_lsp.bsl");
    let fixture_text =
        std::fs::read_to_string(&fixture_path).expect("read examples/test_lsp.bsl fixture");
    let uri = Url::from_file_path(&fixture_path).expect("fixture uri");
    harness
        .send_notification(
            "textDocument/didOpen",
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: fixture_text.clone(),
                },
            },
        )
        .await;
    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;

    let position = find_utf16_position_after_marker(&fixture_text, "Arr.");
    live_transport_write_completion_request(
        &mut harness,
        FIRST_REQUEST_ID,
        &uri,
        position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if server
                .completion_cancellation_registry_v2
                .get(&FIRST_REQUEST_ID.to_string())
                .is_some()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("first completion request must register before explicit pre-active cancel");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if super::super::language_server::completion_checkpoint_hits_for_test(
                "before_active_turn_registration",
            ) >= 1
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("first completion request must reach pre-active checkpoint before explicit cancel");

    harness
        .send_notification(
            "$/cancelRequest",
            serde_json::json!({ "id": FIRST_REQUEST_ID }),
        )
        .await;

    let completion_response = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let response = harness.read_message().await;
            if response.get("id").and_then(|value| value.as_i64()) == Some(FIRST_REQUEST_ID) {
                break response;
            }
        }
    })
    .await
    .expect("cancelled pre-active completion response timeout");

    let completion_is_safe =
        if let Some(completion_result) = completion_response.get("result").cloned() {
            let completion_lsp: Option<CompletionResponse> =
                serde_json::from_value(completion_result).expect("parse completion result");
            completion_lsp
                .as_ref()
                .is_some_and(completion_response_incomplete_empty)
        } else if let Some(error) = completion_response.get("error") {
            let error_code = error
                .get("code")
                .and_then(|value| value.as_i64())
                .unwrap_or_default();
            let error_message = error
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            error_code == -32800 || error_message.contains("cancel")
        } else {
            false
        };
    assert!(
        completion_is_safe,
        "explicit cancel in pre-active turn_wait window must prevent late completion publish, response={completion_response:?}"
    );

    let no_active_holder = tokio::time::timeout(Duration::from_millis(400), async {
        loop {
            if server
                .completion_dispatcher_v2
                .debug_active_holder_request_id(file_id)
                .await
                .is_none()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(
        no_active_holder.is_ok(),
        "explicitly cancelled pre-active completion must not become active"
    );

    live_transport_close_document(&mut harness, &uri).await;
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_same_file_completion_supersession_releases_active_turn_at_format_checkpoint() {
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

    const FIRST_REQUEST_ID: i64 = 40_712;
    const SECOND_REQUEST_ID: i64 = 40_713;
    const FIRST_POLL_BUDGET_MS: u64 = 450;

    fn completion_response_incomplete_empty(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::List(list) => list.is_incomplete && list.items.is_empty(),
            CompletionResponse::Array(items) => items.is_empty(),
        }
    }

    let _env_lock = lock_test_env().await;
    super::super::language_server::reset_completion_checkpoint_hits_for_test();
    let _checkpoint_delay_guard = EnvVarGuard::set(
        "BSL_TEST_COMPLETION_CHECKPOINT_DELAYS",
        "before_format_checkpoint=1000,after_format_outcome=1000",
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let fixture_path = workspace_root.join("examples").join("test_lsp.bsl");
    let fixture_text =
        std::fs::read_to_string(&fixture_path).expect("read examples/test_lsp.bsl fixture");
    let uri = Url::from_file_path(&fixture_path).expect("fixture uri");
    harness
        .send_notification(
            "textDocument/didOpen",
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: fixture_text.clone(),
                },
            },
        )
        .await;
    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;

    let position = find_utf16_position_after_marker(&fixture_text, "Arr.");
    live_transport_write_completion_request(
        &mut harness,
        FIRST_REQUEST_ID,
        &uri,
        position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if server
                .completion_cancellation_registry_v2
                .get(&FIRST_REQUEST_ID.to_string())
                .is_some()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("first completion request must register before supersession");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if super::super::language_server::completion_checkpoint_hits_for_test(
                "before_format_checkpoint",
            ) >= 1
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("first completion request must reach the pre-format checkpoint window");

    live_transport_write_completion_request(
        &mut harness,
        SECOND_REQUEST_ID,
        &uri,
        position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if super::super::language_server::completion_checkpoint_hits_for_test(
                "after_format_outcome",
            ) >= 1
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("stale first completion must reach the post-format checkpoint window");

    let second_request_active = tokio::time::timeout(Duration::from_millis(400), async {
        loop {
            if server
                .completion_dispatcher_v2
                .debug_active_holder_request_id(file_id)
                .await
                .as_deref()
                == Some(&SECOND_REQUEST_ID.to_string())
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(
        second_request_active.is_ok(),
        "newer same-file completion must become the active holder while stale format checkpoint is still unwinding"
    );

    let (first_response, second_response) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut first_response = None;
        let mut second_response = None;
        loop {
            let response = harness.read_message().await;
            match response.get("id").and_then(|value| value.as_i64()) {
                Some(FIRST_REQUEST_ID) => first_response = Some(response),
                Some(SECOND_REQUEST_ID) => second_response = Some(response),
                _ => {}
            }
            if first_response.is_some() && second_response.is_some() {
                break (
                    first_response.take().expect("first completion response"),
                    second_response.take().expect("second completion response"),
                );
            }
        }
    })
    .await
    .expect("both completion responses must arrive");

    let parse_completion_response = |response: &serde_json::Value| {
        let result = response
            .get("result")
            .cloned()
            .expect("completion result field");
        serde_json::from_value::<Option<CompletionResponse>>(result)
            .expect("parse completion response")
    };
    let first_completion =
        parse_completion_response(&first_response).expect("first completion result present");
    assert!(
        completion_response_incomplete_empty(&first_completion),
        "older superseded completion must resolve to bounded empty response at format checkpoint, response={first_response:?}"
    );
    assert!(
        parse_completion_response(&second_response).is_some(),
        "newer same-file completion must still produce a bounded response"
    );

    let traces = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let timeline = live_transport_get_completion_timeline(&mut harness, 40_814, 32).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("completion timeline traces array");
            let first_trace = traces.iter().find(|trace| {
                trace.get("request_id").and_then(|value| value.as_str())
                    == Some(&FIRST_REQUEST_ID.to_string())
            });
            let second_trace = traces.iter().find(|trace| {
                trace.get("request_id").and_then(|value| value.as_str())
                    == Some(&SECOND_REQUEST_ID.to_string())
            });
            if let (Some(first_trace), Some(second_trace)) = (first_trace, second_trace) {
                break (first_trace.clone(), second_trace.clone());
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("completion format-checkpoint traces must appear in timeline");
    let (first_trace, second_trace) = traces;

    let second_first_poll_wait_ms =
        completion_timeline_server_edge_u64(&second_trace, "service_future_to_first_poll_wait_ms")
            .expect("second request service_future_to_first_poll_wait_ms");
    assert!(
        second_first_poll_wait_ms <= FIRST_POLL_BUDGET_MS,
        "newer same-file completion must reach first poll within budget while stale format checkpoint is unwinding, trace={second_trace:?}"
    );
    assert!(
        matches!(
            first_trace.get("outcome").and_then(|value| value.as_str()),
            Some("cancelled" | "superseded")
        ),
        "older overlap trace must terminate with cancelled/superseded outcome, trace={first_trace:?}"
    );

    live_transport_close_document(&mut harness, &uri).await;
    harness.shutdown().await;
}
