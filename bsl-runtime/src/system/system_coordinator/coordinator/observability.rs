use super::*;

impl SystemCoordinator {
    pub fn record_completion_latency(&self, duration: std::time::Duration) {
        self.observability.record_completion_latency(duration);
    }

    pub fn record_completion_stage_latency(&self, stage: &str, duration: std::time::Duration) {
        self.observability
            .record_completion_stage_latency(stage, duration);
    }

    pub fn record_completion_resource_pressure(&self, reason: &str, duration: std::time::Duration) {
        self.observability
            .record_completion_resource_pressure(reason, duration);
    }

    pub fn record_completion_error(&self) {
        self.observability.record_completion_error();
    }

    pub fn record_completion_resolve_latency(&self, duration: std::time::Duration) {
        self.observability
            .record_completion_resolve_latency(duration);
    }

    pub fn record_completion_incomplete(&self) {
        self.observability.record_completion_incomplete();
    }

    pub fn record_intellisense_v2_completion_outcome(&self, outcome: &str) {
        self.observability
            .record_intellisense_v2_completion_outcome(outcome);
    }

    pub fn record_intellisense_v2_completion_items_count(&self, items_count: usize) {
        self.observability
            .record_intellisense_v2_completion_items_count(items_count);
    }

    pub fn record_intellisense_v2_completion_temperature(&self, state: &str) {
        self.observability
            .record_intellisense_v2_completion_temperature(state);
    }

    pub fn record_intellisense_v2_completion_trigger_mode(&self, mode: &str) {
        self.observability
            .record_intellisense_v2_completion_trigger_mode(mode);
    }

    pub fn record_intellisense_v2_completion_parity_drift(&self, mode: &str) {
        self.observability
            .record_intellisense_v2_completion_parity_drift(mode);
    }

    pub fn record_intellisense_v2_completion_parity_overlap_bucket(
        &self,
        mode: &str,
        bucket: &str,
    ) {
        self.observability
            .record_intellisense_v2_completion_parity_overlap_bucket(mode, bucket);
    }

    pub fn record_intellisense_v2_completion_member_access_terminal_empty(
        &self,
        mode: &str,
        reason: &str,
    ) {
        self.observability
            .record_intellisense_v2_completion_member_access_terminal_empty(mode, reason);
    }

    pub fn record_intellisense_v2_completion_owner_hint_result(&self, reason: &str) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_result(reason);
    }

    pub fn record_intellisense_v2_type_index_reason(&self, reason: &str) {
        self.observability
            .record_intellisense_v2_type_index_reason(reason);
    }

    pub fn record_intellisense_v2_completion_owner_hint_lookup_path(&self, path: &str) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_lookup_path(path);
    }

    pub fn record_intellisense_v2_completion_owner_hint_lookup_result(&self, result: &str) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_lookup_result(result);
    }

    pub fn record_intellisense_v2_completion_owner_hint_context(
        &self,
        line_len_chars: usize,
        receiver_len_chars: usize,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_context(
                line_len_chars,
                receiver_len_chars,
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
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_salsa_counters(counters);
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_active_gauge(
        &self,
        active: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_active_gauge(active);
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch(
        &self,
        count: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch(
                count,
            );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch(
        &self,
        count: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch(
                count,
            );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch(
        &self,
        count: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch(
                count,
            );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch(
        &self,
        count: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch(
                count,
            );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch(
        &self,
        count: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch(
                count,
            );
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch(
        &self,
        count: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch(count);
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch(
        &self,
        count: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch(count);
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch(
        &self,
        count: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch(count);
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch(
        &self,
        count: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch(count);
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch(
        &self,
        count: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch(count);
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch(
        &self,
        count: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch(count);
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_revision_start(
        &self,
        revision: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_revision_start(revision);
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_revision_end(
        &self,
        revision: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_revision_end(revision);
    }

    pub fn record_intellisense_v2_completion_owner_hint_index_fetch_revision_delta(
        &self,
        delta: u64,
    ) {
        self.observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_revision_delta(delta);
    }

    pub fn observability_metrics(&self) -> Value {
        self.observability.get_metrics().export_metrics()
    }

    pub fn record_signature_help_latency(&self, duration: std::time::Duration) {
        self.observability.record_signature_help_latency(duration);
    }

    pub fn record_signature_help_empty(&self) {
        self.observability.record_signature_help_empty();
    }

    pub fn record_intellisense_v2_wait_for_file_version(
        &self,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.record_intellisense_v2_wait_for_file_version_with_origin("runtime", kind, duration);
    }

    pub fn record_intellisense_v2_wait_for_file_version_with_origin(
        &self,
        origin: &str,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_wait_for_file_version_with_origin(origin, kind, duration);
    }

    pub fn record_intellisense_v2_wait_for_file_version_with_origin_and_mode(
        &self,
        origin: &str,
        kind: &str,
        completion_mode: Option<&str>,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_wait_for_file_version_with_origin_and_mode(
                origin,
                kind,
                completion_mode,
                duration,
            );
    }

    pub fn record_intellisense_v2_snapshot_latency(
        &self,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.record_intellisense_v2_snapshot_latency_with_origin("runtime", kind, duration);
    }

    pub fn record_intellisense_v2_snapshot_latency_with_origin(
        &self,
        origin: &str,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_snapshot_latency_with_origin(origin, kind, duration);
    }

    pub fn record_intellisense_v2_snapshot_latency_with_origin_and_mode(
        &self,
        origin: &str,
        kind: &str,
        completion_mode: Option<&str>,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_snapshot_latency_with_origin_and_mode(
                origin,
                kind,
                completion_mode,
                duration,
            );
    }

    pub fn record_intellisense_v2_ir_query_latency(
        &self,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.record_intellisense_v2_ir_query_latency_with_origin("runtime", kind, duration);
    }

    pub fn record_intellisense_v2_ir_query_latency_with_origin(
        &self,
        origin: &str,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_ir_query_latency_with_origin(origin, kind, duration);
    }

    pub fn record_intellisense_v2_ir_query_latency_with_origin_and_mode(
        &self,
        origin: &str,
        kind: &str,
        completion_mode: Option<&str>,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_ir_query_latency_with_origin_and_mode(
                origin,
                kind,
                completion_mode,
                duration,
            );
    }

    pub fn record_intellisense_v2_ir_query_cancelled(&self, kind: &str) {
        self.record_intellisense_v2_ir_query_cancelled_with_origin("runtime", kind);
    }

    pub fn record_intellisense_v2_ir_query_cancelled_with_origin(&self, origin: &str, kind: &str) {
        self.observability
            .record_intellisense_v2_ir_query_cancelled_with_origin(origin, kind);
    }

    pub fn record_intellisense_v2_ir_query_cancelled_with_origin_and_mode(
        &self,
        origin: &str,
        kind: &str,
        completion_mode: Option<&str>,
    ) {
        self.observability
            .record_intellisense_v2_ir_query_cancelled_with_origin_and_mode(
                origin,
                kind,
                completion_mode,
            );
    }

    pub fn record_intellisense_v2_syntax_diagnostics_query_latency(
        &self,
        duration: std::time::Duration,
    ) {
        self.record_intellisense_v2_syntax_diagnostics_query_latency_with_origin(
            "runtime", duration,
        );
    }

    pub fn record_intellisense_v2_syntax_diagnostics_query_latency_with_origin(
        &self,
        origin: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_syntax_diagnostics_query_latency_with_origin(origin, duration);
    }

    pub fn record_intellisense_v2_syntax_diagnostics_query_latency_with_origin_and_mode(
        &self,
        origin: &str,
        parse_mode: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_syntax_diagnostics_query_latency_with_origin_and_mode(
                origin, parse_mode, duration,
            );
    }

    pub fn record_intellisense_v2_semantic_diagnostics_query_latency(
        &self,
        duration: std::time::Duration,
    ) {
        self.record_intellisense_v2_semantic_diagnostics_query_latency_with_origin(
            "runtime", duration,
        );
    }

    pub fn record_intellisense_v2_semantic_diagnostics_query_latency_with_origin(
        &self,
        origin: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_semantic_diagnostics_query_latency_with_origin(
                origin, duration,
            );
    }

    pub fn record_intellisense_v2_parse_result_query_latency(&self, duration: std::time::Duration) {
        self.record_intellisense_v2_parse_result_query_latency_with_origin_and_operation(
            "runtime", "other", duration,
        );
    }

    pub fn record_intellisense_v2_parse_result_query_latency_with_origin(
        &self,
        origin: &str,
        duration: std::time::Duration,
    ) {
        self.record_intellisense_v2_parse_result_query_latency_with_origin_and_operation(
            origin, "other", duration,
        );
    }

    pub fn record_intellisense_v2_parse_result_query_latency_with_origin_and_operation(
        &self,
        origin: &str,
        operation: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_parse_result_query_latency_with_origin_and_operation(
                origin, operation, duration,
            );
    }

    pub fn record_intellisense_v2_parse_result_query_latency_with_origin_operation_and_mode(
        &self,
        origin: &str,
        operation: &str,
        completion_mode: Option<&str>,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_parse_result_query_latency_with_origin_operation_and_mode(
                origin,
                operation,
                completion_mode,
                duration,
            );
    }

    pub fn record_intellisense_v2_payload_shape(
        &self,
        operation: &str,
        stage: &str,
        file_bytes: usize,
        line_count: usize,
    ) {
        self.record_intellisense_v2_payload_shape_with_origin(
            "runtime", operation, stage, file_bytes, line_count,
        );
    }

    pub fn record_intellisense_v2_payload_shape_with_origin(
        &self,
        origin: &str,
        operation: &str,
        stage: &str,
        file_bytes: usize,
        line_count: usize,
    ) {
        self.observability
            .record_intellisense_v2_payload_shape_with_origin(
                origin, operation, stage, file_bytes, line_count,
            );
    }

    pub fn record_intellisense_v2_interactive_wait_budget_exhausted(&self) {
        self.observability
            .record_intellisense_v2_interactive_wait_budget_exhausted();
    }

    pub fn record_intellisense_v2_interactive_stale_served(&self) {
        self.observability
            .record_intellisense_v2_interactive_stale_served();
    }

    pub fn record_intellisense_v2_interactive_knob_clamped(&self) {
        self.observability
            .record_intellisense_v2_interactive_knob_clamped();
    }

    pub fn record_intellisense_v2_completion_stale_fallback(&self) {
        self.observability
            .record_intellisense_v2_completion_stale_fallback();
    }

    pub fn record_intellisense_v2_completion_fallback_unavailable(&self) {
        self.observability
            .record_intellisense_v2_completion_fallback_unavailable();
    }

    pub fn record_intellisense_v2_revision_lag(&self, lag_versions: i32) {
        self.observability
            .record_intellisense_v2_revision_lag(lag_versions);
    }

    pub fn record_intellisense_v2_singleflight_leader(&self) {
        self.record_intellisense_v2_singleflight_leader_with_origin("runtime", "ir");
    }

    pub fn record_intellisense_v2_singleflight_leader_with_origin(
        &self,
        origin: &str,
        query_kind: &str,
    ) {
        self.observability
            .record_intellisense_v2_singleflight_leader_with_origin(origin, query_kind);
    }

    pub fn record_intellisense_v2_singleflight_shared(&self) {
        self.record_intellisense_v2_singleflight_shared_with_origin("runtime", "ir");
    }

    pub fn record_intellisense_v2_singleflight_shared_with_origin(
        &self,
        origin: &str,
        query_kind: &str,
    ) {
        self.observability
            .record_intellisense_v2_singleflight_shared_with_origin(origin, query_kind);
    }

    pub fn record_intellisense_v2_singleflight_key_unavailable_with_origin(
        &self,
        origin: &str,
        query_kind: &str,
    ) {
        self.observability
            .record_intellisense_v2_singleflight_key_unavailable_with_origin(origin, query_kind);
    }

    pub fn record_intellisense_v2_singleflight_wait_latency(&self, duration: std::time::Duration) {
        self.record_intellisense_v2_singleflight_wait_latency_with_origin(
            "runtime", "ir", duration,
        );
    }

    pub fn record_intellisense_v2_singleflight_wait_latency_with_origin(
        &self,
        origin: &str,
        query_kind: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_singleflight_wait_latency_with_origin(
                origin, query_kind, duration,
            );
    }

    pub fn record_intellisense_v2_runtime_queue_wait_class_latency(
        &self,
        class: &str,
        duration: std::time::Duration,
    ) {
        self.record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
            "runtime", class, duration,
        );
    }

    pub fn record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
        &self,
        origin: &str,
        class: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
                origin, class, duration,
            );
    }

    pub fn record_intellisense_v2_runtime_exec_class_latency(
        &self,
        class: &str,
        duration: std::time::Duration,
    ) {
        self.record_intellisense_v2_runtime_exec_class_latency_with_origin(
            "runtime", class, duration,
        );
    }

    pub fn record_intellisense_v2_runtime_exec_class_latency_with_origin(
        &self,
        origin: &str,
        class: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_runtime_exec_class_latency_with_origin(origin, class, duration);
    }

    pub fn record_intellisense_v2_query_cancelled(&self, kind: &str) {
        self.record_intellisense_v2_query_cancelled_with_origin("runtime", kind);
    }

    pub fn record_intellisense_v2_query_cancelled_with_origin(&self, origin: &str, kind: &str) {
        self.observability
            .record_intellisense_v2_query_cancelled_with_origin(origin, kind);
    }

    pub fn record_intellisense_v2_query_cancelled_with_origin_and_mode(
        &self,
        origin: &str,
        kind: &str,
        completion_mode: Option<&str>,
    ) {
        self.observability
            .record_intellisense_v2_query_cancelled_with_origin_and_mode(
                origin,
                kind,
                completion_mode,
            );
    }

    pub fn record_intellisense_v2_runtime_queue_wait_latency(
        &self,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.record_intellisense_v2_runtime_queue_wait_latency_with_origin(
            "runtime", kind, duration,
        );
    }

    pub fn record_intellisense_v2_runtime_queue_wait_latency_with_origin(
        &self,
        origin: &str,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_runtime_queue_wait_latency_with_origin(origin, kind, duration);
    }

    pub fn record_intellisense_v2_runtime_exec_latency(
        &self,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.record_intellisense_v2_runtime_exec_latency_with_origin("runtime", kind, duration);
    }

    pub fn record_intellisense_v2_runtime_exec_latency_with_origin(
        &self,
        origin: &str,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_runtime_exec_latency_with_origin(origin, kind, duration);
    }

    pub fn record_intellisense_v2_runtime_apply_changes_batch_size(&self, batch_size: usize) {
        self.observability
            .record_intellisense_v2_runtime_apply_changes_batch_size(batch_size);
    }

    pub fn record_intellisense_v2_runtime_apply_changes_changed_files_count(
        &self,
        changed_files_count: usize,
    ) {
        self.observability
            .record_intellisense_v2_runtime_apply_changes_changed_files_count(changed_files_count);
    }

    pub fn record_intellisense_v2_runtime_saturation_gauge_with_origin(
        &self,
        origin: &str,
        saturation_metric: &str,
        value: f64,
        legacy_key: &str,
    ) {
        self.observability
            .record_intellisense_v2_runtime_saturation_gauge_with_origin(
                origin,
                saturation_metric,
                value,
                legacy_key,
            );
    }

    pub fn record_intellisense_v2_diagnostics_pipeline_event(
        &self,
        origin: &str,
        trigger: &str,
        profile: &str,
        reason: &str,
    ) {
        self.observability
            .record_intellisense_v2_diagnostics_pipeline_event(origin, trigger, profile, reason);
    }

    pub fn record_intellisense_v2_large_churn_transition(&self, origin: &str, state: &str) {
        self.observability
            .record_intellisense_v2_large_churn_transition(origin, state);
    }

    pub fn record_intellisense_v2_heavy_diagnostics_deferred(
        &self,
        origin: &str,
        profile: &str,
        reason: &str,
    ) {
        self.observability
            .record_intellisense_v2_heavy_diagnostics_deferred(origin, profile, reason);
    }

    pub fn record_intellisense_v2_parse_snapshot(
        &self,
        origin: &str,
        mode: &str,
        changed_ranges_count: usize,
        changed_ranges_bytes: usize,
        fallback_reason: Option<&str>,
        build_duration: std::time::Duration,
    ) {
        self.observability.record_intellisense_v2_parse_snapshot(
            origin,
            mode,
            changed_ranges_count,
            changed_ranges_bytes,
            fallback_reason,
            build_duration,
        );
    }

    pub fn record_intellisense_v2_deps_update_build_latency(&self, duration: std::time::Duration) {
        self.observability
            .record_intellisense_v2_deps_update_build_latency(duration);
    }

    pub fn record_intellisense_v2_deps_update_apply_latency(&self, duration: std::time::Duration) {
        self.observability
            .record_intellisense_v2_deps_update_apply_latency(duration);
    }

    pub fn record_intellisense_v2_deps_update_success(&self) {
        self.observability
            .record_intellisense_v2_deps_update_success();
    }

    pub fn record_intellisense_v2_deps_update_error(&self) {
        self.observability
            .record_intellisense_v2_deps_update_error();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_completion_quality(
        &self,
        total_candidates: usize,
        dedup_removed: usize,
        score_samples: &[f32],
        prefix_exact: usize,
        prefix_starts: usize,
        prefix_contains: usize,
        prefix_none: usize,
        member_access: usize,
        has_owner: usize,
    ) {
        self.observability.record_completion_quality(
            total_candidates,
            dedup_removed,
            score_samples,
            prefix_exact,
            prefix_starts,
            prefix_contains,
            prefix_none,
            member_access,
            has_owner,
        );
    }
}
