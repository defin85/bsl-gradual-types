#[tokio::test]
async fn p7_completion_after_did_change_does_not_hang() {
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

    let uri = Url::parse("file:///test_p7.bsl").expect("test uri");

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Procedure Test()\nEndProcedure".to_string(),
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
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "Procedure Test()\n\t// p7\nEndProcedure".to_string(),
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

    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 0,
                character: 0,
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

    let completion_response = tokio::time::timeout(
        tokio::time::Duration::from_secs(2),
        service.ready().await.unwrap().call(completion_req),
    )
    .await
    .expect("completion request timeout")
    .expect("completion request");

    assert!(
        completion_response.is_some(),
        "completion should return a response"
    );

    drain_task.abort();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn p28_cancel_request_stops_completion_and_prevents_late_publish() {
    fn completion_response_incomplete_empty(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::List(list) => list.is_incomplete && list.items.is_empty(),
            CompletionResponse::Array(items) => items.is_empty(),
        }
    }
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
    let _delay_guard = EnvVarGuard::set("BSL_TEST_COMPLETION_DELAY_MS", "40");

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

    let uri = Url::parse("file:///test_p28_cancel_request.bsl").expect("test uri");
    let mut base_text = String::from("Процедура Тест()\n    ЛокМассив = Новый Массив;\n");
    for value in 0..800 {
        base_text.push_str(&format!("    ЛокМассив.Добавить({value});\n"));
    }
    base_text.push_str("    ЛокМассив.\nКонецПроцедуры\n");
    let completion_line = 802_u32;
    let completion_character = "    ЛокМассив.".encode_utf16().count() as u32;

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: base_text.clone(),
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
        .as_ref()
        .cloned()
        .expect("server instance");
    let file_id = server.get_or_create_file_id_v2(&uri).await;

    let mut observed_cancelled_completion = false;
    for attempt in 0..8_i32 {
        let version = attempt + 2;
        let changed_text = format!("{base_text}// attempt {attempt}\n");
        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: changed_text,
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

        let request_id = 100_i64 + i64::from(attempt);
        let completion_params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(completion_line, completion_character),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: Some("__bsl_shadow_internal__:46".to_string()),
            }),
        };
        let completion_req = Request::build("textDocument/completion")
            .id(request_id)
            .params(serde_json::to_value(completion_params).expect("CompletionParams"))
            .finish();
        let completion_future = service.ready().await.unwrap().call(completion_req);
        let completion_task = tokio::spawn(completion_future);
        let expected_epoch = u64::try_from(attempt + 1).expect("positive epoch");
        let mut before_state = None;
        for _ in 0..100 {
            if let Some((file_seq, epoch)) =
                server.completion_dispatcher_v2.debug_state(file_id).await
            {
                if epoch >= expected_epoch {
                    before_state = Some((file_seq, epoch));
                    break;
                }
            }
            tokio::task::yield_now().await;
        }
        let (before_file_seq, before_epoch) = before_state.expect("dispatcher state before cancel");
        let request_id_string = request_id.to_string();
        let mut registration_present = false;
        for _ in 0..20 {
            if server
                .completion_cancellation_registry_v2
                .get(&request_id_string)
                .is_some()
            {
                registration_present = true;
                break;
            }
            tokio::task::yield_now().await;
        }

        let cancel_req = Request::build("$/cancelRequest")
            .params(serde_json::json!({ "id": request_id }))
            .finish();
        let cancel_response = service
            .call(cancel_req)
            .await
            .expect("cancel request notification");
        assert!(cancel_response.is_none(), "cancel is a notification");

        let mut cancel_event_observed = false;
        for _ in 0..20 {
            if let Some((after_file_seq, after_epoch)) =
                server.completion_dispatcher_v2.debug_state(file_id).await
            {
                if after_file_seq > before_file_seq && after_epoch >= before_epoch {
                    cancel_event_observed = true;
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }

        let completion_response =
            tokio::time::timeout(tokio::time::Duration::from_secs(5), completion_task)
                .await
                .expect("completion request timeout")
                .expect("completion task join")
                .expect("completion request")
                .expect("completion response");
        let completion_value =
            serde_json::to_value(&completion_response).expect("serialize completion");
        let completion_is_safe =
            if let Some(completion_result) = completion_value.get("result").cloned() {
                let completion_lsp: Option<CompletionResponse> =
                    serde_json::from_value(completion_result).expect("parse completion result");
                completion_lsp
                    .as_ref()
                    .is_some_and(completion_response_incomplete_empty)
            } else if let Some(error) = completion_value.get("error") {
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
        if registration_present && cancel_event_observed && completion_is_safe {
            observed_cancelled_completion = true;
            break;
        }
    }

    assert!(
        observed_cancelled_completion,
        "expected $/cancelRequest to enqueue Cancel(request_id) and avoid late completion publish"
    );

    drain_task.abort();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn p28_cancel_request_before_first_poll_honors_cancellation() {
    fn completion_response_incomplete_empty(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::List(list) => list.is_incomplete && list.items.is_empty(),
            CompletionResponse::Array(items) => items.is_empty(),
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
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let uri =
        Url::parse("file:///test_p28_cancel_request_before_first_poll.bsl").expect("test uri");
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

    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position::new(2, "    ЛокМассив.".encode_utf16().count() as u32),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: Some("__bsl_shadow_internal__:46".to_string()),
        }),
    };
    let completion_req = Request::build("textDocument/completion")
        .id(701_i64)
        .params(serde_json::to_value(completion_params).expect("CompletionParams"))
        .finish();
    let completion_future = service.ready().await.unwrap().call(completion_req);

    let cancel_req = Request::build("$/cancelRequest")
        .params(serde_json::json!({ "id": 701_i64 }))
        .finish();
    let cancel_response = service
        .ready()
        .await
        .unwrap()
        .call(cancel_req)
        .await
        .expect("cancel request notification");
    assert!(cancel_response.is_none(), "cancel is a notification");

    let completion_response =
        tokio::time::timeout(tokio::time::Duration::from_secs(5), completion_future)
            .await
            .expect("completion request timeout")
            .expect("completion request")
            .expect("completion response");
    let completion_value =
        serde_json::to_value(&completion_response).expect("serialize completion");
    let completion_is_safe =
        if let Some(completion_result) = completion_value.get("result").cloned() {
            let completion_lsp: Option<CompletionResponse> =
                serde_json::from_value(completion_result).expect("parse completion result");
            completion_lsp
                .as_ref()
                .is_some_and(completion_response_incomplete_empty)
        } else if let Some(error) = completion_value.get("error") {
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
        "expected $/cancelRequest before first poll to prevent late completion publish"
    );

    drain_task.abort();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn p28_newer_completion_proactively_cancels_older_active_completion_on_same_file() {
    fn completion_response_incomplete_empty(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::List(list) => list.is_incomplete && list.items.is_empty(),
            CompletionResponse::Array(items) => items.is_empty(),
        }
    }

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
    let _delay_guard = EnvVarGuard::set("BSL_TEST_COMPLETION_DELAY_MS", "80");

    let fixture =
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
    let (service, drain_task, server, uri, _file_id) = open_lsp_fixture_with_snapshot(
        fixture,
        "file:///test_p28_active_completion_supersession.bsl",
    )
    .await;
    let mut service = crate::server::request_context::RequestContextService::new(service);
    let position = find_utf16_position_after_marker(fixture, "ЛокМассив.");

    let first_req = Request::build("textDocument/completion")
        .id(28001)
        .params(
            serde_json::to_value(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position,
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
        .finish();
    let first_future = service.ready().await.unwrap().call(first_req);
    let first_task = tokio::spawn(first_future);

    for _ in 0..40 {
        if server
            .completion_cancellation_registry_v2
            .get("28001")
            .is_some()
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        server
            .completion_cancellation_registry_v2
            .get("28001")
            .is_some(),
        "first completion request must register cancellation token before newer request arrives"
    );

    let second_req = Request::build("textDocument/completion")
        .id(28002)
        .params(
            serde_json::to_value(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::INVOKED,
                    trigger_character: None,
                }),
            })
            .expect("CompletionParams"),
        )
        .finish();
    let second_response = service
        .ready()
        .await
        .unwrap()
        .call(second_req)
        .await
        .expect("second completion request")
        .expect("second completion response");

    for _ in 0..40 {
        if server
            .completion_cancellation_registry_v2
            .get("28001")
            .is_none()
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        server
            .completion_cancellation_registry_v2
            .get("28001")
            .is_none(),
        "newer completion must proactively cancel the older active completion request on the same file"
    );

    let first_response = first_task
        .await
        .expect("first completion join")
        .expect("first completion request")
        .expect("first completion response");
    let first_value = serde_json::to_value(&first_response).expect("serialize first response");
    let first_result = first_value
        .get("result")
        .cloned()
        .expect("first completion result field");
    let first_completion: Option<CompletionResponse> =
        serde_json::from_value(first_result).expect("parse first completion result");
    assert!(
        first_completion
            .as_ref()
            .is_some_and(completion_response_incomplete_empty),
        "older superseded completion must resolve to bounded incomplete empty response, response={first_completion:?}"
    );

    let second_value = serde_json::to_value(&second_response).expect("serialize second response");
    let second_result = second_value
        .get("result")
        .cloned()
        .expect("second completion result field");
    let second_completion: Option<CompletionResponse> =
        serde_json::from_value(second_result).expect("parse second completion result");
    assert!(
        second_completion.is_some(),
        "newer completion request must still produce a response"
    );

    drain_task.abort();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn p29_completion_mode_matrix_parity_on_fixed_revision() {
    const CHANGE_ID: &str = "refactor-v2-completion-event-driven-pipeline";
    const ITERATIONS: usize = 40;
    const MAX_USER_FACING_DRIFT_RATE: f64 = 0.01;
    const MAX_SHADOW_PARITY_DRIFT_RATE: f64 = 0.01;
    const MIN_FIRST_TRIGGER_SUCCESS_RATE: f64 = 0.99;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct CompletionFingerprint {
        is_incomplete: bool,
        labels: Vec<String>,
    }

    #[derive(Debug, Clone, Copy)]
    struct ModeScenario {
        name: &'static str,
        completion_mode: &'static str,
        canary_percent: u8,
    }

    #[derive(Debug)]
    struct ModeOutcome {
        name: String,
        completion_p95_ms: f64,
        completion_p99_ms: f64,
        completion_total: u64,
        first_trigger_success_rate: f64,
        parity_drift_rate: f64,
        legacy_stage_total: u64,
        shadow_stage_total: u64,
        event_driven_stage_total: u64,
        dot_fingerprints: Vec<CompletionFingerprint>,
        invoked_fingerprints: Vec<CompletionFingerprint>,
    }

    struct CompletionModeEnvGuard {
        previous_mode: Option<String>,
        previous_canary_percent: Option<String>,
    }

    impl CompletionModeEnvGuard {
        fn new() -> Self {
            Self {
                previous_mode: std::env::var("BSL_INTELLISENSE_V2_COMPLETION_MODE").ok(),
                previous_canary_percent: std::env::var(
                    "BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT",
                )
                .ok(),
            }
        }

        fn apply(&self, completion_mode: &str, canary_percent: u8) {
            std::env::set_var("BSL_INTELLISENSE_V2_COMPLETION_MODE", completion_mode);
            std::env::set_var(
                "BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT",
                canary_percent.to_string(),
            );
            bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
        }
    }

    impl Drop for CompletionModeEnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.previous_mode {
                std::env::set_var("BSL_INTELLISENSE_V2_COMPLETION_MODE", value);
            } else {
                std::env::remove_var("BSL_INTELLISENSE_V2_COMPLETION_MODE");
            }
            if let Some(value) = &self.previous_canary_percent {
                std::env::set_var("BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT", value);
            } else {
                std::env::remove_var("BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT");
            }
            bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
        }
    }

    fn metric_as_f64(value: Option<&serde_json::Value>) -> f64 {
        value
            .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
            .unwrap_or(0.0)
    }

    fn completion_items_count(response: &CompletionResponse) -> usize {
        match response {
            CompletionResponse::Array(items) => items.len(),
            CompletionResponse::List(list) => list.items.len(),
        }
    }

    fn completion_fingerprint(response: &CompletionResponse) -> CompletionFingerprint {
        let (is_incomplete, labels) = match response {
            CompletionResponse::Array(items) => (
                false,
                items
                    .iter()
                    .map(|item| item.label.clone())
                    .collect::<BTreeSet<_>>(),
            ),
            CompletionResponse::List(list) => (
                list.is_incomplete,
                list.items
                    .iter()
                    .map(|item| item.label.clone())
                    .collect::<BTreeSet<_>>(),
            ),
        };
        CompletionFingerprint {
            is_incomplete,
            labels: labels.into_iter().collect(),
        }
    }

    fn sum_counters_by_prefix(
        counters: &serde_json::Map<String, serde_json::Value>,
        prefix: &str,
    ) -> u64 {
        counters
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(_, value)| value.as_u64().unwrap_or(0))
            .sum()
    }

    fn completion_stage_mode_total(
        counters: &serde_json::Map<String, serde_json::Value>,
        mode: &str,
    ) -> u64 {
        counters
            .iter()
            .filter(|(key, _)| {
                key.starts_with("intellisense_v2_drilldown_stage_total_")
                    && key.contains("_origin_lsp_")
                    && key.contains("_operation_completion_")
                    && key.contains(&format!("_mode_{mode}"))
            })
            .map(|(_, value)| value.as_u64().unwrap_or(0))
            .sum()
    }

    async fn run_mode_scenario(scenario: ModeScenario, iterations: usize) -> ModeOutcome {
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
            .as_ref()
            .cloned()
            .expect("server instance");
        prime_server_with_syntax_helper_deps(&server).await;

        let uri =
            Url::parse(&format!("file:///test_p29_mode_{}.bsl", scenario.name)).expect("test uri");
        let text = concat!(
            "Процедура Тест()\n",
            "    ЛокМассив = Новый Массив;\n",
            "    ЛокМассив.\n",
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
        let member_character = "    ЛокМассив."
            .chars()
            .map(|ch| ch.len_utf16())
            .sum::<usize>() as u32;

        let mut dot_fingerprints = Vec::with_capacity(iterations);
        let mut invoked_fingerprints = Vec::with_capacity(iterations);
        let mut first_trigger_success_total = 0_u64;

        for _ in 0..iterations {
            let dot_completion = server
                .completion(CompletionParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position: Position::new(2, member_character),
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
            if completion_items_count(&dot_completion) > 0 {
                first_trigger_success_total += 1;
            }
            dot_fingerprints.push(completion_fingerprint(&dot_completion));

            let invoked_completion = server
                .completion(CompletionParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position: Position::new(2, member_character),
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
            invoked_fingerprints.push(completion_fingerprint(&invoked_completion));
        }

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");
        let completion_hist = histograms
            .get("completion_duration_ms")
            .and_then(|value| value.as_object())
            .expect("completion duration histogram");
        let completion_p95_ms = metric_as_f64(completion_hist.get("p95"));
        let completion_p99_ms = metric_as_f64(completion_hist.get("p99"));
        let completion_total = counters
            .get("completion_total")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let parity_pairs_total = (iterations as u64) * 2;
        let parity_drift_total = sum_counters_by_prefix(
            counters,
            "intellisense_v2_completion_parity_drift_total_mode_",
        );
        let parity_drift_rate = parity_drift_total as f64 / parity_pairs_total.max(1) as f64;
        let first_trigger_success_rate =
            first_trigger_success_total as f64 / iterations.max(1) as f64;
        let legacy_stage_total = completion_stage_mode_total(counters, "legacy");
        let shadow_stage_total = completion_stage_mode_total(counters, "shadow");
        let event_driven_stage_total = completion_stage_mode_total(counters, "event_driven");

        drain_task.abort();

        ModeOutcome {
            name: scenario.name.to_string(),
            completion_p95_ms,
            completion_p99_ms,
            completion_total,
            first_trigger_success_rate,
            parity_drift_rate,
            legacy_stage_total,
            shadow_stage_total,
            event_driven_stage_total,
            dot_fingerprints,
            invoked_fingerprints,
        }
    }

    let _env_lock = lock_test_env_blocking();
    let env_guard = CompletionModeEnvGuard::new();

    let scenarios = [
        ModeScenario {
            name: "off",
            completion_mode: "off",
            canary_percent: 0,
        },
        ModeScenario {
            name: "shadow",
            completion_mode: "shadow",
            canary_percent: 0,
        },
        ModeScenario {
            name: "canary",
            completion_mode: "canary",
            canary_percent: 100,
        },
        ModeScenario {
            name: "on",
            completion_mode: "on",
            canary_percent: 0,
        },
    ];

    let mut outcomes = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        env_guard.apply(scenario.completion_mode, scenario.canary_percent);
        let outcome = run_mode_scenario(scenario, ITERATIONS).await;
        assert!(
            outcome.first_trigger_success_rate >= MIN_FIRST_TRIGGER_SUCCESS_RATE,
            "mode={} first-trigger success rate={:.4} < {:.4}",
            outcome.name,
            outcome.first_trigger_success_rate,
            MIN_FIRST_TRIGGER_SUCCESS_RATE
        );
        outcomes.push(outcome);
    }

    let off_outcome = outcomes
        .iter()
        .find(|outcome| outcome.name == "off")
        .expect("off mode outcome");
    let mut drift_by_mode = serde_json::Map::new();
    for outcome in outcomes.iter().filter(|outcome| outcome.name != "off") {
        let dot_mismatch_total = outcome
            .dot_fingerprints
            .iter()
            .zip(off_outcome.dot_fingerprints.iter())
            .filter(|(actual, expected)| actual != expected)
            .count() as u64;
        let invoked_mismatch_total = outcome
            .invoked_fingerprints
            .iter()
            .zip(off_outcome.invoked_fingerprints.iter())
            .filter(|(actual, expected)| actual != expected)
            .count() as u64;
        let mismatch_total = dot_mismatch_total + invoked_mismatch_total;
        let mismatch_rate = mismatch_total as f64 / ((ITERATIONS * 2) as f64);

        drift_by_mode.insert(
            outcome.name.clone(),
            serde_json::json!({
                "mismatch_total": mismatch_total,
                "mismatch_rate": mismatch_rate,
                "dot_mismatch_total": dot_mismatch_total,
                "invoked_mismatch_total": invoked_mismatch_total,
            }),
        );
        assert!(
            mismatch_rate <= MAX_USER_FACING_DRIFT_RATE,
            "mode={} user-facing completion drift rate={:.4} > {:.4}",
            outcome.name,
            mismatch_rate,
            MAX_USER_FACING_DRIFT_RATE
        );
    }

    let shadow_outcome = outcomes
        .iter()
        .find(|outcome| outcome.name == "shadow")
        .expect("shadow mode outcome");
    let canary_outcome = outcomes
        .iter()
        .find(|outcome| outcome.name == "canary")
        .expect("canary mode outcome");
    let on_outcome = outcomes
        .iter()
        .find(|outcome| outcome.name == "on")
        .expect("on mode outcome");

    assert!(
        off_outcome.legacy_stage_total > 0
            && off_outcome.shadow_stage_total == 0
            && off_outcome.event_driven_stage_total == 0,
        "off mode stage routing must be strictly legacy: {:?}",
        (
            off_outcome.legacy_stage_total,
            off_outcome.shadow_stage_total,
            off_outcome.event_driven_stage_total
        )
    );
    assert!(
        shadow_outcome.legacy_stage_total > 0
            && shadow_outcome.shadow_stage_total > 0
            && shadow_outcome.event_driven_stage_total == 0,
        "shadow mode must route user-facing via legacy and run shadow pipeline: {:?}",
        (
            shadow_outcome.legacy_stage_total,
            shadow_outcome.shadow_stage_total,
            shadow_outcome.event_driven_stage_total
        )
    );
    assert!(
        shadow_outcome.parity_drift_rate <= MAX_SHADOW_PARITY_DRIFT_RATE,
        "shadow mode parity drift rate={:.4} > {:.4}",
        shadow_outcome.parity_drift_rate,
        MAX_SHADOW_PARITY_DRIFT_RATE
    );
    assert!(
        canary_outcome.event_driven_stage_total > 0
            && canary_outcome.legacy_stage_total == 0
            && canary_outcome.shadow_stage_total == 0,
        "canary(100) mode must route completion via event-driven only: {:?}",
        (
            canary_outcome.legacy_stage_total,
            canary_outcome.shadow_stage_total,
            canary_outcome.event_driven_stage_total
        )
    );
    assert!(
        on_outcome.event_driven_stage_total > 0
            && on_outcome.legacy_stage_total == 0
            && on_outcome.shadow_stage_total == 0,
        "on mode must route completion via event-driven only: {:?}",
        (
            on_outcome.legacy_stage_total,
            on_outcome.shadow_stage_total,
            on_outcome.event_driven_stage_total
        )
    );

    let mut modes_report = serde_json::Map::new();
    for outcome in &outcomes {
        modes_report.insert(
            outcome.name.clone(),
            serde_json::json!({
                "completion_total": outcome.completion_total,
                "completion_p95_ms": outcome.completion_p95_ms,
                "completion_p99_ms": outcome.completion_p99_ms,
                "first_trigger_success_rate": outcome.first_trigger_success_rate,
                "parity_drift_rate": outcome.parity_drift_rate,
                "stage_totals": {
                    "legacy": outcome.legacy_stage_total,
                    "shadow": outcome.shadow_stage_total,
                    "event_driven": outcome.event_driven_stage_total
                }
            }),
        );
    }
    let report = serde_json::json!({
        "change_id": CHANGE_ID,
        "profile": "p29_completion_mode_matrix_parity_on_fixed_revision",
        "iterations": ITERATIONS,
        "thresholds": {
            "max_user_facing_drift_rate": MAX_USER_FACING_DRIFT_RATE,
            "max_shadow_parity_drift_rate": MAX_SHADOW_PARITY_DRIFT_RATE,
            "min_first_trigger_success_rate": MIN_FIRST_TRIGGER_SUCCESS_RATE
        },
        "mode_user_facing_drift_vs_off": drift_by_mode,
        "modes": serde_json::Value::Object(modes_report),
    });
    let report_path = std::env::var("BSL_V2_COMPLETION_MODE_MATRIX_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!("{CHANGE_ID}-mode-parity-matrix.json"))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for completion mode matrix report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report)
            .expect("failed to serialize completion mode matrix report"),
    )
    .expect("failed to write completion mode matrix report");
    println!("v2_completion_mode_matrix_report={}", report_path.display());
}

#[tokio::test]
async fn p30_backpressure_fairness_interactive_vs_background_no_starvation() {
    const CHANGE_ID: &str = "refactor-v2-completion-event-driven-pipeline";
    const INTERACTIVE_PROBE_TOTAL: usize = 24;
    const BACKGROUND_BURST_TOTAL: usize = 24;
    const INTERACTIVE_BURST_TOTAL: usize = 32;
    const BACKGROUND_PROBE_TOTAL: usize = 16;
    const ROUND_TIMEOUT_SECS: u64 = 30;
    const MAX_REQUEST_LATENCY_MS: f64 = 10_000.0;

    async fn run_hover_requests(
        server: BslLanguageServer,
        uri: Url,
        position: Position,
        total: usize,
    ) -> (u64, f64) {
        let mut success_total = 0_u64;
        let mut max_latency_ms = 0.0_f64;
        for _ in 0..total {
            let started = Instant::now();
            let response = server
                .hover(HoverParams {
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position,
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .await;
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            max_latency_ms = max_latency_ms.max(elapsed_ms);
            if response.is_ok() {
                success_total += 1;
            }
        }
        (success_total, max_latency_ms)
    }

    async fn run_hover_burst(
        server: BslLanguageServer,
        uri: Url,
        position: Position,
        total: usize,
    ) -> (u64, f64) {
        let mut handles = Vec::with_capacity(total);
        for _ in 0..total {
            let server = server.clone();
            let uri = uri.clone();
            handles.push(tokio::spawn(async move {
                let started = Instant::now();
                let response = server
                    .hover(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri },
                            position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                    })
                    .await;
                (response.is_ok(), started.elapsed().as_secs_f64() * 1000.0)
            }));
        }
        let mut success_total = 0_u64;
        let mut max_latency_ms = 0.0_f64;
        for handle in handles {
            let (ok, latency_ms) = handle.await.expect("hover burst task join");
            if ok {
                success_total += 1;
            }
            max_latency_ms = max_latency_ms.max(latency_ms);
        }
        (success_total, max_latency_ms)
    }

    async fn run_workspace_symbol_requests(
        server: BslLanguageServer,
        query: String,
        total: usize,
    ) -> (u64, f64) {
        let mut success_total = 0_u64;
        let mut max_latency_ms = 0.0_f64;
        for _ in 0..total {
            let started = Instant::now();
            let response = server
                .symbol(WorkspaceSymbolParams {
                    query: query.clone(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await;
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            max_latency_ms = max_latency_ms.max(elapsed_ms);
            if response.is_ok() {
                success_total += 1;
            }
        }
        (success_total, max_latency_ms)
    }

    async fn run_workspace_symbol_burst(
        server: BslLanguageServer,
        query: String,
        total: usize,
    ) -> (u64, f64) {
        let mut handles = Vec::with_capacity(total);
        for _ in 0..total {
            let server = server.clone();
            let query = query.clone();
            handles.push(tokio::spawn(async move {
                let started = Instant::now();
                let response = server
                    .symbol(WorkspaceSymbolParams {
                        query,
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    })
                    .await;
                (response.is_ok(), started.elapsed().as_secs_f64() * 1000.0)
            }));
        }
        let mut success_total = 0_u64;
        let mut max_latency_ms = 0.0_f64;
        for handle in handles {
            let (ok, latency_ms) = handle.await.expect("workspace_symbol burst task join");
            if ok {
                success_total += 1;
            }
            max_latency_ms = max_latency_ms.max(latency_ms);
        }
        (success_total, max_latency_ms)
    }

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

    let mut primary_uri: Option<Url> = None;
    for index in 0..8_u32 {
        let uri = Url::parse(&format!("file:///test_p30_fairness_{index}.bsl")).expect("uri");
        if primary_uri.is_none() {
            primary_uri = Some(uri.clone());
        }
        let mut text = format!("Процедура Тест{index}()\n    ЛокПерем = Новый Массив;\n");
        for value in 0..120_u32 {
            text.push_str(&format!("    ЛокПерем.Добавить({value});\n"));
        }
        text.push_str("    Возврат ЛокПерем.Количество();\nКонецПроцедуры\n");
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text,
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
    }
    let primary_uri = primary_uri.expect("primary uri");
    let hover_position = Position::new(2, 8);

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .as_ref()
        .cloned()
        .expect("server instance");

    let (warm_interactive_success, _) =
        run_hover_requests(server.clone(), primary_uri.clone(), hover_position, 2).await;
    assert!(
        warm_interactive_success > 0,
        "warm-up interactive requests should succeed"
    );
    let (warm_background_success, _) =
        run_workspace_symbol_requests(server.clone(), "Тест".to_string(), 2).await;
    assert!(
        warm_background_success > 0,
        "warm-up background requests should succeed"
    );

    let round_a_background = tokio::spawn(run_workspace_symbol_burst(
        server.clone(),
        "Тест".to_string(),
        BACKGROUND_BURST_TOTAL,
    ));
    let round_a_interactive = tokio::spawn(run_hover_requests(
        server.clone(),
        primary_uri.clone(),
        hover_position,
        INTERACTIVE_PROBE_TOTAL,
    ));
    let (round_a_background_success, round_a_background_max_ms) =
        tokio::time::timeout(Duration::from_secs(ROUND_TIMEOUT_SECS), round_a_background)
            .await
            .expect("background burst timeout in round A")
            .expect("background burst join in round A");
    let (round_a_interactive_success, round_a_interactive_max_ms) =
        tokio::time::timeout(Duration::from_secs(ROUND_TIMEOUT_SECS), round_a_interactive)
            .await
            .expect("interactive probe timeout in round A")
            .expect("interactive probe join in round A");

    let round_b_interactive = tokio::spawn(run_hover_burst(
        server.clone(),
        primary_uri.clone(),
        hover_position,
        INTERACTIVE_BURST_TOTAL,
    ));
    let round_b_background = tokio::spawn(run_workspace_symbol_requests(
        server.clone(),
        "Тест".to_string(),
        BACKGROUND_PROBE_TOTAL,
    ));
    let (round_b_interactive_success, round_b_interactive_max_ms) =
        tokio::time::timeout(Duration::from_secs(ROUND_TIMEOUT_SECS), round_b_interactive)
            .await
            .expect("interactive burst timeout in round B")
            .expect("interactive burst join in round B");
    let (round_b_background_success, round_b_background_max_ms) =
        tokio::time::timeout(Duration::from_secs(ROUND_TIMEOUT_SECS), round_b_background)
            .await
            .expect("background probe timeout in round B")
            .expect("background probe join in round B");

    assert_eq!(
        round_a_interactive_success, INTERACTIVE_PROBE_TOTAL as u64,
        "interactive requests must progress under background burst"
    );
    assert_eq!(
        round_a_background_success, BACKGROUND_BURST_TOTAL as u64,
        "background burst must complete without starvation"
    );
    assert_eq!(
        round_b_interactive_success, INTERACTIVE_BURST_TOTAL as u64,
        "interactive burst must complete under mixed load"
    );
    assert_eq!(
        round_b_background_success, BACKGROUND_PROBE_TOTAL as u64,
        "background probe must progress under interactive burst"
    );
    for (name, value) in [
        ("round_a_background_max_ms", round_a_background_max_ms),
        ("round_a_interactive_max_ms", round_a_interactive_max_ms),
        ("round_b_background_max_ms", round_b_background_max_ms),
        ("round_b_interactive_max_ms", round_b_interactive_max_ms),
    ] {
        assert!(
            value <= MAX_REQUEST_LATENCY_MS,
            "{name} exceeded bounded latency: {value:.2}ms > {MAX_REQUEST_LATENCY_MS:.2}ms"
        );
    }

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let interactive_queue_wait_total = counters
        .get("intellisense_v2_runtime_queue_wait_interactive_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let background_queue_wait_total = counters
        .get("intellisense_v2_runtime_queue_wait_background_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let interactive_exec_total = counters
        .get("intellisense_v2_runtime_exec_interactive_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let background_exec_total = counters
        .get("intellisense_v2_runtime_exec_background_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);

    assert!(
        interactive_queue_wait_total > 0,
        "interactive queue-wait counter must be present under mixed load"
    );
    assert!(
        background_queue_wait_total > 0,
        "background queue-wait counter must be present under mixed load"
    );
    assert!(
        interactive_exec_total > 0,
        "interactive exec counter must be present under mixed load"
    );
    assert!(
        background_exec_total > 0,
        "background exec counter must be present under mixed load"
    );

    let report = serde_json::json!({
        "change_id": CHANGE_ID,
        "profile": "p30_backpressure_fairness_interactive_vs_background_no_starvation",
        "thresholds": {
            "round_timeout_secs": ROUND_TIMEOUT_SECS,
            "max_request_latency_ms": MAX_REQUEST_LATENCY_MS,
        },
        "rounds": {
            "background_burst_vs_interactive_probe": {
                "interactive_total": INTERACTIVE_PROBE_TOTAL,
                "interactive_success": round_a_interactive_success,
                "interactive_max_latency_ms": round_a_interactive_max_ms,
                "background_total": BACKGROUND_BURST_TOTAL,
                "background_success": round_a_background_success,
                "background_max_latency_ms": round_a_background_max_ms,
            },
            "interactive_burst_vs_background_probe": {
                "interactive_total": INTERACTIVE_BURST_TOTAL,
                "interactive_success": round_b_interactive_success,
                "interactive_max_latency_ms": round_b_interactive_max_ms,
                "background_total": BACKGROUND_PROBE_TOTAL,
                "background_success": round_b_background_success,
                "background_max_latency_ms": round_b_background_max_ms,
            }
        },
        "metrics": {
            "intellisense_v2_runtime_queue_wait_interactive_total": interactive_queue_wait_total,
            "intellisense_v2_runtime_queue_wait_background_total": background_queue_wait_total,
            "intellisense_v2_runtime_exec_interactive_total": interactive_exec_total,
            "intellisense_v2_runtime_exec_background_total": background_exec_total,
        },
        "pass": true
    });
    let report_path = std::env::var("BSL_V2_COMPLETION_FAIRNESS_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!(
                    "{CHANGE_ID}-fairness-interactive-vs-background.json"
                ))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for completion fairness report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report)
            .expect("failed to serialize completion fairness report"),
    )
    .expect("failed to write completion fairness report");
    println!("v2_completion_fairness_report={}", report_path.display());

    drain_task.abort();
}

#[tokio::test]
async fn p30_cross_file_did_change_parallel_completion_no_global_lock_bottleneck() {
    const CHANGE_ID: &str = "add-performance-first-ai-engineering-guardrails";
    const DID_CHANGE_BURST_PER_FILE: u32 = 8;
    const COMPLETION_BURST_PER_FILE: usize = 20;
    const REQUEST_TIMEOUT_SECS: u64 = 30;
    const MAX_COMPLETION_LATENCY_MS: f64 = 10_000.0;
    const MAX_QUEUE_WAIT_P95_MS: f64 = 2_000.0;
    const MIN_CONCURRENCY_GAIN: f64 = 1.10;
    const MIN_AVG_LATENCY_FOR_GAIN_CHECK_MS: f64 = 2.0;

    fn completion_items_count(response: &CompletionResponse) -> usize {
        match response {
            CompletionResponse::Array(items) => items.len(),
            CompletionResponse::List(list) => list.items.len(),
        }
    }

    fn completion_is_incomplete(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::Array(_) => false,
            CompletionResponse::List(list) => list.is_incomplete,
        }
    }

    fn metric_as_f64(value: Option<&serde_json::Value>) -> f64 {
        value
            .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
            .unwrap_or(0.0)
    }

    fn build_document_text(function_name: &str) -> String {
        format!(
            "Процедура {function_name}()\n    ДляCompletion = (Новый Массив()).\nКонецПроцедуры\n"
        )
    }

    struct DocumentState {
        uri: Url,
        version: i32,
        text: String,
    }

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
    {
        let server = server_holder
            .lock()
            .expect("server holder lock")
            .as_ref()
            .cloned()
            .expect("server instance");
        prime_server_with_syntax_helper_deps(&server).await;
    }

    let mut documents = vec![
        DocumentState {
            uri: Url::parse("file:///test_p30_cross_file_a.bsl").expect("document uri A"),
            version: 1,
            text: build_document_text("ТестA"),
        },
        DocumentState {
            uri: Url::parse("file:///test_p30_cross_file_b.bsl").expect("document uri B"),
            version: 1,
            text: build_document_text("ТестB"),
        },
    ];

    for document in &documents {
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: document.uri.clone(),
                language_id: "bsl".to_string(),
                version: document.version,
                text: document.text.clone(),
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
    }

    for burst_idx in 0..DID_CHANGE_BURST_PER_FILE {
        for (doc_idx, document) in documents.iter_mut().enumerate() {
            document.version += 1;
            document
                .text
                .push_str(&format!("// churn doc={doc_idx} step={burst_idx}\n"));
            let did_change = DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: document.uri.clone(),
                    version: document.version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: document.text.clone(),
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
    }

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .as_ref()
        .cloned()
        .expect("server instance");
    for document in &documents {
        let file_id = server.get_or_create_file_id_v2(&document.uri).await;
        assert!(
            server
                .analysis_v2
                .wait_for_file_version(file_id, document.version)
                .await,
            "analysis runtime must catch up to didChange burst for {}",
            document.uri
        );
        wait_for_type_index_precompute_completion(&server, file_id).await;
    }
    let mut owner_hint_type_names = Vec::with_capacity(documents.len());
    let completion_position =
        find_utf16_position_after_marker(&documents[0].text, "(Новый Массив()).");
    for document in &documents {
        let file_id = server.get_or_create_file_id_v2(&document.uri).await;
        let analysis = server.analysis_v2.snapshot().await;
        owner_hint_type_names.push((
            document.uri.to_string(),
            bsl_runtime::application::completion_member_access_owner_type_hints_from_analysis(
                &analysis,
                file_id,
                &document.text,
                completion_position.line,
                completion_position.character,
            )
            .into_iter()
            .map(|hint| hint.type_name())
            .collect::<Vec<_>>(),
        ));
    }

    let mut handles = Vec::with_capacity(documents.len().saturating_mul(COMPLETION_BURST_PER_FILE));
    let wall_started = Instant::now();
    for document in &documents {
        for _ in 0..COMPLETION_BURST_PER_FILE {
            let server = server.clone();
            let uri = document.uri.clone();
            handles.push(tokio::spawn(async move {
                let started = Instant::now();
                let response = server
                    .completion(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri },
                            position: completion_position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                        context: Some(CompletionContext {
                            trigger_kind: CompletionTriggerKind::INVOKED,
                            trigger_character: None,
                        }),
                    })
                    .await;
                (response, started.elapsed().as_secs_f64() * 1000.0)
            }));
        }
    }
    let completion_outcomes =
        tokio::time::timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS), async move {
            let mut outcomes = Vec::with_capacity(handles.len());
            for handle in handles {
                outcomes.push(handle.await.expect("parallel completion task join"));
            }
            outcomes
        })
        .await
        .expect("parallel completion burst timed out");
    let wall_time_ms = wall_started.elapsed().as_secs_f64() * 1000.0;

    let mut success_total = 0_u64;
    let mut non_empty_total = 0_u64;
    let mut empty_incomplete_total = 0_u64;
    let mut empty_complete_total = 0_u64;
    let mut sum_latency_ms = 0.0_f64;
    let mut max_latency_ms = 0.0_f64;
    for (response, latency_ms) in completion_outcomes {
        sum_latency_ms += latency_ms;
        max_latency_ms = max_latency_ms.max(latency_ms);
        if let Ok(Some(completion)) = response {
            success_total += 1;
            if completion_items_count(&completion) > 0 {
                non_empty_total += 1;
            } else if completion_is_incomplete(&completion) {
                empty_incomplete_total += 1;
            } else {
                empty_complete_total += 1;
            }
        }
    }

    let total_requests = (documents.len() * COMPLETION_BURST_PER_FILE) as u64;
    let average_latency_ms = sum_latency_ms / total_requests.max(1) as f64;
    let concurrency_gain = if wall_time_ms > 0.0 {
        sum_latency_ms / wall_time_ms
    } else {
        1.0
    };

    assert_eq!(
        success_total, total_requests,
        "parallel completion burst must complete successfully for all cross-file requests"
    );
    assert!(
        non_empty_total > 0,
        "parallel completion burst produced only empty completion payloads after didChange burst: empty_incomplete_total={empty_incomplete_total}, empty_complete_total={empty_complete_total}, owner_hint_type_names={owner_hint_type_names:?}"
    );
    assert!(
        max_latency_ms <= MAX_COMPLETION_LATENCY_MS,
        "cross-file completion max latency exceeded: {max_latency_ms:.2}ms > {MAX_COMPLETION_LATENCY_MS:.2}ms"
    );
    if average_latency_ms >= MIN_AVG_LATENCY_FOR_GAIN_CHECK_MS {
        assert!(
            concurrency_gain >= MIN_CONCURRENCY_GAIN,
            "parallel completion behaved as serialized workload after didChange burst: gain={concurrency_gain:.2} (sum={sum_latency_ms:.2}ms wall={wall_time_ms:.2}ms)"
        );
    }

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    let queue_wait_interactive_p95 = histograms
        .get("intellisense_v2_runtime_queue_wait_interactive_ms")
        .and_then(|value| value.as_object())
        .map(|hist| metric_as_f64(hist.get("p95")))
        .unwrap_or(0.0);
    let completion_total_counter = counters
        .get("completion_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let interactive_exec_total = counters
        .get("intellisense_v2_runtime_exec_interactive_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);

    assert!(
        completion_total_counter > 0,
        "completion_total counter must be populated for cross-file parallel burst"
    );
    assert!(
        interactive_exec_total > 0,
        "interactive exec counter must be populated for cross-file parallel burst"
    );
    assert!(
        queue_wait_interactive_p95 <= MAX_QUEUE_WAIT_P95_MS,
        "interactive queue-wait p95 regression after didChange burst: {queue_wait_interactive_p95:.2}ms > {MAX_QUEUE_WAIT_P95_MS:.2}ms"
    );

    let report = serde_json::json!({
        "change_id": CHANGE_ID,
        "profile": "p30_cross_file_did_change_parallel_completion_no_global_lock_bottleneck",
        "inputs": {
            "documents_total": documents.len(),
            "did_change_burst_per_file": DID_CHANGE_BURST_PER_FILE,
            "parallel_completion_burst_per_file": COMPLETION_BURST_PER_FILE,
        },
        "thresholds": {
            "request_timeout_secs": REQUEST_TIMEOUT_SECS,
            "max_completion_latency_ms": MAX_COMPLETION_LATENCY_MS,
            "max_queue_wait_interactive_p95_ms": MAX_QUEUE_WAIT_P95_MS,
            "min_concurrency_gain": MIN_CONCURRENCY_GAIN,
            "min_avg_latency_for_gain_check_ms": MIN_AVG_LATENCY_FOR_GAIN_CHECK_MS,
        },
        "results": {
            "total_requests": total_requests,
            "success_total": success_total,
            "non_empty_total": non_empty_total,
            "sum_latency_ms": sum_latency_ms,
            "wall_time_ms": wall_time_ms,
            "average_latency_ms": average_latency_ms,
            "max_latency_ms": max_latency_ms,
            "concurrency_gain": concurrency_gain,
            "queue_wait_interactive_p95_ms": queue_wait_interactive_p95,
            "completion_total_counter": completion_total_counter,
            "interactive_exec_total": interactive_exec_total,
        },
        "pass": true
    });
    let report_path = std::env::var("BSL_V2_COMPLETION_CROSS_FILE_DID_CHANGE_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!(
                    "{CHANGE_ID}-didchange-parallel-completion-cross-file.json"
                ))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for cross-file completion report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report)
            .expect("failed to serialize cross-file completion report"),
    )
    .expect("failed to write cross-file completion report");
    println!(
        "v2_completion_cross_file_did_change_report={}",
        report_path.display()
    );

    drain_task.abort();
}
