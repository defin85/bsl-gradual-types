#[test]
fn p54_real_conf_big_diagnostics_shadow_state_timeout_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("debug two-save-cycle tokio runtime");
    runtime.block_on(async {
        init_test_tracing();
        struct EnvVarGuard {
            key: &'static str,
            previous: Option<String>,
            reload_runtime_config: bool,
        }

        impl EnvVarGuard {
            fn set(key: &'static str, value: &str) -> Self {
                Self::set_with_reload(key, value, false)
            }

            fn set_with_reload(
                key: &'static str,
                value: &str,
                reload_runtime_config: bool,
            ) -> Self {
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

        const PROFILE_NAME: &str =
            "p54_real_conf_big_diagnostics_shadow_state_timeout_report_live";
        const APPLY_DELAY_MS: u64 = 4_000;
        const DID_CHANGE_BLOCKING_PARSE_DELAY_MS: u64 = 1_500;
        const TIMELINE_OBSERVE_BUDGET_MS: u64 = 30_000;
        const FIRST_SAVE_VERSION: i32 = 7;
        const SECOND_SAVE_VERSION: i32 = 11;

        let _env_lock = lock_test_env().await;
        let _apply_delay_guard = EnvVarGuard::set(
            "BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS",
            &APPLY_DELAY_MS.to_string(),
        );
        let _debounce_guard =
            EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "1200", true);
        let mut blocking_delay_guard = Some(EnvVarGuard::set(
            "BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS",
            &DID_CHANGE_BLOCKING_PARSE_DELAY_MS.to_string(),
        ));

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-41-ready-snapshot-before-first-parse-exec-subphase-bounding".to_string()
        });

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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for debug report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(
            &server,
            &workspace_setup,
            "debug_real_conf_big_two_save_cycle_setup",
        )
        .await;

        let uri = Url::from_file_path(&module_path).expect("real conf_big module uri for debug");
        harness
            .send_notification(
                "textDocument/didOpen",
                DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: uri.clone(),
                        language_id: "bsl".to_string(),
                        version: 1,
                        text: module_text.clone(),
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
        .expect("didOpen must register version 1 for debug");
        tokio::time::timeout(Duration::from_secs(180), async {
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
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("didOpen must materialize same-version ready parse snapshot for debug");

        let baseline_metrics =
            live_transport_get_observability_metrics(&mut harness, 54_100_900).await;
        let baseline_counters = baseline_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("baseline metrics.counters object for p54");
        let baseline_histograms = baseline_metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("baseline metrics.histograms object for p54");

        let cycle1_semantic =
            "\nПроцедура Refactor31LiveCycle1()\n    Сообщить(НеобъявленнаяПеременная);\nКонецПроцедуры\n";
        let cycle2_semantic =
            "\nПроцедура Refactor31LiveCycle2()\n    Сообщить(СовсемНеобъявленнаяПеременная);\nКонецПроцедуры\n";

        let mut current_text = module_text.clone();
        for version in 2..FIRST_SAVE_VERSION {
            current_text.push_str(&format!("\n// debug cycle1 churn v{version}\n"));
            live_transport_ranged_did_change(
                &mut harness,
                &uri,
                version,
                vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: current_text.clone(),
                }],
            )
            .await;
        }
        current_text = format!(
            "{module_text}{cycle1_semantic}\n// debug cycle1 save v{FIRST_SAVE_VERSION}\n"
        );
        live_transport_ranged_did_change(
            &mut harness,
            &uri,
            FIRST_SAVE_VERSION,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: current_text.clone(),
            }],
        )
        .await;

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let shadow_version = server
                    .latest_document_shadow_state_v2
                    .read()
                    .await
                    .get(&file_id)
                    .map(|state| state.version);
                let ready_version = server
                    .latest_ready_parse_snapshots_v2
                    .read()
                    .await
                    .get(&file_id)
                    .map(|state| state.parse_snapshot.file_version);
                if shadow_version == Some(FIRST_SAVE_VERSION) && ready_version == Some(1) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cycle1 churn must advance shadow state to first save version while ready snapshot still lags at v1");

        live_transport_save_document(&mut harness, &uri).await;

        let cycle1_timeline_deadline =
            Instant::now() + Duration::from_millis(TIMELINE_OBSERVE_BUDGET_MS);
        let cycle1_started_trace = loop {
            let timeline =
                live_transport_get_diagnostics_save_timeline(&mut harness, 53_100_903, 16).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces for cycle1");
            let matching_trace = traces
                .iter()
                .filter(|trace| {
                    trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                        && trace
                            .get("requested_version")
                            .and_then(|value| value.as_i64())
                            == Some(FIRST_SAVE_VERSION as i64)
                })
                .max_by_key(|trace| {
                    trace
                        .get("save_cycle_sequence")
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0)
                })
                .cloned();
            let Some(trace) = matching_trace else {
                if Instant::now() >= cycle1_timeline_deadline {
                    panic!(
                        "debug harness must expose diagnostics save trace for requested_version={FIRST_SAVE_VERSION}"
                    );
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };
            let followup_publish = trace
                .get("followup_publish")
                .and_then(|value| value.as_object());
            let followup_semantic_path = trace
                .get("followup_semantic_path")
                .and_then(|value| value.as_str());
            if followup_publish.is_some() || followup_semantic_path == Some("shadow_state") {
                break trace;
            }
            if Instant::now() >= cycle1_timeline_deadline {
                panic!(
                    "debug harness must observe cycle1 follow-up publish or semantic-path decision, last_trace={trace:?}"
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };

        drop(blocking_delay_guard.take());
        tokio::time::timeout(Duration::from_secs(120), async {
            loop {
                let ready_version = server
                    .latest_ready_parse_snapshots_v2
                    .read()
                    .await
                    .get(&file_id)
                    .map(|state| state.parse_snapshot.file_version);
                if ready_version == Some(FIRST_SAVE_VERSION) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("debug harness must let ready snapshot recover to first save version before cycle2");

        let _cycle2_blocking_delay_guard = EnvVarGuard::set(
            "BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS",
            &DID_CHANGE_BLOCKING_PARSE_DELAY_MS.to_string(),
        );

        for version in (FIRST_SAVE_VERSION + 1)..SECOND_SAVE_VERSION {
            current_text.push_str(&format!("\n// debug cycle2 churn v{version}\n"));
            live_transport_ranged_did_change(
                &mut harness,
                &uri,
                version,
                vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: current_text.clone(),
                }],
            )
            .await;
        }
        current_text =
            format!(
                "{module_text}{cycle1_semantic}{cycle2_semantic}\n// debug cycle2 save v{SECOND_SAVE_VERSION}\n"
            );
        live_transport_ranged_did_change(
            &mut harness,
            &uri,
            SECOND_SAVE_VERSION,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: current_text.clone(),
            }],
        )
        .await;

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let shadow_version = server
                    .latest_document_shadow_state_v2
                    .read()
                    .await
                    .get(&file_id)
                    .map(|state| state.version);
                let ready_version = server
                    .latest_ready_parse_snapshots_v2
                    .read()
                    .await
                    .get(&file_id)
                    .map(|state| state.parse_snapshot.file_version);
                if shadow_version == Some(SECOND_SAVE_VERSION)
                    && ready_version == Some(FIRST_SAVE_VERSION)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cycle2 churn must advance shadow state to second save version while ready snapshot still lags at first save version");

        live_transport_save_document(&mut harness, &uri).await;

        let cycle2_timeline_deadline =
            Instant::now() + Duration::from_millis(TIMELINE_OBSERVE_BUDGET_MS);
        let trace_cycle_2 = loop {
            let timeline =
                live_transport_get_diagnostics_save_timeline(&mut harness, 53_100_904, 16).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces for cycle2");
            let matching_trace = traces
                .iter()
                .filter(|trace| {
                    trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                        && trace
                            .get("requested_version")
                            .and_then(|value| value.as_i64())
                            == Some(SECOND_SAVE_VERSION as i64)
                })
                .max_by_key(|trace| {
                    trace
                        .get("save_cycle_sequence")
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0)
                })
                .cloned();
            let Some(trace) = matching_trace else {
                if Instant::now() >= cycle2_timeline_deadline {
                    panic!(
                        "debug harness must expose diagnostics save trace for requested_version={SECOND_SAVE_VERSION}"
                    );
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };
            let followup_publish = trace
                .get("followup_publish")
                .and_then(|value| value.as_object());
            let followup_wait_reason = trace
                .get("followup_wait_reason")
                .and_then(|value| value.as_str());
            let followup_runtime_queue_wait_present = trace
                .get("followup_runtime_queue_wait_ms")
                .and_then(|value| value.as_u64())
                .is_some();
            let followup_apply_lag_present = trace
                .get("followup_apply_lag_ms")
                .and_then(|value| value.as_u64())
                .is_some();
            if followup_publish.is_some()
                || followup_wait_reason.is_some_and(|reason| reason != "pending_publish")
                || followup_runtime_queue_wait_present
                || followup_apply_lag_present
            {
                break trace;
            }
            if Instant::now() >= cycle2_timeline_deadline {
                panic!(
                    "p54 must observe cycle2 follow-up publish or explicit residual attribution, last_trace={trace:?}"
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };

        let cycle1_final_deadline =
            Instant::now() + Duration::from_millis(TIMELINE_OBSERVE_BUDGET_MS);
        let trace_cycle_1 = loop {
            let timeline =
                live_transport_get_diagnostics_save_timeline(&mut harness, 53_100_905, 16).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces for final cycle1");
            let matching_trace = traces
                .iter()
                .filter(|trace| {
                    trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                        && trace
                            .get("requested_version")
                            .and_then(|value| value.as_i64())
                            == Some(FIRST_SAVE_VERSION as i64)
                })
                .max_by_key(|trace| {
                    trace
                        .get("save_cycle_sequence")
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0)
                })
                .cloned();
            let Some(trace) = matching_trace else {
                if Instant::now() >= cycle1_final_deadline {
                    panic!(
                        "p54 must expose final cycle1 diagnostics save trace for requested_version={FIRST_SAVE_VERSION}"
                    );
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };
            let idle_heavy_outcome = trace
                .get("idle_heavy_outcome")
                .and_then(|value| value.as_str());
            if idle_heavy_outcome == Some("superseded_generation") {
                break trace;
            }
            if Instant::now() >= cycle1_final_deadline {
                panic!(
                    "p54 must observe final cycle1 superseded outcome, last_trace={trace:?}"
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };

        let wait_budget_ms = diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET
            .as_millis()
            .min(u64::MAX as u128) as u64;
        let relief_budget_ms =
            diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET
                .as_millis()
                .min(u64::MAX as u128) as u64;

        let assert_shadow_state_timeout_trace =
            |label: &str, trace: &serde_json::Value, expected_version: i32| {
                assert_eq!(
                    trace.get("requested_version").and_then(|value| value.as_i64()),
                    Some(expected_version as i64),
                    "{label} must stay pinned to the expected requested_version, trace={trace:?}"
                );
                assert_eq!(
                    trace.get("followup_ready_snapshot_zero_probe")
                        .and_then(|value| value.as_str()),
                    Some("not_ready"),
                    "{label} must report zero-budget miss before timeout fallback, trace={trace:?}"
                );
                assert_eq!(
                    trace.get("followup_ready_snapshot_wait_probe")
                        .and_then(|value| value.as_str()),
                    Some("timeout"),
                    "{label} must report bounded ready-snapshot timeout, trace={trace:?}"
                );
                assert_eq!(
                    trace.get("followup_ready_snapshot_task_state")
                        .and_then(|value| value.as_str()),
                    Some("in_flight_same_version"),
                    "{label} must keep the same-version exact worker in flight while timing out, trace={trace:?}"
                );
                assert_eq!(
                    trace.get("followup_ready_snapshot_timeout_phase")
                        .and_then(|value| value.as_str()),
                    Some("parse_exec"),
                    "{label} must attribute the timeout to parse_exec, trace={trace:?}"
                );
                assert_eq!(
                    trace.get("followup_ready_snapshot_timeout_leaf")
                        .and_then(|value| value.as_str()),
                    Some("parser_base_recovery"),
                    "{label} must expose that the live timeout stayed inside the bounded parser_base_recovery checkpoint on the lagging-shadow recovery branch, trace={trace:?}"
                );
                assert_eq!(
                    trace.get("followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint")
                        .and_then(|value| value.as_str()),
                    Some("parser_base_recovery"),
                    "{label} must attribute the live timeout to the bounded parser_base_recovery checkpoint, trace={trace:?}"
                );
                assert_eq!(
                    trace.get("followup_ready_snapshot_relief_valve_outcome")
                        .and_then(|value| value.as_str()),
                    Some("engaged_timed_out"),
                    "{label} must report engaged_timed_out relief attribution, trace={trace:?}"
                );
                assert_eq!(
                    trace.get("followup_ready_snapshot_continuation_reason")
                        .and_then(|value| value.as_str()),
                    Some("exhausted_continuation_proof"),
                    "{label} must distinguish exhausted continuation proof from terminal supersession or cancellation, trace={trace:?}"
                );
                assert_eq!(
                    trace.get("followup_shadow_state_available")
                        .and_then(|value| value.as_bool()),
                    Some(true),
                    "{label} must confirm shadow state availability on fallback, trace={trace:?}"
                );
                assert_eq!(
                    trace.get("followup_semantic_path")
                        .and_then(|value| value.as_str()),
                    Some("shadow_state"),
                    "{label} must fall back to shadow_state on the incident path, trace={trace:?}"
                );
                assert!(
                    trace.get("followup_ready_snapshot_timeout_phase_elapsed_ms")
                        .and_then(|value| value.as_u64())
                        .is_some_and(|value| value > 0),
                    "{label} must export non-zero timeout phase elapsed time, trace={trace:?}"
                );
                assert!(
                    trace.get("followup_ready_snapshot_timeout_leaf_elapsed_ms")
                        .and_then(|value| value.as_u64())
                        .is_some_and(|value| value > 0),
                    "{label} must export non-zero timeout leaf elapsed time, trace={trace:?}"
                );
                assert!(
                    trace.get("followup_ready_snapshot_parse_exec_ms")
                        .and_then(|value| value.as_u64())
                        .is_some_and(|value| value > 0),
                    "{label} must export non-zero parse_exec timing on timeout fallback, trace={trace:?}"
                );
            };

        let assert_shadow_state_followup_publish =
            |label: &str, trace: &serde_json::Value, expected_outcome: &str| {
                let followup_publish = trace
                    .get("followup_publish")
                    .and_then(|value| value.as_object())
                    .unwrap_or_else(|| {
                        panic!(
                            "{label} must expose final followup_publish on the incident path, trace={trace:?}"
                        )
                    });
                assert_eq!(
                    followup_publish.get("profile").and_then(|value| value.as_str()),
                    Some("idle_heavy"),
                    "{label} must expose idle_heavy followup publish details, trace={trace:?}"
                );
                assert_eq!(
                    followup_publish.get("semantic_path").and_then(|value| value.as_str()),
                    Some("shadow_state"),
                    "{label} followup publish must confirm shadow_state semantics, trace={trace:?}"
                );
                assert_eq!(
                    followup_publish
                        .get("semantic_parse_source")
                        .and_then(|value| value.as_str()),
                    Some("salsa"),
                    "{label} followup publish must expose salsa semantic parse source, trace={trace:?}"
                );
                if let Some(semantic_ir_source) = followup_publish
                    .get("semantic_ir_source")
                    .and_then(|value| value.as_str())
                {
                    assert_eq!(
                        semantic_ir_source,
                        "salsa",
                        "{label} followup publish must keep semantic IR on salsa when exported, trace={trace:?}"
                    );
                }
                assert_eq!(
                    followup_publish
                        .get("syntax_work_mode")
                        .and_then(|value| value.as_str()),
                    Some("reused"),
                    "{label} must still reuse syntax artifacts on the fallback path, trace={trace:?}"
                );
                assert!(
                    followup_publish
                        .get("semantic_diagnostics_query_ms")
                        .and_then(|value| value.as_u64())
                        .is_some_and(|value| value > 0),
                    "{label} must export semantic diagnostics query latency on the fallback path, trace={trace:?}"
                );
                assert!(
                    followup_publish
                        .get("elapsed_ms")
                        .and_then(|value| value.as_u64())
                        .is_some_and(|value| value >= wait_budget_ms.saturating_add(relief_budget_ms)),
                    "{label} followup publish must stay beyond the base wait + relief budgets on timeout fallback, trace={trace:?}"
                );
                assert_eq!(
                    trace.get("idle_heavy_outcome").and_then(|value| value.as_str()),
                    Some(expected_outcome),
                    "{label} must expose the expected idle_heavy outcome, trace={trace:?}"
                );
                assert_eq!(
                    trace.get("terminal_outcome").and_then(|value| value.as_str()),
                    Some(expected_outcome),
                    "{label} must expose the expected terminal outcome, trace={trace:?}"
                );
                assert_eq!(
                    followup_publish.get("outcome").and_then(|value| value.as_str()),
                    Some(expected_outcome),
                    "{label} followup publish must carry the expected outcome, trace={trace:?}"
                );
            };

        assert_eq!(
            cycle1_started_trace
                .get("followup_semantic_path")
                .and_then(|value| value.as_str()),
            Some("shadow_state"),
            "p54 cycle1 must first expose shadow_state fallback before cycle2 churn begins, trace={cycle1_started_trace:?}"
        );

        assert_eq!(
            trace_cycle_1
                .get("idle_heavy_outcome")
                .and_then(|value| value.as_str()),
            Some("superseded_generation"),
            "cycle1 must end in superseded_generation once cycle2 overtakes it, trace={trace_cycle_1:?}"
        );
        assert_eq!(
            trace_cycle_1
                .get("terminal_outcome")
                .and_then(|value| value.as_str()),
            Some("superseded_generation"),
            "cycle1 terminal outcome must stay superseded_generation, trace={trace_cycle_1:?}"
        );
        assert_eq!(
            cycle1_started_trace
                .get("followup_wait_reason")
                .and_then(|value| value.as_str()),
            Some("semantic_work"),
            "cycle1 must expose semantic_work when it first falls back to shadow_state, trace={cycle1_started_trace:?}"
        );
        assert_eq!(
            cycle1_started_trace
                .get("followup_shadow_state_available")
                .and_then(|value| value.as_bool()),
            Some(true),
            "cycle1 must confirm shadow-state availability on the fallback path, trace={cycle1_started_trace:?}"
        );
        assert_eq!(
            cycle1_started_trace
                .get("followup_syntax_work_mode")
                .and_then(|value| value.as_str()),
            Some("reused"),
            "cycle1 must preserve syntax reuse while falling back to shadow_state, trace={cycle1_started_trace:?}"
        );
        assert_shadow_state_timeout_trace(
            "cycle2",
            &trace_cycle_2,
            SECOND_SAVE_VERSION,
        );
        if trace_cycle_2.get("followup_publish").and_then(|value| value.as_object()).is_some() {
            assert_shadow_state_followup_publish("cycle2", &trace_cycle_2, "published");
        } else {
            assert_eq!(
                trace_cycle_2
                    .get("followup_wait_reason")
                    .and_then(|value| value.as_str()),
                Some("semantic_work"),
                "cycle2 must expose semantic_work when the timeout path has not published yet, trace={trace_cycle_2:?}"
            );
            assert!(
                trace_cycle_2
                    .get("followup_apply_lag_ms")
                    .and_then(|value| value.as_u64())
                    .is_some()
                    || trace_cycle_2
                        .get("followup_runtime_queue_wait_ms")
                        .and_then(|value| value.as_u64())
                        .is_some(),
                "cycle2 must expose explicit residual attribution when final followup publish is still pending, trace={trace_cycle_2:?}"
            );
        }

        let final_metrics = live_transport_get_observability_metrics(&mut harness, 54_100_902).await;
        let final_counters = final_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("final metrics.counters object for p54");
        let final_histograms = final_metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("final metrics.histograms object for p54");

        let bounded_wait_timeout_key =
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_timeout";
        let relief_probe_timeout_key =
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_timeout";
        let relief_timed_out_key =
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_timed_out";
        let continuation_exhausted_key =
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation_total_reason_exhausted_continuation_proof";
        let shadow_path_key =
            "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_shadow_state";
        let ready_path_key =
            "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_ready_artifacts";
        let semantic_query_total_key = "intellisense_v2_semantic_diagnostics_query_total";
        let semantic_query_histogram_key = "intellisense_v2_semantic_diagnostics_query_ms";

        let bounded_wait_timeout_delta =
            read_u64_metric(final_counters.get(bounded_wait_timeout_key)).saturating_sub(
                read_u64_metric(baseline_counters.get(bounded_wait_timeout_key)),
            );
        let relief_probe_timeout_delta =
            read_u64_metric(final_counters.get(relief_probe_timeout_key)).saturating_sub(
                read_u64_metric(baseline_counters.get(relief_probe_timeout_key)),
            );
        let relief_timed_out_delta =
            read_u64_metric(final_counters.get(relief_timed_out_key)).saturating_sub(
                read_u64_metric(baseline_counters.get(relief_timed_out_key)),
            );
        let continuation_exhausted_delta = read_u64_metric(
            final_counters.get(continuation_exhausted_key),
        )
        .saturating_sub(read_u64_metric(
            baseline_counters.get(continuation_exhausted_key),
        ));
        let shadow_path_delta = read_u64_metric(final_counters.get(shadow_path_key))
            .saturating_sub(read_u64_metric(baseline_counters.get(shadow_path_key)));
        let ready_path_delta = read_u64_metric(final_counters.get(ready_path_key))
            .saturating_sub(read_u64_metric(baseline_counters.get(ready_path_key)));
        let semantic_query_total_delta =
            read_u64_metric(final_counters.get(semantic_query_total_key)).saturating_sub(
                read_u64_metric(baseline_counters.get(semantic_query_total_key)),
            );
        let semantic_query_histogram_count_delta = read_u64_metric(
            final_histograms
                .get(semantic_query_histogram_key)
                .and_then(|value| value.as_object())
                .and_then(|histogram| histogram.get("count")),
        )
        .saturating_sub(read_u64_metric(
            baseline_histograms
                .get(semantic_query_histogram_key)
                .and_then(|value| value.as_object())
                .and_then(|histogram| histogram.get("count")),
        ));

        assert!(
            bounded_wait_timeout_delta >= 1,
            "p54 must record at least one bounded ready-snapshot timeout on the incident path, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert!(
            relief_probe_timeout_delta >= 1,
            "p54 must record at least one relief-valve timeout probe on the incident path, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert!(
            relief_timed_out_delta >= 1,
            "p54 must record at least one engaged_timed_out relief outcome on the incident path, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert!(
            continuation_exhausted_delta >= 1,
            "p54 must record exhausted continuation proof on the incident path, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert!(
            shadow_path_delta >= 2,
            "p54 must record shadow_state semantic-path attribution for both save cycles, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert_eq!(
            ready_path_delta, 0,
            "p54 incident signature must stay off ready_artifacts for both save cycles, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert!(
            semantic_query_total_delta >= 1,
            "p54 must record at least one semantic diagnostics query on the shadow_state timeout path, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert!(
            semantic_query_histogram_count_delta >= 1,
            "p54 must add at least one semantic diagnostics histogram sample on the shadow_state timeout path, final_histograms={final_histograms:?}, baseline_histograms={baseline_histograms:?}"
        );

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "ready_snapshot_wait_budget_ms": wait_budget_ms,
            "ready_snapshot_relief_budget_ms": relief_budget_ms,
            "apply_delay_ms": APPLY_DELAY_MS,
            "did_change_blocking_parse_delay_ms": DID_CHANGE_BLOCKING_PARSE_DELAY_MS,
            "cycle_1_started_trace": cycle1_started_trace,
            "cycle_1": {
                "requested_version": FIRST_SAVE_VERSION,
                "trace": trace_cycle_1,
            },
            "cycle_2": {
                "requested_version": SECOND_SAVE_VERSION,
                "trace": trace_cycle_2,
            },
            "bounded_wait_timeout_delta": bounded_wait_timeout_delta,
            "relief_probe_timeout_delta": relief_probe_timeout_delta,
            "relief_timed_out_delta": relief_timed_out_delta,
            "continuation_exhausted_delta": continuation_exhausted_delta,
            "shadow_path_delta": shadow_path_delta,
            "ready_path_delta": ready_path_delta,
            "semantic_query_total_delta": semantic_query_total_delta,
            "semantic_query_histogram_count_delta": semantic_query_histogram_count_delta,
        });
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("backend crate must live under the workspace root");
        let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_SHADOW_STATE_TIMEOUT_REPORT")
            .map(std::path::PathBuf::from)
            .map(|path| {
                if path.is_relative() {
                    workspace_root.join(path)
                } else {
                    path
                }
            })
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!(
                        "{change_id}-real-conf-big-diagnostics-shadow-state-timeout-live.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p54 shadow-state timeout report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p54 shadow-state timeout report"),
        )
        .expect("write p54 shadow-state timeout report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}
