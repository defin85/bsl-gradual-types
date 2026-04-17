use super::*;

fn normalize_ready_parse_snapshot_source_label(source: &str) -> &'static str {
    match source {
        "did_open" => "did_open",
        "did_change" => "did_change",
        "did_save" => "did_save",
        _ => "other",
    }
}

fn normalize_ready_parse_snapshot_worker_termination_reason_label(reason: &str) -> &'static str {
    match reason {
        "aborted" => "aborted",
        "superseded" => "superseded",
        "retargeted_before_parse" => "retargeted_before_parse",
        "retargeted_during_parse" => "retargeted_during_parse",
        "retargeted_before_materialization" => "retargeted_before_materialization",
        "latest_version_mismatch" => "latest_version_mismatch",
        "build_snapshot_aborted" => "build_snapshot_aborted",
        _ => "other",
    }
}

fn normalize_ready_parse_snapshot_phase_label(phase: &str) -> &'static str {
    match phase {
        "parse_exec" => "parse_exec",
        "post_parse_pre_materialization" => "post_parse_pre_materialization",
        "ready_install" => "ready_install",
        "document_symbol_side_work" => "document_symbol_side_work",
        _ => "other",
    }
}

fn normalize_diagnostics_save_followup_probe_slot_label(slot: &str) -> &'static str {
    match slot {
        "zero_budget" => "zero_budget",
        "bounded_wait" => "bounded_wait",
        "relief_valve" => "relief_valve",
        _ => "other",
    }
}

fn normalize_ready_parse_snapshot_probe_outcome_label(outcome: &str) -> &'static str {
    match outcome {
        "ready" => "ready",
        "not_ready" => "not_ready",
        "generation_mismatch" => "generation_mismatch",
        "version_mismatch" => "version_mismatch",
        "timeout" => "timeout",
        "cancelled" => "cancelled",
        "superseded" => "superseded",
        _ => "other",
    }
}

fn normalize_diagnostics_save_followup_semantic_path_label(path: &str) -> &'static str {
    match path {
        "ready_artifacts" => "ready_artifacts",
        "shadow_state" => "shadow_state",
        "generic_pipeline" => "generic_pipeline",
        _ => "other",
    }
}

fn normalize_diagnostics_save_followup_wait_reason_label(reason: &str) -> &'static str {
    match reason {
        "pending_publish" => "pending_publish",
        "runtime_queue_wait" => "runtime_queue_wait",
        "semantic_work" => "semantic_work",
        "apply_lag" => "apply_lag",
        _ => "other",
    }
}

fn normalize_diagnostics_save_followup_relief_valve_outcome_label(outcome: &str) -> &'static str {
    match outcome {
        "engaged_helped" => "engaged_helped",
        "engaged_timed_out" => "engaged_timed_out",
        "engaged_version_mismatch" => "engaged_version_mismatch",
        "engaged_generation_mismatch" => "engaged_generation_mismatch",
        "engaged_cancelled" => "engaged_cancelled",
        "engaged_superseded" => "engaged_superseded",
        "skipped_not_exact_still_current" => "skipped_not_exact_still_current",
        "skipped_runtime_queue_wait" => "skipped_runtime_queue_wait",
        "skipped_apply_lag" => "skipped_apply_lag",
        "skipped_timeout_phase_unavailable" => "skipped_timeout_phase_unavailable",
        "skipped_timeout_phase_waiting" => "skipped_timeout_phase_waiting",
        _ => "other",
    }
}

fn normalize_diagnostics_save_followup_continuation_reason_label(reason: &str) -> &'static str {
    match reason {
        "continued_still_current" => "continued_still_current",
        "exhausted_continuation_proof" => "exhausted_continuation_proof",
        "superseded" => "superseded",
        "cancelled" => "cancelled",
        "other_terminal" => "other_terminal",
        _ => "other",
    }
}

impl BasicObservability {
    pub fn record_intellisense_v2_document_symbol_outcome(&self, outcome: &str) {
        let outcome = match outcome {
            "current_ready" => "current_ready",
            "latest_ready" => "latest_ready",
            "unavailable" => "unavailable",
            "superseded" => "superseded",
            _ => "other",
        };
        let key = format!("intellisense_v2_document_symbol_outcome_total_outcome_{outcome}");
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_interactive_wait_budget_exhausted(&self) {
        self.metrics
            .increment("intellisense_v2_interactive_wait_budget_exhausted_total");
    }

    pub fn record_intellisense_v2_interactive_stale_served(&self) {
        self.metrics
            .increment("intellisense_v2_interactive_stale_served_total");
    }

    pub fn record_intellisense_v2_interactive_knob_clamped(&self) {
        self.metrics
            .increment("intellisense_v2_interactive_knob_clamped_total");
    }

    pub fn record_intellisense_v2_completion_stale_fallback(&self) {
        self.metrics
            .increment("intellisense_v2_completion_stale_fallback_total");
    }

    pub fn record_intellisense_v2_completion_fallback_unavailable(&self) {
        self.metrics
            .increment("intellisense_v2_completion_fallback_unavailable_total");
    }

    pub fn record_intellisense_v2_revision_lag(&self, lag_versions: i32) {
        let lag_versions = lag_versions.max(0) as f64;
        self.metrics
            .increment("intellisense_v2_revision_lag_sample_total");
        self.metrics
            .observe_histogram("intellisense_v2_revision_lag_versions", lag_versions);
    }

    pub fn record_intellisense_v2_ready_parse_snapshot_materialization(
        &self,
        origin: &str,
        source: &str,
        duration: Duration,
    ) {
        let origin = normalize_observability_origin_label(origin);
        let source = normalize_ready_parse_snapshot_source_label(source);
        let counter_key = format!(
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_{origin}_source_{source}"
        );
        let histogram_key = format!(
            "intellisense_v2_ready_parse_snapshot_materialization_ms_origin_{origin}_source_{source}"
        );
        self.metrics.increment(&counter_key);
        self.metrics
            .observe_histogram(&histogram_key, duration.as_millis() as f64);
    }

    pub fn record_intellisense_v2_ready_parse_snapshot_phase_latency(
        &self,
        origin: &str,
        source: &str,
        phase: &str,
        duration: Duration,
    ) {
        let origin = normalize_observability_origin_label(origin);
        let source = normalize_ready_parse_snapshot_source_label(source);
        let phase = normalize_ready_parse_snapshot_phase_label(phase);
        let counter_key = format!(
            "intellisense_v2_ready_parse_snapshot_phase_total_origin_{origin}_source_{source}_phase_{phase}"
        );
        let histogram_key = format!(
            "intellisense_v2_ready_parse_snapshot_phase_ms_origin_{origin}_source_{source}_phase_{phase}"
        );
        self.metrics.increment(&counter_key);
        self.metrics
            .observe_histogram(&histogram_key, duration.as_millis() as f64);
    }

    pub fn record_intellisense_v2_ready_parse_snapshot_worker_started(
        &self,
        origin: &str,
        source: &str,
    ) {
        let origin = normalize_observability_origin_label(origin);
        let source = normalize_ready_parse_snapshot_source_label(source);
        let key =
            format!("intellisense_v2_ready_parse_snapshot_worker_started_total_origin_{origin}_source_{source}");
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization(
        &self,
        origin: &str,
        source: &str,
        reason: &str,
        duration: Duration,
    ) {
        let origin = normalize_observability_origin_label(origin);
        let source = normalize_ready_parse_snapshot_source_label(source);
        let reason = normalize_ready_parse_snapshot_worker_termination_reason_label(reason);
        let counter_key = format!(
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_{origin}_source_{source}_reason_{reason}"
        );
        let histogram_key = format!(
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_{origin}_source_{source}_reason_{reason}"
        );
        self.metrics.increment(&counter_key);
        self.metrics
            .observe_histogram(&histogram_key, duration.as_millis() as f64);
    }

    pub fn record_intellisense_v2_diagnostics_save_followup_ready_snapshot_probe(
        &self,
        slot: &str,
        outcome: &str,
        duration: Duration,
    ) {
        let slot = normalize_diagnostics_save_followup_probe_slot_label(slot);
        let outcome = normalize_ready_parse_snapshot_probe_outcome_label(outcome);
        let counter_key = format!(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_{slot}_outcome_{outcome}"
        );
        let histogram_key = format!(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_{slot}_outcome_{outcome}"
        );
        self.metrics.increment(&counter_key);
        self.metrics
            .observe_histogram(&histogram_key, duration.as_millis() as f64);
    }

    pub fn record_intellisense_v2_diagnostics_save_followup_semantic_path(&self, path: &str) {
        let path = normalize_diagnostics_save_followup_semantic_path_label(path);
        let key =
            format!("intellisense_v2_diagnostics_save_followup_semantic_path_total_path_{path}");
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_diagnostics_save_followup_wait_state(&self, reason: &str) {
        let reason = normalize_diagnostics_save_followup_wait_reason_label(reason);
        let key =
            format!("intellisense_v2_diagnostics_save_followup_wait_state_total_reason_{reason}");
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve(
        &self,
        outcome: &str,
        duration: Duration,
    ) {
        let outcome = normalize_diagnostics_save_followup_relief_valve_outcome_label(outcome);
        let counter_key = format!(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_{outcome}"
        );
        let histogram_key = format!(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_ms_outcome_{outcome}"
        );
        self.metrics.increment(&counter_key);
        self.metrics
            .observe_histogram(&histogram_key, duration.as_millis() as f64);
    }

    pub fn record_intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation(
        &self,
        reason: &str,
    ) {
        let reason = normalize_diagnostics_save_followup_continuation_reason_label(reason);
        let key = format!(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation_total_reason_{reason}"
        );
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_completion_exact_type_index_wait_outcome(&self, reason: &str) {
        let reason = normalize_completion_exact_type_index_wait_reason_label(reason);
        let key = format!(
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_{reason}"
        );
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_completion_exact_type_index_wait_promotion(&self) {
        self.metrics
            .increment("intellisense_v2_completion_exact_type_index_wait_promotion_total");
    }

    pub fn record_intellisense_v2_completion_exact_type_index_wait_join(&self) {
        self.metrics
            .increment("intellisense_v2_completion_exact_type_index_wait_join_total");
    }

    pub fn record_intellisense_v2_completion_exact_type_index_wait_ready_after_wait(&self) {
        self.metrics
            .increment("intellisense_v2_completion_exact_type_index_wait_ready_after_wait_total");
    }

    pub fn record_intellisense_v2_singleflight_leader(&self) {
        self.record_intellisense_v2_singleflight_leader_with_origin("runtime", "ir");
    }

    pub fn record_intellisense_v2_singleflight_leader_with_origin(
        &self,
        origin: &str,
        query_kind: &str,
    ) {
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::SingleflightEffectivenessTotal,
                origin,
                mode: None,
                operation: None,
                stage: None,
                outcome: Some("leader"),
                reason: None,
                query_kind: Some(normalize_query_kind_label(query_kind)),
                work_class: None,
                saturation_metric: None,
                value_kind: CanonicalValueKind::Counter,
                value: 1.0,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: "intellisense_v2_singleflight_leader_total",
                kind: LegacyMetricKind::Counter,
            }),
        );
    }

    pub fn record_intellisense_v2_singleflight_shared(&self) {
        self.record_intellisense_v2_singleflight_shared_with_origin("runtime", "ir");
    }

    pub fn record_intellisense_v2_singleflight_shared_with_origin(
        &self,
        origin: &str,
        query_kind: &str,
    ) {
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::SingleflightEffectivenessTotal,
                origin,
                mode: None,
                operation: None,
                stage: None,
                outcome: Some("shared"),
                reason: None,
                query_kind: Some(normalize_query_kind_label(query_kind)),
                work_class: None,
                saturation_metric: None,
                value_kind: CanonicalValueKind::Counter,
                value: 1.0,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: "intellisense_v2_singleflight_shared_total",
                kind: LegacyMetricKind::Counter,
            }),
        );
    }

    pub fn record_intellisense_v2_singleflight_key_unavailable_with_origin(
        &self,
        origin: &str,
        query_kind: &str,
    ) {
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::SingleflightEffectivenessTotal,
                origin,
                mode: None,
                operation: None,
                stage: None,
                outcome: Some("key_unavailable"),
                reason: None,
                query_kind: Some(normalize_query_kind_label(query_kind)),
                work_class: None,
                saturation_metric: None,
                value_kind: CanonicalValueKind::Counter,
                value: 1.0,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: "intellisense_v2_singleflight_key_unavailable_total",
                kind: LegacyMetricKind::Counter,
            }),
        );
    }

    pub fn record_intellisense_v2_singleflight_key_unavailable(&self, query_kind: &str) {
        self.record_intellisense_v2_singleflight_key_unavailable_with_origin("runtime", query_kind);
    }

    pub fn record_intellisense_v2_singleflight_wait_latency(&self, duration: Duration) {
        self.record_intellisense_v2_singleflight_wait_latency_with_origin(
            "runtime", "ir", duration,
        );
    }

    pub fn record_intellisense_v2_singleflight_wait_latency_with_origin(
        &self,
        _origin: &str,
        _query_kind: &str,
        duration: Duration,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_singleflight_wait_ms",
            duration.as_millis() as f64,
        );
    }

    pub fn record_intellisense_v2_runtime_queue_wait_class_latency(
        &self,
        class: &str,
        duration: Duration,
    ) {
        self.record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
            "runtime", class, duration,
        );
    }

    pub fn record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
        &self,
        origin: &str,
        class: &str,
        duration: Duration,
    ) {
        let (total_metric, histogram_metric) = match class {
            "background" => (
                "intellisense_v2_runtime_queue_wait_background_total",
                "intellisense_v2_runtime_queue_wait_background_ms",
            ),
            _ => (
                "intellisense_v2_runtime_queue_wait_interactive_total",
                "intellisense_v2_runtime_queue_wait_interactive_ms",
            ),
        };
        let work_class = normalize_work_class_label(class);
        let elapsed_ms = duration.as_millis() as f64;
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::SaturationSampleTotal,
                origin,
                mode: None,
                operation: None,
                stage: None,
                outcome: None,
                reason: Some("queue_wait"),
                query_kind: None,
                work_class: Some(work_class),
                saturation_metric: None,
                value_kind: CanonicalValueKind::Counter,
                value: 1.0,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: total_metric,
                kind: LegacyMetricKind::Counter,
            }),
        );
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::SaturationSampleLatencyMs,
                origin,
                mode: None,
                operation: None,
                stage: None,
                outcome: None,
                reason: Some("queue_wait"),
                query_kind: None,
                work_class: Some(work_class),
                saturation_metric: None,
                value_kind: CanonicalValueKind::HistogramMs,
                value: elapsed_ms,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: histogram_metric,
                kind: LegacyMetricKind::HistogramMs,
            }),
        );
    }

    pub fn record_intellisense_v2_runtime_exec_class_latency(
        &self,
        class: &str,
        duration: Duration,
    ) {
        self.record_intellisense_v2_runtime_exec_class_latency_with_origin(
            "runtime", class, duration,
        );
    }

    pub fn record_intellisense_v2_runtime_exec_class_latency_with_origin(
        &self,
        origin: &str,
        class: &str,
        duration: Duration,
    ) {
        let (total_metric, histogram_metric) = match class {
            "background" => (
                "intellisense_v2_runtime_exec_background_total",
                "intellisense_v2_runtime_exec_background_ms",
            ),
            _ => (
                "intellisense_v2_runtime_exec_interactive_total",
                "intellisense_v2_runtime_exec_interactive_ms",
            ),
        };
        let work_class = normalize_work_class_label(class);
        let elapsed_ms = duration.as_millis() as f64;
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::SaturationSampleTotal,
                origin,
                mode: None,
                operation: None,
                stage: None,
                outcome: None,
                reason: Some("exec"),
                query_kind: None,
                work_class: Some(work_class),
                saturation_metric: None,
                value_kind: CanonicalValueKind::Counter,
                value: 1.0,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: total_metric,
                kind: LegacyMetricKind::Counter,
            }),
        );
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::SaturationSampleLatencyMs,
                origin,
                mode: None,
                operation: None,
                stage: None,
                outcome: None,
                reason: Some("exec"),
                query_kind: None,
                work_class: Some(work_class),
                saturation_metric: None,
                value_kind: CanonicalValueKind::HistogramMs,
                value: elapsed_ms,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: histogram_metric,
                kind: LegacyMetricKind::HistogramMs,
            }),
        );
    }

    pub fn record_intellisense_v2_runtime_lane_queue_wait_latency_with_origin(
        &self,
        origin: &str,
        lane: &str,
        duration: Duration,
    ) {
        let origin = normalize_observability_origin_label(origin);
        let lane = normalize_runtime_lane_label(lane);
        let counter_key =
            format!("intellisense_v2_runtime_lane_queue_wait_total_origin_{origin}_lane_{lane}");
        let histogram_key =
            format!("intellisense_v2_runtime_lane_queue_wait_ms_origin_{origin}_lane_{lane}");
        self.metrics.increment(&counter_key);
        self.metrics
            .observe_histogram(&histogram_key, duration.as_millis() as f64);
    }

    pub fn record_intellisense_v2_runtime_lane_exec_latency_with_origin(
        &self,
        origin: &str,
        lane: &str,
        duration: Duration,
    ) {
        let origin = normalize_observability_origin_label(origin);
        let lane = normalize_runtime_lane_label(lane);
        let counter_key =
            format!("intellisense_v2_runtime_lane_exec_total_origin_{origin}_lane_{lane}");
        let histogram_key =
            format!("intellisense_v2_runtime_lane_exec_ms_origin_{origin}_lane_{lane}");
        self.metrics.increment(&counter_key);
        self.metrics
            .observe_histogram(&histogram_key, duration.as_millis() as f64);
    }

    pub fn record_intellisense_v2_runtime_lane_saturation_gauge_with_origin(
        &self,
        origin: &str,
        lane: &str,
        saturation_metric: &str,
        value: f64,
    ) {
        let origin = normalize_observability_origin_label(origin);
        let lane = normalize_runtime_lane_label(lane);
        let saturation_metric = normalize_runtime_lane_saturation_metric_label(saturation_metric);
        let key = format!(
            "intellisense_v2_runtime_lane_saturation_gauge_origin_{origin}_lane_{lane}_metric_{saturation_metric}"
        );
        self.metrics.observe(&key, value);
    }

    pub fn record_intellisense_v2_query_cancelled(&self, kind: &str) {
        self.record_intellisense_v2_query_cancelled_with_origin("runtime", kind);
    }

    pub fn record_intellisense_v2_query_cancelled_with_origin(&self, origin: &str, kind: &str) {
        self.record_intellisense_v2_query_cancelled_with_origin_and_mode(origin, kind, None);
    }

    pub fn record_intellisense_v2_query_cancelled_with_origin_and_mode(
        &self,
        origin: &str,
        kind: &str,
        completion_mode: Option<&str>,
    ) {
        let completion_mode = completion_mode.map(normalize_completion_observability_mode_label);
        let (metric, stage) = match kind {
            "syntax" => (
                "intellisense_v2_query_cancelled_total_syntax",
                "syntax_diagnostics_query",
            ),
            "semantic" => (
                "intellisense_v2_query_cancelled_total_semantic",
                "semantic_diagnostics_query",
            ),
            _ => (
                "intellisense_v2_query_cancelled_total_other",
                "parse_result_query",
            ),
        };
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageReasonTotal,
                origin,
                mode: completion_mode,
                operation: Some("diagnostics"),
                stage: Some(stage),
                outcome: None,
                reason: Some(normalize_reason_label(kind)),
                query_kind: None,
                work_class: None,
                saturation_metric: None,
                value_kind: CanonicalValueKind::Counter,
                value: 1.0,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: metric,
                kind: LegacyMetricKind::Counter,
            }),
        );
    }

    pub fn record_intellisense_v2_runtime_queue_wait_latency(
        &self,
        kind: &str,
        duration: Duration,
    ) {
        self.record_intellisense_v2_runtime_queue_wait_latency_with_origin(
            "runtime", kind, duration,
        );
    }

    pub fn record_intellisense_v2_runtime_queue_wait_latency_with_origin(
        &self,
        origin: &str,
        kind: &str,
        duration: Duration,
    ) {
        let (total_metric, histogram_metric) = legacy_runtime_queue_wait_metrics(kind);
        let operation = normalize_runtime_stage_kind(kind);
        let stage = "runtime_queue_wait";
        let elapsed_ms = duration.as_millis() as f64;
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageTotal,
                origin,
                mode: None,
                operation: Some(operation),
                stage: Some(stage),
                outcome: None,
                reason: None,
                query_kind: None,
                work_class: None,
                saturation_metric: None,
                value_kind: CanonicalValueKind::Counter,
                value: 1.0,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: total_metric,
                kind: LegacyMetricKind::Counter,
            }),
        );
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageLatencyMs,
                origin,
                mode: None,
                operation: Some(operation),
                stage: Some(stage),
                outcome: None,
                reason: None,
                query_kind: None,
                work_class: None,
                saturation_metric: None,
                value_kind: CanonicalValueKind::HistogramMs,
                value: elapsed_ms,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: histogram_metric,
                kind: LegacyMetricKind::HistogramMs,
            }),
        );
    }

    pub fn record_intellisense_v2_runtime_exec_latency(&self, kind: &str, duration: Duration) {
        self.record_intellisense_v2_runtime_exec_latency_with_origin("runtime", kind, duration);
    }

    pub fn record_intellisense_v2_runtime_exec_latency_with_origin(
        &self,
        origin: &str,
        kind: &str,
        duration: Duration,
    ) {
        let (total_metric, histogram_metric) = legacy_runtime_exec_metrics(kind);
        let operation = normalize_runtime_stage_kind(kind);
        let stage = "runtime_exec";
        let elapsed_ms = duration.as_millis() as f64;
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageTotal,
                origin,
                mode: None,
                operation: Some(operation),
                stage: Some(stage),
                outcome: None,
                reason: None,
                query_kind: None,
                work_class: None,
                saturation_metric: None,
                value_kind: CanonicalValueKind::Counter,
                value: 1.0,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: total_metric,
                kind: LegacyMetricKind::Counter,
            }),
        );
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageLatencyMs,
                origin,
                mode: None,
                operation: Some(operation),
                stage: Some(stage),
                outcome: None,
                reason: None,
                query_kind: None,
                work_class: None,
                saturation_metric: None,
                value_kind: CanonicalValueKind::HistogramMs,
                value: elapsed_ms,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: histogram_metric,
                kind: LegacyMetricKind::HistogramMs,
            }),
        );
    }

    pub fn record_intellisense_v2_runtime_apply_changes_batch_size(&self, batch_size: usize) {
        self.metrics.observe_histogram(
            "intellisense_v2_runtime_apply_changes_batch_size",
            batch_size as f64,
        );
    }

    pub fn record_intellisense_v2_runtime_apply_changes_changed_files_count(
        &self,
        changed_files_count: usize,
    ) {
        self.metrics.observe_histogram(
            "intellisense_v2_runtime_apply_changes_changed_files_count",
            changed_files_count as f64,
        );
    }

    pub fn record_intellisense_v2_runtime_saturation_gauge_with_origin(
        &self,
        origin: &str,
        saturation_metric: &str,
        value: f64,
        legacy_key: &str,
    ) {
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::SaturationGauge,
                origin,
                mode: None,
                operation: None,
                stage: None,
                outcome: None,
                reason: None,
                query_kind: None,
                work_class: None,
                saturation_metric: Some(saturation_metric),
                value_kind: CanonicalValueKind::Gauge,
                value,
                requires_legacy_projection: true,
            },
            Some(LegacyMetricTarget {
                key: legacy_key,
                kind: LegacyMetricKind::Gauge,
            }),
        );
        self.metrics
            .increment("intellisense_v2_runtime_saturation_sample_total");
    }

    pub fn record_intellisense_v2_diagnostics_pipeline_event(
        &self,
        origin: &str,
        trigger: &str,
        profile: &str,
        reason: &str,
    ) {
        let origin = normalize_observability_origin_label(origin);
        let trigger = normalize_diagnostics_trigger_label(trigger);
        let profile = normalize_diagnostics_profile_label(profile);
        let reason = normalize_diagnostics_reason_label(reason);
        let key = format!(
            "intellisense_v2_diagnostics_pipeline_total_origin_{origin}_trigger_{trigger}_profile_{profile}_reason_{reason}"
        );
        self.metrics.increment(&key);
        if diagnostics_reason_is_cancellation(reason) {
            let key = format!(
                "intellisense_v2_diagnostics_pipeline_cancel_sample_origin_{origin}_trigger_{trigger}_profile_{profile}_reason_{reason}"
            );
            self.metrics.observe_histogram(&key, 1.0);
        }
    }

    pub fn record_intellisense_v2_diagnostics_pipeline_publish_latency(
        &self,
        origin: &str,
        trigger: &str,
        profile: &str,
        duration: Duration,
    ) {
        let origin = normalize_observability_origin_label(origin);
        let trigger = normalize_diagnostics_trigger_label(trigger);
        let profile = normalize_diagnostics_profile_label(profile);
        let key = format!(
            "intellisense_v2_diagnostics_pipeline_publish_ms_origin_{origin}_trigger_{trigger}_profile_{profile}"
        );
        self.metrics
            .observe_histogram(&key, duration.as_millis() as f64);
    }

    pub fn record_intellisense_v2_large_churn_transition(&self, origin: &str, state: &str) {
        let origin = normalize_observability_origin_label(origin);
        let state = normalize_large_churn_state_label(state);
        let key = format!("intellisense_v2_large_churn_state_total_origin_{origin}_state_{state}");
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_heavy_diagnostics_deferred(
        &self,
        origin: &str,
        profile: &str,
        reason: &str,
    ) {
        let origin = normalize_observability_origin_label(origin);
        let profile = normalize_diagnostics_profile_label(profile);
        let reason = normalize_heavy_deferred_reason_label(reason);
        let key = format!(
            "intellisense_v2_heavy_diagnostics_deferred_total_origin_{origin}_profile_{profile}_reason_{reason}"
        );
        self.metrics.increment(&key);
    }

    pub fn record_intellisense_v2_parse_snapshot(
        &self,
        origin: &str,
        mode: &str,
        changed_ranges_count: usize,
        changed_ranges_bytes: usize,
        fallback_reason: Option<&str>,
        build_duration: Duration,
    ) {
        let origin = normalize_observability_origin_label(origin);
        let mode = normalize_parse_snapshot_mode_label(mode);
        let mode_key = format!("intellisense_v2_parse_snapshot_total_origin_{origin}_mode_{mode}");
        self.metrics.increment(&mode_key);
        let latency_key =
            format!("intellisense_v2_parse_snapshot_build_ms_origin_{origin}_mode_{mode}");
        self.metrics
            .observe_histogram(&latency_key, build_duration.as_millis() as f64);

        let changed_ranges_histogram =
            format!("intellisense_v2_parse_snapshot_changed_ranges_count_origin_{origin}");
        let changed_bytes_histogram =
            format!("intellisense_v2_parse_snapshot_changed_ranges_bytes_origin_{origin}");
        self.metrics
            .observe_histogram(&changed_ranges_histogram, changed_ranges_count as f64);
        self.metrics
            .observe_histogram(&changed_bytes_histogram, changed_ranges_bytes as f64);

        if let Some(reason) = fallback_reason {
            let reason = normalize_parse_snapshot_fallback_reason_label(reason);
            let fallback_key = format!(
                "intellisense_v2_parse_snapshot_fallback_total_origin_{origin}_reason_{reason}"
            );
            self.metrics.increment(&fallback_key);
        }
    }

    pub fn record_intellisense_v2_deps_update_build_latency(&self, duration: Duration) {
        self.metrics
            .increment("intellisense_v2_deps_update_build_total");
        self.metrics.observe_histogram(
            "intellisense_v2_deps_update_build_ms",
            duration.as_millis() as f64,
        );
    }

    pub fn record_intellisense_v2_deps_update_apply_latency(&self, duration: Duration) {
        self.metrics
            .increment("intellisense_v2_deps_update_apply_total");
        self.metrics.observe_histogram(
            "intellisense_v2_deps_update_apply_ms",
            duration.as_millis() as f64,
        );
    }

    pub fn record_intellisense_v2_deps_update_success(&self) {
        self.metrics
            .increment("intellisense_v2_deps_update_success_total");
    }

    pub fn record_intellisense_v2_deps_update_error(&self) {
        self.metrics
            .increment("intellisense_v2_deps_update_error_total");
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
        self.metrics
            .add_counter("completion_candidates_total", total_candidates as u64);
        self.metrics
            .add_counter("completion_dedup_removed_total", dedup_removed as u64);
        self.metrics
            .add_counter("completion_prefix_exact_total", prefix_exact as u64);
        self.metrics
            .add_counter("completion_prefix_starts_total", prefix_starts as u64);
        self.metrics
            .add_counter("completion_prefix_contains_total", prefix_contains as u64);
        self.metrics
            .add_counter("completion_prefix_none_total", prefix_none as u64);
        self.metrics
            .add_counter("completion_member_access_total", member_access as u64);
        self.metrics
            .add_counter("completion_has_owner_total", has_owner as u64);

        for score in score_samples {
            self.metrics
                .observe_histogram("completion_score", f64::from(*score));
        }
    }

    /// Простая проверка здоровья
    pub fn health_check(&self) -> HealthStatus {
        let uptime = self.start_time.elapsed();

        HealthStatus {
            status: "healthy".to_string(),
            uptime,
            components: vec![
                ComponentHealth {
                    name: "cache".to_string(),
                    status: "operational".to_string(),
                    details: None,
                },
                ComponentHealth {
                    name: "parser".to_string(),
                    status: "operational".to_string(),
                    details: None,
                },
                ComponentHealth {
                    name: "type_resolver".to_string(),
                    status: "operational".to_string(),
                    details: None,
                },
            ],
        }
    }

    /// Получить метрики
    pub fn get_metrics(&self) -> &SimpleMetrics {
        &self.metrics
    }
}
