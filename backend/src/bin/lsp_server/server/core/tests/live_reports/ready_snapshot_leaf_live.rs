#[test]
fn p55_real_conf_big_diagnostics_ready_snapshot_leaf_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p55 tokio runtime");
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
            "p55_real_conf_big_diagnostics_ready_snapshot_leaf_report_live";
        const APPLY_DELAY_MS: u64 = 0;
        const DID_CHANGE_BLOCKING_PARSE_DELAY_MS: u64 = 0;
        const TIMELINE_OBSERVE_BUDGET_MS: u64 = 30_000;
        const V1_STATEMENT: &str = "СтруктураВозврата = Новый Структура;";
        const V2_STATEMENT: &str = "СтруктураВозврата = НеобъявленнаяПеременная;";
        const V3_STATEMENT: &str = "СтруктураВозврата = ЕщеНеобъявленнаяПеременная;";
        // These budgets stay above current representative p55 measurements with
        // headroom, but below the old slowdown profile that forced save follow-up
        // through live ready_install instead of the detached diagnostics path.
        const FOLLOWUP_PUBLISH_ELAPSED_BUDGET_MS: u64 = 2_800;
        const READY_SNAPSHOT_PARSE_EXEC_BUDGET_MS: u64 = 1_200;
        const READY_SNAPSHOT_CORE_PARSE_BUILD_BUDGET_MS: u64 = 1_200;
        const READY_SNAPSHOT_EXACT_ASSEMBLY_BUDGET_MS: u64 = 1_100;
        const SEMANTIC_DIAGNOSTICS_QUERY_BUDGET_MS: u64 = 1_600;
        const SEMANTIC_DIAGNOSTICS_IR_BUDGET_MS: u64 = 1_100;
        const SEMANTIC_DIAGNOSTICS_COLLECT_BUDGET_MS: u64 = 600;
        const LOCAL_FUNCTION_SUMMARIES_BUDGET_MS: u64 = 450;
        const BASELINE_CAPTURED_AT: &str = "2026-04-18T18:52:50Z";
        const BASELINE_DID_CHANGE_MATERIALIZATION_P50_MS: f64 = 3_226.0;
        const BASELINE_DID_CHANGE_MATERIALIZATION_P95_MS: f64 = 3_329.0;

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

        let _env_lock = lock_test_env().await;
        let _apply_delay_guard = EnvVarGuard::set(
            "BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS",
            &APPLY_DELAY_MS.to_string(),
        );
        let _did_change_blocking_parse_delay_guard = EnvVarGuard::set(
            "BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS",
            &DID_CHANGE_BLOCKING_PARSE_DELAY_MS.to_string(),
        );
        let _debounce_guard =
            EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "1200", true);

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-46-save-followup-dual-artifact-wait".to_string()
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
            .expect("read conf_big module text for p55 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p55_real_conf_big_live_setup")
            .await;

        let uri = Url::from_file_path(&module_path).expect("real conf_big module uri for p55");
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
        .expect("didOpen must register version 1 for p55");
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
        .expect("didOpen must materialize same-version ready parse snapshot for p55");

        let v2_text = module_text.replacen(V1_STATEMENT, V2_STATEMENT, 1);
        assert_ne!(
            v2_text, module_text,
            "p55 fixture must edit an existing statement inside the representative module"
        );
        let v2_range = utf16_range_for_substring(&module_text, V1_STATEMENT);
        live_transport_ranged_did_change(
            &mut harness,
            &uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: Some(v2_range),
                range_length: None,
                text: V2_STATEMENT.to_string(),
            }],
        )
        .await;

        tokio::time::timeout(Duration::from_secs(60), async {
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
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("local ranged didChange must materialize same-version ready snapshot v2 for p55");

        let v3_text = v2_text.replacen(V2_STATEMENT, V3_STATEMENT, 1);
        assert_ne!(
            v3_text, v2_text,
            "p55 fixture must keep churn inside the same live statement before save"
        );
        let v3_range = utf16_range_for_substring(&v2_text, V2_STATEMENT);
        live_transport_ranged_did_change(
            &mut harness,
            &uri,
            3,
            vec![TextDocumentContentChangeEvent {
                range: Some(v3_range),
                range_length: None,
                text: V3_STATEMENT.to_string(),
            }],
        )
        .await;

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
                if received_version == Some(3) && shadow_version == Some(3) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("didChange must advance latest received version and shadow state to v3 for p55");

        live_transport_save_document(&mut harness, &uri).await;

        let timeline_deadline = Instant::now() + Duration::from_millis(TIMELINE_OBSERVE_BUDGET_MS);
        let timeline = loop {
            let timeline =
                live_transport_get_diagnostics_save_timeline(&mut harness, 55_100_902, 12).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces for p55");
            let matching_trace = traces
                .iter()
                .filter(|trace| {
                    trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                        && trace
                            .get("requested_version")
                            .and_then(|value| value.as_i64())
                            == Some(3)
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
                    panic!("p55 must expose a diagnostics save trace for requested_version=3");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };
            let followup_publish = trace
                .get("followup_publish")
                .and_then(|value| value.as_object());
            if followup_publish.is_some() {
                break trace;
            }
            if Instant::now() >= timeline_deadline {
                panic!(
                    "p55 must observe the final idle_heavy follow-up publish on the production-like path, last_trace={trace:?}"
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };

        let full_publish = timeline
            .get("followup_publish")
            .and_then(|value| value.as_object())
            .filter(|publish| {
                publish.get("profile").and_then(|value| value.as_str()) == Some("idle_heavy")
            });
        let followup_semantic_path = full_publish
            .and_then(|publish| publish.get("semantic_path").and_then(|value| value.as_str()))
            .or_else(|| {
                timeline
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
            });
        assert_eq!(
            followup_semantic_path,
            Some("detached_ready_artifacts"),
            "p55 must publish through detached_ready_artifacts on the production-like live path once ready_install remains pending, trace={timeline:?}"
        );
        let ready_artifacts_publish =
            full_publish.expect("p55 detached ready path must expose an idle_heavy follow-up publish object");
        assert_eq!(
            ready_artifacts_publish
                .get("semantic_parse_source")
                .and_then(|value| value.as_str()),
            Some("snapshot")
        );
        let followup_semantic_materialization_path = ready_artifacts_publish
            .get("semantic_materialization_path")
            .and_then(|value| value.as_str())
            .or_else(|| {
                timeline
                    .get("followup_semantic_materialization_path")
                    .and_then(|value| value.as_str())
            });
        assert_eq!(
            followup_semantic_materialization_path,
            Some("diagnostics_only"),
            "p55 must export the traced diagnostics semantic materialization path directly, trace={timeline:?}"
        );
        let followup_publish_elapsed_ms = ready_artifacts_publish
            .get("elapsed_ms")
            .and_then(|value| value.as_u64());
        let semantic_diagnostics_query_ms = ready_artifacts_publish
            .get("semantic_diagnostics_query_ms")
            .and_then(|value| value.as_u64());
        let semantic_diagnostics_ir_ms = ready_artifacts_publish
            .get("semantic_diagnostics_ir_ms")
            .and_then(|value| value.as_u64());
        let semantic_diagnostics_collect_ms = ready_artifacts_publish
            .get("semantic_diagnostics_collect_ms")
            .and_then(|value| value.as_u64());
        let diagnostics_only_semantic_facts_ms = ready_artifacts_publish
            .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_ms")
            .and_then(|value| value.as_u64());
        let diagnostics_only_local_function_summaries_ms = ready_artifacts_publish
            .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_ms")
            .and_then(|value| value.as_u64());
        let ready_snapshot_parse_exec_ms = timeline
            .get("followup_ready_snapshot_parse_exec_ms")
            .and_then(|value| value.as_u64());
        let ready_snapshot_core_parse_build_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_parse_build_ms")
            .and_then(|value| value.as_u64());
        let ready_snapshot_exact_assembly_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms")
            .and_then(|value| value.as_u64());
        let semantic_query_dominates_ready_snapshot_parse_exec = semantic_diagnostics_query_ms
            .zip(ready_snapshot_parse_exec_ms)
            .map(|(query_ms, parse_exec_ms)| query_ms > parse_exec_ms);
        assert!(
            followup_publish_elapsed_ms.is_some_and(|value| value > 0),
            "p55 must expose non-zero followup publish latency on the production-like path, trace={timeline:?}"
        );
        assert!(
            semantic_diagnostics_query_ms.is_some_and(|value| value > 0),
            "p55 must expose non-zero semantic_diagnostics_query_ms on the production-like path, trace={timeline:?}"
        );
        assert_eq!(
            semantic_query_dominates_ready_snapshot_parse_exec,
            Some(true),
            "p55 must prove that post-parse semantic diagnostics now dominate ready-snapshot parse_exec on the production-like path, trace={timeline:?}"
        );
        assert_optional_u64_budget(
            &timeline,
            "p55",
            "followup_publish_elapsed_ms",
            followup_publish_elapsed_ms,
            FOLLOWUP_PUBLISH_ELAPSED_BUDGET_MS,
        );
        assert_optional_u64_budget(
            &timeline,
            "p55",
            "followup_ready_snapshot_parse_exec_ms",
            ready_snapshot_parse_exec_ms,
            READY_SNAPSHOT_PARSE_EXEC_BUDGET_MS,
        );
        assert_optional_u64_budget(
            &timeline,
            "p55",
            "followup_ready_snapshot_parse_exec_core_parse_build_ms",
            ready_snapshot_core_parse_build_ms,
            READY_SNAPSHOT_CORE_PARSE_BUILD_BUDGET_MS,
        );
        assert_optional_u64_budget(
            &timeline,
            "p55",
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms",
            ready_snapshot_exact_assembly_ms,
            READY_SNAPSHOT_EXACT_ASSEMBLY_BUDGET_MS,
        );
        assert_optional_u64_budget(
            &timeline,
            "p55",
            "semantic_diagnostics_query_ms",
            semantic_diagnostics_query_ms,
            SEMANTIC_DIAGNOSTICS_QUERY_BUDGET_MS,
        );
        assert_optional_u64_budget(
            &timeline,
            "p55",
            "semantic_diagnostics_ir_ms",
            semantic_diagnostics_ir_ms,
            SEMANTIC_DIAGNOSTICS_IR_BUDGET_MS,
        );
        assert_optional_u64_budget(
            &timeline,
            "p55",
            "semantic_diagnostics_collect_ms",
            semantic_diagnostics_collect_ms,
            SEMANTIC_DIAGNOSTICS_COLLECT_BUDGET_MS,
        );
        assert_optional_u64_budget(
            &timeline,
            "p55",
            "diagnostics_only_local_function_summaries_ms",
            diagnostics_only_local_function_summaries_ms,
            LOCAL_FUNCTION_SUMMARIES_BUDGET_MS,
        );
        if ready_artifacts_publish
            .get("semantic_diagnostics_ir_ms")
            .and_then(|value| value.as_u64())
            .is_some()
        {
            assert_eq!(
                ready_artifacts_publish
                    .get("semantic_ir_source")
                    .and_then(|value| value.as_str()),
                Some("snapshot_build"),
                "p55 must attribute live semantic IR to snapshot_build when exporting IR breakdown, trace={timeline:?}"
            );
            for key in [
                "semantic_diagnostics_ir_semantic_facts_materialize_ms",
                "semantic_diagnostics_ir_semantic_facts_seed_module_context_ms",
                "semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms",
                "semantic_diagnostics_ir_semantic_facts_local_function_summaries_prep_ms",
                "semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_ms",
                "semantic_diagnostics_ir_semantic_facts_local_function_summaries_snapshot_build_ms",
                "semantic_diagnostics_ir_semantic_facts_local_function_summaries_body_infer_ms",
                "semantic_diagnostics_ir_semantic_facts_local_function_summaries_function_count",
                "semantic_diagnostics_ir_semantic_facts_local_function_summaries_scc_count",
                "semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_iteration_count",
                "semantic_diagnostics_ir_semantic_facts_local_function_summaries_singleton_fast_path_count",
                "semantic_diagnostics_ir_semantic_facts_local_function_summaries_recursive_scc_count",
                "semantic_diagnostics_ir_semantic_facts_visit_statements_ms",
                "semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms",
                "semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms",
                "semantic_diagnostics_ir_semantic_facts_visit_callable_body_count",
                "semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_count",
                "semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_ms",
                "semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_count",
                "semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_ms",
                "semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_count",
                "semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_ms",
                "semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_count",
                "semantic_diagnostics_ir_semantic_facts_statement_count",
                "semantic_diagnostics_ir_semantic_facts_local_function_summary_count",
                "semantic_diagnostics_ir_semantic_facts_index_entry_count",
            ] {
                assert_eq!(
                    ready_artifacts_publish
                        .get(key)
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0),
                    0,
                    "p55 must keep full-path-only semantic facts fields absent or zero on diagnostics_only materialization, key={key}, trace={timeline:?}"
                );
            }
            assert!(
                ready_artifacts_publish
                    .get("semantic_diagnostics_ir_ast_to_ir_convert_ms")
                    .and_then(|value| value.as_u64())
                    .is_some()
                    || ready_artifacts_publish
                        .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_ms")
                        .and_then(|value| value.as_u64())
                        .is_some()
                    || ready_artifacts_publish
                        .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_seed_module_context_ms")
                        .and_then(|value| value.as_u64())
                        .is_some()
                    || ready_artifacts_publish
                        .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_ms")
                        .and_then(|value| value.as_u64())
                        .is_some()
                    || ready_artifacts_publish
                        .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_statements_ms")
                        .and_then(|value| value.as_u64())
                        .is_some()
                    || ready_artifacts_publish
                        .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_callable_body_ms")
                        .and_then(|value| value.as_u64())
                        .is_some()
                    || ready_artifacts_publish
                        .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_merge_control_flow_env_ms")
                        .and_then(|value| value.as_u64())
                        .is_some(),
                "p55 must export AST->IR or diagnostics-only semantic facts leaf attribution when semantic IR latency is observed, trace={timeline:?}"
            );
            if diagnostics_only_local_function_summaries_ms.is_some() {
                assert!(
                    ready_artifacts_publish
                        .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_prep_ms")
                        .and_then(|value| value.as_u64())
                        .is_some()
                        && ready_artifacts_publish
                            .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_ms")
                            .and_then(|value| value.as_u64())
                            .is_some()
                        && ready_artifacts_publish
                            .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_snapshot_build_ms")
                            .and_then(|value| value.as_u64())
                            .is_some()
                        && ready_artifacts_publish
                            .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_body_infer_ms")
                            .and_then(|value| value.as_u64())
                            .is_some()
                        && ready_artifacts_publish
                            .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_function_count")
                            .and_then(|value| value.as_u64())
                            .is_some()
                        && ready_artifacts_publish
                            .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_scc_count")
                            .and_then(|value| value.as_u64())
                            .is_some()
                        && ready_artifacts_publish
                            .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_iteration_count")
                            .and_then(|value| value.as_u64())
                            .is_some()
                        && ready_artifacts_publish
                            .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_singleton_fast_path_count")
                            .and_then(|value| value.as_u64())
                            .is_some()
                        && ready_artifacts_publish
                            .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_recursive_scc_count")
                            .and_then(|value| value.as_u64())
                            .is_some(),
                    "p55 must export diagnostics-only local_function_summaries sub-breakdown when that IR subphase is observed, trace={timeline:?}"
                );
            }
            if let (Some(ir_ms), Some(ast_to_ir_ms), Some(diag_only_ms)) = (
                semantic_diagnostics_ir_ms,
                ready_artifacts_publish
                    .get("semantic_diagnostics_ir_ast_to_ir_convert_ms")
                    .and_then(|value| value.as_u64()),
                diagnostics_only_semantic_facts_ms,
            ) {
                assert!(
                    ir_ms >= ast_to_ir_ms.saturating_add(diag_only_ms),
                    "p55 diagnostics-only facts attribution must stay bounded by total semantic IR latency, trace={timeline:?}"
                );
            }
        }
        assert_ne!(
            timeline
                .get("followup_ready_snapshot_zero_probe")
                .and_then(|value| value.as_str()),
            Some("ready"),
            "p55 detached path must not claim a zero-budget ready snapshot hit, trace={timeline:?}"
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_zero_probe")
                .and_then(|value| value.as_str()),
            Some("not_ready"),
            "p55 detached path must prove the canonical zero-budget probe missed before publish, trace={timeline:?}"
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_wait_probe")
                .and_then(|value| value.as_str()),
            Some("not_ready"),
            "p55 detached path must keep the canonical bounded wait probe in not_ready while the detached winner is still-current, trace={timeline:?}"
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_bounded_wait_winner")
                .and_then(|value| value.as_str()),
            Some("detached_ready_artifacts"),
            "p55 detached path must attribute bounded wait completion to detached_ready_artifacts, trace={timeline:?}"
        );
        assert!(
            timeline
                .get("followup_ready_snapshot_bounded_wait_elapsed_ms")
                .and_then(|value| value.as_u64())
                .is_some(),
            "p55 detached path must export bounded wait elapsed attribution even when the detached wakeup rounds down to 0ms, trace={timeline:?}"
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_timeout_leaf")
                .and_then(|value| value.as_str()),
            None,
            "p55 detached bounded-wait winner must not fabricate a timeout leaf, trace={timeline:?}"
        );
        assert!(
            timeline
                .get("followup_ready_snapshot_dominant_phase")
                .and_then(|value| value.as_str())
                .is_some(),
            "p55 must export dominant ready-snapshot phase attribution on the production-like path, trace={timeline:?}"
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_dominant_phase")
                .and_then(|value| value.as_str()),
            Some("parse_exec"),
            "p55 detached bounded-wait winner must now expose parse_exec as the dominant canonical residual before timeout-sized ready_install can dominate, trace={timeline:?}"
        );

        let program_conversion_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms")
            .and_then(|value| value.as_u64());
        let packaging_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_reuse_outcome = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome")
            .and_then(|value| value.as_str());
        let program_lowering_reused_lowering_units = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuilt_lowering_units = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units")
            .and_then(|value| value.as_u64());
        let program_lowering_reused_window_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuilt_window_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count")
            .and_then(|value| value.as_u64());
        let program_lowering_largest_rebuilt_window_lowering_units = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units")
            .and_then(|value| value.as_u64());
        let program_lowering_fully_reused_top_level_node_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count")
            .and_then(|value| value.as_u64());
        let program_lowering_fully_rebuilt_top_level_node_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count")
            .and_then(|value| value.as_u64());
        let program_lowering_routine_body_reuse_node_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count")
            .and_then(|value| value.as_u64());
        let program_lowering_fully_reused_top_level_lowering_units = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units")
            .and_then(|value| value.as_u64());
        let program_lowering_fully_rebuilt_top_level_lowering_units = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units")
            .and_then(|value| value.as_u64());
        let program_lowering_routine_body_reused_prefix_lowering_units = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units")
            .and_then(|value| value.as_u64());
        let program_lowering_routine_body_reused_suffix_lowering_units = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units")
            .and_then(|value| value.as_u64());
        let program_lowering_routine_body_rebuilt_lowering_units = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units")
            .and_then(|value| value.as_u64());
        let program_lowering_reuse_plan_build_source = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source")
            .and_then(|value| value.as_str());
        let program_lowering_reuse_plan_take_if_unique_hit = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit")
            .and_then(|value| value.as_bool());
        let program_lowering_reuse_plan_borrowed_cache_hit = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit")
            .and_then(|value| value.as_bool());
        let program_lowering_reuse_plan_build_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_reuse_plan_owned_build_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_owned_build_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_reuse_plan_borrowed_build_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_build_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_reuse_plan_rebase_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_reuse_plan_rebase_statement_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_statement_count")
            .and_then(|value| value.as_u64());
        let program_lowering_reused_progress_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_reused_progress_call_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_call_count")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_call_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_call_count")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_callable_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_callable_call_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_call_count")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_callable_body_dispatch_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_callable_body_dispatch_call_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_call_count")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_control_flow_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_control_flow_call_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_call_count")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_simple_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_simple_call_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_call_count")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_other_ms = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_ms")
            .and_then(|value| value.as_u64());
        let program_lowering_rebuild_dispatch_other_call_count = timeline
            .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_call_count")
            .and_then(|value| value.as_u64());
        if program_lowering_ms.is_some() {
            let program_lowering_reuse_summary_present = program_lowering_reuse_outcome.is_some()
                || program_lowering_reused_lowering_units.is_some()
                || program_lowering_rebuilt_lowering_units.is_some()
                || program_lowering_reuse_plan_build_source.is_some();
            if program_lowering_reuse_summary_present {
                assert!(
                    program_lowering_reuse_outcome.is_some()
                        && program_lowering_reused_lowering_units.is_some()
                        && program_lowering_rebuilt_lowering_units.is_some()
                        && program_lowering_reused_window_count.is_some()
                        && program_lowering_rebuilt_window_count.is_some()
                        && program_lowering_largest_rebuilt_window_lowering_units.is_some()
                        && program_lowering_fully_reused_top_level_node_count.is_some()
                        && program_lowering_fully_rebuilt_top_level_node_count.is_some()
                        && program_lowering_routine_body_reuse_node_count.is_some()
                        && program_lowering_fully_reused_top_level_lowering_units.is_some()
                        && program_lowering_fully_rebuilt_top_level_lowering_units.is_some()
                        && program_lowering_routine_body_reused_prefix_lowering_units.is_some()
                        && program_lowering_routine_body_reused_suffix_lowering_units.is_some()
                        && program_lowering_routine_body_rebuilt_lowering_units.is_some()
                        && program_lowering_reuse_plan_build_source.is_some()
                        && program_lowering_reuse_plan_take_if_unique_hit.is_some()
                        && program_lowering_reuse_plan_borrowed_cache_hit.is_some()
                        && program_lowering_reuse_plan_build_ms.is_some()
                        && program_lowering_reuse_plan_rebase_statement_count.is_some()
                        && program_lowering_rebuild_dispatch_ms.is_some()
                        && program_lowering_rebuild_dispatch_call_count.is_some()
                        && program_lowering_rebuild_dispatch_callable_body_dispatch_ms.is_some()
                        && program_lowering_rebuild_dispatch_callable_body_dispatch_call_count
                            .is_some()
                        && program_lowering_rebuild_dispatch_control_flow_ms.is_some()
                        && program_lowering_rebuild_dispatch_control_flow_call_count.is_some(),
                    "p55 must export a complete bounded reuse-vs-rebuild summary when program-lowering reuse metadata is present, trace={timeline:?}"
                );
                match program_lowering_reuse_plan_build_source {
                    Some("owned") => assert!(
                        program_lowering_reuse_plan_owned_build_ms.is_some(),
                        "p55 owned reuse-plan builds must export owned_build_ms, trace={timeline:?}"
                    ),
                    Some("borrowed") => assert!(
                        program_lowering_reuse_plan_borrowed_build_ms.is_some(),
                        "p55 borrowed reuse-plan builds must export borrowed_build_ms, trace={timeline:?}"
                    ),
                    _ => {}
                }
                if program_lowering_reused_progress_ms.is_some() {
                    assert!(
                        program_lowering_reused_progress_call_count.is_some(),
                        "p55 reused-progress latency must not be exported without a matching call_count, trace={timeline:?}"
                    );
                }
                if program_lowering_rebuild_dispatch_callable_ms.is_some() {
                    assert!(
                        program_lowering_rebuild_dispatch_callable_call_count.is_some(),
                        "p55 callable rebuild dispatch latency must not be exported without a matching call_count, trace={timeline:?}"
                    );
                }
                if program_lowering_rebuild_dispatch_simple_ms.is_some() {
                    assert!(
                        program_lowering_rebuild_dispatch_simple_call_count.is_some(),
                        "p55 simple rebuild dispatch latency must not be exported without a matching call_count, trace={timeline:?}"
                    );
                }
                if program_lowering_rebuild_dispatch_other_ms.is_some() {
                    assert!(
                        program_lowering_rebuild_dispatch_other_call_count.is_some(),
                        "p55 other rebuild dispatch latency must not be exported without a matching call_count, trace={timeline:?}"
                    );
                }
            }
        }
        if let (Some(program_conversion_ms), Some(program_lowering_ms)) =
            (program_conversion_ms, program_lowering_ms)
        {
            assert!(
                program_conversion_ms >= program_lowering_ms,
                "p55 exported program_conversion_ms must stay coherent with program_lowering_ms, trace={timeline:?}"
            );
        }
        if let (Some(program_conversion_ms), Some(packaging_ms)) =
            (program_conversion_ms, packaging_ms)
        {
            assert!(
                program_conversion_ms >= packaging_ms,
                "p55 exported program_conversion_ms must stay coherent with packaging_ms, trace={timeline:?}"
            );
        }

        let observability = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let response =
                    live_transport_get_observability_metrics_response(&mut harness, 55_100_901)
                        .await;
                let evidence = response
                    .get("didChangeParseSnapshotEvidence")
                    .and_then(|value| value.get("entries"))
                    .and_then(|value| value.as_array())
                    .and_then(|entries| {
                        entries.iter().find(|entry| {
                            entry.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                                && entry
                                    .get("requestedVersion")
                                    .and_then(|value| value.as_i64())
                                    == Some(3)
                        })
                    })
                    .cloned();
                if let Some(entry) = evidence {
                    break entry;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .ok();
        if let Some(observability) = observability.as_ref() {
            assert_eq!(
                observability.get("parseMode").and_then(|value| value.as_str()),
                Some("incremental")
            );
            assert_eq!(
                observability.get("baseTextSource").and_then(|value| value.as_str()),
                Some("shadow_state")
            );
            assert_eq!(
                observability.get("fallbackReason").and_then(|value| value.as_str()),
                None
            );
            assert_eq!(
                observability
                    .get("parserBaseRootCause")
                    .and_then(|value| value.as_str()),
                None
            );
        } else {
            panic!(
                "p55 must retain didChange observability evidence on the production-like path, trace={timeline:?}"
            );
        }

        let final_metrics = live_transport_get_observability_metrics(&mut harness, 55_100_903).await;
        let final_histograms = final_metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("final metrics.histograms object");
        let did_change_materialization_histogram = final_histograms
            .get("intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_did_change")
            .and_then(|value| value.as_object())
            .expect("p55 did_change materialization histogram");
        let did_change_materialization_histogram_count = read_u64_metric(
            did_change_materialization_histogram.get("count"),
        );
        let did_change_materialization_p50_ms =
            read_numeric_metric(did_change_materialization_histogram.get("p50"));
        let did_change_materialization_p95_ms =
            read_numeric_metric(did_change_materialization_histogram.get("p95"));
        assert!(
            did_change_materialization_histogram_count > 0,
            "p55 must export did_change ready-snapshot materialization latency, final_histograms={final_histograms:?}"
        );
        let baseline_refactor_41_representative_bundle = serde_json::json!({
            "captured_at": BASELINE_CAPTURED_AT,
            "did_change_ready_snapshot_materialization_ms": {
                "p50": BASELINE_DID_CHANGE_MATERIALIZATION_P50_MS,
                "p95": BASELINE_DID_CHANGE_MATERIALIZATION_P95_MS,
            },
        });
        let did_change_ready_snapshot_materialization = serde_json::json!({
            "histogram_count": did_change_materialization_histogram_count,
            "p50_ms": did_change_materialization_p50_ms,
            "p95_ms": did_change_materialization_p95_ms,
            "p50_vs_refactor_41_baseline_delta_ms": did_change_materialization_p50_ms
                - BASELINE_DID_CHANGE_MATERIALIZATION_P50_MS,
            "p95_vs_refactor_41_baseline_delta_ms": did_change_materialization_p95_ms
                - BASELINE_DID_CHANGE_MATERIALIZATION_P95_MS,
        });

        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("backend crate must live under the workspace root");
        let baseline_report_path = workspace_root
            .join("backend")
            .join("tests")
            .join("perf")
            .join("reports")
            .join("refactor-36-diagnostics-semantic-hints-split-real-conf-big-diagnostics-ready-snapshot-leaf-live.json");
        let baseline_report = std::fs::read_to_string(&baseline_report_path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
        let baseline_refactor_36_semantic_ir_ms = baseline_report
            .as_ref()
            .and_then(|value| {
                value
                    .get("followup_publish_semantic_diagnostics_ir_ms")
                    .and_then(|value| value.as_u64())
            });
        let baseline_refactor_36_ast_to_ir_convert_ms = baseline_report
            .as_ref()
            .and_then(|value| {
                value
                    .get("followup_publish_semantic_diagnostics_ir_ast_to_ir_convert_ms")
                    .and_then(|value| value.as_u64())
            });
        let baseline_refactor_36_unattributed_diagnostics_ir_residual_ms =
            baseline_refactor_36_semantic_ir_ms.zip(baseline_refactor_36_ast_to_ir_convert_ms).map(
                |(ir_ms, ast_to_ir_ms)| ir_ms.saturating_sub(ast_to_ir_ms),
            );
        let current_diagnostics_ir_residual_after_attribution_ms = semantic_diagnostics_ir_ms
            .zip(
                ready_artifacts_publish
                    .get("semantic_diagnostics_ir_ast_to_ir_convert_ms")
                    .and_then(|value| value.as_u64()),
            )
            .zip(diagnostics_only_semantic_facts_ms)
            .map(|((ir_ms, ast_to_ir_ms), diag_only_ms)| {
                ir_ms
                    .saturating_sub(ast_to_ir_ms)
                    .saturating_sub(diag_only_ms)
            });
        let diagnostics_only_semantic_facts_vs_refactor_36_unattributed_delta_ms =
            diagnostics_only_semantic_facts_ms
                .zip(baseline_refactor_36_unattributed_diagnostics_ir_residual_ms)
                .map(|(current_ms, baseline_ms)| current_ms as i64 - baseline_ms as i64);

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "baseline_refactor_36_report_path": baseline_report_path.display().to_string(),
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "did_change_versions": [2, 3],
            "apply_delay_ms": APPLY_DELAY_MS,
            "did_change_blocking_parse_delay_ms": DID_CHANGE_BLOCKING_PARSE_DELAY_MS,
            "did_change_evidence_present": observability.is_some(),
            "baseline_refactor_41_representative_bundle": baseline_refactor_41_representative_bundle,
            "parse_mode": observability.as_ref().and_then(|value| value.get("parseMode")).and_then(|value| value.as_str()),
            "base_text_source": observability.as_ref().and_then(|value| value.get("baseTextSource")).and_then(|value| value.as_str()),
            "change_shape": observability.as_ref().and_then(|value| value.get("changeShape")).and_then(|value| value.as_str()),
            "content_changes_count": observability.as_ref().and_then(|value| value.get("contentChangesCount")).and_then(|value| value.as_u64()),
            "replay_order": observability.as_ref().and_then(|value| value.get("replayOrder")).and_then(|value| value.as_str()),
            "base_document_version": observability.as_ref().and_then(|value| value.get("baseDocumentVersion")).and_then(|value| value.as_i64()),
            "changed_ranges_count": observability.as_ref().and_then(|value| value.get("changedRangesCount")).and_then(|value| value.as_u64()),
            "fallback_reason": observability.as_ref().and_then(|value| value.get("fallbackReason")).and_then(|value| value.as_str()),
            "parser_base_root_cause": observability.as_ref().and_then(|value| value.get("parserBaseRootCause")).and_then(|value| value.as_str()),
            "did_change_ready_snapshot_materialization": did_change_ready_snapshot_materialization,
            "save_cycle_sequence": timeline
                .get("save_cycle_sequence")
                .and_then(|value| value.as_u64()),
            "diagnostics_generation": timeline
                .get("diagnostics_generation")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_zero_probe": timeline
                .get("followup_ready_snapshot_zero_probe")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_wait_probe": timeline
                .get("followup_ready_snapshot_wait_probe")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_bounded_wait_winner": timeline
                .get("followup_ready_snapshot_bounded_wait_winner")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_bounded_wait_elapsed_ms": timeline
                .get("followup_ready_snapshot_bounded_wait_elapsed_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_task_state": timeline
                .get("followup_ready_snapshot_task_state")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_timeout_phase": timeline
                .get("followup_ready_snapshot_timeout_phase")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_timeout_leaf": timeline
                .get("followup_ready_snapshot_timeout_leaf")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_timeout_leaf_elapsed_ms": timeline
                .get("followup_ready_snapshot_timeout_leaf_elapsed_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_ms": timeline
                .get("followup_ready_snapshot_parse_exec_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_timeout_subphase": timeline
                .get("followup_ready_snapshot_parse_exec_timeout_subphase")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms": timeline
                .get("followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_parse_build_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_parse_build_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms": program_conversion_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms": program_lowering_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome": program_lowering_reuse_outcome,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units": program_lowering_reused_lowering_units,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units": program_lowering_rebuilt_lowering_units,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count": program_lowering_reused_window_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count": program_lowering_rebuilt_window_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units": program_lowering_largest_rebuilt_window_lowering_units,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count": program_lowering_fully_reused_top_level_node_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count": program_lowering_fully_rebuilt_top_level_node_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count": program_lowering_routine_body_reuse_node_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units": program_lowering_fully_reused_top_level_lowering_units,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units": program_lowering_fully_rebuilt_top_level_lowering_units,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units": program_lowering_routine_body_reused_prefix_lowering_units,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units": program_lowering_routine_body_reused_suffix_lowering_units,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units": program_lowering_routine_body_rebuilt_lowering_units,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source": program_lowering_reuse_plan_build_source,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit": program_lowering_reuse_plan_take_if_unique_hit,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit": program_lowering_reuse_plan_borrowed_cache_hit,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms": program_lowering_reuse_plan_build_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_owned_build_ms": program_lowering_reuse_plan_owned_build_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_build_ms": program_lowering_reuse_plan_borrowed_build_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_ms": program_lowering_reuse_plan_rebase_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_statement_count": program_lowering_reuse_plan_rebase_statement_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms": program_lowering_reused_progress_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_call_count": program_lowering_reused_progress_call_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms": program_lowering_rebuild_dispatch_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_call_count": program_lowering_rebuild_dispatch_call_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_ms": program_lowering_rebuild_dispatch_callable_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_call_count": program_lowering_rebuild_dispatch_callable_call_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_ms": program_lowering_rebuild_dispatch_callable_body_dispatch_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_call_count": program_lowering_rebuild_dispatch_callable_body_dispatch_call_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms": program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_ms": program_lowering_rebuild_dispatch_control_flow_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_call_count": program_lowering_rebuild_dispatch_control_flow_call_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_ms": program_lowering_rebuild_dispatch_simple_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_call_count": program_lowering_rebuild_dispatch_simple_call_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_ms": program_lowering_rebuild_dispatch_other_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_call_count": program_lowering_rebuild_dispatch_other_call_count,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms": packaging_ms,
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms": timeline
                .get("followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_dominant_subphase": timeline
                .get("followup_ready_snapshot_parse_exec_dominant_subphase")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_parse_exec_dominant_subphase_ms": timeline
                .get("followup_ready_snapshot_parse_exec_dominant_subphase_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_post_parse_pre_materialization_ms": timeline
                .get("followup_ready_snapshot_post_parse_pre_materialization_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_ready_install_ms": timeline
                .get("followup_ready_snapshot_ready_install_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_document_symbol_side_work_ms": timeline
                .get("followup_ready_snapshot_document_symbol_side_work_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_dominant_phase": timeline
                .get("followup_ready_snapshot_dominant_phase")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_dominant_phase_ms": timeline
                .get("followup_ready_snapshot_dominant_phase_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_relief_valve_outcome": timeline
                .get("followup_ready_snapshot_relief_valve_outcome")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_continuation_reason": timeline
                .get("followup_ready_snapshot_continuation_reason")
                .and_then(|value| value.as_str()),
            "followup_semantic_path": timeline
                .get("followup_semantic_path")
                .and_then(|value| value.as_str()),
            "followup_semantic_materialization_path": timeline
                .get("followup_semantic_materialization_path")
                .and_then(|value| value.as_str()),
            "followup_publish_semantic_path": followup_semantic_path,
            "followup_publish_semantic_materialization_path": followup_semantic_materialization_path,
            "followup_publish_semantic_parse_source": ready_artifacts_publish
                .get("semantic_parse_source")
                .and_then(|value| value.as_str()),
            "followup_publish_semantic_ir_source": ready_artifacts_publish
                .get("semantic_ir_source")
                .and_then(|value| value.as_str()),
            "followup_publish_semantic_diagnostics_inputs_ms": ready_artifacts_publish
                .get("semantic_diagnostics_inputs_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_parse_result_ms": ready_artifacts_publish
                .get("semantic_diagnostics_parse_result_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_collect_ms": ready_artifacts_publish
                .get("semantic_diagnostics_collect_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_flow_sensitive_ms": ready_artifacts_publish
                .get("semantic_diagnostics_flow_sensitive_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_ast_to_ir_convert_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_ast_to_ir_convert_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_ms": diagnostics_only_semantic_facts_ms,
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_seed_module_context_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_seed_module_context_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_ms": diagnostics_only_local_function_summaries_ms,
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_prep_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_prep_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_snapshot_build_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_snapshot_build_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_body_infer_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_body_infer_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_function_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_function_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_scc_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_scc_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_iteration_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_iteration_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_singleton_fast_path_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_singleton_fast_path_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_recursive_scc_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_recursive_scc_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_statements_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_statements_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_callable_body_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_callable_body_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_merge_control_flow_env_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_merge_control_flow_env_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_callable_body_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_callable_body_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_merge_control_flow_env_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_merge_control_flow_env_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_statement_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_statement_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summary_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summary_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_diagnostics_only_semantic_facts_index_entry_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_diagnostics_only_semantic_facts_index_entry_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_materialize_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_materialize_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_seed_module_context_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_seed_module_context_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summaries_prep_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_local_function_summaries_prep_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summaries_snapshot_build_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_local_function_summaries_snapshot_build_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summaries_body_infer_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_local_function_summaries_body_infer_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summaries_function_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_local_function_summaries_function_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summaries_scc_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_local_function_summaries_scc_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_iteration_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_iteration_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summaries_singleton_fast_path_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_local_function_summaries_singleton_fast_path_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summaries_recursive_scc_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_local_function_summaries_recursive_scc_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_visit_statements_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_visit_statements_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_visit_callable_body_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_visit_callable_body_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_ms": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_statement_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_statement_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summary_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_local_function_summary_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_index_entry_count": ready_artifacts_publish
                .get("semantic_diagnostics_ir_semantic_facts_index_entry_count")
                .and_then(|value| value.as_u64()),
            "followup_publish_elapsed_ms": ready_artifacts_publish
                .get("elapsed_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_query_ms": ready_artifacts_publish
                .get("semantic_diagnostics_query_ms")
                .and_then(|value| value.as_u64()),
            "semantic_query_dominates_ready_snapshot_parse_exec": semantic_query_dominates_ready_snapshot_parse_exec,
            "current_diagnostics_ir_residual_after_attribution_ms": current_diagnostics_ir_residual_after_attribution_ms,
            "baseline_refactor_36_followup_publish_semantic_diagnostics_ir_ms": baseline_refactor_36_semantic_ir_ms,
            "baseline_refactor_36_followup_publish_semantic_diagnostics_ir_ast_to_ir_convert_ms": baseline_refactor_36_ast_to_ir_convert_ms,
            "baseline_refactor_36_unattributed_diagnostics_ir_residual_ms": baseline_refactor_36_unattributed_diagnostics_ir_residual_ms,
            "diagnostics_only_semantic_facts_vs_refactor_36_unattributed_delta_ms": diagnostics_only_semantic_facts_vs_refactor_36_unattributed_delta_ms,
            "program_conversion_coherent_with_program_lowering": program_conversion_ms
                .zip(program_lowering_ms)
                .map(|(conversion, lowering)| conversion >= lowering),
            "program_conversion_coherent_with_publishable_artifact_packaging": program_conversion_ms
                .zip(packaging_ms)
                .map(|(conversion, packaging)| conversion >= packaging),
            "v2_text_len_bytes": v2_text.len(),
            "v3_text_len_bytes": v3_text.len(),
            "final_statement_len_bytes": V3_STATEMENT.len(),
        });
        let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_READY_SNAPSHOT_LEAF_REPORT")
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
                        "{change_id}-real-conf-big-diagnostics-ready-snapshot-leaf-live.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p55 real conf_big ready-snapshot leaf report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p55 real conf_big ready-snapshot leaf report"),
        )
        .expect("write p55 real conf_big ready-snapshot leaf report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}
