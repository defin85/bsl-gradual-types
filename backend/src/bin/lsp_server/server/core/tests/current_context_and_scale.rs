use super::*;

#[tokio::test]
async fn p33_get_current_context_uses_parse_snapshot_without_warming_exact_type_index() {
    let fixture = concat!(
        "Процедура Тест(ПервыйПараметр, ВторойПараметр)\n",
        "    Если Истина Тогда\n",
        "        Сообщить(ПервыйПараметр);\n",
        "    КонецЕсли;\n",
        "КонецПроцедуры\n",
    );

    let (mut service, drain_task, server, uri, file_id) =
        open_lsp_fixture_with_snapshot(fixture, "file:///current_context_fixture.bsl").await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;
    crate::server::command_handlers::reset_get_current_context_parse_attempts_for_test();

    let execute = Request::build("workspace/executeCommand")
        .id(13301)
        .params(serde_json::json!({
            "command": "bsl.getCurrentContext",
            "arguments": [{
                "uri": uri.to_string(),
                "line": 2,
                "character": 18,
            }],
        }))
        .finish();
    let execute_response = tokio::time::timeout(Duration::from_secs(2), async {
        service
            .ready()
            .await
            .unwrap()
            .call(execute)
            .await
            .expect("workspace/executeCommand request")
    })
    .await
    .expect("bsl.getCurrentContext timeout")
    .expect("workspace/executeCommand response");

    let value = serde_json::to_value(&execute_response).expect("serialize response");
    let result = value.get("result").cloned().expect("result field");
    assert_eq!(
        result.get("functionName").and_then(|value| value.as_str()),
        Some("Тест"),
        "current context must resolve enclosing procedure name from parse snapshot"
    );
    assert_eq!(
        result.get("functionKind").and_then(|value| value.as_str()),
        Some("procedure"),
        "current context must resolve enclosing routine kind from parse snapshot"
    );
    let params = result
        .get("params")
        .and_then(|value| value.as_array())
        .expect("current context params array");
    let params = params
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        params,
        vec!["ПервыйПараметр", "ВторойПараметр"],
        "current context must surface routine parameters without exact type index"
    );
    assert_eq!(
        crate::server::command_handlers::get_current_context_parse_attempts_for_test(),
        0,
        "getCurrentContext must reuse latest ready parse snapshot instead of launching a same-version auxiliary parse"
    );
    let metrics = server.coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_role_total_role_ready_snapshot")
        ),
        1,
        "ready parse snapshot path must record an explicit ready_snapshot role"
    );
    assert_eq!(
        read_u64_metric(
            counters
                .get("intellisense_v2_current_context_parse_source_total_source_ready_snapshot")
        ),
        1,
        "ready parse snapshot path must record ready_snapshot as its parse source"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_terminal_total_outcome_resolved")
        ),
        1,
        "ready parse snapshot request must resolve through the explicit current-context contract"
    );
    let report = serde_json::json!({
        "change_id": "refactor-11-current-context-parse-broker-bounding",
        "profile": "current_context_ready_snapshot_smoke",
        "command": "cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_uses_parse_snapshot_without_warming_exact_type_index -- --nocapture",
        "summary": {
            "parse_attempts": crate::server::command_handlers::get_current_context_parse_attempts_for_test(),
            "ready_snapshot_role_total": read_u64_metric(
                counters.get("intellisense_v2_current_context_role_total_role_ready_snapshot")
            ),
            "ready_snapshot_source_total": read_u64_metric(
                counters.get("intellisense_v2_current_context_parse_source_total_source_ready_snapshot")
            ),
            "broker_leader_total": read_u64_metric(
                counters.get("intellisense_v2_current_context_role_total_role_broker_leader")
            ),
            "broker_follower_total": read_u64_metric(
                counters.get("intellisense_v2_current_context_role_total_role_broker_follower")
            ),
            "resolved_total": read_u64_metric(
                counters.get("intellisense_v2_current_context_terminal_total_outcome_resolved")
            ),
        },
        "selected_histograms": {
            "current_context_parse_ms_role_ready_snapshot": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_current_context_parse_ms_role_ready_snapshot",
                None
            ),
            "current_context_wall_ms_role_ready_snapshot": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_current_context_wall_ms_role_ready_snapshot",
                None
            ),
            "current_context_wall_ms_source_ready_snapshot": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_current_context_wall_ms_source_ready_snapshot",
                None
            ),
        },
    });
    let report_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("perf")
        .join("reports")
        .join("refactor-11-current-context-parse-broker-bounding-ready-snapshot-smoke.json");
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("create current-context ready-snapshot smoke report dir");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report)
            .expect("serialize current-context ready-snapshot report"),
    )
    .expect("write current-context ready-snapshot report");

    let exact_ready = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready after getCurrentContext");
    assert!(
        !exact_ready,
        "getCurrentContext must not eagerly warm exact type index on the request path"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_get_current_context_briefly_waits_for_equivalent_snapshot_worker_before_broker_parse()
{
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

    const V1_FIXTURE: &str = concat!(
        "Процедура Тест(ПервыйПараметр, ВторойПараметр)\n",
        "    Если Истина Тогда\n",
        "        Сообщить(ПервыйПараметр);\n",
        "    КонецЕсли;\n",
        "КонецПроцедуры\n",
    );
    const V2_SUFFIX: &str = "\n// exact snapshot wait\n";
    const DID_CHANGE_PARSE_DELAY_MS: u64 = 300;
    const READY_SNAPSHOT_WAIT_BUDGET_MS: u64 = 600;

    let _env_lock = lock_test_env().await;
    let _did_change_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &DID_CHANGE_PARSE_DELAY_MS.to_string(),
    );
    let _ready_snapshot_wait_guard = EnvVarGuard::set(
        "BSL_TEST_GET_CURRENT_CONTEXT_READY_SNAPSHOT_WAIT_BUDGET_MS",
        &READY_SNAPSHOT_WAIT_BUDGET_MS.to_string(),
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

    let uri = Url::parse("file:///current_context_exact_worker_wait_fixture.bsl").expect("uri");
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
    let v2_text = format!("{V1_FIXTURE}{V2_SUFFIX}");
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
                            text: v2_text.clone(),
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
    .expect(
        "didChange must register an exact same-version snapshot worker before getCurrentContext",
    );

    crate::server::command_handlers::reset_get_current_context_parse_attempts_for_test();
    let position = find_utf16_position_after_marker(&v2_text, "Сообщить(ПервыйПараметр");
    let execute_response = tokio::time::timeout(Duration::from_secs(5), async {
        service
            .ready()
            .await
            .unwrap()
            .call(
                Request::build("workspace/executeCommand")
                    .id(13302)
                    .params(serde_json::json!({
                        "command": "bsl.getCurrentContext",
                        "arguments": [{
                            "uri": uri.to_string(),
                            "line": position.line,
                            "character": position.character,
                        }],
                    }))
                    .finish(),
            )
            .await
            .expect("workspace/executeCommand request")
    })
    .await
    .expect("bsl.getCurrentContext timeout")
    .expect("workspace/executeCommand response");

    let response_value = serde_json::to_value(&execute_response).expect("serialize response");
    let result = response_value.get("result").cloned().expect("result field");
    assert_eq!(
        result.get("functionName").and_then(|value| value.as_str()),
        Some("Тест"),
        "getCurrentContext must resolve against the equivalent same-version snapshot worker once it materializes"
    );
    assert_eq!(
        crate::server::command_handlers::get_current_context_parse_attempts_for_test(),
        0,
        "getCurrentContext must not launch an independent broker parse when the equivalent exact worker wins within the bounded wait"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_role_total_role_ready_snapshot")
        ),
        1,
        "equivalent exact worker reuse must still resolve through the ready_snapshot role"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_role_total_role_broker_leader")
        ),
        0,
        "equivalent exact worker reuse must not fall through to broker leader parse"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_role_total_role_broker_follower")
        ),
        0,
        "equivalent exact worker reuse must not fall through to broker follower parse"
    );

    drain_task.abort();
}

#[test]
fn scale_aware_progress_emits_start_step_and_finish() {
    assert!(should_emit_scale_aware_progress(0, 55, 10));
    assert!(should_emit_scale_aware_progress(9, 55, 10));
    assert!(should_emit_scale_aware_progress(54, 55, 10));
}

#[test]
fn scale_aware_phase_plan_defaults_match_acceptance_contract() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn unset(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, previous }
        }

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
    let _guards = [
        EnvVarGuard::unset("BSL_V2_SCALE_AWARE_START_ITERATIONS"),
        EnvVarGuard::unset("BSL_V2_SCALE_AWARE_COLD_ITERATIONS"),
        EnvVarGuard::unset("BSL_V2_SCALE_AWARE_WARM_WARMUP"),
        EnvVarGuard::unset("BSL_V2_SCALE_AWARE_WARM_ITERATIONS"),
        EnvVarGuard::unset("BSL_V2_SCALE_AWARE_REQUIRED_WARM_SAMPLES"),
    ];

    let phases = scale_aware_phase_plan_from_env();
    assert_eq!(phases[0].name, "start");
    assert_eq!(phases[0].warmup, 0);
    assert_eq!(phases[0].iterations, 1);
    assert_eq!(phases[1].name, "cold");
    assert_eq!(phases[1].warmup, 0);
    assert_eq!(phases[1].iterations, 5);
    assert_eq!(phases[2].name, "warm");
    assert_eq!(phases[2].warmup, 5);
    assert_eq!(phases[2].iterations, 50);
    assert_eq!(scale_aware_required_warm_samples_from_env(), 50);

    let _override_guard = EnvVarGuard::set("BSL_V2_SCALE_AWARE_START_ITERATIONS", "7");
    assert_eq!(scale_aware_phase_plan_from_env()[0].iterations, 7);
}

#[test]
fn scale_aware_phase_plan_accepts_local_debug_overrides() {
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
    let _guards = [
        EnvVarGuard::set("BSL_V2_SCALE_AWARE_START_ITERATIONS", "2"),
        EnvVarGuard::set("BSL_V2_SCALE_AWARE_COLD_ITERATIONS", "1"),
        EnvVarGuard::set("BSL_V2_SCALE_AWARE_WARM_WARMUP", "1"),
        EnvVarGuard::set("BSL_V2_SCALE_AWARE_WARM_ITERATIONS", "4"),
        EnvVarGuard::set("BSL_V2_SCALE_AWARE_REQUIRED_WARM_SAMPLES", "4"),
    ];

    let phases = scale_aware_phase_plan_from_env();
    assert_eq!(phases[0].iterations, 2);
    assert_eq!(phases[1].iterations, 1);
    assert_eq!(phases[2].warmup, 1);
    assert_eq!(phases[2].iterations, 4);
    assert_eq!(scale_aware_required_warm_samples_from_env(), 4);
}

#[test]
fn scale_aware_progress_skips_intermediate_non_step_points() {
    assert!(!should_emit_scale_aware_progress(1, 55, 10));
    assert!(!should_emit_scale_aware_progress(8, 55, 10));
    assert!(!should_emit_scale_aware_progress(53, 55, 10));
    assert!(!should_emit_scale_aware_progress(0, 0, 10));
}

#[test]
fn scale_aware_progress_percent_and_eta_are_stable() {
    let elapsed = Duration::from_millis(2_500);
    let completed = 5;
    let total = 10;
    let percent = scale_aware_progress_percent(completed, total);
    let eta_ms = scale_aware_progress_eta_ms(elapsed, completed, total);
    assert!((percent - 50.0).abs() < f64::EPSILON);
    assert_eq!(eta_ms, 2_500);
    assert_eq!(scale_aware_progress_eta_ms(elapsed, 0, total), 0);
    assert_eq!(scale_aware_progress_eta_ms(elapsed, total, total), 0);
}

#[test]
fn scale_aware_dominant_stage_includes_completion_pipeline_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_wait_for_file_version_completion_ms": {"p95": 2.0},
        "intellisense_v2_snapshot_completion_ms": {"p95": 1.0},
        "intellisense_v2_ir_query_completion_ms": {"p95": 3.0},
        "intellisense_v2_runtime_queue_wait_interactive_ms": {"p95": 2.0},
        "intellisense_v2_syntax_diagnostics_query_ms": {"p95": 0.0},
        "intellisense_v2_semantic_diagnostics_query_ms": {"p95": 10.0},
        "completion_stage_snapshot_read_ms": {"p95": 15.0},
        "completion_stage_collect_ms": {"p95": 120.0},
        "completion_stage_rank_ms": {"p95": 8.0},
        "completion_stage_format_ms": {"p95": 6.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_collect"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        120.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_completion_turn_wait_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_wait_for_file_version_completion_ms": {"p95": 2.0},
        "intellisense_v2_snapshot_completion_ms": {"p95": 1.0},
        "intellisense_v2_ir_query_completion_ms": {"p95": 3.0},
        "intellisense_v2_runtime_queue_wait_interactive_ms": {"p95": 2.0},
        "intellisense_v2_syntax_diagnostics_query_ms": {"p95": 0.0},
        "intellisense_v2_semantic_diagnostics_query_ms": {"p95": 280.0},
        "completion_stage_turn_wait_ms": {"p95": 1500.0},
        "completion_stage_prepare_stateful_ms": {"p95": 20.0},
        "completion_stage_sync_globals_ms": {"p95": 5.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_turn_wait"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        1500.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_completion_query_bundle_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_wait_for_file_version_completion_ms": {"p95": 2.0},
        "intellisense_v2_snapshot_completion_ms": {"p95": 1.0},
        "intellisense_v2_ir_query_completion_ms": {"p95": 9.0},
        "intellisense_v2_runtime_queue_wait_interactive_ms": {"p95": 2.0},
        "intellisense_v2_semantic_diagnostics_query_ms": {"p95": 320.0},
        "intellisense_v2_parse_result_query_ms": {"p95": 120.0},
        "intellisense_v2_singleflight_wait_ms": {"p95": 40.0},
        "intellisense_v2_runtime_exec_interactive_ms": {"p95": 25.0},
        "completion_stage_query_bundle_ir_query_ms": {"p95": 2400.0},
        "completion_stage_response_build_ms": {"p95": 50.0},
        "completion_stage_cache_store_ms": {"p95": 30.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_ir_query"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        2400.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_completion_query_bundle_owner_hint_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_wait_for_file_version_completion_ms": {"p95": 2.0},
        "intellisense_v2_snapshot_completion_ms": {"p95": 1.0},
        "intellisense_v2_ir_query_completion_ms": {"p95": 9.0},
        "intellisense_v2_runtime_queue_wait_interactive_ms": {"p95": 2.0},
        "intellisense_v2_semantic_diagnostics_query_ms": {"p95": 320.0},
        "completion_stage_query_bundle_owner_hint_ms": {"p95": 3500.0},
        "completion_stage_query_bundle_deps_and_file_snapshot_ms": {"p95": 100.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_owner_hint"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        3500.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_owner_hint_index_fetch_wait_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_wait_for_file_version_completion_ms": {"p95": 2.0},
        "intellisense_v2_snapshot_completion_ms": {"p95": 1.0},
        "intellisense_v2_ir_query_completion_ms": {"p95": 9.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms": {"p95": 2800.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_ms": {"p95": 2700.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_build_total_ms": {"p95": 100.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_owner_hint"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        2800.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_owner_hint_index_fetch_inside_salsa_window_breakdown() {
    let metrics = serde_json::json!({
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms": {"p95": 2000.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window_ms": {"p95": 3100.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait_ms": {"p95": 100.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail_ms": {"p95": 50.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_owner_hint"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        3100.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_owner_hint_first_will_check_to_first_will_execute_breakdown()
{
    let metrics = serde_json::json!({
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms": {"p95": 2000.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation_ms": {"p95": 1200.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms": {"p95": 1500.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index_ms": {"p95": 3300.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_owner_hint"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        3300.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_owner_hint_first_will_execute_other_breakdown() {
    let metrics = serde_json::json!({
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms": {"p95": 2000.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms": {"p95": 1500.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result_ms": {"p95": 1700.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other_ms": {"p95": 3400.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_owner_hint"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        3400.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_owner_hint_will_iterate_cycle_breakdown() {
    let metrics = serde_json::json!({
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms": {"p95": 1500.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle_ms": {"p95": 3600.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle_ms": {"p95": 3400.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_owner_hint"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        3600.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_runtime_apply_changes_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_runtime_apply_changes_queue_wait_ms": {"p95": 3500.0},
        "intellisense_v2_runtime_apply_changes_exec_ms": {"p95": 3200.0},
        "intellisense_v2_runtime_apply_change_set_file_exec_ms": {"p95": 2800.0},
        "completion_stage_query_bundle_ir_query_ms": {"p95": 1200.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "runtime_apply_changes_queue_wait"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        3500.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_type_index_precompute_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_runtime_type_index_precompute_queue_wait_ms": {"p95": 120.0},
        "intellisense_v2_runtime_type_index_precompute_exec_ms": {"p95": 4800.0},
        "intellisense_v2_runtime_type_index_precompute_build_exec_ms": {"p95": 14.0},
        "intellisense_v2_runtime_type_index_precompute_ir_exec_ms": {"p95": 4700.0},
        "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms": {"p95": 1900.0},
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms": {"p95": 2600.0},
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms": {"p95": 120.0},
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms": {"p95": 4100.0},
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms": {"p95": 320.0},
        "completion_stage_prepare_stateful_ms": {"p95": 121.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "runtime_type_index_precompute_exec"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        4800.0
    );
    assert_eq!(
        dominant
            .get("candidates_p95_ms")
            .and_then(|value| value
                .get("runtime_type_index_precompute_semantic_facts_local_function_summaries_exec"))
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        4100.0
    );
}

#[test]
fn scale_aware_baseline_schema_requires_explicit_pass_fail_summary() {
    let baseline = synthetic_scale_aware_report("baseline", 100.0, 100.0, 100.0, 0.0);
    let err = validate_scale_aware_baseline_schema_for_acceptance(&baseline)
        .expect_err("baseline without gate.pass must be rejected");
    assert!(
        err.contains("gate.pass"),
        "expected error mentioning gate.pass, got: {err}"
    );
}

#[test]
fn scale_aware_baseline_schema_accepts_required_shape() {
    let mut baseline = synthetic_scale_aware_report("baseline", 100.0, 100.0, 100.0, 0.0);
    baseline["gate"] = serde_json::json!({
        "pass": true
    });
    validate_scale_aware_baseline_schema_for_acceptance(&baseline)
        .expect("baseline with required gate summary and metrics should validate");
}

#[tokio::test]
async fn p31_scale_aware_large_small_completion_gate_live() {
    init_test_tracing();
    const CHANGE_ID: &str = "add-bounded-stale-completion-fastpath";
    let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();

    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let Some(conf_big_root) = conf_big_root_for_tests() else {
        if allow_fixture_skip {
            eprintln!(
                "skipping p31 scale-aware gate: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
            );
            return;
        }
        panic!(
            "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly"
        );
    };

    let large_module_path = conf_big_large_module_path_for_tests(&conf_big_root);
    if !large_module_path.exists() {
        if allow_fixture_skip {
            eprintln!(
                "skipping p31 scale-aware gate: conf_big module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
                large_module_path.display()
            );
            return;
        }
        panic!(
            "conf_big module fixture is missing: {}; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly",
            large_module_path.display()
        );
    }

    let small_module_path = workspace_root.join("examples").join("test_lsp.bsl");
    assert!(
        small_module_path.exists(),
        "small module fixture not found: {}",
        small_module_path.display()
    );

    let large_text = std::fs::read_to_string(&large_module_path)
        .expect("read conf_big module text for p31 scale-aware gate");
    let small_text = std::fs::read_to_string(&small_module_path)
        .expect("read small module text for p31 scale-aware gate");

    let large_position = find_utf16_position_after_marker(&large_text, "Объект.");
    let small_position = find_utf16_position_after_marker(&small_text, "Arr.");
    let phases = scale_aware_phase_plan_from_env();
    let required_warm_samples = scale_aware_required_warm_samples_from_env();
    let churn_mode = scale_aware_churn_mode_from_env();
    let churn_every = scale_aware_churn_every_from_env();

    let large_profile = run_scale_aware_profile(
        "large",
        Url::parse("file:///p31_scale_large_module.bsl").expect("large uri"),
        large_text,
        large_position,
        &phases,
        churn_mode,
        churn_every,
        None,
        None,
    )
    .await;
    let small_profile = run_scale_aware_profile(
        "small",
        Url::parse("file:///p31_scale_small_module.bsl").expect("small uri"),
        small_text,
        small_position,
        &phases,
        churn_mode,
        churn_every,
        None,
        None,
    )
    .await;

    let mut report = serde_json::json!({
        "change_id": CHANGE_ID,
        "profile": "p31_scale_aware_large_small_completion_gate_live",
        "schema_version": 1,
        "phases": phases.iter().map(|phase| {
            serde_json::json!({
                "name": phase.name,
                "warmup": phase.warmup,
                "iterations": phase.iterations
            })
        }).collect::<Vec<_>>(),
        "churn": {
            "mode": churn_mode.as_str(),
            "every": churn_every
        },
        "requirements": {
            "required_warm_samples": required_warm_samples
        },
        "profiles": {
            "large": large_profile,
            "small": small_profile
        }
    });

    let baseline_path = std::env::var("BSL_V2_SCALE_AWARE_GATE_BASELINE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("baselines")
                .join(format!("{CHANGE_ID}.json"))
        });
    let enforce_gate = std::env::var("BSL_V2_SCALE_AWARE_GATE_ENFORCE")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !baseline_path.exists() {
        panic!(
            "scale-aware baseline is required but missing: {}",
            baseline_path.display()
        );
    }

    let baseline_raw =
        std::fs::read_to_string(&baseline_path).expect("read scale-aware baseline file");
    let baseline_report: serde_json::Value =
        serde_json::from_str(&baseline_raw).expect("parse scale-aware baseline json");
    validate_scale_aware_baseline_schema_for_acceptance(&baseline_report)
        .expect("validate scale-aware baseline schema");
    let gate = evaluate_scale_aware_gate_for_acceptance(&report, &baseline_report)
        .expect("evaluate scale-aware large/small gate");
    report["baseline"] = serde_json::json!({
        "path": baseline_path,
        "present": true
    });
    report["gate"] = gate.clone();

    if enforce_gate {
        let pass = gate
            .get("pass")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        assert!(
            pass,
            "p31 scale-aware gate failed in enforce mode: {}",
            serde_json::to_string_pretty(&gate).unwrap_or_else(|_| "<gate json>".to_string())
        );
    }

    let report_path = std::env::var("BSL_V2_SCALE_AWARE_GATE_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!("{CHANGE_ID}-live.json"))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for p31 scale-aware report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize p31 scale-aware report"),
    )
    .expect("write p31 scale-aware report");
    println!("p31_scale_aware_gate_report={}", report_path.display());

    let large_warm_total =
        get_report_u64(&report, &["profiles", "large", "warm", "completion_total"])
            .expect("large warm completion_total");
    let small_warm_total =
        get_report_u64(&report, &["profiles", "small", "warm", "completion_total"])
            .expect("small warm completion_total");
    assert!(
        large_warm_total >= required_warm_samples && small_warm_total >= required_warm_samples,
        "expected >={required_warm_samples} warm completion samples for both profiles, got large={} small={}",
        large_warm_total,
        small_warm_total
    );
}

#[tokio::test]
async fn p36_real_conf_big_completion_and_observability_gate_live() {
    init_test_tracing();
    let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();

    let Some(conf_big_root) = conf_big_root_for_tests() else {
        if allow_fixture_skip {
            eprintln!(
                "skipping p36 real conf_big gate: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
            );
            return;
        }
        panic!(
            "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly"
        );
    };

    let module_path = conf_big_large_module_path_for_tests(&conf_big_root);
    if !module_path.exists() {
        if allow_fixture_skip {
            eprintln!(
                "skipping p36 real conf_big gate: module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
                module_path.display()
            );
            return;
        }
        panic!(
            "conf_big module fixture is missing: {}; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly",
            module_path.display()
        );
    }

    let module_text =
        std::fs::read_to_string(&module_path).expect("read conf_big module text for p36 gate");
    let phases = real_module_phase_plan_from_env();
    let required_warm_samples = real_module_required_warm_samples_from_env();
    let churn_mode = scale_aware_churn_mode_from_env();
    let churn_every = scale_aware_churn_every_from_env();
    let observability_probe = real_module_observability_probe_from_env();
    let workspace_setup = ScaleAwareWorkspaceSetup {
        platform_docs_archive: syntax_helper_path_for_tests(),
        configuration_path: conf_big_root.clone(),
        platform_version: "8.3.25".to_string(),
    };
    let profile = run_scale_aware_profile(
        "large",
        Url::from_file_path(&module_path).expect("real conf_big module uri"),
        module_text.clone(),
        find_utf16_position_after_marker(&module_text, "Объект."),
        &phases,
        churn_mode,
        churn_every,
        Some(&workspace_setup),
        Some(observability_probe),
    )
    .await;

    let mut report = serde_json::json!({
        "change_id": "refactor-ir-canonical-semantic-pipeline",
        "profile": "p36_real_conf_big_completion_and_observability_gate_live",
        "schema_version": 1,
        "configuration_path": conf_big_root,
        "module_path": module_path,
        "phases": phases.iter().map(|phase| {
            serde_json::json!({
                "name": phase.name,
                "warmup": phase.warmup,
                "iterations": phase.iterations
            })
        }).collect::<Vec<_>>(),
        "churn": {
            "mode": churn_mode.as_str(),
            "every": churn_every
        },
        "requirements": {
            "required_warm_samples": required_warm_samples
        },
        "observability_probe": {
            "every": observability_probe.every,
            "timeout_ms": observability_probe.timeout.as_millis(),
        },
        "profile_report": profile,
    });

    let report_path = std::env::var("BSL_V2_REAL_MODULE_GATE_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join("real-conf-big-completion-observability-live.json")
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for p36 real-module report");
    }

    let warm_completion_total =
        get_report_u64(&report, &["profile_report", "warm", "completion_total"])
            .expect("real-module warm completion_total");
    let warm_fail_closed_total = get_report_u64(
        &report,
        &[
            "profile_report",
            "warm",
            "completion_outcomes",
            "fail_closed",
        ],
    )
    .expect("real-module warm fail_closed");
    let warm_ok_non_empty_total = get_report_u64(
        &report,
        &[
            "profile_report",
            "warm",
            "completion_outcomes",
            "ok_non_empty",
        ],
    )
    .expect("real-module warm ok_non_empty");
    let warm_observability_timeout_total = get_report_u64(
        &report,
        &[
            "profile_report",
            "warm",
            "observability_sidebar_probe",
            "timeout_total",
        ],
    )
    .expect("real-module warm observability timeout_total");
    report["summary"] = serde_json::json!({
        "warm_completion_total": warm_completion_total,
        "warm_fail_closed_total": warm_fail_closed_total,
        "warm_ok_non_empty_total": warm_ok_non_empty_total,
        "warm_observability_timeout_total": warm_observability_timeout_total,
    });

    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize p36 real-module report"),
    )
    .expect("write p36 real-module report");
    println!("p36_real_module_gate_report={}", report_path.display());

    assert!(
        warm_completion_total >= required_warm_samples,
        "expected >={required_warm_samples} warm completion samples for real module, got {}",
        warm_completion_total
    );
}

#[test]
fn p37_real_conf_big_warm_cache_completion_perf_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p37 tokio runtime");
    runtime.block_on(async {
    init_test_tracing();
    let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
    const PROFILE_NAME: &str = "p37_real_conf_big_warm_cache_completion_perf_report_live";
    let change_id = std::env::var("CHANGE_ID")
        .unwrap_or_else(|_| "refactor-completion-prepare-lightweight-exact-split".to_string());
    const WARMUP_REQUESTS: usize = 5;
    const MEASURE_REQUESTS: usize = 4;
    const WARM_HEAD_PATH_P95_BUDGET_MS: f64 = 150.0;

    let Some(conf_big_root) = conf_big_root_for_tests() else {
        if allow_fixture_skip {
            eprintln!(
                "skipping {PROFILE_NAME}: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
            );
            return;
        }
        panic!(
            "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly"
        );
    };

    let module_path = conf_big_large_module_path_for_tests(&conf_big_root);
    if !module_path.exists() {
        if allow_fixture_skip {
            eprintln!(
                "skipping {PROFILE_NAME}: module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
                module_path.display()
            );
            return;
        }
        panic!(
            "conf_big module fixture is missing: {}; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly",
            module_path.display()
        );
    }

    let module_text =
        std::fs::read_to_string(&module_path).expect("read conf_big module text for p37 report");
    let workspace_setup = ScaleAwareWorkspaceSetup {
        platform_docs_archive: syntax_helper_path_for_tests(),
        configuration_path: conf_big_root.clone(),
        platform_version: "8.3.25".to_string(),
    };
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
    prime_server_with_workspace_setup(&server, &workspace_setup, "p37_real_conf_big_live_setup")
        .await;

    let uri = Url::from_file_path(&module_path).expect("real conf_big module uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: module_text.clone(),
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
    let opened_version = server
        .latest_received_file_versions_v2
        .read()
        .await
        .get(&file_id)
        .copied()
        .expect("latest received version for p37 opened file");
    assert_eq!(
        opened_version, 1,
        "real conf_big fixture must open at version 1"
    );
    assert!(
        server
            .analysis_v2
            .wait_for_file_version(file_id, opened_version)
            .await,
        "analysis runtime must catch up to opened real conf_big file version"
    );
    let exact_type_index_seed = serde_json::json!({
        "mode": "background_only",
    });

    let completion_position = find_utf16_position_after_marker(&module_text, "Объект.");
    let completion_context = Some(CompletionContext {
        trigger_kind: CompletionTriggerKind::INVOKED,
        trigger_character: None,
    });

    let mut warmup_samples = Vec::new();
    for index in 0..WARMUP_REQUESTS {
        let request_id = 37_100_000_i64 + index as i64;
        let started = Instant::now();
        let labels = lsp_completion_labels_with_request(
            &mut service,
            request_id,
            &uri,
            completion_position,
            completion_context.clone(),
        )
        .await;
        warmup_samples.push(serde_json::json!({
            "step": format!("warmup_completion_{}", index + 1),
            "request_id": request_id,
            "elapsed_ms": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            "label_count": labels.len(),
            "labels": labels,
            "version": opened_version,
        }));
    }

    let mut measured_samples = Vec::new();
    for index in 0..MEASURE_REQUESTS {
        let request_id = 37_100_100_i64 + index as i64;
        let started = Instant::now();
        let labels = lsp_completion_labels_with_request(
            &mut service,
            request_id,
            &uri,
            completion_position,
            completion_context.clone(),
        )
        .await;
        measured_samples.push(serde_json::json!({
            "step": format!("measured_warm_completion_{}", index + 1),
            "request_id": request_id,
            "elapsed_ms": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            "label_count": labels.len(),
            "labels": labels,
            "version": opened_version,
        }));
    }

    let completion_timeline = lsp_get_completion_timeline(&mut service, 37_100_900, 64).await;
    let observability_metrics = lsp_get_observability_metrics(&mut service, 37_100_901).await;
    let timeline_traces = completion_timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let filtered_traces: Vec<serde_json::Value> = timeline_traces
        .iter()
        .filter(|trace| trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str()))
        .cloned()
        .collect();
    assert!(
        !filtered_traces.is_empty(),
        "expected non-empty completion timeline traces for real conf_big module"
    );

    let histograms = observability_metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    let counters = observability_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");

    let total_sample_count = WARMUP_REQUESTS + MEASURE_REQUESTS;
    let trace_request_id_present_total = filtered_traces
        .iter()
        .filter(|trace| {
            trace
                .get("request_id")
                .and_then(|value| value.as_str())
                .is_some()
        })
        .count();
    let trace_matching_mode = if trace_request_id_present_total > 0 {
        "request_id"
    } else {
        "ordinal_by_filtered_uri_trace_order"
    };
    let fallback_trace_window: Vec<serde_json::Value> =
        if filtered_traces.len() >= total_sample_count {
            filtered_traces[filtered_traces.len() - total_sample_count..].to_vec()
        } else {
            filtered_traces.clone()
        };

    let enrich_samples = |samples: Vec<serde_json::Value>,
                          sample_offset: usize|
     -> Vec<serde_json::Value> {
        samples
                .into_iter()
                .enumerate()
                .map(|(sample_index, sample)| {
                    let request_id_text = sample
                        .get("request_id")
                        .and_then(|value| value.as_i64())
                        .map(|value| value.to_string());
                    let trace = if trace_request_id_present_total > 0 {
                        request_id_text.as_ref().and_then(|request_id| {
                            filtered_traces.iter().find(|trace| {
                                trace.get("request_id").and_then(|value| value.as_str())
                                    == Some(request_id)
                            })
                        })
                    } else {
                        fallback_trace_window.get(sample_offset + sample_index)
                    };
                let trace_summary = trace.map(|trace| {
                    serde_json::json!({
                        "trace_id": trace.get("trace_id").and_then(|value| value.as_str()),
                        "request_id": trace.get("request_id").and_then(|value| value.as_str()),
                        "trigger_mode": trace.get("trigger_mode").and_then(|value| value.as_str()),
                        "outcome": trace.get("outcome").and_then(|value| value.as_str()),
                        "route": completion_timeline_prepare_detail_str(trace, "route"),
                        "prepare_kind": completion_timeline_prepare_detail_str(trace, "kind"),
                        "fail_closed_cause": completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
                        "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                        "dominant_stage": trace.get("dominant_stage").and_then(|value| value.as_str()),
                        "queue_outcome": trace.get("queue_outcome").and_then(|value| value.as_str()),
                        "turn_wait_outcome": trace.get("turn_wait_outcome").and_then(|value| value.as_str()),
                        "queued_completion_ahead": trace.get("queued_completion_ahead").and_then(|value| value.as_u64()),
                        "did_change_ahead": trace.get("did_change_ahead").and_then(|value| value.as_u64()),
                        "active_completion_count": trace.get("active_completion_count").and_then(|value| value.as_u64()),
                        "prepare_guard_outcome": completion_timeline_prepare_detail_str(trace, "guard_outcome"),
                        "prepare_outcome": completion_timeline_prepare_detail_str(trace, "outcome"),
                        "prepare_wait_elapsed_ms": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("wait_elapsed_ms"))
                            .and_then(|value| value.as_u64()),
                        "prepare_snapshot_elapsed_ms": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("snapshot_elapsed_ms"))
                            .and_then(|value| value.as_u64()),
                        "turn_wait_ms": completion_timeline_trace_stage_duration_ms(trace, "turn_wait"),
                        "prepare_stateful_ms": completion_timeline_trace_stage_duration_ms(trace, "prepare_stateful"),
                        "wait_exact_type_index_ms": completion_timeline_trace_stage_duration_ms(trace, "wait_exact_type_index"),
                        "query_bundle": completion_timeline_query_bundle_breakdown(trace),
                        "collect_ms": completion_timeline_trace_stage_duration_ms(trace, "collect"),
                        "response_build_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build"),
                    })
                });
                let mut sample_object = sample
                    .as_object()
                    .cloned()
                    .expect("sample must be json object");
                sample_object.insert(
                    "trace".to_string(),
                    trace_summary.unwrap_or(serde_json::json!(null)),
                );
                serde_json::Value::Object(sample_object)
                })
                .collect::<Vec<_>>()
    };

    let warmup_samples = enrich_samples(warmup_samples, 0);
    let measured_samples = enrich_samples(measured_samples, WARMUP_REQUESTS);

    let latest_trace_summaries = filtered_traces
        .iter()
        .rev()
        .take(16)
        .map(|trace| {
            serde_json::json!({
                "trace_id": trace.get("trace_id").and_then(|value| value.as_str()),
                "request_id": trace.get("request_id").and_then(|value| value.as_str()),
                "trigger_mode": trace.get("trigger_mode").and_then(|value| value.as_str()),
                "outcome": trace.get("outcome").and_then(|value| value.as_str()),
                "route": completion_timeline_prepare_detail_str(trace, "route"),
                "prepare_kind": completion_timeline_prepare_detail_str(trace, "kind"),
                "fail_closed_cause": completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
                "started_at_ms": trace.get("started_at_ms").and_then(|value| value.as_u64()),
                "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                "dominant_stage": trace.get("dominant_stage").and_then(|value| value.as_str()),
                "queue_outcome": trace.get("queue_outcome").and_then(|value| value.as_str()),
                "turn_wait_outcome": trace.get("turn_wait_outcome").and_then(|value| value.as_str()),
                "queued_completion_ahead": trace.get("queued_completion_ahead").and_then(|value| value.as_u64()),
                "did_change_ahead": trace.get("did_change_ahead").and_then(|value| value.as_u64()),
                "active_completion_count": trace.get("active_completion_count").and_then(|value| value.as_u64()),
                "prepare_guard_outcome": completion_timeline_prepare_detail_str(trace, "guard_outcome"),
                "prepare_outcome": completion_timeline_prepare_detail_str(trace, "outcome"),
                "prepare_wait_elapsed_ms": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("wait_elapsed_ms"))
                    .and_then(|value| value.as_u64()),
                "prepare_snapshot_elapsed_ms": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("snapshot_elapsed_ms"))
                    .and_then(|value| value.as_u64()),
                "turn_wait_ms": completion_timeline_trace_stage_duration_ms(trace, "turn_wait"),
                "prepare_stateful_ms": completion_timeline_trace_stage_duration_ms(trace, "prepare_stateful"),
                "wait_exact_type_index_ms": completion_timeline_trace_stage_duration_ms(trace, "wait_exact_type_index"),
                "query_bundle": completion_timeline_query_bundle_breakdown(trace),
                "collect_ms": completion_timeline_trace_stage_duration_ms(trace, "collect"),
                "rank_ms": completion_timeline_trace_stage_duration_ms(trace, "rank"),
                "response_build_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build"),
                "response_build_other_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build_other"),
            })
        })
        .collect::<Vec<_>>();

    let completion_total = read_u64_metric(counters.get("completion_total"));
    let fail_closed_total =
        read_u64_metric(counters.get("intellisense_v2_completion_result_total_fail_closed"));
    let cancelled_total =
        read_u64_metric(counters.get("intellisense_v2_completion_result_total_cancelled"));
    let ok_non_empty_total =
        read_u64_metric(counters.get("intellisense_v2_completion_result_total_ok_non_empty"));
    let ok_empty_total =
        read_u64_metric(counters.get("intellisense_v2_completion_result_total_ok_empty"));
    let deadline_total = read_u64_metric(
        counters
            .get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"),
    );
    let ready_total = read_u64_metric(
        counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready"),
    );
    let warmup_non_empty_samples = warmup_samples
        .iter()
        .filter(|sample| {
            sample
                .get("label_count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0
        })
        .count();
    let measured_non_empty_samples = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("label_count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0
        })
        .count();
    let measured_ok_non_empty_traces = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("outcome"))
                .and_then(|value| value.as_str())
                == Some("ok_non_empty")
        })
        .count();
    let measured_route_coverage = warm_completion_route_coverage(&measured_samples);
    let measured_head_hit_traces = measured_route_coverage.head_hit_samples;
    let measured_exact_hit_traces = measured_route_coverage.exact_hit_samples;
    let sample_elapsed_histogram = |samples: &[serde_json::Value]| {
        let values = samples
            .iter()
            .filter_map(|sample| sample.get("elapsed_ms").and_then(|value| value.as_u64()))
            .map(|value| value as f64)
            .collect::<Vec<_>>();
        sample_histogram_value(&values)
    };
    let sample_trace_histogram = |samples: &[serde_json::Value], field: &str| {
        let values = samples
            .iter()
            .filter_map(|sample| {
                let trace = sample.get("trace")?;
                if field == "query_bundle_total_ms" {
                    return trace
                        .get("query_bundle")
                        .and_then(|value| value.get("total_ms"))
                        .and_then(|value| value.as_u64());
                }
                trace.get(field).and_then(|value| value.as_u64())
            })
            .map(|value| value as f64)
            .collect::<Vec<_>>();
        sample_histogram_value(&values)
    };
    let warmup_latency_histogram = sample_elapsed_histogram(&warmup_samples);
    let measured_latency_histogram = sample_elapsed_histogram(&measured_samples);
    let measured_latency_p95_ms = read_numeric_metric(measured_latency_histogram.get("p95"));

    let report = serde_json::json!({
        "change_id": change_id,
        "profile": PROFILE_NAME,
        "schema_version": 1,
        "configuration_path": conf_big_root,
        "module_path": module_path,
        "marker": "Объект.",
        "request_plan": {
            "cache_mode": "self_warmed_same_process",
            "wait_for_current_revision": true,
            "exact_type_index_seed_mode": "background_only",
            "warmup_requests": WARMUP_REQUESTS,
            "measured_requests": MEASURE_REQUESTS,
            "completion_trigger_mode": "invoked",
        },
        "warm_cache_seed": exact_type_index_seed,
        "warmup_samples": warmup_samples,
        "measured_samples": measured_samples,
        "summary": {
            "completion_total": completion_total,
            "trace_count_for_uri": filtered_traces.len(),
            "ok_non_empty_total": ok_non_empty_total,
            "ok_empty_total": ok_empty_total,
            "fail_closed_total": fail_closed_total,
            "cancelled_total": cancelled_total,
            "deadline_total": deadline_total,
            "ready_total": ready_total,
            "head_hit_total": read_u64_metric(
                counters.get("intellisense_v2_completion_route_total_route_head_hit")
            ),
            "exact_hit_total": read_u64_metric(
                counters.get("intellisense_v2_completion_route_total_route_exact_hit")
            ),
            "head_to_exact_upgrade_total": read_u64_metric(
                counters.get("intellisense_v2_completion_head_to_exact_upgrade_total")
            ),
            "prepare_timeout_total": read_u64_metric(
                counters.get(
                    "intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout"
                )
            ),
            "exact_deadline_total": read_u64_metric(
                counters.get(
                    "intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline"
                )
            ),
            "fallback_unavailable_total": read_u64_metric(
                counters.get("intellisense_v2_completion_fallback_unavailable_total")
            ),
            "interactive_wait_budget_exhausted_total": read_u64_metric(
                counters.get("intellisense_v2_interactive_wait_budget_exhausted_total")
            ),
            "trace_matching_mode": trace_matching_mode,
            "trace_request_id_present_total": trace_request_id_present_total,
            "warmup_non_empty_samples": warmup_non_empty_samples,
            "measured_non_empty_samples": measured_non_empty_samples,
            "measured_ok_non_empty_traces": measured_ok_non_empty_traces,
            "measured_route_attributed_traces": measured_route_coverage.attributed_samples,
            "measured_head_hit_traces": measured_head_hit_traces,
            "measured_exact_hit_traces": measured_exact_hit_traces,
            "warmup_latency_ms": warmup_latency_histogram,
            "measured_latency_ms": measured_latency_histogram,
            "measured_turn_wait_ms": sample_trace_histogram(&measured_samples, "turn_wait_ms"),
            "measured_prepare_stateful_ms": sample_trace_histogram(&measured_samples, "prepare_stateful_ms"),
            "measured_wait_exact_type_index_ms": sample_trace_histogram(&measured_samples, "wait_exact_type_index_ms"),
            "measured_query_bundle_total_ms": sample_trace_histogram(
                &measured_samples,
                "query_bundle_total_ms",
            ),
            "measured_collect_ms": sample_trace_histogram(&measured_samples, "collect_ms"),
        },
        "extension_like_key_latencies": {
            "intellisense_v2_wait_for_file_version_diagnostics": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_wait_for_file_version_diagnostics_ms",
                None
            ),
            "intellisense_v2_syntax_diagnostics_query": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_syntax_diagnostics_query_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_ms",
                None
            ),
            "intellisense_v2_wait_for_file_version_completion": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_wait_for_file_version_completion_ms",
                None
            ),
            "intellisense_v2_snapshot_completion": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_snapshot_completion_ms",
                None
            ),
            "intellisense_v2_ir_query_completion": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_ir_query_completion_ms",
                None
            ),
        },
        "latest_trace_summaries": latest_trace_summaries,
        "completion_timeline": {
            "trace_count": filtered_traces.len(),
            "selected_traces": filtered_traces,
            "raw": completion_timeline,
        },
        "observability": {
            "raw": observability_metrics,
        }
    });

    let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_WARM_CACHE_COMPLETION_PERF_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!(
                    "{change_id}-real-conf-big-warm-cache-completion-perf-live.json"
                ))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for p37 real conf_big perf report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize p37 real conf_big perf report"),
    )
    .expect("write p37 real conf_big perf report");
    println!("{PROFILE_NAME}_path={}", report_path.display());

    assert!(
        trace_matching_mode == "request_id",
        "expected request-context parity to expose JSON-RPC request ids in completion timeline, trace_matching_mode={}, trace_request_id_present_total={}, filtered_traces={filtered_traces:?}",
        trace_matching_mode,
        trace_request_id_present_total
    );
    assert!(
        measured_non_empty_samples == MEASURE_REQUESTS,
        "expected all measured warm-cache samples to be non-empty, measured_non_empty_samples={}, measured_samples={measured_samples:?}",
        measured_non_empty_samples
    );
    assert!(
        measured_ok_non_empty_traces >= MEASURE_REQUESTS.saturating_sub(1),
        "expected nearly all measured warm-cache traces to be ok_non_empty, measured_ok_non_empty_traces={}, measured_samples={measured_samples:?}",
        measured_ok_non_empty_traces
    );
    assert_warm_completion_head_first_gate(&measured_samples);
    assert!(
        measured_latency_p95_ms <= WARM_HEAD_PATH_P95_BUDGET_MS,
        "warm-cache head-path p95 regression: measured_latency_p95_ms={}ms > {}ms, measured_samples={measured_samples:?}",
        measured_latency_p95_ms,
        WARM_HEAD_PATH_P95_BUDGET_MS
    );
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_head_hit"))
            + read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_exact_hit"))
            > 0,
        "expected warm-cache live report to expose at least one completion route bucket, counters={counters:?}"
    );
    assert!(
        completion_total >= (WARMUP_REQUESTS + MEASURE_REQUESTS) as u64,
        "expected completion_total >= collected request samples, completion_total={}, request_samples={}",
        completion_total,
        WARMUP_REQUESTS + MEASURE_REQUESTS
    );

    drop(server);
    drop(service);
    drain_task.abort();
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}
