#[tokio::test]
async fn p7_diagnostics_save_followup_stays_isolated_from_generic_background_reserved_blocker() {
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
    let _background_reserved_only_guard =
        EnvVarGuard::set("BSL_TEST_RUNTIME_BACKGROUND_RESERVED_ONLY", "1");
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
    let _server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");
    let uri =
        Url::parse("file:///did_save_followup_runtime_queue_wait_fixture.bsl").expect("fixture");
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
    while published_rx.try_recv().is_ok() {}

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
        .expect("background holder must start before didSave follow-up")
        .expect("background holder ready signal");

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

    let first_publish = tokio::time::timeout(Duration::from_secs(3), async {
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
    .expect("save_fastlane first publish must arrive before queued follow-up");
    assert!(
        first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax")),
        "save_fastlane first publish must stay syntax-only while heavy follow-up is still pending, diagnostics={:?}",
        first_publish.diagnostics
    );

    let interactive_probe = tokio::time::timeout(
        Duration::from_millis(500),
        bsl_runtime::application::spawn_bounded_blocking_with_class_observed_call_origin(
            bsl_runtime::application::CpuWorkClass::Interactive,
            bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
            Some(coordinator.as_ref()),
            || {
                std::thread::sleep(Duration::from_millis(50));
                7_u8
            },
        ),
    )
    .await
    .expect("interactive probe must not be stranded behind didSave follow-up lane pressure");
    assert!(
        interactive_probe.join_result.is_ok(),
        "interactive probe must complete successfully while didSave follow-up is active"
    );
    assert!(
        interactive_probe.queue_wait_elapsed < Duration::from_millis(250),
        "didSave follow-up lane must not borrow interactive reserved capacity under background saturation, observed interactive queue wait={:?}",
        interactive_probe.queue_wait_elapsed
    );

    let followup_publish = tokio::time::timeout(Duration::from_secs(3), async {
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
        "didSave heavy follow-up must publish richer diagnostics before the generic background reserved permit is released",
    );
    assert!(
        followup_publish
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.source.as_deref() == Some("bsl-analysis-v2")),
        "heavy follow-up publish must include semantic diagnostics while the generic background reserved permit is still blocked, diagnostics={:?}",
        followup_publish.diagnostics
    );
    assert!(
        !background_holder.is_finished(),
        "generic background blocker should still be holding its reserved permit when the didSave follow-up publishes"
    );

    let trace = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_712, 8).await;
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
                    && trace.get("followup_publish").is_some()
            }) {
                break trace.clone();
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("didSave follow-up publish trace must be observable");
    let full_publish = trace
        .get("followup_publish")
        .and_then(|value| value.as_object())
        .expect("idle_heavy full publish trace");
    assert_eq!(
        full_publish.get("profile").and_then(|value| value.as_str()),
        Some("idle_heavy")
    );
    assert_ne!(
        trace
            .get("followup_wait_reason")
            .and_then(|value| value.as_str()),
        Some("runtime_queue_wait"),
        "isolated didSave follow-up must not report generic runtime_queue_wait as the default blocker when generic background reserved capacity is already occupied, trace={trace:?}"
    );
    assert!(
        full_publish
            .get("runtime_queue_wait_ms")
            .and_then(|value| value.as_u64())
            .is_none_or(|value| value < 250),
        "isolated didSave follow-up must keep any residual runtime queue wait bounded while publishing under generic background pressure, trace={trace:?}"
    );
    assert!(
        trace
            .get("followup_runtime_queue_wait_ms")
            .and_then(|value| value.as_u64())
            .is_none_or(|value| value < 250),
        "top-level didSave follow-up trace must not surface large generic runtime queue wait after isolation, trace={trace:?}"
    );
    assert!(
        full_publish.get("wait_for_file_version_ms").is_none(),
        "isolated didSave follow-up must not regress into wait_for_file_version gating, trace={trace:?}"
    );
    assert_eq!(
        full_publish
            .get("syntax_work_mode")
            .and_then(|value| value.as_str()),
        Some("reused"),
        "isolated didSave follow-up must still reuse save_fastlane syntax artifacts, trace={trace:?}"
    );

    background_holder_barrier.wait();
    tokio::time::timeout(Duration::from_secs(3), background_holder)
        .await
        .expect("background holder task timeout")
        .expect("background holder join");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    let gauges = metrics
        .get("gauges")
        .and_then(|value| value.as_object())
        .expect("metrics.gauges object");
    assert_eq!(
        counters
            .get("intellisense_v2_observability_contract_violation_reason_invalid_saturation_metric")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        0,
        "dedicated didSave follow-up lane metrics must not emit generic invalid saturation violations, metrics={metrics:?}"
    );
    assert!(
        counters
            .get("intellisense_v2_runtime_lane_queue_wait_total_origin_lsp_lane_did_save_followup")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0),
        "isolated didSave follow-up must export dedicated lane queue-wait counter, metrics={metrics:?}"
    );
    assert!(
        counters
            .get("intellisense_v2_runtime_lane_exec_total_origin_lsp_lane_did_save_followup")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0),
        "isolated didSave follow-up must export dedicated lane exec counter, metrics={metrics:?}"
    );
    assert!(
        histograms
            .get("intellisense_v2_runtime_lane_queue_wait_ms_origin_lsp_lane_did_save_followup")
            .and_then(|value| value.get("count"))
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0),
        "isolated didSave follow-up must export dedicated lane queue-wait histogram, metrics={metrics:?}"
    );
    assert!(
        histograms
            .get("intellisense_v2_runtime_lane_exec_ms_origin_lsp_lane_did_save_followup")
            .and_then(|value| value.get("count"))
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0),
        "isolated didSave follow-up must export dedicated lane exec histogram, metrics={metrics:?}"
    );
    for gauge_key in [
        "intellisense_v2_runtime_lane_saturation_gauge_origin_lsp_lane_did_save_followup_metric_quota",
        "intellisense_v2_runtime_lane_saturation_gauge_origin_lsp_lane_did_save_followup_metric_active_slots",
        "intellisense_v2_runtime_lane_saturation_gauge_origin_lsp_lane_did_save_followup_metric_queue_depth",
    ] {
        assert!(
            gauges
                .get(gauge_key)
                .and_then(|value| value.as_f64())
                .is_some(),
            "isolated didSave follow-up must export {gauge_key}, metrics={metrics:?}"
        );
    }

    drain_task.abort();
}

#[tokio::test]
async fn p8_did_save_followup_reuses_clean_save_fastlane_syntax_artifacts_after_apply_delay() {
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
    const FOLLOWUP_PUBLISH_BUDGET_MS: u64 = 10_000;

    let _env_lock = lock_test_env().await;
    let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "4000");
    let _did_save_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_SAVE_BLOCKING_PARSE_DELAY_MS", "4000");
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
        Url::parse("file:///did_save_followup_clean_syntax_reuse_fixture.bsl").expect("fixture");
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

    server
        .latest_ready_parse_snapshots_v2
        .write()
        .await
        .remove(&file_id);
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

    tokio::time::timeout(Duration::from_millis(2500), async {
        loop {
            let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_710, 8).await;
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
    .expect("save_fastlane must publish before generic follow-up syntax reuse");

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
    .expect("generic heavy follow-up must eventually publish full diagnostics");

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_711, 8).await;
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
        .expect("idle_heavy full publish trace");
    assert_eq!(
        full_publish
            .get("syntax_work_mode")
            .and_then(|value| value.as_str()),
        Some("reused"),
        "generic follow-up must reuse same-version syntax-clean save artifacts instead of recomputing syntax, trace={trace:?}"
    );
    assert!(
        full_publish.get("syntax_diagnostics_query_ms").is_none(),
        "generic follow-up must not expose recomputed syntax timing when syntax-clean save artifacts are reused, trace={trace:?}"
    );
    assert!(
        full_publish.get("semantic_diagnostics_query_ms").is_some(),
        "generic follow-up must still expose semantic timing, trace={trace:?}"
    );
    assert!(
        full_publish.get("wait_for_file_version_ms").is_none(),
        "shadow-state follow-up must not regress into wait_for_file_version gating when same-version save artifacts are already fresh, trace={trace:?}"
    );
    assert!(
        full_publish
            .get("apply_lag_ms")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0),
        "shadow-state follow-up must still expose factual apply lag separately from the bounded publish path, trace={trace:?}"
    );
    assert!(
        trace
            .get("followup_apply_lag_ms")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0),
        "top-level follow-up trace must surface apply lag fact for operator-facing summary, trace={trace:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p26_did_save_followup_relief_valve_publishes_ready_artifacts_despite_delayed_apply() {
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
    const DID_CHANGE_PARSE_DELAY_MS: u64 = 3_600;
    const APPLY_DELAY_MS: u64 = 4_000;
    const FIRST_PUBLISH_BUDGET_MS: u64 = 3_500;
    const FOLLOWUP_PUBLISH_BUDGET_MS: u64 = 7_000;

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

    let uri = Url::parse("file:///p26_did_save_followup_relief_exact_fixture.bsl").expect("uri");
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
        .expect("didSave must still publish save_fastlane before relief-valve exact wait");
    assert!(
        first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax")),
        "didSave first publish must remain syntax-only before relief-valve success, diagnostics={:?}",
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
    .expect("relief valve must still let exact ready artifacts publish before delayed apply");

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_726, 8).await;
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
        .expect("idle_heavy full publish trace");
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
        Some("timeout")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_relief_valve_outcome")
            .and_then(|value| value.as_str()),
        Some("engaged_helped")
    );
    assert!(
        trace
            .get("followup_ready_snapshot_continuation_reason")
            .and_then(|value| value.as_str())
            .is_none(),
        "direct relief help must not leave continuation reason, trace={trace:?}"
    );
    assert_eq!(
        full_publish
            .get("semantic_path")
            .and_then(|value| value.as_str()),
        Some("ready_artifacts")
    );
    assert_eq!(
        trace
            .get("followup_semantic_path")
            .and_then(|value| value.as_str()),
        Some("ready_artifacts")
    );
    assert!(
        trace
            .get("followup_wait_reason")
            .and_then(|value| value.as_str())
            .is_none(),
        "published exact follow-up must not leave wait_reason as final blocker, trace={trace:?}"
    );
    assert!(
        trace
            .get("followup_blocker_reason")
            .and_then(|value| value.as_str())
            .is_none(),
        "published exact follow-up must not leave blocker_reason after publish, trace={trace:?}"
    );
    assert!(
        full_publish
            .get("wait_for_file_version_ms")
            .and_then(|value| value.as_u64())
            .is_none(),
        "relief-valve exact publish must stay off wait_for_file_version gating, trace={trace:?}"
    );

    let counters = coordinator
        .observability_metrics()
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object")
        .clone();
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_helped"
        )) > 0,
        "relief-valve exact publish must export engaged_helped, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_skipped_apply_lag"
        )),
        0,
        "relief-valve exact publish must no longer regress into skipped_apply_lag on this path, counters={counters:?}"
    );

    drain_task.abort();
}
