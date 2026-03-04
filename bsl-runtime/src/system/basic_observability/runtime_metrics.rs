use super::*;

impl BasicObservability {
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
