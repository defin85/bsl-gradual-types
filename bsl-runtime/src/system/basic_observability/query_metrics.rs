use super::*;

impl BasicObservability {
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
        let origin = normalize_observability_origin_label(origin);
        let operation = normalize_operation_label(operation);
        let stage = normalize_payload_shape_stage_label(stage);
        let size_bucket = payload_size_bucket(file_bytes);
        let line_bucket = payload_line_bucket(line_count);

        let bucket_counter = format!(
            "intellisense_v2_payload_shape_total_origin_{origin}_operation_{operation}_stage_{stage}_size_bucket_{size_bucket}_line_bucket_{line_bucket}"
        );
        let bytes_histogram =
            format!("intellisense_v2_payload_shape_bytes_origin_{origin}_operation_{operation}_stage_{stage}");
        let lines_histogram =
            format!("intellisense_v2_payload_shape_lines_origin_{origin}_operation_{operation}_stage_{stage}");

        self.metrics.increment(&bucket_counter);
        self.metrics
            .observe_histogram(&bytes_histogram, file_bytes as f64);
        self.metrics
            .observe_histogram(&lines_histogram, line_count as f64);
    }

    pub fn record_intellisense_v2_wait_for_file_version(&self, kind: &str, duration: Duration) {
        self.record_intellisense_v2_wait_for_file_version_with_origin("runtime", kind, duration);
    }

    pub fn record_intellisense_v2_wait_for_file_version_with_origin(
        &self,
        origin: &str,
        kind: &str,
        duration: Duration,
    ) {
        self.record_intellisense_v2_wait_for_file_version_with_origin_and_mode(
            origin, kind, None, duration,
        );
    }

    pub fn record_intellisense_v2_wait_for_file_version_with_origin_and_mode(
        &self,
        origin: &str,
        kind: &str,
        completion_mode: Option<&str>,
        duration: Duration,
    ) {
        let completion_mode = completion_mode.map(normalize_completion_observability_mode_label);
        let (total_metric, histogram_metric) = legacy_wait_for_file_version_metrics(kind);
        let operation = normalize_operation_label(kind);
        let stage = "runtime_wait_for_file_version";
        let elapsed_ms = duration.as_millis() as f64;
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageTotal,
                origin,
                mode: completion_mode,
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
                mode: completion_mode,
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

    pub fn record_intellisense_v2_snapshot_latency(&self, kind: &str, duration: Duration) {
        self.record_intellisense_v2_snapshot_latency_with_origin("runtime", kind, duration);
    }

    pub fn record_intellisense_v2_snapshot_latency_with_origin(
        &self,
        origin: &str,
        kind: &str,
        duration: Duration,
    ) {
        self.record_intellisense_v2_snapshot_latency_with_origin_and_mode(
            origin, kind, None, duration,
        );
    }

    pub fn record_intellisense_v2_snapshot_latency_with_origin_and_mode(
        &self,
        origin: &str,
        kind: &str,
        completion_mode: Option<&str>,
        duration: Duration,
    ) {
        let completion_mode = completion_mode.map(normalize_completion_observability_mode_label);
        let (total_metric, histogram_metric) = legacy_snapshot_metrics(kind);
        let operation = normalize_operation_label(kind);
        let stage = "runtime_snapshot_with_deps";
        let elapsed_ms = duration.as_millis() as f64;
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageTotal,
                origin,
                mode: completion_mode,
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
                mode: completion_mode,
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

    pub fn record_intellisense_v2_ir_query_latency(&self, kind: &str, duration: Duration) {
        self.record_intellisense_v2_ir_query_latency_with_origin("runtime", kind, duration);
    }

    pub fn record_intellisense_v2_ir_query_latency_with_origin(
        &self,
        origin: &str,
        kind: &str,
        duration: Duration,
    ) {
        self.record_intellisense_v2_ir_query_latency_with_origin_and_mode(
            origin, kind, None, duration,
        );
    }

    pub fn record_intellisense_v2_ir_query_latency_with_origin_and_mode(
        &self,
        origin: &str,
        kind: &str,
        completion_mode: Option<&str>,
        duration: Duration,
    ) {
        let completion_mode = completion_mode.map(normalize_completion_observability_mode_label);
        let (total_metric, histogram_metric) = legacy_ir_query_metrics(kind);
        let operation = normalize_operation_label(kind);
        let stage = "ir_query";
        let elapsed_ms = duration.as_millis() as f64;
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageTotal,
                origin,
                mode: completion_mode,
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
                mode: completion_mode,
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

    pub fn record_intellisense_v2_ir_query_cancelled(&self, kind: &str) {
        self.record_intellisense_v2_ir_query_cancelled_with_origin("runtime", kind);
    }

    pub fn record_intellisense_v2_ir_query_cancelled_with_origin(&self, origin: &str, kind: &str) {
        self.record_intellisense_v2_ir_query_cancelled_with_origin_and_mode(origin, kind, None);
    }

    pub fn record_intellisense_v2_ir_query_cancelled_with_origin_and_mode(
        &self,
        origin: &str,
        kind: &str,
        completion_mode: Option<&str>,
    ) {
        let completion_mode = completion_mode.map(normalize_completion_observability_mode_label);
        let metric = legacy_ir_query_cancelled_metric(kind);
        let operation = normalize_operation_label(kind);
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageReasonTotal,
                origin,
                mode: completion_mode,
                operation: Some(operation),
                stage: Some("ir_query"),
                outcome: None,
                reason: Some("other"),
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

    pub fn record_intellisense_v2_syntax_diagnostics_query_latency(&self, duration: Duration) {
        self.record_intellisense_v2_syntax_diagnostics_query_latency_with_origin(
            "runtime", duration,
        );
    }

    pub fn record_intellisense_v2_syntax_diagnostics_query_latency_with_origin(
        &self,
        origin: &str,
        duration: Duration,
    ) {
        let elapsed_ms = duration.as_millis() as f64;
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageTotal,
                origin,
                mode: None,
                operation: Some("diagnostics"),
                stage: Some("syntax_diagnostics_query"),
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
                key: "intellisense_v2_syntax_diagnostics_query_total",
                kind: LegacyMetricKind::Counter,
            }),
        );
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageLatencyMs,
                origin,
                mode: None,
                operation: Some("diagnostics"),
                stage: Some("syntax_diagnostics_query"),
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
                key: "intellisense_v2_syntax_diagnostics_query_ms",
                kind: LegacyMetricKind::HistogramMs,
            }),
        );
    }

    pub fn record_intellisense_v2_semantic_diagnostics_query_latency(&self, duration: Duration) {
        self.record_intellisense_v2_semantic_diagnostics_query_latency_with_origin(
            "runtime", duration,
        );
    }

    pub fn record_intellisense_v2_semantic_diagnostics_query_latency_with_origin(
        &self,
        origin: &str,
        duration: Duration,
    ) {
        let elapsed_ms = duration.as_millis() as f64;
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageTotal,
                origin,
                mode: None,
                operation: Some("diagnostics"),
                stage: Some("semantic_diagnostics_query"),
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
                key: "intellisense_v2_semantic_diagnostics_query_total",
                kind: LegacyMetricKind::Counter,
            }),
        );
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageLatencyMs,
                origin,
                mode: None,
                operation: Some("diagnostics"),
                stage: Some("semantic_diagnostics_query"),
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
                key: "intellisense_v2_semantic_diagnostics_query_ms",
                kind: LegacyMetricKind::HistogramMs,
            }),
        );
    }

    pub fn record_intellisense_v2_parse_result_query_latency(&self, duration: Duration) {
        self.record_intellisense_v2_parse_result_query_latency_with_origin_and_operation(
            "runtime", "other", duration,
        );
    }

    pub fn record_intellisense_v2_parse_result_query_latency_with_origin(
        &self,
        origin: &str,
        duration: Duration,
    ) {
        self.record_intellisense_v2_parse_result_query_latency_with_origin_and_operation(
            origin, "other", duration,
        );
    }

    pub fn record_intellisense_v2_parse_result_query_latency_with_origin_and_operation(
        &self,
        origin: &str,
        operation: &str,
        duration: Duration,
    ) {
        self.record_intellisense_v2_parse_result_query_latency_with_origin_operation_and_mode(
            origin, operation, None, duration,
        );
    }

    pub fn record_intellisense_v2_parse_result_query_latency_with_origin_operation_and_mode(
        &self,
        origin: &str,
        operation: &str,
        completion_mode: Option<&str>,
        duration: Duration,
    ) {
        let completion_mode = completion_mode.map(normalize_completion_observability_mode_label);
        let operation = normalize_operation_label(operation);
        let elapsed_ms = duration.as_millis() as f64;
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageTotal,
                origin,
                mode: completion_mode,
                operation: Some(operation),
                stage: Some("parse_result_query"),
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
                key: "intellisense_v2_parse_result_query_total",
                kind: LegacyMetricKind::Counter,
            }),
        );
        self.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageLatencyMs,
                origin,
                mode: completion_mode,
                operation: Some(operation),
                stage: Some("parse_result_query"),
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
                key: "intellisense_v2_parse_result_query_ms",
                kind: LegacyMetricKind::HistogramMs,
            }),
        );
    }
}
