#[test]
fn p38_real_conf_big_revision_churn_completion_perf_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p38 tokio runtime");
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

    let _env_lock = lock_test_env().await;
    let _blocking_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "1500");

    let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
    const PROFILE_NAME: &str =
        "p38_real_conf_big_post_handoff_readiness_completion_perf_report_live";
    let change_id = std::env::var("CHANGE_ID")
        .unwrap_or_else(|_| "refactor-completion-turn-wait-slot-release".to_string());
    const WARMUP_REQUESTS: usize = 1;
    const MEASURE_REQUESTS: usize = 10;
    const DID_CHANGE_BURST_NOTIFICATIONS: usize = 4;
    const REVISION_CHURN_HEAD_PATH_P95_BUDGET_MS: f64 = 150.0;
    const SERVICE_FUTURE_FIRST_POLL_P95_BUDGET_MS: f64 = 250.0;
    const SERVICE_FUTURE_FIRST_POLL_MAX_BUDGET_MS: u64 = 1_000;
    const POST_HANDOFF_QUEUE_WAIT_P95_FACTOR: f64 = 0.50;
    const POST_HANDOFF_QUEUE_WAIT_MAX_FACTOR: u64 = 4;
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
        std::fs::read_to_string(&module_path).expect("read conf_big module text for p38 report");
    let workspace_setup = ScaleAwareWorkspaceSetup {
        platform_docs_archive: syntax_helper_path_for_tests(),
        configuration_path: conf_big_root.clone(),
        platform_version: "8.3.25".to_string(),
    };
    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_workspace_setup(&server, &workspace_setup, "p38_real_conf_big_live_setup")
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
    server.did_open(did_open).await;

    server.sync_v2_globals().await;
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
    .expect("didOpen must publish latest received version on live transport");
    let opened_version = server
        .latest_received_file_versions_v2
        .read()
        .await
        .get(&file_id)
        .copied()
        .expect("latest received version for p38 opened file");
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
        "mode": "not_requested",
    });
    server.cancel_type_index_precompute_v2(file_id).await;

    let completion_position = find_utf16_position_after_marker(&module_text, "Объект.");
    let completion_context = Some(CompletionContext {
        trigger_kind: CompletionTriggerKind::INVOKED,
        trigger_character: None,
    });

    let mut current_text = module_text.clone();
    let mut current_version = opened_version;

    let mut warmup_samples = Vec::new();
    for index in 0..WARMUP_REQUESTS {
        let request_id = 38_100_000_i64 + index as i64;
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
        let mut burst_versions = Vec::new();
        let mut burst_appended = String::new();
        for burst_index in 0..DID_CHANGE_BURST_NOTIFICATIONS {
            let appended_text = if (index + burst_index) % 2 == 0 {
                " "
            } else {
                "\n"
            };
            let next_version = current_version
                .checked_add(1)
                .expect("p38 revision churn version overflow");
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
            burst_versions.push(current_version);
            burst_appended.push_str(appended_text);
        }

        let request_id = 38_100_100_i64 + index as i64;
        let started = Instant::now();
        let labels = live_transport_completion_labels_with_request(
            &mut harness,
            request_id,
            &uri,
            completion_position,
            completion_context.clone(),
        )
        .await;
        measured_samples.push(serde_json::json!({
            "step": format!("measured_revision_churn_completion_{}", index + 1),
            "request_id": request_id,
            "elapsed_ms": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            "label_count": labels.len(),
            "labels": labels,
            "version": current_version,
            "burst_notification_count": DID_CHANGE_BURST_NOTIFICATIONS,
            "burst_versions": burst_versions,
            "appended_text": burst_appended,
        }));
    }

    let completion_timeline =
        live_transport_get_completion_timeline(&mut harness, 38_100_900, 96).await;
    let observability_metrics =
        live_transport_get_observability_metrics(&mut harness, 38_100_901).await;
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
                        "min_file_version": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("min_file_version"))
                            .and_then(|value| value.as_i64()),
                        "observed_file_version": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("observed_file_version"))
                            .and_then(|value| value.as_i64()),
                        "wait_for_file_version_runtime_queue_wait_ms": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("wait_for_file_version_runtime"))
                            .and_then(|value| value.get("queue_wait_ms"))
                            .and_then(|value| value.as_u64()),
                        "timeout_phase": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("timeout_attribution"))
                            .and_then(|value| value.get("phase"))
                            .and_then(|value| value.as_str()),
                        "timeout_source": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("timeout_attribution"))
                            .and_then(|value| value.get("source"))
                            .and_then(|value| value.as_str()),
                        "head_ready_before_wait": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("exact_wait"))
                            .and_then(|value| value.get("head_ready_before_wait"))
                            .and_then(|value| value.as_bool()),
                        "exact_ready_before_wait": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("exact_wait"))
                            .and_then(|value| value.get("exact_ready_before_wait"))
                            .and_then(|value| value.as_bool()),
                        "artifact_poll": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("exact_wait"))
                            .and_then(|value| value.get("artifact_poll"))
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "dispatch_to_request_context_wait_ms": completion_timeline_server_edge_u64(
                            trace,
                            "dispatch_to_request_context_wait_ms",
                        ),
                        "transport_to_service_future_wait_ms": completion_timeline_server_edge_u64(
                            trace,
                            "transport_to_service_future_wait_ms",
                        ),
                        "service_future_to_first_poll_wait_ms": completion_timeline_server_edge_u64(
                            trace,
                            "service_future_to_first_poll_wait_ms",
                        ),
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
                        "response_build_other_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build_other"),
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
        .take(20)
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
                "min_file_version": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("min_file_version"))
                    .and_then(|value| value.as_i64()),
                "observed_file_version": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("observed_file_version"))
                    .and_then(|value| value.as_i64()),
                "wait_for_file_version_runtime_queue_wait_ms": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("wait_for_file_version_runtime"))
                    .and_then(|value| value.get("queue_wait_ms"))
                    .and_then(|value| value.as_u64()),
                "timeout_phase": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("timeout_attribution"))
                    .and_then(|value| value.get("phase"))
                    .and_then(|value| value.as_str()),
                "timeout_source": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("timeout_attribution"))
                    .and_then(|value| value.get("source"))
                    .and_then(|value| value.as_str()),
                "head_ready_before_wait": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("exact_wait"))
                    .and_then(|value| value.get("head_ready_before_wait"))
                    .and_then(|value| value.as_bool()),
                "exact_ready_before_wait": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("exact_wait"))
                    .and_then(|value| value.get("exact_ready_before_wait"))
                    .and_then(|value| value.as_bool()),
                "artifact_poll": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("exact_wait"))
                    .and_then(|value| value.get("artifact_poll"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                "prepare_wait_elapsed_ms": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("wait_elapsed_ms"))
                    .and_then(|value| value.as_u64()),
                "dispatch_to_request_context_wait_ms": completion_timeline_server_edge_u64(
                    trace,
                    "dispatch_to_request_context_wait_ms",
                ),
                "transport_to_service_future_wait_ms": completion_timeline_server_edge_u64(
                    trace,
                    "transport_to_service_future_wait_ms",
                ),
                "service_future_to_first_poll_wait_ms": completion_timeline_server_edge_u64(
                    trace,
                    "service_future_to_first_poll_wait_ms",
                ),
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

    let counter_delta = |name: &str| -> u64 {
        read_u64_metric(counters.get(name))
            .saturating_sub(read_u64_metric(counters_before_measured.get(name)))
    };

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
    let measured_head_hit_traces = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("route"))
                .and_then(|value| value.as_str())
                == Some("head_hit")
        })
        .count();
    let measured_exact_hit_traces = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("route"))
                .and_then(|value| value.as_str())
                == Some("exact_hit")
        })
        .count();
    let measured_trace_linked_samples = measured_samples
        .iter()
        .filter(|sample| sample.get("trace").is_some_and(|trace| !trace.is_null()))
        .count();
    let measured_lightweight_prepare_traces = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("prepare_kind"))
                .and_then(|value| value.as_str())
                == Some("lightweight_current_revision")
        })
        .count();
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
    let warmup_latency_histogram = sample_elapsed_histogram(&warmup_samples);
    let measured_latency_histogram = sample_elapsed_histogram(&measured_samples);
    let measured_latency_p95_ms = read_numeric_metric(measured_latency_histogram.get("p95"));
    let measured_wait_for_file_version_runtime_queue_wait_histogram = sample_trace_histogram(
        &measured_samples,
        "wait_for_file_version_runtime_queue_wait_ms",
    );
    let measured_wait_for_file_version_runtime_queue_wait_p95_ms =
        read_numeric_metric(measured_wait_for_file_version_runtime_queue_wait_histogram.get("p95"));
    let measured_wait_for_file_version_runtime_queue_wait_present_samples = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("wait_for_file_version_runtime_queue_wait_ms"))
                .and_then(|value| value.as_u64())
                .is_some()
        })
        .count();
    let measured_head_ready_before_wait_present_samples = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("head_ready_before_wait"))
                .and_then(|value| value.as_bool())
                .is_some()
        })
        .count();
    let measured_head_ready_before_wait_true_samples = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("head_ready_before_wait"))
                .and_then(|value| value.as_bool())
                == Some(true)
        })
        .count();
    let measured_exact_ready_before_wait_present_samples = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("exact_ready_before_wait"))
                .and_then(|value| value.as_bool())
                .is_some()
        })
        .count();
    let measured_exact_ready_before_wait_true_samples = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("exact_ready_before_wait"))
                .and_then(|value| value.as_bool())
                == Some(true)
        })
        .count();
    let measured_wait_for_file_version_runtime_queue_wait_bypassed_fast_ready_samples =
        measured_samples
            .iter()
            .filter(|sample| {
                let trace = sample.get("trace");
                trace
                    .and_then(|trace| trace.get("wait_for_file_version_runtime_queue_wait_ms"))
                    .and_then(|value| value.as_u64())
                    .is_none()
                    && (trace
                        .and_then(|trace| trace.get("head_ready_before_wait"))
                        .and_then(|value| value.as_bool())
                        == Some(true)
                        || trace
                            .and_then(|trace| trace.get("exact_ready_before_wait"))
                            .and_then(|value| value.as_bool())
                            == Some(true))
            })
            .count();
    let measured_wait_for_file_version_runtime_queue_wait_missing_without_fast_ready_samples =
        measured_samples
            .iter()
            .filter(|sample| {
                let trace = sample.get("trace");
                trace
                    .and_then(|trace| trace.get("wait_for_file_version_runtime_queue_wait_ms"))
                    .and_then(|value| value.as_u64())
                    .is_none()
                    && trace
                        .and_then(|trace| trace.get("head_ready_before_wait"))
                        .and_then(|value| value.as_bool())
                        != Some(true)
                    && trace
                        .and_then(|trace| trace.get("exact_ready_before_wait"))
                        .and_then(|value| value.as_bool())
                        != Some(true)
            })
            .count();
    let measured_wait_for_file_version_runtime_queue_wait_max_ms = measured_samples
        .iter()
        .filter_map(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("wait_for_file_version_runtime_queue_wait_ms"))
                .and_then(|value| value.as_u64())
        })
        .max()
        .unwrap_or(0);
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
    let measured_post_apply_head_gap_exact_deadline_samples = measured_samples
        .iter()
        .filter(|sample| {
            let trace = sample.get("trace");
            let artifact_poll = trace
                .and_then(|trace| trace.get("artifact_poll"))
                .filter(|value| !value.is_null());
            trace
                .and_then(|trace| trace.get("fail_closed_cause"))
                .and_then(|value| value.as_str())
                == Some("exact_deadline")
                && trace
                    .and_then(|trace| trace.get("head_ready_before_wait"))
                    .and_then(|value| value.as_bool())
                    == Some(false)
                && trace
                    .and_then(|trace| trace.get("min_file_version"))
                    .and_then(|value| value.as_i64())
                    .zip(
                        artifact_poll
                            .and_then(|poll| poll.get("observed_file_version"))
                            .and_then(|value| value.as_i64()),
                    )
                    .is_some_and(|(min_file_version, observed_file_version)| {
                        min_file_version == observed_file_version
                    })
        })
        .count();
    let post_handoff_queue_wait_p95_budget_ms =
        (interactive_wait_budget_ms as f64) * POST_HANDOFF_QUEUE_WAIT_P95_FACTOR;
    let post_handoff_queue_wait_max_budget_ms =
        interactive_wait_budget_ms.saturating_mul(POST_HANDOFF_QUEUE_WAIT_MAX_FACTOR);
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

    let report = serde_json::json!({
        "change_id": change_id,
        "profile": PROFILE_NAME,
        "schema_version": 1,
        "configuration_path": conf_big_root,
        "module_path": module_path,
        "marker": "Объект.",
        "request_plan": {
            "cache_mode": "self_warmed_then_revision_churn_same_process",
            "wait_for_current_revision_before_seed": true,
            "exact_type_index_seed_mode": "not_requested",
            "warmup_requests": WARMUP_REQUESTS,
            "measured_requests": MEASURE_REQUESTS,
            "completion_trigger_mode": "invoked",
            "transport_path": "tower_lsp_server_serve_duplex",
            "churn_profile": "didChange-burst",
            "churn_before_each_measured_completion": true,
            "churn_edit_kind": "append_at_eof_incremental",
            "did_change_notifications_per_measured_completion": DID_CHANGE_BURST_NOTIFICATIONS,
            "did_change_blocking_parse_delay_ms": 1500,
        },
        "warm_cache_seed": exact_type_index_seed,
        "warmup_samples": warmup_samples,
        "measured_samples": measured_samples,
        "summary": {
            "trace_count_for_uri": filtered_traces.len(),
            "trace_matching_mode": trace_matching_mode,
            "trace_request_id_present_total": trace_request_id_present_total,
            "warmup_non_empty_samples": warmup_non_empty_samples,
            "measured_trace_linked_samples": measured_trace_linked_samples,
            "measured_non_empty_samples": measured_non_empty_samples,
            "measured_ok_non_empty_traces": measured_ok_non_empty_traces,
            "measured_fail_closed_traces": measured_fail_closed_traces,
            "measured_head_hit_traces": measured_head_hit_traces,
            "measured_exact_hit_traces": measured_exact_hit_traces,
            "measured_lightweight_prepare_traces": measured_lightweight_prepare_traces,
            "measured_completion_total_delta": counter_delta("completion_total"),
            "measured_ok_non_empty_total_delta": counter_delta("intellisense_v2_completion_result_total_ok_non_empty"),
            "measured_ok_empty_total_delta": counter_delta("intellisense_v2_completion_result_total_ok_empty"),
            "measured_fail_closed_total_delta": counter_delta("intellisense_v2_completion_result_total_fail_closed"),
            "measured_cancelled_total_delta": counter_delta("intellisense_v2_completion_result_total_cancelled"),
            "measured_deadline_total_delta": counter_delta("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"),
            "measured_ready_total_delta": counter_delta("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready"),
            "measured_no_matching_task_total_delta": counter_delta("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task"),
            "measured_task_present_wrong_version_total_delta": counter_delta("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_task_present_wrong_version"),
            "measured_observed_version_mismatch_total_delta": counter_delta("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_observed_version_mismatch"),
            "measured_head_hit_total_delta": counter_delta("intellisense_v2_completion_route_total_route_head_hit"),
            "measured_exact_hit_total_delta": counter_delta("intellisense_v2_completion_route_total_route_exact_hit"),
            "measured_head_to_exact_upgrade_total_delta": counter_delta("intellisense_v2_completion_head_to_exact_upgrade_total"),
            "measured_prepare_timeout_total_delta": counter_delta(
                "intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout"
            ),
            "measured_exact_deadline_total_delta": counter_delta(
                "intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline"
            ),
            "measured_fallback_unavailable_total_delta": counter_delta("intellisense_v2_completion_fallback_unavailable_total"),
            "measured_interactive_wait_budget_exhausted_total_delta": counter_delta("intellisense_v2_interactive_wait_budget_exhausted_total"),
            "warmup_latency_ms": warmup_latency_histogram,
            "measured_latency_ms": measured_latency_histogram,
            "interactive_wait_budget_ms": interactive_wait_budget_ms,
            "measured_wait_for_file_version_runtime_queue_wait_ms": measured_wait_for_file_version_runtime_queue_wait_histogram,
            "measured_wait_for_file_version_runtime_queue_wait_present_samples": measured_wait_for_file_version_runtime_queue_wait_present_samples,
            "measured_head_ready_before_wait_present_samples": measured_head_ready_before_wait_present_samples,
            "measured_head_ready_before_wait_true_samples": measured_head_ready_before_wait_true_samples,
            "measured_exact_ready_before_wait_present_samples": measured_exact_ready_before_wait_present_samples,
            "measured_exact_ready_before_wait_true_samples": measured_exact_ready_before_wait_true_samples,
            "measured_wait_for_file_version_runtime_queue_wait_bypassed_fast_ready_samples": measured_wait_for_file_version_runtime_queue_wait_bypassed_fast_ready_samples,
            "measured_wait_for_file_version_runtime_queue_wait_missing_without_fast_ready_samples": measured_wait_for_file_version_runtime_queue_wait_missing_without_fast_ready_samples,
            "measured_wait_for_file_version_runtime_queue_wait_max_ms": measured_wait_for_file_version_runtime_queue_wait_max_ms,
            "measured_prepare_timeout_wait_for_file_version_samples": measured_prepare_timeout_wait_for_file_version_samples,
            "measured_post_apply_head_gap_exact_deadline_samples": measured_post_apply_head_gap_exact_deadline_samples,
            "measured_service_future_to_first_poll_wait_ms": measured_service_future_first_poll_histogram,
            "measured_service_future_to_first_poll_wait_max_ms": measured_service_future_first_poll_max_ms,
            "measured_dispatch_to_request_context_wait_ms": sample_trace_server_edge_histogram(
                &measured_samples,
                "dispatch_to_request_context_wait_ms"
            ),
            "measured_transport_to_service_future_wait_ms": sample_trace_server_edge_histogram(
                &measured_samples,
                "transport_to_service_future_wait_ms"
            ),
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

    let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_REVISION_CHURN_COMPLETION_PERF_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!(
                    "{change_id}-real-conf-big-revision-churn-completion-perf-live.json"
                ))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for p38 real conf_big perf report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize p38 real conf_big perf report"),
    )
    .expect("write p38 real conf_big perf report");
    println!("{PROFILE_NAME}_path={}", report_path.display());

    assert!(
        trace_matching_mode == "request_id",
        "expected request-context parity to expose JSON-RPC request ids in completion timeline, trace_matching_mode={}, trace_request_id_present_total={}, filtered_traces={filtered_traces:?}",
        trace_matching_mode,
        trace_request_id_present_total
    );
    assert!(
        warmup_non_empty_samples == WARMUP_REQUESTS,
        "expected baseline warm-cache samples to be non-empty before churn, warmup_non_empty_samples={}, warmup_samples={warmup_samples:?}",
        warmup_non_empty_samples
    );
    assert!(
        measured_trace_linked_samples == MEASURE_REQUESTS,
        "expected every measured churn sample to link to a completion timeline trace, measured_trace_linked_samples={}, measured_samples={measured_samples:?}",
        measured_trace_linked_samples
    );
    assert!(
        measured_wait_for_file_version_runtime_queue_wait_present_samples
            + measured_wait_for_file_version_runtime_queue_wait_bypassed_fast_ready_samples
            == MEASURE_REQUESTS,
        "expected every measured split-prepare sample to either expose wait_for_file_version_runtime.queue_wait_ms or bypass wait via head-ready/exact-ready fast path, present_samples={}, bypassed_fast_ready_samples={}, measured_samples={measured_samples:?}",
        measured_wait_for_file_version_runtime_queue_wait_present_samples,
        measured_wait_for_file_version_runtime_queue_wait_bypassed_fast_ready_samples
    );
    assert!(
        measured_wait_for_file_version_runtime_queue_wait_missing_without_fast_ready_samples == 0,
        "wait_for_file_version_runtime.queue_wait_ms may be absent only when completion is already head-ready/exact-ready before wait, missing_without_fast_ready_samples={}, measured_samples={measured_samples:?}",
        measured_wait_for_file_version_runtime_queue_wait_missing_without_fast_ready_samples
    );
    assert!(
        measured_head_ready_before_wait_present_samples == MEASURE_REQUESTS,
        "expected every measured post-handoff readiness sample to expose head_ready_before_wait, present_samples={}, measured_samples={measured_samples:?}",
        measured_head_ready_before_wait_present_samples
    );
    assert!(
        measured_exact_ready_before_wait_present_samples == MEASURE_REQUESTS,
        "expected every measured split-prepare sample to expose exact_ready_before_wait, present_samples={}, measured_samples={measured_samples:?}",
        measured_exact_ready_before_wait_present_samples
    );
    assert!(
        measured_lightweight_prepare_traces == MEASURE_REQUESTS,
        "expected every measured revision-churn sample to report lightweight current-revision prepare kind, measured_lightweight_prepare_traces={}, measured_samples={measured_samples:?}",
        measured_lightweight_prepare_traces
    );
    assert!(
        counter_delta("completion_total") >= MEASURE_REQUESTS as u64,
        "expected measured completion_total delta >= churn request samples, completion_total_delta={}, measured_requests={}",
        counter_delta("completion_total"),
        MEASURE_REQUESTS
    );
    assert!(
        measured_non_empty_samples == MEASURE_REQUESTS,
        "expected every measured revision-churn sample to return a first-response candidate list, measured_non_empty_samples={}, measured_samples={measured_samples:?}",
        measured_non_empty_samples
    );
    assert!(
        measured_ok_non_empty_traces == MEASURE_REQUESTS,
        "expected every measured revision-churn trace to be ok_non_empty, measured_ok_non_empty_traces={}, measured_samples={measured_samples:?}",
        measured_ok_non_empty_traces
    );
    assert!(
        measured_fail_closed_traces == 0
            && counter_delta("intellisense_v2_completion_result_total_fail_closed") == 0,
        "revision-churn gate must fail on first-response fail_closed regressions, measured_fail_closed_traces={}, fail_closed_total_delta={}, measured_samples={measured_samples:?}",
        measured_fail_closed_traces,
        counter_delta("intellisense_v2_completion_result_total_fail_closed")
    );
    assert!(
        counter_delta("intellisense_v2_completion_fallback_unavailable_total") == 0,
        "revision-churn gate must not degrade to fallback_unavailable, fallback_unavailable_total_delta={}, counters={counters:?}",
        counter_delta("intellisense_v2_completion_fallback_unavailable_total")
    );
    assert!(
        counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout")
            == 0
            && counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline")
                == 0,
        "revision-churn gate must keep first-response fail-closed cause buckets at zero after head-path rollout, prepare_timeout_total_delta={}, exact_deadline_total_delta={}, counters={counters:?}",
        counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout"),
        counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline")
    );
    assert!(
        measured_head_hit_traces + measured_exact_hit_traces == MEASURE_REQUESTS,
        "expected every measured revision-churn trace to expose head/exact route attribution, measured_head_hit_traces={}, measured_exact_hit_traces={}, measured_samples={measured_samples:?}",
        measured_head_hit_traces,
        measured_exact_hit_traces
    );
    assert!(
        measured_latency_p95_ms <= REVISION_CHURN_HEAD_PATH_P95_BUDGET_MS,
        "revision-churn head-path p95 regression: measured_latency_p95_ms={}ms > {}ms, measured_samples={measured_samples:?}",
        measured_latency_p95_ms,
        REVISION_CHURN_HEAD_PATH_P95_BUDGET_MS
    );
    if measured_wait_for_file_version_runtime_queue_wait_present_samples > 0 {
        assert!(
            measured_wait_for_file_version_runtime_queue_wait_p95_ms
                <= post_handoff_queue_wait_p95_budget_ms,
            "post-handoff readiness queue-wait p95 regression: measured_wait_for_file_version_runtime.queue_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_wait_for_file_version_runtime_queue_wait_p95_ms,
            post_handoff_queue_wait_p95_budget_ms
        );
        assert!(
            measured_wait_for_file_version_runtime_queue_wait_max_ms
                <= post_handoff_queue_wait_max_budget_ms,
            "post-handoff readiness queue-wait max regression: measured_wait_for_file_version_runtime.queue_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_wait_for_file_version_runtime_queue_wait_max_ms,
            post_handoff_queue_wait_max_budget_ms
        );
    }
    assert!(
        measured_prepare_timeout_wait_for_file_version_samples == 0,
        "post-handoff readiness gate must fail on prepare_timeout@wait_for_file_version after same-file handoff, prepare_timeout_wait_for_file_version_samples={}, measured_samples={measured_samples:?}",
        measured_prepare_timeout_wait_for_file_version_samples
    );
    assert!(
        measured_post_apply_head_gap_exact_deadline_samples == 0,
        "post-handoff readiness gate must fail on exact_deadline with observed current revision and head_ready_before_wait=false, samples={}, measured_samples={measured_samples:?}",
        measured_post_apply_head_gap_exact_deadline_samples
    );
    assert!(
        measured_service_future_first_poll_p95_ms <= SERVICE_FUTURE_FIRST_POLL_P95_BUDGET_MS,
        "revision-churn pre-poll p95 regression: measured_service_future_to_first_poll_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
        measured_service_future_first_poll_p95_ms,
        SERVICE_FUTURE_FIRST_POLL_P95_BUDGET_MS
    );
    assert!(
        measured_service_future_first_poll_max_ms <= SERVICE_FUTURE_FIRST_POLL_MAX_BUDGET_MS,
        "revision-churn pre-poll max regression: measured_service_future_to_first_poll_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
        measured_service_future_first_poll_max_ms,
        SERVICE_FUTURE_FIRST_POLL_MAX_BUDGET_MS
    );
    assert!(
        counter_delta("intellisense_v2_completion_route_total_route_head_hit")
            + counter_delta("intellisense_v2_completion_route_total_route_exact_hit")
            >= MEASURE_REQUESTS as u64,
        "expected measured revision-churn route counters to cover all samples, head_hit_total_delta={}, exact_hit_total_delta={}, counters={counters:?}",
        counter_delta("intellisense_v2_completion_route_total_route_head_hit"),
        counter_delta("intellisense_v2_completion_route_total_route_exact_hit")
    );

    live_transport_close_document(&mut harness, &uri).await;
    drop(server);
    harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}

#[test]
fn p40_real_conf_big_same_file_overlap_completion_perf_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p40 tokio runtime");
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

        fn completion_response_empty(response: &CompletionResponse) -> bool {
            match response {
                CompletionResponse::List(list) => list.items.is_empty(),
                CompletionResponse::Array(items) => items.is_empty(),
            }
        }

        let _env_lock = lock_test_env().await;
        let _response_build_delay_guard =
            EnvVarGuard::set("BSL_TEST_COMPLETION_RESPONSE_BUILD_DELAY_MS", "300");

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        const PROFILE_NAME: &str = "p40_real_conf_big_same_file_overlap_completion_perf_report_live";
        const WARMUP_REQUESTS: usize = 1;
        const MEASURE_REQUESTS: usize = 5;
        const OVERLAP_FIRST_POLL_BUDGET_MS: u64 = 250;
        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-completion-superseded-active-turn-release".to_string()
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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p40 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p40_real_conf_big_live_setup")
            .await;

        let uri = Url::from_file_path(&module_path).expect("real conf_big module uri");
        server
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: module_text.clone(),
                },
            })
            .await;

        server.sync_v2_globals().await;
        let file_id = server.get_or_create_file_id_v2(&uri).await;
        let opened_version = server
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
            .expect("latest received version for p40 opened file");
        assert_eq!(opened_version, 1, "real conf_big fixture must open at version 1");
        assert!(
            server
                .analysis_v2
                .wait_for_file_version(file_id, opened_version)
                .await,
            "analysis runtime must catch up to opened real conf_big file version"
        );

        let completion_position = find_utf16_position_after_marker(&module_text, "Объект.");
        let completion_context = Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        });

        let mut warmup_samples = Vec::new();
        for index in 0..WARMUP_REQUESTS {
            let request_id = 40_200_000_i64 + index as i64;
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
                "top_labels": labels.into_iter().take(8).collect::<Vec<_>>(),
                "version": opened_version,
            }));
        }

        let metrics_before_measured = coordinator.observability_metrics();
        let counters_before_measured = metrics_before_measured
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics_before_measured.counters object");
        crate::server::command_handlers::reset_get_current_context_parse_attempts_for_test();

        let parse_completion_response = |response: &serde_json::Value| {
            let result = response
                .get("result")
                .cloned()
                .expect("completion result field");
            serde_json::from_value::<Option<CompletionResponse>>(result)
                .expect("parse completion response")
                .expect("completion result present")
        };

        let mut measured_samples = Vec::new();
        for index in 0..MEASURE_REQUESTS {
            let first_request_id = 40_200_100_i64 + (index as i64 * 10);
            let second_request_id = first_request_id + 1;

            live_transport_write_completion_request(
                &mut harness,
                first_request_id,
                &uri,
                completion_position,
                completion_context.clone(),
            )
            .await;

            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    if server
                        .completion_cancellation_registry_v2
                        .get(&first_request_id.to_string())
                        .is_some()
                    {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("first overlap completion request must register");
            tokio::time::sleep(Duration::from_millis(50)).await;

            live_transport_write_completion_request(
                &mut harness,
                second_request_id,
                &uri,
                completion_position,
                completion_context.clone(),
            )
            .await;

            let (first_response, second_response) =
                tokio::time::timeout(Duration::from_secs(20), async {
                    let mut first_response = None;
                    let mut second_response = None;
                    loop {
                        let response = harness.read_message().await;
                        match response.get("id").and_then(|value| value.as_i64()) {
                            Some(id) if id == first_request_id => first_response = Some(response),
                            Some(id) if id == second_request_id => {
                                second_response = Some(response)
                            }
                            _ => {}
                        }
                        if first_response.is_some() && second_response.is_some() {
                            break (
                                first_response.take().expect("first overlap response"),
                                second_response.take().expect("second overlap response"),
                            );
                        }
                    }
                })
                .await
                .expect("both overlap completion responses must arrive");

            for _ in 0..80 {
                if server
                    .completion_cancellation_registry_v2
                    .get(&first_request_id.to_string())
                    .is_none()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }

            let first_completion = parse_completion_response(&first_response);
            let second_completion = parse_completion_response(&second_response);
            let second_labels = normalize_lsp_member_labels(&second_completion);
            measured_samples.push(serde_json::json!({
                "step": format!("measured_same_file_overlap_{}", index + 1),
                "first_request_id": first_request_id,
                "second_request_id": second_request_id,
                "first_response_empty": completion_response_empty(&first_completion),
                "second_label_count": second_labels.len(),
                "second_top_labels": second_labels.into_iter().take(8).collect::<Vec<_>>(),
                "first_registry_cleared": server
                    .completion_cancellation_registry_v2
                    .get(&first_request_id.to_string())
                    .is_none(),
            }));
        }

        let completion_timeline =
            live_transport_get_completion_timeline(&mut harness, 40_200_900, 160).await;
        let observability_metrics =
            live_transport_get_observability_metrics(&mut harness, 40_200_901).await;
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
            "expected non-empty completion timeline traces for p40 real conf_big overlap gate"
        );

        let counters = observability_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");

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
        let trace_summary = |trace: &serde_json::Value| {
            serde_json::json!({
                "request_id": trace.get("request_id").and_then(|value| value.as_str()),
                "outcome": trace.get("outcome").and_then(|value| value.as_str()),
                "route": completion_timeline_prepare_detail_str(trace, "route"),
                "fail_closed_cause": completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
                "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                "service_future_to_first_poll_wait_ms": completion_timeline_server_edge_u64(
                    trace,
                    "service_future_to_first_poll_wait_ms",
                ),
                "cancel_observed_after_handler_enter_ms": completion_timeline_server_edge_u64(
                    trace,
                    "cancel_observed_after_handler_enter_ms",
                ),
                "turn_wait_ms": completion_timeline_trace_stage_duration_ms(trace, "turn_wait"),
                "response_build_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build"),
                "response_build_other_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build_other"),
            })
        };

        let measured_samples = measured_samples
            .into_iter()
            .map(|sample| {
                let first_request_id = sample
                    .get("first_request_id")
                    .and_then(|value| value.as_i64())
                    .expect("first_request_id");
                let second_request_id = sample
                    .get("second_request_id")
                    .and_then(|value| value.as_i64())
                    .expect("second_request_id");
                let first_trace = filtered_traces.iter().find(|trace| {
                    trace.get("request_id").and_then(|value| value.as_str())
                        == Some(&first_request_id.to_string())
                });
                let second_trace = filtered_traces.iter().find(|trace| {
                    trace.get("request_id").and_then(|value| value.as_str())
                        == Some(&second_request_id.to_string())
                });

                let mut sample_object = sample
                    .as_object()
                    .cloned()
                    .expect("sample must be json object");
                sample_object.insert(
                    "first_trace".to_string(),
                    first_trace.map(trace_summary).unwrap_or(serde_json::json!(null)),
                );
                sample_object.insert(
                    "second_trace".to_string(),
                    second_trace.map(trace_summary).unwrap_or(serde_json::json!(null)),
                );
                serde_json::Value::Object(sample_object)
            })
            .collect::<Vec<_>>();

        let counter_delta = |name: &str| -> u64 {
            read_u64_metric(counters.get(name))
                .saturating_sub(read_u64_metric(counters_before_measured.get(name)))
        };
        let sample_histogram = |field: &str| {
            let values = measured_samples
                .iter()
                .filter_map(|sample| {
                    sample
                        .get("second_trace")
                        .and_then(|trace| trace.get(field))
                        .and_then(|value| value.as_u64())
                })
                .map(|value| value as f64)
                .collect::<Vec<_>>();
            sample_histogram_value(&values)
        };

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
        let measured_trace_linked_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample.get("first_trace").is_some_and(|trace| !trace.is_null())
                    && sample.get("second_trace").is_some_and(|trace| !trace.is_null())
            })
            .count();
        let measured_first_empty_response_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("first_response_empty")
                    .and_then(|value| value.as_bool())
                    == Some(true)
            })
            .count();
        let measured_first_registry_cleared_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("first_registry_cleared")
                    .and_then(|value| value.as_bool())
                    == Some(true)
            })
            .count();
        let measured_second_non_empty_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("second_label_count")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
                    > 0
            })
            .count();
        let measured_first_cancelled_or_superseded_traces = measured_samples
            .iter()
            .filter(|sample| {
                matches!(
                    sample
                        .get("first_trace")
                        .and_then(|trace| trace.get("outcome"))
                        .and_then(|value| value.as_str()),
                    Some("cancelled" | "superseded")
                )
            })
            .count();
        let measured_head_hit_traces = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("second_trace")
                    .and_then(|trace| trace.get("route"))
                    .and_then(|value| value.as_str())
                    == Some("head_hit")
            })
            .count();
        let measured_exact_hit_traces = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("second_trace")
                    .and_then(|trace| trace.get("route"))
                    .and_then(|value| value.as_str())
                    == Some("exact_hit")
            })
            .count();
        let measured_second_first_poll_histogram = sample_histogram("service_future_to_first_poll_wait_ms");
        let measured_second_first_poll_max_ms = measured_samples
            .iter()
            .filter_map(|sample| {
                sample
                    .get("second_trace")
                    .and_then(|trace| trace.get("service_future_to_first_poll_wait_ms"))
                    .and_then(|value| value.as_u64())
            })
            .max()
            .unwrap_or(0);

        let latest_trace_summaries = filtered_traces
            .iter()
            .rev()
            .take(MEASURE_REQUESTS * 2 + WARMUP_REQUESTS)
            .map(trace_summary)
            .collect::<Vec<_>>();

        let report = serde_json::json!({
            "change_id": change_id,
            "profile": PROFILE_NAME,
            "schema_version": 1,
            "configuration_path": conf_big_root,
            "module_path": module_path,
            "marker": "Объект.",
            "request_plan": {
                "cache_mode": "self_warmed_same_process",
                "warmup_requests": WARMUP_REQUESTS,
                "measured_requests": MEASURE_REQUESTS,
                "transport_path": "tower_lsp_server_serve_duplex",
                "profile_kind": "same-file-overlap",
                "response_build_delay_ms": 300,
                "completion_trigger_mode": "invoked",
            },
            "warmup_samples": warmup_samples,
            "measured_samples": measured_samples,
            "summary": {
                "trace_count_for_uri": filtered_traces.len(),
                "trace_matching_mode": trace_matching_mode,
                "trace_request_id_present_total": trace_request_id_present_total,
                "warmup_non_empty_samples": warmup_non_empty_samples,
                "measured_trace_linked_samples": measured_trace_linked_samples,
                "measured_first_empty_response_samples": measured_first_empty_response_samples,
                "measured_first_registry_cleared_samples": measured_first_registry_cleared_samples,
                "measured_first_cancelled_or_superseded_traces": measured_first_cancelled_or_superseded_traces,
                "measured_second_non_empty_samples": measured_second_non_empty_samples,
                "measured_head_hit_traces": measured_head_hit_traces,
                "measured_exact_hit_traces": measured_exact_hit_traces,
                "measured_prepare_timeout_total_delta": counter_delta(
                    "intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout"
                ),
                "measured_exact_deadline_total_delta": counter_delta(
                    "intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline"
                ),
                "measured_cancelled_total_delta": counter_delta(
                    "intellisense_v2_completion_result_total_cancelled"
                ),
                "measured_fail_closed_total_delta": counter_delta(
                    "intellisense_v2_completion_result_total_fail_closed"
                ),
                "measured_service_future_to_first_poll_wait_ms": measured_second_first_poll_histogram,
                "measured_service_future_to_first_poll_wait_max_ms": measured_second_first_poll_max_ms,
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

        let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_OVERLAP_COMPLETION_PERF_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!(
                        "{change_id}-real-conf-big-overlap-completion-perf-live.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p40 real conf_big overlap report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report).expect("serialize p40 real conf_big overlap report"),
        )
        .expect("write p40 real conf_big overlap report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        assert!(
            warmup_non_empty_samples == WARMUP_REQUESTS,
            "expected warmup completion samples to be non-empty before overlap profile, warmup_samples={warmup_samples:?}"
        );
        assert!(
            trace_matching_mode == "request_id",
            "expected request-context parity to expose JSON-RPC request ids in overlap trace set, trace_matching_mode={}, filtered_traces={filtered_traces:?}",
            trace_matching_mode
        );
        assert!(
            measured_trace_linked_samples == MEASURE_REQUESTS,
            "expected every measured overlap sample to link both first and second traces, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_first_empty_response_samples == MEASURE_REQUESTS,
            "older overlap request must always terminate with bounded empty response, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_first_registry_cleared_samples == MEASURE_REQUESTS,
            "older overlap request must always clear cancellation registry entry, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_first_cancelled_or_superseded_traces == MEASURE_REQUESTS,
            "older overlap trace must always terminate with cancelled/superseded outcome, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_second_non_empty_samples == MEASURE_REQUESTS,
            "newer overlap request must return non-empty completion labels on representative module, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_head_hit_traces + measured_exact_hit_traces >= MEASURE_REQUESTS,
            "newer overlap traces must preserve route attribution, measured_head_hit_traces={}, measured_exact_hit_traces={}, measured_samples={measured_samples:?}",
            measured_head_hit_traces,
            measured_exact_hit_traces
        );
        assert!(
            counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout") == 0
                && counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline") == 0,
            "overlap gate must not regress into prepare_timeout/exact_deadline, counters={counters:?}"
        );
        assert!(
            measured_second_first_poll_max_ms <= OVERLAP_FIRST_POLL_BUDGET_MS,
            "newer same-file completion must reach first poll within overlap budget, measured_second_first_poll_max_ms={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_second_first_poll_max_ms,
            OVERLAP_FIRST_POLL_BUDGET_MS
        );

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}

#[test]
fn p41_real_conf_big_pre_active_turn_wait_overlap_completion_perf_report_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p41 tokio runtime");
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

        fn completion_response_empty(response: &CompletionResponse) -> bool {
            match response {
                CompletionResponse::List(list) => list.items.is_empty(),
                CompletionResponse::Array(items) => items.is_empty(),
            }
        }

        let _env_lock = lock_test_env().await;

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        const PROFILE_NAME: &str =
            "p41_real_conf_big_pre_active_turn_wait_overlap_completion_perf_report_live";
        const WARMUP_REQUESTS: usize = 1;
        const MEASURE_REQUESTS: usize = 5;
        const OVERLAP_FIRST_POLL_BUDGET_MS: u64 = 250;
        const STRANDED_PRE_ACTIVE_TURN_WAIT_AGE_BUDGET_MS: u64 = 500;
        const PRE_ACTIVE_TURN_WAIT_DELAY_MS: u64 = 300;
        let change_id = std::env::var("CHANGE_ID")
            .unwrap_or_else(|_| "refactor-completion-turn-wait-slot-release".to_string());

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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p41 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p41_real_conf_big_live_setup")
            .await;

        let uri = Url::from_file_path(&module_path).expect("real conf_big module uri");
        server
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: module_text.clone(),
                },
            })
            .await;

        server.sync_v2_globals().await;
        let file_id = server.get_or_create_file_id_v2(&uri).await;
        let opened_version = server
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
            .expect("latest received version for p41 opened file");
        assert_eq!(opened_version, 1, "real conf_big fixture must open at version 1");
        assert!(
            server
                .analysis_v2
                .wait_for_file_version(file_id, opened_version)
                .await,
            "analysis runtime must catch up to opened real conf_big file version"
        );

        let completion_position = find_utf16_position_after_marker(&module_text, "Объект.");
        let completion_context = Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        });

        let mut warmup_samples = Vec::new();
        for index in 0..WARMUP_REQUESTS {
            let request_id = 40_300_000_i64 + index as i64;
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
                "top_labels": labels.into_iter().take(8).collect::<Vec<_>>(),
                "version": opened_version,
            }));
        }

        let metrics_before_measured = coordinator.observability_metrics();
        let counters_before_measured = metrics_before_measured
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics_before_measured.counters object");

        let parse_completion_response = |response: &serde_json::Value| {
            let result = response
                .get("result")
                .cloned()
                .expect("completion result field");
            serde_json::from_value::<Option<CompletionResponse>>(result)
                .expect("parse completion response")
                .expect("completion result present")
        };

        let mut measured_samples = Vec::new();
        for index in 0..MEASURE_REQUESTS {
            crate::server::language_server::reset_completion_checkpoint_hits_for_test();
            let checkpoint_delay_guard = EnvVarGuard::set(
                "BSL_TEST_COMPLETION_CHECKPOINT_DELAYS",
                "before_active_turn_registration=300",
            );
            let first_request_id = 40_300_100_i64 + (index as i64 * 10);
            let second_request_id = first_request_id + 1;

            live_transport_write_completion_request(
                &mut harness,
                first_request_id,
                &uri,
                completion_position,
                completion_context.clone(),
            )
            .await;

            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    if server
                        .completion_cancellation_registry_v2
                        .get(&first_request_id.to_string())
                        .is_some()
                    {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("first pre-active overlap completion request must register");
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    if crate::server::language_server::completion_checkpoint_hits_for_test(
                        "before_active_turn_registration",
                    ) >= 1
                    {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("first pre-active overlap completion request must reach pre-active checkpoint");
            drop(checkpoint_delay_guard);

            live_transport_write_completion_request(
                &mut harness,
                second_request_id,
                &uri,
                completion_position,
                completion_context.clone(),
            )
            .await;

            let (first_response, second_response) =
                tokio::time::timeout(Duration::from_secs(20), async {
                    let mut first_response = None;
                    let mut second_response = None;
                    loop {
                        let response = harness.read_message().await;
                        match response.get("id").and_then(|value| value.as_i64()) {
                            Some(id) if id == first_request_id => first_response = Some(response),
                            Some(id) if id == second_request_id => {
                                second_response = Some(response)
                            }
                            _ => {}
                        }
                        if first_response.is_some() && second_response.is_some() {
                            break (
                                first_response.take().expect("first overlap response"),
                                second_response.take().expect("second overlap response"),
                            );
                        }
                    }
                })
                .await
                .expect("both pre-active overlap completion responses must arrive");

            for _ in 0..80 {
                if server
                    .completion_cancellation_registry_v2
                    .get(&first_request_id.to_string())
                    .is_none()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }

            let first_completion = parse_completion_response(&first_response);
            let second_completion = parse_completion_response(&second_response);
            let second_labels = normalize_lsp_member_labels(&second_completion);
            measured_samples.push(serde_json::json!({
                "step": format!("measured_pre_active_overlap_{}", index + 1),
                "first_request_id": first_request_id,
                "second_request_id": second_request_id,
                "first_response_empty": completion_response_empty(&first_completion),
                "second_label_count": second_labels.len(),
                "second_top_labels": second_labels.into_iter().take(8).collect::<Vec<_>>(),
                "first_registry_cleared": server
                    .completion_cancellation_registry_v2
                    .get(&first_request_id.to_string())
                    .is_none(),
            }));
        }

        let completion_timeline =
            live_transport_get_completion_timeline(&mut harness, 40_300_900, 160).await;
        let observability_metrics =
            live_transport_get_observability_metrics(&mut harness, 40_300_901).await;
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
            "expected non-empty completion timeline traces for p41 real conf_big pre-active overlap gate"
        );

        let counters = observability_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");

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
        let pre_active_turn_wait_contender_age_ms = |trace: &serde_json::Value| {
            trace
                .get("server_edge_details")
                .and_then(|details| details.get("first_poll_contention_contenders"))
                .and_then(|value| value.as_array())
                .and_then(|contenders| {
                    contenders.iter().find_map(|contender| {
                        let request_class =
                            contender.get("request_class").and_then(|value| value.as_str());
                        let phase = contender.get("phase").and_then(|value| value.as_str());
                        if request_class == Some("completion") && phase == Some("turn_wait") {
                            contender.get("age_ms").and_then(|value| value.as_u64())
                        } else {
                            None
                        }
                    })
                })
        };
        let trace_summary = |trace: &serde_json::Value| {
            serde_json::json!({
                "request_id": trace.get("request_id").and_then(|value| value.as_str()),
                "outcome": trace.get("outcome").and_then(|value| value.as_str()),
                "route": completion_timeline_prepare_detail_str(trace, "route"),
                "fail_closed_cause": completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
                "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                "transport_slot_released_at_ms": completion_timeline_server_edge_u64(
                    trace,
                    "transport_slot_released_at_ms",
                ),
                "transport_to_slot_release_wait_ms": completion_timeline_server_edge_u64(
                    trace,
                    "transport_to_slot_release_wait_ms",
                ),
                "slot_release_to_handler_wait_ms": completion_timeline_server_edge_u64(
                    trace,
                    "slot_release_to_handler_wait_ms",
                ),
                "service_future_to_first_poll_wait_ms": completion_timeline_server_edge_u64(
                    trace,
                    "service_future_to_first_poll_wait_ms",
                ),
                "pre_active_turn_wait_contender_age_ms": pre_active_turn_wait_contender_age_ms(trace),
                "turn_wait_entered_at_ms": trace
                    .get("turn_attribution")
                    .and_then(|value| value.get("turn_wait_entered_at_ms"))
                    .and_then(|value| value.as_u64()),
                "turn_wait_outcome": trace
                    .get("turn_attribution")
                    .and_then(|value| value.get("turn_wait_outcome"))
                    .and_then(|value| value.as_str()),
                "turn_wait_ms": completion_timeline_trace_stage_duration_ms(trace, "turn_wait"),
            })
        };

        let measured_samples = measured_samples
            .into_iter()
            .map(|sample| {
                let first_request_id = sample
                    .get("first_request_id")
                    .and_then(|value| value.as_i64())
                    .expect("first_request_id");
                let second_request_id = sample
                    .get("second_request_id")
                    .and_then(|value| value.as_i64())
                    .expect("second_request_id");
                let first_trace = filtered_traces.iter().find(|trace| {
                    trace.get("request_id").and_then(|value| value.as_str())
                        == Some(&first_request_id.to_string())
                });
                let second_trace = filtered_traces.iter().find(|trace| {
                    trace.get("request_id").and_then(|value| value.as_str())
                        == Some(&second_request_id.to_string())
                });

                let mut sample_object = sample
                    .as_object()
                    .cloned()
                    .expect("sample must be json object");
                sample_object.insert(
                    "first_trace".to_string(),
                    first_trace.map(trace_summary).unwrap_or(serde_json::json!(null)),
                );
                sample_object.insert(
                    "second_trace".to_string(),
                    second_trace.map(trace_summary).unwrap_or(serde_json::json!(null)),
                );
                serde_json::Value::Object(sample_object)
            })
            .collect::<Vec<_>>();

        let counter_delta = |name: &str| -> u64 {
            read_u64_metric(counters.get(name))
                .saturating_sub(read_u64_metric(counters_before_measured.get(name)))
        };
        let sample_histogram = |field: &str| {
            let values = measured_samples
                .iter()
                .filter_map(|sample| {
                    sample
                        .get("second_trace")
                        .and_then(|trace| trace.get(field))
                        .and_then(|value| value.as_u64())
                })
                .map(|value| value as f64)
                .collect::<Vec<_>>();
            sample_histogram_value(&values)
        };

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
        let measured_trace_linked_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample.get("first_trace").is_some_and(|trace| !trace.is_null())
                    && sample.get("second_trace").is_some_and(|trace| !trace.is_null())
            })
            .count();
        let measured_first_empty_response_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("first_response_empty")
                    .and_then(|value| value.as_bool())
                    == Some(true)
            })
            .count();
        let measured_first_registry_cleared_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("first_registry_cleared")
                    .and_then(|value| value.as_bool())
                    == Some(true)
            })
            .count();
        let measured_first_cancelled_or_superseded_traces = measured_samples
            .iter()
            .filter(|sample| {
                matches!(
                    sample
                        .get("first_trace")
                        .and_then(|trace| trace.get("outcome"))
                        .and_then(|value| value.as_str()),
                    Some("cancelled" | "superseded")
                )
            })
            .count();
        let measured_first_pre_active_turn_wait_ready_traces = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("first_trace")
                    .and_then(|trace| trace.get("turn_wait_outcome"))
                    .and_then(|value| value.as_str())
                    == Some("ready")
            })
            .count();
        let measured_second_non_empty_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("second_label_count")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
                    > 0
            })
            .count();
        let measured_head_hit_traces = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("second_trace")
                    .and_then(|trace| trace.get("route"))
                    .and_then(|value| value.as_str())
                    == Some("head_hit")
            })
            .count();
        let measured_exact_hit_traces = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("second_trace")
                    .and_then(|trace| trace.get("route"))
                    .and_then(|value| value.as_str())
                    == Some("exact_hit")
            })
            .count();
        let measured_stranded_pre_active_turn_wait_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("second_trace")
                    .and_then(|trace| trace.get("pre_active_turn_wait_contender_age_ms"))
                    .and_then(|value| value.as_u64())
                    .is_some_and(|age_ms| age_ms > STRANDED_PRE_ACTIVE_TURN_WAIT_AGE_BUDGET_MS)
            })
            .count();
        let measured_second_transport_slot_released_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("second_trace")
                    .and_then(|trace| trace.get("transport_slot_released_at_ms"))
                    .and_then(|value| value.as_u64())
                    .is_some()
            })
            .count();
        let measured_second_slot_release_before_turn_wait_samples = measured_samples
            .iter()
            .filter(|sample| {
                let second_trace = sample.get("second_trace");
                let transport_slot_released_at_ms = second_trace
                    .and_then(|trace| trace.get("transport_slot_released_at_ms"))
                    .and_then(|value| value.as_u64());
                let turn_wait_entered_at_ms = second_trace
                    .and_then(|trace| trace.get("turn_wait_entered_at_ms"))
                    .and_then(|value| value.as_u64());
                matches!(
                    (transport_slot_released_at_ms, turn_wait_entered_at_ms),
                    (Some(slot_release_ms), Some(turn_wait_entered_ms))
                        if slot_release_ms <= turn_wait_entered_ms
                )
            })
            .count();
        let measured_second_first_poll_histogram =
            sample_histogram("service_future_to_first_poll_wait_ms");
        let measured_second_first_poll_max_ms = measured_samples
            .iter()
            .filter_map(|sample| {
                sample
                    .get("second_trace")
                    .and_then(|trace| trace.get("service_future_to_first_poll_wait_ms"))
                    .and_then(|value| value.as_u64())
            })
            .max()
            .unwrap_or(0);
        let measured_second_transport_to_slot_release_histogram =
            sample_histogram("transport_to_slot_release_wait_ms");
        let measured_second_transport_to_slot_release_max_ms = measured_samples
            .iter()
            .filter_map(|sample| {
                sample
                    .get("second_trace")
                    .and_then(|trace| trace.get("transport_to_slot_release_wait_ms"))
                    .and_then(|value| value.as_u64())
            })
            .max()
            .unwrap_or(0);

        let latest_trace_summaries = filtered_traces
            .iter()
            .rev()
            .take(MEASURE_REQUESTS * 2 + WARMUP_REQUESTS)
            .map(trace_summary)
            .collect::<Vec<_>>();

        let report = serde_json::json!({
            "change_id": change_id,
            "profile": PROFILE_NAME,
            "schema_version": 1,
            "configuration_path": conf_big_root,
            "module_path": module_path,
            "marker": "Объект.",
            "request_plan": {
                "cache_mode": "self_warmed_same_process",
                "warmup_requests": WARMUP_REQUESTS,
                "measured_requests": MEASURE_REQUESTS,
                "transport_path": "tower_lsp_server_serve_duplex",
                "profile_kind": "same-file-pre-active-overlap",
                "pre_active_turn_wait_delay_ms": PRE_ACTIVE_TURN_WAIT_DELAY_MS,
                "completion_trigger_mode": "invoked",
            },
            "warmup_samples": warmup_samples,
            "measured_samples": measured_samples,
            "summary": {
                "trace_count_for_uri": filtered_traces.len(),
                "trace_matching_mode": trace_matching_mode,
                "trace_request_id_present_total": trace_request_id_present_total,
                "warmup_non_empty_samples": warmup_non_empty_samples,
                "measured_trace_linked_samples": measured_trace_linked_samples,
                "measured_first_empty_response_samples": measured_first_empty_response_samples,
                "measured_first_registry_cleared_samples": measured_first_registry_cleared_samples,
                "measured_first_cancelled_or_superseded_traces": measured_first_cancelled_or_superseded_traces,
                "measured_first_pre_active_turn_wait_ready_traces": measured_first_pre_active_turn_wait_ready_traces,
                "measured_second_non_empty_samples": measured_second_non_empty_samples,
                "measured_head_hit_traces": measured_head_hit_traces,
                "measured_exact_hit_traces": measured_exact_hit_traces,
                "measured_stranded_pre_active_turn_wait_samples": measured_stranded_pre_active_turn_wait_samples,
                "measured_second_transport_slot_released_samples": measured_second_transport_slot_released_samples,
                "measured_second_slot_release_before_turn_wait_samples": measured_second_slot_release_before_turn_wait_samples,
                "measured_prepare_timeout_total_delta": counter_delta(
                    "intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout"
                ),
                "measured_exact_deadline_total_delta": counter_delta(
                    "intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline"
                ),
                "measured_cancelled_total_delta": counter_delta(
                    "intellisense_v2_completion_result_total_cancelled"
                ),
                "measured_fail_closed_total_delta": counter_delta(
                    "intellisense_v2_completion_result_total_fail_closed"
                ),
                "measured_transport_to_slot_release_wait_ms": measured_second_transport_to_slot_release_histogram,
                "measured_transport_to_slot_release_wait_max_ms": measured_second_transport_to_slot_release_max_ms,
                "measured_service_future_to_first_poll_wait_ms": measured_second_first_poll_histogram,
                "measured_service_future_to_first_poll_wait_max_ms": measured_second_first_poll_max_ms,
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

        let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_PRE_ACTIVE_OVERLAP_COMPLETION_PERF_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!(
                        "{change_id}-real-conf-big-pre-active-overlap-completion-perf-live.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p41 real conf_big pre-active overlap report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p41 real conf_big pre-active overlap report"),
        )
        .expect("write p41 real conf_big pre-active overlap report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        assert!(
            warmup_non_empty_samples == WARMUP_REQUESTS,
            "expected warmup completion samples to be non-empty before pre-active overlap profile, warmup_samples={warmup_samples:?}"
        );
        assert!(
            trace_matching_mode == "request_id",
            "expected request-context parity to expose JSON-RPC request ids in pre-active overlap trace set, trace_matching_mode={}, filtered_traces={filtered_traces:?}",
            trace_matching_mode
        );
        assert!(
            measured_trace_linked_samples == MEASURE_REQUESTS,
            "expected every measured pre-active overlap sample to link both first and second traces, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_first_empty_response_samples == MEASURE_REQUESTS,
            "older pre-active overlap request must always terminate with bounded empty response, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_first_registry_cleared_samples == MEASURE_REQUESTS,
            "older pre-active overlap request must always clear cancellation registry entry, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_first_cancelled_or_superseded_traces == MEASURE_REQUESTS,
            "older pre-active overlap trace must always terminate with cancelled/superseded outcome, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_first_pre_active_turn_wait_ready_traces == MEASURE_REQUESTS,
            "older pre-active overlap trace must prove that request had already exited queue before supersession, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_second_non_empty_samples == MEASURE_REQUESTS,
            "newer pre-active overlap request must return non-empty completion labels on representative module, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_head_hit_traces + measured_exact_hit_traces >= MEASURE_REQUESTS,
            "newer pre-active overlap traces must preserve route attribution, measured_head_hit_traces={}, measured_exact_hit_traces={}, measured_samples={measured_samples:?}",
            measured_head_hit_traces,
            measured_exact_hit_traces
        );
        assert!(
            counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout") == 0
                && counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline") == 0,
            "pre-active overlap gate must not regress into prepare_timeout/exact_deadline, counters={counters:?}"
        );
        assert!(
            measured_stranded_pre_active_turn_wait_samples == 0,
            "pre-active overlap gate must fail on stranded completion contender in phase=turn_wait beyond bounded age, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_second_transport_slot_released_samples == MEASURE_REQUESTS,
            "pre-active overlap gate must prove that every newer same-file request recorded transport_slot_released_at_ms before passive wait, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_second_slot_release_before_turn_wait_samples == MEASURE_REQUESTS,
            "pre-active overlap gate must fail when turn_wait starts before transport_slot_released_at_ms, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_second_transport_to_slot_release_max_ms <= OVERLAP_FIRST_POLL_BUDGET_MS,
            "newer same-file completion must release transport slot within pre-active overlap budget, measured_second_transport_to_slot_release_max_ms={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_second_transport_to_slot_release_max_ms,
            OVERLAP_FIRST_POLL_BUDGET_MS
        );
        assert!(
            measured_second_first_poll_max_ms <= OVERLAP_FIRST_POLL_BUDGET_MS,
            "newer same-file completion must reach first poll within pre-active overlap budget, measured_second_first_poll_max_ms={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_second_first_poll_max_ms,
            OVERLAP_FIRST_POLL_BUDGET_MS
        );

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}
