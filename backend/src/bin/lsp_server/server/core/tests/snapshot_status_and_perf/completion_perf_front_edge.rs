#[test]
fn p42_real_conf_big_front_edge_completion_perf_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p42 tokio runtime");
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
        let _parse_delay_guard = EnvVarGuard::set("BSL_TEST_DID_CHANGE_PARSE_DELAY_MS", "1500");
        let _did_save_parse_delay_guard =
            EnvVarGuard::set("BSL_TEST_DID_SAVE_PARSE_DELAY_MS", "1500");

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        const PROFILE_NAME: &str = "p42_real_conf_big_front_edge_completion_perf_report_live";
        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-completion-front-edge-readiness-window".to_string()
        });
        const WARMUP_REQUESTS: usize = 1;
        const MEASURE_REQUESTS: usize = 10;
        const MEASURE_RESPONSE_TIMEOUT_SECS: u64 = 120;
        const TRUTHFUL_WAIT_P95_FACTOR: f64 = 1.0;
        const TRUTHFUL_WAIT_MAX_FACTOR: u64 = 4;
        const SERVICE_FUTURE_FIRST_POLL_P95_BUDGET_MS: f64 = 250.0;
        const SERVICE_FUTURE_FIRST_POLL_MAX_BUDGET_MS: u64 = 1_000;
        const FRONT_EDGE_LATENCY_SLOP_MS: f64 = 10.0;
        let interactive_wait_budget_ms = bsl_runtime::system::global_runtime_config()
            .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
            .unwrap_or(120);

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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p42 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p42_real_conf_big_live_setup")
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

        server.sync_v2_globals().await;
        let file_id = server.get_or_create_file_id_v2(&uri).await;
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if server
                    .latest_current_revision_handoff_versions_v2
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
        .expect("didOpen must publish latest received version on live transport");
        let opened_version = server
            .latest_current_revision_handoff_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
            .expect("latest received version for p42 opened file");
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

        let completion_position = find_utf16_position_after_marker(&module_text, "ЭтотОбъек");
        let completion_context = Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        });
        let mut current_text = module_text.clone();
        let mut current_version = opened_version;

        let mut warmup_samples = Vec::new();
        for index in 0..WARMUP_REQUESTS {
            let request_id = 42_100_000_i64 + index as i64;
            let started = Instant::now();
            let labels = live_transport_completion_labels_with_request(
                &mut harness,
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
                "version": current_version,
            }));
        }

        let metrics_before_measured = coordinator.observability_metrics();
        let counters_before_measured = metrics_before_measured
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics_before_measured.counters object");

        let mut measured_samples = Vec::new();
        for index in 0..MEASURE_REQUESTS {
            let appended_text = if index % 2 == 0 { " " } else { "\n" };
            let next_version = current_version
                .checked_add(1)
                .expect("p42 front-edge version overflow");
            live_transport_append_text_change(
                &mut harness,
                &uri,
                &current_text,
                next_version,
                appended_text,
            )
            .await;
            current_text.push_str(appended_text);
            current_version = next_version;
            live_transport_save_document(&mut harness, &uri).await;

            let parse_gap_source = tokio::time::timeout(Duration::from_millis(800), async {
                loop {
                    if server
                        .latest_current_revision_handoff_versions_v2
                        .read()
                        .await
                        .get(&file_id)
                        .copied()
                        == Some(current_version)
                    {
                        if crate::server::language_server::did_change_inline_parse_delay_active_for_test()
                        {
                            break "didChange";
                        }
                        if crate::server::language_server::did_save_inline_parse_delay_active_for_test()
                        {
                            break "didSave";
                        }
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("test must observe parse-snapshot gap before front-edge completion");

            let request_id = 42_100_100_i64 + index as i64;
            let completion_started = Instant::now();
            let completion_request_written_at_ms = live_transport_write_completion_request(
                &mut harness,
                request_id,
                &uri,
                completion_position,
                completion_context.clone(),
            )
            .await;
            let completion_response = tokio::time::timeout(
                Duration::from_secs(MEASURE_RESPONSE_TIMEOUT_SECS),
                async {
                loop {
                    let response = harness.read_message().await;
                    if response.get("id").and_then(|value| value.as_i64()) == Some(request_id) {
                        break response;
                    }
                }
            },
            )
            .await
            .expect("front-edge completion response must arrive");
            let labels = completion_labels_from_jsonrpc_response(&completion_response);
            measured_samples.push(serde_json::json!({
                "step": format!("measured_front_edge_completion_{}", index + 1),
                "request_id": request_id,
                "elapsed_ms": completion_started.elapsed().as_millis().min(u64::MAX as u128) as u64,
                "label_count": labels.len(),
                "labels": labels,
                "version": current_version,
                "completion_request_written_at_ms": completion_request_written_at_ms,
                "did_change_notifications_per_measured_completion": 1,
                "did_save_after_did_change": true,
                "parse_gap_source": parse_gap_source,
                "appended_text": appended_text,
            }));
        }

        let completion_timeline =
            live_transport_get_completion_timeline(&mut harness, 42_100_900, 160).await;
        let observability_metrics =
            live_transport_get_observability_metrics(&mut harness, 42_100_901).await;
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
                            "timeout_phase": trace
                                .get("prepare_details")
                                .and_then(|value| value.get("timeout_attribution"))
                                .and_then(|value| value.get("phase"))
                                .and_then(|value| value.as_str()),
                            "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                            "dominant_stage": trace.get("dominant_stage").and_then(|value| value.as_str()),
                            "adapter_to_dispatch_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "adapter_to_dispatch_wait_ms",
                            ),
                            "service_future_to_first_poll_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "service_future_to_first_poll_wait_ms",
                            ),
                            "response_output_handoff_send_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "response_output_handoff_send_wait_ms",
                            ),
                            "prepare_stateful_ms": completion_timeline_trace_stage_duration_ms(trace, "prepare_stateful"),
                            "query_bundle_total_ms": completion_timeline_query_bundle_total_ms(trace),
                            "query_bundle": completion_timeline_query_bundle_breakdown(trace),
                            "collect_ms": completion_timeline_trace_stage_duration_ms(trace, "collect"),
                            "response_build_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build"),
                        })
                    });
                    let client_to_transport_wait_ms = trace.and_then(|trace| {
                        sample
                            .get("completion_request_written_at_ms")
                            .and_then(|value| value.as_u64())
                            .zip(completion_timeline_server_edge_u64(trace, "adapter_read_at_ms"))
                            .map(|(request_written_at_ms, adapter_read_at_ms)| {
                                adapter_read_at_ms.saturating_sub(request_written_at_ms)
                            })
                    });
                    let mut sample_object = sample
                        .as_object()
                        .cloned()
                        .expect("sample must be json object");
                    if let Some(client_to_transport_wait_ms) = client_to_transport_wait_ms {
                        sample_object.insert(
                            "client_to_transport_wait_ms".to_string(),
                            serde_json::json!(client_to_transport_wait_ms),
                        );
                    }
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
            .take(24)
            .map(|trace| {
                serde_json::json!({
                    "trace_id": trace.get("trace_id").and_then(|value| value.as_str()),
                    "request_id": trace.get("request_id").and_then(|value| value.as_str()),
                    "trigger_mode": trace.get("trigger_mode").and_then(|value| value.as_str()),
                    "outcome": trace.get("outcome").and_then(|value| value.as_str()),
                    "route": completion_timeline_prepare_detail_str(trace, "route"),
                    "prepare_kind": completion_timeline_prepare_detail_str(trace, "kind"),
                    "fail_closed_cause": completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
                    "timeout_phase": trace
                        .get("prepare_details")
                        .and_then(|value| value.get("timeout_attribution"))
                        .and_then(|value| value.get("phase"))
                        .and_then(|value| value.as_str()),
                    "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                    "dominant_stage": trace.get("dominant_stage").and_then(|value| value.as_str()),
                    "adapter_to_dispatch_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "adapter_to_dispatch_wait_ms",
                    ),
                    "service_future_to_first_poll_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "service_future_to_first_poll_wait_ms",
                    ),
                    "response_output_handoff_send_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "response_output_handoff_send_wait_ms",
                    ),
                    "prepare_stateful_ms": completion_timeline_trace_stage_duration_ms(trace, "prepare_stateful"),
                    "query_bundle": completion_timeline_query_bundle_breakdown(trace),
                    "collect_ms": completion_timeline_trace_stage_duration_ms(trace, "collect"),
                })
            })
            .collect::<Vec<_>>();

        let counter_delta = |name: &str| -> u64 {
            read_u64_metric(counters.get(name))
                .saturating_sub(read_u64_metric(counters_before_measured.get(name)))
        };
        let sample_elapsed_histogram = |samples: &[serde_json::Value]| {
            let values = samples
                .iter()
                .filter_map(|sample| sample.get("elapsed_ms").and_then(|value| value.as_u64()))
                .map(|value| value as f64)
                .collect::<Vec<_>>();
            sample_histogram_value(&values)
        };
        let sample_scalar_histogram = |samples: &[serde_json::Value], field: &str| {
            let values = samples
                .iter()
                .filter_map(|sample| sample.get(field).and_then(|value| value.as_u64()))
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
        let sample_trace_server_edge_histogram = |samples: &[serde_json::Value], field: &str| {
            let values = samples
                .iter()
                .filter_map(|sample| {
                    sample
                        .get("trace")
                        .and_then(|trace| trace.get(field))
                        .and_then(|value| value.as_u64())
                })
                .map(|value| value as f64)
                .collect::<Vec<_>>();
            sample_histogram_value(&values)
        };
        let measured_latency_histogram = sample_elapsed_histogram(&measured_samples);
        let warmup_latency_histogram = sample_elapsed_histogram(&warmup_samples);
        let measured_latency_p95_ms = read_numeric_metric(measured_latency_histogram.get("p95"));
        let truthful_wait_p95_budget_ms =
            (interactive_wait_budget_ms as f64) * TRUTHFUL_WAIT_P95_FACTOR;
        let truthful_wait_max_budget_ms =
            interactive_wait_budget_ms.saturating_mul(TRUTHFUL_WAIT_MAX_FACTOR);
        let measured_client_to_transport_histogram =
            sample_scalar_histogram(&measured_samples, "client_to_transport_wait_ms");
        let measured_client_to_transport_p95_ms =
            read_numeric_metric(measured_client_to_transport_histogram.get("p95"));
        let measured_client_to_transport_present_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("client_to_transport_wait_ms")
                    .and_then(|value| value.as_u64())
                    .is_some()
            })
            .count();
        let measured_client_to_transport_max_ms = measured_samples
            .iter()
            .filter_map(|sample| {
                sample
                    .get("client_to_transport_wait_ms")
                    .and_then(|value| value.as_u64())
            })
            .max()
            .unwrap_or(0);
        let measured_service_future_first_poll_histogram = sample_trace_server_edge_histogram(
            &measured_samples,
            "service_future_to_first_poll_wait_ms",
        );
        let measured_service_future_first_poll_p95_ms =
            read_numeric_metric(measured_service_future_first_poll_histogram.get("p95"));
        let measured_service_future_first_poll_max_ms = measured_samples
            .iter()
            .filter_map(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("service_future_to_first_poll_wait_ms"))
                    .and_then(|value| value.as_u64())
            })
            .max()
            .unwrap_or(0);
        let measured_response_output_handoff_send_histogram = sample_trace_server_edge_histogram(
            &measured_samples,
            "response_output_handoff_send_wait_ms",
        );
        let measured_response_output_handoff_send_p95_ms =
            read_numeric_metric(measured_response_output_handoff_send_histogram.get("p95"));
        let measured_response_output_handoff_send_max_ms = measured_samples
            .iter()
            .filter_map(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("response_output_handoff_send_wait_ms"))
                    .and_then(|value| value.as_u64())
            })
            .max()
            .unwrap_or(0);
        let measured_trace_linked_samples = measured_samples
            .iter()
            .filter(|sample| sample.get("trace").is_some_and(|trace| !trace.is_null()))
            .count();
        let measured_successful_traces = measured_samples
            .iter()
            .filter(|sample| {
                matches!(
                    sample
                        .get("trace")
                        .and_then(|trace| trace.get("outcome"))
                        .and_then(|value| value.as_str()),
                    Some("ok_non_empty" | "ok_empty")
                )
            })
            .count();
        let measured_fail_closed_traces = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("outcome"))
                    .and_then(|value| value.as_str())
                    == Some("fail_closed")
            })
            .count();
        let measured_cancelled_traces = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("outcome"))
                    .and_then(|value| value.as_str())
                    == Some("cancelled")
            })
            .count();
        let measured_superseded_traces = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("outcome"))
                    .and_then(|value| value.as_str())
                    == Some("superseded")
            })
            .count();
        let measured_prepare_timeout_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("fail_closed_cause"))
                    .and_then(|value| value.as_str())
                    == Some("prepare_timeout")
            })
            .count();
        let measured_prepare_timeout_wait_for_file_version_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("fail_closed_cause"))
                    .and_then(|value| value.as_str())
                    == Some("prepare_timeout")
                    && sample
                        .get("trace")
                        .and_then(|trace| trace.get("timeout_phase"))
                        .and_then(|value| value.as_str())
                        == Some("wait_for_file_version")
            })
            .count();
        let measured_prepare_timeout_snapshot_with_deps_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("fail_closed_cause"))
                    .and_then(|value| value.as_str())
                    == Some("prepare_timeout")
                    && sample
                        .get("trace")
                        .and_then(|trace| trace.get("timeout_phase"))
                        .and_then(|value| value.as_str())
                        == Some("snapshot_with_deps")
            })
            .count();
        let measured_cold_query_bundle_pool_wait_samples = measured_samples
            .iter()
            .filter(|sample| {
                matches!(
                    sample
                        .get("trace")
                        .and_then(|trace| trace.get("outcome"))
                        .and_then(|value| value.as_str()),
                    Some("ok_non_empty" | "ok_empty")
                )
                    && sample
                        .get("trace")
                        .and_then(|trace| trace.get("query_bundle"))
                        .and_then(|query_bundle| query_bundle.get("pool_wait_ms"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0)
                        > interactive_wait_budget_ms
            })
            .count();
        let measured_wait_exact_type_index_dominant_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("dominant_stage"))
                    .and_then(|value| value.as_str())
                    == Some("wait_exact_type_index")
            })
            .count();
        let measured_missing_semantic_index_fail_closed_total_delta = counter_delta(
            "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_completion_reason_missing_semantic_index",
        );

        let report = serde_json::json!({
            "change_id": change_id,
            "profile": PROFILE_NAME,
            "schema_version": 1,
            "configuration_path": conf_big_root,
            "module_path": module_path,
            "marker": "ЭтотОбъек",
            "request_plan": {
                "cache_mode": "same_file_didChange_didSave_immediate_invoked_completion_burst",
                "wait_for_current_revision_before_seed": true,
                "warmup_requests": WARMUP_REQUESTS,
                "measured_requests": MEASURE_REQUESTS,
                "completion_trigger_mode": "invoked",
                "completion_position_kind": "non_member_identifier_tail",
                "did_change_notifications_per_measured_completion": 1,
                "did_save_after_did_change": true,
                "did_change_parse_delay_ms": 1500,
                "did_save_parse_delay_ms": 1500,
                "parse_gap_activation_sources": ["didChange", "didSave"],
            },
            "warmup_samples": warmup_samples,
            "measured_samples": measured_samples,
            "summary": {
                "trace_count_for_uri": filtered_traces.len(),
                "trace_matching_mode": trace_matching_mode,
                "trace_request_id_present_total": trace_request_id_present_total,
                "measured_trace_linked_samples": measured_trace_linked_samples,
                "measured_successful_traces": measured_successful_traces,
                "measured_fail_closed_traces": measured_fail_closed_traces,
                "measured_cancelled_traces": measured_cancelled_traces,
                "measured_superseded_traces": measured_superseded_traces,
                "measured_prepare_timeout_samples": measured_prepare_timeout_samples,
                "measured_prepare_timeout_wait_for_file_version_samples": measured_prepare_timeout_wait_for_file_version_samples,
                "measured_prepare_timeout_snapshot_with_deps_samples": measured_prepare_timeout_snapshot_with_deps_samples,
                "measured_cold_query_bundle_pool_wait_samples": measured_cold_query_bundle_pool_wait_samples,
                "measured_wait_exact_type_index_dominant_samples": measured_wait_exact_type_index_dominant_samples,
                "measured_completion_total_delta": counter_delta("completion_total"),
                "measured_fail_closed_total_delta": counter_delta("intellisense_v2_completion_result_total_fail_closed"),
                "measured_missing_semantic_index_fail_closed_total_delta": measured_missing_semantic_index_fail_closed_total_delta,
                "measured_prepare_timeout_total_delta": counter_delta(
                    "intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout"
                ),
                "measured_exact_deadline_total_delta": counter_delta(
                    "intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline"
                ),
                "measured_fallback_unavailable_total_delta": counter_delta("intellisense_v2_completion_fallback_unavailable_total"),
                "measured_interactive_wait_budget_exhausted_total_delta": counter_delta("intellisense_v2_interactive_wait_budget_exhausted_total"),
                "interactive_wait_budget_ms": interactive_wait_budget_ms,
                "truthful_wait_p95_budget_ms": truthful_wait_p95_budget_ms,
                "truthful_wait_max_budget_ms": truthful_wait_max_budget_ms,
                "warmup_latency_ms": warmup_latency_histogram,
                "measured_latency_ms": measured_latency_histogram,
                "measured_client_to_transport_wait_ms": measured_client_to_transport_histogram,
                "measured_client_to_transport_wait_max_ms": measured_client_to_transport_max_ms,
                "measured_client_to_transport_present_samples": measured_client_to_transport_present_samples,
                "measured_service_future_to_first_poll_wait_ms": measured_service_future_first_poll_histogram,
                "measured_service_future_to_first_poll_wait_max_ms": measured_service_future_first_poll_max_ms,
                "measured_response_output_handoff_send_wait_ms": measured_response_output_handoff_send_histogram,
                "measured_response_output_handoff_send_wait_max_ms": measured_response_output_handoff_send_max_ms,
                "measured_prepare_stateful_ms": sample_trace_histogram(&measured_samples, "prepare_stateful_ms"),
                "measured_query_bundle_total_ms": sample_trace_histogram(&measured_samples, "query_bundle_total_ms"),
                "measured_collect_ms": sample_trace_histogram(&measured_samples, "collect_ms"),
                "measured_collect_breakdown_ms": {
                    "non_member_local_symbols": histogram_metric_value_or_zero(
                        histograms,
                        "completion_stage_collect_non_member_local_symbols_ms",
                        None
                    ),
                    "non_member_contextual_symbols": histogram_metric_value_or_zero(
                        histograms,
                        "completion_stage_collect_non_member_contextual_symbols_ms",
                        None
                    ),
                    "non_member_module_routines": histogram_metric_value_or_zero(
                        histograms,
                        "completion_stage_collect_non_member_module_routines_ms",
                        None
                    ),
                    "non_member_global_functions": histogram_metric_value_or_zero(
                        histograms,
                        "completion_stage_collect_non_member_global_functions_ms",
                        None
                    ),
                    "non_member_metadata_items": histogram_metric_value_or_zero(
                        histograms,
                        "completion_stage_collect_non_member_metadata_items_ms",
                        None
                    ),
                    "non_member_repository_types": histogram_metric_value_or_zero(
                        histograms,
                        "completion_stage_collect_non_member_repository_types_ms",
                        None
                    ),
                    "non_member_keywords": histogram_metric_value_or_zero(
                        histograms,
                        "completion_stage_collect_non_member_keywords_ms",
                        None
                    ),
                },
            },
            "extension_like_key_latencies": {
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

        let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_FRONT_EDGE_COMPLETION_PERF_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!(
                        "{change_id}-real-conf-big-front-edge-completion-perf-live.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p42 real conf_big perf report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report).expect("serialize p42 real conf_big perf report"),
        )
        .expect("write p42 real conf_big perf report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        assert!(
            trace_matching_mode == "request_id",
            "expected request-context parity to expose JSON-RPC request ids in completion timeline, trace_matching_mode={}, trace_request_id_present_total={}, filtered_traces={filtered_traces:?}",
            trace_matching_mode,
            trace_request_id_present_total
        );
        assert!(
            measured_trace_linked_samples == MEASURE_REQUESTS,
            "expected every measured front-edge sample to link to a completion timeline trace, measured_trace_linked_samples={}, measured_samples={measured_samples:?}",
            measured_trace_linked_samples
        );
        assert!(
            measured_client_to_transport_present_samples == MEASURE_REQUESTS,
            "expected every measured front-edge sample to expose harness-derived client_to_transport_wait_ms, measured_client_to_transport_present_samples={}, measured_samples={measured_samples:?}",
            measured_client_to_transport_present_samples
        );
        assert!(
            measured_prepare_timeout_samples == 0
                && counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout") == 0,
            "front-edge gate must fail on any prepare_timeout after same-file handoff, measured_prepare_timeout_samples={}, prepare_timeout_total_delta={}, measured_samples={measured_samples:?}",
            measured_prepare_timeout_samples,
            counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout")
        );
        assert!(
            counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline")
                == 0,
            "front-edge gate must fail on any exact_deadline after same-file handoff, exact_deadline_total_delta={}, measured_samples={measured_samples:?}",
            counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline")
        );
        assert!(
            measured_cancelled_traces == 0 && measured_superseded_traces == 0,
            "front-edge gate must not regress into cancelled/superseded outcomes after same-file handoff, measured_cancelled_traces={}, measured_superseded_traces={}, measured_samples={measured_samples:?}",
            measured_cancelled_traces,
            measured_superseded_traces
        );
        assert!(
            measured_successful_traces > 0,
            "front-edge samples must produce at least one bounded successful current-revision response after same-file handoff, measured_successful_traces={}, measured_samples={measured_samples:?}",
            measured_successful_traces,
        );
        assert!(
            measured_fail_closed_traces == 0
                && counter_delta("intellisense_v2_completion_result_total_fail_closed") == 0
                && measured_missing_semantic_index_fail_closed_total_delta == 0
                && measured_wait_exact_type_index_dominant_samples == 0,
            "front-edge gate must fail on hidden exact-deadline/missing_semantic_index regressions once current-revision success is available, measured_fail_closed_traces={}, measured_missing_semantic_index_fail_closed_total_delta={}, measured_wait_exact_type_index_dominant_samples={}, measured_samples={measured_samples:?}",
            measured_fail_closed_traces,
            measured_missing_semantic_index_fail_closed_total_delta,
            measured_wait_exact_type_index_dominant_samples
        );
        assert!(
            measured_client_to_transport_p95_ms <= truthful_wait_p95_budget_ms,
            "front-edge truthful ingress p95 regression: measured_client_to_transport_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_client_to_transport_p95_ms,
            truthful_wait_p95_budget_ms
        );
        assert!(
            measured_client_to_transport_max_ms <= truthful_wait_max_budget_ms,
            "front-edge truthful ingress max regression: measured_client_to_transport_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_client_to_transport_max_ms,
            truthful_wait_max_budget_ms
        );
        assert!(
            measured_service_future_first_poll_p95_ms <= SERVICE_FUTURE_FIRST_POLL_P95_BUDGET_MS,
            "front-edge pre-poll p95 regression: measured_service_future_to_first_poll_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_service_future_first_poll_p95_ms,
            SERVICE_FUTURE_FIRST_POLL_P95_BUDGET_MS
        );
        assert!(
            measured_service_future_first_poll_max_ms <= SERVICE_FUTURE_FIRST_POLL_MAX_BUDGET_MS,
            "front-edge pre-poll max regression: measured_service_future_to_first_poll_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_service_future_first_poll_max_ms,
            SERVICE_FUTURE_FIRST_POLL_MAX_BUDGET_MS
        );
        assert!(
            measured_response_output_handoff_send_p95_ms <= truthful_wait_p95_budget_ms,
            "front-edge egress handoff p95 regression: measured_response_output_handoff_send_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_response_output_handoff_send_p95_ms,
            truthful_wait_p95_budget_ms
        );
        assert!(
            measured_response_output_handoff_send_max_ms <= truthful_wait_max_budget_ms,
            "front-edge egress handoff max regression: measured_response_output_handoff_send_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_response_output_handoff_send_max_ms,
            truthful_wait_max_budget_ms
        );
        if measured_cold_query_bundle_pool_wait_samples == 0 {
            assert!(
                measured_latency_p95_ms
                    <= interactive_wait_budget_ms as f64 + FRONT_EDGE_LATENCY_SLOP_MS,
                "front-edge latency p95 regression: measured_latency_p95_ms={}ms > {}ms without downstream cold query_bundle_pool_wait bucket, measured_samples={measured_samples:?}",
                measured_latency_p95_ms,
                interactive_wait_budget_ms as f64 + FRONT_EDGE_LATENCY_SLOP_MS
            );
        }

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}
