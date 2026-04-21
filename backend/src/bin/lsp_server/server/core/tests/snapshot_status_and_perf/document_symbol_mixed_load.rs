#[test]
fn p39_real_conf_big_document_symbol_mixed_load_gate_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p39 tokio runtime");
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
        const PROFILE_NAME: &str = "p39_real_conf_big_document_symbol_mixed_load_gate_live";
        let change_id = std::env::var("CHANGE_ID")
            .unwrap_or_else(|_| "refactor-conf-big-parse-apply-contention-bounding".to_string());
        const WARMUP_REQUESTS: usize = 1;
        const WAITER_RAMP_REQUESTS: usize = 2;
        const MEASURE_REQUESTS: usize = 8;
        const TOTAL_MIXED_LOAD_REQUESTS: usize = WAITER_RAMP_REQUESTS + MEASURE_REQUESTS;
        const DOCUMENT_SYMBOL_BURST_REQUESTS: usize = 4;
        const CURRENT_CONTEXT_REQUESTS_PER_MEASURED_COMPLETION: usize = 1;
        const APPLY_DELAY_MS: u64 = 80;
        const WARMUP_COMPLETION_MARKER: &str = "Объект.";
        const MEASURED_COMPLETION_MARKER_MODE: &str = "per_probe_form_context_marker";
        const ADAPTER_TO_DISPATCH_MAX_FACTOR: u64 = 4;
        const TRUTHFUL_WAIT_P95_FACTOR: f64 = 1.0;
        const TRUTHFUL_WAIT_MAX_FACTOR: u64 = 4;
        const APPLY_BACKLOG_P95_BUDGET_MS: f64 = 3_500.0;
        const APPLY_BACKLOG_MAX_BUDGET_MS: u64 = 5_000;
        const SERVICE_FUTURE_FIRST_POLL_P95_BUDGET_MS: f64 = 250.0;
        const SERVICE_FUTURE_FIRST_POLL_MAX_BUDGET_MS: u64 = 1_000;
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

        let module_path = conf_big_waiter_mixed_load_module_path_for_tests(&conf_big_root);
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
            std::fs::read_to_string(&module_path).expect("read conf_big module text for p39 report");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(&server, &workspace_setup, "p39_real_conf_big_live_setup")
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
            .expect("latest received version for p39 opened file");
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

        let seeded_ready = tokio::task::spawn_blocking({
            let module_text = module_text.clone();
            move || {
                let parse_result = bsl_syntax::parse_fast(module_text.as_str())
                    .map_err(|err| err.to_string())?;
                crate::handlers::symbols::build_document_symbols(module_text.as_str(), &parse_result)
                    .map_err(|err| err.to_string())
            }
        })
        .await
        .expect("seed outline builder join")
        .expect("seed outline builder must succeed on the opened real conf_big module");
        assert!(
            !document_symbol_names(&seeded_ready).is_empty(),
            "seed outline builder must produce a non-empty ready cache for the opened real conf_big module"
        );
        server
            .record_document_symbol_ready_v2(file_id, opened_version, seeded_ready)
            .await;

        let seeded_response = tokio::time::timeout(
            Duration::from_secs(2),
            harness.send_request(
                39_100_000,
                "textDocument/documentSymbol",
                DocumentSymbolParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                },
            ),
        )
        .await
        .expect("pre-seeded live documentSymbol request must stay bounded");
        let seeded = document_symbol_response_from_jsonrpc_response(&seeded_response)
            .expect("pre-seeded live documentSymbol request must observe ready outline cache");
        assert!(
            !document_symbol_names(&seeded).is_empty(),
            "seed documentSymbol response must expose non-empty real-module outline"
        );
        let seeded_ready = server
            .latest_document_symbol_ready_v2(file_id)
            .await
            .expect("pre-seeded real conf_big outline cache must be present");
        assert!(
            seeded_ready.file_version >= opened_version,
            "pre-seeded real conf_big outline cache must target the opened version"
        );
        assert!(
            !document_symbol_names(&seeded_ready.response).is_empty(),
            "pre-seeded real conf_big ready cache must stay non-empty"
        );

        let completion_position =
            find_utf16_position_after_marker(&module_text, WARMUP_COMPLETION_MARKER);
        let completion_context = Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        });
        let current_context_session_id = format!("{}::p39-conf-big-current-context", uri);
        let mut current_text = module_text.clone();
        let mut current_version = opened_version;

        let mut warmup_samples = Vec::new();
        for index in 0..WARMUP_REQUESTS {
            let request_id = 39_100_100_i64 + index as i64;
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

        let _apply_delay_guard =
            EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", &APPLY_DELAY_MS.to_string());
        let metrics_before_measured = coordinator.observability_metrics();
        let counters_before_measured = metrics_before_measured
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics_before_measured.counters object");

        let mut ramp_samples: Vec<serde_json::Value> = Vec::new();
        let mut measured_samples: Vec<serde_json::Value> = Vec::new();
        for index in 0..TOTAL_MIXED_LOAD_REQUESTS {
            let outline_probe_name = format!("DocumentSymbolIsolationProbe{}", index + 1);
            let completion_marker =
                format!("Процедура {outline_probe_name}() Экспорт\n    Этот");
            let appended_outline = format!(
                "\n#Область {outline_probe_name}\n&НаСервере\nПроцедура {outline_probe_name}() Экспорт\n    Этот\n    Сообщить(\"{outline_probe_name}\");\nКонецПроцедуры\n#КонецОбласти\n"
            );
            let next_version = current_version
                .checked_add(1)
                .expect("p39 mixed-load version overflow");
            live_transport_append_text_change(
                &mut harness,
                &uri,
                &current_text,
                next_version,
                &appended_outline,
            )
            .await;
            current_text.push_str(&appended_outline);
            current_version = next_version;
            let completion_position =
                find_utf16_position_after_marker(&current_text, &completion_marker);
            let current_context_marker = format!("Сообщить(\"{outline_probe_name}\")");
            let current_context_position =
                find_utf16_position_after_marker(&current_text, &current_context_marker);
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
            .expect("test must observe parse-snapshot gap before mixed outline load");

            let document_symbol_request_ids = (0..DOCUMENT_SYMBOL_BURST_REQUESTS)
                .map(|burst_index| 39_200_000_i64 + (index as i64 * 10) + burst_index as i64)
                .collect::<Vec<_>>();
            for request_id in &document_symbol_request_ids {
                live_transport_write_document_symbol_request(&mut harness, *request_id, &uri).await;
            }
            let current_context_request_id = 39_250_000_i64 + index as i64;
            live_transport_write_execute_command_request(
                &mut harness,
                current_context_request_id,
                "bsl.getCurrentContext",
                vec![serde_json::json!({
                    "uri": uri.to_string(),
                    "line": current_context_position.line,
                    "character": current_context_position.character,
                    "editorSessionId": current_context_session_id.as_str(),
                    "requestGeneration": (index + 1) as u64,
                })],
            )
            .await;

            let forced_shadow_fast_path_off_version = current_version
                .checked_sub(1)
                .expect("measured mixed-load revisions must stay above the seeded version");
            server.latest_document_shadow_state_v2.write().await.insert(
                file_id,
                DocumentShadowStateV2 {
                    version: forced_shadow_fast_path_off_version,
                    text: Arc::from(current_text.clone()),
                },
            );

            let completion_request_id = 39_300_000_i64 + index as i64;
            let completion_started = Instant::now();
            let completion_request_written_at_ms = live_transport_write_completion_request(
                &mut harness,
                completion_request_id,
                &uri,
                completion_position,
                completion_context.clone(),
            )
            .await;

            let (
                completion_response,
                completion_elapsed_ms,
                document_symbol_responses,
                current_context_response,
            ) =
                tokio::time::timeout(Duration::from_secs(10), async {
                    let mut completion_response = None;
                    let mut completion_elapsed_ms = None;
                    let mut document_symbol_responses = Vec::new();
                    let mut current_context_response = None;
                    loop {
                        let response = harness.read_message().await;
                        let Some(response_id) = response.get("id").and_then(|value| value.as_i64())
                        else {
                            continue;
                        };
                        if response_id == completion_request_id {
                            completion_elapsed_ms = Some(
                                completion_started.elapsed().as_millis().min(u64::MAX as u128)
                                    as u64,
                            );
                            completion_response = Some(response);
                        } else if response_id == current_context_request_id {
                            current_context_response = Some(response);
                        } else if document_symbol_request_ids.contains(&response_id) {
                            document_symbol_responses.push(response);
                        }
                        if document_symbol_responses.len() == document_symbol_request_ids.len() {
                            if let Some(completion_response) = completion_response.take() {
                                break (
                                    completion_response,
                                    completion_elapsed_ms.expect("completion elapsed"),
                                    document_symbol_responses,
                                    current_context_response,
                                );
                            }
                        }
                    }
                })
                .await
                .expect("mixed-load completion and outline responses must arrive");
            let completion_result = completion_response
                .get("result")
                .cloned()
                .expect("completion result field");
            let completion: Option<CompletionResponse> =
                serde_json::from_value(completion_result).expect("parse completion result");
            let labels = match completion.expect("completion result present") {
                CompletionResponse::List(list) => {
                    list.items.into_iter().map(|item| item.label).collect::<Vec<_>>()
                }
                CompletionResponse::Array(items) => {
                    items.into_iter().map(|item| item.label).collect::<Vec<_>>()
                }
            };
            let document_symbol_response_summaries = document_symbol_request_ids
                .iter()
                .map(|request_id| {
                    let response = document_symbol_responses
                        .iter()
                        .find(|response| {
                            response.get("id").and_then(|value| value.as_i64()) == Some(*request_id)
                        })
                        .expect("documentSymbol response by request id");
                    let parsed = document_symbol_response_from_jsonrpc_response(response);
                    let names = parsed
                        .as_ref()
                        .map(document_symbol_names)
                        .unwrap_or_default();
                    let contains_fresh_outline_name =
                        names.iter().any(|name| name == &outline_probe_name);
                    serde_json::json!({
                        "request_id": request_id,
                        "result_kind": if parsed.is_some() { "present" } else { "null" },
                        "symbol_count": names.len(),
                        "contains_fresh_outline_name": contains_fresh_outline_name,
                        "sample_names": names.into_iter().take(8).collect::<Vec<_>>(),
                    })
                })
                .collect::<Vec<_>>();
            let document_symbol_present_responses = document_symbol_response_summaries
                .iter()
                .filter(|summary| {
                    summary
                        .get("result_kind")
                        .and_then(|value| value.as_str())
                        == Some("present")
                })
                .count();
            let document_symbol_fresh_outline_present = document_symbol_response_summaries
                .iter()
                .any(|summary| {
                    summary
                        .get("contains_fresh_outline_name")
                        .and_then(|value| value.as_bool())
                        == Some(true)
                });
            assert!(
                document_symbol_present_responses > 0,
                "mixed-load sample must keep at least one bounded outline response, summaries={document_symbol_response_summaries:?}"
            );
            assert!(
                !document_symbol_fresh_outline_present,
                "outline mixed-load sample must not masquerade as current revision while parse gap is active, fresh_outline_name={outline_probe_name}, summaries={document_symbol_response_summaries:?}"
            );
            let current_context_function_name = current_context_response
                .as_ref()
                .and_then(|response| response.get("result"))
                .and_then(|value| value.get("functionName"))
                .and_then(|value| value.as_str())
                .map(str::to_string);

            let sample = serde_json::json!({
                "step": format!("measured_document_symbol_mixed_load_completion_{}", index + 1),
                "request_id": completion_request_id,
                "elapsed_ms": completion_elapsed_ms,
                "label_count": labels.len(),
                "labels": labels,
                "version": current_version,
                "completion_request_written_at_ms": completion_request_written_at_ms,
                "fresh_outline_name": outline_probe_name,
                "document_symbol_request_ids": document_symbol_request_ids,
                "document_symbol_response_summaries": document_symbol_response_summaries,
                "document_symbol_present_responses": document_symbol_present_responses,
                "current_context_request_id": current_context_request_id,
                "current_context_function_name": current_context_function_name,
                "current_context_response_seen": current_context_response.is_some(),
                "completion_marker_mode": MEASURED_COMPLETION_MARKER_MODE,
                "forced_shadow_fast_path_off_version": forced_shadow_fast_path_off_version,
                "did_change_notifications_per_measured_completion": 1,
                "did_save_after_did_change": true,
                "parse_gap_source": parse_gap_source,
            });
            if index < WAITER_RAMP_REQUESTS {
                ramp_samples.push(sample);
            } else {
                measured_samples.push(sample);
            }
        }

        let completion_timeline =
            live_transport_get_completion_timeline(&mut harness, 39_300_900, 160).await;
        let observability_metrics =
            live_transport_get_observability_metrics(&mut harness, 39_300_901).await;
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

        let total_sample_count = WARMUP_REQUESTS + WAITER_RAMP_REQUESTS + MEASURE_REQUESTS;
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
                            "wait_for_file_version_runtime_queue_wait_ms": trace
                                .get("prepare_details")
                                .and_then(|value| value.get("wait_for_file_version_runtime"))
                                .and_then(|value| value.get("queue_wait_ms"))
                                .and_then(|value| value.as_u64()),
                            "wait_for_file_version_runtime_exec_ms": trace
                                .get("prepare_details")
                                .and_then(|value| value.get("wait_for_file_version_runtime"))
                                .and_then(|value| value.get("exec_ms"))
                                .and_then(|value| value.as_u64()),
                            "wait_for_file_version_runtime_wake_wait_ms": trace
                                .get("prepare_details")
                                .and_then(|value| value.get("wait_for_file_version_runtime"))
                                .and_then(|value| value.get("wake_wait_ms"))
                                .and_then(|value| value.as_u64()),
                            "wait_for_file_version_runtime_resolution": trace
                                .get("prepare_details")
                                .and_then(|value| value.get("wait_for_file_version_runtime"))
                                .and_then(|value| value.get("resolution"))
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
                            "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                    "dominant_stage": trace.get("dominant_stage").and_then(|value| value.as_str()),
                    "queue_outcome": trace.get("queue_outcome").and_then(|value| value.as_str()),
                    "turn_wait_outcome": trace.get("turn_wait_outcome").and_then(|value| value.as_str()),
                            "adapter_read_at_ms": completion_timeline_server_edge_u64(
                                trace,
                                "adapter_read_at_ms",
                            ),
                            "transport_received_at_ms": completion_timeline_server_edge_u64(
                                trace,
                                "transport_received_at_ms",
                            ),
                            "jsonrpc_dispatch_received_at_ms": completion_timeline_server_edge_u64(
                                trace,
                                "jsonrpc_dispatch_received_at_ms",
                            ),
                            "read_loop_wait_reason": trace
                                .get("server_edge_details")
                                .and_then(|value| value.get("read_loop_wait_reason"))
                                .and_then(|value| value.as_str()),
                            "read_loop_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "read_loop_wait_ms",
                            ),
                            "pending_completion_spillover_depth": completion_timeline_server_edge_u64(
                                trace,
                                "pending_completion_spillover_depth",
                            ),
                            "adapter_to_dispatch_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "adapter_to_dispatch_wait_ms",
                            ),
                            "admission_queue_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "admission_queue_wait_ms",
                            ),
                            "scheduler_poll_ready_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "scheduler_poll_ready_wait_ms",
                            ),
                            "completion_barrier_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "completion_barrier_wait_ms",
                            ),
                            "completion_barrier_owner_method": trace
                                .get("server_edge_details")
                                .and_then(|value| value.get("completion_barrier_owner_method"))
                                .and_then(|value| value.as_str()),
                            "completion_barrier_owner_uri": trace
                                .get("server_edge_details")
                                .and_then(|value| value.get("completion_barrier_owner_uri"))
                                .and_then(|value| value.as_str()),
                            "completion_barrier_owner_version": trace
                                .get("server_edge_details")
                                .and_then(|value| value.get("completion_barrier_owner_version"))
                                .and_then(|value| value.as_i64()),
                            "same_file_ingress_token_required_version": trace
                                .get("server_edge_details")
                                .and_then(|value| {
                                    value.get("same_file_ingress_token_required_version")
                                })
                                .and_then(|value| value.as_i64()),
                            "same_file_ingress_token_published_at_ms": completion_timeline_server_edge_u64(
                                trace,
                                "same_file_ingress_token_published_at_ms",
                            ),
                            "same_file_ingress_token_source": trace
                                .get("server_edge_details")
                                .and_then(|value| value.get("same_file_ingress_token_source"))
                                .and_then(|value| value.as_str()),
                            "same_file_ingress_token_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "same_file_ingress_token_wait_ms",
                            ),
                            "scheduler_ready_to_dispatch_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "scheduler_ready_to_dispatch_wait_ms",
                            ),
                            "dispatch_to_request_context_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "dispatch_to_request_context_wait_ms",
                            ),
                            "transport_to_handler_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "transport_to_handler_wait_ms",
                            ),
                            "transport_to_service_future_wait_ms": completion_timeline_server_edge_u64(
                                trace,
                                "transport_to_service_future_wait_ms",
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
                            "wait_exact_type_index_ms": completion_timeline_trace_stage_duration_ms(trace, "wait_exact_type_index"),
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
        let ramp_samples = enrich_samples(ramp_samples, WARMUP_REQUESTS);
        let measured_samples =
            enrich_samples(measured_samples, WARMUP_REQUESTS + WAITER_RAMP_REQUESTS);

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
                    "wait_for_file_version_runtime_queue_wait_ms": trace
                        .get("prepare_details")
                        .and_then(|value| value.get("wait_for_file_version_runtime"))
                        .and_then(|value| value.get("queue_wait_ms"))
                        .and_then(|value| value.as_u64()),
                    "wait_for_file_version_runtime_exec_ms": trace
                        .get("prepare_details")
                        .and_then(|value| value.get("wait_for_file_version_runtime"))
                        .and_then(|value| value.get("exec_ms"))
                        .and_then(|value| value.as_u64()),
                    "wait_for_file_version_runtime_wake_wait_ms": trace
                        .get("prepare_details")
                        .and_then(|value| value.get("wait_for_file_version_runtime"))
                        .and_then(|value| value.get("wake_wait_ms"))
                        .and_then(|value| value.as_u64()),
                    "wait_for_file_version_runtime_resolution": trace
                        .get("prepare_details")
                        .and_then(|value| value.get("wait_for_file_version_runtime"))
                        .and_then(|value| value.get("resolution"))
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
                    "started_at_ms": trace.get("started_at_ms").and_then(|value| value.as_u64()),
                    "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                    "dominant_stage": trace.get("dominant_stage").and_then(|value| value.as_str()),
                    "queue_outcome": trace.get("queue_outcome").and_then(|value| value.as_str()),
                    "turn_wait_outcome": trace.get("turn_wait_outcome").and_then(|value| value.as_str()),
                    "adapter_read_at_ms": completion_timeline_server_edge_u64(
                        trace,
                        "adapter_read_at_ms",
                    ),
                    "transport_received_at_ms": completion_timeline_server_edge_u64(
                        trace,
                        "transport_received_at_ms",
                    ),
                    "jsonrpc_dispatch_received_at_ms": completion_timeline_server_edge_u64(
                        trace,
                        "jsonrpc_dispatch_received_at_ms",
                    ),
                    "read_loop_wait_reason": trace
                        .get("server_edge_details")
                        .and_then(|value| value.get("read_loop_wait_reason"))
                        .and_then(|value| value.as_str()),
                    "read_loop_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "read_loop_wait_ms",
                    ),
                    "pending_completion_spillover_depth": completion_timeline_server_edge_u64(
                        trace,
                        "pending_completion_spillover_depth",
                    ),
                    "adapter_to_dispatch_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "adapter_to_dispatch_wait_ms",
                    ),
                    "admission_queue_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "admission_queue_wait_ms",
                    ),
                    "scheduler_poll_ready_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "scheduler_poll_ready_wait_ms",
                    ),
                    "completion_barrier_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "completion_barrier_wait_ms",
                    ),
                    "completion_barrier_owner_method": trace
                        .get("server_edge_details")
                        .and_then(|value| value.get("completion_barrier_owner_method"))
                        .and_then(|value| value.as_str()),
                    "completion_barrier_owner_uri": trace
                        .get("server_edge_details")
                        .and_then(|value| value.get("completion_barrier_owner_uri"))
                        .and_then(|value| value.as_str()),
                    "completion_barrier_owner_version": trace
                        .get("server_edge_details")
                        .and_then(|value| value.get("completion_barrier_owner_version"))
                        .and_then(|value| value.as_i64()),
                    "same_file_ingress_token_required_version": trace
                        .get("server_edge_details")
                        .and_then(|value| {
                            value.get("same_file_ingress_token_required_version")
                        })
                        .and_then(|value| value.as_i64()),
                    "same_file_ingress_token_published_at_ms": completion_timeline_server_edge_u64(
                        trace,
                        "same_file_ingress_token_published_at_ms",
                    ),
                    "same_file_ingress_token_source": trace
                        .get("server_edge_details")
                        .and_then(|value| value.get("same_file_ingress_token_source"))
                        .and_then(|value| value.as_str()),
                    "same_file_ingress_token_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "same_file_ingress_token_wait_ms",
                    ),
                    "scheduler_ready_to_dispatch_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "scheduler_ready_to_dispatch_wait_ms",
                    ),
                    "transport_to_handler_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "transport_to_handler_wait_ms",
                    ),
                    "transport_to_service_future_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "transport_to_service_future_wait_ms",
                    ),
                    "service_future_to_first_poll_wait_ms": completion_timeline_server_edge_u64(
                        trace,
                        "service_future_to_first_poll_wait_ms",
                    ),
                    "prepare_stateful_ms": completion_timeline_trace_stage_duration_ms(trace, "prepare_stateful"),
                    "wait_exact_type_index_ms": completion_timeline_trace_stage_duration_ms(trace, "wait_exact_type_index"),
                    "query_bundle": completion_timeline_query_bundle_breakdown(trace),
                    "collect_ms": completion_timeline_trace_stage_duration_ms(trace, "collect"),
                    "response_build_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build"),
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
        let sample_trace_server_edge_present_samples =
            |samples: &[serde_json::Value], field: &str| {
                samples
                    .iter()
                    .filter(|sample| {
                        sample
                            .get("trace")
                            .and_then(|trace| trace.get(field))
                            .and_then(|value| value.as_u64())
                            .is_some()
                    })
                    .count()
            };
        let sample_trace_server_edge_max_ms = |samples: &[serde_json::Value], field: &str| {
            samples
                .iter()
                .filter_map(|sample| {
                    sample
                        .get("trace")
                        .and_then(|trace| trace.get(field))
                        .and_then(|value| value.as_u64())
                })
                .max()
                .unwrap_or(0)
        };
        let sample_trace_server_edge_over_budget_samples =
            |samples: &[serde_json::Value], field: &str, budget_ms: u64| {
                samples
                    .iter()
                    .filter(|sample| {
                        sample
                            .get("trace")
                            .and_then(|trace| trace.get(field))
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0)
                            > budget_ms
                    })
                    .count()
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
            .filter(|sample| sample.get("trace").is_some_and(|trace| !trace.is_null()))
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
        let measured_document_symbol_present_responses_total = measured_samples
            .iter()
            .filter_map(|sample| {
                sample
                    .get("document_symbol_present_responses")
                    .and_then(|value| value.as_u64())
            })
            .sum::<u64>();
        let measured_document_symbol_null_responses_total =
            (MEASURE_REQUESTS * DOCUMENT_SYMBOL_BURST_REQUESTS) as u64
                - measured_document_symbol_present_responses_total;
        let measured_document_symbol_fresh_outline_leak_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("document_symbol_response_summaries")
                    .and_then(|value| value.as_array())
                    .is_some_and(|summaries| {
                        summaries.iter().any(|summary| {
                            summary
                                .get("contains_fresh_outline_name")
                                .and_then(|value| value.as_bool())
                                == Some(true)
                        })
                    })
            })
            .count();
        let measured_document_symbol_latest_ready_total_delta = counter_delta(
            "intellisense_v2_document_symbol_outcome_total_outcome_latest_ready",
        );
        let measured_document_symbol_current_ready_total_delta = counter_delta(
            "intellisense_v2_document_symbol_outcome_total_outcome_current_ready",
        );
        let measured_document_symbol_unavailable_total_delta = counter_delta(
            "intellisense_v2_document_symbol_outcome_total_outcome_unavailable",
        );
        let measured_document_symbol_superseded_total_delta = counter_delta(
            "intellisense_v2_document_symbol_outcome_total_outcome_superseded",
        );
        let measured_document_symbol_total_outcome_delta =
            measured_document_symbol_latest_ready_total_delta
                + measured_document_symbol_current_ready_total_delta
                + measured_document_symbol_unavailable_total_delta
                + measured_document_symbol_superseded_total_delta;
        let measured_current_context_response_seen_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("current_context_response_seen")
                    .and_then(|value| value.as_bool())
                    == Some(true)
            })
            .count();
        let measured_parse_snapshot_full_total_delta =
            counter_delta("intellisense_v2_parse_snapshot_total_origin_lsp_mode_full");
        let measured_parse_snapshot_incremental_total_delta =
            counter_delta("intellisense_v2_parse_snapshot_total_origin_lsp_mode_incremental");
        let measured_parse_snapshot_reused_total_delta =
            counter_delta("intellisense_v2_parse_snapshot_total_origin_lsp_mode_reused");
        let measured_parse_snapshot_no_previous_tree_total_delta = counter_delta(
            "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_no_previous_tree",
        );
        let measured_parse_snapshot_no_edits_total_delta = counter_delta(
            "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_no_edits_provided",
        );
        let measured_current_context_parse_attempts_total =
            crate::server::command_handlers::get_current_context_parse_attempts_for_test();
        let warmup_latency_histogram = sample_elapsed_histogram(&warmup_samples);
        let measured_latency_histogram = sample_elapsed_histogram(&measured_samples);
        let _measured_latency_p95_ms = read_numeric_metric(measured_latency_histogram.get("p95"));
        let truthful_wait_p95_budget_ms =
            (interactive_wait_budget_ms as f64) * TRUTHFUL_WAIT_P95_FACTOR;
        let truthful_wait_max_budget_ms =
            interactive_wait_budget_ms.saturating_mul(TRUTHFUL_WAIT_MAX_FACTOR);
        let measured_wait_for_file_version_runtime_queue_wait_histogram = sample_trace_histogram(
            &measured_samples,
            "wait_for_file_version_runtime_queue_wait_ms",
        );
        let measured_wait_for_file_version_runtime_queue_wait_p95_ms = read_numeric_metric(
            measured_wait_for_file_version_runtime_queue_wait_histogram.get("p95"),
        );
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
        let measured_wait_for_file_version_runtime_waiter_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("wait_for_file_version_runtime_resolution"))
                    .and_then(|value| value.as_str())
                    == Some("waiter")
            })
            .count();
        let measured_wait_for_file_version_runtime_immediate_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("wait_for_file_version_runtime_resolution"))
                    .and_then(|value| value.as_str())
                    == Some("immediate")
            })
            .count();
        let measured_runtime_apply_changes_queue_wait_ms = histogram_metric_value_or_zero(
            histograms,
            "intellisense_v2_runtime_apply_changes_queue_wait_ms",
            None,
        );
        let measured_runtime_apply_changes_queue_wait_present_samples =
            read_u64_metric(measured_runtime_apply_changes_queue_wait_ms.get("count"));
        let measured_runtime_apply_changes_queue_wait_p95_ms =
            read_numeric_metric(measured_runtime_apply_changes_queue_wait_ms.get("p95"));
        let measured_runtime_apply_changes_queue_wait_max_ms =
            read_numeric_metric(measured_runtime_apply_changes_queue_wait_ms.get("max"));
        let measured_prepare_apply_age_at_start_ms = histogram_metric_value_or_zero(
            histograms,
            "completion_stage_prepare_apply_age_at_start_ms",
            None,
        );
        let measured_prepare_apply_age_at_terminal_ms = histogram_metric_value_or_zero(
            histograms,
            "completion_stage_prepare_apply_age_at_terminal_ms",
            None,
        );
        let measured_exact_wait_apply_age_at_start_ms = histogram_metric_value_or_zero(
            histograms,
            "completion_stage_exact_wait_apply_age_at_start_ms",
            None,
        );
        let measured_exact_wait_apply_age_at_terminal_ms = histogram_metric_value_or_zero(
            histograms,
            "completion_stage_exact_wait_apply_age_at_terminal_ms",
            None,
        );
        let measured_adapter_to_dispatch_histogram = sample_trace_server_edge_histogram(
            &measured_samples,
            "adapter_to_dispatch_wait_ms",
        );
        let measured_adapter_to_dispatch_p95_ms =
            read_numeric_metric(measured_adapter_to_dispatch_histogram.get("p95"));
        let measured_adapter_to_dispatch_max_ms =
            sample_trace_server_edge_max_ms(&measured_samples, "adapter_to_dispatch_wait_ms");
        let measured_pre_dispatch_wait_over_budget_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("adapter_to_dispatch_wait_ms"))
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
                    > interactive_wait_budget_ms
            })
            .count();
        let measured_pre_dispatch_wait_over_hard_cap_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("adapter_to_dispatch_wait_ms"))
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
                    > interactive_wait_budget_ms
                        .saturating_mul(ADAPTER_TO_DISPATCH_MAX_FACTOR)
            })
            .count();
        let measured_read_loop_wait_histogram =
            sample_trace_server_edge_histogram(&measured_samples, "read_loop_wait_ms");
        let measured_read_loop_wait_p95_ms =
            read_numeric_metric(measured_read_loop_wait_histogram.get("p95"));
        let measured_read_loop_wait_present_samples =
            sample_trace_server_edge_present_samples(&measured_samples, "read_loop_wait_ms");
        let measured_read_loop_wait_max_ms =
            sample_trace_server_edge_max_ms(&measured_samples, "read_loop_wait_ms");
        let measured_read_loop_wait_over_hard_cap_samples = sample_trace_server_edge_over_budget_samples(
            &measured_samples,
            "read_loop_wait_ms",
            truthful_wait_max_budget_ms,
        );
        let measured_admission_queue_wait_histogram =
            sample_trace_server_edge_histogram(&measured_samples, "admission_queue_wait_ms");
        let measured_admission_queue_wait_p95_ms =
            read_numeric_metric(measured_admission_queue_wait_histogram.get("p95"));
        let measured_admission_queue_wait_present_samples =
            sample_trace_server_edge_present_samples(&measured_samples, "admission_queue_wait_ms");
        let measured_admission_queue_wait_max_ms =
            sample_trace_server_edge_max_ms(&measured_samples, "admission_queue_wait_ms");
        let measured_admission_queue_wait_over_hard_cap_samples =
            sample_trace_server_edge_over_budget_samples(
                &measured_samples,
                "admission_queue_wait_ms",
                truthful_wait_max_budget_ms,
            );
        let measured_scheduler_poll_ready_wait_histogram = sample_trace_server_edge_histogram(
            &measured_samples,
            "scheduler_poll_ready_wait_ms",
        );
        let measured_scheduler_poll_ready_wait_p95_ms =
            read_numeric_metric(measured_scheduler_poll_ready_wait_histogram.get("p95"));
        let measured_scheduler_poll_ready_wait_present_samples = sample_trace_server_edge_present_samples(
            &measured_samples,
            "scheduler_poll_ready_wait_ms",
        );
        let measured_scheduler_poll_ready_wait_max_ms =
            sample_trace_server_edge_max_ms(&measured_samples, "scheduler_poll_ready_wait_ms");
        let measured_scheduler_poll_ready_wait_over_hard_cap_samples =
            sample_trace_server_edge_over_budget_samples(
                &measured_samples,
                "scheduler_poll_ready_wait_ms",
                truthful_wait_max_budget_ms,
            );
        let measured_completion_barrier_wait_histogram = sample_trace_server_edge_histogram(
            &measured_samples,
            "completion_barrier_wait_ms",
        );
        let measured_completion_barrier_wait_p95_ms =
            read_numeric_metric(measured_completion_barrier_wait_histogram.get("p95"));
        let measured_completion_barrier_wait_present_samples =
            sample_trace_server_edge_present_samples(&measured_samples, "completion_barrier_wait_ms");
        let measured_completion_barrier_wait_max_ms =
            sample_trace_server_edge_max_ms(&measured_samples, "completion_barrier_wait_ms");
        let measured_completion_barrier_wait_over_hard_cap_samples =
            sample_trace_server_edge_over_budget_samples(
                &measured_samples,
                "completion_barrier_wait_ms",
                truthful_wait_max_budget_ms,
            );
        let measured_same_file_ingress_token_wait_histogram = sample_trace_server_edge_histogram(
            &measured_samples,
            "same_file_ingress_token_wait_ms",
        );
        let measured_same_file_ingress_token_wait_p95_ms =
            read_numeric_metric(measured_same_file_ingress_token_wait_histogram.get("p95"));
        let measured_same_file_ingress_token_wait_present_samples =
            sample_trace_server_edge_present_samples(
                &measured_samples,
                "same_file_ingress_token_wait_ms",
            );
        let measured_same_file_ingress_token_wait_max_ms =
            sample_trace_server_edge_max_ms(&measured_samples, "same_file_ingress_token_wait_ms");
        let measured_same_file_ingress_token_wait_over_hard_cap_samples =
            sample_trace_server_edge_over_budget_samples(
                &measured_samples,
                "same_file_ingress_token_wait_ms",
                truthful_wait_max_budget_ms,
            );
        let measured_scheduler_ready_to_dispatch_wait_histogram =
            sample_trace_server_edge_histogram(
                &measured_samples,
                "scheduler_ready_to_dispatch_wait_ms",
            );
        let measured_scheduler_ready_to_dispatch_wait_p95_ms =
            read_numeric_metric(measured_scheduler_ready_to_dispatch_wait_histogram.get("p95"));
        let measured_scheduler_ready_to_dispatch_wait_present_samples =
            sample_trace_server_edge_present_samples(
                &measured_samples,
                "scheduler_ready_to_dispatch_wait_ms",
            );
        let measured_scheduler_ready_to_dispatch_wait_max_ms = sample_trace_server_edge_max_ms(
            &measured_samples,
            "scheduler_ready_to_dispatch_wait_ms",
        );
        let measured_scheduler_ready_to_dispatch_wait_over_hard_cap_samples =
            sample_trace_server_edge_over_budget_samples(
                &measured_samples,
                "scheduler_ready_to_dispatch_wait_ms",
                truthful_wait_max_budget_ms,
            );
        let measured_same_file_ingress_token_published_samples =
            sample_trace_server_edge_present_samples(
                &measured_samples,
                "same_file_ingress_token_published_at_ms",
            );
        let measured_truthful_pre_dispatch_bucket_shift_samples = measured_samples
            .iter()
            .filter(|sample| {
                [
                    "read_loop_wait_ms",
                    "admission_queue_wait_ms",
                    "scheduler_poll_ready_wait_ms",
                    "completion_barrier_wait_ms",
                    "same_file_ingress_token_wait_ms",
                    "scheduler_ready_to_dispatch_wait_ms",
                ]
                .into_iter()
                .any(|field| {
                    sample
                        .get("trace")
                        .and_then(|trace| trace.get(field))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0)
                        > truthful_wait_max_budget_ms
                })
            })
            .count();
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
        let measured_client_to_transport_over_hard_cap_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("client_to_transport_wait_ms")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
                    > truthful_wait_max_budget_ms
            })
            .count();
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
        let measured_response_output_handoff_send_over_hard_cap_samples = measured_samples
            .iter()
            .filter(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("response_output_handoff_send_wait_ms"))
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
                    > truthful_wait_max_budget_ms
            })
            .count();
        let measured_transport_to_handler_histogram = sample_trace_server_edge_histogram(
            &measured_samples,
            "transport_to_handler_wait_ms",
        );
        let measured_transport_to_handler_max_ms = measured_samples
            .iter()
            .filter_map(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get("transport_to_handler_wait_ms"))
                    .and_then(|value| value.as_u64())
            })
            .max()
            .unwrap_or(0);
        let measured_ingress_regression_samples = measured_samples
            .iter()
            .filter(|sample| {
                let service_future_wait = sample
                    .get("trace")
                    .and_then(|trace| trace.get("service_future_to_first_poll_wait_ms"))
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0);
                let transport_to_handler_wait = sample
                    .get("trace")
                    .and_then(|trace| trace.get("transport_to_handler_wait_ms"))
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0);
                service_future_wait > interactive_wait_budget_ms
                    && transport_to_handler_wait > interactive_wait_budget_ms
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
        let measured_cold_query_bundle_samples = measured_samples
            .iter()
            .filter(|sample| {
                let query_bundle_total_ms = sample
                    .get("trace")
                    .and_then(|trace| trace.get("query_bundle_total_ms"))
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0);
                let fail_closed_cause = sample
                    .get("trace")
                    .and_then(|trace| trace.get("fail_closed_cause"))
                    .and_then(|value| value.as_str());
                query_bundle_total_ms > interactive_wait_budget_ms
                    && fail_closed_cause != Some("prepare_timeout")
            })
            .count();
        let worst_outlier_correlation_slice = measured_samples
            .iter()
            .max_by_key(|sample| {
                let trace = sample.get("trace");
                [
                    sample
                        .get("client_to_transport_wait_ms")
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0),
                    trace
                        .and_then(|trace| trace.get("read_loop_wait_ms"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0),
                    trace
                        .and_then(|trace| trace.get("admission_queue_wait_ms"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0),
                    trace
                        .and_then(|trace| trace.get("scheduler_poll_ready_wait_ms"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0),
                    trace
                        .and_then(|trace| trace.get("completion_barrier_wait_ms"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0),
                    trace
                        .and_then(|trace| trace.get("same_file_ingress_token_wait_ms"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0),
                    trace
                        .and_then(|trace| trace.get("scheduler_ready_to_dispatch_wait_ms"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0),
                    trace
                        .and_then(|trace| trace.get("adapter_to_dispatch_wait_ms"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0),
                ]
                .into_iter()
                .max()
                .unwrap_or(0)
            })
            .map(|sample| {
                let trace = sample.get("trace");
                let dominant_server_edge_buckets = [
                    (
                        "read_loop_wait_ms",
                        trace.and_then(|trace| trace.get("read_loop_wait_ms"))
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0),
                    ),
                    (
                        "admission_queue_wait_ms",
                        trace.and_then(|trace| trace.get("admission_queue_wait_ms"))
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0),
                    ),
                    (
                        "scheduler_poll_ready_wait_ms",
                        trace.and_then(|trace| trace.get("scheduler_poll_ready_wait_ms"))
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0),
                    ),
                    (
                        "completion_barrier_wait_ms",
                        trace.and_then(|trace| trace.get("completion_barrier_wait_ms"))
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0),
                    ),
                    (
                        "same_file_ingress_token_wait_ms",
                        trace.and_then(|trace| trace.get("same_file_ingress_token_wait_ms"))
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0),
                    ),
                    (
                        "scheduler_ready_to_dispatch_wait_ms",
                        trace.and_then(|trace| trace.get("scheduler_ready_to_dispatch_wait_ms"))
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0),
                    ),
                    (
                        "adapter_to_dispatch_wait_ms",
                        trace.and_then(|trace| trace.get("adapter_to_dispatch_wait_ms"))
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0),
                    ),
                ];
                let (dominant_server_edge_bucket, dominant_server_edge_wait_ms) =
                    dominant_server_edge_buckets
                        .into_iter()
                        .max_by_key(|(_, value)| *value)
                        .unwrap_or(("adapter_to_dispatch_wait_ms", 0));
                serde_json::json!({
                    "step": sample.get("step").and_then(|value| value.as_str()),
                    "request_id": sample.get("request_id").and_then(|value| value.as_i64()),
                    "version": sample.get("version").and_then(|value| value.as_i64()),
                    "elapsed_ms": sample.get("elapsed_ms").and_then(|value| value.as_u64()),
                    "completion_request_written_at_ms": sample
                        .get("completion_request_written_at_ms")
                        .and_then(|value| value.as_u64()),
                    "did_change_notifications_per_measured_completion": sample
                        .get("did_change_notifications_per_measured_completion")
                        .and_then(|value| value.as_u64()),
                    "did_save_after_did_change": sample
                        .get("did_save_after_did_change")
                        .and_then(|value| value.as_bool()),
                    "parse_gap_source": sample
                        .get("parse_gap_source")
                        .and_then(|value| value.as_str()),
                    "forced_shadow_fast_path_off_version": sample
                        .get("forced_shadow_fast_path_off_version")
                        .and_then(|value| value.as_i64()),
                    "dominant_server_edge_bucket": dominant_server_edge_bucket,
                    "dominant_server_edge_wait_ms": dominant_server_edge_wait_ms,
                    "client_to_transport_wait_ms": sample
                        .get("client_to_transport_wait_ms")
                        .and_then(|value| value.as_u64()),
                    "read_loop_wait_reason": trace
                        .and_then(|trace| trace.get("read_loop_wait_reason"))
                        .and_then(|value| value.as_str()),
                    "read_loop_wait_ms": trace
                        .and_then(|trace| trace.get("read_loop_wait_ms"))
                        .and_then(|value| value.as_u64()),
                    "pending_completion_spillover_depth": trace
                        .and_then(|trace| trace.get("pending_completion_spillover_depth"))
                        .and_then(|value| value.as_u64()),
                    "admission_queue_wait_ms": trace
                        .and_then(|trace| trace.get("admission_queue_wait_ms"))
                        .and_then(|value| value.as_u64()),
                    "scheduler_poll_ready_wait_ms": trace
                        .and_then(|trace| trace.get("scheduler_poll_ready_wait_ms"))
                        .and_then(|value| value.as_u64()),
                    "completion_barrier_wait_ms": trace
                        .and_then(|trace| trace.get("completion_barrier_wait_ms"))
                        .and_then(|value| value.as_u64()),
                    "completion_barrier_owner_method": trace
                        .and_then(|trace| trace.get("completion_barrier_owner_method"))
                        .and_then(|value| value.as_str()),
                    "completion_barrier_owner_uri": trace
                        .and_then(|trace| trace.get("completion_barrier_owner_uri"))
                        .and_then(|value| value.as_str()),
                    "completion_barrier_owner_version": trace
                        .and_then(|trace| trace.get("completion_barrier_owner_version"))
                        .and_then(|value| value.as_i64()),
                    "required_token_version": trace
                        .and_then(|trace| trace.get("same_file_ingress_token_required_version"))
                        .and_then(|value| value.as_i64()),
                    "current_published_token_version": trace
                        .and_then(|trace| trace.get("same_file_ingress_token_required_version"))
                        .and_then(|value| value.as_i64()),
                    "current_published_token_version_note": "derived from same-file publication observed by the sampled completion request context",
                    "same_file_ingress_token_published_at_ms": trace
                        .and_then(|trace| trace.get("same_file_ingress_token_published_at_ms"))
                        .and_then(|value| value.as_u64()),
                    "current_published_token_source": trace
                        .and_then(|trace| trace.get("same_file_ingress_token_source"))
                        .and_then(|value| value.as_str()),
                    "same_file_ingress_token_wait_ms": trace
                        .and_then(|trace| trace.get("same_file_ingress_token_wait_ms"))
                        .and_then(|value| value.as_u64()),
                    "scheduler_ready_to_dispatch_wait_ms": trace
                        .and_then(|trace| trace.get("scheduler_ready_to_dispatch_wait_ms"))
                        .and_then(|value| value.as_u64()),
                    "adapter_to_dispatch_wait_ms": trace
                        .and_then(|trace| trace.get("adapter_to_dispatch_wait_ms"))
                        .and_then(|value| value.as_u64()),
                    "wait_for_file_version_runtime_queue_wait_ms": trace
                        .and_then(|trace| trace.get("wait_for_file_version_runtime_queue_wait_ms"))
                        .and_then(|value| value.as_u64()),
                    "wait_for_file_version_runtime_resolution": trace
                        .and_then(|trace| trace.get("wait_for_file_version_runtime_resolution"))
                        .and_then(|value| value.as_str()),
                })
            })
            .unwrap_or(serde_json::json!(null));
        let parse_cold_start_evidence = serde_json::json!({
            "measured_cold_query_bundle_samples": measured_cold_query_bundle_samples,
            "measured_parse_snapshot_full_total_delta": measured_parse_snapshot_full_total_delta,
            "measured_parse_snapshot_incremental_total_delta": measured_parse_snapshot_incremental_total_delta,
            "measured_parse_snapshot_reused_total_delta": measured_parse_snapshot_reused_total_delta,
            "measured_parse_snapshot_no_previous_tree_total_delta": measured_parse_snapshot_no_previous_tree_total_delta,
            "measured_parse_snapshot_no_edits_total_delta": measured_parse_snapshot_no_edits_total_delta,
            "intellisense_v2_parse_snapshot_build_full_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_full",
                None
            ),
            "intellisense_v2_parse_snapshot_build_reused_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_reused",
                None
            ),
        });
        let apply_backlog_evidence = serde_json::json!({
            "measured_prepare_timeout_wait_for_file_version_samples": measured_prepare_timeout_wait_for_file_version_samples,
            "measured_wait_for_file_version_runtime_queue_wait_ms": measured_wait_for_file_version_runtime_queue_wait_histogram,
            "measured_wait_for_file_version_runtime_queue_wait_present_samples": measured_wait_for_file_version_runtime_queue_wait_present_samples,
            "measured_wait_for_file_version_runtime_queue_wait_max_ms": measured_wait_for_file_version_runtime_queue_wait_max_ms,
            "measured_wait_for_file_version_runtime_waiter_samples": measured_wait_for_file_version_runtime_waiter_samples,
            "measured_wait_for_file_version_runtime_immediate_samples": measured_wait_for_file_version_runtime_immediate_samples,
            "intellisense_v2_runtime_apply_changes_queue_wait_ms": measured_runtime_apply_changes_queue_wait_ms,
            "completion_stage_prepare_apply_age_at_start_ms": measured_prepare_apply_age_at_start_ms,
            "completion_stage_prepare_apply_age_at_terminal_ms": measured_prepare_apply_age_at_terminal_ms,
            "completion_stage_exact_wait_apply_age_at_start_ms": measured_exact_wait_apply_age_at_start_ms,
            "completion_stage_exact_wait_apply_age_at_terminal_ms": measured_exact_wait_apply_age_at_terminal_ms,
        });
        let truthful_pre_dispatch_split_evidence = serde_json::json!({
            "interactive_wait_budget_ms": interactive_wait_budget_ms,
            "truthful_wait_p95_budget_ms": truthful_wait_p95_budget_ms,
            "truthful_wait_max_budget_ms": truthful_wait_max_budget_ms,
            "measured_read_loop_wait_ms": measured_read_loop_wait_histogram,
            "measured_read_loop_wait_present_samples": measured_read_loop_wait_present_samples,
            "measured_read_loop_wait_max_ms": measured_read_loop_wait_max_ms,
            "measured_read_loop_wait_over_hard_cap_samples": measured_read_loop_wait_over_hard_cap_samples,
            "measured_admission_queue_wait_ms": measured_admission_queue_wait_histogram,
            "measured_admission_queue_wait_present_samples": measured_admission_queue_wait_present_samples,
            "measured_admission_queue_wait_max_ms": measured_admission_queue_wait_max_ms,
            "measured_admission_queue_wait_over_hard_cap_samples": measured_admission_queue_wait_over_hard_cap_samples,
            "measured_scheduler_poll_ready_wait_ms": measured_scheduler_poll_ready_wait_histogram,
            "measured_scheduler_poll_ready_wait_present_samples": measured_scheduler_poll_ready_wait_present_samples,
            "measured_scheduler_poll_ready_wait_max_ms": measured_scheduler_poll_ready_wait_max_ms,
            "measured_scheduler_poll_ready_wait_over_hard_cap_samples": measured_scheduler_poll_ready_wait_over_hard_cap_samples,
            "measured_completion_barrier_wait_ms": measured_completion_barrier_wait_histogram,
            "measured_completion_barrier_wait_present_samples": measured_completion_barrier_wait_present_samples,
            "measured_completion_barrier_wait_max_ms": measured_completion_barrier_wait_max_ms,
            "measured_completion_barrier_wait_over_hard_cap_samples": measured_completion_barrier_wait_over_hard_cap_samples,
            "measured_same_file_ingress_token_wait_ms": measured_same_file_ingress_token_wait_histogram,
            "measured_same_file_ingress_token_wait_present_samples": measured_same_file_ingress_token_wait_present_samples,
            "measured_same_file_ingress_token_wait_max_ms": measured_same_file_ingress_token_wait_max_ms,
            "measured_same_file_ingress_token_wait_over_hard_cap_samples": measured_same_file_ingress_token_wait_over_hard_cap_samples,
            "measured_same_file_ingress_token_published_samples": measured_same_file_ingress_token_published_samples,
            "measured_scheduler_ready_to_dispatch_wait_ms": measured_scheduler_ready_to_dispatch_wait_histogram,
            "measured_scheduler_ready_to_dispatch_wait_present_samples": measured_scheduler_ready_to_dispatch_wait_present_samples,
            "measured_scheduler_ready_to_dispatch_wait_max_ms": measured_scheduler_ready_to_dispatch_wait_max_ms,
            "measured_scheduler_ready_to_dispatch_wait_over_hard_cap_samples": measured_scheduler_ready_to_dispatch_wait_over_hard_cap_samples,
            "measured_truthful_pre_dispatch_bucket_shift_samples": measured_truthful_pre_dispatch_bucket_shift_samples,
        });
        let parse_cold_start_validation_pass = measured_cold_query_bundle_samples == 0
            && measured_parse_snapshot_no_previous_tree_total_delta == 0
            && measured_parse_snapshot_full_total_delta <= MEASURE_REQUESTS as u64;
        let apply_backlog_wait_for_file_version_p95_pass =
            measured_wait_for_file_version_runtime_queue_wait_present_samples == 0
                || measured_wait_for_file_version_runtime_queue_wait_p95_ms
                    <= APPLY_BACKLOG_P95_BUDGET_MS;
        let apply_backlog_wait_for_file_version_max_pass =
            measured_wait_for_file_version_runtime_queue_wait_present_samples == 0
                || measured_wait_for_file_version_runtime_queue_wait_max_ms as f64
                    <= APPLY_BACKLOG_MAX_BUDGET_MS as f64;
        let apply_backlog_runtime_apply_queue_p95_pass =
            measured_runtime_apply_changes_queue_wait_present_samples == 0
                || measured_runtime_apply_changes_queue_wait_p95_ms
                    <= APPLY_BACKLOG_P95_BUDGET_MS;
        let apply_backlog_runtime_apply_queue_max_pass =
            measured_runtime_apply_changes_queue_wait_present_samples == 0
                || measured_runtime_apply_changes_queue_wait_max_ms
                    <= APPLY_BACKLOG_MAX_BUDGET_MS as f64;
        let truthful_pre_dispatch_read_loop_p95_pass = measured_read_loop_wait_present_samples == 0
            || measured_read_loop_wait_p95_ms <= truthful_wait_p95_budget_ms;
        let truthful_pre_dispatch_read_loop_max_pass = measured_read_loop_wait_present_samples == 0
            || measured_read_loop_wait_max_ms <= truthful_wait_max_budget_ms;
        let truthful_pre_dispatch_admission_queue_p95_pass =
            measured_admission_queue_wait_present_samples == 0
                || measured_admission_queue_wait_p95_ms <= truthful_wait_p95_budget_ms;
        let truthful_pre_dispatch_admission_queue_max_pass =
            measured_admission_queue_wait_present_samples == 0
                || measured_admission_queue_wait_max_ms <= truthful_wait_max_budget_ms;
        let truthful_pre_dispatch_scheduler_poll_ready_p95_pass =
            measured_scheduler_poll_ready_wait_present_samples == 0
                || measured_scheduler_poll_ready_wait_p95_ms <= truthful_wait_p95_budget_ms;
        let truthful_pre_dispatch_scheduler_poll_ready_max_pass =
            measured_scheduler_poll_ready_wait_present_samples == 0
                || measured_scheduler_poll_ready_wait_max_ms <= truthful_wait_max_budget_ms;
        let truthful_pre_dispatch_completion_barrier_p95_pass =
            measured_completion_barrier_wait_present_samples == 0
                || measured_completion_barrier_wait_p95_ms <= truthful_wait_p95_budget_ms;
        let truthful_pre_dispatch_completion_barrier_max_pass =
            measured_completion_barrier_wait_present_samples == 0
                || measured_completion_barrier_wait_max_ms <= truthful_wait_max_budget_ms;
        let truthful_pre_dispatch_same_file_token_p95_pass =
            measured_same_file_ingress_token_wait_present_samples == 0
                || measured_same_file_ingress_token_wait_p95_ms <= truthful_wait_p95_budget_ms;
        let truthful_pre_dispatch_same_file_token_max_pass =
            measured_same_file_ingress_token_wait_present_samples == 0
                || measured_same_file_ingress_token_wait_max_ms <= truthful_wait_max_budget_ms;
        let truthful_pre_dispatch_scheduler_ready_to_dispatch_p95_pass =
            measured_scheduler_ready_to_dispatch_wait_present_samples == 0
                || measured_scheduler_ready_to_dispatch_wait_p95_ms
                    <= truthful_wait_p95_budget_ms;
        let truthful_pre_dispatch_scheduler_ready_to_dispatch_max_pass =
            measured_scheduler_ready_to_dispatch_wait_present_samples == 0
                || measured_scheduler_ready_to_dispatch_wait_max_ms
                    <= truthful_wait_max_budget_ms;
        let truthful_pre_dispatch_token_publication_pass =
            measured_same_file_ingress_token_published_samples == MEASURE_REQUESTS;
        let truthful_pre_dispatch_bucket_shift_pass =
            measured_truthful_pre_dispatch_bucket_shift_samples == 0;
        let apply_backlog_validation_pass = measured_prepare_timeout_wait_for_file_version_samples
            == 0
            && apply_backlog_wait_for_file_version_p95_pass
            && apply_backlog_wait_for_file_version_max_pass
            && apply_backlog_runtime_apply_queue_p95_pass
            && apply_backlog_runtime_apply_queue_max_pass;
        let truthful_pre_dispatch_split_validation_pass =
            truthful_pre_dispatch_read_loop_p95_pass
                && truthful_pre_dispatch_read_loop_max_pass
                && truthful_pre_dispatch_admission_queue_p95_pass
                && truthful_pre_dispatch_admission_queue_max_pass
                && truthful_pre_dispatch_scheduler_poll_ready_p95_pass
                && truthful_pre_dispatch_scheduler_poll_ready_max_pass
                && truthful_pre_dispatch_completion_barrier_p95_pass
                && truthful_pre_dispatch_completion_barrier_max_pass
                && truthful_pre_dispatch_same_file_token_p95_pass
                && truthful_pre_dispatch_same_file_token_max_pass
                && truthful_pre_dispatch_scheduler_ready_to_dispatch_p95_pass
                && truthful_pre_dispatch_scheduler_ready_to_dispatch_max_pass
                && truthful_pre_dispatch_token_publication_pass
                && truthful_pre_dispatch_bucket_shift_pass;
        let parse_cold_start_validation = serde_json::json!({
            "failure_class": "parse_cold_start",
            "status": if parse_cold_start_validation_pass { "pass" } else { "fail" },
            "checks": {
                "cold_query_bundle_samples": {
                    "status": if measured_cold_query_bundle_samples == 0 { "pass" } else { "fail" },
                    "observed": measured_cold_query_bundle_samples,
                    "expected_max": 0,
                },
                "same_version_no_previous_tree_total_delta": {
                    "status": if measured_parse_snapshot_no_previous_tree_total_delta == 0 { "pass" } else { "fail" },
                    "observed": measured_parse_snapshot_no_previous_tree_total_delta,
                    "expected_max": 0,
                },
                "same_version_full_parse_total_delta": {
                    "status": if measured_parse_snapshot_full_total_delta <= MEASURE_REQUESTS as u64 { "pass" } else { "fail" },
                    "observed": measured_parse_snapshot_full_total_delta,
                    "expected_max": MEASURE_REQUESTS,
                },
            },
        });
        let apply_backlog_validation = serde_json::json!({
            "failure_class": "apply_backlog",
            "status": if apply_backlog_validation_pass { "pass" } else { "fail" },
            "checks": {
                "prepare_timeout_wait_for_file_version_samples": {
                    "status": if measured_prepare_timeout_wait_for_file_version_samples == 0 { "pass" } else { "fail" },
                    "observed": measured_prepare_timeout_wait_for_file_version_samples,
                    "expected_max": 0,
                },
                "wait_for_file_version_runtime_queue_wait_p95_ms": {
                    "status": if apply_backlog_wait_for_file_version_p95_pass { "pass" } else { "fail" },
                    "observed_ms": measured_wait_for_file_version_runtime_queue_wait_p95_ms,
                    "budget_ms": APPLY_BACKLOG_P95_BUDGET_MS,
                    "present_samples": measured_wait_for_file_version_runtime_queue_wait_present_samples,
                },
                "wait_for_file_version_runtime_queue_wait_max_ms": {
                    "status": if apply_backlog_wait_for_file_version_max_pass { "pass" } else { "fail" },
                    "observed_ms": measured_wait_for_file_version_runtime_queue_wait_max_ms,
                    "budget_ms": APPLY_BACKLOG_MAX_BUDGET_MS,
                    "present_samples": measured_wait_for_file_version_runtime_queue_wait_present_samples,
                },
                "runtime_apply_changes_queue_wait_p95_ms": {
                    "status": if apply_backlog_runtime_apply_queue_p95_pass { "pass" } else { "fail" },
                    "observed_ms": measured_runtime_apply_changes_queue_wait_p95_ms,
                    "budget_ms": APPLY_BACKLOG_P95_BUDGET_MS,
                    "present_samples": measured_runtime_apply_changes_queue_wait_present_samples,
                },
                "runtime_apply_changes_queue_wait_max_ms": {
                    "status": if apply_backlog_runtime_apply_queue_max_pass { "pass" } else { "fail" },
                    "observed_ms": measured_runtime_apply_changes_queue_wait_max_ms,
                    "budget_ms": APPLY_BACKLOG_MAX_BUDGET_MS,
                    "present_samples": measured_runtime_apply_changes_queue_wait_present_samples,
                },
            },
        });
        let truthful_pre_dispatch_split_validation = serde_json::json!({
            "failure_class": "truthful_pre_dispatch_split",
            "status": if truthful_pre_dispatch_split_validation_pass { "pass" } else { "fail" },
            "checks": {
                "read_loop_wait_p95_ms": {
                    "status": if truthful_pre_dispatch_read_loop_p95_pass { "pass" } else { "fail" },
                    "observed_ms": measured_read_loop_wait_p95_ms,
                    "budget_ms": truthful_wait_p95_budget_ms,
                    "present_samples": measured_read_loop_wait_present_samples,
                },
                "read_loop_wait_max_ms": {
                    "status": if truthful_pre_dispatch_read_loop_max_pass { "pass" } else { "fail" },
                    "observed_ms": measured_read_loop_wait_max_ms,
                    "budget_ms": truthful_wait_max_budget_ms,
                    "present_samples": measured_read_loop_wait_present_samples,
                },
                "admission_queue_wait_p95_ms": {
                    "status": if truthful_pre_dispatch_admission_queue_p95_pass { "pass" } else { "fail" },
                    "observed_ms": measured_admission_queue_wait_p95_ms,
                    "budget_ms": truthful_wait_p95_budget_ms,
                    "present_samples": measured_admission_queue_wait_present_samples,
                },
                "admission_queue_wait_max_ms": {
                    "status": if truthful_pre_dispatch_admission_queue_max_pass { "pass" } else { "fail" },
                    "observed_ms": measured_admission_queue_wait_max_ms,
                    "budget_ms": truthful_wait_max_budget_ms,
                    "present_samples": measured_admission_queue_wait_present_samples,
                },
                "scheduler_poll_ready_wait_p95_ms": {
                    "status": if truthful_pre_dispatch_scheduler_poll_ready_p95_pass { "pass" } else { "fail" },
                    "observed_ms": measured_scheduler_poll_ready_wait_p95_ms,
                    "budget_ms": truthful_wait_p95_budget_ms,
                    "present_samples": measured_scheduler_poll_ready_wait_present_samples,
                },
                "scheduler_poll_ready_wait_max_ms": {
                    "status": if truthful_pre_dispatch_scheduler_poll_ready_max_pass { "pass" } else { "fail" },
                    "observed_ms": measured_scheduler_poll_ready_wait_max_ms,
                    "budget_ms": truthful_wait_max_budget_ms,
                    "present_samples": measured_scheduler_poll_ready_wait_present_samples,
                },
                "completion_barrier_wait_p95_ms": {
                    "status": if truthful_pre_dispatch_completion_barrier_p95_pass { "pass" } else { "fail" },
                    "observed_ms": measured_completion_barrier_wait_p95_ms,
                    "budget_ms": truthful_wait_p95_budget_ms,
                    "present_samples": measured_completion_barrier_wait_present_samples,
                },
                "completion_barrier_wait_max_ms": {
                    "status": if truthful_pre_dispatch_completion_barrier_max_pass { "pass" } else { "fail" },
                    "observed_ms": measured_completion_barrier_wait_max_ms,
                    "budget_ms": truthful_wait_max_budget_ms,
                    "present_samples": measured_completion_barrier_wait_present_samples,
                },
                "same_file_ingress_token_wait_p95_ms": {
                    "status": if truthful_pre_dispatch_same_file_token_p95_pass { "pass" } else { "fail" },
                    "observed_ms": measured_same_file_ingress_token_wait_p95_ms,
                    "budget_ms": truthful_wait_p95_budget_ms,
                    "present_samples": measured_same_file_ingress_token_wait_present_samples,
                },
                "same_file_ingress_token_wait_max_ms": {
                    "status": if truthful_pre_dispatch_same_file_token_max_pass { "pass" } else { "fail" },
                    "observed_ms": measured_same_file_ingress_token_wait_max_ms,
                    "budget_ms": truthful_wait_max_budget_ms,
                    "present_samples": measured_same_file_ingress_token_wait_present_samples,
                },
                "scheduler_ready_to_dispatch_wait_p95_ms": {
                    "status": if truthful_pre_dispatch_scheduler_ready_to_dispatch_p95_pass { "pass" } else { "fail" },
                    "observed_ms": measured_scheduler_ready_to_dispatch_wait_p95_ms,
                    "budget_ms": truthful_wait_p95_budget_ms,
                    "present_samples": measured_scheduler_ready_to_dispatch_wait_present_samples,
                },
                "scheduler_ready_to_dispatch_wait_max_ms": {
                    "status": if truthful_pre_dispatch_scheduler_ready_to_dispatch_max_pass { "pass" } else { "fail" },
                    "observed_ms": measured_scheduler_ready_to_dispatch_wait_max_ms,
                    "budget_ms": truthful_wait_max_budget_ms,
                    "present_samples": measured_scheduler_ready_to_dispatch_wait_present_samples,
                },
                "same_file_ingress_token_published_samples": {
                    "status": if truthful_pre_dispatch_token_publication_pass { "pass" } else { "fail" },
                    "observed": measured_same_file_ingress_token_published_samples,
                    "expected": MEASURE_REQUESTS,
                },
                "truthful_pre_dispatch_bucket_shift_samples": {
                    "status": if truthful_pre_dispatch_bucket_shift_pass { "pass" } else { "fail" },
                    "observed": measured_truthful_pre_dispatch_bucket_shift_samples,
                    "expected_max": 0,
                },
            },
        });
        let overall_validation_failing_classes = [
            (!parse_cold_start_validation_pass).then_some("parse_cold_start"),
            (!apply_backlog_validation_pass).then_some("apply_backlog"),
            (!truthful_pre_dispatch_split_validation_pass)
                .then_some("truthful_pre_dispatch_split"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let overall_validation = serde_json::json!({
            "status": if parse_cold_start_validation_pass
                && apply_backlog_validation_pass
                && truthful_pre_dispatch_split_validation_pass
            {
                "pass"
            } else {
                "fail"
            },
            "failing_classes": overall_validation_failing_classes,
        });

        let report = serde_json::json!({
            "change_id": change_id,
            "profile": PROFILE_NAME,
            "schema_version": 1,
            "configuration_path": conf_big_root,
            "module_path": module_path,
            "marker": MEASURED_COMPLETION_MARKER_MODE,
            "request_plan": {
                "cache_mode": "preseed_ready_outline_from_opened_text_then_same_file_mixed_load",
                "wait_for_current_revision_before_seed": true,
                "warmup_requests": WARMUP_REQUESTS,
                "waiter_ramp_requests": WAITER_RAMP_REQUESTS,
                "measured_requests": MEASURE_REQUESTS,
                "completion_trigger_mode": "invoked",
                "transport_path": "tower_lsp_server_serve_duplex",
                "mixed_load_profile": "didChange+didSave+documentSymbol_burst+currentContext+completion",
                "completion_wait_path_mode": "shadow_fast_path_forced_off_same_revision",
                "warmup_marker": WARMUP_COMPLETION_MARKER,
                "measured_marker_mode": MEASURED_COMPLETION_MARKER_MODE,
                "document_symbol_requests_per_measured_completion": DOCUMENT_SYMBOL_BURST_REQUESTS,
                "current_context_requests_per_measured_completion": CURRENT_CONTEXT_REQUESTS_PER_MEASURED_COMPLETION,
                "did_change_notifications_per_measured_completion": 1,
                "did_save_after_did_change": true,
                "runtime_apply_set_file_delay_ms": APPLY_DELAY_MS,
                "did_change_blocking_parse_delay_ms": 1500,
                "did_save_parse_delay_ms": 1500,
                "parse_gap_activation_sources": ["didChange", "didSave"],
            },
            "warmup_samples": warmup_samples,
            "ramp_samples": ramp_samples,
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
                "measured_completion_total_delta": counter_delta("completion_total"),
                "measured_ok_non_empty_total_delta": counter_delta("intellisense_v2_completion_result_total_ok_non_empty"),
                "measured_fail_closed_total_delta": counter_delta("intellisense_v2_completion_result_total_fail_closed"),
                "measured_prepare_timeout_total_delta": counter_delta(
                    "intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout"
                ),
                "measured_exact_deadline_total_delta": counter_delta(
                    "intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline"
                ),
                "measured_fallback_unavailable_total_delta": counter_delta("intellisense_v2_completion_fallback_unavailable_total"),
                "measured_interactive_wait_budget_exhausted_total_delta": counter_delta("intellisense_v2_interactive_wait_budget_exhausted_total"),
                "measured_document_symbol_requests_total": (MEASURE_REQUESTS * DOCUMENT_SYMBOL_BURST_REQUESTS) as u64,
                "measured_document_symbol_present_responses_total": measured_document_symbol_present_responses_total,
                "measured_document_symbol_null_responses_total": measured_document_symbol_null_responses_total,
                "measured_document_symbol_fresh_outline_leak_samples": measured_document_symbol_fresh_outline_leak_samples,
                "measured_current_context_response_seen_samples": measured_current_context_response_seen_samples,
                "measured_current_context_parse_attempts_total": measured_current_context_parse_attempts_total,
                "measured_parse_snapshot_full_total_delta": measured_parse_snapshot_full_total_delta,
                "measured_parse_snapshot_incremental_total_delta": measured_parse_snapshot_incremental_total_delta,
                "measured_parse_snapshot_reused_total_delta": measured_parse_snapshot_reused_total_delta,
                "measured_parse_snapshot_no_previous_tree_total_delta": measured_parse_snapshot_no_previous_tree_total_delta,
                "measured_parse_snapshot_no_edits_total_delta": measured_parse_snapshot_no_edits_total_delta,
                "measured_document_symbol_latest_ready_total_delta": measured_document_symbol_latest_ready_total_delta,
                "measured_document_symbol_current_ready_total_delta": measured_document_symbol_current_ready_total_delta,
                "measured_document_symbol_unavailable_total_delta": measured_document_symbol_unavailable_total_delta,
                "measured_document_symbol_superseded_total_delta": measured_document_symbol_superseded_total_delta,
                "measured_document_symbol_total_outcome_delta": measured_document_symbol_total_outcome_delta,
                "measured_ingress_regression_samples": measured_ingress_regression_samples,
                "measured_prepare_timeout_wait_for_file_version_samples": measured_prepare_timeout_wait_for_file_version_samples,
                "measured_cold_query_bundle_samples": measured_cold_query_bundle_samples,
                "measured_wait_for_file_version_runtime_queue_wait_ms": measured_wait_for_file_version_runtime_queue_wait_histogram,
                "measured_wait_for_file_version_runtime_queue_wait_present_samples": measured_wait_for_file_version_runtime_queue_wait_present_samples,
                "measured_wait_for_file_version_runtime_queue_wait_max_ms": measured_wait_for_file_version_runtime_queue_wait_max_ms,
                "measured_wait_for_file_version_runtime_waiter_samples": measured_wait_for_file_version_runtime_waiter_samples,
                "measured_wait_for_file_version_runtime_immediate_samples": measured_wait_for_file_version_runtime_immediate_samples,
                "intellisense_v2_runtime_apply_changes_queue_wait_ms": measured_runtime_apply_changes_queue_wait_ms,
                "completion_stage_prepare_apply_age_at_start_ms": measured_prepare_apply_age_at_start_ms,
                "completion_stage_prepare_apply_age_at_terminal_ms": measured_prepare_apply_age_at_terminal_ms,
                "completion_stage_exact_wait_apply_age_at_start_ms": measured_exact_wait_apply_age_at_start_ms,
                "completion_stage_exact_wait_apply_age_at_terminal_ms": measured_exact_wait_apply_age_at_terminal_ms,
                "measured_pre_dispatch_wait_over_budget_samples": measured_pre_dispatch_wait_over_budget_samples,
                "measured_pre_dispatch_wait_over_hard_cap_samples": measured_pre_dispatch_wait_over_hard_cap_samples,
                "interactive_wait_budget_ms": interactive_wait_budget_ms,
                "truthful_wait_p95_budget_ms": truthful_wait_p95_budget_ms,
                "truthful_wait_max_budget_ms": truthful_wait_max_budget_ms,
                "warmup_latency_ms": warmup_latency_histogram,
                "measured_latency_ms": measured_latency_histogram,
                "measured_read_loop_wait_ms": measured_read_loop_wait_histogram,
                "measured_read_loop_wait_max_ms": measured_read_loop_wait_max_ms,
                "measured_read_loop_wait_present_samples": measured_read_loop_wait_present_samples,
                "measured_read_loop_wait_over_hard_cap_samples": measured_read_loop_wait_over_hard_cap_samples,
                "measured_adapter_to_dispatch_wait_ms": measured_adapter_to_dispatch_histogram,
                "measured_adapter_to_dispatch_wait_max_ms": measured_adapter_to_dispatch_max_ms,
                "measured_admission_queue_wait_ms": measured_admission_queue_wait_histogram,
                "measured_admission_queue_wait_max_ms": measured_admission_queue_wait_max_ms,
                "measured_admission_queue_wait_present_samples": measured_admission_queue_wait_present_samples,
                "measured_admission_queue_wait_over_hard_cap_samples": measured_admission_queue_wait_over_hard_cap_samples,
                "measured_scheduler_poll_ready_wait_ms": measured_scheduler_poll_ready_wait_histogram,
                "measured_scheduler_poll_ready_wait_max_ms": measured_scheduler_poll_ready_wait_max_ms,
                "measured_scheduler_poll_ready_wait_present_samples": measured_scheduler_poll_ready_wait_present_samples,
                "measured_scheduler_poll_ready_wait_over_hard_cap_samples": measured_scheduler_poll_ready_wait_over_hard_cap_samples,
                "measured_completion_barrier_wait_ms": measured_completion_barrier_wait_histogram,
                "measured_completion_barrier_wait_max_ms": measured_completion_barrier_wait_max_ms,
                "measured_completion_barrier_wait_present_samples": measured_completion_barrier_wait_present_samples,
                "measured_completion_barrier_wait_over_hard_cap_samples": measured_completion_barrier_wait_over_hard_cap_samples,
                "measured_same_file_ingress_token_wait_ms": measured_same_file_ingress_token_wait_histogram,
                "measured_same_file_ingress_token_wait_max_ms": measured_same_file_ingress_token_wait_max_ms,
                "measured_same_file_ingress_token_wait_present_samples": measured_same_file_ingress_token_wait_present_samples,
                "measured_same_file_ingress_token_wait_over_hard_cap_samples": measured_same_file_ingress_token_wait_over_hard_cap_samples,
                "measured_same_file_ingress_token_published_samples": measured_same_file_ingress_token_published_samples,
                "measured_scheduler_ready_to_dispatch_wait_ms": measured_scheduler_ready_to_dispatch_wait_histogram,
                "measured_scheduler_ready_to_dispatch_wait_max_ms": measured_scheduler_ready_to_dispatch_wait_max_ms,
                "measured_scheduler_ready_to_dispatch_wait_present_samples": measured_scheduler_ready_to_dispatch_wait_present_samples,
                "measured_scheduler_ready_to_dispatch_wait_over_hard_cap_samples": measured_scheduler_ready_to_dispatch_wait_over_hard_cap_samples,
                "measured_truthful_pre_dispatch_bucket_shift_samples": measured_truthful_pre_dispatch_bucket_shift_samples,
                "measured_client_to_transport_wait_ms": measured_client_to_transport_histogram,
                "measured_client_to_transport_wait_max_ms": measured_client_to_transport_max_ms,
                "measured_client_to_transport_present_samples": measured_client_to_transport_present_samples,
                "measured_client_to_transport_over_hard_cap_samples": measured_client_to_transport_over_hard_cap_samples,
                "measured_service_future_to_first_poll_wait_ms": measured_service_future_first_poll_histogram,
                "measured_service_future_to_first_poll_wait_max_ms": measured_service_future_first_poll_max_ms,
                "measured_response_output_handoff_send_wait_ms": measured_response_output_handoff_send_histogram,
                "measured_response_output_handoff_send_wait_max_ms": measured_response_output_handoff_send_max_ms,
                "measured_response_output_handoff_send_over_hard_cap_samples": measured_response_output_handoff_send_over_hard_cap_samples,
                "measured_transport_to_handler_wait_ms": measured_transport_to_handler_histogram,
                "measured_transport_to_handler_wait_max_ms": measured_transport_to_handler_max_ms,
                "measured_dispatch_to_request_context_wait_ms": sample_trace_server_edge_histogram(
                    &measured_samples,
                    "dispatch_to_request_context_wait_ms"
                ),
                "measured_prepare_stateful_ms": sample_trace_histogram(&measured_samples, "prepare_stateful_ms"),
                "measured_wait_exact_type_index_ms": sample_trace_histogram(&measured_samples, "wait_exact_type_index_ms"),
                "measured_query_bundle_total_ms": sample_trace_histogram(
                    &measured_samples,
                    "query_bundle_total_ms",
                ),
                "measured_collect_ms": sample_trace_histogram(&measured_samples, "collect_ms"),
                "parse_cold_start_evidence": parse_cold_start_evidence,
                "apply_backlog_evidence": apply_backlog_evidence,
                "truthful_pre_dispatch_split_evidence": truthful_pre_dispatch_split_evidence,
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
                "intellisense_v2_parse_snapshot_build_full_ms": histogram_metric_value_or_zero(
                    histograms,
                    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_full",
                    None
                ),
                "intellisense_v2_parse_snapshot_build_reused_ms": histogram_metric_value_or_zero(
                    histograms,
                    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_reused",
                    None
                ),
            },
            "latest_trace_summaries": latest_trace_summaries,
            "completion_timeline": {
                "trace_count": filtered_traces.len(),
                "selected_traces": filtered_traces,
                "raw": completion_timeline,
            },
            "validation": {
                "parse_cold_start": parse_cold_start_validation,
                "apply_backlog": apply_backlog_validation,
                "truthful_pre_dispatch_split": truthful_pre_dispatch_split_validation,
                "overall": overall_validation,
            },
            "worst_outlier_correlation_slice": worst_outlier_correlation_slice,
            "observability": {
                "raw": observability_metrics,
            }
        });

        let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_DOCUMENT_SYMBOL_MIXED_LOAD_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!(
                        "{change_id}-real-conf-big-document-symbol-mixed-load-live.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p39 real conf_big perf report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report).expect("serialize p39 real conf_big perf report"),
        )
        .expect("write p39 real conf_big perf report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        assert!(
            trace_matching_mode == "request_id",
            "expected request-context parity to expose JSON-RPC request ids in completion timeline, trace_matching_mode={}, trace_request_id_present_total={}, filtered_traces={filtered_traces:?}",
            trace_matching_mode,
            trace_request_id_present_total
        );
        assert!(
            warmup_non_empty_samples == WARMUP_REQUESTS,
            "expected baseline warm-cache samples to be non-empty before outline churn, warmup_non_empty_samples={}, warmup_samples={warmup_samples:?}",
            warmup_non_empty_samples
        );
        assert!(
            measured_trace_linked_samples == MEASURE_REQUESTS,
            "expected every measured mixed-load sample to link to a completion timeline trace, measured_trace_linked_samples={}, measured_samples={measured_samples:?}",
            measured_trace_linked_samples
        );
        assert!(
            measured_client_to_transport_present_samples == MEASURE_REQUESTS,
            "expected every measured mixed-load sample to expose harness-derived client_to_transport_wait_ms, measured_client_to_transport_present_samples={}, measured_samples={measured_samples:?}",
            measured_client_to_transport_present_samples
        );
        assert!(
            counter_delta("completion_total") >= MEASURE_REQUESTS as u64,
            "expected measured completion_total delta >= mixed-load request samples, completion_total_delta={}, measured_requests={}",
            counter_delta("completion_total"),
            MEASURE_REQUESTS
        );
        assert!(
            measured_non_empty_samples == MEASURE_REQUESTS,
            "expected every measured mixed-load sample to return a first-response candidate list, measured_non_empty_samples={}, measured_samples={measured_samples:?}",
            measured_non_empty_samples
        );
        assert!(
            measured_ok_non_empty_traces == MEASURE_REQUESTS,
            "expected every measured mixed-load trace to be ok_non_empty, measured_ok_non_empty_traces={}, measured_samples={measured_samples:?}",
            measured_ok_non_empty_traces
        );
        assert!(
            measured_fail_closed_traces == 0,
            "measured waiter window must not keep fail_closed traces after ramp-up, measured_fail_closed_traces={}, measured_samples={measured_samples:?}",
            measured_fail_closed_traces,
        );
        assert!(
            counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline")
                == 0,
            "waiter workload must not regress into exact_deadline even while forcing same-file waiters, exact_deadline_total_delta={}, counters={counters:?}",
            counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline")
        );
        assert!(
            measured_prepare_timeout_wait_for_file_version_samples == 0,
            "mixed-load gate must fail on post-edit/save readiness timeout separately from cold query-body cost, prepare_timeout_wait_for_file_version_samples={}, measured_samples={measured_samples:?}",
            measured_prepare_timeout_wait_for_file_version_samples
        );
        assert!(
            measured_wait_for_file_version_runtime_queue_wait_present_samples == MEASURE_REQUESTS,
            "mixed-load gate must force every measured completion through wait_for_file_version instrumentation once shadow fast path is disabled, present_samples={}, measured_samples={measured_samples:?}",
            measured_wait_for_file_version_runtime_queue_wait_present_samples
        );
        assert!(
            measured_wait_for_file_version_runtime_waiter_samples == MEASURE_REQUESTS,
            "measured waiter window must keep every sample on a real waiter-path completion after forcing shadow fast path off, waiter_samples={}, measured_samples={measured_samples:?}",
            measured_wait_for_file_version_runtime_waiter_samples
        );
        if measured_wait_for_file_version_runtime_queue_wait_present_samples > 0 {
            assert!(
                measured_wait_for_file_version_runtime_queue_wait_p95_ms
                    <= APPLY_BACKLOG_P95_BUDGET_MS,
                "mixed-load apply-backlog p95 regression: measured_wait_for_file_version_runtime_queue_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_wait_for_file_version_runtime_queue_wait_p95_ms,
                APPLY_BACKLOG_P95_BUDGET_MS
            );
            assert!(
                measured_wait_for_file_version_runtime_queue_wait_max_ms as f64
                    <= APPLY_BACKLOG_MAX_BUDGET_MS as f64,
                "mixed-load apply-backlog max regression: measured_wait_for_file_version_runtime_queue_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_wait_for_file_version_runtime_queue_wait_max_ms,
                APPLY_BACKLOG_MAX_BUDGET_MS
            );
        }
        if measured_runtime_apply_changes_queue_wait_present_samples > 0 {
            assert!(
                measured_runtime_apply_changes_queue_wait_p95_ms
                    <= APPLY_BACKLOG_P95_BUDGET_MS,
                "mixed-load runtime apply queue p95 regression: intellisense_v2_runtime_apply_changes_queue_wait_ms p95={}ms > {}ms, metrics={histograms:?}",
                measured_runtime_apply_changes_queue_wait_p95_ms,
                APPLY_BACKLOG_P95_BUDGET_MS
            );
            assert!(
                measured_runtime_apply_changes_queue_wait_max_ms
                    <= APPLY_BACKLOG_MAX_BUDGET_MS as f64,
                "mixed-load runtime apply queue max regression: intellisense_v2_runtime_apply_changes_queue_wait_ms max={}ms > {}ms, metrics={histograms:?}",
                measured_runtime_apply_changes_queue_wait_max_ms,
                APPLY_BACKLOG_MAX_BUDGET_MS
            );
        }
        assert!(
            measured_adapter_to_dispatch_p95_ms <= interactive_wait_budget_ms as f64,
            "mixed-load pre-dispatch p95 regression: measured_adapter_to_dispatch_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_adapter_to_dispatch_p95_ms,
            interactive_wait_budget_ms
        );
        assert!(
            measured_adapter_to_dispatch_max_ms
                <= interactive_wait_budget_ms
                    .saturating_mul(ADAPTER_TO_DISPATCH_MAX_FACTOR),
            "mixed-load pre-dispatch max regression: measured_adapter_to_dispatch_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_adapter_to_dispatch_max_ms,
            interactive_wait_budget_ms
                .saturating_mul(ADAPTER_TO_DISPATCH_MAX_FACTOR)
        );
        assert!(
            measured_pre_dispatch_wait_over_hard_cap_samples == 0,
            "mixed-load gate must fail on seconds-scale pre-dispatch backlog under concurrent outline load, measured_pre_dispatch_wait_over_hard_cap_samples={}, measured_samples={measured_samples:?}",
            measured_pre_dispatch_wait_over_hard_cap_samples
        );
        assert!(
            measured_same_file_ingress_token_published_samples == MEASURE_REQUESTS,
            "mixed-load gate must observe same-file ingress token publication for every measured completion, measured_same_file_ingress_token_published_samples={}, measured_samples={measured_samples:?}",
            measured_same_file_ingress_token_published_samples
        );
        if measured_read_loop_wait_present_samples > 0 {
            assert!(
                measured_read_loop_wait_p95_ms <= truthful_wait_p95_budget_ms,
                "mixed-load reader-wait p95 regression: measured_read_loop_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_read_loop_wait_p95_ms,
                truthful_wait_p95_budget_ms
            );
            assert!(
                measured_read_loop_wait_max_ms <= truthful_wait_max_budget_ms,
                "mixed-load reader-wait max regression: measured_read_loop_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_read_loop_wait_max_ms,
                truthful_wait_max_budget_ms
            );
        }
        if measured_admission_queue_wait_present_samples > 0 {
            assert!(
                measured_admission_queue_wait_p95_ms <= truthful_wait_p95_budget_ms,
                "mixed-load admission-queue p95 regression: measured_admission_queue_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_admission_queue_wait_p95_ms,
                truthful_wait_p95_budget_ms
            );
            assert!(
                measured_admission_queue_wait_max_ms <= truthful_wait_max_budget_ms,
                "mixed-load admission-queue max regression: measured_admission_queue_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_admission_queue_wait_max_ms,
                truthful_wait_max_budget_ms
            );
        }
        if measured_scheduler_poll_ready_wait_present_samples > 0 {
            assert!(
                measured_scheduler_poll_ready_wait_p95_ms <= truthful_wait_p95_budget_ms,
                "mixed-load scheduler-poll-ready p95 regression: measured_scheduler_poll_ready_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_scheduler_poll_ready_wait_p95_ms,
                truthful_wait_p95_budget_ms
            );
            assert!(
                measured_scheduler_poll_ready_wait_max_ms <= truthful_wait_max_budget_ms,
                "mixed-load scheduler-poll-ready max regression: measured_scheduler_poll_ready_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_scheduler_poll_ready_wait_max_ms,
                truthful_wait_max_budget_ms
            );
        }
        if measured_completion_barrier_wait_present_samples > 0 {
            assert!(
                measured_completion_barrier_wait_p95_ms <= truthful_wait_p95_budget_ms,
                "mixed-load completion-barrier p95 regression: measured_completion_barrier_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_completion_barrier_wait_p95_ms,
                truthful_wait_p95_budget_ms
            );
            assert!(
                measured_completion_barrier_wait_max_ms <= truthful_wait_max_budget_ms,
                "mixed-load completion-barrier max regression: measured_completion_barrier_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_completion_barrier_wait_max_ms,
                truthful_wait_max_budget_ms
            );
        }
        if measured_same_file_ingress_token_wait_present_samples > 0 {
            assert!(
                measured_same_file_ingress_token_wait_p95_ms <= truthful_wait_p95_budget_ms,
                "mixed-load same-file-token p95 regression: measured_same_file_ingress_token_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_same_file_ingress_token_wait_p95_ms,
                truthful_wait_p95_budget_ms
            );
            assert!(
                measured_same_file_ingress_token_wait_max_ms <= truthful_wait_max_budget_ms,
                "mixed-load same-file-token max regression: measured_same_file_ingress_token_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_same_file_ingress_token_wait_max_ms,
                truthful_wait_max_budget_ms
            );
        }
        if measured_scheduler_ready_to_dispatch_wait_present_samples > 0 {
            assert!(
                measured_scheduler_ready_to_dispatch_wait_p95_ms <= truthful_wait_p95_budget_ms,
                "mixed-load scheduler-ready-to-dispatch p95 regression: measured_scheduler_ready_to_dispatch_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_scheduler_ready_to_dispatch_wait_p95_ms,
                truthful_wait_p95_budget_ms
            );
            assert!(
                measured_scheduler_ready_to_dispatch_wait_max_ms <= truthful_wait_max_budget_ms,
                "mixed-load scheduler-ready-to-dispatch max regression: measured_scheduler_ready_to_dispatch_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
                measured_scheduler_ready_to_dispatch_wait_max_ms,
                truthful_wait_max_budget_ms
            );
        }
        assert!(
            measured_truthful_pre_dispatch_bucket_shift_samples == 0,
            "mixed-load gate must fail if seconds-scale wait merely shifts into a newly exposed pre-dispatch bucket, measured_truthful_pre_dispatch_bucket_shift_samples={}, measured_samples={measured_samples:?}",
            measured_truthful_pre_dispatch_bucket_shift_samples
        );
        assert!(
            measured_client_to_transport_p95_ms <= truthful_wait_p95_budget_ms,
            "mixed-load truthful ingress p95 regression: measured_client_to_transport_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_client_to_transport_p95_ms,
            truthful_wait_p95_budget_ms
        );
        assert!(
            measured_client_to_transport_max_ms <= truthful_wait_max_budget_ms,
            "mixed-load truthful ingress max regression: measured_client_to_transport_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_client_to_transport_max_ms,
            truthful_wait_max_budget_ms
        );
        assert!(
            measured_client_to_transport_over_hard_cap_samples == 0,
            "mixed-load gate must fail on seconds-scale truthful ingress backlog, measured_client_to_transport_over_hard_cap_samples={}, measured_samples={measured_samples:?}",
            measured_client_to_transport_over_hard_cap_samples
        );
        assert!(
            measured_service_future_first_poll_p95_ms <= SERVICE_FUTURE_FIRST_POLL_P95_BUDGET_MS,
            "mixed-load first-poll p95 regression: measured_service_future_to_first_poll_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_service_future_first_poll_p95_ms,
            SERVICE_FUTURE_FIRST_POLL_P95_BUDGET_MS
        );
        assert!(
            measured_service_future_first_poll_max_ms <= SERVICE_FUTURE_FIRST_POLL_MAX_BUDGET_MS,
            "mixed-load first-poll max regression: measured_service_future_to_first_poll_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_service_future_first_poll_max_ms,
            SERVICE_FUTURE_FIRST_POLL_MAX_BUDGET_MS
        );
        assert!(
            measured_response_output_handoff_send_p95_ms <= truthful_wait_p95_budget_ms,
            "mixed-load truthful handoff p95 regression: measured_response_output_handoff_send_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_response_output_handoff_send_p95_ms,
            truthful_wait_p95_budget_ms
        );
        assert!(
            measured_response_output_handoff_send_max_ms <= truthful_wait_max_budget_ms,
            "mixed-load truthful handoff max regression: measured_response_output_handoff_send_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
            measured_response_output_handoff_send_max_ms,
            truthful_wait_max_budget_ms
        );
        assert!(
            measured_response_output_handoff_send_over_hard_cap_samples == 0,
            "mixed-load gate must fail on seconds-scale truthful output-handoff backlog, measured_response_output_handoff_send_over_hard_cap_samples={}, measured_samples={measured_samples:?}",
            measured_response_output_handoff_send_over_hard_cap_samples
        );
        assert!(
            measured_document_symbol_latest_ready_total_delta > 0,
            "mixed-load gate must observe latest_ready outline outcomes, counters={counters:?}"
        );
        assert!(
            measured_document_symbol_current_ready_total_delta == 0,
            "mixed-load gate intentionally samples documentSymbol during parse gap and must not observe current_ready outcomes, counters={counters:?}"
        );
        assert!(
            measured_document_symbol_unavailable_total_delta == 0,
            "mixed-load gate must keep outline path bounded by latest_ready/superseded, unavailable_total_delta={}, counters={counters:?}",
            measured_document_symbol_unavailable_total_delta
        );
        assert!(
            measured_document_symbol_fresh_outline_leak_samples == 0,
            "mixed-load gate must not leak current outline payload while serving latest_ready cache, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_document_symbol_present_responses_total > 0,
            "mixed-load gate must keep at least one non-null outline response across measured samples, measured_samples={measured_samples:?}"
        );
        assert!(
            measured_document_symbol_present_responses_total
                == (MEASURE_REQUESTS * DOCUMENT_SYMBOL_BURST_REQUESTS) as u64
                && measured_document_symbol_null_responses_total == 0,
            "measured waiter window must keep every outline response bounded and non-null, present_total={}, null_total={}, measured_samples={measured_samples:?}",
            measured_document_symbol_present_responses_total,
            measured_document_symbol_null_responses_total
        );
        assert!(
            measured_parse_snapshot_no_previous_tree_total_delta <= 1,
            "measured waiter workload must not degrade into repeated cold-start tree misses on a warmed conf_big file, measured_parse_snapshot_no_previous_tree_total_delta={}, counters={counters:?}",
            measured_parse_snapshot_no_previous_tree_total_delta
        );
        assert!(
            measured_current_context_parse_attempts_total > 0,
            "mixed-load gate must exercise current-context as a real auxiliary parse-only load instead of a no-op request stream, measured_current_context_parse_attempts_total={}, measured_samples={measured_samples:?}",
            measured_current_context_parse_attempts_total
        );
        assert!(
            measured_parse_snapshot_full_total_delta <= TOTAL_MIXED_LOAD_REQUESTS as u64,
            "mixed-load gate must fail on repeated identical same-version full parse beyond the single didChange fallback per measured cycle, measured_parse_snapshot_full_total_delta={}, measured_requests={}, counters={counters:?}",
            measured_parse_snapshot_full_total_delta,
            TOTAL_MIXED_LOAD_REQUESTS
        );

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}
