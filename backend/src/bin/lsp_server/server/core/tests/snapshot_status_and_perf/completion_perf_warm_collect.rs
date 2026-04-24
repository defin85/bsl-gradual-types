#[test]
fn p42_real_conf_big_warm_non_member_collect_breakdown_gate_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p42 warm non-member tokio runtime");
    runtime.block_on(async {
        init_test_tracing();

        let _env_lock = lock_test_env().await;
        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        const PROFILE_NAME: &str =
            "p42_real_conf_big_warm_non_member_collect_breakdown_gate_live";
        const PROBE_NAME: &str = "Refactor13WarmNonMemberCollectBreakdownProbe";
        const WARMUP_REQUESTS: usize = 1;
        const MEASURE_REQUESTS: usize = 10;
        const WARM_NON_MEMBER_COLLECT_P95_BUDGET_MS: f64 = 40.0;
        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-13-non-member-completion-catalog-precompute".to_string()
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
            .expect("read conf_big module text for warm non-member report");
        let completion_marker = format!("Процедура {PROBE_NAME}() Экспорт\n    ЭтотОбъек");
        let appended_probe = format!(
            "\n#Область {PROBE_NAME}\n&НаСервере\nПроцедура {PROBE_NAME}() Экспорт\n    ЭтотОбъек\nКонецПроцедуры\n#КонецОбласти\n"
        );
        let current_text = format!("{module_text}{appended_probe}");
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
            "p42_real_conf_big_warm_non_member_live_setup",
        )
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
                        text: current_text.clone(),
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
            .expect("latest received version for warm non-member opened file");
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
        let _ = server.analysis_v2.snapshot().await.ir(file_id);

        let completion_position = find_utf16_position_after_marker(&current_text, &completion_marker);
        let completion_context = Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        });

        let mut warmup_samples = Vec::new();
        for index in 0..WARMUP_REQUESTS {
            let request_id = 42_200_000_i64 + index as i64;
            let started = Instant::now();
            let completion_response = live_transport_completion_response_with_request(
                &mut harness,
                request_id,
                &uri,
                completion_position,
                completion_context.clone(),
            )
            .await;
            let labels = completion_item_labels_from_jsonrpc_response(&completion_response);
            warmup_samples.push(serde_json::json!({
                "step": format!("warmup_non_member_completion_{}", index + 1),
                "request_id": request_id,
                "elapsed_ms": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
                "label_count": labels.len(),
                "labels": labels,
                "version": opened_version,
            }));
        }

        let metrics_before_measured = coordinator.observability_metrics();
        let counters_before_measured = metrics_before_measured
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics_before_measured.counters object");

        let mut measured_samples = Vec::new();
        for index in 0..MEASURE_REQUESTS {
            let request_id = 42_200_100_i64 + index as i64;
            let started = Instant::now();
            let completion_response = live_transport_completion_response_with_request(
                &mut harness,
                request_id,
                &uri,
                completion_position,
                completion_context.clone(),
            )
            .await;
            let labels = completion_item_labels_from_jsonrpc_response(&completion_response);
            measured_samples.push(serde_json::json!({
                "step": format!("measured_warm_non_member_completion_{}", index + 1),
                "request_id": request_id,
                "elapsed_ms": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
                "label_count": labels.len(),
                "labels": labels,
                "version": opened_version,
            }));
        }

        let completion_timeline = live_transport_get_completion_timeline(&mut harness, 42_200_900, 64).await;
        let observability_metrics =
            live_transport_get_observability_metrics(&mut harness, 42_200_901).await;
        let timeline_traces = completion_timeline
            .get("traces")
            .and_then(|value| value.as_array())
            .expect("completion timeline traces array");
        let tracked_request_ids = warmup_samples
            .iter()
            .chain(measured_samples.iter())
            .filter_map(|sample| {
                sample
                    .get("request_id")
                    .and_then(|value| value.as_i64())
                    .map(|value| value.to_string())
            })
            .collect::<BTreeSet<_>>();
        let filtered_traces = timeline_traces
            .iter()
            .filter(|trace| {
                trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                    && trace
                        .get("request_id")
                        .and_then(|value| value.as_str())
                        .is_some_and(|request_id| tracked_request_ids.contains(request_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            !filtered_traces.is_empty(),
            "expected non-empty completion timeline traces for warm non-member gate"
        );

        let histograms = observability_metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");
        let counters = observability_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");

        let trace_request_id_present_total = filtered_traces.len();
        let trace_matching_mode = "request_id";
        let trace_collect_breakdown = |trace: &serde_json::Value| {
            trace
                .get("collect_breakdown")
                .cloned()
                .unwrap_or(serde_json::Value::Null)
        };
        let trace_summaries_by_request_id = filtered_traces
            .iter()
            .filter_map(|trace| {
                let request_id = trace
                    .get("request_id")
                    .and_then(|value| value.as_str())?
                    .to_string();
                Some((
                    request_id,
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
                        "prepare_stateful_ms": completion_timeline_trace_stage_duration_ms(trace, "prepare_stateful"),
                        "query_bundle": completion_timeline_query_bundle_breakdown(trace),
                        "snapshot_read_ms": completion_timeline_trace_stage_duration_ms(trace, "snapshot_read"),
                        "collect_ms": completion_timeline_trace_stage_duration_ms(trace, "collect"),
                        "collect_breakdown": trace_collect_breakdown(trace),
                        "rank_ms": completion_timeline_trace_stage_duration_ms(trace, "rank"),
                        "response_build_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build"),
                        "format_ms": completion_timeline_trace_stage_duration_ms(trace, "format"),
                    }),
                ))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let enrich_samples = |samples: Vec<serde_json::Value>| -> Vec<serde_json::Value> {
            samples
                .into_iter()
                .map(|sample| {
                    let request_id = sample
                        .get("request_id")
                        .and_then(|value| value.as_i64())
                        .map(|value| value.to_string());
                    let trace = request_id
                        .as_ref()
                        .and_then(|request_id| trace_summaries_by_request_id.get(request_id))
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let mut sample_object = sample
                        .as_object()
                        .cloned()
                        .expect("sample must be json object");
                    sample_object.insert("trace".to_string(), trace);
                    serde_json::Value::Object(sample_object)
                })
                .collect::<Vec<_>>()
        };
        let warmup_samples = enrich_samples(warmup_samples);
        let measured_samples = enrich_samples(measured_samples);
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
                    "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                    "dominant_stage": trace.get("dominant_stage").and_then(|value| value.as_str()),
                    "prepare_stateful_ms": completion_timeline_trace_stage_duration_ms(trace, "prepare_stateful"),
                    "query_bundle": completion_timeline_query_bundle_breakdown(trace),
                    "snapshot_read_ms": completion_timeline_trace_stage_duration_ms(trace, "snapshot_read"),
                    "collect_ms": completion_timeline_trace_stage_duration_ms(trace, "collect"),
                    "collect_breakdown": trace_collect_breakdown(trace),
                    "rank_ms": completion_timeline_trace_stage_duration_ms(trace, "rank"),
                    "response_build_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build"),
                    "format_ms": completion_timeline_trace_stage_duration_ms(trace, "format"),
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
        let sample_trace_collect_breakdown_histogram = |samples: &[serde_json::Value], field: &str| {
            let values = samples
                .iter()
                .filter_map(|sample| {
                    sample
                        .get("trace")
                        .and_then(|trace| trace.get("collect_breakdown"))
                        .and_then(|value| value.get(field))
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
        let measured_contains_this_object_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("labels")
                    .and_then(|value| value.as_array())
                    .is_some_and(|labels| {
                        labels.iter().any(|label| label.as_str() == Some("ЭтотОбъект"))
                    })
            })
            .count();
        let measured_trace_linked_samples = measured_samples
            .iter()
            .filter(|sample| sample.get("trace").is_some_and(|trace| !trace.is_null()))
            .count();
        let measured_collect_breakdown_linked_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("collect_breakdown"))
                    .is_some_and(|value| value.is_object())
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
        let warmup_latency_histogram = sample_elapsed_histogram(&warmup_samples);
        let measured_latency_histogram = sample_elapsed_histogram(&measured_samples);
        let measured_collect_histogram =
            sample_trace_histogram(&measured_samples, "collect_ms");
        let raw_collect_observability_histogram =
            histogram_metric_value(histograms, "completion_stage_collect_ms", None);
        let measured_collect_breakdown = serde_json::json!({
            "non_member_local_symbols": sample_trace_collect_breakdown_histogram(
                &measured_samples,
                "non_member_local_symbols_ms"
            ),
            "non_member_contextual_symbols": sample_trace_collect_breakdown_histogram(
                &measured_samples,
                "non_member_contextual_symbols_ms"
            ),
            "non_member_module_routines": sample_trace_collect_breakdown_histogram(
                &measured_samples,
                "non_member_module_routines_ms"
            ),
            "non_member_global_functions": sample_trace_collect_breakdown_histogram(
                &measured_samples,
                "non_member_global_functions_ms"
            ),
            "non_member_metadata_items": sample_trace_collect_breakdown_histogram(
                &measured_samples,
                "non_member_metadata_items_ms"
            ),
            "non_member_repository_types": sample_trace_collect_breakdown_histogram(
                &measured_samples,
                "non_member_repository_types_ms"
            ),
            "non_member_keywords": sample_trace_collect_breakdown_histogram(
                &measured_samples,
                "non_member_keywords_ms"
            ),
        });
        let measured_collect_p95_ms = read_numeric_metric(measured_collect_histogram.get("p95"));

        let report = serde_json::json!({
            "change_id": change_id,
            "profile": PROFILE_NAME,
            "schema_version": 1,
            "configuration_path": conf_big_root,
            "module_path": module_path,
            "marker": completion_marker,
            "request_plan": {
                "cache_mode": "same_version_warm_non_member_repeat",
                "wait_for_current_revision": true,
                "current_completion_head_seed_mode": "warmup_request",
                "warmup_requests": WARMUP_REQUESTS,
                "measured_requests": MEASURE_REQUESTS,
                "completion_trigger_mode": "invoked",
                "completion_position_kind": "non_member_identifier_tail",
                "probe_strategy": "append_exported_form_procedure",
            },
            "warmup_samples": warmup_samples,
            "measured_samples": measured_samples,
            "summary": {
                "trace_count_for_requests": filtered_traces.len(),
                "trace_matching_mode": trace_matching_mode,
                "trace_request_id_present_total": trace_request_id_present_total,
                "warmup_non_empty_samples": warmup_non_empty_samples,
                "measured_non_empty_samples": measured_non_empty_samples,
                "measured_contains_this_object_samples": measured_contains_this_object_samples,
                "measured_trace_linked_samples": measured_trace_linked_samples,
                "measured_collect_breakdown_linked_samples": measured_collect_breakdown_linked_samples,
                "measured_ok_non_empty_traces": measured_ok_non_empty_traces,
                "measured_fail_closed_traces": measured_fail_closed_traces,
                "measured_completion_total_delta": counter_delta("completion_total"),
                "measured_fail_closed_total_delta": counter_delta(
                    "intellisense_v2_completion_result_total_fail_closed"
                ),
                "warmup_latency_ms": warmup_latency_histogram,
                "measured_latency_ms": measured_latency_histogram,
                "measured_collect_budget_p95_ms": WARM_NON_MEMBER_COLLECT_P95_BUDGET_MS,
                "measured_collect_ms": measured_collect_histogram,
                "raw_collect_observability_ms": raw_collect_observability_histogram,
                "measured_snapshot_read_ms": sample_trace_histogram(
                    &measured_samples,
                    "snapshot_read_ms"
                ),
                "measured_rank_ms": sample_trace_histogram(
                    &measured_samples,
                    "rank_ms"
                ),
                "measured_format_ms": sample_trace_histogram(
                    &measured_samples,
                    "format_ms"
                ),
                "measured_collect_breakdown_ms": measured_collect_breakdown,
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

        let report_path =
            std::env::var("BSL_V2_REAL_CONF_BIG_WARM_NON_MEMBER_COLLECT_BREAKDOWN_REPORT")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("tests")
                        .join("perf")
                        .join("reports")
                        .join(format!(
                            "{change_id}-real-conf-big-warm-non-member-collect-breakdown-live.json"
                        ))
                });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for warm non-member collect report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize warm non-member collect report"),
        )
        .expect("write warm non-member collect report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        assert!(
            trace_matching_mode == "request_id",
            "expected request-context parity to expose JSON-RPC request ids in completion timeline, filtered_traces={filtered_traces:?}"
        );
        assert!(
            measured_non_empty_samples == MEASURE_REQUESTS,
            "expected all measured warm non-member samples to be non-empty, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_contains_this_object_samples == MEASURE_REQUESTS,
            "expected every measured warm non-member sample to preserve form contextual completion label ЭтотОбъект, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_trace_linked_samples == MEASURE_REQUESTS,
            "expected every measured warm non-member sample to link to a completion timeline trace, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_collect_breakdown_linked_samples == MEASURE_REQUESTS,
            "expected every measured warm non-member sample to link to a collect breakdown trace, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_ok_non_empty_traces == MEASURE_REQUESTS,
            "expected every measured warm non-member trace to stay ok_non_empty, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_fail_closed_traces == 0
                && counter_delta("intellisense_v2_completion_result_total_fail_closed") == 0,
            "warm non-member gate must fail on any fail_closed result, measured_samples={measured_samples:?}"
        );
        assert!(
            counter_delta("completion_total") >= MEASURE_REQUESTS as u64,
            "expected measured completion_total delta to cover measured requests, completion_total_delta={}",
            counter_delta("completion_total")
        );
        assert!(
            read_u64_metric(raw_collect_observability_histogram.get("count"))
                >= MEASURE_REQUESTS as u64,
            "expected completion_stage_collect_ms to record measured warm non-member samples, histogram={raw_collect_observability_histogram:?}"
        );
        assert!(
            read_u64_metric(
                measured_collect_breakdown
                    .get("non_member_global_functions")
                    .and_then(|value| value.get("count"))
            ) == MEASURE_REQUESTS as u64,
            "expected trace-linked global-functions collect breakdown for every measured sample, breakdown={measured_collect_breakdown:?}"
        );
        assert!(
            read_u64_metric(
                measured_collect_breakdown
                    .get("non_member_metadata_items")
                    .and_then(|value| value.get("count"))
            ) == MEASURE_REQUESTS as u64,
            "expected trace-linked metadata-items collect breakdown for every measured sample, breakdown={measured_collect_breakdown:?}"
        );
        assert!(
            read_u64_metric(
                measured_collect_breakdown
                    .get("non_member_repository_types")
                    .and_then(|value| value.get("count"))
            ) == MEASURE_REQUESTS as u64,
            "expected trace-linked repository-types collect breakdown for every measured sample, breakdown={measured_collect_breakdown:?}"
        );
        assert!(
            read_u64_metric(
                measured_collect_breakdown
                    .get("non_member_keywords")
                    .and_then(|value| value.get("count"))
            ) == MEASURE_REQUESTS as u64,
            "expected trace-linked keywords collect breakdown for every measured sample, breakdown={measured_collect_breakdown:?}"
        );
        assert!(
            measured_collect_p95_ms <= WARM_NON_MEMBER_COLLECT_P95_BUDGET_MS,
            "warm non-member collect p95 regression: measured_collect_p95_ms={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_collect_p95_ms,
            WARM_NON_MEMBER_COLLECT_P95_BUDGET_MS
        );

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}
