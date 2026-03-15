use super::*;

impl BasicObservability {
    pub fn record_completion_latency(&self, duration: Duration) {
        self.metrics.increment("completion_total");
        self.metrics
            .observe_histogram("completion_duration_ms", duration.as_millis() as f64);
    }

    pub fn record_completion_stage_latency(&self, stage: &str, duration: Duration) {
        let metric = match stage {
            "snapshot_read" => "completion_stage_snapshot_read_ms",
            "collect" => "completion_stage_collect_ms",
            "rank" => "completion_stage_rank_ms",
            "format" => "completion_stage_format_ms",
            "turn_wait" => "completion_stage_turn_wait_ms",
            "prepare_stateful" => "completion_stage_prepare_stateful_ms",
            "prepare_apply_age_at_start" => "completion_stage_prepare_apply_age_at_start_ms",
            "prepare_apply_age_at_terminal" => "completion_stage_prepare_apply_age_at_terminal_ms",
            "sync_globals" => "completion_stage_sync_globals_ms",
            "exact_wait_apply_age_at_start" => "completion_stage_exact_wait_apply_age_at_start_ms",
            "exact_wait_apply_age_at_terminal" => {
                "completion_stage_exact_wait_apply_age_at_terminal_ms"
            }
            "query_bundle" => "completion_stage_query_bundle_ms",
            "query_bundle_owner_hint" => "completion_stage_query_bundle_owner_hint_ms",
            "query_bundle_owner_hint_extract" => {
                "completion_stage_query_bundle_owner_hint_extract_ms"
            }
            "query_bundle_owner_hint_offset" => {
                "completion_stage_query_bundle_owner_hint_offset_ms"
            }
            "query_bundle_owner_hint_flow_lookup" => {
                "completion_stage_query_bundle_owner_hint_flow_lookup_ms"
            }
            "query_bundle_owner_hint_type_lookup_direct" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_direct_ms"
            }
            "query_bundle_owner_hint_type_lookup_fallback" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_fallback_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_wait" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_unattributed" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_unattributed_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_type_index" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_type_index_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_other" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_other_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_cancellation" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_cancellation_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_to_first_will_execute_type_index" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_to_first_will_execute_type_index_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_idle_before_first_will_execute_type_index" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_idle_before_first_will_execute_type_index_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_apply_age_at_query_start" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_age_at_query_start_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_apply_to_first_will_execute_type_index" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_first_will_execute_type_index_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_fetch_apply_to_fetch_end" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_fetch_end_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_query_total" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_total_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_query_inputs" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_inputs_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_query_parse_result_query" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_parse_result_query_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_query_build" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_build_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_parse_result" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_parse_result_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_build_total" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_total_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_build_seed_context" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_seed_context_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_build_local_function_summaries" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_local_function_summaries_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_build_visit_statements" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_visit_statements_ms"
            }
            "query_bundle_owner_hint_type_lookup_index_scan" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_index_scan_ms"
            }
            "query_bundle_owner_hint_type_lookup" => {
                "completion_stage_query_bundle_owner_hint_type_lookup_ms"
            }
            "query_bundle_deps_and_file_snapshot" => {
                "completion_stage_query_bundle_deps_and_file_snapshot_ms"
            },
            "response_build" => "completion_stage_response_build_ms",
            "cache_store" => "completion_stage_cache_store_ms",
            _ => "completion_stage_other_ms",
        };

        self.metrics
            .observe_histogram(metric, duration.as_millis() as f64);
    }

    pub fn record_completion_error(&self) {
        self.metrics.increment("completion_error_total");
    }

    pub fn record_completion_resolve_latency(&self, duration: Duration) {
        self.metrics.increment("completion_resolve_total");
        self.metrics.observe_histogram(
            "completion_resolve_duration_ms",
            duration.as_millis() as f64,
        );
    }

    pub fn record_signature_help_latency(&self, duration: Duration) {
        self.metrics.increment("signature_help_total");
        self.metrics
            .observe_histogram("signature_help_duration_ms", duration.as_millis() as f64);
    }

    pub fn record_signature_help_empty(&self) {
        self.metrics.increment("signature_help_empty_total");
    }

    pub fn record_completion_incomplete(&self) {
        self.metrics.increment("completion_incomplete_total");
    }

    pub fn record_intellisense_v2_completion_outcome(&self, outcome: &str) {
        let metric = match normalize_public_completion_outcome_label(outcome) {
            "ok_non_empty" => "intellisense_v2_completion_result_total_ok_non_empty",
            "ok_empty" => "intellisense_v2_completion_result_total_ok_empty",
            "fail_closed" => "intellisense_v2_completion_result_total_fail_closed",
            "cancelled" => "intellisense_v2_completion_result_total_cancelled",
            "handler_error" => "intellisense_v2_completion_result_total_handler_error",
            _ => "intellisense_v2_completion_result_total_other",
        };
        self.metrics.increment(metric);
    }

    pub fn record_intellisense_v2_completion_items_count(&self, items_count: usize) {
        self.metrics
            .observe_histogram("intellisense_v2_completion_items_count", items_count as f64);
    }

    pub fn record_intellisense_v2_completion_temperature(&self, state: &str) {
        let metric = match state {
            "first" => "intellisense_v2_completion_first_for_file_total",
            _ => "intellisense_v2_completion_warm_for_file_total",
        };
        self.metrics.increment(metric);
    }

    pub fn record_intellisense_v2_completion_trigger_mode(&self, mode: &str) {
        let mode = normalize_completion_trigger_mode_label(mode);
        let key = format!("intellisense_v2_completion_trigger_mode_total_mode_{mode}");
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_completion_parity_drift(&self, mode: &str) {
        let mode = normalize_completion_trigger_mode_label(mode);
        let key = format!("intellisense_v2_completion_parity_drift_total_mode_{mode}");
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_completion_parity_overlap_bucket(
        &self,
        mode: &str,
        bucket: &str,
    ) {
        let mode = normalize_completion_trigger_mode_label(mode);
        let bucket = normalize_completion_parity_overlap_bucket_label(bucket);
        let key =
            format!("intellisense_v2_completion_parity_overlap_total_mode_{mode}_bucket_{bucket}");
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_completion_member_access_terminal_empty(
        &self,
        mode: &str,
        reason: &str,
    ) {
        let mode = normalize_completion_trigger_mode_label(mode);
        let reason = normalize_completion_terminal_reason_label(reason);
        let key = format!(
            "intellisense_v2_completion_member_access_terminal_empty_total_mode_{mode}_reason_{reason}"
        );
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_completion_owner_hint_result(&self, reason: &str) {
        let reason = normalize_completion_owner_hint_reason_label(reason);
        let key = format!("intellisense_v2_completion_owner_hint_result_total_reason_{reason}");
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_interactive_fail_closed_reason(
        &self,
        origin: &str,
        operation: &str,
        reason: &str,
    ) {
        let origin = normalize_observability_origin_label(origin);
        let operation = normalize_operation_label(operation);
        let is_known_reason = SHARED_FAIL_CLOSED_REASON_REGISTRY
            .iter()
            .any(|(raw, _normalized)| *raw == reason);
        let reason = normalize_shared_fail_closed_reason_label(reason);
        let key = format!(
            "intellisense_v2_fail_closed_reason_total_origin_{origin}_operation_{operation}_reason_{reason}"
        );
        self.metrics.increment(&key);
        if !is_known_reason {
            self.record_observability_contract_violation("unknown_fail_closed_reason");
        }
    }

    pub fn record_intellisense_v2_type_index_reason(&self, reason: &str) {
        let is_known_reason = TYPE_INDEX_REASON_REGISTRY
            .iter()
            .any(|(raw, _normalized)| *raw == reason);
        let reason = normalize_type_index_reason_label(reason);
        let key = format!("intellisense_v2_type_index_reason_total_reason_{reason}");
        self.metrics.increment(&key);
        if !is_known_reason {
            self.record_observability_contract_violation("unknown_type_index_reason");
        }
    }

    pub fn record_completion_resource_pressure(&self, reason: &str, duration: Duration) {
        let reason = normalize_completion_resource_reason_label(reason);
        let counter_key =
            format!("intellisense_v2_completion_resource_pressure_total_reason_{reason}");
        let histogram_key =
            format!("intellisense_v2_completion_resource_pressure_ms_reason_{reason}");
        self.metrics.increment(&counter_key);
        self.metrics
            .observe_histogram(&histogram_key, duration.as_millis() as f64);
    }

    pub fn record_intellisense_v2_completion_owner_hint_lookup_path(&self, path: &str) {
        let metric = match path {
            "direct" => "intellisense_v2_completion_owner_hint_lookup_path_total_direct",
            "flow_only" => "intellisense_v2_completion_owner_hint_lookup_path_total_flow_only",
            "flow_plus_fallback" => {
                "intellisense_v2_completion_owner_hint_lookup_path_total_flow_plus_fallback"
            }
            _ => "intellisense_v2_completion_owner_hint_lookup_path_total_other",
        };
        self.metrics.increment(metric);
    }

    pub fn record_intellisense_v2_completion_owner_hint_lookup_result(&self, result: &str) {
        let metric = match result {
            "hit" => "intellisense_v2_completion_owner_hint_lookup_result_total_hit",
            "miss" => "intellisense_v2_completion_owner_hint_lookup_result_total_miss",
            "cancelled" => "intellisense_v2_completion_owner_hint_lookup_result_total_cancelled",
            "error" => "intellisense_v2_completion_owner_hint_lookup_result_total_error",
            _ => "intellisense_v2_completion_owner_hint_lookup_result_total_other",
        };
        self.metrics.increment(metric);
    }

    pub fn record_intellisense_v2_completion_owner_hint_context(
        &self,
        line_len_chars: usize,
        receiver_len_chars: usize,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_line_len_chars",
            line_len_chars as f64,
        );
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_receiver_len_chars",
            receiver_len_chars as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_block_on(
        &self,
        total: u64,
        type_index: u64,
        parse_result: u64,
        other: u64,
    ) {
        self.record_intellisense_v2_completion_owner_hint_index_fetch_salsa_counters(
            CompletionOwnerHintIndexFetchSalsaCounters {
                block_on_total: total,
                block_on_type_index_total: type_index,
                block_on_parse_result_total: parse_result,
                block_on_other_total: other,
                ..CompletionOwnerHintIndexFetchSalsaCounters::default()
            },
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_salsa_counters(
        &self,
        counters: CompletionOwnerHintIndexFetchSalsaCounters,
    ) {
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_total",
            counters.block_on_total,
        );
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_type_index_total",
            counters.block_on_type_index_total,
        );
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_parse_result_total",
            counters.block_on_parse_result_total,
        );
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_other_total",
            counters.block_on_other_total,
        );
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_total",
            counters.will_execute_total,
        );
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_type_index_total",
            counters.will_execute_type_index_total,
        );
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_parse_result_total",
            counters.will_execute_parse_result_total,
        );
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_other_total",
            counters.will_execute_other_total,
        );
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_total",
            counters.did_validate_memoized_total,
        );
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_type_index_total",
            counters.did_validate_memoized_type_index_total,
        );
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_parse_result_total",
            counters.did_validate_memoized_parse_result_total,
        );
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_other_total",
            counters.did_validate_memoized_other_total,
        );
        self.metrics.add_counter(
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_check_cancellation_total",
            counters.will_check_cancellation_total,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_active_gauge(
        &self,
        active: u64,
    ) {
        self.metrics.observe(
            "intellisense_v2_completion_owner_hint_index_fetch_active",
            active as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch(
        &self,
        count: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch",
            count as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch(
        &self,
        count: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch",
            count as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch(
        &self,
        count: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch",
            count as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch(
        &self,
        count: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch",
            count as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch(
        &self,
        count: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch",
            count as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch(
        &self,
        count: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch",
            count as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch(
        &self,
        count: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch",
            count as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch(
        &self,
        count: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch",
            count as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch(
        &self,
        count: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch",
            count as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch(
        &self,
        count: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch",
            count as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch(
        &self,
        count: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch",
            count as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_revision_start(
        &self,
        revision: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_revision_start",
            revision as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_revision_end(
        &self,
        revision: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_revision_end",
            revision as f64,
        );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_revision_delta(
        &self,
        delta: u64,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_completion_owner_hint_index_fetch_revision_delta",
            delta as f64,
        );
    }
}
