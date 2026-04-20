#[tokio::test]
async fn p7_did_save_fastlane_followup_publishes_full_diagnostics_from_ready_artifacts_before_delayed_apply(
) {
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
    const FOLLOWUP_PUBLISH_BUDGET_MS: u64 = 5000;

    let _env_lock = lock_test_env().await;
    let _save_fastlane_delay_guard =
        EnvVarGuard::set("BSL_TEST_SAVE_FASTLANE_PARSE_DELAY_MS", "2200");
    let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "4000");
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

    let uri = Url::parse("file:///did_save_followup_ready_artifacts_fixture.bsl").expect("fixture");
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

    let file_id = server
        .get_file_id_v2(&uri)
        .await
        .expect("file id after didOpen");
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

    let first_publish_trace = tokio::time::timeout(Duration::from_millis(3500), async {
        loop {
            let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_706, 8).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces");
            if let Some(trace) = traces.iter().find(|trace| {
                trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                    && trace
                        .get("requested_version")
                        .and_then(|value| value.as_i64())
                        == Some(2)
                    && trace
                        .get("first_publish")
                        .and_then(|value| value.get("profile"))
                        .and_then(|value| value.as_str())
                        == Some("save_fastlane")
            }) {
                break trace.clone();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("didSave save_fastlane must publish first before ready-artifacts follow-up");
    assert_eq!(
        first_publish_trace
            .get("first_publish")
            .and_then(|value| value.get("publish_kind"))
            .and_then(|value| value.as_str()),
        Some("syntax_only"),
        "didSave first publish must remain save_fastlane syntax-only before ready-artifacts follow-up, trace={first_publish_trace:?}"
    );

    tokio::time::timeout(Duration::from_millis(FOLLOWUP_PUBLISH_BUDGET_MS), async {
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
    .expect("same-version ready artifacts must publish full follow-up before delayed apply");

    let followup_elapsed = did_save_started.elapsed();
    assert!(
        followup_elapsed <= Duration::from_millis(FOLLOWUP_PUBLISH_BUDGET_MS),
        "heavy follow-up must stay bounded when same-version ready artifacts already exist (elapsed={followup_elapsed:?})"
    );

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_707, 8).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("diagnostics save timeline traces");
    let trace = traces
        .iter()
        .find(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(2)
        })
        .expect("matching diagnostics save timeline trace");
    let full_publish = trace
        .get("followup_publish")
        .and_then(|value| value.as_object())
        .filter(|publish| {
            publish.get("profile").and_then(|value| value.as_str()) == Some("idle_heavy")
        })
        .or_else(|| {
            trace
                .get("first_publish")
                .and_then(|value| value.as_object())
                .filter(|publish| {
                    publish.get("profile").and_then(|value| value.as_str()) == Some("idle_heavy")
                })
        })
        .expect("idle_heavy full publish trace");
    assert_eq!(
        full_publish.get("profile").and_then(|value| value.as_str()),
        Some("idle_heavy")
    );
    assert_eq!(
        full_publish
            .get("publish_kind")
            .and_then(|value| value.as_str()),
        Some("full")
    );
    assert!(
        full_publish.get("wait_for_file_version_ms").is_none(),
        "ready-artifact follow-up must not expose wait_for_file_version as primary gate, trace={trace:?}"
    );
    assert_eq!(
        full_publish
            .get("syntax_work_mode")
            .and_then(|value| value.as_str()),
        Some("reused"),
        "ready-artifact follow-up must report syntax reuse explicitly, trace={trace:?}"
    );
    assert_eq!(
        full_publish
            .get("semantic_path")
            .and_then(|value| value.as_str()),
        Some("ready_artifacts"),
        "ready-artifact follow-up must publish explicit semantic path, trace={trace:?}"
    );
    assert_eq!(
        full_publish
            .get("semantic_parse_source")
            .and_then(|value| value.as_str()),
        Some("snapshot"),
        "ready-artifact follow-up must publish snapshot parse source, trace={trace:?}"
    );
    assert_eq!(
        full_publish
            .get("semantic_ir_source")
            .and_then(|value| value.as_str()),
        Some("snapshot_build"),
        "ready-artifact follow-up must publish snapshot-backed IR source, trace={trace:?}"
    );
    assert!(
        full_publish.get("syntax_diagnostics_query_ms").is_none(),
        "ready-artifact follow-up must not rerun full syntax diagnostics query when same-version syntax artifacts are already available, trace={trace:?}"
    );
    assert!(
        full_publish.get("semantic_diagnostics_query_ms").is_some(),
        "ready-artifact follow-up must expose semantic query timing, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("idle_heavy_outcome")
            .and_then(|value| value.as_str()),
        Some("published")
    );
    assert_eq!(
        trace
            .get("followup_semantic_path")
            .and_then(|value| value.as_str()),
        Some("ready_artifacts")
    );
    assert_eq!(
        trace
            .get("followup_semantic_parse_source")
            .and_then(|value| value.as_str()),
        Some("snapshot")
    );
    assert_eq!(
        trace
            .get("followup_semantic_ir_source")
            .and_then(|value| value.as_str()),
        Some("snapshot_build")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_zero_probe")
            .and_then(|value| value.as_str()),
        Some("ready")
    );
    assert!(
        trace.get("followup_ready_snapshot_wait_probe").is_none(),
        "successful zero-budget ready-snapshot probe must not fabricate bounded-wait attribution, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_task_state")
            .and_then(|value| value.as_str()),
        Some("ready_same_version")
    );
    assert_eq!(
        trace
            .get("followup_shadow_state_available")
            .and_then(|value| value.as_bool()),
        Some(true)
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_did_save_followup_prefers_inflight_same_version_ready_snapshot_before_shadow_state() {
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

    const CHANGE_ID: &str = "refactor-17-diagnostics-save-inflight-snapshot-preference";
    const V1_FIXTURE: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    Сообщить(необъявленная);\nКонецПроцедуры\n";
    const DID_CHANGE_PARSE_DELAY_MS: u64 = 1_200;
    const DID_SAVE_PARSE_DELAY_MS: u64 = 0;
    const APPLY_DELAY_MS: u64 = 4_000;
    const FIRST_PUBLISH_BUDGET_MS: u64 = 3_500;
    const FOLLOWUP_PUBLISH_BUDGET_MS: u64 = 5_000;

    let _env_lock = lock_test_env().await;
    let _did_change_parse_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS",
        &DID_CHANGE_PARSE_DELAY_MS.to_string(),
    );
    let _apply_delay_guard = EnvVarGuard::set(
        "BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS",
        &APPLY_DELAY_MS.to_string(),
    );
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

    let uri = Url::parse("file:///did_save_followup_inflight_exact_snapshot_fixture.bsl")
        .expect("fixture");
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

    let file_id = server
        .get_file_id_v2(&uri)
        .await
        .expect("file id after didOpen");
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
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let task_requested_version = {
                let tasks = server.background_parse_snapshot_apply_tasks_v2.lock().await;
                tasks.get(&file_id).map(|task| {
                    task.target
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .requested_version
                })
            };
            if task_requested_version == Some(2) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange must register an in-flight same-version parse snapshot task");
    assert_ne!(
        server
            .latest_ready_parse_snapshots_v2
            .read()
            .await
            .get(&file_id)
            .map(|state| state.parse_snapshot.file_version),
        Some(2),
        "exact same-version ready snapshot must still be absent before didSave"
    );
    while published_rx.try_recv().is_ok() {}

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

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let task_requested_version = {
                let tasks = server.background_parse_snapshot_apply_tasks_v2.lock().await;
                tasks.get(&file_id).map(|task| {
                    task.target
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .requested_version
                })
            };
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if task_requested_version == Some(2) && ready_version != Some(2) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didSave must keep the exact same-version snapshot task in-flight before follow-up");

    let first_publish =
        tokio::time::timeout(Duration::from_millis(FIRST_PUBLISH_BUDGET_MS), async {
            loop {
                let params = published_rx
                    .recv()
                    .await
                    .expect("publishDiagnostics channel must stay open");
                if params.uri == uri && params.version == Some(2) {
                    break params;
                }
            }
        })
        .await
        .expect("didSave must still publish save_fastlane before the bounded exact wait");
    assert!(
        first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax")),
        "didSave first publish must remain syntax-only before the in-flight exact wait resolves, diagnostics={:?}",
        first_publish.diagnostics
    );

    tokio::time::timeout(Duration::from_millis(FOLLOWUP_PUBLISH_BUDGET_MS), async {
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
    .expect("bounded exact wait must let same-version ready artifacts win the same save cycle");

    let followup_elapsed = did_save_started.elapsed();
    assert!(
        followup_elapsed
            <= diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET
                + Duration::from_millis(2_000),
        "exact in-flight wait must stay bounded (elapsed={followup_elapsed:?})"
    );

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_710, 8).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("diagnostics save timeline traces");
    let trace = traces
        .iter()
        .find(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(2)
        })
        .expect("matching diagnostics save timeline trace");
    let first_publish_trace = trace
        .get("first_publish")
        .and_then(|value| value.as_object())
        .expect("save_fastlane first publish trace");
    assert_eq!(
        first_publish_trace
            .get("profile")
            .and_then(|value| value.as_str()),
        Some("save_fastlane")
    );
    assert_eq!(
        first_publish_trace
            .get("publish_kind")
            .and_then(|value| value.as_str()),
        Some("syntax_only")
    );
    let full_publish = trace
        .get("followup_publish")
        .and_then(|value| value.as_object())
        .filter(|publish| {
            publish.get("profile").and_then(|value| value.as_str()) == Some("idle_heavy")
        })
        .or_else(|| {
            trace
                .get("first_publish")
                .and_then(|value| value.as_object())
                .filter(|publish| {
                    publish.get("profile").and_then(|value| value.as_str()) == Some("idle_heavy")
                })
        })
        .expect("idle_heavy full publish trace");
    assert_eq!(
        full_publish.get("profile").and_then(|value| value.as_str()),
        Some("idle_heavy")
    );
    assert_eq!(
        full_publish
            .get("publish_kind")
            .and_then(|value| value.as_str()),
        Some("full")
    );
    assert!(
        full_publish.get("wait_for_file_version_ms").is_none(),
        "exact-task wait success must not regress into wait_for_file_version gating, trace={trace:?}"
    );
    assert_eq!(
        full_publish
            .get("syntax_work_mode")
            .and_then(|value| value.as_str()),
        Some("reused")
    );
    assert_eq!(
        full_publish
            .get("semantic_path")
            .and_then(|value| value.as_str()),
        Some("ready_artifacts")
    );
    assert_eq!(
        full_publish
            .get("semantic_parse_source")
            .and_then(|value| value.as_str()),
        Some("snapshot")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_zero_probe")
            .and_then(|value| value.as_str()),
        Some("not_ready")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_wait_probe")
            .and_then(|value| value.as_str()),
        Some("ready")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_task_state")
            .and_then(|value| value.as_str()),
        Some("in_flight_same_version")
    );
    assert_eq!(
        trace
            .get("followup_shadow_state_available")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        trace
            .get("followup_semantic_path")
            .and_then(|value| value.as_str()),
        Some("ready_artifacts")
    );
    assert_eq!(
        trace
            .get("followup_semantic_parse_source")
            .and_then(|value| value.as_str()),
        Some("snapshot")
    );
    assert_eq!(
        trace
            .get("idle_heavy_outcome")
            .and_then(|value| value.as_str()),
        Some("published")
    );
    assert!(
        trace
            .get("followup_ready_snapshot_parse_exec_ms")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0),
        "exact ready path must export parse_exec timing on successful reuse, trace={trace:?}"
    );
    assert!(
        trace.get("followup_ready_snapshot_timeout_phase").is_none(),
        "successful exact ready path must not fabricate timeout phase attribution, trace={trace:?}"
    );
    assert!(
        trace
            .get("followup_ready_snapshot_dominant_phase")
            .and_then(|value| value.as_str())
            .is_some(),
        "successful exact ready path must expose a dominant ready-snapshot phase, trace={trace:?}"
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
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_did_change"
        )) > 0,
        "didChange same-version worker must export worker-start counter, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_did_save"
        )),
        0,
        "didSave exact wait must not start a duplicate didSave snapshot worker for the same text/version, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_change"
        )) > 0,
        "didChange same-version worker must export ready snapshot materialization counter, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_save"
        )),
        0,
        "didSave exact wait must not materialize a duplicate didSave snapshot worker for the same text/version, counters={counters:?}"
    );
    assert!(
        read_u64_metric(
            histograms
                .get("intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_did_change")
                .and_then(|value| value.get("count"))
        ) > 0,
        "didChange same-version worker must export ready snapshot materialization latency, histograms={histograms:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_change_phase_parse_exec"
        )) > 0,
        "didChange exact worker must export parse_exec phase counter, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_change_phase_document_symbol_side_work"
        )) > 0,
        "didChange exact worker must export documentSymbol side-work as a separate phase, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_zero_budget_outcome_not_ready"
        )) > 0,
        "didSave exact wait must export zero-budget probe miss counter, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_ready"
        )) > 0,
        "didSave exact wait must export bounded-wait ready counter, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_wait_state_total_reason_semantic_work"
        )) > 0,
        "didSave exact wait must export semantic_work wait-state counter, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_ready_artifacts"
        )) > 0,
        "didSave exact wait must export ready_artifacts semantic path counter, counters={counters:?}"
    );

    let report = serde_json::json!({
        "change_id": CHANGE_ID,
        "uri": uri.to_string(),
        "did_change_parse_delay_ms": DID_CHANGE_PARSE_DELAY_MS,
        "did_save_parse_delay_ms": DID_SAVE_PARSE_DELAY_MS,
        "apply_delay_ms": APPLY_DELAY_MS,
        "first_publish_budget_ms": FIRST_PUBLISH_BUDGET_MS,
        "followup_publish_budget_ms": FOLLOWUP_PUBLISH_BUDGET_MS,
        "first_publish_elapsed_ms": first_publish_trace
            .get("elapsed_ms")
            .and_then(|value| value.as_u64()),
        "first_publish_syntax_only": true,
        "followup_publish_elapsed_ms": full_publish
            .get("elapsed_ms")
            .and_then(|value| value.as_u64()),
        "followup_ready_snapshot_zero_probe": trace
            .get("followup_ready_snapshot_zero_probe")
            .and_then(|value| value.as_str()),
        "followup_ready_snapshot_wait_probe": trace
            .get("followup_ready_snapshot_wait_probe")
            .and_then(|value| value.as_str()),
        "followup_ready_snapshot_task_state": trace
            .get("followup_ready_snapshot_task_state")
            .and_then(|value| value.as_str()),
        "followup_shadow_state_available": trace
            .get("followup_shadow_state_available")
            .and_then(|value| value.as_bool()),
        "followup_wait_reason": trace
            .get("followup_wait_reason")
            .and_then(|value| value.as_str()),
        "followup_semantic_path": trace
            .get("followup_semantic_path")
            .and_then(|value| value.as_str()),
        "followup_semantic_parse_source": trace
            .get("followup_semantic_parse_source")
            .and_then(|value| value.as_str()),
        "followup_semantic_ir_source": trace
            .get("followup_semantic_ir_source")
            .and_then(|value| value.as_str()),
        "followup_wait_for_file_version_ms": trace
            .get("followup_wait_for_file_version_ms")
            .and_then(|value| value.as_u64()),
        "save_cycle_sequence": trace
            .get("save_cycle_sequence")
            .and_then(|value| value.as_u64()),
        "diagnostics_generation": trace
            .get("diagnostics_generation")
            .and_then(|value| value.as_u64()),
    });
    let report_path = std::env::var("BSL_V2_DID_SAVE_FOLLOWUP_INFLIGHT_EXACT_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!("{CHANGE_ID}-did-save-followup-inflight-exact.json"))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for refactor-17 in-flight exact report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report)
            .expect("serialize refactor-17 in-flight exact report"),
    )
    .expect("write refactor-17 in-flight exact report");

    drain_task.abort();
}

#[tokio::test]
async fn p7_did_save_followup_skips_bounded_wait_after_exact_producer_is_retargeted_away() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let (harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    let uri = Url::parse("file:///did_save_wait_probe_coalesced_away_fixture.bsl").expect("uri");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let exact_text: Arc<str> = Arc::from("Procedure Test()\n    Return 2;\nEndProcedure\n");
    let newer_text: Arc<str> = Arc::from("Procedure Test()\n    Return 3;\nEndProcedure\n");
    let exact_text_hash = *blake3::hash(exact_text.as_bytes()).as_bytes();
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server
        .diagnostics_generation_v2
        .write()
        .await
        .insert(file_id, 17);
    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, 2);
    server.latest_document_shadow_state_v2.write().await.insert(
        file_id,
        DocumentShadowStateV2 {
            version: 2,
            text: exact_text.clone(),
        },
    );
    server
        .background_parse_snapshot_apply_tasks_v2
        .lock()
        .await
        .insert(
            file_id,
            crate::server::BackgroundParseSnapshotApplyTaskV2 {
                target_epoch: Arc::new(std::sync::atomic::AtomicU64::new(1)),
                target: Arc::new(std::sync::Mutex::new(
                    crate::server::BackgroundParseSnapshotApplyTargetV2 {
                        requested_version: 2,
                        text_hash: exact_text_hash,
                        source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
                        path: Arc::<str>::from(uri.path().to_string()),
                        text: exact_text.clone(),
                        parser_base_recovery_text: None,
                        parser_base_recovery_reuse_parse_result: None,
                        parser_edits: Vec::new(),
                        forced_full_parse_reason: None,
                        async_delay_mode: crate::server::ParseSnapshotAsyncDelayMode::None,
                        blocking_delay_env_key: None,
                        did_change_attribution: None,
                        epoch: 1,
                    },
                )),
                control: control.clone(),
                handle: tokio::spawn(async {}),
            },
        );

    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: 17,
        save_cycle_sequence: Some(1),
        requested_version: 2,
    };
    let waiter = tokio::spawn({
        let server = server.clone();
        async move {
            server
                .wait_for_ready_parse_snapshot_probe_outcome_v2(
                    &supersession_key,
                    None,
                    diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET,
                    Some(exact_text_hash),
                )
                .await
        }
    });

    tokio::task::yield_now().await;
    {
        let tasks = server.background_parse_snapshot_apply_tasks_v2.lock().await;
        let task = tasks
            .get(&file_id)
            .expect("exact producer must stay registered before retarget");
        let next_epoch = task.target_epoch.fetch_add(1, Ordering::SeqCst) + 1;
        {
            let mut target = task
                .target
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *target = crate::server::BackgroundParseSnapshotApplyTargetV2 {
                requested_version: 3,
                text_hash: *blake3::hash(newer_text.as_bytes()).as_bytes(),
                source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
                path: Arc::<str>::from(uri.path().to_string()),
                text: newer_text,
                parser_base_recovery_text: None,
                parser_base_recovery_reuse_parse_result: None,
                parser_edits: Vec::new(),
                forced_full_parse_reason: None,
                async_delay_mode: crate::server::ParseSnapshotAsyncDelayMode::None,
                blocking_delay_env_key: None,
                did_change_attribution: None,
                epoch: next_epoch,
            };
        }
        task.control.cancel_requested.store(true, Ordering::SeqCst);
        task.control
            .retarget_requested
            .store(true, Ordering::SeqCst);
        task.control.control_notify.notify_waiters();
    }
    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, 3);

    let outcome = tokio::time::timeout(Duration::from_secs(1), waiter)
        .await
        .expect("probe wait must complete once exact producer is retargeted away")
        .expect("probe waiter task");
    assert_eq!(
        outcome,
        diagnostics_runtime::ReadyParseSnapshotProbeOutcomeV2::VersionMismatch
    );

    harness.shutdown().await;
}

#[tokio::test]
async fn p7_did_save_followup_promotes_already_queued_exact_worker_before_shadow_fallback() {
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
    const V2_FIXTURE: &str =
        "Процедура Тест()\n    Сообщить(необъявленнаяQueued);\nКонецПроцедуры\n";
    const FIRST_PUBLISH_BUDGET_MS: u64 = 2_500;
    const FOLLOWUP_READY_ARTIFACT_BUDGET_MS: u64 = 2_000;

    let _env_lock = lock_test_env().await;
    let _background_reserved_only_guard =
        EnvVarGuard::set("BSL_TEST_RUNTIME_BACKGROUND_RESERVED_ONLY", "1");
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

    let background_holder_barrier = Arc::new(std::sync::Barrier::new(2));
    let (background_holder_ready_tx, background_holder_ready_rx) = tokio::sync::oneshot::channel();
    let background_holder_barrier_for_task = background_holder_barrier.clone();
    let background_holder_coordinator = coordinator.clone();
    let background_holder = tokio::spawn(async move {
        let _ = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
            bsl_runtime::application::CpuWorkClass::Background,
            bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
            Some(background_holder_coordinator.as_ref()),
            move || {
                let _ = background_holder_ready_tx.send(());
                background_holder_barrier_for_task.wait();
            },
        )
        .await;
    });
    tokio::time::timeout(Duration::from_secs(3), background_holder_ready_rx)
        .await
        .expect("background holder must seize generic background reserved permit")
        .expect("background holder ready signal");

    let uri =
        Url::parse("file:///did_save_followup_queued_exact_worker_fixture.bsl").expect("fixture");
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

    let file_id = server
        .get_file_id_v2(&uri)
        .await
        .expect("file id after didOpen");
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
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let task_requested_version = {
                let tasks = server.background_parse_snapshot_apply_tasks_v2.lock().await;
                tasks.get(&file_id).map(|task| {
                    task.target
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .requested_version
                })
            };
            if task_requested_version == Some(2) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange must register exact same-version snapshot task");
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_ne!(
        server
            .latest_ready_parse_snapshots_v2
            .read()
            .await
            .get(&file_id)
            .map(|state| state.parse_snapshot.file_version),
        Some(2),
        "ready snapshot must still be absent while generic background reserved permit is held"
    );
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

    let first_publish =
        tokio::time::timeout(Duration::from_millis(FIRST_PUBLISH_BUDGET_MS), async {
            loop {
                let params = published_rx
                    .recv()
                    .await
                    .expect("publishDiagnostics channel must stay open");
                if params.uri == uri && params.version == Some(2) {
                    break params;
                }
            }
        })
        .await
        .expect("save_fastlane first publish must stay fast while exact worker is queued");
    assert!(
        first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax")),
        "first publish must remain syntax-only before exact ready snapshot resolves, diagnostics={:?}",
        first_publish.diagnostics
    );

    let full_publish = tokio::time::timeout(
        Duration::from_millis(FOLLOWUP_READY_ARTIFACT_BUDGET_MS),
        async {
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
        },
    )
    .await
    .expect("promoted exact worker must win before shadow-state fallback wait budget elapses");
    assert!(
        !background_holder.is_finished(),
        "full publish must arrive while the generic background reserved permit is still blocked; otherwise the queued exact worker was not actually promoted"
    );

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_716, 8).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("diagnostics save timeline traces");
    let trace = traces
        .iter()
        .find(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(2)
        })
        .expect("matching diagnostics save timeline trace");
    let full_publish_trace = trace
        .get("followup_publish")
        .and_then(|value| value.as_object())
        .filter(|publish| {
            publish.get("profile").and_then(|value| value.as_str()) == Some("idle_heavy")
        })
        .expect("idle_heavy follow-up trace");
    assert_eq!(
        full_publish_trace
            .get("semantic_path")
            .and_then(|value| value.as_str()),
        Some("ready_artifacts")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_zero_probe")
            .and_then(|value| value.as_str()),
        Some("not_ready")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_wait_probe")
            .and_then(|value| value.as_str()),
        Some("ready")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_task_state")
            .and_then(|value| value.as_str()),
        Some("in_flight_same_version")
    );
    assert_eq!(
        trace
            .get("followup_semantic_path")
            .and_then(|value| value.as_str()),
        Some("ready_artifacts")
    );

    background_holder_barrier.wait();
    let _ = background_holder.await;
    let _ = full_publish;
    drain_task.abort();
}

#[tokio::test]
async fn p7_did_save_followup_skips_bounded_wait_when_only_did_save_refresh_task_exists() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
        reload_runtime_config: bool,
    }

    impl EnvVarGuard {
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
    const FOLLOWUP_PUBLISH_BUDGET_MS: u64 = 2000;

    let _env_lock = lock_test_env().await;
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

    let uri = Url::parse("file:///did_save_followup_applied_state_preferred_fixture.bsl")
        .expect("fixture");
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

    let file_id = server
        .get_file_id_v2(&uri)
        .await
        .expect("file id after didOpen");
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
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let applied_ready = server
                .analysis_v2
                .snapshot()
                .await
                .file_version(file_id)
                .ok()
                .flatten()
                == Some(2);
            let ready_parse_snapshot = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if applied_ready
                && ready_parse_snapshot
                    .as_ref()
                    .is_some_and(|state| state.parse_snapshot.file_version == 2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange must materialize applied state and ready parse snapshot for version 2");
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if !server
                .background_parse_snapshot_apply_tasks_v2
                .lock()
                .await
                .contains_key(&file_id)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("completed didChange parse snapshot task must clean up before absent-exact-task didSave regression");
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
    {
        let tasks = server.background_parse_snapshot_apply_tasks_v2.lock().await;
        let task = tasks
            .get(&file_id)
            .expect("didSave must seed a same-version refresh task for this regression");
        let target = task
            .target
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        assert_eq!(
            target.requested_version, 2,
            "didSave refresh task must target the saved revision"
        );
        assert_eq!(
            target.source,
            crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidSave,
            "regression must exercise the same-version didSave refresh task that should not count as exact-task evidence"
        );
    }

    server
        .latest_ready_parse_snapshots_v2
        .write()
        .await
        .remove(&file_id);

    tokio::time::timeout(Duration::from_millis(1000), async {
        loop {
            let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_708, 8).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces");
            if traces.iter().any(|trace| {
                trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                    && trace
                        .get("requested_version")
                        .and_then(|value| value.as_i64())
                        == Some(2)
                    && trace
                        .get("first_publish")
                        .and_then(|value| value.get("profile"))
                        .and_then(|value| value.as_str())
                        == Some("save_fastlane")
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("didSave must publish save_fastlane first before applied-state follow-up");

    tokio::time::timeout(Duration::from_millis(FOLLOWUP_PUBLISH_BUDGET_MS), async {
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
    .expect(
        "follow-up must publish full diagnostics even when only the didSave refresh task exists",
    );

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_709, 8).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("diagnostics save timeline traces");
    let trace = traces
        .iter()
        .find(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(2)
        })
        .expect("matching diagnostics save timeline trace");
    assert_eq!(
        trace
            .get("followup_wait_reason")
            .and_then(|value| value.as_str()),
        None,
        "successful applied-state follow-up must not remain in apply-lag/pending state, trace={trace:?}"
    );
    let full_publish = trace
        .get("followup_publish")
        .and_then(|value| value.as_object())
        .expect("idle_heavy full publish trace");
    assert_eq!(
        full_publish.get("profile").and_then(|value| value.as_str()),
        Some("idle_heavy")
    );
    assert_eq!(
        full_publish
            .get("publish_kind")
            .and_then(|value| value.as_str()),
        Some("full")
    );
    assert!(
        full_publish.get("wait_for_file_version_ms").is_none(),
        "applied-state follow-up must not regress into wait_for_file_version gating, trace={trace:?}"
    );
    assert_eq!(
        full_publish
            .get("syntax_work_mode")
            .and_then(|value| value.as_str()),
        Some("reused"),
        "applied-state follow-up must reuse same-version save_fastlane syntax artifacts when they are already fresh for the save cycle, trace={trace:?}"
    );
    assert_eq!(
        full_publish
            .get("semantic_path")
            .and_then(|value| value.as_str()),
        Some("shadow_state"),
        "fallback after missing ready snapshot must stay truthful about shadow-state semantic path, trace={trace:?}"
    );
    assert_eq!(
        full_publish
            .get("semantic_parse_source")
            .and_then(|value| value.as_str()),
        Some("salsa")
    );
    assert_eq!(
        full_publish
            .get("semantic_ir_source")
            .and_then(|value| value.as_str()),
        Some("salsa")
    );
    assert!(
        full_publish.get("syntax_diagnostics_query_ms").is_none(),
        "applied-state follow-up must not expose recomputed syntax timing when same-version save_fastlane syntax artifacts are reused, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("followup_semantic_path")
            .and_then(|value| value.as_str()),
        Some("shadow_state")
    );
    assert_eq!(
        trace
            .get("followup_semantic_parse_source")
            .and_then(|value| value.as_str()),
        Some("salsa")
    );
    assert_eq!(
        trace
            .get("followup_semantic_ir_source")
            .and_then(|value| value.as_str()),
        Some("salsa")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_zero_probe")
            .and_then(|value| value.as_str()),
        Some("not_ready"),
        "stale latest-version path must retain zero-budget probe attribution before the bounded wait observes version mismatch, trace={trace:?}"
    );
    assert!(
        trace.get("followup_ready_snapshot_wait_probe").is_none(),
        "shadow-state fallback must not report bounded-wait probe when fallback succeeds first, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_task_state")
            .and_then(|value| value.as_str()),
        Some("absent")
    );
    assert_eq!(
        trace
            .get("followup_shadow_state_available")
            .and_then(|value| value.as_bool()),
        Some(true)
    );

    drain_task.abort();
}
