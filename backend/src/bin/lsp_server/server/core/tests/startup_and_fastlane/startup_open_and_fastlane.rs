#[test]
fn diagnostics_debounce_floor_prevents_zero_ms_tight_loops() {
    assert_eq!(clamp_diagnostics_debounce_ms(0), 25);
    assert_eq!(clamp_diagnostics_debounce_ms(1), 25);
    assert_eq!(clamp_diagnostics_debounce_ms(25), 25);
    assert_eq!(clamp_diagnostics_debounce_ms(250), 250);
}

#[tokio::test]
async fn p34_initialized_with_startup_config_returns_without_waiting_for_startup() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) =
        LspService::build(move |client| BslLanguageServer::new(client, coordinator.clone()))
            .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let syntax_helper_path = syntax_helper_path_for_tests();
    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities {
            window: Some(tower_lsp::lsp_types::WindowClientCapabilities {
                work_done_progress: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        },
        initialization_options: Some(serde_json::json!({
            "platformDocsArchive": syntax_helper_path.to_string_lossy(),
            "platformVersion": "8.3.25",
            "cacheEnabled": true,
            "enableTypeHints": false,
            "enableCodeActions": false
        })),
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
    let initialized_response = tokio::time::timeout(std::time::Duration::from_millis(200), async {
        service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification")
    })
    .await
    .expect("initialized must return without waiting for startup");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let stats_request = Request::build("workspace/executeCommand")
        .id(2)
        .params(serde_json::json!({
            "command": "bsl.getTypeRepositoryStats",
            "arguments": [{}]
        }))
        .finish();
    let stats_response = tokio::time::timeout(std::time::Duration::from_millis(200), async {
        service
            .ready()
            .await
            .unwrap()
            .call(stats_request)
            .await
            .expect("bsl.getTypeRepositoryStats request")
    })
    .await
    .expect("interactive command must stay responsive during startup");
    assert!(
        stats_response.is_some(),
        "executeCommand should return a response"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p35_large_conf_big_did_open_returns_promptly() {
    let conf_big_module = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join("conf_big")
        .join("Documents")
        .join("РеализацияТоваровУслуг")
        .join("Forms")
        .join("ФормаДокументаОбщая")
        .join("Ext")
        .join("Form")
        .join("Module.bsl");
    if !conf_big_module.exists() {
        eprintln!(
            "skipping p35_large_conf_big_did_open_returns_promptly: missing fixture {}",
            conf_big_module.display()
        );
        return;
    }

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) =
        LspService::build(move |client| BslLanguageServer::new(client, coordinator.clone()))
            .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;
    let mut service = crate::server::request_context::DispatchContextService::new(
        crate::server::request_context::RequestContextService::new(service),
    );

    let uri = Url::from_file_path(&conf_big_module).expect("conf_big module uri");
    let text = std::fs::read_to_string(&conf_big_module).expect("read conf_big module");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri,
            language_id: "bsl".to_string(),
            version: 1,
            text,
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification")
    })
    .await
    .expect("didOpen on large conf_big module must return promptly");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    drain_task.abort();
}

#[tokio::test]
async fn p33_did_open_returns_before_blocking_parse_snapshot_finishes() {
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

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let _blocking_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_OPEN_BLOCKING_PARSE_DELAY_MS", "1500");

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

    let uri = Url::parse("file:///test_p33_did_open_prompt_return.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
        },
    };
    let started = Instant::now();
    let did_open_response = tokio::time::timeout(Duration::from_millis(250), async {
        service
            .ready()
            .await
            .unwrap()
            .call(
                Request::build("textDocument/didOpen")
                    .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                    .finish(),
            )
            .await
            .expect("didOpen notification")
    })
    .await
    .expect("didOpen must return before blocking parse snapshot completes");
    let elapsed = started.elapsed();
    assert!(did_open_response.is_none(), "didOpen is a notification");
    assert!(
        elapsed < Duration::from_millis(250),
        "didOpen must stay short-lived under blocking parse snapshot delay (elapsed={elapsed:?})"
    );

    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    tokio::time::timeout(Duration::from_millis(100), async {
        loop {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(1)
                && server
                    .analysis_v2
                    .file_revision_state(file_id)
                    .await
                    .map(|state| state.version)
                    == Some(1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didOpen must complete after current-revision handoff");

    drain_task.abort();
}

#[tokio::test]
async fn p6_fast_did_change_series_publish_diagnostics_is_monotonic() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let (published_tx, mut published_rx) =
        tokio::sync::mpsc::unbounded_channel::<tower_lsp::lsp_types::PublishDiagnosticsParams>();

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

    // LSP initialize handshake is required, otherwise client notifications are suppressed.
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

    let uri = Url::parse("file:///test.bsl").expect("test uri");

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

    // Two fast didChange events with different versions. We want to ensure that the server
    // never publishes diagnostics for an older version after a newer one is published.
    let did_change_v2 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "Procedure Test(\nEndProcedure".to_string(),
        }],
    };
    let did_change_req_v2 = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change_v2).expect("DidChangeTextDocumentParams v2"))
        .finish();
    let did_change_response_v2 = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req_v2)
        .await
        .expect("didChange v2 notification");
    assert!(
        did_change_response_v2.is_none(),
        "didChange is a notification"
    );

    let did_change_v3 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 3,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "Procedure Test()\nEndProcedure".to_string(),
        }],
    };
    let did_change_req_v3 = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change_v3).expect("DidChangeTextDocumentParams v3"))
        .finish();
    let did_change_response_v3 = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req_v3)
        .await
        .expect("didChange v3 notification");
    assert!(
        did_change_response_v3.is_none(),
        "didChange is a notification"
    );

    let mut versions = Vec::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(2);

    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let Ok(next) = tokio::time::timeout(remaining, published_rx.recv()).await else {
            break;
        };
        let Some(params) = next else {
            break;
        };
        if params.uri != uri {
            continue;
        }
        let Some(version) = params.version else {
            continue;
        };

        versions.push(version);
        if version == 3 {
            break;
        }
    }

    assert!(
        versions.contains(&3),
        "expected diagnostics for version 3 to be published, got {:?}",
        versions
    );

    for pair in versions.windows(2) {
        assert!(
            pair[1] >= pair[0],
            "publishDiagnostics versions must not go backwards: {:?}",
            versions
        );
    }

    // After observing version 3, ensure we don't later publish version 1/2 (no jump-back).
    let after_deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(300);
    while tokio::time::Instant::now() < after_deadline {
        let remaining = after_deadline.saturating_duration_since(tokio::time::Instant::now());
        let Ok(next) = tokio::time::timeout(remaining, published_rx.recv()).await else {
            break;
        };
        let Some(params) = next else {
            break;
        };
        if params.uri != uri {
            continue;
        }
        let Some(version) = params.version else {
            continue;
        };
        assert!(
            version >= 3,
            "unexpected jump-back diagnostics: v{}",
            version
        );
    }

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let saw_stale_or_cancelled = counters.iter().any(|(key, value)| {
        key.starts_with("intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_")
            && (key.contains("reason_superseded_version")
                || key.contains("reason_superseded_generation"))
            && metric_number(value) > 0.0
    });
    assert!(
        saw_stale_or_cancelled
            || counters.iter().any(|(key, value)| {
                key.starts_with("intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_")
                    && key.contains("reason_other_cancel")
                    && metric_number(value) > 0.0
            }),
        "expected diagnostics pipeline metrics to record stale/cancelled runs after rapid didChange series"
    );
    let saw_did_change_fast_profile = counters.iter().any(|(key, value)| {
        key.starts_with(
            "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_did_change_profile_fast_",
        ) && metric_number(value) > 0.0
    });
    assert!(
        saw_did_change_fast_profile,
        "expected didChange traffic to execute fast diagnostics profile"
    );
    let saw_did_change_idle_heavy_profile = counters.iter().any(|(key, value)| {
        key.starts_with(
            "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_did_change_profile_idle_heavy_",
        ) && metric_number(value) > 0.0
    });
    assert!(
        !saw_did_change_idle_heavy_profile,
        "idle_heavy diagnostics must not execute under trigger_did_change"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p6_did_save_fastlane_publishes_same_version_syntax_diagnostics_before_delayed_apply() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
        reload_runtime_config: bool,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            Self::set_with_reload(key, value, false)
        }

        fn set_with_reload(key: &'static str, value: &str, reload_runtime_config: bool) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            if reload_runtime_config {
                bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
            }
            Self {
                key,
                previous,
                reload_runtime_config,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
            if self.reload_runtime_config {
                bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
            }
        }
    }

    const V1_FIXTURE: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест(\n    Возврат 1;\nКонецПроцедуры\n";
    const FIRST_PUBLISH_BUDGET_MS: u64 = 1_800;

    let _env_lock = lock_test_env().await;
    let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "4000");
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "1200", true);

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
            let Ok(parsed) = serde_json::from_value::<PublishDiagnosticsParams>(params) else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///did_save_fastlane_same_version_fixture.bsl").expect("fixture");
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

    let did_save_started = Instant::now();
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

    let first_publish = tokio::time::timeout(
        Duration::from_millis(FIRST_PUBLISH_BUDGET_MS),
        async {
            loop {
                let params = published_rx
                    .recv()
                    .await
                    .expect("publishDiagnostics channel must stay open");
                if params.uri != uri {
                    continue;
                }
                if params.version != Some(2) {
                    panic!(
                        "didSave fastlane must not publish stale diagnostics after save, got version={:?}",
                        params.version
                    );
                }
                if params.diagnostics.is_empty() {
                    continue;
                }
                break params;
            }
        },
    )
    .await
    .expect("didSave first publish must stay bounded under delayed apply");

    let first_publish_elapsed = did_save_started.elapsed();
    assert!(
        first_publish_elapsed <= Duration::from_millis(FIRST_PUBLISH_BUDGET_MS),
        "didSave first publish must stay bounded under delayed apply (elapsed={first_publish_elapsed:?})"
    );
    assert!(
        first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax")),
        "save fastlane must stay syntax-only for same-version first publish, diagnostics={:?}",
        first_publish.diagnostics
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
    assert!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_did_save_profile_save_fastlane_reason_published"
            )
        ) > 0,
        "didSave fastlane publish must be observable via dedicated save_fastlane profile, counters={counters:?}"
    );
    assert!(
        histograms.contains_key(
            "intellisense_v2_diagnostics_pipeline_publish_ms_origin_lsp_trigger_did_save_profile_save_fastlane"
        ),
        "didSave fastlane publish latency histogram must be exported, histograms={:?}",
        histograms.keys().collect::<Vec<_>>()
    );

    drain_task.abort();
}

#[tokio::test]
async fn p6_did_save_fastlane_does_not_erase_richer_idle_heavy_publish() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
        reload_runtime_config: bool,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            Self::set_with_reload(key, value, false)
        }

        fn set_with_reload(key: &'static str, value: &str, reload_runtime_config: bool) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            if reload_runtime_config {
                bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
            }
            Self {
                key,
                previous,
                reload_runtime_config,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
            if self.reload_runtime_config {
                bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
            }
        }
    }

    const V1_FIXTURE: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    Сообщить(необъявленная);\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let _save_fastlane_delay_guard =
        EnvVarGuard::set("BSL_TEST_SAVE_FASTLANE_PARSE_DELAY_MS", "1200");
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);

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
            let Ok(parsed) = serde_json::from_value::<PublishDiagnosticsParams>(params) else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;

    let uri =
        Url::parse("file:///did_save_fastlane_monotonic_publish_fixture.bsl").expect("fixture");
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
    while published_rx.try_recv().is_ok() {}

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

    let first_publish = tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            let params = published_rx
                .recv()
                .await
                .expect("publishDiagnostics channel must stay open");
            if params.uri != uri || params.version != Some(2) {
                continue;
            }
            if !params.diagnostics.is_empty() {
                break params;
            }
        }
    })
    .await
    .expect("didSave must publish at least one same-version diagnostics payload");

    let heavy_publish = if first_publish
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.source.as_deref() == Some("bsl-analysis-v2"))
    {
        first_publish
    } else {
        tokio::time::timeout(Duration::from_secs(6), async {
            loop {
                let params = published_rx
                    .recv()
                    .await
                    .expect("publishDiagnostics channel must stay open");
                if params.uri != uri || params.version != Some(2) {
                    continue;
                }
                if params
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.source.as_deref() == Some("bsl-analysis-v2"))
                {
                    break params;
                }
            }
        })
        .await
        .expect("idle_heavy publish with semantic diagnostics must eventually arrive")
    };

    assert!(
        heavy_publish
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.source.as_deref() == Some("bsl-analysis-v2")),
        "didSave heavy publish must surface semantic diagnostics, diagnostics={:?}",
        heavy_publish.diagnostics
    );

    let regressing_publish = tokio::time::timeout(Duration::from_millis(2500), async {
        loop {
            let params = published_rx
                .recv()
                .await
                .expect("publishDiagnostics channel must stay open");
            if params.uri != uri || params.version != Some(2) {
                continue;
            }
            let syntax_only = !params.diagnostics.is_empty()
                && params
                    .diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax"));
            if params.diagnostics.is_empty() || syntax_only {
                break params;
            }
        }
    })
    .await
    .ok();
    assert!(
        regressing_publish.is_none(),
        "late save_fastlane publish must not erase richer same-generation idle_heavy diagnostics, publish={regressing_publish:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p6_did_save_fastlane_uses_ready_parse_snapshot_when_shadow_is_missing() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
        reload_runtime_config: bool,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            Self::set_with_reload(key, value, false)
        }

        fn set_with_reload(key: &'static str, value: &str, reload_runtime_config: bool) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            if reload_runtime_config {
                bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
            }
            Self {
                key,
                previous,
                reload_runtime_config,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
            if self.reload_runtime_config {
                bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
            }
        }
    }

    const V1_FIXTURE: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест(\n    Возврат 1;\nКонецПроцедуры\n";
    const FIRST_PUBLISH_BUDGET_MS: u64 = 400;

    let _env_lock = lock_test_env().await;
    let _save_fastlane_delay_guard =
        EnvVarGuard::set("BSL_TEST_SAVE_FASTLANE_PARSE_DELAY_MS", "1500");
    let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "1500");
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "1200", true);

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
            let Ok(parsed) = serde_json::from_value::<PublishDiagnosticsParams>(params) else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;
    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");

    let uri =
        Url::parse("file:///did_save_fastlane_ready_parse_snapshot_fixture.bsl").expect("fixture");
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

    let file_id = server
        .get_file_id_v2(&uri)
        .await
        .expect("file id after didOpen");
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
    .expect("didChange must materialize same-version ready parse snapshot");
    while published_rx.try_recv().is_ok() {}

    server
        .latest_document_shadow_state_v2
        .write()
        .await
        .remove(&file_id);

    let did_save_started = Instant::now();
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

    let first_publish =
        tokio::time::timeout(Duration::from_millis(FIRST_PUBLISH_BUDGET_MS), async {
            loop {
                let params = published_rx
                    .recv()
                    .await
                    .expect("publishDiagnostics channel must stay open");
                if params.uri != uri || params.version != Some(2) {
                    continue;
                }
                if params.diagnostics.is_empty() {
                    continue;
                }
                break params;
            }
        })
        .await
        .expect("ready parse snapshot fastlane must publish before delayed shadow/apply path");

    let first_publish_elapsed = did_save_started.elapsed();
    assert!(
        first_publish_elapsed <= Duration::from_millis(FIRST_PUBLISH_BUDGET_MS),
        "ready parse snapshot fastlane must stay bounded without shadow state (elapsed={first_publish_elapsed:?})"
    );
    assert!(
        first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax")),
        "ready parse snapshot fastlane must stay syntax-only, diagnostics={:?}",
        first_publish.diagnostics
    );

    drain_task.abort();
}
