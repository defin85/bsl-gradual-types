#[test]
fn p43_real_conf_big_did_save_diagnostics_fastlane_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p43 tokio runtime");
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

        let _env_lock = lock_test_env().await;
        const PROFILE_NAME: &str = "p43_real_conf_big_did_save_diagnostics_fastlane_report_live";
        const APPLY_DELAY_MS: u64 = 0;
        const FIRST_PUBLISH_BUDGET_MS: u64 = 2_500;
        let _apply_delay_guard =
            EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", &APPLY_DELAY_MS.to_string());
        let _debounce_guard = EnvVarGuard::set_with_reload(
            "BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS",
            "1200",
            true,
        );

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        let change_id = std::env::var("CHANGE_ID")
            .unwrap_or_else(|_| "refactor-03-diagnostics-save-freshness-fastlane".to_string());

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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p43 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p43_real_conf_big_live_setup")
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
        .expect("didOpen must register version 1 for p43");

        let broken_suffix = "\nПроцедура SaveFastlaneBroken(\n";
        let next_version = 2;
        live_transport_append_text_change(
            &mut harness,
            &uri,
            &module_text,
            next_version,
            broken_suffix,
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
        .expect("didChange must register version 2 for p43");

        let did_save_started = Instant::now();
        live_transport_save_document(&mut harness, &uri).await;
        let first_publish = live_transport_wait_publish_diagnostics(
            &mut harness,
            &uri,
            next_version,
            Duration::from_millis(FIRST_PUBLISH_BUDGET_MS),
        )
        .await;
        let first_publish_elapsed_ms =
            did_save_started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let syntax_only = first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax"));

        assert!(
            !first_publish.diagnostics.is_empty(),
            "save fastlane live report must observe non-empty diagnostics on broken conf_big revision"
        );
        assert!(
            syntax_only,
            "save fastlane live report must keep first publish syntax-only, diagnostics={:?}",
            first_publish.diagnostics
        );
        assert!(
            first_publish_elapsed_ms <= FIRST_PUBLISH_BUDGET_MS,
            "save fastlane live report exceeded bounded first-publish budget: first_publish_elapsed_ms={}ms > {}ms",
            first_publish_elapsed_ms,
            FIRST_PUBLISH_BUDGET_MS
        );
        let observability_metrics =
            live_transport_get_observability_metrics(&mut harness, 43_100_901).await;
        let counters = observability_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let histograms = observability_metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");
        let save_fastlane_published_total = read_u64_metric(
            counters.get(
                "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_did_save_profile_save_fastlane_reason_published",
            ),
        );
        assert!(
            save_fastlane_published_total > 0,
            "save fastlane live report must export dedicated published counter, counters={counters:?}"
        );
        assert!(
            histograms.contains_key(
                "intellisense_v2_diagnostics_pipeline_publish_ms_origin_lsp_trigger_did_save_profile_save_fastlane"
            ),
            "save fastlane live report must export publish latency histogram, histograms={:?}",
            histograms.keys().collect::<Vec<_>>()
        );

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "apply_delay_ms": APPLY_DELAY_MS,
            "first_publish_budget_ms": FIRST_PUBLISH_BUDGET_MS,
            "first_publish_elapsed_ms": first_publish_elapsed_ms,
            "first_publish_version": first_publish.version,
            "first_publish_diagnostics_count": first_publish.diagnostics.len(),
            "first_publish_syntax_only": syntax_only,
            "save_fastlane_published_total": save_fastlane_published_total,
        });
        let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_DID_SAVE_DIAGNOSTICS_FASTLANE_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!(
                        "{change_id}-real-conf-big-did-save-diagnostics-fastlane-live.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p43 real conf_big diagnostics report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p43 real conf_big diagnostics report"),
        )
        .expect("write p43 real conf_big diagnostics report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}

#[test]
fn p46_real_conf_big_did_save_diagnostics_followup_runtime_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p46 tokio runtime");
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

        let _env_lock = lock_test_env().await;
        const PROFILE_NAME: &str =
            "p46_real_conf_big_did_save_diagnostics_followup_runtime_report_live";
        const APPLY_DELAY_MS: u64 = 4_000;
        const FIRST_PUBLISH_BUDGET_MS: u64 = 2_500;
        const FOLLOWUP_OBSERVE_BUDGET_MS: u64 = 60_000;
        const DOCUMENT_SYMBOL_BURST_REQUESTS: usize = 6;
        const FOLLOWUP_RUNTIME_QUEUE_WAIT_BUDGET_MS: u64 = 5_000;
        let _apply_delay_guard =
            EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", &APPLY_DELAY_MS.to_string());
        let _debounce_guard = EnvVarGuard::set_with_reload(
            "BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS",
            "1200",
            true,
        );

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-10-diagnostics-save-followup-background-isolation".to_string()
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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p46 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p46_real_conf_big_live_setup")
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
        .expect("didOpen must register version 1 for p46");

        let broken_suffix = "\nПроцедура SaveFollowupBroken(\n";
        let next_version = 2;
        live_transport_append_text_change(
            &mut harness,
            &uri,
            &module_text,
            next_version,
            broken_suffix,
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
        .expect("didChange must register version 2 for p46");

        let did_save_started = Instant::now();
        live_transport_save_document(&mut harness, &uri).await;
        let first_publish = live_transport_wait_publish_diagnostics(
            &mut harness,
            &uri,
            next_version,
            Duration::from_millis(FIRST_PUBLISH_BUDGET_MS),
        )
        .await;
        let first_publish_elapsed_ms =
            did_save_started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let syntax_only = first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax"));

        assert!(
            !first_publish.diagnostics.is_empty(),
            "follow-up live report must observe non-empty diagnostics on broken conf_big revision"
        );
        assert!(
            syntax_only,
            "follow-up live report must keep first publish syntax-only, diagnostics={:?}",
            first_publish.diagnostics
        );
        assert!(
            first_publish_elapsed_ms <= FIRST_PUBLISH_BUDGET_MS,
            "follow-up live report exceeded bounded first-publish budget: first_publish_elapsed_ms={}ms > {}ms",
            first_publish_elapsed_ms,
            FIRST_PUBLISH_BUDGET_MS
        );

        let mut document_symbol_present_responses_total = 0_u64;
        let mut document_symbol_null_responses_total = 0_u64;
        for request_offset in 0..DOCUMENT_SYMBOL_BURST_REQUESTS {
            let response = tokio::time::timeout(
                Duration::from_secs(15),
                harness.send_request(
                    44_100_910 + request_offset as i64,
                    "textDocument/documentSymbol",
                    DocumentSymbolParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    },
                ),
            )
            .await
            .expect("documentSymbol mixed-load request must stay bounded");
            if document_symbol_response_from_jsonrpc_response(&response).is_some() {
                document_symbol_present_responses_total += 1;
            } else {
                document_symbol_null_responses_total += 1;
            }
        }

        let timeline_deadline = Instant::now() + Duration::from_millis(FOLLOWUP_OBSERVE_BUDGET_MS);
        let timeline = loop {
            let timeline = live_transport_get_diagnostics_save_timeline(
                &mut harness,
                44_100_901,
                12,
            )
            .await;
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
                    panic!("follow-up live report must expose publish or explicit residual attribution on the observed didSave cycle");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };
            let followup_publish_present = trace
                .get("followup_publish")
                .and_then(|value| value.as_object())
                .is_some();
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
            let idle_heavy_outcome = trace
                .get("idle_heavy_outcome")
                .and_then(|value| value.as_str());
            if followup_publish_present
                || followup_wait_reason.is_some_and(|reason| reason != "pending_publish")
                || followup_runtime_queue_wait_present
                || followup_apply_lag_present
                || matches!(idle_heavy_outcome, Some("superseded_generation"))
            {
                break trace;
            }
            if Instant::now() >= timeline_deadline {
                panic!(
                    "follow-up live report must expose publish or explicit residual attribution on the observed didSave cycle, last_trace={trace:?}"
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };

        let followup_publish = timeline.get("followup_publish").and_then(|value| value.as_object());
        let followup_wait_reason = timeline
            .get("followup_wait_reason")
            .and_then(|value| value.as_str());
        let idle_heavy_outcome = timeline
            .get("idle_heavy_outcome")
            .and_then(|value| value.as_str());
        let observed_followup_runtime_queue_wait_ms = followup_publish
            .and_then(|publish| publish.get("runtime_queue_wait_ms"))
            .and_then(|value| value.as_u64())
            .or_else(|| {
                timeline
                    .get("followup_runtime_queue_wait_ms")
                    .and_then(|value| value.as_u64())
            });
        assert!(
            followup_publish.is_some()
                || followup_wait_reason.is_some_and(|reason| reason != "pending_publish")
                || timeline
                    .get("followup_runtime_queue_wait_ms")
                    .and_then(|value| value.as_u64())
                    .is_some()
                || timeline
                    .get("followup_apply_lag_ms")
                    .and_then(|value| value.as_u64())
                    .is_some()
                || matches!(idle_heavy_outcome, Some("superseded_generation")),
            "follow-up runtime live report must expose publish or explicit runtime/apply attribution, trace={timeline:?}"
        );
        assert_ne!(
            timeline
                .get("followup_runtime_queue_wait_ms")
                .and_then(|value| value.as_u64()),
            Some(0),
            "follow-up runtime live report must omit zero-valued top-level runtime queue wait noise, trace={timeline:?}"
        );
        assert_ne!(
            timeline
                .get("followup_apply_lag_ms")
                .and_then(|value| value.as_u64()),
            Some(0),
            "follow-up runtime live report must omit zero-valued top-level apply lag noise, trace={timeline:?}"
        );
        assert_ne!(
            followup_publish
                .and_then(|publish| publish.get("runtime_queue_wait_ms"))
                .and_then(|value| value.as_u64()),
            Some(0),
            "follow-up runtime live report must omit zero-valued publish runtime queue wait noise, trace={timeline:?}"
        );
        assert_ne!(
            followup_publish
                .and_then(|publish| publish.get("apply_lag_ms"))
                .and_then(|value| value.as_u64()),
            Some(0),
            "follow-up runtime live report must omit zero-valued publish apply lag noise, trace={timeline:?}"
        );
        assert!(
            observed_followup_runtime_queue_wait_ms
                .is_none_or(|value| value <= FOLLOWUP_RUNTIME_QUEUE_WAIT_BUDGET_MS),
            "follow-up runtime live report must keep runtime_queue_wait bounded under comparable mixed documentSymbol load: observed_followup_runtime_queue_wait_ms={observed_followup_runtime_queue_wait_ms:?} > {}ms, trace={timeline:?}",
            FOLLOWUP_RUNTIME_QUEUE_WAIT_BUDGET_MS
        );

        let observability_metrics =
            live_transport_get_observability_metrics(&mut harness, 44_100_902).await;
        let counters = observability_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let save_fastlane_published_total = read_u64_metric(
            counters.get(
                "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_did_save_profile_save_fastlane_reason_published",
            ),
        );
        assert!(
            save_fastlane_published_total > 0,
            "follow-up live report must still observe save_fastlane published counter, counters={counters:?}"
        );

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "request_plan": {
                "mixed_load_profile": "didSave+documentSymbol_burst",
                "document_symbol_requests_total": DOCUMENT_SYMBOL_BURST_REQUESTS,
            },
            "apply_delay_ms": APPLY_DELAY_MS,
            "first_publish_budget_ms": FIRST_PUBLISH_BUDGET_MS,
            "first_publish_elapsed_ms": first_publish_elapsed_ms,
            "first_publish_version": first_publish.version,
            "first_publish_diagnostics_count": first_publish.diagnostics.len(),
            "first_publish_syntax_only": syntax_only,
            "save_fastlane_published_total": save_fastlane_published_total,
            "document_symbol_present_responses_total": document_symbol_present_responses_total,
            "document_symbol_null_responses_total": document_symbol_null_responses_total,
            "followup_runtime_queue_wait_budget_ms": FOLLOWUP_RUNTIME_QUEUE_WAIT_BUDGET_MS,
            "observed_followup_runtime_queue_wait_ms": observed_followup_runtime_queue_wait_ms,
            "save_cycle_sequence": timeline
                .get("save_cycle_sequence")
                .and_then(|value| value.as_u64()),
            "diagnostics_generation": timeline
                .get("diagnostics_generation")
                .and_then(|value| value.as_u64()),
            "followup_publish_profile": followup_publish
                .and_then(|publish| publish.get("profile"))
                .and_then(|value| value.as_str()),
            "followup_publish_kind": followup_publish
                .and_then(|publish| publish.get("publish_kind"))
                .and_then(|value| value.as_str()),
            "followup_publish_elapsed_ms": followup_publish
                .and_then(|publish| publish.get("elapsed_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_runtime_queue_wait_ms": followup_publish
                .and_then(|publish| publish.get("runtime_queue_wait_ms"))
                .and_then(|value| value.as_u64()),
            "followup_publish_apply_lag_ms": followup_publish
                .and_then(|publish| publish.get("apply_lag_ms"))
                .and_then(|value| value.as_u64()),
            "followup_wait_reason": followup_wait_reason,
            "followup_runtime_queue_wait_ms": timeline
                .get("followup_runtime_queue_wait_ms")
                .and_then(|value| value.as_u64()),
            "followup_apply_lag_ms": timeline
                .get("followup_apply_lag_ms")
                .and_then(|value| value.as_u64()),
            "followup_ready_snapshot_zero_probe": timeline
                .get("followup_ready_snapshot_zero_probe")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_wait_probe": timeline
                .get("followup_ready_snapshot_wait_probe")
                .and_then(|value| value.as_str()),
            "followup_ready_snapshot_task_state": timeline
                .get("followup_ready_snapshot_task_state")
                .and_then(|value| value.as_str()),
            "followup_shadow_state_available": timeline
                .get("followup_shadow_state_available")
                .and_then(|value| value.as_bool()),
            "followup_semantic_path": timeline
                .get("followup_semantic_path")
                .and_then(|value| value.as_str()),
            "followup_semantic_parse_source": timeline
                .get("followup_semantic_parse_source")
                .and_then(|value| value.as_str()),
            "followup_wait_for_file_version_ms": timeline
                .get("followup_wait_for_file_version_ms")
                .and_then(|value| value.as_u64()),
            "followup_snapshot_with_deps_ms": timeline
                .get("followup_snapshot_with_deps_ms")
                .and_then(|value| value.as_u64()),
            "idle_heavy_outcome": idle_heavy_outcome,
            "terminal_outcome": timeline
                .get("terminal_outcome")
                .and_then(|value| value.as_str()),
        });
        let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_DID_SAVE_DIAGNOSTICS_FOLLOWUP_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!(
                        "{change_id}-real-conf-big-did-save-diagnostics-followup-runtime-live.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p46 real conf_big diagnostics report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p46 real conf_big diagnostics report"),
        )
        .expect("write p46 real conf_big diagnostics report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}

#[test]
fn p45_real_conf_big_did_save_diagnostics_followup_syntax_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p45 tokio runtime");
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

        let _env_lock = lock_test_env().await;
        const PROFILE_NAME: &str =
            "p45_real_conf_big_did_save_diagnostics_followup_syntax_report_live";
        const APPLY_DELAY_MS: u64 = 0;
        const FIRST_PUBLISH_OBSERVE_BUDGET_MS: u64 = 60_000;
        const FOLLOWUP_PUBLISH_BUDGET_MS: u64 = 60_000;
        let _apply_delay_guard =
            EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", &APPLY_DELAY_MS.to_string());
        let _debounce_guard = EnvVarGuard::set_with_reload(
            "BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS",
            "1200",
            true,
        );

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-14-diagnostics-save-followup-semantic-snapshot-reuse".to_string()
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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p45 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p45_real_conf_big_live_setup")
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
        .expect("didOpen must register version 1 for p45");

        let broken_suffix = "\nПроцедура SaveFollowupSyntaxBroken(\n";
        let next_version = 2;
        live_transport_append_text_change(
            &mut harness,
            &uri,
            &module_text,
            next_version,
            broken_suffix,
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
        .expect("didChange must register version 2 for p45");
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
                    .is_some_and(|state| state.parse_snapshot.file_version == next_version)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("didChange must materialize same-version ready parse snapshot for p45");

        live_transport_save_document(&mut harness, &uri).await;
        let first_publish = live_transport_wait_publish_diagnostics(
            &mut harness,
            &uri,
            next_version,
            Duration::from_millis(FIRST_PUBLISH_OBSERVE_BUDGET_MS),
        )
        .await;
        let first_publish_syntax_only = first_publish
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source.as_deref() == Some("bsl-syntax"));
        assert!(
            !first_publish.diagnostics.is_empty(),
            "p45 must observe non-empty save_fastlane diagnostics on broken conf_big revision"
        );
        assert!(
            first_publish_syntax_only,
            "p45 first publish must stay syntax-only, diagnostics={:?}",
            first_publish.diagnostics
        );
        let timeline_deadline = Instant::now() + Duration::from_millis(FOLLOWUP_PUBLISH_BUDGET_MS);
        let timeline = loop {
            let timeline = live_transport_get_diagnostics_save_timeline(
                &mut harness,
                45_100_901,
                12,
            )
            .await;
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
                    panic!("p45 must observe diagnostics save trace on conf_big");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };
            let first_publish = trace
                .get("first_publish")
                .and_then(|value| value.as_object())
                .filter(|publish| {
                    publish.get("profile").and_then(|value| value.as_str())
                        == Some("save_fastlane")
                });
            let followup_publish = trace
                .get("followup_publish")
                .and_then(|value| value.as_object())
                .filter(|publish| {
                    publish.get("profile").and_then(|value| value.as_str())
                        == Some("idle_heavy")
                });
            let followup_wait_reason = trace
                .get("followup_wait_reason")
                .and_then(|value| value.as_str());
            let followup_syntax_work_mode = trace
                .get("followup_syntax_work_mode")
                .and_then(|value| value.as_str());
            let followup_semantic_path = trace
                .get("followup_semantic_path")
                .and_then(|value| value.as_str());
            let explicit_reuse_proof_in_flight = followup_wait_reason == Some("semantic_work")
                && followup_syntax_work_mode == Some("reused")
                && followup_semantic_path == Some("ready_artifacts");
            if first_publish.is_some()
                && (followup_publish.is_some() || explicit_reuse_proof_in_flight)
            {
                break trace;
            }
            if Instant::now() >= timeline_deadline {
                panic!(
                    "p45 must observe idle_heavy syntax reuse proof on conf_big, last_trace={trace:?}"
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };

        let timeline_first_publish = timeline
            .get("first_publish")
            .and_then(|value| value.as_object())
            .expect("p45 save_fastlane first publish trace");
        assert_eq!(
            timeline_first_publish
                .get("profile")
                .and_then(|value| value.as_str()),
            Some("save_fastlane")
        );
        assert_eq!(
            timeline_first_publish
                .get("publish_kind")
                .and_then(|value| value.as_str()),
            Some("syntax_only")
        );
        assert_eq!(
            timeline_first_publish
                .get("outcome")
                .and_then(|value| value.as_str()),
            Some("published")
        );
        let first_publish_elapsed_ms = timeline_first_publish
            .get("elapsed_ms")
            .and_then(|value| value.as_u64())
            .expect("p45 save_fastlane first publish elapsed_ms");
        let followup_syntax_work_mode = timeline
            .get("followup_syntax_work_mode")
            .and_then(|value| value.as_str());
        let followup_semantic_path = timeline
            .get("followup_semantic_path")
            .and_then(|value| value.as_str());
        assert_eq!(
            followup_syntax_work_mode,
            Some("reused"),
            "p45 must expose actual syntax reuse for idle_heavy follow-up, trace={timeline:?}"
        );
        assert_eq!(
            followup_semantic_path,
            Some("ready_artifacts"),
            "p45 must expose ready_artifacts semantic path for same-version snapshot-backed follow-up, trace={timeline:?}"
        );
        let followup_wait_reason = timeline
            .get("followup_wait_reason")
            .and_then(|value| value.as_str());

        let followup_publish = timeline
            .get("followup_publish")
            .and_then(|value| value.as_object())
            .filter(|publish| {
                publish.get("profile").and_then(|value| value.as_str()) == Some("idle_heavy")
            });
        if let Some(followup_publish) = followup_publish {
            assert_eq!(
                followup_publish
                    .get("publish_kind")
                    .and_then(|value| value.as_str()),
                Some("full")
            );
            assert_eq!(
                followup_publish
                    .get("outcome")
                    .and_then(|value| value.as_str()),
                Some("published")
            );
            assert_eq!(
                followup_publish
                    .get("syntax_work_mode")
                    .and_then(|value| value.as_str()),
                Some("reused"),
                "p45 follow-up publish must report syntax reuse, trace={timeline:?}"
            );
            assert_eq!(
                followup_publish
                    .get("semantic_path")
                    .and_then(|value| value.as_str()),
                Some("ready_artifacts"),
                "p45 follow-up publish must report ready_artifacts semantic path, trace={timeline:?}"
            );
            assert_eq!(
                followup_publish
                    .get("semantic_parse_source")
                    .and_then(|value| value.as_str()),
                Some("snapshot"),
                "p45 follow-up publish must report snapshot parse source, trace={timeline:?}"
            );
            assert!(
                followup_publish
                    .get("semantic_ir_source")
                    .and_then(|value| value.as_str())
                    .is_none_or(|value| matches!(value, "snapshot_build" | "exact_cache")),
                "p45 follow-up publish must either report snapshot-backed IR source or omit it when semantic IR short-circuits on syntax errors, trace={timeline:?}"
            );
            assert!(
                followup_publish.get("syntax_diagnostics_query_ms").is_none(),
                "p45 follow-up publish must not expose recomputed syntax query timing when syntax artifacts are reused, trace={timeline:?}"
            );
        } else {
            assert_eq!(
                followup_wait_reason,
                Some("semantic_work"),
                "p45 in-flight syntax reuse proof must already be on the semantic follow-up path, trace={timeline:?}"
            );
        }

        let observability_metrics =
            live_transport_get_observability_metrics(&mut harness, 45_100_902).await;
        let counters = observability_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let save_fastlane_published_total = read_u64_metric(
            counters.get(
                "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_did_save_profile_save_fastlane_reason_published",
            ),
        );
        assert!(
            save_fastlane_published_total > 0,
            "p45 must observe save_fastlane published counter, counters={counters:?}"
        );

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "apply_delay_ms": APPLY_DELAY_MS,
            "first_publish_observe_budget_ms": FIRST_PUBLISH_OBSERVE_BUDGET_MS,
            "followup_publish_budget_ms": FOLLOWUP_PUBLISH_BUDGET_MS,
            "first_publish_elapsed_ms": first_publish_elapsed_ms,
            "first_publish_version": next_version,
            "first_publish_diagnostics_count": first_publish.diagnostics.len(),
            "first_publish_syntax_only": first_publish_syntax_only,
            "save_fastlane_published_total": save_fastlane_published_total,
            "save_cycle_sequence": timeline
                .get("save_cycle_sequence")
                .and_then(|value| value.as_u64()),
            "diagnostics_generation": timeline
                .get("diagnostics_generation")
                .and_then(|value| value.as_u64()),
            "followup_publish_profile": followup_publish
                .and_then(|publish| publish.get("profile"))
                .and_then(|value| value.as_str()),
            "followup_publish_kind": followup_publish
                .and_then(|publish| publish.get("publish_kind"))
                .and_then(|value| value.as_str()),
            "followup_publish_elapsed_ms": followup_publish
                .and_then(|publish| publish.get("elapsed_ms"))
                .and_then(|value| value.as_u64()),
            "followup_syntax_work_mode": followup_publish
                .and_then(|publish| publish.get("syntax_work_mode"))
                .and_then(|value| value.as_str())
                .or(followup_syntax_work_mode),
            "followup_semantic_path": followup_publish
                .and_then(|publish| publish.get("semantic_path"))
                .and_then(|value| value.as_str())
                .or(followup_semantic_path),
            "followup_semantic_parse_source": followup_publish
                .and_then(|publish| publish.get("semantic_parse_source"))
                .and_then(|value| value.as_str())
                .or_else(|| {
                    timeline
                        .get("followup_semantic_parse_source")
                        .and_then(|value| value.as_str())
                }),
            "followup_semantic_ir_source": followup_publish
                .and_then(|publish| publish.get("semantic_ir_source"))
                .and_then(|value| value.as_str())
                .or_else(|| {
                    timeline
                        .get("followup_semantic_ir_source")
                        .and_then(|value| value.as_str())
                }),
            "followup_syntax_diagnostics_query_ms": followup_publish
                .and_then(|publish| publish.get("syntax_diagnostics_query_ms"))
                .and_then(|value| value.as_u64()),
            "followup_semantic_diagnostics_query_ms": followup_publish
                .and_then(|publish| publish.get("semantic_diagnostics_query_ms"))
                .and_then(|value| value.as_u64()),
            "followup_wait_for_file_version_ms": followup_publish
                .and_then(|publish| publish.get("wait_for_file_version_ms"))
                .and_then(|value| value.as_u64())
                .or_else(|| {
                    timeline
                        .get("followup_wait_for_file_version_ms")
                        .and_then(|value| value.as_u64())
                }),
            "followup_snapshot_with_deps_ms": followup_publish
                .and_then(|publish| publish.get("snapshot_with_deps_ms"))
                .and_then(|value| value.as_u64())
                .or_else(|| {
                    timeline
                        .get("followup_snapshot_with_deps_ms")
                        .and_then(|value| value.as_u64())
                }),
            "followup_wait_reason": timeline
                .get("followup_wait_reason")
                .and_then(|value| value.as_str()),
            "idle_heavy_outcome": timeline
                .get("idle_heavy_outcome")
                .and_then(|value| value.as_str()),
            "terminal_outcome": timeline
                .get("terminal_outcome")
                .and_then(|value| value.as_str()),
        });
        let report_path = std::env::var(
            "BSL_V2_REAL_CONF_BIG_DID_SAVE_DIAGNOSTICS_FOLLOWUP_SYNTAX_REPORT",
        )
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!(
                    "{change_id}-real-conf-big-did-save-diagnostics-followup-syntax-live.json"
                ))
        });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p45 real conf_big diagnostics report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p45 real conf_big diagnostics report"),
        )
        .expect("write p45 real conf_big diagnostics report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}

#[test]
fn p47_real_conf_big_sequential_ranged_did_change_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p47 tokio runtime");
    runtime.block_on(async {
        init_test_tracing();

        const PROFILE_NAME: &str = "p47_real_conf_big_sequential_ranged_did_change_report_live";
        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        let change_id = std::env::var("CHANGE_ID")
            .unwrap_or_else(|_| "refactor-19-did-change-sequential-replay-order".to_string());

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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p47 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p47_real_conf_big_live_setup")
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
        .expect("didOpen must register version 1 for p47");
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
        .expect("didOpen must materialize same-version ready parse snapshot for p47");

        let first_insert = "\n// refactor-19 live sequential replay\nПроцедура Refactor19Live()\n";
        let second_insert = "КонецПроцедуры\n";
        let first_end_position = utf16_end_position(&module_text);
        let second_end_position = utf16_end_position(&(module_text.clone() + first_insert));
        let next_version = 2;

        live_transport_ranged_did_change(
            &mut harness,
            &uri,
            next_version,
            vec![
                TextDocumentContentChangeEvent {
                    range: Some(Range {
                        start: first_end_position,
                        end: first_end_position,
                    }),
                    range_length: None,
                    text: first_insert.to_string(),
                },
                TextDocumentContentChangeEvent {
                    range: Some(Range {
                        start: second_end_position,
                        end: second_end_position,
                    }),
                    range_length: None,
                    text: second_insert.to_string(),
                },
            ],
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
        .expect("didChange must register version 2 for p47");

        let observability = tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                let response =
                    live_transport_get_observability_metrics_response(&mut harness, 47_100_901)
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
                                    == Some(next_version as i64)
                        })
                    })
                    .cloned();
                if let Some(entry) = evidence {
                    break entry;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("p47 must expose didChange parse-snapshot evidence for the sequential live change");

        assert_eq!(
            observability.get("parseMode").and_then(|value| value.as_str()),
            Some("incremental")
        );
        assert_eq!(
            observability.get("baseTextSource").and_then(|value| value.as_str()),
            Some("shadow_state")
        );
        assert_eq!(
            observability.get("changeShape").and_then(|value| value.as_str()),
            Some("ranged")
        );
        assert_eq!(
            observability
                .get("contentChangesCount")
                .and_then(|value| value.as_u64()),
            Some(2)
        );
        assert_eq!(
            observability.get("replayOrder").and_then(|value| value.as_str()),
            Some("receive_order")
        );
        assert_eq!(
            observability
                .get("baseDocumentVersion")
                .and_then(|value| value.as_i64()),
            Some(1)
        );
        assert_eq!(
            observability
                .get("fallbackReason")
                .and_then(|value| value.as_str()),
            None
        );

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "requested_version": next_version,
            "parse_mode": observability.get("parseMode").and_then(|value| value.as_str()),
            "base_text_source": observability.get("baseTextSource").and_then(|value| value.as_str()),
            "change_shape": observability.get("changeShape").and_then(|value| value.as_str()),
            "content_changes_count": observability.get("contentChangesCount").and_then(|value| value.as_u64()),
            "replay_order": observability.get("replayOrder").and_then(|value| value.as_str()),
            "base_document_version": observability.get("baseDocumentVersion").and_then(|value| value.as_i64()),
            "changed_ranges_count": observability.get("changedRangesCount").and_then(|value| value.as_u64()),
            "fallback_reason": observability.get("fallbackReason").and_then(|value| value.as_str()),
            "first_insert_len_bytes": first_insert.len(),
            "second_insert_len_bytes": second_insert.len(),
        });
        let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_DID_CHANGE_SEQUENTIAL_REPLAY_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!(
                        "{change_id}-real-conf-big-sequential-ranged-did-change-live.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p47 real conf_big didChange report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p47 real conf_big didChange report"),
        )
        .expect("write p47 real conf_big didChange report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}

#[test]
fn p48_real_conf_big_coalesced_did_change_save_followup_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p48 tokio runtime");
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
            "p48_real_conf_big_coalesced_did_change_save_followup_report_live";
        const APPLY_DELAY_MS: u64 = 4_000;
        const DID_CHANGE_PARSE_DELAY_MS: u64 = 500;
        const TIMELINE_OBSERVE_BUDGET_MS: u64 = 20_000;
        let _env_lock = lock_test_env().await;
        let _apply_delay_guard =
            EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", &APPLY_DELAY_MS.to_string());
        let _did_change_parse_delay_guard = EnvVarGuard::set(
            "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
            &DID_CHANGE_PARSE_DELAY_MS.to_string(),
        );
        let _debounce_guard = EnvVarGuard::set_with_reload(
            "BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS",
            "1200",
            true,
        );

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-21-did-change-ready-snapshot-coalescing".to_string()
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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p48 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p48_real_conf_big_live_setup")
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
        .expect("didOpen must register version 1 for p48");
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
        .expect("didOpen must materialize same-version ready parse snapshot for p48");

        let baseline_metrics = live_transport_get_observability_metrics(&mut harness, 48_100_900).await;
        let baseline_counters = baseline_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("baseline metrics.counters object");

        let appended_procedure = "\nПроцедура Refactor21Live()\n    Возврат;\nКонецПроцедуры\n";
        let v2_text = format!("{module_text}{appended_procedure}");
        live_transport_append_text_change(&mut harness, &uri, &module_text, 2, appended_procedure)
            .await;

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(2)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("didChange must register version 2 for p48");
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
                if task_requested_version == Some(2)
                    && crate::server::language_server::did_change_inline_parse_delay_active_for_test()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("version 2 didChange worker must enter async parse delay before retarget");

        let replacement_start =
            find_utf16_position_after_marker(&v2_text, "Процедура Refactor21Live()\n    ");
        let replacement_end =
            find_utf16_position_after_marker(&v2_text, "Процедура Refactor21Live()\n    Возврат;");
        let semantic_statement = "Сообщить(НеобъявленнаяПеременная);";
        let v3_text = v2_text.replacen("Возврат;", semantic_statement, 1);
        live_transport_ranged_did_change(
            &mut harness,
            &uri,
            3,
            vec![TextDocumentContentChangeEvent {
                range: Some(Range {
                    start: replacement_start,
                    end: replacement_end,
                }),
                range_length: None,
                text: semantic_statement.to_string(),
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
                    == Some(3)
                    && task_requested_version == Some(3)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("didChange must retarget the coalesced worker to version 3 for p48");

        tokio::time::timeout(Duration::from_secs(60), async {
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
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("version 3 exact ready snapshot must materialize before save in p48");

        live_transport_save_document(&mut harness, &uri).await;

        let timeline_deadline = Instant::now() + Duration::from_millis(TIMELINE_OBSERVE_BUDGET_MS);
        let timeline = loop {
            let timeline =
                live_transport_get_diagnostics_save_timeline(&mut harness, 48_100_901, 12).await;
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
                    panic!("p48 must expose a diagnostics save trace for requested_version=3");
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
                    "p48 must observe an idle_heavy follow-up publish on the coalesced exact worker path, last_trace={trace:?}"
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
            full_publish
                .get("publish_kind")
                .and_then(|value| value.as_str()),
            Some("full")
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
        assert!(
            full_publish
                .get("wait_for_file_version_ms")
                .and_then(|value| value.as_u64())
                .is_none(),
            "coalesced exact worker reuse must not regress into wait_for_file_version gating, trace={timeline:?}"
        );
        assert_eq!(
            timeline
                .get("followup_semantic_path")
                .and_then(|value| value.as_str()),
            Some("ready_artifacts")
        );
        assert_eq!(
            timeline
                .get("followup_semantic_parse_source")
                .and_then(|value| value.as_str()),
            Some("snapshot")
        );
        assert!(
            matches!(
                timeline
                    .get("followup_ready_snapshot_zero_probe")
                    .and_then(|value| value.as_str()),
                Some("ready") | Some("not_ready")
            ),
            "p48 must retain explicit ready-snapshot zero-probe attribution, trace={timeline:?}"
        );
        assert!(
            matches!(
                timeline
                    .get("followup_ready_snapshot_wait_probe")
                    .and_then(|value| value.as_str()),
                Some("ready") | None
            ),
            "p48 must not regress into timeout/version-mismatch after coalesced exact worker materialization, trace={timeline:?}"
        );

        let final_metrics = live_transport_get_observability_metrics(&mut harness, 48_100_902).await;
        let final_counters = final_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("final metrics.counters object");

        let started_key =
            "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_did_change";
        let materialized_key =
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_change";
        let retargeted_before_parse_key =
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_parse";
        let retargeted_before_materialization_key =
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_materialization";
        let superseded_key =
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_superseded";

        let started_delta = read_u64_metric(final_counters.get(started_key))
            .saturating_sub(read_u64_metric(baseline_counters.get(started_key)));
        let materialized_delta = read_u64_metric(final_counters.get(materialized_key))
            .saturating_sub(read_u64_metric(baseline_counters.get(materialized_key)));
        let retargeted_before_parse_delta = read_u64_metric(final_counters.get(retargeted_before_parse_key))
            .saturating_sub(read_u64_metric(baseline_counters.get(retargeted_before_parse_key)));
        let retargeted_before_materialization_delta = read_u64_metric(
            final_counters.get(retargeted_before_materialization_key),
        )
        .saturating_sub(read_u64_metric(
            baseline_counters.get(retargeted_before_materialization_key),
        ));
        let superseded_delta = read_u64_metric(final_counters.get(superseded_key))
            .saturating_sub(read_u64_metric(baseline_counters.get(superseded_key)));

        assert!(
            started_delta >= 1,
            "coalesced didChange path must still record at least one producer iteration, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert_eq!(
            materialized_delta, 1,
            "the coalesced producer must materialize exactly the final exact same-file revision, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert!(
            retargeted_before_parse_delta > 0 || retargeted_before_materialization_delta > 0,
            "the older same-file revision must be coalesced away before stale publish, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );
        assert_eq!(
            superseded_delta, 0,
            "same-file coalescing evidence should no longer present as generic superseded churn on this exact-worker path, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
        );

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "did_change_versions": [2, 3],
            "did_change_parse_delay_ms": DID_CHANGE_PARSE_DELAY_MS,
            "apply_delay_ms": APPLY_DELAY_MS,
            "v2_append_len_bytes": appended_procedure.len(),
            "v3_text_len_bytes": v3_text.len(),
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
            "followup_ready_snapshot_task_state": timeline
                .get("followup_ready_snapshot_task_state")
                .and_then(|value| value.as_str()),
            "followup_semantic_path": timeline
                .get("followup_semantic_path")
                .and_then(|value| value.as_str()),
            "followup_semantic_parse_source": timeline
                .get("followup_semantic_parse_source")
                .and_then(|value| value.as_str()),
            "followup_wait_for_file_version_ms": full_publish
                .get("wait_for_file_version_ms")
                .and_then(|value| value.as_u64()),
            "followup_publish_elapsed_ms": full_publish
                .get("elapsed_ms")
                .and_then(|value| value.as_u64()),
            "did_change_worker_started_delta": started_delta,
            "did_change_worker_materialized_delta": materialized_delta,
            "did_change_worker_retargeted_before_parse_delta": retargeted_before_parse_delta,
            "did_change_worker_retargeted_before_materialization_delta": retargeted_before_materialization_delta,
            "did_change_worker_superseded_delta": superseded_delta,
        });
        let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_COALESCED_DID_CHANGE_SAVE_FOLLOWUP_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!(
                        "{change_id}-real-conf-big-coalesced-did-change-save-followup-live.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p48 real conf_big coalescing report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p48 real conf_big coalescing report"),
        )
        .expect("write p48 real conf_big coalescing report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}
