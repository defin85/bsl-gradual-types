#[test]
fn p49_real_conf_big_stale_parser_base_root_cause_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p49 tokio runtime");
    runtime.block_on(async {
        init_test_tracing();
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

        const PROFILE_NAME: &str = "p49_real_conf_big_stale_parser_base_root_cause_report_live";
        let _env_lock = lock_test_env().await;
        let _did_change_parse_delay_guard =
            EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "1500");
        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        let change_id = std::env::var("CHANGE_ID")
            .unwrap_or_else(|_| "refactor-32-ready-snapshot-shadow-state-lag-reduction".to_string());

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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p49 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p49_real_conf_big_live_setup")
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
        .expect("didOpen must register version 1 for p49");
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
        .expect("didOpen must materialize same-version ready parse snapshot for p49");

        let appended_procedure = "\nПроцедура Refactor22Live()\n    Возврат;\nКонецПроцедуры\n";
        let semantic_procedure =
            "\nПроцедура Refactor22Live()\n    Сообщить(НеобъявленнаяПеременная);\nКонецПроцедуры\n";
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
        .expect("same-file churn must advance shadow state to v3 while ready snapshot still lags at v1 for p49");

        let replacement_start =
            find_utf16_position_after_marker(&v3_text, "Процедура Refactor22Live()\n    ");
        let replacement_end = find_utf16_position_after_marker(
            &v3_text,
            "Процедура Refactor22Live()\n    Сообщить(НеобъявленнаяПеременная);",
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

        let observability = tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                let response =
                    live_transport_get_observability_metrics_response(&mut harness, 49_100_901)
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
        .expect("p49 must expose didChange parse-snapshot evidence for the lagging-shadow live change");

        assert_eq!(
            observability.get("parseMode").and_then(|value| value.as_str()),
            Some("incremental")
        );
        assert_eq!(
            observability.get("baseTextSource").and_then(|value| value.as_str()),
            Some("shadow_state")
        );
        assert_eq!(
            observability
                .get("baseDocumentVersion")
                .and_then(|value| value.as_i64()),
            Some(3)
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
        assert_eq!(
            observability
                .get("shadowDocumentVersion")
                .and_then(|value| value.as_i64()),
            None
        );
        assert_eq!(
            observability
                .get("latestReadyDocumentVersion")
                .and_then(|value| value.as_i64()),
            None
        );
        assert_eq!(
            observability
                .get("matchingReadySnapshotForShadowState")
                .and_then(|value| value.as_bool()),
            None
        );
        assert_eq!(
            observability
                .get("readySnapshotPrimeAttempted")
                .and_then(|value| value.as_bool()),
            None
        );

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "did_change_versions": [2, 3, 4],
            "parse_mode": observability.get("parseMode").and_then(|value| value.as_str()),
            "base_text_source": observability.get("baseTextSource").and_then(|value| value.as_str()),
            "change_shape": observability.get("changeShape").and_then(|value| value.as_str()),
            "content_changes_count": observability.get("contentChangesCount").and_then(|value| value.as_u64()),
            "replay_order": observability.get("replayOrder").and_then(|value| value.as_str()),
            "base_document_version": observability.get("baseDocumentVersion").and_then(|value| value.as_i64()),
            "changed_ranges_count": observability.get("changedRangesCount").and_then(|value| value.as_u64()),
            "fallback_reason": observability.get("fallbackReason").and_then(|value| value.as_str()),
            "parser_base_root_cause": observability.get("parserBaseRootCause").and_then(|value| value.as_str()),
            "shadow_document_version": observability.get("shadowDocumentVersion").and_then(|value| value.as_i64()),
            "latest_ready_document_version": observability.get("latestReadyDocumentVersion").and_then(|value| value.as_i64()),
            "matching_ready_snapshot_for_shadow_state": observability
                .get("matchingReadySnapshotForShadowState")
                .and_then(|value| value.as_bool()),
            "ready_snapshot_prime_attempted": observability
                .get("readySnapshotPrimeAttempted")
                .and_then(|value| value.as_bool()),
            "tree_cache_matches_shadow_text_after_prime": observability
                .get("treeCacheMatchesShadowTextAfterPrime")
                .and_then(|value| value.as_bool()),
            "v2_text_len_bytes": v2_text.len(),
            "v3_text_len_bytes": v3_text.len(),
            "v4_statement_len_bytes": v4_statement.len(),
        });
        let report_path =
            std::env::var("BSL_V2_REAL_CONF_BIG_STALE_PARSER_BASE_ROOT_CAUSE_REPORT")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("tests")
                        .join("perf")
                        .join("reports")
                        .join(format!(
                            "{change_id}-real-conf-big-stale-parser-base-root-cause-live.json"
                        ))
                });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p49 real conf_big stale-parser-base report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p49 real conf_big stale-parser-base report"),
        )
        .expect("write p49 real conf_big stale-parser-base report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}

#[test]
fn p50_real_conf_big_ready_snapshot_phase_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p50 tokio runtime");
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

        const PROFILE_NAME: &str = "p50_real_conf_big_ready_snapshot_phase_report_live";
        const DID_CHANGE_BLOCKING_PARSE_DELAY_MS: u64 = 500;
        const TIMELINE_OBSERVE_BUDGET_MS: u64 = 20_000;
        let _env_lock = lock_test_env().await;
        let _did_change_blocking_parse_delay_guard = EnvVarGuard::set(
            "BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS",
            &DID_CHANGE_BLOCKING_PARSE_DELAY_MS.to_string(),
        );

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-23-ready-snapshot-materialization-phase-attribution".to_string()
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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p50 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p50_real_conf_big_live_setup")
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
        .expect("didOpen must register version 1 for p50");
        tokio::time::timeout(Duration::from_secs(90), async {
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
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("didOpen must materialize version 1 ready snapshot for p50");

        let appended_procedure = "\nПроцедура Refactor23Live()\n    Возврат;\nКонецПроцедуры\n";
        let next_version = 2;
        live_transport_append_text_change(
            &mut harness,
            &uri,
            &module_text,
            next_version,
            appended_procedure,
        )
        .await;
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(next_version)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("didChange must register version 2 for p50");
        tokio::time::timeout(Duration::from_secs(90), async {
            loop {
                let ready_version = server
                    .latest_ready_parse_snapshots_v2
                    .read()
                    .await
                    .get(&file_id)
                    .map(|state| state.parse_snapshot.file_version);
                if ready_version == Some(next_version) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("didChange must materialize version 2 ready snapshot for p50");

        live_transport_save_document(&mut harness, &uri).await;

        let timeline_deadline = Instant::now() + Duration::from_millis(TIMELINE_OBSERVE_BUDGET_MS);
        let timeline = loop {
            let timeline =
                live_transport_get_diagnostics_save_timeline(&mut harness, 50_100_901, 12).await;
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
                            == Some(next_version as i64)
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
                    panic!("p50 must expose a diagnostics save trace for requested_version=2");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };
            if trace.get("followup_publish").is_some() {
                break trace;
            }
            if Instant::now() >= timeline_deadline {
                panic!(
                    "p50 must observe an idle_heavy follow-up publish on the exact ready path, last_trace={trace:?}"
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };

        let full_publish = timeline
            .get("followup_publish")
            .and_then(|value| value.as_object())
            .filter(|publish| publish.get("profile").and_then(|value| value.as_str()) == Some("idle_heavy"))
            .expect("idle_heavy follow-up publish trace");
        assert_eq!(
            timeline
                .get("followup_semantic_path")
                .and_then(|value| value.as_str()),
            Some("ready_artifacts")
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_timeout_phase")
                .and_then(|value| value.as_str()),
            None,
            "p50 ready path must not fabricate timeout phase attribution, trace={timeline:?}"
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_dominant_phase")
                .and_then(|value| value.as_str()),
            Some("parse_exec"),
            "blocking parse delay should keep parse_exec dominant on the real conf_big exact path, trace={timeline:?}"
        );
        assert!(
            timeline
                .get("followup_ready_snapshot_parse_exec_ms")
                .and_then(|value| value.as_u64())
                .is_some_and(|value| value >= DID_CHANGE_BLOCKING_PARSE_DELAY_MS),
            "p50 must export parse_exec timing for the real exact path, trace={timeline:?}"
        );

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "did_change_blocking_parse_delay_ms": DID_CHANGE_BLOCKING_PARSE_DELAY_MS,
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
            "followup_ready_snapshot_parse_exec_ms": timeline
                .get("followup_ready_snapshot_parse_exec_ms")
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
            "followup_semantic_path": timeline
                .get("followup_semantic_path")
                .and_then(|value| value.as_str()),
            "followup_publish_elapsed_ms": full_publish
                .get("elapsed_ms")
                .and_then(|value| value.as_u64()),
        });
        let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_READY_SNAPSHOT_PHASE_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!(
                        "{change_id}-real-conf-big-ready-snapshot-phase-live.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p50 real conf_big phase report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p50 real conf_big phase report"),
        )
        .expect("write p50 real conf_big phase report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}

#[test]
fn p51_real_conf_test_ready_snapshot_relief_valve_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p51 tokio runtime");
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

        const PROFILE_NAME: &str = "p51_real_conf_test_ready_snapshot_relief_valve_report_live";
        const DID_CHANGE_BLOCKING_PARSE_DELAY_MS: u64 = 3_800;
        const TIMELINE_OBSERVE_BUDGET_MS: u64 = 30_000;
        let _env_lock = lock_test_env().await;
        let _did_change_blocking_parse_delay_guard = EnvVarGuard::set(
            "BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS",
            &DID_CHANGE_BLOCKING_PARSE_DELAY_MS.to_string(),
        );
        let _debounce_guard =
            EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);

        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-24-diagnostics-save-followup-budget-valve".to_string()
        });

        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .to_path_buf();
        let conf_test_root = workspace_root.join("examples").join("conf").join("conf_test");
        assert!(
            conf_test_root.join("Configuration.xml").exists(),
            "conf_test fixture is missing: {}",
            conf_test_root.display()
        );
        let module_path = conf_test_root
            .join("Documents")
            .join("ЗаказНаряды")
            .join("Forms")
            .join("ФормаДокумента")
            .join("Ext")
            .join("Form")
            .join("Module.bsl");
        if !module_path.exists() {
            panic!(
                "conf_test module fixture is missing: {}",
                module_path.display()
            );
        }

        let module_text =
            std::fs::read_to_string(&module_path).expect("read conf_test module text for p51 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_test_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(
            &server,
            &workspace_setup,
            "p51_real_conf_test_live_setup",
        )
        .await;

        let uri = Url::from_file_path(&module_path).expect("real conf_test module uri");
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
        .expect("didOpen must register version 1 for p51");
        tokio::time::timeout(Duration::from_secs(90), async {
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
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("didOpen must materialize version 1 ready snapshot for p51");

        let baseline_metrics =
            live_transport_get_observability_metrics(&mut harness, 51_100_900).await;
        let baseline_counters = baseline_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("baseline metrics.counters object");

        let semantic_suffix =
            "\nПроцедура Refactor24Live()\n    Возврат;\nКонецПроцедуры\n";
        let next_version = 2;
        live_transport_append_text_change(
            &mut harness,
            &uri,
            &module_text,
            next_version,
            semantic_suffix,
        )
        .await;
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let task_state = {
                    let tasks = server.background_parse_snapshot_apply_tasks_v2.lock().await;
                    tasks.get(&file_id).map(|task| {
                        let target = task
                            .target
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        (
                            target.requested_version,
                            task.control.phase.load(std::sync::atomic::Ordering::SeqCst),
                        )
                    })
                };
                if server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(next_version)
                    && matches!(
                        task_state,
                        Some((
                            version,
                            phase,
                        )) if version == next_version
                            && phase
                                == crate::server::BackgroundParseSnapshotApplyTaskPhaseV2::Parsing
                                    as u8
                    )
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("didChange worker must enter parse_exec before p51 save");

        let did_save_started = Instant::now();
        live_transport_save_document(&mut harness, &uri).await;

        let timeline_deadline = Instant::now() + Duration::from_millis(TIMELINE_OBSERVE_BUDGET_MS);
        let timeline = loop {
            let timeline =
                live_transport_get_diagnostics_save_timeline(&mut harness, 51_100_901, 12).await;
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
                            == Some(next_version as i64)
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
                    panic!("p51 must expose a diagnostics save trace for requested_version=2");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };
            let relief_outcome = trace
                .get("followup_ready_snapshot_relief_valve_outcome")
                .and_then(|value| value.as_str());
            let followup_publish = trace
                .get("followup_publish")
                .and_then(|value| value.as_object());
            if relief_outcome == Some("engaged_helped") && followup_publish.is_some() {
                break trace;
            }
            if Instant::now() >= timeline_deadline {
                panic!(
                    "p51 must observe relief-valve help on the exact save path, last_trace={trace:?}"
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };

        let full_publish = timeline
            .get("followup_publish")
            .and_then(|value| value.as_object())
            .filter(|publish| {
                publish.get("profile").and_then(|value| value.as_str()) == Some("idle_heavy")
            })
            .expect("idle_heavy follow-up publish trace");
        let relief_budget_ms =
            diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET
                .as_millis()
                .min(u64::MAX as u128) as u64;
        let did_save_elapsed_ms = did_save_started.elapsed().as_millis().min(u64::MAX as u128) as u64;

        assert_eq!(
            full_publish
                .get("semantic_path")
                .and_then(|value| value.as_str()),
            Some("ready_artifacts")
        );
        assert_eq!(
            timeline
                .get("followup_semantic_path")
                .and_then(|value| value.as_str()),
            Some("ready_artifacts")
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_zero_probe")
                .and_then(|value| value.as_str()),
            Some("not_ready")
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_wait_probe")
                .and_then(|value| value.as_str()),
            Some("timeout")
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_timeout_phase")
                .and_then(|value| value.as_str()),
            Some("parse_exec")
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_timeout_leaf")
                .and_then(|value| value.as_str()),
            Some("parser_tree_build")
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint")
                .and_then(|value| value.as_str()),
            Some("parser_tree_build")
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_task_state")
                .and_then(|value| value.as_str()),
            Some("in_flight_same_version")
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_relief_valve_outcome")
                .and_then(|value| value.as_str()),
            Some("engaged_helped")
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_relief_valve_budget_ms")
                .and_then(|value| value.as_u64()),
            Some(relief_budget_ms)
        );
        assert!(
            timeline
                .get("followup_ready_snapshot_relief_valve_elapsed_ms")
                .and_then(|value| value.as_u64())
                .is_some_and(|value| value > 0 && value <= relief_budget_ms),
            "p51 must expose spent relief wait within bounded budget, trace={timeline:?}"
        );
        assert!(
            timeline
                .get("followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms")
                .and_then(|value| value.as_u64())
                .is_some_and(|value| value > 0),
            "p51 must expose non-zero parser_tree_build timing on the live timeout path, trace={timeline:?}"
        );
        assert_eq!(
            timeline
                .get("followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint")
                .and_then(|value| value.as_str()),
            Some("parser_tree_build"),
            "p51 live timeout path must keep the dominant core-build checkpoint on parser_tree_build, trace={timeline:?}"
        );
        assert!(
            timeline
                .get("followup_apply_lag_ms")
                .and_then(|value| value.as_u64())
                .is_none(),
            "relief-valve live path must stay off the apply-lag fallback, trace={timeline:?}"
        );
        assert!(
            timeline
                .get("followup_runtime_queue_wait_ms")
                .and_then(|value| value.as_u64())
                .is_none(),
            "relief-valve live path must stay off the runtime-queue fallback, trace={timeline:?}"
        );

        let final_metrics = live_transport_get_observability_metrics(&mut harness, 51_100_902).await;
        let final_counters = final_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("final metrics.counters object");

        let bounded_wait_timeout_key =
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_timeout";
        let relief_probe_ready_key =
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_ready";
        let relief_helped_key =
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_helped";
        let ready_path_key =
            "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_ready_artifacts";

        let bounded_wait_timeout_delta = read_u64_metric(final_counters.get(bounded_wait_timeout_key))
            .saturating_sub(read_u64_metric(baseline_counters.get(bounded_wait_timeout_key)));
        let relief_probe_ready_delta = read_u64_metric(final_counters.get(relief_probe_ready_key))
            .saturating_sub(read_u64_metric(baseline_counters.get(relief_probe_ready_key)));
        let relief_helped_delta = read_u64_metric(final_counters.get(relief_helped_key))
            .saturating_sub(read_u64_metric(baseline_counters.get(relief_helped_key)));
        let ready_path_delta = read_u64_metric(final_counters.get(ready_path_key))
            .saturating_sub(read_u64_metric(baseline_counters.get(ready_path_key)));

        assert!(
            bounded_wait_timeout_delta > 0,
            "p51 must record that the base exact wait already timed out, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert!(
            relief_probe_ready_delta > 0,
            "p51 must record a relief-valve ready probe after the base timeout, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert!(
            relief_helped_delta > 0,
            "p51 must record explicit engaged_helped relief attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert!(
            ready_path_delta > 0,
            "p51 must still publish through ready_artifacts after relief succeeds, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "did_change_blocking_parse_delay_ms": DID_CHANGE_BLOCKING_PARSE_DELAY_MS,
            "did_save_elapsed_ms": did_save_elapsed_ms,
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
            "followup_ready_snapshot_timeout_leaf": timeline
                .get("followup_ready_snapshot_timeout_leaf")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint": timeline
                .get("followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_relief_valve_outcome": timeline
                .get("followup_ready_snapshot_relief_valve_outcome")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_relief_valve_budget_ms": timeline
                .get("followup_ready_snapshot_relief_valve_budget_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_relief_valve_elapsed_ms": timeline
                .get("followup_ready_snapshot_relief_valve_elapsed_ms")
                .and_then(|value| value.as_u64()),
            "followup_semantic_path": timeline
                .get("followup_semantic_path")
                .and_then(|value| value.as_str()),
            "followup_publish_elapsed_ms": full_publish
                .get("elapsed_ms")
                .and_then(|value| value.as_u64()),
            "bounded_wait_timeout_delta": bounded_wait_timeout_delta,
            "relief_probe_ready_delta": relief_probe_ready_delta,
            "relief_helped_delta": relief_helped_delta,
            "ready_path_delta": ready_path_delta,
        });
        let report_path = std::env::var("BSL_V2_REAL_CONF_TEST_READY_SNAPSHOT_RELIEF_VALVE_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!(
                        "{change_id}-real-conf-test-ready-snapshot-relief-valve-live.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p51 real conf_test relief-valve report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p51 real conf_test relief-valve report"),
        )
        .expect("write p51 real conf_test relief-valve report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}
