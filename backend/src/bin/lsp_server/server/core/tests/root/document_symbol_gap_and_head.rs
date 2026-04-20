#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_document_symbol_returns_latest_ready_from_cache_during_parse_gap() {
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

    const V1_FIXTURE: &str =
        "#Область Public\nПроцедура OldProc() Экспорт\nКонецПроцедуры\n#КонецОбласти\n";
    const V2_FIXTURE: &str =
        "#Область Public\nПроцедура NewProc() Экспорт\nКонецПроцедуры\n#КонецОбласти\n";
    const PARSE_DELAY_MS: u64 = 1200;

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

    let uri = Url::parse("file:///test_p33_document_symbol_latest_ready_parse_gap.bsl")
        .expect("test uri");
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

    let seeded = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(response) =
                lsp_document_symbol_with_request(&mut service, 50_330, &uri).await
            {
                break response;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("seed documentSymbol response must arrive");
    let seeded_names = document_symbol_names(&seeded);
    assert!(
        seeded_names.iter().any(|name| name == "OldProc"),
        "seed documentSymbol response must expose OldProc, names={seeded_names:?}"
    );

    let _parse_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &PARSE_DELAY_MS.to_string(),
    );

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

    let file_id = server.get_or_create_file_id_v2(&uri).await;
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
    .expect("test must observe parse-snapshot gap before requesting latest_ready outline");

    let started = Instant::now();
    let response = tokio::time::timeout(
        Duration::from_millis(250),
        lsp_document_symbol_with_request(&mut service, 50_331, &uri),
    )
    .await
    .expect("documentSymbol latest_ready request must stay bounded")
    .expect("documentSymbol latest_ready response must be present");
    let elapsed = started.elapsed();
    let names = document_symbol_names(&response);
    assert!(
        names.iter().any(|name| name == "OldProc"),
        "latest_ready outline must serve cached previous structure while new revision is not ready, names={names:?}"
    );
    assert!(
        names.iter().all(|name| name != "NewProc"),
        "latest_ready outline must not masquerade as current revision before new snapshot is ready, names={names:?}"
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "latest_ready outline must stay bounded during parse-snapshot gap (elapsed={elapsed:?})"
    );

    let metrics = lsp_get_observability_metrics(&mut service, 50_399).await;
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(
            counters.get("intellisense_v2_document_symbol_outcome_total_outcome_latest_ready")
        ) > 0,
        "latest_ready outcome must be observable, counters={counters:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_document_symbol_returns_unavailable_before_ready_outline_from_did_open_gap() {
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

    const FIXTURE: &str =
        "#Область Public\nПроцедура OnlyProc() Экспорт\nКонецПроцедуры\n#КонецОбласти\n";
    const PARSE_DELAY_MS: u64 = 1200;

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

    let _parse_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DID_OPEN_BLOCKING_PARSE_DELAY_MS",
        &PARSE_DELAY_MS.to_string(),
    );
    let uri = Url::parse("file:///test_p33_document_symbol_unavailable_did_open_parse_gap.bsl")
        .expect("test uri");
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

    let started = Instant::now();
    let response = tokio::time::timeout(
        Duration::from_millis(250),
        lsp_document_symbol_with_request(&mut service, 50_329, &uri),
    )
    .await
    .expect("documentSymbol unavailable request must stay bounded");
    let elapsed = started.elapsed();
    assert!(
        response.is_none(),
        "documentSymbol must return unavailable while no ready outline exists during didOpen parse gap"
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "documentSymbol unavailable path must stay bounded during didOpen parse gap (elapsed={elapsed:?})"
    );

    let metrics = lsp_get_observability_metrics(&mut service, 50_328).await;
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(
            counters.get("intellisense_v2_document_symbol_outcome_total_outcome_unavailable")
        ) > 0,
        "unavailable outcome must be observable, counters={counters:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_document_symbol_request_bootstrap_materializes_ready_outline_after_did_open_gap() {
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

    const FIXTURE: &str =
        "#Область Public\nПроцедура OnlyProc() Экспорт\nКонецПроцедуры\n#КонецОбласти\n";
    const PARSE_DELAY_MS: u64 = 5_000;

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

    let _parse_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DID_OPEN_BLOCKING_PARSE_DELAY_MS",
        &PARSE_DELAY_MS.to_string(),
    );
    let uri = Url::parse("file:///test_p33_document_symbol_request_bootstrap_did_open_gap.bsl")
        .expect("test uri");
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

    let initial = tokio::time::timeout(
        Duration::from_millis(250),
        lsp_document_symbol_with_request(&mut service, 50_332, &uri),
    )
    .await
    .expect("initial documentSymbol request must stay bounded");
    assert!(
        initial.is_none(),
        "initial request must still fail closed while no ready outline exists during didOpen parse gap"
    );

    let seeded = tokio::time::timeout(Duration::from_secs(2), async {
        let mut request_id = 50_333;
        loop {
            if let Some(response) = lsp_document_symbol_with_request(&mut service, request_id, &uri)
                .await
            {
                break response;
            }
            request_id += 1;
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("request-triggered bootstrap must materialize ready outline before slow didOpen parse snapshot finishes");
    let seeded_names = document_symbol_names(&seeded);
    assert!(
        seeded_names.iter().any(|name| name == "OnlyProc"),
        "bootstrap response must expose initial outline, names={seeded_names:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_live_transport_document_symbol_request_bootstrap_materializes_ready_outline_after_did_open_gap(
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

    const FIXTURE: &str =
        "#Область Public\nПроцедура OnlyProc() Экспорт\nКонецПроцедуры\n#КонецОбласти\n";
    const PARSE_DELAY_MS: u64 = 5_000;

    let _env_lock = lock_test_env().await;

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let _parse_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DID_OPEN_BLOCKING_PARSE_DELAY_MS",
        &PARSE_DELAY_MS.to_string(),
    );
    let uri = Url::parse("file:///test_p33_live_transport_document_symbol_request_bootstrap.bsl")
        .expect("test uri");
    harness
        .send_notification(
            "textDocument/didOpen",
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: FIXTURE.to_string(),
                },
            },
        )
        .await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didOpen must publish latest received version on live transport");

    let initial = tokio::time::timeout(
        Duration::from_millis(250),
        harness.send_request(
            50_334,
            "textDocument/documentSymbol",
            DocumentSymbolParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        ),
    )
    .await
    .expect("initial live transport documentSymbol request must stay bounded");
    assert!(
        document_symbol_response_from_jsonrpc_response(&initial).is_none(),
        "initial live transport request must still fail closed while no ready outline exists during didOpen parse gap"
    );

    let seeded = tokio::time::timeout(Duration::from_secs(2), async {
        let mut request_id = 50_335;
        loop {
            let response = harness
                .send_request(
                    request_id,
                    "textDocument/documentSymbol",
                    DocumentSymbolParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    },
                )
                .await;
            if let Some(parsed) = document_symbol_response_from_jsonrpc_response(&response) {
                break parsed;
            }
            request_id += 1;
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("live transport request-triggered bootstrap must materialize ready outline before slow didOpen parse snapshot finishes");
    let seeded_names = document_symbol_names(&seeded);
    assert!(
        seeded_names.iter().any(|name| name == "OnlyProc"),
        "live transport bootstrap response must expose initial outline, names={seeded_names:?}"
    );

    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_document_symbol_supersedes_older_outstanding_refresh() {
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

    const V1_FIXTURE: &str =
        "#Область Public\nПроцедура OldProc() Экспорт\nКонецПроцедуры\n#КонецОбласти\n";
    const V2_FIXTURE: &str =
        "#Область Public\nПроцедура NewProc() Экспорт\nКонецПроцедуры\n#КонецОбласти\n";
    const PARSE_DELAY_MS: u64 = 1200;

    let _env_lock = lock_test_env().await;

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_document_symbol_superseded_refresh.bsl").expect("uri");
    harness
        .send_notification(
            "textDocument/didOpen",
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: V1_FIXTURE.to_string(),
                },
            },
        )
        .await;

    let seeded = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let response = harness
                .send_request(
                    50_340,
                    "textDocument/documentSymbol",
                    DocumentSymbolParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    },
                )
                .await;
            if let Some(result) = response.get("result").cloned() {
                let parsed: Option<DocumentSymbolResponse> =
                    serde_json::from_value(result).expect("parse seeded documentSymbol response");
                if let Some(response) = parsed {
                    break response;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("seed documentSymbol response must arrive");
    let seeded_names = document_symbol_names(&seeded);
    assert!(
        seeded_names.iter().any(|name| name == "OldProc"),
        "seed documentSymbol response must expose OldProc, names={seeded_names:?}"
    );

    let _delay_guard = EnvVarGuard::set("BSL_TEST_DOCUMENT_SYMBOL_DELAY_MS", "300");
    let _parse_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &PARSE_DELAY_MS.to_string(),
    );
    harness
        .write_message(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 50_341,
            "method": "textDocument/documentSymbol",
            "params": DocumentSymbolParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        }))
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    harness
        .send_notification(
            "textDocument/didChange",
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: V2_FIXTURE.to_string(),
                }],
            },
        )
        .await;

    harness
        .write_message(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 50_342,
            "method": "textDocument/documentSymbol",
            "params": DocumentSymbolParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        }))
        .await;
    let (first_response, second_response) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut first_response = None;
        let mut second_response = None;
        loop {
            let response = harness.read_message().await;
            match response.get("id").and_then(|value| value.as_i64()) {
                Some(50_341) => first_response = Some(response),
                Some(50_342) => second_response = Some(response),
                _ => {}
            }
            if first_response.is_some() && second_response.is_some() {
                break (
                    first_response.take().expect("first response"),
                    second_response.take().expect("second response"),
                );
            }
        }
    })
    .await
    .expect("both documentSymbol responses must arrive");
    let second_result = second_response
        .get("result")
        .cloned()
        .expect("second documentSymbol result field");
    let second_parsed: Option<DocumentSymbolResponse> =
        serde_json::from_value(second_result).expect("parse second documentSymbol response");
    let second_parsed = second_parsed.expect("second documentSymbol response must be present");
    let second_names = document_symbol_names(&second_parsed);
    assert!(
        second_names.iter().any(|name| name == "OldProc"),
        "newer outline refresh should still return bounded latest_ready response, names={second_names:?}"
    );

    assert!(
        first_response
            .get("result")
            .is_some_and(|value| value.is_null()),
        "older outstanding outline refresh must be superseded once a newer refresh arrives, response={first_response:?}"
    );

    let metrics = live_transport_get_observability_metrics(&mut harness, 50_343).await;
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(
            counters.get("intellisense_v2_document_symbol_outcome_total_outcome_superseded")
        ) > 0,
        "superseded outcome must be observable, counters={counters:?}"
    );

    live_transport_close_document(&mut harness, &uri).await;
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_document_symbol_burst_does_not_delay_completion_first_poll_under_parse_gap() {
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

    const V1_FIXTURE: &str = "Процедура Тест()\n    ДляCompletion = Объект.\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "#Область Public\nПроцедура AddedProc() Экспорт\nКонецПроцедуры\n#КонецОбласти\nПроцедура Тест()\n    ДляCompletion = Объект.\nКонецПроцедуры\n";
    const PARSE_DELAY_MS: u64 = 1200;
    const FIRST_POLL_BUDGET_MS: u64 = 200;
    const SATURATING_REQUESTS: i64 = 4;

    let _env_lock = lock_test_env().await;

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri =
        Url::parse("file:///test_p33_document_symbol_burst_first_poll_parse_gap.bsl").expect("uri");
    harness
        .send_notification(
            "textDocument/didOpen",
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: V1_FIXTURE.to_string(),
                },
            },
        )
        .await;
    let seeded = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let response = harness
                .send_request(
                    50_350,
                    "textDocument/documentSymbol",
                    DocumentSymbolParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    },
                )
                .await;
            if let Some(result) = response.get("result").cloned() {
                let parsed: Option<DocumentSymbolResponse> =
                    serde_json::from_value(result).expect("parse seeded documentSymbol response");
                if let Some(response) = parsed {
                    break response;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("seed documentSymbol response must arrive");
    let seeded_names = document_symbol_names(&seeded);
    assert!(
        seeded_names.iter().any(|name| name == "Тест"),
        "seed documentSymbol response must expose initial outline, names={seeded_names:?}"
    );

    let _parse_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &PARSE_DELAY_MS.to_string(),
    );
    harness
        .send_notification(
            "textDocument/didChange",
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: V2_FIXTURE.to_string(),
                }],
            },
        )
        .await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if server
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
    .expect("test must observe current revision handoff before outline burst");
    tokio::time::sleep(Duration::from_millis(50)).await;

    for request_id in 0..SATURATING_REQUESTS {
        harness
            .write_message(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 50_360 + request_id,
                "method": "textDocument/documentSymbol",
                "params": DocumentSymbolParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                },
            }))
            .await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;

    let completion_position =
        find_utf16_position_after_marker(V2_FIXTURE, "ДляCompletion = Объект.");
    let completion_response = harness
        .send_request(
            50_399,
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
        "completion request under outline burst must still complete"
    );

    let trace = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let timeline = live_transport_get_completion_timeline(&mut harness, 50_398, 32).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("completion timeline traces array");
            if let Some(trace) = traces.iter().find(|trace| {
                trace.get("request_id").and_then(|value| value.as_str()) == Some("50399")
            }) {
                break trace.clone();
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("completion trace must appear in timeline");

    let service_future_to_first_poll_wait_ms =
        completion_timeline_server_edge_u64(&trace, "service_future_to_first_poll_wait_ms")
            .expect("service_future_to_first_poll_wait_ms");
    assert!(
        service_future_to_first_poll_wait_ms <= FIRST_POLL_BUDGET_MS,
        "outline burst must not delay completion before first poll under parse-snapshot gap, trace={trace:?}"
    );

    let metrics = live_transport_get_observability_metrics(&mut harness, 50_397).await;
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(
            counters.get("intellisense_v2_document_symbol_outcome_total_outcome_latest_ready")
        ) > 0,
        "outline burst gate must observe latest_ready outcomes while current revision is not ready, counters={counters:?}"
    );

    live_transport_close_document(&mut harness, &uri).await;
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_live_transport_changed_text_current_revision_survives_document_symbol_backlog() {
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
    const DOCUMENT_SYMBOL_DELAY_MS: u64 = 300;
    const DOCUMENT_SYMBOL_BURST_REQUESTS: i64 = 48;

    let _env_lock = lock_test_env().await;
    let _document_symbol_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DOCUMENT_SYMBOL_DELAY_MS",
        &DOCUMENT_SYMBOL_DELAY_MS.to_string(),
    );
    let _blocking_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "1500");

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_live_transport_changed_text_backlog_priority.bsl")
        .expect("uri");
    harness
        .send_notification(
            "textDocument/didOpen",
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: V1_FIXTURE.to_string(),
                },
            },
        )
        .await;

    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didOpen must publish latest received version on live transport");

    for request_id in 0..DOCUMENT_SYMBOL_BURST_REQUESTS {
        live_transport_write_document_symbol_request(&mut harness, 50_520 + request_id, &uri).await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;

    harness
        .send_notification(
            "textDocument/didChange",
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: V2_FIXTURE.to_string(),
                }],
            },
        )
        .await;

    let completion_labels = live_transport_completion_labels_with_request(
        &mut harness,
        50_620,
        &uri,
        find_utf16_position_after_marker(V2_FIXTURE, "ДляCompletion = S."),
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;
    assert!(
        completion_labels.iter().any(|label| label == "Описание"),
        "changed-text completion on the default transport path must still see the latest didChange before unrelated documentSymbol backlog, labels={completion_labels:?}"
    );

    tokio::time::timeout(Duration::from_secs(5), async {
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
    .expect("didChange under documentSymbol backlog must still publish changed current revision on the live transport path");

    live_transport_close_document(&mut harness, &uri).await;
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_document_symbol_burst_does_not_delay_hover_signature_help_or_definition_under_parse_gap(
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

    const V1_FIXTURE: &str = "Процедура Тест()\n\
МассивДляHover = Новый Массив;\n\
ЗначДляHover = МассивДляHover.Количество();\n\
\n\
МассивДляSignature = Новый Массив;\n\
МассивДляSignature.Количество();\n\
\n\
МассивДляDefinition = Новый Массив;\n\
МассивДляDefinition.Количество();\n\
КонецПроцедуры\n";
    const APPENDED_OUTLINE: &str =
        "\n#Область Public\nПроцедура AddedProc() Экспорт\nКонецПроцедуры\n#КонецОбласти\n";
    const PARSE_DELAY_MS: u64 = 1200;
    const DOCUMENT_SYMBOL_DELAY_MS: u64 = 300;
    const INTERACTIVE_BUDGET_MS: u64 = 250;
    const SATURATING_REQUESTS: i64 = 4;

    let _env_lock = lock_test_env().await;

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse(
        "file:///test_p33_document_symbol_burst_hover_signature_definition_parse_gap.bsl",
    )
    .expect("uri");
    harness
        .send_notification(
            "textDocument/didOpen",
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: V1_FIXTURE.to_string(),
                },
            },
        )
        .await;
    let seeded = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let response = harness
                .send_request(
                    50_352,
                    "textDocument/documentSymbol",
                    DocumentSymbolParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    },
                )
                .await;
            if let Some(response) = document_symbol_response_from_jsonrpc_response(&response) {
                break response;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("seed documentSymbol response must arrive");
    let seeded_names = document_symbol_names(&seeded);
    assert!(
        seeded_names.iter().any(|name| name == "Тест"),
        "seed documentSymbol response must expose initial outline, names={seeded_names:?}"
    );

    let did_change_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &PARSE_DELAY_MS.to_string(),
    );
    let did_save_guard = EnvVarGuard::set(
        "BSL_TEST_DID_SAVE_PARSE_DELAY_MS",
        &PARSE_DELAY_MS.to_string(),
    );
    let document_symbol_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DOCUMENT_SYMBOL_DELAY_MS",
        &DOCUMENT_SYMBOL_DELAY_MS.to_string(),
    );
    let v2_fixture = format!("{V1_FIXTURE}{APPENDED_OUTLINE}");
    harness
        .send_notification(
            "textDocument/didChange",
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: v2_fixture.clone(),
                }],
            },
        )
        .await;
    live_transport_save_document(&mut harness, &uri).await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(2)
                && (super::super::language_server::did_change_inline_parse_delay_active_for_test()
                    || super::super::language_server::did_save_inline_parse_delay_active_for_test())
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("test must observe parse-snapshot gap before outline burst");

    let hover_position =
        find_utf16_position_after_marker(&v2_fixture, "ЗначДляHover = МассивДляHover");
    let signature_position =
        find_utf16_position_after_marker(&v2_fixture, "МассивДляSignature.Количество(");
    let definition_position =
        find_utf16_position_after_marker(&v2_fixture, "МассивДляDefinition.Количество");

    let hover_request_ids = (0..SATURATING_REQUESTS)
        .map(|request_index| 50_361 + request_index)
        .collect::<Vec<_>>();
    for request_id in &hover_request_ids {
        live_transport_write_document_symbol_request(&mut harness, *request_id, &uri).await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    let hover_started = Instant::now();
    let hover_response = tokio::time::timeout(
        Duration::from_millis(INTERACTIVE_BUDGET_MS),
        harness.send_request(
            50_369,
            "textDocument/hover",
            HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: hover_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        ),
    )
    .await
    .expect("hover under outline burst must stay bounded");
    assert!(
        hover_response.get("result").is_some(),
        "hover request under outline burst must still return a response envelope"
    );
    let hover_trace = take_test_request_server_edge_trace(50_369).await;
    assert_request_first_poll_budget(&hover_trace, "textDocument/hover", INTERACTIVE_BUDGET_MS);
    assert!(
        hover_started.elapsed() <= Duration::from_millis(INTERACTIVE_BUDGET_MS),
        "outline burst must not delay hover on the current revision path"
    );

    let signature_request_ids = (0..SATURATING_REQUESTS)
        .map(|request_index| 50_371 + request_index)
        .collect::<Vec<_>>();
    for request_id in &signature_request_ids {
        live_transport_write_document_symbol_request(&mut harness, *request_id, &uri).await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    let signature_started = Instant::now();
    let signature_response = tokio::time::timeout(
        Duration::from_millis(INTERACTIVE_BUDGET_MS),
        harness.send_request(
            50_379,
            "textDocument/signatureHelp",
            tower_lsp::lsp_types::SignatureHelpParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: signature_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                context: None,
            },
        ),
    )
    .await
    .expect("signatureHelp under outline burst must stay bounded");
    assert!(
        signature_response.get("result").is_some(),
        "signatureHelp under outline burst must still return a response envelope"
    );
    let signature_trace = take_test_request_server_edge_trace(50_379).await;
    assert_request_first_poll_budget(
        &signature_trace,
        "textDocument/signatureHelp",
        INTERACTIVE_BUDGET_MS,
    );
    assert!(
        signature_started.elapsed() <= Duration::from_millis(INTERACTIVE_BUDGET_MS),
        "outline burst must not delay signatureHelp on the current revision path"
    );

    let definition_request_ids = (0..SATURATING_REQUESTS)
        .map(|request_index| 50_381 + request_index)
        .collect::<Vec<_>>();
    for request_id in &definition_request_ids {
        live_transport_write_document_symbol_request(&mut harness, *request_id, &uri).await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    let definition_started = Instant::now();
    let definition_response = tokio::time::timeout(
        Duration::from_millis(INTERACTIVE_BUDGET_MS),
        harness.send_request(
            50_389,
            "textDocument/definition",
            GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: definition_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        ),
    )
    .await
    .expect("definition under outline burst must stay bounded");
    assert!(
        definition_response.get("result").is_some(),
        "definition under outline burst must still return a response envelope"
    );
    let definition_trace = take_test_request_server_edge_trace(50_389).await;
    assert_request_first_poll_budget(
        &definition_trace,
        "textDocument/definition",
        INTERACTIVE_BUDGET_MS,
    );
    assert!(
        definition_started.elapsed() <= Duration::from_millis(INTERACTIVE_BUDGET_MS),
        "outline burst must not delay definition on the current revision path"
    );

    let metrics = live_transport_get_observability_metrics(&mut harness, 50_390).await;
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(
            counters.get("intellisense_v2_document_symbol_outcome_total_outcome_latest_ready")
        ) + read_u64_metric(
            counters.get("intellisense_v2_document_symbol_outcome_total_outcome_unavailable")
        ) > 0,
        "outline burst must keep documentSymbol on a bounded auxiliary path while current revision is not ready, counters={counters:?}"
    );

    drop(document_symbol_delay_guard);
    drop(did_save_guard);
    drop(did_change_guard);
    live_transport_close_document(&mut harness, &uri).await;
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_did_save_coalesces_same_version_outline_refresh_on_default_path() {
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

    const V1_FIXTURE: &str =
        "#Область Public\nПроцедура OldProc() Экспорт\nКонецПроцедуры\n#КонецОбласти\n";
    const V2_FIXTURE: &str =
        "#Область Public\nПроцедура NewProc() Экспорт\nКонецПроцедуры\n#КонецОбласти\n";
    const PARSE_DELAY_MS: u64 = 1200;

    let _env_lock = lock_test_env().await;

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_document_symbol_did_save_same_version_refresh.bsl")
        .expect("uri");
    harness
        .send_notification(
            "textDocument/didOpen",
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: V1_FIXTURE.to_string(),
                },
            },
        )
        .await;
    let seeded = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let response = harness
                .send_request(
                    50_392,
                    "textDocument/documentSymbol",
                    DocumentSymbolParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    },
                )
                .await;
            if let Some(response) = document_symbol_response_from_jsonrpc_response(&response) {
                break response;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("seed documentSymbol response must arrive");
    let seeded_names = document_symbol_names(&seeded);
    assert!(
        seeded_names.iter().any(|name| name == "OldProc"),
        "seed documentSymbol response must expose OldProc, names={seeded_names:?}"
    );

    let did_change_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &PARSE_DELAY_MS.to_string(),
    );
    let did_save_guard = EnvVarGuard::set(
        "BSL_TEST_DID_SAVE_PARSE_DELAY_MS",
        &PARSE_DELAY_MS.to_string(),
    );
    harness
        .send_notification(
            "textDocument/didChange",
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: V2_FIXTURE.to_string(),
                }],
            },
        )
        .await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
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
    .expect("didChange parse-snapshot gap must become observable before didSave");

    live_transport_save_document(&mut harness, &uri).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        super::super::language_server::did_change_inline_parse_delay_active_for_test(),
        "didChange same-version parse gap must still be active when didSave arrives"
    );
    assert!(
        !super::super::language_server::did_save_inline_parse_delay_active_for_test(),
        "didSave must coalesce behind the in-flight same-version refresh instead of spawning a second delayed parse worker"
    );

    let response = tokio::time::timeout(
        Duration::from_millis(250),
        harness.send_request(
            50_393,
            "textDocument/documentSymbol",
            DocumentSymbolParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        ),
    )
    .await
    .expect("documentSymbol during didSave parse gap must stay bounded");
    let parsed = document_symbol_response_from_jsonrpc_response(&response)
        .expect("didSave parse gap should still serve latest_ready outline");
    let names = document_symbol_names(&parsed);
    assert!(
        names.iter().any(|name| name == "OldProc"),
        "coalesced same-version outline refresh must keep serving the previous ready cache while rebuilding, names={names:?}"
    );
    assert!(
        names.iter().all(|name| name != "NewProc"),
        "coalesced same-version outline refresh must not masquerade as the new revision before refresh completes, names={names:?}"
    );

    drop(did_change_guard);
    drop(did_save_guard);

    let refreshed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let response = harness
                .send_request(
                    50_394,
                    "textDocument/documentSymbol",
                    DocumentSymbolParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    },
                )
                .await;
            if let Some(parsed) = document_symbol_response_from_jsonrpc_response(&response) {
                let names = document_symbol_names(&parsed);
                if names.iter().any(|name| name == "NewProc") {
                    break names;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("coalesced same-version outline refresh must eventually materialize");
    assert!(
        refreshed.iter().any(|name| name == "NewProc"),
        "coalesced same-version refresh must eventually expose the new outline, names={refreshed:?}"
    );

    live_transport_close_document(&mut harness, &uri).await;
    harness.shutdown().await;
}
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_current_revision_head_precompute_stays_available_under_background_cpu_saturation() {
    const V1_FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\n    ДляCompletion = S.\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";

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

    let total_cpu_permits = std::thread::available_parallelism()
        .map(|parallelism| parallelism.get().max(2))
        .unwrap_or(4);
    let interactive_reserved = if total_cpu_permits >= 4 { 2 } else { 1 };
    let background_blocker_count = total_cpu_permits
        .saturating_sub(interactive_reserved)
        .max(1);
    let mut blocker_handles = Vec::new();
    let mut blocker_started = Vec::new();
    for _ in 0..background_blocker_count {
        let (started_tx, started_rx) = tokio::sync::oneshot::channel::<()>();
        blocker_handles.push(tokio::spawn(async move {
            bsl_runtime::application::spawn_bounded_blocking_with_class(
                bsl_runtime::application::CpuWorkClass::Background,
                move || {
                    let _ = started_tx.send(());
                    std::thread::sleep(Duration::from_millis(400));
                },
            )
            .await
            .expect("background blocker join");
        }));
        blocker_started.push(started_rx);
    }
    for started_rx in blocker_started {
        started_rx
            .await
            .expect("background blocker should acquire non-interactive CPU permit");
    }

    let uri = Url::parse("file:///test_p33_current_revision_head_under_background_saturation.bsl")
        .expect("uri");
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
    let file_id = server.get_or_create_file_id_v2(&uri).await;

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
    .expect("current revision apply must reach version 2 before head-fast-lane measurement");

    let completion_position = find_utf16_position_after_marker(V2_FIXTURE, "ДляCompletion = S.");
    let head_started = Instant::now();
    let head_owner_hints = tokio::time::timeout(Duration::from_millis(250), async {
        loop {
            let analysis = server.analysis_v2.snapshot().await;
            let head_ready = analysis
                .current_completion_head_ready(file_id)
                .ok()
                .unwrap_or(false);
            let Some(file_text) = analysis.file_text(file_id).ok().flatten() else {
                tokio::task::yield_now().await;
                continue;
            };
            let owner_hints = bsl_runtime::application::completion_member_access_owner_type_hints_from_completion_head(
                &analysis,
                file_id,
                file_text.as_ref(),
                completion_position.line,
                completion_position.character,
            );
            if head_ready && !owner_hints.is_empty() {
                break owner_hints;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("current-revision completion head must stay available under background CPU saturation");
    let head_elapsed = head_started.elapsed();
    assert!(
        head_elapsed < Duration::from_millis(250),
        "current-revision completion head must stay bounded under background CPU saturation (elapsed={head_elapsed:?}, owner_hints={head_owner_hints:?}, blockers={background_blocker_count}, total_permits={total_cpu_permits})"
    );

    let started = Instant::now();
    let completion_response = tokio::time::timeout(
        Duration::from_millis(1200),
        server.completion(CompletionParams {
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
        }),
    )
    .await
    .expect("completion should eventually finish after head fast-lane availability under background CPU saturation")
    .expect("completion request")
    .expect("completion response");
    let elapsed = started.elapsed();
    let completion_labels: Vec<String> = match completion_response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    };
    assert!(
        completion_labels.iter().any(|label| label == "Количество"),
        "current-revision head must remain available under background CPU saturation, labels={completion_labels:?}"
    );
    assert!(
        elapsed < Duration::from_millis(1200),
        "completion should eventually resolve after current-revision head becomes available (elapsed={elapsed:?}, blockers={background_blocker_count}, total_permits={total_cpu_permits})"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 4064, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("completion trace under background CPU saturation");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "background CPU saturation must still resolve completion through current-revision head route, trace={trace:?}"
    );
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "kind"),
        Some("lightweight_current_revision"),
        "background CPU saturation must route head-hit completion through the lightweight current-revision prepare boundary, trace={trace:?}"
    );
    assert_ne!(
        completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
        Some("exact_deadline"),
        "background CPU saturation must not regress into exact_deadline when current-revision head should take the fast lane, trace={trace:?}"
    );

    for blocker_handle in blocker_handles {
        blocker_handle.await.expect("background blocker task");
    }

    drain_task.abort();
}
