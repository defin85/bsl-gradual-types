use super::*;

pub(super) fn record_completion_owner_hint_type_lookup_profile(
    coordinator: &bsl_runtime::system::SystemCoordinator,
    profile: &bsl_analysis_v2::TypeAtByteOffsetProfile,
    apply_age_at_query_start_ms: Option<u128>,
) {
    let ms_to_duration =
        |value_ms: u128| std::time::Duration::from_millis(value_ms.min(u64::MAX as u128) as u64);

    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch",
        ms_to_duration(profile.index_fetch_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_wait",
        ms_to_duration(profile.index_fetch_wait_ms),
    );
    if profile.index_fetch_wait_ms > 0 {
        coordinator.record_completion_resource_pressure(
            "lock_wait",
            ms_to_duration(profile.index_fetch_wait_ms),
        );
    }
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_unattributed",
        ms_to_duration(profile.index_fetch_unattributed_ms),
    );
    if profile.index_fetch_unattributed_ms > 0 {
        coordinator.record_completion_resource_pressure(
            "other",
            ms_to_duration(profile.index_fetch_unattributed_ms),
        );
    }
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait",
        ms_to_duration(profile.index_fetch_pre_first_salsa_event_wait_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail",
        ms_to_duration(profile.index_fetch_post_last_salsa_event_tail_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window",
        ms_to_duration(profile.index_fetch_inside_salsa_window_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index",
        ms_to_duration(profile.index_fetch_first_will_execute_type_index_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_type_index",
        ms_to_duration(profile.index_fetch_last_will_execute_type_index_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result",
        ms_to_duration(profile.index_fetch_first_will_execute_parse_result_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other",
        ms_to_duration(profile.index_fetch_first_will_execute_other_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result",
        ms_to_duration(profile.index_fetch_last_will_execute_parse_result_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_other",
        ms_to_duration(profile.index_fetch_last_will_execute_other_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle",
        ms_to_duration(profile.index_fetch_first_will_iterate_cycle_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle",
        ms_to_duration(profile.index_fetch_last_will_iterate_cycle_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation",
        ms_to_duration(profile.index_fetch_first_will_check_cancellation_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_cancellation",
        ms_to_duration(profile.index_fetch_last_will_check_cancellation_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index",
        ms_to_duration(profile.index_fetch_first_will_check_to_first_will_execute_type_index_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_to_first_will_execute_type_index",
        ms_to_duration(profile.index_fetch_last_will_check_to_first_will_execute_type_index_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index",
        ms_to_duration(
            profile.index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index_ms,
        ),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_idle_before_first_will_execute_type_index",
        ms_to_duration(profile.index_fetch_idle_before_first_will_execute_type_index_ms),
    );
    if let Some(apply_age_ms) = apply_age_at_query_start_ms {
        coordinator.record_completion_stage_latency(
            "query_bundle_owner_hint_type_lookup_index_fetch_apply_age_at_query_start",
            ms_to_duration(apply_age_ms),
        );
        if apply_age_ms > 0 {
            coordinator.record_completion_resource_pressure(
                "queue_backpressure",
                ms_to_duration(apply_age_ms),
            );
        }
        coordinator.record_completion_stage_latency(
            "query_bundle_owner_hint_type_lookup_index_fetch_apply_to_fetch_end",
            ms_to_duration(apply_age_ms.saturating_add(profile.index_fetch_ms)),
        );
        if profile.index_fetch_first_will_execute_type_index_seen_total > 0 {
            coordinator.record_completion_stage_latency(
                "query_bundle_owner_hint_type_lookup_index_fetch_apply_to_first_will_execute_type_index",
                ms_to_duration(
                    apply_age_ms.saturating_add(profile.index_fetch_first_will_execute_type_index_ms),
                ),
            );
        }
    }
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_query_total",
        ms_to_duration(profile.index_query_total_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_query_inputs",
        ms_to_duration(profile.index_query_inputs_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_query_parse_result_query",
        ms_to_duration(profile.index_query_parse_result_query_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_query_build",
        ms_to_duration(profile.index_query_build_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_parse_result",
        ms_to_duration(profile.index_parse_result_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_build_total",
        ms_to_duration(profile.index_build_total_ms),
    );
    if profile.index_build_total_ms > 0 {
        coordinator.record_completion_resource_pressure(
            "allocator_pressure",
            ms_to_duration(profile.index_build_total_ms),
        );
    }
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_build_seed_context",
        ms_to_duration(profile.index_build_seed_module_context_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_build_local_function_summaries",
        ms_to_duration(profile.index_build_local_function_summaries_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_build_visit_statements",
        ms_to_duration(profile.index_build_visit_statements_ms),
    );
    coordinator.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_scan",
        ms_to_duration(profile.index_scan_ms),
    );
    coordinator.record_intellisense_v2_completion_owner_hint_index_fetch_salsa_counters(
        bsl_runtime::system::basic_observability::CompletionOwnerHintIndexFetchSalsaCounters {
            block_on_total: profile.index_fetch_will_block_on_total,
            block_on_type_index_total: profile.index_fetch_will_block_on_type_index_total,
            block_on_parse_result_total: profile.index_fetch_will_block_on_parse_result_total,
            block_on_other_total: profile.index_fetch_will_block_on_other_total,
            will_execute_total: profile.index_fetch_will_execute_total,
            will_execute_type_index_total: profile.index_fetch_will_execute_type_index_total,
            will_execute_parse_result_total: profile.index_fetch_will_execute_parse_result_total,
            will_execute_other_total: profile.index_fetch_will_execute_other_total,
            did_validate_memoized_total: profile.index_fetch_did_validate_memoized_total,
            did_validate_memoized_type_index_total: profile
                .index_fetch_did_validate_memoized_type_index_total,
            did_validate_memoized_parse_result_total: profile
                .index_fetch_did_validate_memoized_parse_result_total,
            did_validate_memoized_other_total: profile
                .index_fetch_did_validate_memoized_other_total,
            will_check_cancellation_total: profile.index_fetch_will_check_cancellation_total,
        },
    );
    coordinator.record_intellisense_v2_completion_owner_hint_index_fetch_active_gauge(
        profile.index_fetch_active_at_entry,
    );
    coordinator
        .record_intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch(
            profile.index_fetch_will_check_cancellation_total,
        );
    coordinator
        .record_intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch(
            profile.index_fetch_will_execute_other_total,
        );
    coordinator
        .record_intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch(
            profile.index_fetch_will_iterate_cycle_total,
        );
    coordinator.record_intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch(
        profile.index_fetch_did_set_cancellation_flag_total,
    );
    coordinator.record_intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch(
        profile.index_fetch_global_did_set_cancellation_flag_total,
    );
    coordinator.record_intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch(
        profile.index_fetch_did_discard_total,
    );
    coordinator
        .record_intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch(
            profile.index_fetch_did_discard_accumulated_total,
        );
    coordinator.record_intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch(
        profile.index_fetch_events_before_first_will_execute_type_index_total,
    );
    coordinator.record_intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch(
        profile.index_fetch_will_check_before_first_will_execute_type_index_total,
    );
    coordinator.record_intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch(
        profile.index_fetch_will_execute_parse_result_before_first_will_execute_type_index_total,
    );
    coordinator.record_intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch(
        profile.index_fetch_first_will_execute_type_index_seen_total,
    );
    coordinator.record_intellisense_v2_completion_owner_hint_index_fetch_revision_start(
        profile.index_fetch_revision_start,
    );
    coordinator.record_intellisense_v2_completion_owner_hint_index_fetch_revision_end(
        profile.index_fetch_revision_end,
    );
    coordinator.record_intellisense_v2_completion_owner_hint_index_fetch_revision_delta(
        profile.index_fetch_revision_delta,
    );
}

pub(super) fn compute_member_access_owner_hint(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    position: Position,
    member_access_request_for_query: bool,
    file_content: Option<&str>,
    coordinator: &bsl_runtime::system::SystemCoordinator,
    apply_age_at_query_start_ms: Option<u128>,
) -> Option<bsl_shared::domain::types::TypeResolution> {
    let owner_hint_started = Instant::now();
    let mut owner_hint_reason = "not_member_access";
    let mut owner_hint_line_len_chars: Option<usize> = None;
    let mut owner_hint_receiver_len_chars: Option<usize> = None;
    let mut owner_hint_lookup_path: Option<&'static str> = None;
    let mut owner_hint_lookup_result: Option<&'static str> = None;

    let member_access_owner_type_hint = if member_access_request_for_query {
        let owner_hint_extract_started = Instant::now();
        let extracted_owner_hint = (|| {
            let text = match file_content {
                Some(text) => text,
                None => {
                    owner_hint_reason = "no_file_content";
                    return None;
                }
            };
            let line_text = match text.lines().nth(position.line as usize) {
                Some(line_text) => line_text,
                None => {
                    owner_hint_reason = "no_line";
                    return None;
                }
            };
            owner_hint_line_len_chars = Some(line_text.chars().count());
            let cursor_byte = bsl_backend::system::positioning::utf16_to_byte_offset(
                line_text,
                position.character,
            );
            let line_prefix = match line_text.get(..cursor_byte) {
                Some(line_prefix) => line_prefix,
                None => {
                    owner_hint_reason = "no_dot";
                    return None;
                }
            };
            let line_prefix = if line_text
                .get(cursor_byte..)
                .and_then(|tail| tail.chars().next())
                == Some('.')
            {
                line_text.get(..cursor_byte + 1).unwrap_or(line_prefix)
            } else {
                line_prefix
            };
            let dot_in_line = match line_prefix.rfind('.') {
                Some(dot_in_line) => dot_in_line,
                None => {
                    owner_hint_reason = "no_dot";
                    return None;
                }
            };
            let receiver = match line_prefix.get(..dot_in_line) {
                Some(receiver) => receiver.trim_end(),
                None => {
                    owner_hint_reason = "no_receiver";
                    return None;
                }
            };
            owner_hint_receiver_len_chars = Some(receiver.chars().count());
            let (probe_byte, _) = match receiver
                .char_indices()
                .rev()
                .find(|(_, ch)| !ch.is_whitespace())
            {
                Some(probe) => probe,
                None => {
                    owner_hint_reason = "no_receiver";
                    return None;
                }
            };
            Some(bsl_backend::system::positioning::byte_offset_to_utf16(
                line_text, probe_byte,
            ))
        })();
        coordinator.record_completion_stage_latency(
            "query_bundle_owner_hint_extract",
            owner_hint_extract_started.elapsed(),
        );
        match extracted_owner_hint {
            Some(probe_utf16) => {
                let owner_hint_offset_started = Instant::now();
                let offset = analysis
                    .utf16_position_to_byte_offset(file_id, position.line, probe_utf16)
                    .ok()
                    .flatten();
                coordinator.record_completion_stage_latency(
                    "query_bundle_owner_hint_offset",
                    owner_hint_offset_started.elapsed(),
                );
                match offset {
                    Some(offset) => {
                        let offset = offset.min(u32::MAX as usize) as u32;
                        let owner_hint_type_lookup_started = Instant::now();
                        let hint = {
                            owner_hint_lookup_path = Some("direct");
                            let direct_started = Instant::now();
                            let type_hint_result =
                                analysis.type_at_byte_offset_serve_only_profiled(file_id, offset);
                            coordinator.record_completion_stage_latency(
                                "query_bundle_owner_hint_type_lookup_direct",
                                direct_started.elapsed(),
                            );
                            match type_hint_result {
                                Ok(profiled) => {
                                    record_completion_owner_hint_type_lookup_profile(
                                        coordinator,
                                        &profiled.profile,
                                        apply_age_at_query_start_ms,
                                    );
                                    coordinator.record_intellisense_v2_type_index_reason(
                                        profiled.serve_reason_code.as_str(),
                                    );
                                    coordinator
                                        .record_intellisense_v2_completion_owner_hint_result(
                                            profiled.serve_reason_code.as_str(),
                                        );
                                    if let Some(type_hint) = profiled.resolution {
                                        owner_hint_reason = "type_hit";
                                        owner_hint_lookup_result = Some("hit");
                                        Some(type_hint)
                                    } else {
                                        owner_hint_reason = "type_miss";
                                        owner_hint_lookup_result = Some("miss");
                                        None
                                    }
                                }
                                Err(_) => {
                                    owner_hint_reason = "cancelled";
                                    owner_hint_lookup_result = Some("cancelled");
                                    None
                                }
                            }
                        };
                        coordinator.record_completion_stage_latency(
                            "query_bundle_owner_hint_type_lookup",
                            owner_hint_type_lookup_started.elapsed(),
                        );
                        hint
                    }
                    None => {
                        owner_hint_reason = "offset_unresolved";
                        None
                    }
                }
            }
            None => None,
        }
    } else {
        None
    };

    coordinator.record_intellisense_v2_completion_owner_hint_result(owner_hint_reason);
    if let Some(path) = owner_hint_lookup_path {
        coordinator.record_intellisense_v2_completion_owner_hint_lookup_path(path);
    }
    if let Some(result) = owner_hint_lookup_result {
        coordinator.record_intellisense_v2_completion_owner_hint_lookup_result(result);
    }
    if let (Some(line_len_chars), Some(receiver_len_chars)) =
        (owner_hint_line_len_chars, owner_hint_receiver_len_chars)
    {
        coordinator.record_intellisense_v2_completion_owner_hint_context(
            line_len_chars,
            receiver_len_chars,
        );
    }
    coordinator
        .record_completion_stage_latency("query_bundle_owner_hint", owner_hint_started.elapsed());

    member_access_owner_type_hint
}

pub(super) struct CompletionResultMetricsContext<'a> {
    pub(super) member_access_observed: bool,
    pub(super) trigger_mode: &'a str,
    pub(super) observed_file_version_for_completion: Option<i32>,
    pub(super) file_id: bsl_analysis_v2::FileId,
    pub(super) position: &'a Position,
}

pub(super) async fn observe_completion_result_metrics(
    server: &BslLanguageServer,
    completion: &Option<crate::handlers::CompletionResponseWithStats>,
    completion_outcome: &mut Option<&'static str>,
    ctx: CompletionResultMetricsContext<'_>,
) {
    let CompletionResultMetricsContext {
        member_access_observed,
        trigger_mode,
        observed_file_version_for_completion,
        file_id,
        position,
    } = ctx;

    if let Some(result) = completion {
        if result.had_error {
            server.coordinator.record_completion_error();
        }

        let items_count = match &result.response {
            CompletionResponse::List(list) => {
                if list.is_incomplete {
                    server.coordinator.record_completion_incomplete();
                }
                list.items.len()
            }
            CompletionResponse::Array(items) => items.len(),
        };
        server
            .coordinator
            .record_intellisense_v2_completion_items_count(items_count);

        if let Some(stats) = &result.stats {
            server
                .coordinator
                .record_completion_stage_latency("snapshot_read", stats.stage_snapshot_read);
            server
                .coordinator
                .record_completion_stage_latency("collect", stats.stage_collect);
            server
                .coordinator
                .record_completion_stage_latency("rank", stats.stage_rank);
            server
                .coordinator
                .record_completion_stage_latency("format", stats.stage_format);
        }

        if bsl_runtime::system::global_runtime_config()
            .get_bool(bsl_runtime::system::RuntimeKey::CompletionQuality)
            .unwrap_or(false)
        {
            if let Some(stats) = &result.stats {
                server.coordinator.record_completion_quality(
                    stats.total_candidates,
                    stats.dedup_removed,
                    &stats.score_samples,
                    stats.prefix_exact,
                    stats.prefix_starts,
                    stats.prefix_contains,
                    stats.prefix_none,
                    stats.member_access,
                    stats.has_owner,
                );
            }
        }

        if completion_outcome.is_none() {
            *completion_outcome = Some(if result.had_error {
                "handler_error"
            } else if items_count == 0 {
                "ok_empty"
            } else {
                "ok_non_empty"
            });
        }

        if member_access_observed && !result.had_error && items_count == 0 {
            server
                .coordinator
                .record_intellisense_v2_completion_member_access_terminal_empty(
                    trigger_mode,
                    completion_outcome.unwrap_or("ok_empty"),
                );
        }

        if member_access_observed && matches!(trigger_mode, "trigger_character" | "invoked") {
            if let Some(observed_file_version) = observed_file_version_for_completion {
                let key = (
                    file_id,
                    observed_file_version,
                    position.line,
                    position.character,
                );
                let non_empty = items_count > 0;
                let labels = completion_labels_fingerprint(&result.response);
                let parity_result = {
                    let mut parity = server.completion_parity_state_v2.write().await;
                    let entry = parity.entry(key).or_default();
                    if trigger_mode == "trigger_character" {
                        entry.trigger_character_non_empty = Some(non_empty);
                        entry.trigger_character_labels = Some(labels.clone());
                    } else {
                        entry.invoked_non_empty = Some(non_empty);
                        entry.invoked_labels = Some(labels.clone());
                    }
                    match (
                        entry.trigger_character_non_empty,
                        entry.invoked_non_empty,
                        entry.trigger_character_labels.as_ref(),
                        entry.invoked_labels.as_ref(),
                    ) {
                        (
                            Some(trigger_non_empty),
                            Some(invoked_non_empty),
                            Some(trigger_labels),
                            Some(invoked_labels),
                        ) => {
                            let overlap_ratio =
                                completion_labels_overlap_ratio(trigger_labels, invoked_labels);
                            let mismatch = trigger_non_empty != invoked_non_empty
                                || (trigger_non_empty && invoked_non_empty && overlap_ratio <= 0.0);
                            parity.remove(&key);
                            Some((mismatch, overlap_ratio))
                        }
                        _ => None,
                    }
                };
                if let Some((parity_drift, overlap_ratio)) = parity_result {
                    server
                        .coordinator
                        .record_intellisense_v2_completion_parity_overlap_bucket(
                            trigger_mode,
                            completion_parity_overlap_bucket(overlap_ratio),
                        );
                    if parity_drift {
                        server
                            .coordinator
                            .record_intellisense_v2_completion_parity_drift(trigger_mode);
                    }
                }
            }
        }
    }
}
