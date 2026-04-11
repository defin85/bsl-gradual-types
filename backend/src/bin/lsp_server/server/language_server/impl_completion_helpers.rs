use super::*;

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
            let collect_detail_stages = if member_access_observed {
                vec![
                    (
                        "collect_member_owner_resolve",
                        stats.collect_breakdown.member_owner_resolve,
                    ),
                    (
                        "collect_member_methods",
                        stats.collect_breakdown.member_methods,
                    ),
                    (
                        "collect_member_properties",
                        stats.collect_breakdown.member_properties,
                    ),
                    (
                        "collect_member_metadata",
                        stats.collect_breakdown.member_metadata,
                    ),
                ]
            } else {
                vec![
                    (
                        "collect_non_member_local_symbols",
                        stats.collect_breakdown.non_member_local_symbols,
                    ),
                    (
                        "collect_non_member_contextual_symbols",
                        stats.collect_breakdown.non_member_contextual_symbols,
                    ),
                    (
                        "collect_non_member_module_routines",
                        stats.collect_breakdown.non_member_module_routines,
                    ),
                    (
                        "collect_non_member_global_functions",
                        stats.collect_breakdown.non_member_global_functions,
                    ),
                    (
                        "collect_non_member_metadata_items",
                        stats.collect_breakdown.non_member_metadata_items,
                    ),
                    (
                        "collect_non_member_repository_types",
                        stats.collect_breakdown.non_member_repository_types,
                    ),
                    (
                        "collect_non_member_keywords",
                        stats.collect_breakdown.non_member_keywords,
                    ),
                ]
            };
            for (stage, duration) in collect_detail_stages {
                server
                    .coordinator
                    .record_completion_stage_latency(stage, duration);
            }
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
