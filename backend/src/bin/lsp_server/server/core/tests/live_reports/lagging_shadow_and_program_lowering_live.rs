#[test]
fn p52_real_conf_big_lagging_shadow_recovery_save_followup_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p52 tokio runtime");
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
            "p52_real_conf_big_lagging_shadow_recovery_save_followup_report_live";
        const APPLY_DELAY_MS: u64 = 4_000;
        const DID_CHANGE_BLOCKING_PARSE_DELAY_MS: u64 = 1_500;
        const TIMELINE_OBSERVE_BUDGET_MS: u64 = 30_000;

        let _env_lock = lock_test_env().await;
        let _apply_delay_guard =
            EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", &APPLY_DELAY_MS.to_string());
        let did_change_blocking_parse_delay_guard = EnvVarGuard::set(
            "BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS",
            &DID_CHANGE_BLOCKING_PARSE_DELAY_MS.to_string(),
        );
        let _debounce_guard =
            EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "1200", true);

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-32-ready-snapshot-shadow-state-lag-reduction".to_string()
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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p52 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p52_real_conf_big_live_setup")
            .await;

        let uri = Url::from_file_path(&module_path).expect("real conf_big module uri");
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
        .expect("didOpen must register version 1 for p52");
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
                    .is_some_and(|state| state.parse_snapshot.file_version == 1)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("didOpen must materialize same-version ready parse snapshot for p52");

        let appended_procedure = "\nПроцедура Refactor25Live()\n    Возврат;\nКонецПроцедуры\n";
        let semantic_procedure =
            "\nПроцедура Refactor25Live()\n    Сообщить(НеобъявленнаяПеременная);\nКонецПроцедуры\n";
        let v2_text = format!("{module_text}{appended_procedure}");
        live_transport_ranged_did_change(
            &mut harness,
            &uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: v2_text.clone(),
            }],
        )
        .await;

        let v3_text = format!("{module_text}{semantic_procedure}");
        live_transport_ranged_did_change(
            &mut harness,
            &uri,
            3,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: v3_text.clone(),
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
                if shadow_version == Some(3) && ready_version == Some(1) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("same-file churn must advance shadow state to v3 while ready snapshot still lags at v1 for p52");

        drop(did_change_blocking_parse_delay_guard);

        let replacement_start =
            find_utf16_position_after_marker(&v3_text, "Процедура Refactor25Live()\n    ");
        let replacement_end = find_utf16_position_after_marker(
            &v3_text,
            "Процедура Refactor25Live()\n    Сообщить(НеобъявленнаяПеременная);",
        );
        let v4_statement = "Сообщить(СовсемНеобъявленнаяПеременная);";
        live_transport_ranged_did_change(
            &mut harness,
            &uri,
            4,
            vec![TextDocumentContentChangeEvent {
                range: Some(Range {
                    start: replacement_start,
                    end: replacement_end,
                }),
                range_length: None,
                text: v4_statement.to_string(),
            }],
        )
        .await;

        tokio::time::timeout(Duration::from_secs(5), async {
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
                if server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(4)
                    && task_requested_version == Some(4)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("didChange must schedule version 4 exact worker for p52");

        live_transport_save_document(&mut harness, &uri).await;

        let timeline_deadline = Instant::now() + Duration::from_millis(TIMELINE_OBSERVE_BUDGET_MS);
        let timeline = loop {
            let timeline =
                live_transport_get_diagnostics_save_timeline(&mut harness, 52_100_902, 12).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces");
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
                    panic!("p52 must expose a diagnostics save trace for requested_version=4");
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
                    "p52 must observe either a follow-up publish or a bounded follow-up semantic-path decision on the lagging-shadow recovery path, last_trace={trace:?}"
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
        assert!(
            matches!(followup_semantic_path, Some("ready_artifacts") | Some("shadow_state")),
            "p52 must expose whether save follow-up returned to ready_artifacts or still fell back, trace={timeline:?}"
        );
        if followup_semantic_path == Some("ready_artifacts") {
            let full_publish = full_publish.expect(
                "ready_artifacts live path must expose an idle_heavy follow-up publish object",
            );
            assert_eq!(
                full_publish
                    .get("semantic_parse_source")
                    .and_then(|value| value.as_str()),
                Some("snapshot")
            );
        } else {
            assert_eq!(
                timeline
                    .get("followup_ready_snapshot_timeout_phase")
                    .and_then(|value| value.as_str()),
                Some("parse_exec"),
                "if mixed load still falls back after refactor-29, the remaining bounded cause must be parse_exec, trace={timeline:?}"
            );
            assert_eq!(
                timeline
                    .get("followup_ready_snapshot_parse_exec_timeout_subphase")
                    .and_then(|value| value.as_str()),
                Some("core_parse_build"),
                "if mixed load still falls back after refactor-29, the residual must stay inside exact core_parse_build, trace={timeline:?}"
            );
            assert_eq!(
                timeline
                    .get("followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint")
                    .and_then(|value| value.as_str()),
                Some("exact_ready_snapshot_assembly"),
                "if mixed load still falls back after refactor-29, the residual must stay inside exact ready_snapshot_assembly, trace={timeline:?}"
            );
            assert!(
                timeline
                    .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint")
                    .and_then(|value| value.as_str())
                    .is_some(),
                "if mixed load still falls back after refactor-29, the residual must expose a bounded exact ready_snapshot_assembly slice, trace={timeline:?}"
            );
        }

        let observability = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let response =
                    live_transport_get_observability_metrics_response(&mut harness, 52_100_901)
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
                                    == Some(4)
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
            assert_eq!(
                followup_semantic_path,
                Some("shadow_state"),
                "missing didChange evidence is acceptable only when the mixed live path still falls back before exact worker materialization, trace={timeline:?}"
            );
        }

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "did_change_versions": [2, 3, 4],
            "apply_delay_ms": APPLY_DELAY_MS,
            "initial_did_change_blocking_parse_delay_ms": DID_CHANGE_BLOCKING_PARSE_DELAY_MS,
            "did_change_evidence_present": observability.is_some(),
            "parse_mode": observability.as_ref().and_then(|value| value.get("parseMode")).and_then(|value| value.as_str()),
            "base_text_source": observability.as_ref().and_then(|value| value.get("baseTextSource")).and_then(|value| value.as_str()),
            "change_shape": observability.as_ref().and_then(|value| value.get("changeShape")).and_then(|value| value.as_str()),
            "content_changes_count": observability.as_ref().and_then(|value| value.get("contentChangesCount")).and_then(|value| value.as_u64()),
            "replay_order": observability.as_ref().and_then(|value| value.get("replayOrder")).and_then(|value| value.as_str()),
            "base_document_version": observability.as_ref().and_then(|value| value.get("baseDocumentVersion")).and_then(|value| value.as_i64()),
            "changed_ranges_count": observability.as_ref().and_then(|value| value.get("changedRangesCount")).and_then(|value| value.as_u64()),
            "fallback_reason": observability.as_ref().and_then(|value| value.get("fallbackReason")).and_then(|value| value.as_str()),
            "parser_base_root_cause": observability.as_ref().and_then(|value| value.get("parserBaseRootCause")).and_then(|value| value.as_str()),
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
            "followup_ready_snapshot_timeout_phase": timeline
                .get("followup_ready_snapshot_timeout_phase")
                .and_then(|value| value.as_str()),
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
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms")
                .and_then(|value| value.as_u64()),
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
            "followup_ready_snapshot_relief_valve_outcome": timeline
                .get("followup_ready_snapshot_relief_valve_outcome")
                .and_then(|value| value.as_str()),
            "followup_semantic_path": timeline
                .get("followup_semantic_path")
                .and_then(|value| value.as_str()),
            "followup_publish_semantic_path": followup_semantic_path,
            "followup_publish_semantic_parse_source": full_publish
                .and_then(|publish| publish.get("semantic_parse_source"))
                .and_then(|value| value.as_str()),
            "followup_publish_elapsed_ms": full_publish
                .and_then(|publish| publish.get("elapsed_ms"))
                .and_then(|value| value.as_u64()),
            "v2_text_len_bytes": v2_text.len(),
            "v3_text_len_bytes": v3_text.len(),
            "v4_statement_len_bytes": v4_statement.len(),
        });
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("backend crate must live under the workspace root");
        let report_path = std::env::var(
            "BSL_V2_REAL_CONF_BIG_LAGGING_SHADOW_RECOVERY_SAVE_FOLLOWUP_REPORT",
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
                    "{change_id}-real-conf-big-lagging-shadow-recovery-save-followup-live.json"
                ))
        });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p52 real conf_big recovery report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p52 real conf_big recovery report"),
        )
        .expect("write p52 real conf_big recovery report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}

#[test]
fn p53_real_conf_big_exact_program_lowering_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p53 tokio runtime");
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

        const PROFILE_NAME: &str = "p53_real_conf_big_exact_program_lowering_report_live";
        const APPLY_DELAY_MS: u64 = 4_000;
        const DID_CHANGE_BLOCKING_PARSE_DELAY_MS: u64 = 1_500;
        const TIMELINE_OBSERVE_BUDGET_MS: u64 = 30_000;
        const V1_STATEMENT: &str = "СтруктураВозврата = Новый Структура;";
        const V2_STATEMENT: &str = "СтруктураВозврата = НеобъявленнаяПеременная;";
        const V3_STATEMENT: &str = "СтруктураВозврата = ЕщеНеобъявленнаяПеременная;";
        // Keep generous headroom over current live conf_big runs while still
        // failing if the changed-range exact worker regresses back into a
        // multi-second path on the representative module.
        const FOLLOWUP_PUBLISH_ELAPSED_BUDGET_MS: u64 = 3_000;
        const PROGRAM_CONVERSION_BUDGET_MS: u64 = 1_200;
        const PROGRAM_LOWERING_BUDGET_MS: u64 = 1_200;

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
        let _debounce_guard =
            EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "1200", true);

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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p53 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p53_real_conf_big_live_setup")
            .await;

        let uri = Url::from_file_path(&module_path).expect("real conf_big module uri");
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
        .expect("didOpen must register version 1 for p53");
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
                    .is_some_and(|state| state.parse_snapshot.file_version == 1)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("didOpen must materialize same-version ready parse snapshot for p53");

        let v2_text = module_text.replacen(V1_STATEMENT, V2_STATEMENT, 1);
        assert_ne!(
            v2_text, module_text,
            "p53 fixture must edit an existing statement inside the representative module"
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
        .expect("local ranged didChange must materialize same-version ready snapshot v2 for p53");

        let did_change_blocking_parse_delay_guard = EnvVarGuard::set(
            "BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS",
            &DID_CHANGE_BLOCKING_PARSE_DELAY_MS.to_string(),
        );
        let v3_text = v2_text.replacen(V2_STATEMENT, V3_STATEMENT, 1);
        assert_ne!(
            v3_text, v2_text,
            "p53 fixture must keep churn inside the same live statement before the ranged edit"
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
                if shadow_version == Some(3) && ready_version == Some(2) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("same-file churn must advance shadow state to v3 while ready snapshot still lags at v2 for p53");

        drop(did_change_blocking_parse_delay_guard);

        tokio::time::timeout(Duration::from_secs(5), async {
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
                if server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(3)
                    && task_requested_version == Some(3)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("didChange must schedule version 3 exact worker for p53");

        live_transport_save_document(&mut harness, &uri).await;

        let timeline_deadline = Instant::now() + Duration::from_millis(TIMELINE_OBSERVE_BUDGET_MS);
        let timeline = loop {
            let timeline =
                live_transport_get_diagnostics_save_timeline(&mut harness, 53_100_902, 12).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces");
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
                    panic!("p53 must expose a diagnostics save trace for requested_version=3");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };
            let followup_publish = trace
                .get("followup_publish")
                .and_then(|value| value.as_object());
            let program_lowering_observed = trace
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms")
                .and_then(|value| value.as_u64())
                .is_some();
            if followup_publish.is_some() || program_lowering_observed {
                break trace;
            }
            if Instant::now() >= timeline_deadline {
                panic!(
                    "p53 must observe either the final idle_heavy follow-up publish or exact program_lowering attribution on the representative live path, last_trace={trace:?}"
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
        if let Some(ready_artifacts_publish) = full_publish
            .filter(|_| followup_semantic_path == Some("ready_artifacts"))
        {
            assert_eq!(
                ready_artifacts_publish
                    .get("semantic_parse_source")
                    .and_then(|value| value.as_str()),
                Some("snapshot")
            );
        }
        let followup_publish_elapsed_ms = full_publish
            .and_then(|publish| publish.get("elapsed_ms"))
            .and_then(|value| value.as_u64());

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
        assert_optional_u64_budget(
            &timeline,
            "p53",
            "followup_publish_elapsed_ms",
            followup_publish_elapsed_ms,
            FOLLOWUP_PUBLISH_ELAPSED_BUDGET_MS,
        );
        assert_optional_u64_budget(
            &timeline,
            "p53",
            "program_conversion_ms",
            program_conversion_ms,
            PROGRAM_CONVERSION_BUDGET_MS,
        );
        assert_optional_u64_budget(
            &timeline,
            "p53",
            "program_lowering_ms",
            program_lowering_ms,
            PROGRAM_LOWERING_BUDGET_MS,
        );
        if let (Some(program_conversion_ms), Some(program_lowering_ms)) =
            (program_conversion_ms, program_lowering_ms)
        {
            assert!(
                program_conversion_ms >= program_lowering_ms,
                "exported program_conversion_ms must stay coherent with program_lowering_ms, trace={timeline:?}"
            );
        }
        if let (Some(program_conversion_ms), Some(packaging_ms)) =
            (program_conversion_ms, packaging_ms)
        {
            assert!(
                program_conversion_ms >= packaging_ms,
                "exported program_conversion_ms must stay coherent with packaging_ms, trace={timeline:?}"
            );
        }
        if let Some(program_lowering_reuse_outcome) = program_lowering_reuse_outcome {
            assert!(
                matches!(
                    program_lowering_reuse_outcome,
                    "top_level_reuse" | "routine_body_reuse"
                ),
                "p53 must exercise a changed-range lowering reuse path rather than legacy prefix reuse, trace={timeline:?}"
            );
            assert!(
                program_lowering_reused_lowering_units.is_some()
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
                    && program_lowering_reuse_plan_rebase_ms.is_some()
                    && program_lowering_reuse_plan_rebase_statement_count.is_some()
                    && program_lowering_reused_progress_ms.is_some()
                    && program_lowering_reused_progress_call_count.is_some()
                    && program_lowering_rebuild_dispatch_ms.is_some()
                    && program_lowering_rebuild_dispatch_call_count.is_some()
                    && program_lowering_rebuild_dispatch_callable_ms.is_some()
                    && program_lowering_rebuild_dispatch_callable_call_count.is_some()
                    && program_lowering_rebuild_dispatch_callable_body_dispatch_ms.is_some()
                    && program_lowering_rebuild_dispatch_callable_body_dispatch_call_count.is_some()
                    && program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms.is_some()
                    && program_lowering_rebuild_dispatch_control_flow_ms.is_some()
                    && program_lowering_rebuild_dispatch_control_flow_call_count.is_some()
                    && program_lowering_rebuild_dispatch_simple_ms.is_some()
                    && program_lowering_rebuild_dispatch_simple_call_count.is_some()
                    && program_lowering_rebuild_dispatch_other_ms.is_some()
                    && program_lowering_rebuild_dispatch_other_call_count.is_some(),
                "p53 live trace must export bounded reuse-vs-rebuild summaries when exact program_lowering is observed, trace={timeline:?}"
            );
        }

        let observability = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let response =
                    live_transport_get_observability_metrics_response(&mut harness, 53_100_901)
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
                "p53 must retain didChange observability evidence after publishing through ready_artifacts, trace={timeline:?}"
            );
        }

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "did_change_versions": [2, 3],
            "apply_delay_ms": APPLY_DELAY_MS,
            "initial_did_change_blocking_parse_delay_ms": DID_CHANGE_BLOCKING_PARSE_DELAY_MS,
            "did_change_evidence_present": observability.is_some(),
            "parse_mode": observability.as_ref().and_then(|value| value.get("parseMode")).and_then(|value| value.as_str()),
            "base_text_source": observability.as_ref().and_then(|value| value.get("baseTextSource")).and_then(|value| value.as_str()),
            "change_shape": observability.as_ref().and_then(|value| value.get("changeShape")).and_then(|value| value.as_str()),
            "content_changes_count": observability.as_ref().and_then(|value| value.get("contentChangesCount")).and_then(|value| value.as_u64()),
            "replay_order": observability.as_ref().and_then(|value| value.get("replayOrder")).and_then(|value| value.as_str()),
            "base_document_version": observability.as_ref().and_then(|value| value.get("baseDocumentVersion")).and_then(|value| value.as_i64()),
            "changed_ranges_count": observability.as_ref().and_then(|value| value.get("changedRangesCount")).and_then(|value| value.as_u64()),
            "fallback_reason": observability.as_ref().and_then(|value| value.get("fallbackReason")).and_then(|value| value.as_str()),
            "parser_base_root_cause": observability.as_ref().and_then(|value| value.get("parserBaseRootCause")).and_then(|value| value.as_str()),
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
            "followup_ready_snapshot_timeout_phase": timeline
                .get("followup_ready_snapshot_timeout_phase")
                .and_then(|value| value.as_str()),
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
            "followup_ready_snapshot_relief_valve_outcome": timeline
                .get("followup_ready_snapshot_relief_valve_outcome")
                .and_then(|value| value.as_str()),
            "followup_semantic_path": timeline
                .get("followup_semantic_path")
                .and_then(|value| value.as_str()),
            "followup_publish_semantic_path": followup_semantic_path,
            "followup_publish_semantic_parse_source": full_publish
                .and_then(|publish| publish.get("semantic_parse_source"))
                .and_then(|value| value.as_str()),
            "followup_publish_semantic_ir_source": full_publish
                .and_then(|publish| publish.get("semantic_ir_source"))
                .and_then(|value| value.as_str()),
            "followup_publish_semantic_diagnostics_query_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_query_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_inputs_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_inputs_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_parse_result_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_parse_result_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_collect_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_collect_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_flow_sensitive_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_flow_sensitive_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_ast_to_ir_convert_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_ast_to_ir_convert_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_materialize_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_semantic_facts_materialize_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_seed_module_context_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_semantic_facts_seed_module_context_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_visit_statements_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_semantic_facts_visit_statements_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_visit_callable_body_count": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_semantic_facts_visit_callable_body_count"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_count": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_count"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_statement_count": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_semantic_facts_statement_count"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_local_function_summary_count": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_semantic_facts_local_function_summary_count"))
                .and_then(|value| value.as_u64()),
            "followup_publish_semantic_diagnostics_ir_semantic_facts_index_entry_count": full_publish
                .and_then(|publish| publish.get("semantic_diagnostics_ir_semantic_facts_index_entry_count"))
                .and_then(|value| value.as_u64()),
            "followup_publish_elapsed_ms": full_publish
                .and_then(|publish| publish.get("elapsed_ms"))
                .and_then(|value| value.as_u64()),
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
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("backend crate must live under the workspace root");
        let report_path =
            std::env::var("BSL_V2_REAL_CONF_BIG_EXACT_PROGRAM_LOWERING_REPORT")
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
                            "{change_id}-real-conf-big-exact-program-lowering-live.json"
                        ))
                });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p53 real conf_big program-lowering report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p53 real conf_big program-lowering report"),
        )
        .expect("write p53 real conf_big program-lowering report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}
