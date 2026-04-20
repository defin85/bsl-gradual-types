#[test]
fn p56_real_conf_big_diagnostics_representative_save_followup_bundle_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p56 tokio runtime");
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

        fn utf16_range_for_substring(source: &str, needle: &str) -> Range {
            let start_byte = source
                .find(needle)
                .unwrap_or_else(|| panic!("needle not found: {needle}"));
            let end_byte = start_byte + needle.len();
            let start = &source[..start_byte];
            let end = &source[..end_byte];
            let start_line = start.lines().count().saturating_sub(1) as u32;
            let start_character = start
                .lines()
                .last()
                .unwrap_or("")
                .chars()
                .map(|ch| ch.len_utf16())
                .sum::<usize>() as u32;
            let end_line = end.lines().count().saturating_sub(1) as u32;
            let end_character = end
                .lines()
                .last()
                .unwrap_or("")
                .chars()
                .map(|ch| ch.len_utf16())
                .sum::<usize>() as u32;
            Range {
                start: Position::new(start_line, start_character),
                end: Position::new(end_line, end_character),
            }
        }

        const PROFILE_NAME: &str =
            "p56_real_conf_big_diagnostics_representative_save_followup_bundle_live";
        const TIMELINE_OBSERVE_BUDGET_MS: u64 = 30_000;
        const SAVE_CYCLE_COUNT: usize = 4;
        const READY_SNAPSHOT_MATERIALIZATION_TIMEOUT_SECS: u64 = 180;
        const BASELINE_CAPTURED_AT: &str = "2026-04-18T18:52:50Z";
        const BASELINE_DETACHED_READY_ARTIFACTS_COUNT: u64 = 4;
        const BASELINE_READY_ARTIFACTS_COUNT: u64 = 0;
        const BASELINE_SHADOW_STATE_COUNT: u64 = 0;
        const BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MIN_MS: u64 = 5_052;
        const BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MAX_MS: u64 = 5_219;
        const BASELINE_READY_SNAPSHOT_PARSE_EXEC_MIN_MS: u64 = 3_222;
        const BASELINE_READY_SNAPSHOT_PARSE_EXEC_MAX_MS: u64 = 3_327;
        const BASELINE_DID_CHANGE_MATERIALIZATION_P50_MS: f64 = 3_226.0;
        const BASELINE_DID_CHANGE_MATERIALIZATION_P95_MS: f64 = 3_329.0;
        const V1_STATEMENT: &str = "СтруктураВозврата = Новый Структура;";

        let _env_lock = lock_test_env().await;
        let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "0");
        let _did_change_blocking_parse_delay_guard =
            EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "0");
        let _debounce_guard =
            EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "1200", true);

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-44-save-followup-detached-ready-artifacts".to_string()
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

        let module_text = std::fs::read_to_string(&module_path)
            .expect("read conf_big module text for representative bundle");
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
            "p56_real_conf_big_representative_bundle_setup",
        )
        .await;

        let uri = Url::from_file_path(&module_path).expect("real conf_big module uri for p56");
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
        .expect("didOpen must register version 1 for p56");
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
        .expect("didOpen must materialize same-version ready parse snapshot for p56");

        let mut current_text = module_text.clone();
        let mut current_version = 1_i32;
        let mut current_statement = V1_STATEMENT.to_string();
        let mut cycles = Vec::with_capacity(SAVE_CYCLE_COUNT);

        for cycle_index in 0..SAVE_CYCLE_COUNT {
            let cycle_number = cycle_index + 1;
            let stage1_version = current_version
                .checked_add(1)
                .expect("p56 stage1 version overflow");
            let stage2_version = current_version
                .checked_add(2)
                .expect("p56 stage2 version overflow");
            let stage1_statement = format!(
                "СтруктураВозврата = НеобъявленнаяПеременнаяP56Cycle{cycle_number}Stage1;"
            );
            let stage2_statement = format!(
                "СтруктураВозврата = НеобъявленнаяПеременнаяP56Cycle{cycle_number}Stage2;"
            );

            let stage1_range = utf16_range_for_substring(&current_text, &current_statement);
            let stage1_text = current_text.replacen(&current_statement, &stage1_statement, 1);
            assert_ne!(
                stage1_text, current_text,
                "p56 cycle {cycle_number} must update the current statement on stage1"
            );
            live_transport_ranged_did_change(
                &mut harness,
                &uri,
                stage1_version,
                vec![TextDocumentContentChangeEvent {
                    range: Some(stage1_range),
                    range_length: None,
                    text: stage1_statement.clone(),
                }],
            )
            .await;
            current_text = stage1_text;

            tokio::time::timeout(
                Duration::from_secs(READY_SNAPSHOT_MATERIALIZATION_TIMEOUT_SECS),
                async {
                loop {
                    let ready = server
                        .latest_ready_parse_snapshots_v2
                        .read()
                        .await
                        .get(&file_id)
                        .cloned();
                    if ready
                        .as_ref()
                        .is_some_and(|state| state.parse_snapshot.file_version == stage1_version)
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            },
            )
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "p56 cycle {cycle_number} must materialize same-version ready snapshot for stage1"
                )
            });

            let stage2_range = utf16_range_for_substring(&current_text, &stage1_statement);
            let stage2_text = current_text.replacen(&stage1_statement, &stage2_statement, 1);
            assert_ne!(
                stage2_text, current_text,
                "p56 cycle {cycle_number} must update the current statement on stage2"
            );
            live_transport_ranged_did_change(
                &mut harness,
                &uri,
                stage2_version,
                vec![TextDocumentContentChangeEvent {
                    range: Some(stage2_range),
                    range_length: None,
                    text: stage2_statement.clone(),
                }],
            )
            .await;
            current_text = stage2_text;

            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let received_version = server
                        .latest_received_file_versions_v2
                        .read()
                        .await
                        .get(&file_id)
                        .copied();
                    let shadow_version = server
                        .latest_document_shadow_state_v2
                        .read()
                        .await
                        .get(&file_id)
                        .map(|state| state.version);
                    if received_version == Some(stage2_version) && shadow_version == Some(stage2_version)
                    {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "p56 cycle {cycle_number} must advance latest received version and shadow state to stage2"
                )
            });

            live_transport_save_document(&mut harness, &uri).await;

            let timeline_deadline = Instant::now() + Duration::from_millis(TIMELINE_OBSERVE_BUDGET_MS);
            let timeline = loop {
                let timeline =
                    live_transport_get_diagnostics_save_timeline(
                        &mut harness,
                        56_100_900 + cycle_index as i64,
                        16,
                    )
                    .await;
                let traces = timeline
                    .get("traces")
                    .and_then(|value| value.as_array())
                    .expect("diagnostics save timeline traces for p56");
                let matching_trace = traces
                    .iter()
                    .filter(|trace| {
                        trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && trace
                                .get("requested_version")
                                .and_then(|value| value.as_i64())
                                == Some(stage2_version as i64)
                    })
                    .max_by_key(|trace| {
                        trace
                            .get("save_cycle_sequence")
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0)
                    })
                    .cloned();
                let Some(trace) = matching_trace else {
                    if Instant::now() >= timeline_deadline {
                        panic!(
                            "p56 cycle {cycle_number} must expose a diagnostics save trace for requested_version={stage2_version}"
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
                if Instant::now() >= timeline_deadline {
                    panic!(
                        "p56 cycle {cycle_number} must observe a bounded follow-up semantic-path decision, last_trace={trace:?}"
                    );
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            };

            let followup_publish = timeline
                .get("followup_publish")
                .and_then(|value| value.as_object())
                .filter(|publish| {
                    publish.get("profile").and_then(|value| value.as_str()) == Some("idle_heavy")
                });
            let followup_semantic_path = followup_publish
                .and_then(|publish| publish.get("semantic_path").and_then(|value| value.as_str()))
                .or_else(|| {
                    timeline
                        .get("followup_semantic_path")
                        .and_then(|value| value.as_str())
                });

            assert_eq!(
                timeline
                    .get("followup_ready_snapshot_task_state")
                    .and_then(|value| value.as_str()),
                Some("in_flight_same_version"),
                "p56 representative cycle must stay on same-version in-flight producer, trace={timeline:?}"
            );
            assert_eq!(
                timeline
                    .get("followup_ready_snapshot_zero_probe")
                    .and_then(|value| value.as_str()),
                Some("not_ready"),
                "p56 representative cycle must reach save follow-up before the exact producer materializes, trace={timeline:?}"
            );
            let followup_ready_snapshot_wait_probe = timeline
                .get("followup_ready_snapshot_wait_probe")
                .and_then(|value| value.as_str());
            let followup_ready_snapshot_continuation_reason = timeline
                .get("followup_ready_snapshot_continuation_reason")
                .and_then(|value| value.as_str());
            let followup_ready_snapshot_relief_valve_outcome = timeline
                .get("followup_ready_snapshot_relief_valve_outcome")
                .and_then(|value| value.as_str());
            let followup_ready_snapshot_timeout_leaf = timeline
                .get("followup_ready_snapshot_timeout_leaf")
                .and_then(|value| value.as_str());
            let followup_wait_reason = timeline
                .get("followup_wait_reason")
                .and_then(|value| value.as_str());
            let analysis_after_timeout = server.analysis_v2.snapshot().await;
            let exact_ready_after_timeout = analysis_after_timeout
                .current_type_index_serve_only_ready(file_id)
                .expect("current_type_index_serve_only_ready after p56 timeout");
            let completion_head_ready_after_timeout = analysis_after_timeout
                .current_completion_head_ready(file_id)
                .expect("current_completion_head_ready after p56 timeout");
            let type_index_parse_snapshot_meta_after_timeout = Option::<(bool, usize, bool)>::None;
            let observed_version_after_timeout = analysis_after_timeout
                .file_version(file_id)
                .expect("file_version after p56 timeout");
            let ready_snapshot_state_after_timeout = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| {
                    (
                        state.parse_snapshot.file_version,
                        format!("{:?}", state.source),
                        state.syntax_errors_complete,
                    )
                });
            let type_index_task_state_after_timeout = {
                let tasks = server.type_index_precompute_tasks_v2.lock().await;
                tasks.get(&file_id).map(|task| {
                    (
                        task.supersession_key.requested_version,
                        task.work_class,
                        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::from_atomic(
                            task.phase.load(Ordering::Relaxed),
                        ),
                        task.active_requested_version.load(Ordering::Relaxed),
                        task.handle.is_finished(),
                    )
                })
            };
            let current_revision_head_precompute_task_state_after_timeout = {
                let tasks = server.current_revision_head_precompute_tasks_v2.lock().await;
                tasks.get(&file_id).map(|task| {
                    (
                        task.requested_version.load(Ordering::Relaxed),
                        task.handle.is_finished(),
                    )
                })
            };
            let background_parse_task_state_after_timeout = server
                .matching_background_parse_snapshot_task_control_v2(
                    file_id,
                    stage2_version,
                    None,
                )
                .await
                .map(|task| {
                    (
                        crate::server::BackgroundParseSnapshotApplyTaskPhaseV2::from_raw(
                            task.phase.load(Ordering::SeqCst),
                        ),
                        task.promotion_requested.load(Ordering::SeqCst),
                        task.materialized.load(Ordering::SeqCst),
                    )
                });
            let observability_metrics_after_timeout =
                live_transport_get_observability_metrics(&mut harness, 56_100_949 + cycle_index as i64)
                    .await;
            let observability_histograms_after_timeout = observability_metrics_after_timeout
                .get("histograms")
                .and_then(|value| value.as_object())
                .expect("observability metrics.histograms after p56 timeout");
            let observability_counters_after_timeout = observability_metrics_after_timeout
                .get("counters")
                .and_then(|value| value.as_object())
                .expect("observability metrics.counters after p56 timeout");
            let type_index_precompute_exec_histogram_after_timeout = histogram_metric_value_or_zero(
                observability_histograms_after_timeout,
                "intellisense_v2_runtime_type_index_precompute_exec_ms",
                None,
            );
            let type_index_precompute_ir_exec_histogram_after_timeout =
                histogram_metric_value_or_zero(
                    observability_histograms_after_timeout,
                    "intellisense_v2_runtime_type_index_precompute_ir_exec_ms",
                    None,
                );
            let type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout =
                histogram_metric_value_or_zero(
                    observability_histograms_after_timeout,
                    "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms",
                    None,
                );
            let ir_singleflight_wait_histogram_after_timeout = histogram_metric_value_or_zero(
                observability_histograms_after_timeout,
                "intellisense_v2_singleflight_wait_ms",
                None,
            );
            let ir_singleflight_counters_after_timeout = serde_json::json!({
                "leader_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_lsp_outcome_leader_query_kind_ir"
                )),
                "shared_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_lsp_outcome_shared_query_kind_ir"
                )),
                "key_unavailable_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_lsp_outcome_key_unavailable_query_kind_ir"
                )),
            });
            let type_index_counters_after_timeout = serde_json::json!({
                "exact_stored_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_type_index_reason_total_reason_type_index_precompute_exact_stored"
                )),
                "superseded_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_type_index_reason_total_reason_type_index_precompute_superseded"
                )),
                "cancelled_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_type_index_reason_total_reason_type_index_precompute_cancelled"
                )),
                "exact_wait_ready_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready"
                )),
                "exact_wait_deadline_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"
                )),
            });
            let ready_artifacts_publish = match followup_semantic_path {
                Some("detached_ready_artifacts") => {
                    assert_ne!(
                        followup_ready_snapshot_wait_probe,
                        Some("ready"),
                        "p56 detached cycle must not claim a canonical bounded-wait ready hit, trace={timeline:?}"
                    );
                    if followup_ready_snapshot_wait_probe == Some("timeout") {
                        assert_eq!(
                            followup_ready_snapshot_timeout_leaf,
                            Some("ready_install"),
                            "p56 detached timeout cycle must expose ready_install as the timeout leaf, trace={timeline:?}"
                        );
                    } else {
                        assert!(
                            followup_ready_snapshot_timeout_leaf.is_none(),
                            "p56 detached non-timeout cycle must not fabricate a timeout leaf, trace={timeline:?}"
                        );
                    }
                    let publish = followup_publish.expect(
                        "p56 detached path must expose an idle_heavy follow-up publish object",
                    );
                    assert_eq!(
                        publish
                            .get("semantic_parse_source")
                            .and_then(|value| value.as_str()),
                        Some("snapshot")
                    );
                    assert_eq!(
                        publish
                            .get("semantic_ir_source")
                            .and_then(|value| value.as_str()),
                        Some("snapshot_build")
                    );
                    assert_eq!(
                        followup_ready_snapshot_continuation_reason,
                        None,
                        "p56 detached cycle must stay on the primary canonical wait path without synthetic continuation, trace={timeline:?}"
                    );
                    Some(publish)
                }
                Some("ready_artifacts") => panic!(
                    "p56 representative bundle must now surface the still-current late path through detached_ready_artifacts, trace={timeline:?}, observed_version_after_timeout={observed_version_after_timeout:?}, exact_ready_after_timeout={exact_ready_after_timeout}, completion_head_ready_after_timeout={completion_head_ready_after_timeout}, type_index_parse_snapshot_meta_after_timeout={type_index_parse_snapshot_meta_after_timeout:?}, ready_snapshot_state_after_timeout={ready_snapshot_state_after_timeout:?}, type_index_task_state_after_timeout={type_index_task_state_after_timeout:?}, current_revision_head_precompute_task_state_after_timeout={current_revision_head_precompute_task_state_after_timeout:?}, background_parse_task_state_after_timeout={background_parse_task_state_after_timeout:?}, type_index_precompute_exec_histogram_after_timeout={type_index_precompute_exec_histogram_after_timeout:?}, type_index_precompute_ir_exec_histogram_after_timeout={type_index_precompute_ir_exec_histogram_after_timeout:?}, type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout={type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout:?}, ir_singleflight_wait_histogram_after_timeout={ir_singleflight_wait_histogram_after_timeout:?}, ir_singleflight_counters_after_timeout={ir_singleflight_counters_after_timeout:?}, type_index_counters_after_timeout={type_index_counters_after_timeout:?}"
                ),
                Some("shadow_state") => panic!(
                    "p56 representative bundle must not fall back to shadow_state on the still-current path, trace={timeline:?}, observed_version_after_timeout={observed_version_after_timeout:?}, exact_ready_after_timeout={exact_ready_after_timeout}, completion_head_ready_after_timeout={completion_head_ready_after_timeout}, type_index_parse_snapshot_meta_after_timeout={type_index_parse_snapshot_meta_after_timeout:?}, ready_snapshot_state_after_timeout={ready_snapshot_state_after_timeout:?}, type_index_task_state_after_timeout={type_index_task_state_after_timeout:?}, current_revision_head_precompute_task_state_after_timeout={current_revision_head_precompute_task_state_after_timeout:?}, background_parse_task_state_after_timeout={background_parse_task_state_after_timeout:?}, type_index_precompute_exec_histogram_after_timeout={type_index_precompute_exec_histogram_after_timeout:?}, type_index_precompute_ir_exec_histogram_after_timeout={type_index_precompute_ir_exec_histogram_after_timeout:?}, type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout={type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout:?}, ir_singleflight_wait_histogram_after_timeout={ir_singleflight_wait_histogram_after_timeout:?}, ir_singleflight_counters_after_timeout={ir_singleflight_counters_after_timeout:?}, type_index_counters_after_timeout={type_index_counters_after_timeout:?}"
                ),
                _ => panic!(
                    "p56 representative bundle must resolve each cycle to detached_ready_artifacts, trace={timeline:?}, observed_version_after_timeout={observed_version_after_timeout:?}, exact_ready_after_timeout={exact_ready_after_timeout}, completion_head_ready_after_timeout={completion_head_ready_after_timeout:?}, type_index_parse_snapshot_meta_after_timeout={type_index_parse_snapshot_meta_after_timeout:?}, ready_snapshot_state_after_timeout={ready_snapshot_state_after_timeout:?}, type_index_task_state_after_timeout={type_index_task_state_after_timeout:?}, current_revision_head_precompute_task_state_after_timeout={current_revision_head_precompute_task_state_after_timeout:?}, background_parse_task_state_after_timeout={background_parse_task_state_after_timeout:?}, type_index_precompute_exec_histogram_after_timeout={type_index_precompute_exec_histogram_after_timeout:?}, type_index_precompute_ir_exec_histogram_after_timeout={type_index_precompute_ir_exec_histogram_after_timeout:?}, type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout={type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout:?}, ir_singleflight_wait_histogram_after_timeout={ir_singleflight_wait_histogram_after_timeout:?}, ir_singleflight_counters_after_timeout={ir_singleflight_counters_after_timeout:?}, type_index_counters_after_timeout={type_index_counters_after_timeout:?}"
                ),
            };
            let followup_ready_snapshot_parse_exec_ms = timeline
                .get("followup_ready_snapshot_parse_exec_ms")
                .and_then(|value| value.as_u64());
            let followup_publish_elapsed_ms = ready_artifacts_publish.and_then(|publish| {
                publish.get("elapsed_ms").and_then(|value| value.as_u64())
            });
            let followup_publish_semantic_diagnostics_query_ms =
                ready_artifacts_publish.and_then(|publish| {
                    publish
                        .get("semantic_diagnostics_query_ms")
                        .and_then(|value| value.as_u64())
                });
            let semantic_query_dominates_parse_exec = followup_publish_semantic_diagnostics_query_ms
                .zip(followup_ready_snapshot_parse_exec_ms)
                .map(|(query_ms, parse_exec_ms)| query_ms > parse_exec_ms);
            let followup_publish_semantic_diagnostics_ir_ms =
                ready_artifacts_publish.and_then(|publish| {
                    publish
                        .get("semantic_diagnostics_ir_ms")
                        .and_then(|value| value.as_u64())
                });
            let followup_publish_semantic_diagnostics_collect_ms =
                ready_artifacts_publish.and_then(|publish| {
                    publish
                        .get("semantic_diagnostics_collect_ms")
                        .and_then(|value| value.as_u64())
                });
            let followup_publish_non_query_residual_ms = followup_publish_elapsed_ms
                .zip(followup_publish_semantic_diagnostics_query_ms)
                .map(|(publish_ms, query_ms)| publish_ms.saturating_sub(query_ms));
            if ready_artifacts_publish.is_some() {
                assert!(
                    followup_publish_elapsed_ms.is_some_and(|value| value > 0),
                    "p56 cycle {cycle_number} must expose non-zero followup publish latency on the detached path, trace={timeline:?}"
                );
                assert!(
                    followup_publish_semantic_diagnostics_query_ms
                        .is_some_and(|value| value > 0),
                    "p56 cycle {cycle_number} must expose non-zero semantic_diagnostics_query_ms on the detached path, trace={timeline:?}"
                );
                assert_eq!(
                    semantic_query_dominates_parse_exec,
                    Some(true),
                    "p56 cycle {cycle_number} must prove that semantic_diagnostics_query now dominates ready-snapshot parse_exec, trace={timeline:?}"
                );
            }

            cycles.push(serde_json::json!({
                "cycle": cycle_number,
                "stage1_version": stage1_version,
                "requested_version": stage2_version,
                "save_cycle_sequence": timeline
                    .get("save_cycle_sequence")
                    .and_then(|value| value.as_u64()),
                "followup_semantic_path": followup_semantic_path,
                "followup_publish_semantic_path": ready_artifacts_publish
                    .and_then(|publish| publish.get("semantic_path").and_then(|value| value.as_str())),
                "followup_ready_snapshot_task_state": timeline
                    .get("followup_ready_snapshot_task_state")
                    .and_then(|value| value.as_str()),
                "followup_ready_snapshot_zero_probe": timeline
                    .get("followup_ready_snapshot_zero_probe")
                    .and_then(|value| value.as_str()),
                "followup_ready_snapshot_wait_probe": followup_ready_snapshot_wait_probe,
                "followup_ready_snapshot_parse_exec_ms": followup_ready_snapshot_parse_exec_ms,
                "followup_publish_elapsed_ms": followup_publish_elapsed_ms,
                "followup_publish_semantic_diagnostics_query_ms": followup_publish_semantic_diagnostics_query_ms,
                "followup_publish_semantic_diagnostics_ir_ms": followup_publish_semantic_diagnostics_ir_ms,
                "followup_publish_semantic_diagnostics_collect_ms": followup_publish_semantic_diagnostics_collect_ms,
                "semantic_query_dominates_parse_exec": semantic_query_dominates_parse_exec,
                "followup_publish_non_query_residual_ms": followup_publish_non_query_residual_ms,
                "followup_ready_snapshot_continuation_reason": followup_ready_snapshot_continuation_reason,
                "followup_ready_snapshot_relief_valve_outcome": followup_ready_snapshot_relief_valve_outcome,
                "followup_ready_snapshot_timeout_leaf": followup_ready_snapshot_timeout_leaf,
                "followup_wait_reason": followup_wait_reason,
                "observed_version_after_timeout": observed_version_after_timeout,
                "exact_ready_after_timeout": exact_ready_after_timeout,
                "completion_head_ready_after_timeout": completion_head_ready_after_timeout,
                "type_index_parse_snapshot_meta_after_timeout": type_index_parse_snapshot_meta_after_timeout.as_ref().map(
                    |(incremental, changed_ranges_count, serve_only_blocked)| serde_json::json!({
                        "incremental": incremental,
                        "changed_ranges_count": changed_ranges_count,
                        "serve_only_blocked": serve_only_blocked,
                    })
                ),
                "ready_snapshot_state_after_timeout": ready_snapshot_state_after_timeout.as_ref().map(
                    |(file_version, source, syntax_errors_complete)| serde_json::json!({
                        "file_version": file_version,
                        "source": source,
                        "syntax_errors_complete": syntax_errors_complete,
                    })
                ),
                "type_index_task_state_after_timeout": type_index_task_state_after_timeout.as_ref().map(
                    |(requested_version, work_class, phase, active_requested_version, finished)| serde_json::json!({
                        "requested_version": requested_version,
                        "work_class": format!("{work_class:?}"),
                        "phase": phase.as_str(),
                        "active_requested_version": active_requested_version,
                        "finished": finished,
                    })
                ),
                "current_revision_head_precompute_task_state_after_timeout": current_revision_head_precompute_task_state_after_timeout.as_ref().map(
                    |(requested_version, finished)| serde_json::json!({
                        "requested_version": requested_version,
                        "finished": finished,
                    })
                ),
                "background_parse_task_state_after_timeout": background_parse_task_state_after_timeout.as_ref().map(
                    |(phase, promotion_requested, materialized)| serde_json::json!({
                        "phase": format!("{phase:?}"),
                        "promotion_requested": promotion_requested,
                        "materialized": materialized,
                    })
                ),
                "type_index_precompute_exec_histogram_after_timeout": type_index_precompute_exec_histogram_after_timeout,
                "type_index_precompute_ir_exec_histogram_after_timeout": type_index_precompute_ir_exec_histogram_after_timeout,
                "type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout": type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout,
                "ir_singleflight_wait_histogram_after_timeout": ir_singleflight_wait_histogram_after_timeout,
                "ir_singleflight_counters_after_timeout": ir_singleflight_counters_after_timeout,
                "type_index_counters_after_timeout": type_index_counters_after_timeout,
                "final_statement": stage2_statement,
            }));

            tokio::time::timeout(
                Duration::from_secs(READY_SNAPSHOT_MATERIALIZATION_TIMEOUT_SECS),
                async {
                loop {
                    let ready = server
                        .latest_ready_parse_snapshots_v2
                        .read()
                        .await
                        .get(&file_id)
                        .cloned();
                    if ready
                        .as_ref()
                        .is_some_and(|state| state.parse_snapshot.file_version == stage2_version)
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            },
            )
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "p56 cycle {cycle_number} must eventually materialize the saved exact ready snapshot"
                )
            });

            current_version = stage2_version;
            current_statement = stage2_statement;
            tokio::time::sleep(Duration::from_millis(250)).await;
        }

        let detached_ready_artifacts_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
                    == Some("detached_ready_artifacts")
            })
            .count() as u64;
        let ready_artifacts_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
                    == Some("ready_artifacts")
            })
            .count() as u64;
        let shadow_state_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
                    == Some("shadow_state")
            })
            .count() as u64;
        let wait_probe_timeout_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_ready_snapshot_wait_probe")
                    .and_then(|value| value.as_str())
                    == Some("timeout")
            })
            .count() as u64;
        let zero_probe_not_ready_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_ready_snapshot_zero_probe")
                    .and_then(|value| value.as_str())
                    == Some("not_ready")
            })
            .count() as u64;
        let semantic_query_dominates_parse_exec_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("semantic_query_dominates_parse_exec")
                    .and_then(|value| value.as_bool())
                    == Some(true)
            })
            .count() as u64;
        let continuation_reason_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_ready_snapshot_continuation_reason")
                    .and_then(|value| value.as_str())
                    .is_some()
            })
            .count() as u64;
        let timeout_leaf_ready_install_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_ready_snapshot_timeout_leaf")
                    .and_then(|value| value.as_str())
                    == Some("ready_install")
            })
            .count() as u64;
        let max_followup_publish_elapsed_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_publish_elapsed_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_publish_semantic_diagnostics_query_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_publish_semantic_diagnostics_query_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_ready_snapshot_parse_exec_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_ready_snapshot_parse_exec_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();

        let final_metrics = live_transport_get_observability_metrics(&mut harness, 56_100_950).await;
        let final_histograms = final_metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("final metrics.histograms object");
        let did_change_materialization_histogram = final_histograms
            .get("intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_did_change")
            .and_then(|value| value.as_object())
            .expect("p56 did_change materialization histogram");
        let did_change_materialization_histogram_count = read_u64_metric(
            did_change_materialization_histogram.get("count"),
        );
        let did_change_materialization_p50_ms =
            read_numeric_metric(did_change_materialization_histogram.get("p50"));
        let did_change_materialization_p95_ms =
            read_numeric_metric(did_change_materialization_histogram.get("p95"));

        assert_eq!(
            cycles.len(),
            SAVE_CYCLE_COUNT,
            "p56 must record every representative save cycle"
        );
        assert_eq!(
            detached_ready_artifacts_count,
            SAVE_CYCLE_COUNT as u64,
            "p56 representative bundle must keep every still-current cycle on detached_ready_artifacts, cycles={cycles:?}"
        );
        assert_eq!(
            ready_artifacts_count,
            0,
            "p56 representative bundle must not regress back to canonical ready_artifacts on the representative late path, cycles={cycles:?}"
        );
        assert_eq!(
            shadow_state_count,
            0,
            "p56 representative bundle must not report shadow_state on the still-current path, cycles={cycles:?}"
        );
        assert_eq!(
            wait_probe_timeout_count,
            timeout_leaf_ready_install_count,
            "p56 representative bundle must keep timeout-leaf fidelity aligned with timeouted detached cycles, cycles={cycles:?}"
        );
        assert!(
            wait_probe_timeout_count > 0,
            "p56 representative bundle must retain at least one canonical bounded-wait timeout cycle in the representative late family, cycles={cycles:?}"
        );
        assert_eq!(
            zero_probe_not_ready_count,
            SAVE_CYCLE_COUNT as u64,
            "p56 representative bundle must exercise the same-family in-flight producer before bounded wait succeeds, cycles={cycles:?}"
        );
        assert_eq!(
            semantic_query_dominates_parse_exec_count,
            detached_ready_artifacts_count,
            "p56 representative bundle must prove that semantic_diagnostics_query dominates ready-snapshot parse_exec on every detached cycle, cycles={cycles:?}"
        );
        assert_eq!(
            continuation_reason_count,
            0,
            "p56 representative bundle must not need a follow-up continuation reason after refactor-44, cycles={cycles:?}"
        );
        assert_eq!(
            timeout_leaf_ready_install_count,
            wait_probe_timeout_count,
            "p56 representative bundle must expose ready_install exactly on detached timeout cycles, cycles={cycles:?}"
        );
        assert!(
            max_followup_publish_elapsed_ms
                .is_some_and(|value| value <= BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MAX_MS),
            "p56 representative bundle must stay at or below the {BASELINE_CAPTURED_AT} publish baseline ceiling of {}ms, observed_max={max_followup_publish_elapsed_ms:?}, cycles={cycles:?}",
            BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MAX_MS
        );
        assert!(
            max_followup_ready_snapshot_parse_exec_ms
                .is_some_and(|value| value <= BASELINE_READY_SNAPSHOT_PARSE_EXEC_MAX_MS),
            "p56 representative bundle must stay at or below the {BASELINE_CAPTURED_AT} parse_exec baseline ceiling of {}ms, observed_max={max_followup_ready_snapshot_parse_exec_ms:?}, cycles={cycles:?}",
            BASELINE_READY_SNAPSHOT_PARSE_EXEC_MAX_MS
        );
        assert!(
            did_change_materialization_histogram_count > 0,
            "p56 representative bundle must export did_change ready-snapshot materialization latency, final_histograms={final_histograms:?}"
        );
        let representative_cycle = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
                    == Some("detached_ready_artifacts")
            })
            .max_by_key(|cycle| {
                cycle
                    .get("followup_publish_elapsed_ms")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
            })
            .cloned()
            .expect("p56 must keep a representative detached_ready_artifacts cycle summary");

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "cycle_count": SAVE_CYCLE_COUNT,
            "baseline": {
                "captured_at": BASELINE_CAPTURED_AT,
                "followup_publish_elapsed_ms": [BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MIN_MS, BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MAX_MS],
                "followup_ready_snapshot_parse_exec_ms": [BASELINE_READY_SNAPSHOT_PARSE_EXEC_MIN_MS, BASELINE_READY_SNAPSHOT_PARSE_EXEC_MAX_MS],
                "did_change_ready_snapshot_materialization_ms": {
                    "p50": BASELINE_DID_CHANGE_MATERIALIZATION_P50_MS,
                    "p95": BASELINE_DID_CHANGE_MATERIALIZATION_P95_MS,
                },
                "followup_semantic_path_detached_ready_artifacts": BASELINE_DETACHED_READY_ARTIFACTS_COUNT,
                "followup_semantic_path_ready_artifacts": BASELINE_READY_ARTIFACTS_COUNT,
                "followup_semantic_path_shadow_state": BASELINE_SHADOW_STATE_COUNT,
            },
            "summary": {
                "followup_semantic_path_detached_ready_artifacts": detached_ready_artifacts_count,
                "followup_semantic_path_ready_artifacts": ready_artifacts_count,
                "followup_semantic_path_shadow_state": shadow_state_count,
                "followup_ready_snapshot_wait_probe_timeout": wait_probe_timeout_count,
                "followup_ready_snapshot_zero_probe_not_ready": zero_probe_not_ready_count,
                "followup_ready_snapshot_continuation_reason_count": continuation_reason_count,
                "followup_ready_snapshot_timeout_leaf_ready_install_count": timeout_leaf_ready_install_count,
                "semantic_query_dominates_parse_exec_count": semantic_query_dominates_parse_exec_count,
                "representative_canonical_residual_mix": "parse_exec_or_ready_install_before_detached_publish",
                "post_detached_publish_shape": "semantic_query_dominates_parse_exec_with_additional_publish_tail",
            },
            "aggregate": {
                "max_followup_publish_elapsed_ms": max_followup_publish_elapsed_ms,
                "max_followup_publish_semantic_diagnostics_query_ms": max_followup_publish_semantic_diagnostics_query_ms,
                "max_followup_ready_snapshot_parse_exec_ms": max_followup_ready_snapshot_parse_exec_ms,
                "did_change_ready_snapshot_materialization_histogram_count": did_change_materialization_histogram_count,
                "did_change_ready_snapshot_materialization_p50_ms": did_change_materialization_p50_ms,
                "did_change_ready_snapshot_materialization_p95_ms": did_change_materialization_p95_ms,
            },
            "comparison": {
                "max_followup_publish_elapsed_vs_baseline_ceiling_delta_ms": max_followup_publish_elapsed_ms
                    .map(|value| value as i64 - BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MAX_MS as i64),
                "max_followup_ready_snapshot_parse_exec_vs_baseline_ceiling_delta_ms": max_followup_ready_snapshot_parse_exec_ms
                    .map(|value| value as i64 - BASELINE_READY_SNAPSHOT_PARSE_EXEC_MAX_MS as i64),
                "did_change_ready_snapshot_materialization_p50_vs_baseline_delta_ms": did_change_materialization_p50_ms - BASELINE_DID_CHANGE_MATERIALIZATION_P50_MS,
                "did_change_ready_snapshot_materialization_p95_vs_baseline_delta_ms": did_change_materialization_p95_ms - BASELINE_DID_CHANGE_MATERIALIZATION_P95_MS,
            },
            "representative_cycle": representative_cycle,
            "cycles": cycles,
        });
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("backend crate must live under the workspace root");
        let report_path = std::env::var(
            "BSL_V2_REAL_CONF_BIG_REPRESENTATIVE_SAVE_FOLLOWUP_BUNDLE_REPORT",
        )
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
                    "{change_id}-real-conf-big-diagnostics-representative-save-followup-bundle-live.json"
                ))
        });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p56 representative save-followup bundle report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p56 representative save-followup bundle report"),
        )
        .expect("write p56 representative save-followup bundle report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}
