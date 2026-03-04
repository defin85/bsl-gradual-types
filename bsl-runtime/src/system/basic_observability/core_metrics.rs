use super::*;

impl BasicObservability {
    /// Логирование запуска системы
    pub fn log_startup(&self) {
        self.logger.info(
            "system_startup",
            json!({
                "timestamp": Utc::now().to_rfc3339(),
                "version": env!("CARGO_PKG_VERSION"),
                "architecture": "simplified"
            }),
        );
    }

    /// Логирование завершения анализа
    pub fn log_analysis(&self, file_path: &str, duration: Duration) {
        self.logger.info(
            "analysis_completed",
            json!({
                "file": file_path,
                "duration_ms": duration.as_millis(),
                "timestamp": Utc::now().to_rfc3339()
            }),
        );

        self.metrics.increment("analyses_total");
        self.metrics
            .observe("analysis_duration_ms", duration.as_millis() as f64);
    }

    pub fn record_index_warmup_hit(&self, duration: Duration) {
        self.metrics.increment("index_warmup_hit_total");
        self.metrics
            .observe("index_warmup_hit_duration_ms", duration.as_millis() as f64);
    }

    pub fn record_index_warmup_miss(&self, duration: Duration) {
        self.metrics.increment("index_warmup_miss_total");
        self.metrics
            .observe("index_warmup_miss_duration_ms", duration.as_millis() as f64);
    }

    pub fn record_index_warmup_skip(&self, reason: &str) {
        self.metrics
            .increment(&format!("index_warmup_skip_total_{}", reason));
    }

    pub(super) fn record_observability_contract_violation(&self, reason: &str) {
        self.metrics
            .increment("intellisense_v2_observability_contract_violation_total");
        self.metrics.increment(&format!(
            "intellisense_v2_observability_contract_violation_reason_{}",
            sanitize_identifier(reason)
        ));
    }

    pub(super) fn record_projection_missing(&self, reason: &str) {
        self.metrics
            .increment("intellisense_v2_projection_missing_total");
        self.metrics.increment(&format!(
            "intellisense_v2_projection_missing_reason_{}",
            sanitize_identifier(reason)
        ));
    }

    pub(super) fn observe_value_kind(&self, key: &str, kind: CanonicalValueKind, value: f64) {
        match kind {
            CanonicalValueKind::Counter => self.metrics.add_counter(key, value.max(0.0) as u64),
            CanonicalValueKind::HistogramMs => self.metrics.observe_histogram(key, value),
            CanonicalValueKind::Gauge => self.metrics.observe(key, value),
        }
    }

    pub(super) fn observe_legacy_target(&self, target: LegacyMetricTarget<'_>, value: f64) {
        match target.kind {
            LegacyMetricKind::Counter => {
                self.metrics.add_counter(target.key, value.max(0.0) as u64)
            }
            LegacyMetricKind::HistogramMs => self.metrics.observe_histogram(target.key, value),
            LegacyMetricKind::Gauge => self.metrics.observe(target.key, value),
        }
    }

    pub(super) fn canonical_metric_key(event: &CanonicalEvent<'_>) -> String {
        let mut key = format!("intellisense_v2_drilldown_{}", event.family.as_str());
        key.push_str("_origin_");
        key.push_str(event.origin);
        if let Some(mode) = event.mode {
            key.push_str("_mode_");
            key.push_str(mode);
        }
        if let Some(operation) = event.operation {
            key.push_str("_operation_");
            key.push_str(operation);
        }
        if let Some(stage) = event.stage {
            key.push_str("_stage_");
            key.push_str(stage);
        }
        if let Some(outcome) = event.outcome {
            key.push_str("_outcome_");
            key.push_str(outcome);
        }
        if let Some(reason) = event.reason {
            key.push_str("_reason_");
            key.push_str(reason);
        }
        if let Some(query_kind) = event.query_kind {
            key.push_str("_query_kind_");
            key.push_str(query_kind);
        }
        if let Some(work_class) = event.work_class {
            key.push_str("_work_class_");
            key.push_str(work_class);
        }
        if let Some(saturation_metric) = event.saturation_metric {
            key.push_str("_saturation_metric_");
            key.push_str(saturation_metric);
        }
        key
    }

    pub(super) fn validate_canonical_event(event: &CanonicalEvent<'_>) -> Result<(), &'static str> {
        if !contains_allowed(ALLOWED_ORIGINS, event.origin) {
            return Err("invalid_origin");
        }
        if !event.value.is_finite() {
            return Err("invalid_value");
        }
        if let Some(mode) = event.mode {
            if !contains_allowed(ALLOWED_COMPLETION_MODES, mode) {
                return Err("invalid_mode");
            }
        }
        if let Some(operation) = event.operation {
            if !contains_allowed(ALLOWED_OPERATIONS, operation) {
                return Err("invalid_operation");
            }
        }
        if let Some(stage) = event.stage {
            if !contains_allowed(ALLOWED_STAGES, stage) {
                return Err("invalid_stage");
            }
        }
        if let Some(outcome) = event.outcome {
            if !contains_allowed(ALLOWED_OUTCOMES, outcome) {
                return Err("invalid_outcome");
            }
        }
        if let Some(reason) = event.reason {
            if !contains_allowed(ALLOWED_REASONS, reason) {
                return Err("invalid_reason");
            }
        }
        if let Some(query_kind) = event.query_kind {
            if !contains_allowed(ALLOWED_QUERY_KINDS, query_kind) {
                return Err("invalid_query_kind");
            }
        }
        if let Some(work_class) = event.work_class {
            if !contains_allowed(ALLOWED_WORK_CLASSES, work_class) {
                return Err("invalid_work_class");
            }
        }
        if let Some(metric) = event.saturation_metric {
            if !contains_allowed(ALLOWED_SATURATION_METRICS, metric) {
                return Err("invalid_saturation_metric");
            }
        }

        match event.family {
            CanonicalFamily::StageTotal | CanonicalFamily::StageLatencyMs => {
                if event.operation.is_none() || event.stage.is_none() {
                    return Err("stage_family_requires_operation_and_stage");
                }
                if event.outcome.is_some()
                    || event.reason.is_some()
                    || event.query_kind.is_some()
                    || event.work_class.is_some()
                    || event.saturation_metric.is_some()
                {
                    return Err("stage_family_has_forbidden_dimensions");
                }
            }
            CanonicalFamily::StageReasonTotal => {
                if event.operation.is_none() || event.stage.is_none() || event.reason.is_none() {
                    return Err("stage_reason_family_requires_operation_stage_reason");
                }
                if event.outcome.is_some()
                    || event.query_kind.is_some()
                    || event.work_class.is_some()
                    || event.saturation_metric.is_some()
                {
                    return Err("stage_reason_family_has_forbidden_dimensions");
                }
            }
            CanonicalFamily::SingleflightEffectivenessTotal => {
                if event.query_kind.is_none() || event.outcome.is_none() {
                    return Err("singleflight_family_requires_query_kind_and_outcome");
                }
                if event.operation.is_some()
                    || event.stage.is_some()
                    || event.reason.is_some()
                    || event.work_class.is_some()
                    || event.saturation_metric.is_some()
                {
                    return Err("singleflight_family_has_forbidden_dimensions");
                }
            }
            CanonicalFamily::SaturationSampleTotal | CanonicalFamily::SaturationSampleLatencyMs => {
                if event.work_class.is_none() || event.reason.is_none() {
                    return Err("saturation_sample_family_requires_work_class_and_reason");
                }
                if event.operation.is_some()
                    || event.stage.is_some()
                    || event.outcome.is_some()
                    || event.query_kind.is_some()
                    || event.saturation_metric.is_some()
                {
                    return Err("saturation_sample_family_has_forbidden_dimensions");
                }
            }
            CanonicalFamily::SaturationGauge => {
                if event.saturation_metric.is_none() {
                    return Err("saturation_family_requires_metric");
                }
                if event.operation.is_some()
                    || event.stage.is_some()
                    || event.outcome.is_some()
                    || event.reason.is_some()
                    || event.query_kind.is_some()
                {
                    return Err("saturation_family_has_forbidden_dimensions");
                }
            }
        }

        let expected_value_kind = match event.family {
            CanonicalFamily::StageTotal
            | CanonicalFamily::StageReasonTotal
            | CanonicalFamily::SingleflightEffectivenessTotal
            | CanonicalFamily::SaturationSampleTotal => CanonicalValueKind::Counter,
            CanonicalFamily::StageLatencyMs | CanonicalFamily::SaturationSampleLatencyMs => {
                CanonicalValueKind::HistogramMs
            }
            CanonicalFamily::SaturationGauge => CanonicalValueKind::Gauge,
        };
        if expected_value_kind != event.value_kind {
            return Err("family_value_kind_mismatch");
        }

        Ok(())
    }

    pub(super) fn canonical_legacy_projection_target(
        event: &CanonicalEvent<'_>,
    ) -> Option<LegacyMetricTarget<'static>> {
        match event.family {
            CanonicalFamily::StageTotal => {
                let operation = event.operation?;
                let stage = event.stage?;
                match stage {
                    "runtime_wait_for_file_version" => {
                        let (counter_key, _latency_key) =
                            legacy_wait_for_file_version_metrics(operation);
                        Some(LegacyMetricTarget {
                            key: counter_key,
                            kind: LegacyMetricKind::Counter,
                        })
                    }
                    "runtime_snapshot_with_deps" => {
                        let (counter_key, _latency_key) = legacy_snapshot_metrics(operation);
                        Some(LegacyMetricTarget {
                            key: counter_key,
                            kind: LegacyMetricKind::Counter,
                        })
                    }
                    "ir_query" => {
                        let (counter_key, _latency_key) = legacy_ir_query_metrics(operation);
                        Some(LegacyMetricTarget {
                            key: counter_key,
                            kind: LegacyMetricKind::Counter,
                        })
                    }
                    "syntax_diagnostics_query" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_syntax_diagnostics_query_total",
                        kind: LegacyMetricKind::Counter,
                    }),
                    "semantic_diagnostics_query" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_semantic_diagnostics_query_total",
                        kind: LegacyMetricKind::Counter,
                    }),
                    "parse_result_query" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_parse_result_query_total",
                        kind: LegacyMetricKind::Counter,
                    }),
                    "runtime_queue_wait" => {
                        let (counter_key, _latency_key) =
                            legacy_runtime_queue_wait_metrics(operation);
                        Some(LegacyMetricTarget {
                            key: counter_key,
                            kind: LegacyMetricKind::Counter,
                        })
                    }
                    "runtime_exec" => {
                        let (counter_key, _latency_key) = legacy_runtime_exec_metrics(operation);
                        Some(LegacyMetricTarget {
                            key: counter_key,
                            kind: LegacyMetricKind::Counter,
                        })
                    }
                    _ => None,
                }
            }
            CanonicalFamily::StageLatencyMs => {
                let operation = event.operation?;
                let stage = event.stage?;
                match stage {
                    "runtime_wait_for_file_version" => {
                        let (_counter_key, latency_key) =
                            legacy_wait_for_file_version_metrics(operation);
                        Some(LegacyMetricTarget {
                            key: latency_key,
                            kind: LegacyMetricKind::HistogramMs,
                        })
                    }
                    "runtime_snapshot_with_deps" => {
                        let (_counter_key, latency_key) = legacy_snapshot_metrics(operation);
                        Some(LegacyMetricTarget {
                            key: latency_key,
                            kind: LegacyMetricKind::HistogramMs,
                        })
                    }
                    "ir_query" => {
                        let (_counter_key, latency_key) = legacy_ir_query_metrics(operation);
                        Some(LegacyMetricTarget {
                            key: latency_key,
                            kind: LegacyMetricKind::HistogramMs,
                        })
                    }
                    "syntax_diagnostics_query" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_syntax_diagnostics_query_ms",
                        kind: LegacyMetricKind::HistogramMs,
                    }),
                    "semantic_diagnostics_query" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_semantic_diagnostics_query_ms",
                        kind: LegacyMetricKind::HistogramMs,
                    }),
                    "parse_result_query" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_parse_result_query_ms",
                        kind: LegacyMetricKind::HistogramMs,
                    }),
                    "runtime_queue_wait" => {
                        let (_counter_key, latency_key) =
                            legacy_runtime_queue_wait_metrics(operation);
                        Some(LegacyMetricTarget {
                            key: latency_key,
                            kind: LegacyMetricKind::HistogramMs,
                        })
                    }
                    "runtime_exec" => {
                        let (_counter_key, latency_key) = legacy_runtime_exec_metrics(operation);
                        Some(LegacyMetricTarget {
                            key: latency_key,
                            kind: LegacyMetricKind::HistogramMs,
                        })
                    }
                    _ => None,
                }
            }
            CanonicalFamily::StageReasonTotal => {
                let stage = event.stage?;
                let operation = event.operation?;
                let reason = event.reason?;
                match (stage, reason) {
                    ("ir_query", "other") => Some(LegacyMetricTarget {
                        key: legacy_ir_query_cancelled_metric(operation),
                        kind: LegacyMetricKind::Counter,
                    }),
                    ("syntax_diagnostics_query", "syntax") => Some(LegacyMetricTarget {
                        key: "intellisense_v2_query_cancelled_total_syntax",
                        kind: LegacyMetricKind::Counter,
                    }),
                    ("semantic_diagnostics_query", "semantic") => Some(LegacyMetricTarget {
                        key: "intellisense_v2_query_cancelled_total_semantic",
                        kind: LegacyMetricKind::Counter,
                    }),
                    ("parse_result_query", "other") => Some(LegacyMetricTarget {
                        key: "intellisense_v2_query_cancelled_total_other",
                        kind: LegacyMetricKind::Counter,
                    }),
                    _ => None,
                }
            }
            CanonicalFamily::SingleflightEffectivenessTotal => {
                let outcome = event.outcome?;
                match outcome {
                    "leader" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_singleflight_leader_total",
                        kind: LegacyMetricKind::Counter,
                    }),
                    "shared" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_singleflight_shared_total",
                        kind: LegacyMetricKind::Counter,
                    }),
                    "key_unavailable" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_singleflight_key_unavailable_total",
                        kind: LegacyMetricKind::Counter,
                    }),
                    _ => None,
                }
            }
            CanonicalFamily::SaturationSampleTotal => {
                let work_class = event.work_class?;
                let reason = event.reason?;
                match (reason, work_class) {
                    ("queue_wait", "interactive") => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_queue_wait_interactive_total",
                        kind: LegacyMetricKind::Counter,
                    }),
                    ("queue_wait", "background") => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_queue_wait_background_total",
                        kind: LegacyMetricKind::Counter,
                    }),
                    ("exec", "interactive") => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_exec_interactive_total",
                        kind: LegacyMetricKind::Counter,
                    }),
                    ("exec", "background") => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_exec_background_total",
                        kind: LegacyMetricKind::Counter,
                    }),
                    _ => None,
                }
            }
            CanonicalFamily::SaturationSampleLatencyMs => {
                let work_class = event.work_class?;
                let reason = event.reason?;
                match (reason, work_class) {
                    ("queue_wait", "interactive") => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_queue_wait_interactive_ms",
                        kind: LegacyMetricKind::HistogramMs,
                    }),
                    ("queue_wait", "background") => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_queue_wait_background_ms",
                        kind: LegacyMetricKind::HistogramMs,
                    }),
                    ("exec", "interactive") => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_exec_interactive_ms",
                        kind: LegacyMetricKind::HistogramMs,
                    }),
                    ("exec", "background") => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_exec_background_ms",
                        kind: LegacyMetricKind::HistogramMs,
                    }),
                    _ => None,
                }
            }
            CanonicalFamily::SaturationGauge => {
                let saturation_metric = event.saturation_metric?;
                match saturation_metric {
                    "waiters_interactive" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_saturation_waiters_interactive",
                        kind: LegacyMetricKind::Gauge,
                    }),
                    "waiters_background" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_saturation_waiters_background",
                        kind: LegacyMetricKind::Gauge,
                    }),
                    "permits_interactive" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_saturation_permits_interactive",
                        kind: LegacyMetricKind::Gauge,
                    }),
                    "permits_background" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_saturation_permits_background",
                        kind: LegacyMetricKind::Gauge,
                    }),
                    "permits_shared" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_saturation_permits_shared",
                        kind: LegacyMetricKind::Gauge,
                    }),
                    "queue_depth_total" => Some(LegacyMetricTarget {
                        key: "intellisense_v2_runtime_saturation_queue_depth_total",
                        kind: LegacyMetricKind::Gauge,
                    }),
                    _ => None,
                }
            }
        }
    }

    pub(super) fn emit_canonical_event(
        &self,
        event: CanonicalEvent<'_>,
        legacy_target: Option<LegacyMetricTarget<'_>>,
    ) {
        if let Err(reason) = Self::validate_canonical_event(&event) {
            self.record_observability_contract_violation(reason);
            return;
        }

        let projected_legacy_target = if event.requires_legacy_projection {
            let projected = Self::canonical_legacy_projection_target(&event);
            if projected.is_none() {
                self.record_projection_missing("missing_projection_mapping");
                return;
            }
            projected
        } else {
            None
        };

        if let Some(hint) = legacy_target {
            if let Some(projected) = projected_legacy_target {
                if hint.key != projected.key || hint.kind != projected.kind {
                    self.record_observability_contract_violation("projection_hint_mismatch");
                }
            } else {
                self.record_observability_contract_violation("unexpected_projection_hint");
            }
        } else if event.requires_legacy_projection {
            self.record_observability_contract_violation("projection_hint_missing");
        }

        let drilldown_key = Self::canonical_metric_key(&event);
        self.observe_value_kind(&drilldown_key, event.value_kind, event.value);
        if let Some(target) = projected_legacy_target {
            self.observe_legacy_target(target, event.value);
        }
    }
}
