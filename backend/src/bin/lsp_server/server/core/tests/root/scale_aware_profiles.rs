#[allow(clippy::too_many_arguments)]
async fn run_scale_aware_profile(
    profile_name: &str,
    uri: Url,
    text: String,
    position: Position,
    phases: &[ScaleAwarePhase],
    churn_mode: ScaleAwareChurnMode,
    churn_every: u64,
    workspace_setup: Option<&ScaleAwareWorkspaceSetup>,
    observability_probe: Option<ScaleAwareObservabilityProbe>,
) -> serde_json::Value {
    let mut profile_report = serde_json::Map::new();
    let progress_enabled = scale_aware_progress_enabled();
    let progress_every = scale_aware_progress_every();

    for (phase_index, phase) in phases.iter().enumerate() {
        let phase_started = Instant::now();
        let mut progress_line_width = 0usize;
        let coordinator = Arc::new(SystemCoordinator::new());
        let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
            Arc::new(std::sync::Mutex::new(None));
        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            let server_holder = server_holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *server_holder.lock().expect("server holder lock") = Some(server.clone());
                server
            }
        })
        .finish();
        let mut drain_task =
            tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        initialize_lsp_service(&mut service).await;

        let server = server_holder
            .lock()
            .expect("server holder lock")
            .clone()
            .expect("server must be created");
        if let Some(setup) = workspace_setup {
            prime_server_with_workspace_setup(
                &server,
                setup,
                "p31_scale_aware_real_workspace_setup",
            )
            .await;
        }

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.clone(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let mut current_text = text.clone();
        let mut current_version: i32 = 1;
        let mut churn_edits_applied = 0u64;
        let mut observability_ok_latencies_ms = Vec::new();
        let mut observability_timeouts_total = 0u64;
        let mut observability_errors_total = 0u64;

        let total_requests = phase.warmup + phase.iterations;
        if progress_enabled {
            emit_scale_aware_progress_line(
                &format!(
                    "[p31] profile={} phase={} progress=0/{} (0.0%) elapsed_ms=0 eta_ms=0 churn_edits=0 warmup={} iterations={} churn_mode={} churn_every={} progress_every={}",
                    profile_name,
                    phase.name,
                    total_requests,
                    phase.warmup,
                    phase.iterations,
                    churn_mode.as_str(),
                    churn_every,
                    progress_every
                ),
                &mut progress_line_width,
            );
        }
        for request_index in 0..total_requests {
            if should_apply_scale_aware_churn(
                churn_mode,
                profile_name,
                *phase,
                request_index,
                churn_every,
            ) {
                let end_position = utf16_end_position(&current_text);
                let churn_payload = if churn_edits_applied.is_multiple_of(2) {
                    " "
                } else {
                    "\n"
                };
                let next_version = current_version
                    .checked_add(1)
                    .expect("scale-aware churn version overflow");
                let did_change = DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier {
                        uri: uri.clone(),
                        version: next_version,
                    },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: Some(Range {
                            start: end_position,
                            end: end_position,
                        }),
                        range_length: None,
                        text: churn_payload.to_string(),
                    }],
                };
                let did_change_req = Request::build("textDocument/didChange")
                    .params(
                        serde_json::to_value(did_change)
                            .expect("scale-aware churn didChange params"),
                    )
                    .finish();
                let did_change_response = service
                    .ready()
                    .await
                    .unwrap()
                    .call(did_change_req)
                    .await
                    .expect("scale-aware churn didChange notification");
                assert!(did_change_response.is_none(), "didChange is a notification");
                current_version = next_version;
                current_text.push_str(churn_payload);
                churn_edits_applied += 1;
            }

            let completion = server
                .completion(CompletionParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position,
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                    context: Some(CompletionContext {
                        trigger_kind: CompletionTriggerKind::INVOKED,
                        trigger_character: None,
                    }),
                })
                .await
                .expect("completion request");
            assert!(
                completion.is_some(),
                "completion response expected for profile={profile_name}, phase={}",
                phase.name
            );

            if progress_enabled
                && should_emit_scale_aware_progress(request_index, total_requests, progress_every)
            {
                let completed = request_index + 1;
                let elapsed = phase_started.elapsed();
                let progress_percent = scale_aware_progress_percent(completed, total_requests);
                let eta_ms = scale_aware_progress_eta_ms(elapsed, completed, total_requests);
                emit_scale_aware_progress_line(
                    &format!(
                        "[p31] profile={} phase={} progress={}/{} ({:.1}%) elapsed_ms={} eta_ms={} churn_edits={}",
                        profile_name,
                        phase.name,
                        completed,
                        total_requests,
                        progress_percent,
                        elapsed.as_millis(),
                        eta_ms,
                        churn_edits_applied
                    ),
                    &mut progress_line_width,
                );
            }

            if let Some(probe) = observability_probe {
                if should_probe_scale_aware_observability(*phase, request_index, probe.every) {
                    let request_id = 31_000_000_i64
                        .saturating_add((phase_index as i64) * 100_000)
                        .saturating_add(request_index as i64);
                    let (outcome, latency_ms) = probe_observability_sidebar_latency(
                        &mut service,
                        request_id,
                        probe.timeout,
                    )
                    .await;
                    match outcome {
                        ScaleAwareObservabilityProbeOutcome::Ok => {
                            if let Some(latency_ms) = latency_ms {
                                observability_ok_latencies_ms.push(latency_ms);
                            }
                        }
                        ScaleAwareObservabilityProbeOutcome::TimedOut => {
                            observability_timeouts_total += 1;
                        }
                        ScaleAwareObservabilityProbeOutcome::Error => {
                            observability_errors_total += 1;
                        }
                    }
                }
            }
        }

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");
        let gauges = metrics
            .get("gauges")
            .and_then(|value| value.as_object())
            .expect("metrics.gauges object");

        let completion_total = read_u64_metric(counters.get("completion_total"));
        let completion_cancelled_total =
            read_u64_metric(counters.get("intellisense_v2_completion_result_total_cancelled"));
        let completion_cancelled_rate =
            completion_cancelled_total as f64 / completion_total.max(1) as f64;
        let completion_ok_non_empty_total =
            read_u64_metric(counters.get("intellisense_v2_completion_result_total_ok_non_empty"));
        let completion_ok_empty_total =
            read_u64_metric(counters.get("intellisense_v2_completion_result_total_ok_empty"));
        let completion_fail_closed_total =
            read_u64_metric(counters.get("intellisense_v2_completion_result_total_fail_closed"));

        let mut phase_metrics = serde_json::json!({
            "completion_duration_ms": histogram_metric_value(histograms, "completion_duration_ms", None),
            "intellisense_v2_wait_for_file_version_completion_ms": histogram_metric_value(
                histograms,
                "intellisense_v2_wait_for_file_version_completion_ms",
                Some("intellisense_v2_wait_for_file_version_other_ms")
            ),
            "intellisense_v2_snapshot_completion_ms": histogram_metric_value(
                histograms,
                "intellisense_v2_snapshot_completion_ms",
                Some("intellisense_v2_snapshot_other_ms")
            ),
            "intellisense_v2_ir_query_completion_ms": histogram_metric_value(
                histograms,
                "intellisense_v2_ir_query_completion_ms",
                Some("intellisense_v2_ir_query_other_ms")
            ),
            "intellisense_v2_parse_result_query_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_parse_result_query_ms",
                None
            ),
            "intellisense_v2_singleflight_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_singleflight_wait_ms",
                None
            ),
            "intellisense_v2_runtime_exec_interactive_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_exec_interactive_ms",
                None
            ),
            "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms",
                None
            ),
            "intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms",
                None
            ),
            "intellisense_v2_runtime_apply_changes_queue_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_changes_queue_wait_ms",
                None
            ),
            "intellisense_v2_runtime_apply_changes_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_changes_exec_ms",
                None
            ),
            "intellisense_v2_runtime_apply_change_set_file_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_change_set_file_exec_ms",
                None
            ),
            "intellisense_v2_runtime_apply_change_set_file_with_snapshot_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_change_set_file_with_snapshot_exec_ms",
                None
            ),
            "intellisense_v2_runtime_apply_change_remove_file_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_change_remove_file_exec_ms",
                None
            ),
            "intellisense_v2_runtime_apply_change_set_settings_snapshot_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_change_set_settings_snapshot_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_queue_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_queue_wait_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_build_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_build_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_ir_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_ir_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms",
                None
            ),
            "intellisense_v2_runtime_apply_changes_batch_size": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_changes_batch_size",
                None
            ),
            "intellisense_v2_runtime_apply_changes_changed_files_count": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_changes_changed_files_count",
                None
            ),
            "completion_stage_turn_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_turn_wait_ms",
                None
            ),
            "completion_stage_prepare_stateful_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_prepare_stateful_ms",
                None
            ),
            "completion_stage_prepare_apply_age_at_start_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_prepare_apply_age_at_start_ms",
                None
            ),
            "completion_stage_prepare_apply_age_at_terminal_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_prepare_apply_age_at_terminal_ms",
                None
            ),
            "completion_stage_sync_globals_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_sync_globals_ms",
                None
            ),
            "completion_stage_exact_wait_apply_age_at_start_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_exact_wait_apply_age_at_start_ms",
                None
            ),
            "completion_stage_exact_wait_apply_age_at_terminal_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_exact_wait_apply_age_at_terminal_ms",
                None
            ),
            "completion_stage_query_bundle_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_extract_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_extract_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_offset_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_offset_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_flow_lookup_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_flow_lookup_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_direct_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_direct_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_fallback_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_fallback_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_unattributed_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_unattributed_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_other_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_other_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_cancellation_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_cancellation_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_to_first_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_to_first_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_idle_before_first_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_idle_before_first_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_age_at_query_start_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_age_at_query_start_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_first_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_first_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_fetch_end_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_fetch_end_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_total_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_total_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_inputs_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_inputs_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_parse_result_query_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_parse_result_query_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_build_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_build_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_parse_result_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_parse_result_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_total_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_total_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_seed_context_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_seed_context_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_local_function_summaries_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_local_function_summaries_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_visit_statements_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_visit_statements_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_scan_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_scan_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_ms",
                None
            ),
            "completion_stage_query_bundle_deps_and_file_snapshot_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_deps_and_file_snapshot_ms",
                None
            ),
            "completion_stage_response_build_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_response_build_ms",
                None
            ),
            "completion_stage_cache_store_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_cache_store_ms",
                None
            ),
            "completion_stage_snapshot_read_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_snapshot_read_ms",
                None
            ),
            "completion_stage_collect_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_collect_ms",
                None
            ),
            "completion_stage_rank_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_rank_ms",
                None
            ),
            "completion_stage_format_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_format_ms",
                None
            ),
            "intellisense_v2_completion_owner_hint_line_len_chars": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_line_len_chars",
                None
            ),
            "intellisense_v2_completion_owner_hint_receiver_len_chars": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_receiver_len_chars",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_active": read_numeric_metric(
                gauges.get("intellisense_v2_completion_owner_hint_index_fetch_active")
            ),
            "intellisense_v2_runtime_queue_wait_interactive_ms": histogram_metric_value(
                histograms,
                "intellisense_v2_runtime_queue_wait_interactive_ms",
                None
            ),
            "intellisense_v2_syntax_diagnostics_query_ms": histogram_metric_value(
                histograms,
                "intellisense_v2_syntax_diagnostics_query_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query_ms": histogram_metric_value(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query_inputs_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_inputs_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query_parse_result_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_parse_result_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query_ir_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_ir_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query_collect_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_collect_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query_flow_sensitive_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_flow_sensitive_ms",
                None
            ),
            "intellisense_v2_interactive_wait_budget_exhausted_total": read_u64_metric(
                counters.get("intellisense_v2_interactive_wait_budget_exhausted_total")
            ),
            "intellisense_v2_interactive_stale_served_total": read_u64_metric(
                counters.get("intellisense_v2_interactive_stale_served_total")
            ),
            "intellisense_v2_completion_stale_fallback_total": read_u64_metric(
                counters.get("intellisense_v2_completion_stale_fallback_total")
            ),
            "intellisense_v2_completion_fallback_unavailable_total": read_u64_metric(
                counters.get("intellisense_v2_completion_fallback_unavailable_total")
            ),
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready": read_u64_metric(
                counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready")
            ),
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline": read_u64_metric(
                counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline")
            ),
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task": read_u64_metric(
                counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task")
            ),
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_task_present_wrong_version": read_u64_metric(
                counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_task_present_wrong_version")
            ),
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_observed_version_mismatch": read_u64_metric(
                counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_observed_version_mismatch")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_type_index_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_type_index_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_parse_result_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_parse_result_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_other_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_other_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_type_index_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_type_index_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_parse_result_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_parse_result_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_other_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_other_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_type_index_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_type_index_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_parse_result_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_parse_result_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_other_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_other_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_check_cancellation_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_check_cancellation_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_revision_start": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_revision_start",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_revision_end": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_revision_end",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_revision_delta": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_revision_delta",
                None
            ),
        });
        phase_metrics["intellisense_v2_completion_owner_hint_result_total"] = serde_json::json!({
            "not_member_access": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_not_member_access")
            ),
            "no_file_content": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_no_file_content")
            ),
            "no_line": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_no_line")
            ),
            "no_dot": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_no_dot")
            ),
            "no_receiver": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_no_receiver")
            ),
            "offset_unresolved": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_offset_unresolved")
            ),
            "flow_type_hit": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_flow_type_hit")
            ),
            "flow_type_miss": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_flow_type_miss")
            ),
            "type_hit": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_type_hit")
            ),
            "type_miss": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_type_miss")
            ),
            "cancelled": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_cancelled")
            ),
            "other": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_other")
            )
        });
        phase_metrics["intellisense_v2_completion_owner_hint_lookup_path_total"] = serde_json::json!({
            "direct": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_direct")
            ),
            "flow_only": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_flow_only")
            ),
            "flow_plus_fallback": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_flow_plus_fallback")
            ),
            "other": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_other")
            )
        });
        phase_metrics["intellisense_v2_completion_owner_hint_lookup_result_total"] = serde_json::json!({
            "hit": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_result_total_hit")
            ),
            "miss": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_result_total_miss")
            ),
            "cancelled": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_result_total_cancelled")
            ),
            "error": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_result_total_error")
            ),
            "other": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_result_total_other")
            )
        });
        phase_metrics["intellisense_v2_completion_owner_hint_index_fetch_block_on_total_by_kind"] = serde_json::json!({
            "total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_total")
            ),
            "type_index": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_type_index_total")
            ),
            "parse_result": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_parse_result_total")
            ),
            "other": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_other_total")
            )
        });
        phase_metrics["intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_total_by_kind"] = serde_json::json!({
            "total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_total")
            ),
            "type_index": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_type_index_total")
            ),
            "parse_result": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_parse_result_total")
            ),
            "other": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_other_total")
            )
        });
        phase_metrics["intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_total_by_kind"] = serde_json::json!({
            "total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_total")
            ),
            "type_index": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_type_index_total")
            ),
            "parse_result": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_parse_result_total")
            ),
            "other": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_other_total")
            )
        });
        let dominant_stage = dominant_stage_from_metrics(&phase_metrics);
        let phase_report = serde_json::json!({
            "warmup": phase.warmup,
            "iterations": phase.iterations,
            "profile_size": profile_name,
            "churn_mode": churn_mode.as_str(),
            "completion_total": completion_total,
            "completion_outcomes": {
                "ok_non_empty": completion_ok_non_empty_total,
                "ok_empty": completion_ok_empty_total,
                "fail_closed": completion_fail_closed_total,
                "cancelled": completion_cancelled_total,
            },
            "completion_cancelled_total": completion_cancelled_total,
            "completion_cancelled_rate": completion_cancelled_rate,
            "observability_sidebar_probe": {
                "enabled": observability_probe.is_some(),
                "every": observability_probe.map(|probe| probe.every).unwrap_or(0),
                "timeout_ms": observability_probe
                    .map(|probe| probe.timeout.as_millis().min(u64::MAX as u128) as u64)
                    .unwrap_or(0),
                "ok_total": observability_ok_latencies_ms.len(),
                "timeout_total": observability_timeouts_total,
                "error_total": observability_errors_total,
                "request_ms": sample_histogram_value(&observability_ok_latencies_ms),
            },
            "churn_edits_applied": churn_edits_applied,
            "metrics": phase_metrics,
            "dominant_stage": dominant_stage
        });
        if progress_enabled {
            emit_scale_aware_progress_line(
                &format!(
                    "[p31] profile={} phase={} done progress={}/{} (100.0%) elapsed_ms={} eta_ms=0 completion_total={} cancelled_total={} cancelled_rate={:.4} churn_edits={}",
                    profile_name,
                    phase.name,
                    total_requests,
                    total_requests,
                    phase_started.elapsed().as_millis(),
                    completion_total,
                    completion_cancelled_total,
                    completion_cancelled_rate,
                    churn_edits_applied
                ),
                &mut progress_line_width,
            );
            println!();
        }
        profile_report.insert(phase.name.to_string(), phase_report);

        shutdown_lsp_service(&mut service, Some(&uri)).await;
        drop(server);
        drop(service);

        if tokio::time::timeout(Duration::from_millis(500), &mut drain_task)
            .await
            .is_err()
        {
            drain_task.abort();
        }
    }

    serde_json::Value::Object(profile_report)
}

fn synthetic_scale_aware_profile(
    completion_p95: f64,
    wait_p95: f64,
    completion_total: u64,
    completion_cancelled_total: u64,
) -> serde_json::Value {
    let phase = |completion_count: u64, completion_p95_value: f64, wait_p95_value: f64| {
        serde_json::json!({
            "completion_total": completion_count,
            "completion_cancelled_total": 0,
            "metrics": {
                "completion_duration_ms": {
                    "count": completion_count,
                    "p50": completion_p95_value,
                    "p95": completion_p95_value,
                    "p99": completion_p95_value
                },
                "intellisense_v2_wait_for_file_version_completion_ms": {
                    "count": completion_count,
                    "p50": wait_p95_value,
                    "p95": wait_p95_value,
                    "p99": wait_p95_value
                },
                "intellisense_v2_snapshot_completion_ms": {
                    "count": completion_count,
                    "p50": 0.0,
                    "p95": 0.0,
                    "p99": 0.0
                },
                "intellisense_v2_ir_query_completion_ms": {
                    "count": completion_count,
                    "p50": 0.0,
                    "p95": 0.0,
                    "p99": 0.0
                },
                "intellisense_v2_completion_stale_fallback_total": 0,
                "intellisense_v2_interactive_wait_budget_exhausted_total": 0,
                "intellisense_v2_completion_fallback_unavailable_total": 0,
                "intellisense_v2_interactive_stale_served_total": 0
            }
        })
    };
    serde_json::json!({
        "start": phase(1, completion_p95, wait_p95),
        "cold": phase(5, completion_p95, wait_p95),
        "warm": {
            "completion_total": completion_total,
            "completion_cancelled_total": completion_cancelled_total,
            "metrics": {
                "completion_duration_ms": {
                    "count": completion_total,
                    "p50": completion_p95,
                    "p95": completion_p95,
                    "p99": completion_p95
                },
                "intellisense_v2_wait_for_file_version_completion_ms": {
                    "count": completion_total,
                    "p50": wait_p95,
                    "p95": wait_p95,
                    "p99": wait_p95
                },
                "intellisense_v2_snapshot_completion_ms": {
                    "count": completion_total,
                    "p50": 0.0,
                    "p95": 0.0,
                    "p99": 0.0
                },
                "intellisense_v2_ir_query_completion_ms": {
                    "count": completion_total,
                    "p50": 0.0,
                    "p95": 0.0,
                    "p99": 0.0
                },
                "intellisense_v2_completion_stale_fallback_total": 0,
                "intellisense_v2_interactive_wait_budget_exhausted_total": 0,
                "intellisense_v2_completion_fallback_unavailable_total": 0,
                "intellisense_v2_interactive_stale_served_total": 0
            }
        }
    })
}

fn synthetic_scale_aware_report(
    change_id: &str,
    large_completion_p95: f64,
    large_wait_p95: f64,
    small_completion_p95: f64,
    small_wait_p95: f64,
) -> serde_json::Value {
    serde_json::json!({
        "change_id": change_id,
        "profile": "p31_scale_aware_large_small_completion_gate_live",
        "schema_version": 1,
        "profiles": {
            "large": synthetic_scale_aware_profile(large_completion_p95, large_wait_p95, 60, 0),
            "small": synthetic_scale_aware_profile(small_completion_p95, small_wait_p95, 60, 0)
        }
    })
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p57_real_conf_big_cold_start_hover_on_tablznach1_is_ready_without_completion_live() {
    init_test_tracing();
    let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();

    let Some(conf_big_root) = conf_big_root_for_tests() else {
        if allow_fixture_skip {
            eprintln!(
                "skipping p57 real conf_big cold-start hover reproducer: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
            );
            return;
        }
        panic!(
            "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly"
        );
    };

    let module_path = conf_big_root
        .join("Documents")
        .join("РеализацияТоваровУслуг")
        .join("Forms")
        .join("ФормаДокументаОбщая")
        .join("Ext")
        .join("Form")
        .join("Module.bsl");
    if !module_path.exists() {
        if allow_fixture_skip {
            eprintln!(
                "skipping p57 real conf_big cold-start hover reproducer: module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
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
        std::fs::read_to_string(&module_path).expect("read conf_big module text for p57 hover");
    let workspace_setup = ScaleAwareWorkspaceSetup {
        platform_docs_archive: syntax_helper_path_for_tests(),
        configuration_path: conf_big_root.clone(),
        platform_version: "8.3.25".to_string(),
    };
    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_workspace_setup(&server, &workspace_setup, "p57_real_conf_big_live_setup")
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
    tokio::time::timeout(Duration::from_secs(10), async {
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
    .expect("didOpen must publish latest received version for p57 real conf_big module");

    assert!(
        server.analysis_v2.wait_for_file_version(file_id, 1).await,
        "analysis runtime must catch up to the opened real conf_big module before the cold-start hover probe"
    );
    let hover_position = find_utf16_position_at_marker_tail(&module_text, "ТаблЗнач1");
    let ready_status = match tokio::time::timeout(Duration::from_secs(120), async {
        loop {
            let status = server.snapshot_status_for_uri_v2(&uri).await;
            if status.requested_version == Some(1)
                && status.ready_version == Some(1)
                && status.state == SnapshotReadinessStateDto::Ready
                && status.exact
            {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    {
        Ok(status) => status,
        Err(_) => {
            let status_after_timeout = server.snapshot_status_for_uri_v2(&uri).await;
            let exact_ready_after_timeout = server
                .analysis_v2
                .snapshot()
                .await
                .current_type_index_serve_only_ready(file_id)
                .expect("current_type_index_serve_only_ready after p57 timeout");
            let background_parse_task_state = {
                let tasks = server.background_parse_snapshot_apply_tasks_v2.lock().await;
                tasks.get(&file_id).map(|task| {
                    let target = task
                        .target
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .clone();
                    (
                        target.requested_version,
                        target.source,
                        super::super::BackgroundParseSnapshotApplyTaskPhaseV2::from_raw(
                            task.control.phase.load(Ordering::SeqCst),
                        ),
                        task.control.promotion_requested.load(Ordering::SeqCst),
                        task.control.materialized.load(Ordering::SeqCst),
                    )
                })
            };
            let type_index_task_state = {
                let tasks = server.type_index_precompute_tasks_v2.lock().await;
                tasks.get(&file_id).map(|task| {
                    (
                        task.supersession_key.requested_version,
                        task.work_class,
                        super::deps_and_precompute::TypeIndexPrecomputePhaseV2::from_atomic(
                            task.phase.load(Ordering::Relaxed),
                        ),
                        task.active_requested_version.load(Ordering::Relaxed),
                    )
                })
            };
            panic!(
                "same-revision ready/exact publish did not complete on real conf_big within timeout; file={}, status_after_timeout={{requested_version={:?}, ready_version={:?}, exact={}, task_state={:?}, state={:?}, phase={:?}, trigger={:?}}}, exact_ready_after_timeout={}, background_parse_task_state={background_parse_task_state:?}, type_index_task_state={type_index_task_state:?}",
                module_path.display(),
                status_after_timeout.requested_version,
                status_after_timeout.ready_version,
                status_after_timeout.exact,
                status_after_timeout.task_state,
                status_after_timeout.state,
                status_after_timeout.phase,
                status_after_timeout.trigger,
                exact_ready_after_timeout,
            );
        }
    };

    let exact_ready_before_hover = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready before hover in p57");
    assert!(
        exact_ready_before_hover,
        "snapshot status must not report ready/exact before the current exact type index is actually published, status={ready_status:?}"
    );

    let hover_response = tokio::time::timeout(
        Duration::from_secs(2),
        harness.send_request(
            57_100_001,
            "textDocument/hover",
            HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: hover_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        ),
    )
    .await
    .expect("cold-start hover after same-revision ready/exact publish must stay bounded");
    let first_hover_text =
        hover_text_from_jsonrpc_response(&hover_response).expect("cold-start hover text");
    let first_hover_trace = take_test_request_server_edge_trace(57_100_001).await;
    assert!(
        first_hover_text.contains("ТаблицаЗначений"),
        "cold-start hover on real conf_big Module.bsl must expose ТаблицаЗначений for line 37 variable ТаблЗнач1 once same-revision ready/exact publish completes, hover={first_hover_text}, status={ready_status:?}, trace={first_hover_trace:?}",
    );

    live_transport_close_document(&mut harness, &uri).await;
    harness.shutdown().await;
}

async fn wait_for_snapshot_status_notification(
    harness: &mut LiveLspTransportHarness,
    timeout: Duration,
) -> SnapshotReadinessDto {
    tokio::time::timeout(timeout, async {
        loop {
            let message = harness.read_message().await;
            if message.get("method").and_then(|value| value.as_str()) != Some("bsl/snapshotStatus")
            {
                continue;
            }
            let params = message
                .get("params")
                .cloned()
                .expect("snapshot status params");
            break serde_json::from_value(params).expect("snapshot status params dto");
        }
    })
    .await
    .expect("timed out waiting for snapshot status notification")
}

async fn assert_no_snapshot_status_notification(
    harness: &mut LiveLspTransportHarness,
    timeout: Duration,
) {
    let maybe_notification = tokio::time::timeout(timeout, async {
        loop {
            let message = harness.read_message().await;
            if message.get("method").and_then(|value| value.as_str()) == Some("bsl/snapshotStatus")
            {
                break message;
            }
        }
    })
    .await;

    assert!(
        maybe_notification.is_err(),
        "unexpected snapshot status notification: {:?}",
        maybe_notification.ok()
    );
}
