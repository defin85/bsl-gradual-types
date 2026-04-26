#[tokio::test]
async fn p27_did_save_followup_promotes_exact_parse_exec_past_optional_cache_enrichment() {
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
    const OPTIONAL_CACHE_ENRICHMENT_DELAY_MS: u64 = 10_000;
    const FIRST_PUBLISH_BUDGET_MS: u64 = 3_500;
    const FOLLOWUP_PUBLISH_BUDGET_MS: u64 = 5_000;

    let _env_lock = lock_test_env().await;
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);

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

    let uri = Url::parse("file:///p27_did_save_followup_optional_cache_enrichment_fixture.bsl")
        .expect("uri");
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
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize version 1 before didSave save-critical promotion");

    let _optional_delay_guard = EnvVarGuard::set(
        "BSL_TEST_PARSE_SNAPSHOT_OPTIONAL_CACHE_ENRICHMENT_DELAY_MS",
        &OPTIONAL_CACHE_ENRICHMENT_DELAY_MS.to_string(),
    );

    let v2_hash = *blake3::hash(V2_FIXTURE.as_bytes()).as_bytes();
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

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let current_subphase = server
                .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
                .await
                .and_then(|control| {
                    control
                        .phase_attribution_snapshot()
                        .current_parse_exec_subphase
                });
            if current_subphase
                == Some(
                    crate::server::ReadyParseSnapshotParseExecSubphaseV2::OptionalCacheEnrichment,
                )
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange exact worker must enter optional cache enrichment before didSave promotion");

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
        .expect(
            "didSave must publish save_fastlane before save-critical exact ready path finishes",
        );
    assert!(
        first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax")),
        "didSave first publish must remain syntax-only before exact ready follow-up, diagnostics={:?}",
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
    .expect("save-critical exact parse_exec must publish ready artifacts without waiting for optional cache enrichment");

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_727, 8).await;
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
        Some("ready")
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
            .get("followup_semantic_path")
            .and_then(|value| value.as_str()),
        Some("ready_artifacts")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_timeout_phase")
            .and_then(|value| value.as_str()),
        None
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_parse_exec_timeout_subphase")
            .and_then(|value| value.as_str()),
        None
    );
    assert!(
        trace
            .get("followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0 && value < OPTIONAL_CACHE_ENRICHMENT_DELAY_MS / 10),
        "save-critical exact publish must bound optional cache enrichment far below the configured injected delay, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_parse_exec_dominant_subphase")
            .and_then(|value| value.as_str()),
        Some("optional_cache_enrichment")
    );
    assert!(
        trace
            .get("followup_ready_snapshot_parse_exec_ms")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value < OPTIONAL_CACHE_ENRICHMENT_DELAY_MS / 10),
        "save-critical exact publish must bound total parse_exec far below the injected optional delay, trace={trace:?}"
    );
    assert!(
        trace
            .get("followup_ready_snapshot_relief_valve_outcome")
            .is_none(),
        "bounded wait success must not leave stale relief attribution behind, trace={trace:?}"
    );

    let counters = coordinator
        .observability_metrics()
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object")
        .clone();
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_ready"
        )) > 0,
        "save-critical exact publish must export bounded-wait ready probe outcome, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_did_change"
        )) > 0,
        "didChange worker start must preserve original source evidence, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_change"
        )),
        0,
        "same-version didSave promotion must not finalize materialization under original didChange source, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_save"
        )) > 0,
        "same-version didSave promotion must finalize materialization under effective didSave source, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_change_phase_ready_install"
        )),
        0,
        "ready-install phase metrics must not keep stale didChange attribution after didSave promotion, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_save_phase_ready_install"
        )) > 0,
        "ready-install phase metrics must use effective didSave attribution after promotion, counters={counters:?}"
    );

    drain_task.abort();
}

#[test]
fn p55_ready_install_exact_type_index_wait_snapshot_records_terminal_outcomes() {
    for (outcome, exact_ready, blocker_class) in [
        ("ready", Some(true), "ready"),
        ("retargeted", Some(false), "observed_version_mismatch"),
        ("superseded", Some(false), "type_index_computing"),
        (
            "latest_version_mismatch",
            Some(false),
            "observed_version_mismatch",
        ),
        ("deadline", Some(false), "type_index_computing"),
    ] {
        let control = crate::server::BackgroundParseSnapshotApplyTaskControlV2::new();
        let probe = crate::server::ReadyInstallExactTypeIndexWaitProbeV2 {
            waiter_action: Some("promotion"),
            matching_task_state: Some("matching"),
            task_phase: Some("computing"),
            task_requested_version: Some(2),
            task_active_requested_version: Some(2),
            observed_file_version: Some(2),
            exact_ready,
            ready_snapshot_version: Some(1),
            parse_snapshot_incremental: Some(true),
            parse_snapshot_changed_ranges_count: Some(1),
            parse_snapshot_serve_only_blocked: Some(false),
            blocker_class: Some(blocker_class),
        };

        control.start_ready_install_exact_type_index_wait(Some(Duration::from_millis(25)));
        control.update_ready_install_exact_type_index_wait(probe.clone());
        control.finish_ready_install_exact_type_index_wait(outcome, probe);

        let snapshot = control.ready_install_exact_type_index_wait_snapshot();
        assert!(!snapshot.active);
        assert_eq!(snapshot.ceiling_ms, Some(25));
        assert_eq!(snapshot.outcome, Some(outcome));
        assert_eq!(snapshot.waiter_action, Some("promotion"));
        assert_eq!(snapshot.matching_task_state, Some("matching"));
        assert_eq!(snapshot.task_phase, Some("computing"));
        assert_eq!(snapshot.ready_snapshot_version, Some(1));
        assert_eq!(snapshot.blocker_class, Some(blocker_class));
    }
}

#[tokio::test]
async fn p56_pure_did_change_materializes_after_exact_type_index_ready_install() {
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

    fn counter_delta(
        final_counters: &serde_json::Map<String, serde_json::Value>,
        baseline_counters: &serde_json::Map<String, serde_json::Value>,
        key: &str,
    ) -> u64 {
        read_u64_metric(final_counters.get(key))
            .saturating_sub(read_u64_metric(baseline_counters.get(key)))
    }

    const V1_FIXTURE: &str = "Процедура Тест()\n    Сообщить(\"v1\");\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    Сообщить(\"v2\");\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);
    let _ready_install_wait_guard =
        EnvVarGuard::set("BSL_TEST_READY_INSTALL_EXACT_TYPE_INDEX_WAIT_MS", "5000");

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        V1_FIXTURE,
        "file:///p56_pure_did_change_ready_install_success.bsl",
    )
    .await;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if ready_version == Some(1) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize version 1 before pure didChange test");

    let baseline_metrics = server.coordinator.observability_metrics();
    let baseline_counters = baseline_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("baseline metrics.counters object")
        .clone();

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

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if ready_version == Some(2) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("pure didChange must materialize the exact ready snapshot");
    let analysis = server.analysis_v2.snapshot().await;
    assert_eq!(
        analysis.file_version(file_id).ok().flatten(),
        Some(2),
        "pure didChange materialization must install the current analysis version"
    );
    assert!(
        analysis
            .current_type_index_serve_only_ready(file_id)
            .ok()
            .unwrap_or(false),
        "pure didChange materialization must leave the exact type-index artifact ready"
    );
    drop(analysis);

    let final_metrics = server.coordinator.observability_metrics();
    let final_counters = final_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("final metrics.counters object");
    assert!(
        counter_delta(
            final_counters,
            &baseline_counters,
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_change",
        ) > 0,
        "pure didChange success must be counted as didChange materialization, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        counter_delta(
            final_counters,
            &baseline_counters,
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_exact_type_index_deadline_before_ready_install",
        ),
        0,
        "pure didChange ready path must not be classified as a deadline, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p56_pure_did_change_exact_type_index_deadline_is_non_success() {
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

    fn counter_delta(
        final_counters: &serde_json::Map<String, serde_json::Value>,
        baseline_counters: &serde_json::Map<String, serde_json::Value>,
        key: &str,
    ) -> u64 {
        read_u64_metric(final_counters.get(key))
            .saturating_sub(read_u64_metric(baseline_counters.get(key)))
    }

    const V1_FIXTURE: &str = "Процедура Тест()\n    Сообщить(\"v1\");\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    Сообщить(\"v2\");\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);
    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        V1_FIXTURE,
        "file:///p56_pure_did_change_ready_install_deadline.bsl",
    )
    .await;
    let _ready_install_wait_guard =
        EnvVarGuard::set("BSL_TEST_READY_INSTALL_EXACT_TYPE_INDEX_WAIT_MS", "10");
    let _precompute_delay_guard =
        EnvVarGuard::set("BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS", "2000");
    let _head_precompute_delay_guard =
        EnvVarGuard::set("BSL_TEST_CURRENT_REVISION_HEAD_PRECOMPUTE_DELAY_MS", "2000");

    let baseline_metrics = server.coordinator.observability_metrics();
    let baseline_counters = baseline_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("baseline metrics.counters object")
        .clone();

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

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let failure = server.latest_snapshot_failures_v2.read().await.get(&file_id).cloned();
            if failure.as_ref().is_some_and(|state| {
                state.requested_version == 2
                    && state.reason.as_ref() == "exact_type_index_deadline_before_ready_install"
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("pure didChange ready-install exact type-index wait must classify the delayed precompute blocker");

    let ready_version = server
        .latest_ready_parse_snapshots_v2
        .read()
        .await
        .get(&file_id)
        .map(|state| state.parse_snapshot.file_version);
    assert_ne!(
        ready_version,
        Some(2),
        "pure didChange deadline classification must not install a canonical ready snapshot"
    );

    let final_metrics = server.coordinator.observability_metrics();
    let final_counters = final_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("final metrics.counters object");
    assert_eq!(
        counter_delta(
            final_counters,
            &baseline_counters,
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_change",
        ),
        0,
        "pure didChange deadline must stay out of successful didChange materialization samples, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert!(
        counter_delta(
            final_counters,
            &baseline_counters,
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_exact_type_index_deadline_before_ready_install",
        ) > 0,
        "pure didChange deadline must export a didChange non-success terminal reason, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p55_same_version_did_save_promotion_uses_effective_source_for_ready_install_metrics() {
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

    fn counter_delta(
        final_counters: &serde_json::Map<String, serde_json::Value>,
        baseline_counters: &serde_json::Map<String, serde_json::Value>,
        key: &str,
    ) -> u64 {
        read_u64_metric(final_counters.get(key))
            .saturating_sub(read_u64_metric(baseline_counters.get(key)))
    }

    const V1_FIXTURE: &str = "Процедура Тест()\n    Сообщить(\"v1\");\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    Сообщить(\"v2\");\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);
    let _pre_materialization_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_PRE_MATERIALIZATION_DELAY_MS", "350");

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        V1_FIXTURE,
        "file:///p55_effective_source_attribution_after_did_save_promotion.bsl",
    )
    .await;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if ready_version == Some(1) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize version 1 before promotion attribution test");

    let baseline_metrics = server.coordinator.observability_metrics();
    let baseline_counters = baseline_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("baseline metrics.counters object")
        .clone();

    let v2_hash = *blake3::hash(V2_FIXTURE.as_bytes()).as_bytes();
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

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let current_phase = server
                .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
                .await
                .and_then(|control| control.phase_attribution_snapshot().current_phase);
            if current_phase
                == Some(crate::server::ReadyParseSnapshotAttributionPhaseV2::PostParsePreMaterialization)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange worker must pause before materialization so didSave can promote it");

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

    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if ready_version == Some(2) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("promoted didSave target must materialize exact ready snapshot");

    let final_metrics = server.coordinator.observability_metrics();
    let final_counters = final_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("final metrics.counters object");
    let did_change_started_key =
        "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_did_change";
    let did_change_materialization_key =
        "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_change";
    let did_save_materialization_key =
        "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_save";
    let did_change_ready_install_phase_key =
        "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_change_phase_ready_install";
    let did_save_ready_install_phase_key =
        "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_save_phase_ready_install";

    assert!(
        counter_delta(final_counters, &baseline_counters, did_change_started_key) > 0,
        "worker start must preserve original didChange source evidence, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        counter_delta(final_counters, &baseline_counters, did_change_materialization_key),
        0,
        "materialization must not keep stale didChange attribution after didSave promotion, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert!(
        counter_delta(final_counters, &baseline_counters, did_save_materialization_key) > 0,
        "materialization must use effective didSave attribution after promotion, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        counter_delta(final_counters, &baseline_counters, did_change_ready_install_phase_key),
        0,
        "ready-install phase must not keep stale didChange attribution after didSave promotion, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert!(
        counter_delta(final_counters, &baseline_counters, did_save_ready_install_phase_key) > 0,
        "ready-install phase must use effective didSave attribution after promotion, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p55_ready_install_exact_type_index_deadline_exports_classified_blocker() {
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

    const V1_FIXTURE: &str = "Процедура Тест()\n    Сообщить(\"v1\");\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    Сообщить(\"v2\");\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);
    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        V1_FIXTURE,
        "file:///p55_ready_install_exact_type_index_deadline.bsl",
    )
    .await;
    let _ready_install_wait_guard =
        EnvVarGuard::set("BSL_TEST_READY_INSTALL_EXACT_TYPE_INDEX_WAIT_MS", "10");
    let _precompute_delay_guard =
        EnvVarGuard::set("BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS", "2000");
    let _head_precompute_delay_guard =
        EnvVarGuard::set("BSL_TEST_CURRENT_REVISION_HEAD_PRECOMPUTE_DELAY_MS", "2000");
    let _pre_materialization_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_PRE_MATERIALIZATION_DELAY_MS", "350");

    let v2_hash = *blake3::hash(V2_FIXTURE.as_bytes()).as_bytes();
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
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let current_phase = server
                .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
                .await
                .and_then(|control| control.phase_attribution_snapshot().current_phase);
            if current_phase
                == Some(
                    crate::server::ReadyParseSnapshotAttributionPhaseV2::PostParsePreMaterialization,
                )
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange worker must pause before materialization so didSave can attach a save-cycle envelope");

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

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let failure = server.latest_snapshot_failures_v2.read().await.get(&file_id).cloned();
            if failure.as_ref().is_some_and(|state| {
                state.requested_version == 2
                    && state.reason.as_ref() == "exact_type_index_deadline_before_ready_install"
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("ready-install exact type-index wait must classify the delayed precompute blocker");

    let ready_version = server
        .latest_ready_parse_snapshots_v2
        .read()
        .await
        .get(&file_id)
        .map(|state| state.parse_snapshot.file_version);
    assert_ne!(
        ready_version,
        Some(2),
        "deadline classification must not weaken canonical ready snapshot exact gates"
    );

    let counters = server
        .coordinator
        .observability_metrics()
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object")
        .clone();
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_exact_type_index_deadline_before_ready_install"
        )) > 0,
        "deadline classification must be exported as a low-cardinality worker termination reason, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p28_did_save_followup_promotes_exact_core_build_past_tree_cache_install() {
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
    const TREE_CACHE_INSTALL_DELAY_MS: u64 = 10_000;
    const FIRST_PUBLISH_BUDGET_MS: u64 = 3_500;
    const FOLLOWUP_PUBLISH_BUDGET_MS: u64 = 5_000;

    let _env_lock = lock_test_env().await;
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);

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
        Url::parse("file:///p28_did_save_followup_tree_cache_install_fixture.bsl").expect("uri");
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
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize version 1 before didSave tree-cache promotion");

    let _tree_cache_delay_guard = EnvVarGuard::set(
        "BSL_TEST_PARSE_SNAPSHOT_TREE_CACHE_INSTALL_DELAY_MS",
        &TREE_CACHE_INSTALL_DELAY_MS.to_string(),
    );

    let v2_hash = *blake3::hash(V2_FIXTURE.as_bytes()).as_bytes();
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

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let current_checkpoint = server
                .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
                .await
                .and_then(|control| {
                    control
                        .phase_attribution_snapshot()
                        .current_core_build_checkpoint
                });
            if current_checkpoint
                == Some(crate::server::ReadyParseSnapshotCoreBuildCheckpointV2::TreeCacheInstall)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange exact worker must enter tree-cache install before didSave promotion");

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
        .expect(
            "didSave must publish save_fastlane before save-critical exact core build finishes",
        );
    assert!(
        first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax")),
        "didSave first publish must remain syntax-only before exact ready follow-up, diagnostics={:?}",
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
    .expect("save-critical exact core build must publish ready artifacts without waiting for tree-cache install");

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_728, 8).await;
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
        Some("ready")
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
            .get("followup_semantic_path")
            .and_then(|value| value.as_str()),
        Some("ready_artifacts")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_timeout_phase")
            .and_then(|value| value.as_str()),
        None
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_parse_exec_timeout_subphase")
            .and_then(|value| value.as_str()),
        None
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint")
            .and_then(|value| value.as_str()),
        None
    );
    assert!(
        trace
            .get("followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0 && value < TREE_CACHE_INSTALL_DELAY_MS / 10),
        "save-critical exact publish must bound tree-cache install far below the configured injected delay, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint")
            .and_then(|value| value.as_str()),
        Some("tree_cache_install")
    );
    assert!(
        trace
            .get("followup_ready_snapshot_parse_exec_ms")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value < TREE_CACHE_INSTALL_DELAY_MS / 10),
        "save-critical exact publish must bound total parse_exec far below the injected tree-cache delay, trace={trace:?}"
    );
    assert!(
        trace
            .get("followup_ready_snapshot_relief_valve_outcome")
            .is_none(),
        "bounded wait success must not leave stale relief attribution behind, trace={trace:?}"
    );

    let counters = coordinator
        .observability_metrics()
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object")
        .clone();
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_ready"
        )) > 0,
        "save-critical exact publish must export bounded-wait ready probe outcome, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p29_did_save_followup_promotes_exact_ready_snapshot_assembly_past_syntax_error_collection()
{
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
    const SYNTAX_ERROR_ASSEMBLY_DELAY_MS: u64 = 10_000;
    const FIRST_PUBLISH_BUDGET_MS: u64 = 3_500;
    const FOLLOWUP_PUBLISH_BUDGET_MS: u64 = 5_000;

    let _env_lock = lock_test_env().await;
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);

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
        Url::parse("file:///p29_did_save_followup_syntax_error_assembly_fixture.bsl").expect("uri");
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
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize version 1 before didSave assembly promotion");

    let _syntax_error_delay_guard = EnvVarGuard::set(
        "BSL_TEST_PARSE_SNAPSHOT_SYNTAX_ERROR_ASSEMBLY_DELAY_MS",
        &SYNTAX_ERROR_ASSEMBLY_DELAY_MS.to_string(),
    );

    let v2_hash = *blake3::hash(V2_FIXTURE.as_bytes()).as_bytes();
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

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let current_checkpoint = server
                .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
                .await
                .and_then(|control| {
                    control
                        .phase_attribution_snapshot()
                        .current_assembly_checkpoint
                });
            if current_checkpoint
                == Some(
                    crate::server::ReadyParseSnapshotAssemblyCheckpointV2::SyntaxErrorCollection,
                )
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange exact worker must enter syntax-error collection before didSave promotion");

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
        .expect("didSave must publish save_fastlane before save-critical exact assembly finishes");
    assert!(
        first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax")),
        "didSave first publish must remain syntax-only before exact ready follow-up, diagnostics={:?}",
        first_publish.diagnostics
    );

    let followup_publish =
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
        .expect("save-critical exact assembly must publish ready artifacts without waiting for syntax-error collection");
    assert!(
        followup_publish
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.source.as_deref() == Some("bsl-analysis-v2")),
        "save-critical exact follow-up must publish semantic diagnostics from ready artifacts, diagnostics={:?}",
        followup_publish.diagnostics
    );

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let syntax_errors_complete = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .filter(|state| state.parse_snapshot.file_version == 2)
                .map(|state| state.syntax_errors_complete);
            if syntax_errors_complete == Some(true) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("deferred syntax-error assembly must eventually enrich the ready snapshot");

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_729, 8).await;
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
        Some("ready")
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
            .get("followup_semantic_path")
            .and_then(|value| value.as_str()),
        Some("ready_artifacts")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_timeout_phase")
            .and_then(|value| value.as_str()),
        None
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_parse_exec_timeout_subphase")
            .and_then(|value| value.as_str()),
        None
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint")
            .and_then(|value| value.as_str()),
        None
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint")
            .and_then(|value| value.as_str()),
        None
    );
    assert!(
        trace
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0 && value < SYNTAX_ERROR_ASSEMBLY_DELAY_MS / 10),
        "save-critical exact publish must bound syntax-error collection far below the configured injected delay, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint")
            .and_then(|value| value.as_str()),
        Some("syntax_error_collection")
    );
    assert!(
        trace
            .get("followup_ready_snapshot_parse_exec_ms")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value < SYNTAX_ERROR_ASSEMBLY_DELAY_MS / 10),
        "save-critical exact publish must bound total parse_exec far below the injected syntax-error assembly delay, trace={trace:?}"
    );
    assert!(
        trace
            .get("followup_ready_snapshot_relief_valve_outcome")
            .is_none(),
        "bounded wait success must not leave stale relief attribution behind, trace={trace:?}"
    );

    let counters = coordinator
        .observability_metrics()
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object")
        .clone();
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_ready"
        )) > 0,
        "save-critical exact publish must export bounded-wait ready probe outcome, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p30_did_save_followup_promotes_exact_program_conversion_past_publishable_artifact_packaging(
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

    const V1_FIXTURE: &str = "Процедура Тест()\n    Сообщить(1);\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    Сообщить(необъявленная);\nКонецПроцедуры\n";
    const PACKAGING_DELAY_MS: u64 = 10_000;
    const FIRST_PUBLISH_BUDGET_MS: u64 = 3_500;
    const FOLLOWUP_PUBLISH_BUDGET_MS: u64 = 5_000;

    let _env_lock = lock_test_env().await;
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);

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
        Url::parse("file:///p30_did_save_followup_publishable_packaging_fixture.bsl").expect("uri");
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
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize version 1 before didSave packaging promotion");

    let _packaging_delay_guard = EnvVarGuard::set(
        "BSL_TEST_PARSE_SNAPSHOT_PUBLISHABLE_ARTIFACT_PACKAGING_DELAY_MS",
        &PACKAGING_DELAY_MS.to_string(),
    );

    let v2_hash = *blake3::hash(V2_FIXTURE.as_bytes()).as_bytes();
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

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let current_checkpoint = server
                .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
                .await
                .and_then(|control| {
                    control
                        .phase_attribution_snapshot()
                        .current_assembly_checkpoint
                });
            if current_checkpoint
                == Some(
                    crate::server::ReadyParseSnapshotAssemblyCheckpointV2::PublishableArtifactPackaging,
                )
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange exact worker must enter publishable packaging before didSave promotion");

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

    let first_publish = tokio::time::timeout(
        Duration::from_millis(FIRST_PUBLISH_BUDGET_MS),
        async {
            loop {
                let params = published_rx
                    .recv()
                    .await
                    .expect("publishDiagnostics channel must stay open");
                if params.uri == uri && params.version == Some(2) {
                    break params;
                }
            }
        },
    )
    .await
    .expect(
        "didSave must publish save_fastlane before save-critical exact program conversion finishes",
    );
    assert!(
        first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax")),
        "didSave first publish must remain syntax-only before exact ready follow-up, diagnostics={:?}",
        first_publish.diagnostics
    );

    let followup_publish =
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
        .expect("save-critical exact program conversion must publish ready artifacts without waiting for publishable packaging");
    assert!(
        followup_publish
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.source.as_deref() == Some("bsl-analysis-v2")),
        "save-critical exact follow-up must publish semantic diagnostics from ready artifacts, diagnostics={:?}",
        followup_publish.diagnostics
    );

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_730, 9).await;
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
            .get("followup_ready_snapshot_wait_probe")
            .and_then(|value| value.as_str()),
        Some("ready")
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
    assert_eq!(
        trace
            .get("followup_ready_snapshot_timeout_phase")
            .and_then(|value| value.as_str()),
        None
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint")
            .and_then(|value| value.as_str()),
        None
    );
    assert!(
        trace
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0 && value < PACKAGING_DELAY_MS / 10),
        "save-critical exact publish must bound publishable packaging far below the configured injected delay, trace={trace:?}"
    );
    assert!(
        trace
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0 && value < PACKAGING_DELAY_MS / 10),
        "save-critical exact publish must bound total program conversion far below the configured injected delay, trace={trace:?}"
    );
    assert!(
        matches!(
            trace.get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint")
                .and_then(|value| value.as_str()),
            Some("program_lowering") | Some("publishable_artifact_packaging")
        ),
        "dominant exact conversion checkpoint must stay inside the bounded p30 slices, trace={trace:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p30_parsed_did_change_revision_is_retargeted_during_program_lowering_when_newer_target_arrives(
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

    fn build_fixture(tag: &str) -> String {
        let mut text = String::new();
        for idx in 0..768 {
            text.push_str(&format!("Процедура Тест{idx}()\n"));
            text.push_str(&format!("    Сообщить(\"{tag}-{idx}\");\n"));
            text.push_str("КонецПроцедуры\n\n");
        }
        text
    }

    let _env_lock = lock_test_env().await;
    let _debounce_guard = EnvVarGuard::set("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0");
    let _conversion_delay_guard = EnvVarGuard::set(
        "BSL_TEST_PARSE_SNAPSHOT_PROGRAM_CONVERSION_PROGRESS_DELAY_MS",
        "25",
    );

    let v1_fixture = build_fixture("v1");
    let v2_fixture = build_fixture("v2");
    let v3_fixture = build_fixture("v3");
    let v2_hash = *blake3::hash(v2_fixture.as_bytes()).as_bytes();

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        &v1_fixture,
        "file:///did_change_retargeted_during_program_lowering_fixture.bsl",
    )
    .await;

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize version 1 before program-lowering retarget test");

    let baseline_metrics = server.coordinator.observability_metrics();
    let baseline_counters = baseline_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("baseline metrics.counters object");

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
                            text: v2_fixture.clone(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange v2 notification");
    assert!(
        did_change_v2_response.is_none(),
        "didChange is a notification"
    );

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let current_checkpoint = server
                .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
                .await
                .and_then(|control| {
                    control
                        .phase_attribution_snapshot()
                        .current_assembly_checkpoint
                });
            if current_checkpoint
                == Some(crate::server::ReadyParseSnapshotAssemblyCheckpointV2::ProgramLowering)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("version 2 didChange worker must enter program lowering before retarget");

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
                            text: v3_fixture.clone(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange v3 notification");
    assert!(
        did_change_v3_response.is_none(),
        "didChange is a notification"
    );

    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if ready_version == Some(3) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("newer target must materialize after program-lowering retarget");

    let final_metrics = server.coordinator.observability_metrics();
    let final_counters = final_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("final metrics.counters object");
    let retargeted_during_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_during_parse";
    let retargeted_before_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_parse";
    let retargeted_before_materialization_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_materialization";
    let aborted_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_aborted";

    let retargeted_during_parse_delta =
        read_u64_metric(final_counters.get(retargeted_during_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_during_parse_key)),
        );
    let retargeted_before_parse_delta =
        read_u64_metric(final_counters.get(retargeted_before_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_parse_key)),
        );
    let retargeted_before_materialization_delta =
        read_u64_metric(final_counters.get(retargeted_before_materialization_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_materialization_key)),
        );
    let aborted_delta = read_u64_metric(final_counters.get(aborted_key))
        .saturating_sub(read_u64_metric(baseline_counters.get(aborted_key)));

    assert!(
        retargeted_during_parse_delta > 0,
        "program-lowering same-file retarget must export the dedicated lifecycle reason, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_parse_delta, 0,
        "program-lowering retarget test must not regress into before-parse attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_materialization_delta, 0,
        "program-lowering retarget test must not regress into before-materialization attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        aborted_delta, 0,
        "program-lowering retarget test must not regress into generic aborted attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );

    let analysis = server.analysis_v2.snapshot().await;
    let observed_text = analysis
        .file_text(file_id)
        .expect("file_text query")
        .expect("file text after program-lowering retarget");
    assert_eq!(observed_text.as_ref(), v3_fixture.as_str());

    drain_task.abort();
}

#[tokio::test]
async fn p31_did_change_revision_is_retargeted_during_program_lowering_inside_single_large_callable_body_when_newer_target_arrives(
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

    fn build_fixture(tag: &str) -> String {
        let mut text = String::from("Процедура Тест()\n");
        for idx in 0..256 {
            text.push_str(&format!("    Сообщить(\"{tag}-{idx}\");\n"));
        }
        text.push_str("КонецПроцедуры\n");
        text
    }

    let _env_lock = lock_test_env().await;
    let _debounce_guard = EnvVarGuard::set("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0");
    let _conversion_delay_guard = EnvVarGuard::set(
        "BSL_TEST_PARSE_SNAPSHOT_PROGRAM_CONVERSION_PROGRESS_DELAY_MS",
        "40",
    );

    let v1_fixture = build_fixture("v1");
    let v2_fixture = build_fixture("v2");
    let v3_fixture = build_fixture("v3");
    let v2_hash = *blake3::hash(v2_fixture.as_bytes()).as_bytes();

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        &v1_fixture,
        "file:///did_change_retargeted_during_single_large_body_program_lowering_fixture.bsl",
    )
    .await;

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize version 1 before single-large-body retarget test");

    let baseline_metrics = server.coordinator.observability_metrics();
    let baseline_counters = baseline_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("baseline metrics.counters object");

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
                            text: v2_fixture.clone(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange v2 notification");
    assert!(
        did_change_v2_response.is_none(),
        "didChange is a notification"
    );

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let current_checkpoint = server
                .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
                .await
                .and_then(|control| {
                    control
                        .phase_attribution_snapshot()
                        .current_assembly_checkpoint
                });
            if current_checkpoint
                == Some(crate::server::ReadyParseSnapshotAssemblyCheckpointV2::ProgramLowering)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("version 2 single-large-body worker must enter program lowering before retarget");

    tokio::time::sleep(Duration::from_millis(150)).await;
    let sustained_checkpoint = server
        .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
        .await
        .and_then(|control| {
            control
                .phase_attribution_snapshot()
                .current_assembly_checkpoint
        });
    assert_eq!(
        sustained_checkpoint,
        Some(crate::server::ReadyParseSnapshotAssemblyCheckpointV2::ProgramLowering),
        "single-large-body fixture must keep the worker inside program lowering long enough to prove callable-body checkpoints"
    );

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
                            text: v3_fixture.clone(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange v3 notification");
    assert!(
        did_change_v3_response.is_none(),
        "didChange is a notification"
    );

    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if ready_version == Some(3) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("newer target must materialize after single-large-body program-lowering retarget");

    let final_metrics = server.coordinator.observability_metrics();
    let final_counters = final_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("final metrics.counters object");
    let retargeted_during_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_during_parse";
    let retargeted_before_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_parse";
    let retargeted_before_materialization_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_materialization";
    let aborted_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_aborted";

    let retargeted_during_parse_delta =
        read_u64_metric(final_counters.get(retargeted_during_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_during_parse_key)),
        );
    let retargeted_before_parse_delta =
        read_u64_metric(final_counters.get(retargeted_before_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_parse_key)),
        );
    let retargeted_before_materialization_delta =
        read_u64_metric(final_counters.get(retargeted_before_materialization_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_materialization_key)),
        );
    let aborted_delta = read_u64_metric(final_counters.get(aborted_key))
        .saturating_sub(read_u64_metric(baseline_counters.get(aborted_key)));

    assert!(
        retargeted_during_parse_delta > 0,
        "single-large-body retarget must export the dedicated lifecycle reason, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_parse_delta, 0,
        "single-large-body retarget test must not regress into before-parse attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_materialization_delta, 0,
        "single-large-body retarget test must not regress into before-materialization attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        aborted_delta, 0,
        "single-large-body retarget test must not regress into generic aborted attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );

    let analysis = server.analysis_v2.snapshot().await;
    let observed_text = analysis
        .file_text(file_id)
        .expect("file_text query")
        .expect("file text after single-large-body retarget");
    assert_eq!(observed_text.as_ref(), v3_fixture.as_str());

    drain_task.abort();
}

#[tokio::test]
async fn p30_parsed_did_change_revision_is_retargeted_during_publishable_artifact_packaging_when_newer_target_arrives(
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

    fn build_fixture(tag: &str) -> String {
        let mut text = String::from("Процедура Тест()\n");
        for idx in 0..256 {
            text.push_str(&format!("    Сообщить(\"{tag}-{idx}\");\n"));
        }
        text.push_str("КонецПроцедуры\n");
        text
    }

    let _env_lock = lock_test_env().await;
    let _debounce_guard = EnvVarGuard::set("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0");
    let _packaging_delay_guard = EnvVarGuard::set(
        "BSL_TEST_PARSE_SNAPSHOT_PUBLISHABLE_ARTIFACT_PACKAGING_DELAY_MS",
        "10000",
    );

    let v1_fixture = build_fixture("v1");
    let v2_fixture = build_fixture("v2");
    let v3_fixture = build_fixture("v3");
    let v2_hash = *blake3::hash(v2_fixture.as_bytes()).as_bytes();

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        &v1_fixture,
        "file:///did_change_retargeted_during_publishable_artifact_packaging_fixture.bsl",
    )
    .await;

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("opened fixture must materialize version 1 before publishable-packaging retarget test");

    let baseline_metrics = server.coordinator.observability_metrics();
    let baseline_counters = baseline_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("baseline metrics.counters object");

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
                            text: v2_fixture.clone(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange v2 notification");
    assert!(
        did_change_v2_response.is_none(),
        "didChange is a notification"
    );

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let current_checkpoint = server
                .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
                .await
                .and_then(|control| {
                    control
                        .phase_attribution_snapshot()
                        .current_assembly_checkpoint
                });
            if current_checkpoint
                == Some(
                    crate::server::ReadyParseSnapshotAssemblyCheckpointV2::PublishableArtifactPackaging,
                )
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("version 2 didChange worker must enter publishable packaging before retarget");

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
                            text: v3_fixture.clone(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange v3 notification");
    assert!(
        did_change_v3_response.is_none(),
        "didChange is a notification"
    );

    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if ready_version == Some(3) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("newer target must materialize after publishable-packaging retarget");

    let final_metrics = server.coordinator.observability_metrics();
    let final_counters = final_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("final metrics.counters object");
    let retargeted_during_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_during_parse";
    let retargeted_before_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_parse";
    let retargeted_before_materialization_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_materialization";
    let aborted_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_aborted";

    let retargeted_during_parse_delta =
        read_u64_metric(final_counters.get(retargeted_during_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_during_parse_key)),
        );
    let retargeted_before_parse_delta =
        read_u64_metric(final_counters.get(retargeted_before_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_parse_key)),
        );
    let retargeted_before_materialization_delta =
        read_u64_metric(final_counters.get(retargeted_before_materialization_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_materialization_key)),
        );
    let aborted_delta = read_u64_metric(final_counters.get(aborted_key))
        .saturating_sub(read_u64_metric(baseline_counters.get(aborted_key)));

    assert!(
        retargeted_during_parse_delta > 0,
        "publishable-packaging same-file retarget must export the dedicated lifecycle reason, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_parse_delta, 0,
        "publishable-packaging retarget test must not regress into before-parse attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_materialization_delta, 0,
        "publishable-packaging retarget test must not regress into before-materialization attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        aborted_delta, 0,
        "publishable-packaging retarget test must not regress into generic aborted attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );

    let analysis = server.analysis_v2.snapshot().await;
    let observed_text = analysis
        .file_text(file_id)
        .expect("file_text query")
        .expect("file text after publishable-packaging retarget");
    assert_eq!(observed_text.as_ref(), v3_fixture.as_str());

    drain_task.abort();
}
