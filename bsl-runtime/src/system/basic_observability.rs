//! Basic Observability - простая замена сложного observability stack
//!
//! Structured logging + basic metrics вместо полного enterprise стека

use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Простая система наблюдения
pub struct BasicObservability {
    logger: StructuredLogger,
    metrics: SimpleMetrics,
    start_time: Instant,
}

/// Структурированные логи
pub struct StructuredLogger {
    // Wrapper для tracing с JSON форматированием
}

/// Простые метрики
pub struct SimpleMetrics {
    counters: Arc<Mutex<HashMap<String, u64>>>,
    gauges: Arc<Mutex<HashMap<String, f64>>>,
    histograms: Arc<Mutex<HashMap<String, Vec<f64>>>>,
    start_time: Instant,
}

/// Статус здоровья системы
pub struct HealthStatus {
    pub status: String,
    pub uptime: Duration,
    pub components: Vec<ComponentHealth>,
}

/// Здоровье отдельного компонента
pub struct ComponentHealth {
    pub name: String,
    pub status: String,
    pub details: Option<String>,
}

const UNIFIED_INTELLISENSE_V2_COUNTER_KEYS: &[&str] = &[
    "intellisense_v2_runtime_wait_for_file_version_queue_wait_total",
    "intellisense_v2_runtime_wait_for_file_version_exec_total",
    "intellisense_v2_runtime_snapshot_with_deps_queue_wait_total",
    "intellisense_v2_runtime_snapshot_with_deps_exec_total",
    "intellisense_v2_parse_snapshot_total_origin_lsp_mode_incremental",
    "intellisense_v2_parse_snapshot_total_origin_lsp_mode_reused",
    "intellisense_v2_parse_snapshot_total_origin_lsp_mode_full",
    "intellisense_v2_parse_snapshot_total_origin_lsp_mode_other",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_incremental_failed",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_no_previous_tree",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_no_edits_provided",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_other",
    "intellisense_v2_wait_for_file_version_diagnostics_total",
    "intellisense_v2_snapshot_diagnostics_total",
    "intellisense_v2_ir_query_other_total",
    "intellisense_v2_syntax_diagnostics_query_total",
    "intellisense_v2_semantic_diagnostics_query_total",
    "intellisense_v2_parse_result_query_total",
    "intellisense_v2_ir_query_cancelled_total_other",
    "intellisense_v2_query_cancelled_total_syntax",
    "intellisense_v2_query_cancelled_total_semantic",
    "intellisense_v2_interactive_wait_budget_exhausted_total",
    "intellisense_v2_interactive_stale_served_total",
    "intellisense_v2_interactive_knob_clamped_total",
    "intellisense_v2_singleflight_leader_total",
    "intellisense_v2_singleflight_shared_total",
    "intellisense_v2_singleflight_key_unavailable_total",
    "intellisense_v2_runtime_queue_wait_interactive_total",
    "intellisense_v2_runtime_queue_wait_background_total",
    "intellisense_v2_runtime_exec_interactive_total",
    "intellisense_v2_runtime_exec_background_total",
    "intellisense_v2_completion_stale_fallback_total",
    "intellisense_v2_completion_fallback_unavailable_total",
    "intellisense_v2_revision_lag_sample_total",
    "intellisense_v2_observability_contract_violation_total",
    "intellisense_v2_projection_missing_total",
    "intellisense_v2_runtime_saturation_sample_total",
];

const UNIFIED_INTELLISENSE_V2_HISTOGRAM_KEYS: &[&str] = &[
    "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms",
    "intellisense_v2_runtime_wait_for_file_version_exec_ms",
    "intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms",
    "intellisense_v2_runtime_snapshot_with_deps_exec_ms",
    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_incremental",
    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_reused",
    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_full",
    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_other",
    "intellisense_v2_parse_snapshot_changed_ranges_count_origin_lsp",
    "intellisense_v2_parse_snapshot_changed_ranges_bytes_origin_lsp",
    "intellisense_v2_wait_for_file_version_diagnostics_ms",
    "intellisense_v2_snapshot_diagnostics_ms",
    "intellisense_v2_ir_query_other_ms",
    "intellisense_v2_syntax_diagnostics_query_ms",
    "intellisense_v2_semantic_diagnostics_query_ms",
    "intellisense_v2_parse_result_query_ms",
    "intellisense_v2_singleflight_wait_ms",
    "intellisense_v2_runtime_queue_wait_interactive_ms",
    "intellisense_v2_runtime_queue_wait_background_ms",
    "intellisense_v2_runtime_exec_interactive_ms",
    "intellisense_v2_runtime_exec_background_ms",
    "intellisense_v2_revision_lag_versions",
];

const UNIFIED_INTELLISENSE_V2_GAUGE_KEYS: &[&str] = &[
    "intellisense_v2_runtime_saturation_waiters_interactive",
    "intellisense_v2_runtime_saturation_waiters_background",
    "intellisense_v2_runtime_saturation_permits_interactive",
    "intellisense_v2_runtime_saturation_permits_background",
    "intellisense_v2_runtime_saturation_permits_shared",
    "intellisense_v2_runtime_saturation_queue_depth_total",
];

const ALLOWED_ORIGINS: &[&str] = &["lsp", "web", "agent", "runtime"];
const ALLOWED_OPERATIONS: &[&str] = &[
    "completion",
    "hover",
    "signature_help",
    "definition",
    "document_symbol",
    "rename",
    "diagnostics",
    "members",
    "type_at_position",
    "symbol_search",
    "references",
    "wait_for_file_version",
    "snapshot_with_deps",
    "other",
];
const ALLOWED_STAGES: &[&str] = &[
    "runtime_wait_for_file_version",
    "runtime_snapshot_with_deps",
    "runtime_queue_wait",
    "runtime_exec",
    "ir_query",
    "syntax_diagnostics_query",
    "semantic_diagnostics_query",
    "parse_result_query",
];
const ALLOWED_OUTCOMES: &[&str] = &[
    "success",
    "empty",
    "cancelled",
    "error",
    "stale_version",
    "missing_deps",
    "leader",
    "shared",
    "key_unavailable",
];
const ALLOWED_REASONS: &[&str] = &["syntax", "semantic", "other", "queue_wait", "exec"];
const ALLOWED_QUERY_KINDS: &[&str] = &["parse_result", "syntax_diagnostics", "ir", "other"];
const ALLOWED_WORK_CLASSES: &[&str] = &["interactive", "background"];
const ALLOWED_COMPLETION_MODES: &[&str] = &["legacy", "event_driven", "shadow"];
const ALLOWED_SATURATION_METRICS: &[&str] = &[
    "waiters_interactive",
    "waiters_background",
    "permits_interactive",
    "permits_background",
    "permits_shared",
    "queue_depth_total",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanonicalFamily {
    StageTotal,
    StageLatencyMs,
    StageReasonTotal,
    SingleflightEffectivenessTotal,
    SaturationSampleTotal,
    SaturationSampleLatencyMs,
    SaturationGauge,
}

impl CanonicalFamily {
    fn as_str(self) -> &'static str {
        match self {
            CanonicalFamily::StageTotal => "stage_total",
            CanonicalFamily::StageLatencyMs => "stage_latency_ms",
            CanonicalFamily::StageReasonTotal => "stage_reason_total",
            CanonicalFamily::SingleflightEffectivenessTotal => "singleflight_effectiveness_total",
            CanonicalFamily::SaturationSampleTotal => "saturation_sample_total",
            CanonicalFamily::SaturationSampleLatencyMs => "saturation_sample_latency_ms",
            CanonicalFamily::SaturationGauge => "saturation_gauge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanonicalValueKind {
    Counter,
    HistogramMs,
    Gauge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyMetricKind {
    Counter,
    HistogramMs,
    Gauge,
}

#[derive(Debug, Clone, Copy)]
struct LegacyMetricTarget<'a> {
    key: &'a str,
    kind: LegacyMetricKind,
}

#[derive(Debug, Clone, Copy)]
struct CanonicalEvent<'a> {
    family: CanonicalFamily,
    origin: &'a str,
    mode: Option<&'a str>,
    operation: Option<&'a str>,
    stage: Option<&'a str>,
    outcome: Option<&'a str>,
    reason: Option<&'a str>,
    query_kind: Option<&'a str>,
    work_class: Option<&'a str>,
    saturation_metric: Option<&'a str>,
    value_kind: CanonicalValueKind,
    value: f64,
    requires_legacy_projection: bool,
}

impl Default for BasicObservability {
    fn default() -> Self {
        let metrics = SimpleMetrics::new();
        register_unified_intellisense_v2_contract_metrics(&metrics);
        Self {
            logger: StructuredLogger::new(),
            metrics,
            start_time: Instant::now(),
        }
    }
}

fn register_unified_intellisense_v2_contract_metrics(metrics: &SimpleMetrics) {
    for key in UNIFIED_INTELLISENSE_V2_COUNTER_KEYS {
        metrics.register_counter(key);
    }
    for key in UNIFIED_INTELLISENSE_V2_HISTOGRAM_KEYS {
        metrics.register_histogram(key);
    }
    for key in UNIFIED_INTELLISENSE_V2_GAUGE_KEYS {
        metrics.register_gauge(key);
    }
}

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

    fn record_observability_contract_violation(&self, reason: &str) {
        self.metrics
            .increment("intellisense_v2_observability_contract_violation_total");
        self.metrics.increment(&format!(
            "intellisense_v2_observability_contract_violation_reason_{}",
            sanitize_identifier(reason)
        ));
    }

    fn record_projection_missing(&self, reason: &str) {
        self.metrics
            .increment("intellisense_v2_projection_missing_total");
        self.metrics.increment(&format!(
            "intellisense_v2_projection_missing_reason_{}",
            sanitize_identifier(reason)
        ));
    }

    fn observe_value_kind(&self, key: &str, kind: CanonicalValueKind, value: f64) {
        match kind {
            CanonicalValueKind::Counter => self.metrics.add_counter(key, value.max(0.0) as u64),
            CanonicalValueKind::HistogramMs => self.metrics.observe_histogram(key, value),
            CanonicalValueKind::Gauge => self.metrics.observe(key, value),
        }
    }

    fn observe_legacy_target(&self, target: LegacyMetricTarget<'_>, value: f64) {
        match target.kind {
            LegacyMetricKind::Counter => {
                self.metrics.add_counter(target.key, value.max(0.0) as u64)
            }
            LegacyMetricKind::HistogramMs => self.metrics.observe_histogram(target.key, value),
            LegacyMetricKind::Gauge => self.metrics.observe(target.key, value),
        }
    }

    fn canonical_metric_key(event: &CanonicalEvent<'_>) -> String {
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

    fn validate_canonical_event(event: &CanonicalEvent<'_>) -> Result<(), &'static str> {
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

    fn canonical_legacy_projection_target(
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

    fn emit_canonical_event(
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
            "sync_globals" => "completion_stage_sync_globals_ms",
            "query_bundle" => "completion_stage_query_bundle_ms",
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
        let metric = match outcome {
            "wait_not_ready" => "intellisense_v2_completion_result_total_wait_not_ready",
            "missing_file_content" => {
                "intellisense_v2_completion_result_total_missing_file_content"
            }
            "missing_file_path" => "intellisense_v2_completion_result_total_missing_file_path",
            "missing_deps" => "intellisense_v2_completion_result_total_missing_deps",
            "missing_ir" => "intellisense_v2_completion_result_total_missing_ir",
            "degraded_incomplete" => "intellisense_v2_completion_result_total_degraded_incomplete",
            "fallback_unavailable" => {
                "intellisense_v2_completion_result_total_fallback_unavailable"
            }
            "cancelled" => "intellisense_v2_completion_result_total_cancelled",
            "handler_error" => "intellisense_v2_completion_result_total_handler_error",
            "ok_empty" => "intellisense_v2_completion_result_total_ok_empty",
            "ok_non_empty" => "intellisense_v2_completion_result_total_ok_non_empty",
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

impl StructuredLogger {
    fn new() -> Self {
        Self {}
    }

    fn info(&self, event: &str, data: serde_json::Value) {
        info!(event = event, data = %data, "Structured log entry");
    }

    #[allow(dead_code)]
    fn warn(&self, event: &str, data: serde_json::Value) {
        warn!(event = event, data = %data, "Structured warning");
    }

    #[allow(dead_code)]
    fn error(&self, event: &str, data: serde_json::Value) {
        tracing::error!(event = event, data = %data, "Structured error");
    }
}

impl SimpleMetrics {
    fn new() -> Self {
        Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
            gauges: Arc::new(Mutex::new(HashMap::new())),
            histograms: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    fn increment(&self, metric: &str) {
        if let Ok(mut counters) = self.counters.lock() {
            *counters.entry(metric.to_string()).or_insert(0) += 1;
        }
    }

    fn register_counter(&self, metric: &str) {
        if let Ok(mut counters) = self.counters.lock() {
            counters.entry(metric.to_string()).or_insert(0);
        }
    }

    fn register_gauge(&self, metric: &str) {
        if let Ok(mut gauges) = self.gauges.lock() {
            gauges.entry(metric.to_string()).or_insert(0.0);
        }
    }

    fn observe(&self, metric: &str, value: f64) {
        if let Ok(mut gauges) = self.gauges.lock() {
            gauges.insert(metric.to_string(), value);
        }
    }

    fn observe_histogram(&self, metric: &str, value: f64) {
        const MAX_SAMPLES: usize = 2000;

        if let Ok(mut histograms) = self.histograms.lock() {
            let values = histograms.entry(metric.to_string()).or_default();
            values.push(value);
            if values.len() > MAX_SAMPLES {
                let overflow = values.len() - MAX_SAMPLES;
                values.drain(0..overflow);
            }
        }
    }

    fn register_histogram(&self, metric: &str) {
        if let Ok(mut histograms) = self.histograms.lock() {
            histograms.entry(metric.to_string()).or_default();
        }
    }

    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn get_counter(&self, metric: &str) -> u64 {
        self.counters
            .lock()
            .ok()
            .and_then(|counters| counters.get(metric).copied())
            .unwrap_or(0)
    }

    pub fn get_gauge(&self, metric: &str) -> f64 {
        self.gauges
            .lock()
            .ok()
            .and_then(|gauges| gauges.get(metric).copied())
            .unwrap_or(0.0)
    }

    fn add_counter(&self, metric: &str, value: u64) {
        if let Ok(mut counters) = self.counters.lock() {
            *counters.entry(metric.to_string()).or_insert(0) += value;
        }
    }

    /// Экспорт всех метрик (для health endpoints)
    pub fn export_metrics(&self) -> serde_json::Value {
        let counters = self
            .counters
            .lock()
            .map(|c| c.clone())
            .unwrap_or_else(|_| HashMap::new());
        let gauges = self
            .gauges
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| HashMap::new());
        let histograms = self
            .histograms
            .lock()
            .map(|h| h.clone())
            .unwrap_or_else(|_| HashMap::new());

        let mut histogram_stats = HashMap::new();
        for (name, mut values) in histograms {
            if values.is_empty() {
                histogram_stats.insert(
                    name,
                    json!({
                        "count": 0,
                        "p50": 0.0,
                        "p95": 0.0,
                        "p99": 0.0
                    }),
                );
                continue;
            }
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let count = values.len();
            let p50 = percentile_sorted(&values, 0.50);
            let p95 = percentile_sorted(&values, 0.95);
            let p99 = percentile_sorted(&values, 0.99);
            histogram_stats.insert(
                name,
                json!({
                    "count": count,
                    "p50": p50,
                    "p95": p95,
                    "p99": p99
                }),
            );
        }

        let mut rates = HashMap::new();
        if let Some(rate) =
            compute_rate(&counters, "completion_incomplete_total", "completion_total")
        {
            rates.insert("completion_incomplete_rate".to_string(), rate);
        }
        if let Some(rate) = compute_rate(&counters, "completion_error_total", "completion_total") {
            rates.insert("completion_error_rate".to_string(), rate);
        }
        if let Some(rate) = compute_rate(
            &counters,
            "signature_help_empty_total",
            "signature_help_total",
        ) {
            rates.insert("signature_help_empty_rate".to_string(), rate);
        }
        let parse_result_leader = sum_counters_with_all_substrings(
            &counters,
            &[
                "intellisense_v2_drilldown_singleflight_effectiveness_total",
                "_outcome_leader_",
                "_query_kind_parse_result",
            ],
        );
        let parse_result_shared = sum_counters_with_all_substrings(
            &counters,
            &[
                "intellisense_v2_drilldown_singleflight_effectiveness_total",
                "_outcome_shared_",
                "_query_kind_parse_result",
            ],
        );
        let parse_result_singleflight_total = parse_result_leader + parse_result_shared;
        if parse_result_singleflight_total > 0 {
            rates.insert(
                "intellisense_v2_parse_result_singleflight_shared_rate".to_string(),
                parse_result_shared as f64 / parse_result_singleflight_total as f64,
            );
        }
        let parse_result_cancelled = sum_counters_with_all_substrings(
            &counters,
            &[
                "intellisense_v2_drilldown_stage_reason_total",
                "_stage_parse_result_query_",
                "_reason_other",
            ],
        );
        if let Some(parse_total) = counters.get("intellisense_v2_parse_result_query_total") {
            if *parse_total > 0 {
                rates.insert(
                    "intellisense_v2_parse_result_query_cancel_rate".to_string(),
                    parse_result_cancelled as f64 / *parse_total as f64,
                );
            }
        }

        json!({
            "counters": counters,
            "gauges": gauges,
            "histograms": histogram_stats,
            "rates": rates,
            "uptime_seconds": self.uptime().as_secs()
        })
    }
}

fn contains_allowed(allowed: &[&str], value: &str) -> bool {
    allowed.contains(&value)
}

fn sanitize_identifier(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn normalize_operation_label(kind: &str) -> &'static str {
    match kind {
        "completion" => "completion",
        "hover" => "hover",
        "signature_help" => "signature_help",
        "definition" => "definition",
        "document_symbol" => "document_symbol",
        "rename" => "rename",
        "diagnostics" => "diagnostics",
        "members" => "members",
        "type_at_position" => "type_at_position",
        "symbol_search" => "symbol_search",
        "references" => "references",
        _ => "other",
    }
}

fn normalize_runtime_stage_kind(kind: &str) -> &'static str {
    match kind {
        "wait_for_file_version" => "wait_for_file_version",
        "snapshot_with_deps" => "snapshot_with_deps",
        _ => "other",
    }
}

fn normalize_query_kind_label(kind: &str) -> &'static str {
    match kind {
        "parse_result" => "parse_result",
        "syntax_diagnostics" => "syntax_diagnostics",
        "ir" => "ir",
        _ => "other",
    }
}

fn normalize_reason_label(kind: &str) -> &'static str {
    match kind {
        "syntax" => "syntax",
        "semantic" => "semantic",
        _ => "other",
    }
}

fn normalize_work_class_label(class: &str) -> &'static str {
    match class {
        "background" => "background",
        _ => "interactive",
    }
}

fn normalize_observability_origin_label(origin: &str) -> &'static str {
    match origin {
        "lsp" => "lsp",
        "web" => "web",
        "agent" => "agent",
        "runtime" => "runtime",
        _ => "runtime",
    }
}

fn normalize_diagnostics_trigger_label(trigger: &str) -> &'static str {
    match trigger {
        "did_change" => "did_change",
        "did_open" => "did_open",
        "did_save" => "did_save",
        "idle" => "idle",
        "documents_set" => "documents_set",
        "job_start" => "job_start",
        _ => "idle",
    }
}

fn normalize_diagnostics_profile_label(profile: &str) -> &'static str {
    match profile {
        "fast" => "fast",
        "debounced_full" => "debounced_full",
        "idle_heavy" => "idle_heavy",
        _ => "debounced_full",
    }
}

fn normalize_diagnostics_reason_label(reason: &str) -> &'static str {
    match reason {
        "published" => "published",
        "superseded_version" => "superseded_version",
        "superseded_generation" => "superseded_generation",
        "client_cancel" => "client_cancel",
        "other_cancel" | "cancelled" => "other_cancel",
        _ => "other_cancel",
    }
}

fn diagnostics_reason_is_cancellation(reason: &str) -> bool {
    matches!(
        reason,
        "superseded_version" | "superseded_generation" | "client_cancel" | "other_cancel"
    )
}

fn normalize_large_churn_state_label(state: &str) -> &'static str {
    match state {
        "enter" => "enter",
        "exit" => "exit",
        _ => "enter",
    }
}

fn normalize_heavy_deferred_reason_label(reason: &str) -> &'static str {
    match reason {
        "large_and_churn" => "large_and_churn",
        _ => "other",
    }
}

fn normalize_parse_snapshot_mode_label(mode: &str) -> &'static str {
    match mode {
        "incremental" => "incremental",
        "reused" => "reused",
        "full" => "full",
        _ => "other",
    }
}

fn normalize_parse_snapshot_fallback_reason_label(reason: &str) -> &'static str {
    if reason.starts_with("incremental_failed:") {
        return "incremental_failed";
    }
    match reason {
        "no_previous_tree" => "no_previous_tree",
        "no_edits_provided" => "no_edits_provided",
        _ => "other",
    }
}

fn normalize_completion_trigger_mode_label(mode: &str) -> &'static str {
    match mode {
        "trigger_character" => "trigger_character",
        "invoked" => "invoked",
        "trigger_for_incomplete" => "trigger_for_incomplete",
        "none" => "none",
        _ => "other",
    }
}

fn normalize_completion_parity_overlap_bucket_label(bucket: &str) -> &'static str {
    match bucket {
        "none" => "none",
        "low" => "low",
        "high" => "high",
        _ => "other",
    }
}

fn normalize_completion_terminal_reason_label(reason: &str) -> &'static str {
    match reason {
        "ok_empty" => "ok_empty",
        "fallback_unavailable" => "fallback_unavailable",
        "missing_ir" => "missing_ir",
        "wait_not_ready" => "wait_not_ready",
        _ => "other",
    }
}

fn normalize_completion_observability_mode_label(mode: &str) -> &'static str {
    match mode {
        "legacy" => "legacy",
        "event_driven" => "event_driven",
        "shadow" => "shadow",
        _ => "legacy",
    }
}

fn normalize_payload_shape_stage_label(stage: &str) -> &'static str {
    match stage {
        "runtime_snapshot_with_deps" => "runtime_snapshot_with_deps",
        "runtime_wait_for_file_version" => "runtime_wait_for_file_version",
        "syntax_diagnostics_query" => "syntax_diagnostics_query",
        "semantic_diagnostics_query" => "semantic_diagnostics_query",
        "parse_result_query" => "parse_result_query",
        _ => "other",
    }
}

fn payload_size_bucket(file_bytes: usize) -> &'static str {
    match file_bytes {
        0 => "zero",
        1..=4095 => "lt_4k",
        4096..=16383 => "lt_16k",
        16384..=65535 => "lt_64k",
        _ => "ge_64k",
    }
}

fn payload_line_bucket(line_count: usize) -> &'static str {
    match line_count {
        0 => "zero",
        1..=99 => "lt_100",
        100..=499 => "lt_500",
        500..=1999 => "lt_2k",
        _ => "ge_2k",
    }
}

fn legacy_wait_for_file_version_metrics(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "completion" => (
            "intellisense_v2_wait_for_file_version_completion_total",
            "intellisense_v2_wait_for_file_version_completion_ms",
        ),
        "hover" => (
            "intellisense_v2_wait_for_file_version_hover_total",
            "intellisense_v2_wait_for_file_version_hover_ms",
        ),
        "signature_help" => (
            "intellisense_v2_wait_for_file_version_signature_help_total",
            "intellisense_v2_wait_for_file_version_signature_help_ms",
        ),
        "diagnostics" => (
            "intellisense_v2_wait_for_file_version_diagnostics_total",
            "intellisense_v2_wait_for_file_version_diagnostics_ms",
        ),
        _ => (
            "intellisense_v2_wait_for_file_version_other_total",
            "intellisense_v2_wait_for_file_version_other_ms",
        ),
    }
}

fn legacy_snapshot_metrics(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "completion" => (
            "intellisense_v2_snapshot_completion_total",
            "intellisense_v2_snapshot_completion_ms",
        ),
        "hover" => (
            "intellisense_v2_snapshot_hover_total",
            "intellisense_v2_snapshot_hover_ms",
        ),
        "signature_help" => (
            "intellisense_v2_snapshot_signature_help_total",
            "intellisense_v2_snapshot_signature_help_ms",
        ),
        "diagnostics" => (
            "intellisense_v2_snapshot_diagnostics_total",
            "intellisense_v2_snapshot_diagnostics_ms",
        ),
        _ => (
            "intellisense_v2_snapshot_other_total",
            "intellisense_v2_snapshot_other_ms",
        ),
    }
}

fn legacy_runtime_queue_wait_metrics(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "snapshot_with_deps" => (
            "intellisense_v2_runtime_snapshot_with_deps_queue_wait_total",
            "intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms",
        ),
        "wait_for_file_version" => (
            "intellisense_v2_runtime_wait_for_file_version_queue_wait_total",
            "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms",
        ),
        _ => (
            "intellisense_v2_runtime_other_queue_wait_total",
            "intellisense_v2_runtime_other_queue_wait_ms",
        ),
    }
}

fn legacy_runtime_exec_metrics(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "snapshot_with_deps" => (
            "intellisense_v2_runtime_snapshot_with_deps_exec_total",
            "intellisense_v2_runtime_snapshot_with_deps_exec_ms",
        ),
        "wait_for_file_version" => (
            "intellisense_v2_runtime_wait_for_file_version_exec_total",
            "intellisense_v2_runtime_wait_for_file_version_exec_ms",
        ),
        _ => (
            "intellisense_v2_runtime_other_exec_total",
            "intellisense_v2_runtime_other_exec_ms",
        ),
    }
}

fn legacy_ir_query_metrics(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "completion" => (
            "intellisense_v2_ir_query_completion_total",
            "intellisense_v2_ir_query_completion_ms",
        ),
        "hover" => (
            "intellisense_v2_ir_query_hover_total",
            "intellisense_v2_ir_query_hover_ms",
        ),
        _ => (
            "intellisense_v2_ir_query_other_total",
            "intellisense_v2_ir_query_other_ms",
        ),
    }
}

fn legacy_ir_query_cancelled_metric(kind: &str) -> &'static str {
    match kind {
        "completion" => "intellisense_v2_ir_query_cancelled_total_completion",
        "hover" => "intellisense_v2_ir_query_cancelled_total_hover",
        _ => "intellisense_v2_ir_query_cancelled_total_other",
    }
}

fn compute_rate(
    counters: &HashMap<String, u64>,
    numerator: &str,
    denominator: &str,
) -> Option<f64> {
    let numerator = *counters.get(numerator)? as f64;
    let denominator = *counters.get(denominator)? as f64;
    if denominator == 0.0 {
        return None;
    }
    Some(numerator / denominator)
}

fn sum_counters_with_all_substrings(counters: &HashMap<String, u64>, parts: &[&str]) -> u64 {
    counters
        .iter()
        .filter(|(key, _)| parts.iter().all(|part| key.contains(part)))
        .map(|(_, value)| *value)
        .sum()
}

fn percentile_sorted(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let clamped = percentile.clamp(0.0, 1.0);
    let rank = ((values.len() - 1) as f64 * clamped).round() as usize;
    values[rank]
}

#[cfg(test)]
mod observability_contract_tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn counters(metrics: &serde_json::Value) -> &serde_json::Map<String, serde_json::Value> {
        metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object")
    }

    fn gauges(metrics: &serde_json::Value) -> &serde_json::Map<String, serde_json::Value> {
        metrics
            .get("gauges")
            .and_then(|value| value.as_object())
            .expect("metrics.gauges object")
    }

    fn histograms(metrics: &serde_json::Value) -> &serde_json::Map<String, serde_json::Value> {
        metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object")
    }

    fn counter_value(counters: &serde_json::Map<String, serde_json::Value>, key: &str) -> u64 {
        counters
            .get(key)
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
    }

    fn histogram_count(histograms: &serde_json::Map<String, serde_json::Value>, key: &str) -> u64 {
        histograms
            .get(key)
            .and_then(|value| value.get("count"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
    }

    fn contract_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("contracts")
            .join(relative)
    }

    fn contract_json(relative: &str) -> serde_json::Value {
        let path = contract_path(relative);
        let raw = std::fs::read_to_string(&path).expect("contract file must be readable");
        serde_json::from_str(&raw).expect("contract file must be valid json")
    }

    #[test]
    fn canonical_wait_stage_projection_matches_legacy_values() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_wait_for_file_version_with_origin(
            "lsp",
            "diagnostics",
            Duration::from_millis(12),
        );

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let histograms = histograms(&exported);

        let legacy_counter_key = "intellisense_v2_wait_for_file_version_diagnostics_total";
        let drilldown_counter_key = "intellisense_v2_drilldown_stage_total_origin_lsp_operation_diagnostics_stage_runtime_wait_for_file_version";
        assert_eq!(
            counter_value(counters, legacy_counter_key),
            counter_value(counters, drilldown_counter_key),
            "legacy and drilldown counters must stay in deterministic projection parity"
        );

        let legacy_histogram_key = "intellisense_v2_wait_for_file_version_diagnostics_ms";
        let drilldown_histogram_key = "intellisense_v2_drilldown_stage_latency_ms_origin_lsp_operation_diagnostics_stage_runtime_wait_for_file_version";
        assert_eq!(
            histogram_count(histograms, legacy_histogram_key),
            histogram_count(histograms, drilldown_histogram_key),
            "legacy and drilldown histograms must have equal sample count"
        );
    }

    #[test]
    fn completion_stage_metrics_include_mode_dimension_and_keep_projection_parity() {
        let observability = BasicObservability::default();

        observability.record_intellisense_v2_wait_for_file_version_with_origin_and_mode(
            "lsp",
            "completion",
            Some("legacy"),
            Duration::from_millis(12),
        );
        observability.record_intellisense_v2_snapshot_latency_with_origin_and_mode(
            "lsp",
            "completion",
            Some("event_driven"),
            Duration::from_millis(17),
        );
        observability.record_intellisense_v2_ir_query_latency_with_origin_and_mode(
            "lsp",
            "completion",
            Some("shadow"),
            Duration::from_millis(23),
        );
        observability
            .record_intellisense_v2_parse_result_query_latency_with_origin_operation_and_mode(
                "lsp",
                "completion",
                Some("event_driven"),
                Duration::from_millis(19),
            );

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let histograms = histograms(&exported);

        let wait_drilldown_counter = "intellisense_v2_drilldown_stage_total_origin_lsp_mode_legacy_operation_completion_stage_runtime_wait_for_file_version";
        let wait_drilldown_histogram = "intellisense_v2_drilldown_stage_latency_ms_origin_lsp_mode_legacy_operation_completion_stage_runtime_wait_for_file_version";
        assert_eq!(counter_value(counters, wait_drilldown_counter), 1);
        assert_eq!(histogram_count(histograms, wait_drilldown_histogram), 1);
        assert_eq!(
            counter_value(
                counters,
                "intellisense_v2_wait_for_file_version_completion_total"
            ),
            counter_value(counters, wait_drilldown_counter),
            "wait stage legacy projection must stay deterministic even with mode dimension"
        );
        assert_eq!(
            histogram_count(
                histograms,
                "intellisense_v2_wait_for_file_version_completion_ms"
            ),
            histogram_count(histograms, wait_drilldown_histogram),
            "wait stage histogram projection must stay deterministic even with mode dimension"
        );

        let snapshot_drilldown_counter = "intellisense_v2_drilldown_stage_total_origin_lsp_mode_event_driven_operation_completion_stage_runtime_snapshot_with_deps";
        let ir_drilldown_counter = "intellisense_v2_drilldown_stage_total_origin_lsp_mode_shadow_operation_completion_stage_ir_query";
        let parse_drilldown_counter = "intellisense_v2_drilldown_stage_total_origin_lsp_mode_event_driven_operation_completion_stage_parse_result_query";
        assert_eq!(counter_value(counters, snapshot_drilldown_counter), 1);
        assert_eq!(counter_value(counters, ir_drilldown_counter), 1);
        assert_eq!(counter_value(counters, parse_drilldown_counter), 1);
    }

    #[test]
    fn completion_mode_dimension_normalizes_unknown_values() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_wait_for_file_version_with_origin_and_mode(
            "lsp",
            "completion",
            Some("unknown-mode"),
            Duration::from_millis(8),
        );

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let normalized_key = "intellisense_v2_drilldown_stage_total_origin_lsp_mode_legacy_operation_completion_stage_runtime_wait_for_file_version";
        assert_eq!(
            counter_value(counters, normalized_key),
            1,
            "unknown completion mode must collapse into bounded mode label set"
        );
        assert!(
            !counters
                .keys()
                .any(|key| key.contains("_mode_unknown-mode")),
            "unexpected mode labels must not leak into drilldown metrics"
        );
    }

    #[test]
    fn invalid_origin_event_is_dropped_with_contract_violation_signal() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_wait_for_file_version_with_origin(
            "invalid-origin",
            "diagnostics",
            Duration::from_millis(5),
        );

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let histograms = histograms(&exported);

        assert!(
            counter_value(
                counters,
                "intellisense_v2_observability_contract_violation_total"
            ) > 0,
            "schema validation must raise contract violation counter"
        );
        assert_eq!(
            counter_value(
                counters,
                "intellisense_v2_wait_for_file_version_diagnostics_total"
            ),
            0,
            "invalid event must not publish legacy projection"
        );
        assert!(
            !counters
                .keys()
                .any(|key| key.contains("origin_invalid-origin")),
            "invalid event must not publish drilldown counter series"
        );
        assert!(
            !histograms
                .keys()
                .any(|key| key.contains("origin_invalid-origin")),
            "invalid event must not publish drilldown histogram series"
        );
    }

    #[test]
    fn missing_projection_mapping_is_reported_and_not_published() {
        let observability = BasicObservability::default();
        observability.emit_canonical_event(
            CanonicalEvent {
                family: CanonicalFamily::StageReasonTotal,
                origin: "lsp",
                mode: None,
                operation: Some("completion"),
                stage: Some("ir_query"),
                outcome: None,
                reason: Some("syntax"),
                query_kind: None,
                work_class: None,
                saturation_metric: None,
                value_kind: CanonicalValueKind::Counter,
                value: 1.0,
                requires_legacy_projection: true,
            },
            None,
        );

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let drilldown_key = "intellisense_v2_drilldown_stage_reason_total_origin_lsp_operation_completion_stage_ir_query_reason_syntax";
        assert!(
            counter_value(counters, "intellisense_v2_projection_missing_total") > 0,
            "missing canonical->legacy mapping must emit projection_missing signal"
        );
        assert_eq!(
            counter_value(counters, drilldown_key),
            0,
            "event without required projection must not be published as metric"
        );
    }

    #[test]
    fn singleflight_projection_is_deterministic_for_query_kind() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_singleflight_leader_with_origin("agent", "ir");
        observability.record_intellisense_v2_singleflight_leader_with_origin("agent", "ir");

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let drilldown_key =
            "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_agent_outcome_leader_query_kind_ir";
        assert_eq!(
            counter_value(counters, "intellisense_v2_singleflight_leader_total"),
            counter_value(counters, drilldown_key),
            "singleflight legacy and drilldown projections must stay equivalent"
        );
    }

    #[test]
    fn saturation_gauge_projection_writes_legacy_and_drilldown() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_runtime_saturation_gauge_with_origin(
            "agent",
            "queue_depth_total",
            3.0,
            "intellisense_v2_runtime_saturation_queue_depth_total",
        );

        let exported = observability.get_metrics().export_metrics();
        let gauges = gauges(&exported);
        assert!(
            gauges.contains_key("intellisense_v2_runtime_saturation_queue_depth_total"),
            "legacy saturation gauge must be present"
        );
        assert!(
            gauges.contains_key(
                "intellisense_v2_drilldown_saturation_gauge_origin_agent_saturation_metric_queue_depth_total"
            ),
            "drilldown saturation gauge must be present"
        );
    }

    #[test]
    fn runtime_queue_and_exec_projection_do_not_raise_hint_mismatch() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_runtime_queue_wait_latency_with_origin(
            "lsp",
            "wait_for_file_version",
            Duration::from_millis(7),
        );
        observability.record_intellisense_v2_runtime_exec_latency_with_origin(
            "lsp",
            "snapshot_with_deps",
            Duration::from_millis(9),
        );

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let histograms = histograms(&exported);

        assert_eq!(
            counter_value(
                counters,
                "intellisense_v2_observability_contract_violation_reason_projection_hint_mismatch"
            ),
            0,
            "runtime queue/exec canonical events must deterministically match legacy projection"
        );
        assert!(
            counter_value(
                counters,
                "intellisense_v2_runtime_wait_for_file_version_queue_wait_total"
            ) > 0,
            "legacy runtime queue wait counter should be projected"
        );
        assert!(
            counter_value(
                counters,
                "intellisense_v2_runtime_snapshot_with_deps_exec_total"
            ) > 0,
            "legacy runtime exec counter should be projected"
        );
        assert!(
            histogram_count(
                histograms,
                "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms"
            ) > 0,
            "legacy runtime queue histogram should be projected"
        );
        assert!(
            histogram_count(
                histograms,
                "intellisense_v2_runtime_snapshot_with_deps_exec_ms"
            ) > 0,
            "legacy runtime exec histogram should be projected"
        );
    }

    #[test]
    fn diagnostics_pipeline_event_exports_low_cardinality_key() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_diagnostics_pipeline_event(
            "agent",
            "documents_set",
            "idle_heavy",
            "superseded_generation",
        );

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let histograms = histograms(&exported);
        let key = "intellisense_v2_diagnostics_pipeline_total_origin_agent_trigger_documents_set_profile_idle_heavy_reason_superseded_generation";
        let histogram_key = "intellisense_v2_diagnostics_pipeline_cancel_sample_origin_agent_trigger_documents_set_profile_idle_heavy_reason_superseded_generation";
        assert_eq!(
            counter_value(counters, key),
            1,
            "diagnostics pipeline counter must include canonical trigger/profile/reason dimensions"
        );
        assert!(
            histogram_count(histograms, histogram_key) > 0,
            "diagnostics pipeline cancel histogram must include normalized reason dimensions"
        );
    }

    #[test]
    fn diagnostics_pipeline_event_normalizes_unknown_dimensions() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_diagnostics_pipeline_event(
            "unknown-origin",
            "unknown-trigger",
            "unknown-profile",
            "unknown-reason",
        );

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let histograms = histograms(&exported);
        let normalized_key = "intellisense_v2_diagnostics_pipeline_total_origin_runtime_trigger_idle_profile_debounced_full_reason_other_cancel";
        let normalized_histogram_key = "intellisense_v2_diagnostics_pipeline_cancel_sample_origin_runtime_trigger_idle_profile_debounced_full_reason_other_cancel";
        assert_eq!(
            counter_value(counters, normalized_key),
            1,
            "invalid labels must collapse into bounded fallback dimensions"
        );
        assert!(
            histogram_count(histograms, normalized_histogram_key) > 0,
            "unknown dimensions must normalize to bounded cancellation histogram labels"
        );
    }

    #[test]
    fn large_churn_transition_metric_is_low_cardinality() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_large_churn_transition("lsp", "enter");
        observability.record_intellisense_v2_large_churn_transition("lsp", "exit");

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        assert_eq!(
            counter_value(
                counters,
                "intellisense_v2_large_churn_state_total_origin_lsp_state_enter"
            ),
            1
        );
        assert_eq!(
            counter_value(
                counters,
                "intellisense_v2_large_churn_state_total_origin_lsp_state_exit"
            ),
            1
        );
    }

    #[test]
    fn heavy_diagnostics_deferred_metric_normalizes_reason_and_profile() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_heavy_diagnostics_deferred(
            "unknown-origin",
            "unknown-profile",
            "unknown-reason",
        );

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let normalized_key = "intellisense_v2_heavy_diagnostics_deferred_total_origin_runtime_profile_debounced_full_reason_other";
        assert_eq!(counter_value(counters, normalized_key), 1);
    }

    #[test]
    fn export_includes_parse_result_singleflight_and_cancel_rates() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_parse_result_query_latency_with_origin(
            "lsp",
            Duration::from_millis(10),
        );
        observability.record_intellisense_v2_query_cancelled_with_origin("lsp", "other");
        observability.record_intellisense_v2_singleflight_leader_with_origin("lsp", "parse_result");
        observability.record_intellisense_v2_singleflight_shared_with_origin("lsp", "parse_result");
        observability
            .record_intellisense_v2_singleflight_leader_with_origin("agent", "parse_result");

        let exported = observability.get_metrics().export_metrics();
        let rates = exported
            .get("rates")
            .and_then(|value| value.as_object())
            .expect("metrics.rates object");

        let shared_rate = rates
            .get("intellisense_v2_parse_result_singleflight_shared_rate")
            .and_then(|value| value.as_f64())
            .expect("parse_result singleflight shared rate must be exported");
        // leaders=2, shared=1
        assert!(
            (shared_rate - (1.0 / 3.0)).abs() < 1e-9,
            "shared rate must be computed from aggregated parse_result singleflight counters"
        );

        let cancel_rate = rates
            .get("intellisense_v2_parse_result_query_cancel_rate")
            .and_then(|value| value.as_f64())
            .expect("parse_result cancel rate must be exported");
        // parse_result total=1, parse_result cancelled=1
        assert!(
            (cancel_rate - 1.0).abs() < 1e-9,
            "parse_result cancel rate must be derived from parse_result stage-reason counters"
        );
    }

    #[test]
    fn parse_result_query_tracks_operation_dimension() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_parse_result_query_latency_with_origin_and_operation(
            "lsp",
            "completion",
            Duration::from_millis(10),
        );

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let histograms = histograms(&exported);

        let stage_counter_key =
            "intellisense_v2_drilldown_stage_total_origin_lsp_operation_completion_stage_parse_result_query";
        let stage_histogram_key =
            "intellisense_v2_drilldown_stage_latency_ms_origin_lsp_operation_completion_stage_parse_result_query";

        assert_eq!(
            counter_value(counters, stage_counter_key),
            1,
            "parse_result stage counter must be attributed to the operation that issued the query"
        );
        assert_eq!(
            histogram_count(histograms, stage_histogram_key),
            1,
            "parse_result stage latency must be attributed to the operation that issued the query"
        );
    }

    #[test]
    fn completion_outcome_exports_degraded_and_fallback_unavailable() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_completion_outcome("degraded_incomplete");
        observability.record_intellisense_v2_completion_outcome("fallback_unavailable");

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        assert_eq!(
            counter_value(
                counters,
                "intellisense_v2_completion_result_total_degraded_incomplete"
            ),
            1,
            "degraded_incomplete outcome must be exported"
        );
        assert_eq!(
            counter_value(
                counters,
                "intellisense_v2_completion_result_total_fallback_unavailable"
            ),
            1,
            "fallback_unavailable outcome must be exported"
        );
    }

    #[test]
    fn completion_trigger_and_terminal_empty_metrics_normalize_labels() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_completion_trigger_mode("unexpected-mode");
        observability.record_intellisense_v2_completion_parity_drift("invoked");
        observability.record_intellisense_v2_completion_parity_overlap_bucket(
            "trigger_character",
            "unexpected-overlap",
        );
        observability.record_intellisense_v2_completion_member_access_terminal_empty(
            "trigger_character",
            "unexpected-reason",
        );

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        assert_eq!(
            counter_value(
                counters,
                "intellisense_v2_completion_trigger_mode_total_mode_other"
            ),
            1,
            "trigger mode must collapse into bounded label set"
        );
        assert_eq!(
            counter_value(
                counters,
                "intellisense_v2_completion_parity_drift_total_mode_invoked"
            ),
            1,
            "parity drift metric must be exported with normalized mode"
        );
        assert_eq!(
            counter_value(
                counters,
                "intellisense_v2_completion_parity_overlap_total_mode_trigger_character_bucket_other"
            ),
            1,
            "parity overlap metric must normalize unknown bucket"
        );
        assert_eq!(
            counter_value(
                counters,
                "intellisense_v2_completion_member_access_terminal_empty_total_mode_trigger_character_reason_other"
            ),
            1,
            "terminal-empty metric must normalize unknown reason"
        );
    }

    #[test]
    fn completion_v1_contract_matches_runtime_outcomes_and_modes() {
        let contract = contract_json("lsp-completion-v2/v1/contract.json");
        let completion = contract
            .get("completion")
            .and_then(|value| value.as_object())
            .expect("completion contract section");

        let trigger_modes: BTreeSet<String> = completion
            .get("trigger_modes")
            .and_then(|value| value.as_array())
            .expect("trigger_modes array")
            .iter()
            .map(|value| value.as_str().expect("trigger mode string").to_string())
            .collect();
        let expected_modes: BTreeSet<String> = [
            "trigger_character",
            "invoked",
            "trigger_for_incomplete",
            "none",
        ]
        .iter()
        .map(|value| value.to_string())
        .collect();
        assert_eq!(
            trigger_modes, expected_modes,
            "contract trigger modes must match bounded runtime label set"
        );
        for mode in &trigger_modes {
            assert_eq!(
                normalize_completion_trigger_mode_label(mode),
                mode,
                "contract mode must be accepted by runtime normalization"
            );
        }

        let outcomes: BTreeSet<String> = completion
            .get("outcomes")
            .and_then(|value| value.as_array())
            .expect("outcomes array")
            .iter()
            .map(|value| value.as_str().expect("outcome string").to_string())
            .collect();
        let expected_outcomes: BTreeSet<String> = [
            "ok_non_empty",
            "ok_empty",
            "degraded_incomplete",
            "fallback_unavailable",
        ]
        .iter()
        .map(|value| value.to_string())
        .collect();
        assert_eq!(
            outcomes, expected_outcomes,
            "contract outcomes must match v1 completion baseline"
        );

        let observability = BasicObservability::default();
        for outcome in &outcomes {
            observability.record_intellisense_v2_completion_outcome(outcome);
        }

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        for (outcome, metric) in [
            (
                "ok_non_empty",
                "intellisense_v2_completion_result_total_ok_non_empty",
            ),
            (
                "ok_empty",
                "intellisense_v2_completion_result_total_ok_empty",
            ),
            (
                "degraded_incomplete",
                "intellisense_v2_completion_result_total_degraded_incomplete",
            ),
            (
                "fallback_unavailable",
                "intellisense_v2_completion_result_total_fallback_unavailable",
            ),
        ] {
            assert!(
                outcomes.contains(outcome),
                "contract must include outcome {outcome}"
            );
            assert!(
                counter_value(counters, metric) > 0,
                "runtime must export contract outcome metric {metric}"
            );
        }
    }

    #[test]
    fn observability_completion_v1_contract_matches_runtime_metric_labels() {
        let contract = contract_json("observability-completion-v2/v1/contract.json");
        let metrics_contract = contract
            .get("metrics")
            .and_then(|value| value.as_object())
            .expect("metrics contract section");

        assert_eq!(
            metrics_contract
                .get("trigger_mode_counter_prefix")
                .and_then(|value| value.as_str())
                .expect("trigger mode prefix"),
            "intellisense_v2_completion_trigger_mode_total_mode_"
        );
        assert_eq!(
            metrics_contract
                .get("parity_drift_counter_prefix")
                .and_then(|value| value.as_str())
                .expect("parity drift prefix"),
            "intellisense_v2_completion_parity_drift_total_mode_"
        );
        assert_eq!(
            metrics_contract
                .get("member_access_terminal_empty_counter_prefix")
                .and_then(|value| value.as_str())
                .expect("terminal empty prefix"),
            "intellisense_v2_completion_member_access_terminal_empty_total_mode_"
        );
        assert_eq!(
            metrics_contract
                .get("fallback_unavailable_counter")
                .and_then(|value| value.as_str())
                .expect("fallback_unavailable counter"),
            "intellisense_v2_completion_result_total_fallback_unavailable"
        );

        let trigger_modes: Vec<String> = metrics_contract
            .get("allowed_trigger_modes")
            .and_then(|value| value.as_array())
            .expect("allowed_trigger_modes")
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .expect("allowed trigger mode string")
                    .to_string()
            })
            .collect();
        let terminal_reasons: Vec<String> = metrics_contract
            .get("allowed_terminal_empty_reasons")
            .and_then(|value| value.as_array())
            .expect("allowed_terminal_empty_reasons")
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .expect("allowed terminal reason string")
                    .to_string()
            })
            .collect();

        let observability = BasicObservability::default();
        for mode in &trigger_modes {
            observability.record_intellisense_v2_completion_trigger_mode(mode);
            observability.record_intellisense_v2_completion_parity_drift(mode);
            assert_eq!(
                normalize_completion_trigger_mode_label(mode),
                mode,
                "contract mode must remain in bounded normalization set"
            );
        }
        for reason in &terminal_reasons {
            observability.record_intellisense_v2_completion_member_access_terminal_empty(
                "trigger_character",
                reason,
            );
            assert_eq!(
                normalize_completion_terminal_reason_label(reason),
                reason,
                "contract terminal reason must remain in bounded normalization set"
            );
        }
        observability.record_intellisense_v2_completion_outcome("fallback_unavailable");

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        for mode in &trigger_modes {
            let trigger_key = format!("intellisense_v2_completion_trigger_mode_total_mode_{mode}");
            let drift_key = format!("intellisense_v2_completion_parity_drift_total_mode_{mode}");
            assert!(
                counter_value(counters, &trigger_key) > 0,
                "trigger-mode counter must be exported for {mode}"
            );
            assert!(
                counter_value(counters, &drift_key) > 0,
                "parity-drift counter must be exported for {mode}"
            );
        }
        for reason in &terminal_reasons {
            let terminal_key = format!(
                "intellisense_v2_completion_member_access_terminal_empty_total_mode_trigger_character_reason_{reason}"
            );
            assert!(
                counter_value(counters, &terminal_key) > 0,
                "terminal-empty counter must be exported for reason {reason}"
            );
        }
        assert!(
            counter_value(
                counters,
                "intellisense_v2_completion_result_total_fallback_unavailable"
            ) > 0,
            "fallback_unavailable counter must be exported"
        );
    }

    #[test]
    fn observability_diagnostics_v1_contract_matches_runtime_metric_labels() {
        let contract = contract_json("observability-diagnostics-v2/v1/contract.json");
        let metrics_contract = contract
            .get("metrics")
            .and_then(|value| value.as_object())
            .expect("metrics contract section");

        let counter_prefix = metrics_contract
            .get("pipeline_counter_prefix")
            .and_then(|value| value.as_str())
            .expect("pipeline counter prefix");
        let histogram_prefix = metrics_contract
            .get("cancellation_histogram_prefix")
            .and_then(|value| value.as_str())
            .expect("cancellation histogram prefix");
        assert_eq!(
            counter_prefix,
            "intellisense_v2_diagnostics_pipeline_total_origin_"
        );
        assert_eq!(
            histogram_prefix,
            "intellisense_v2_diagnostics_pipeline_cancel_sample_origin_"
        );

        let origins: Vec<String> = metrics_contract
            .get("allowed_origins")
            .and_then(|value| value.as_array())
            .expect("allowed_origins")
            .iter()
            .map(|value| value.as_str().expect("allowed origin string").to_string())
            .collect();
        let triggers: Vec<String> = metrics_contract
            .get("allowed_triggers")
            .and_then(|value| value.as_array())
            .expect("allowed_triggers")
            .iter()
            .map(|value| value.as_str().expect("allowed trigger string").to_string())
            .collect();
        let profiles: Vec<String> = metrics_contract
            .get("allowed_profiles")
            .and_then(|value| value.as_array())
            .expect("allowed_profiles")
            .iter()
            .map(|value| value.as_str().expect("allowed profile string").to_string())
            .collect();
        let reasons: Vec<String> = metrics_contract
            .get("allowed_reasons")
            .and_then(|value| value.as_array())
            .expect("allowed_reasons")
            .iter()
            .map(|value| value.as_str().expect("allowed reason string").to_string())
            .collect();
        let cancellation_reasons: Vec<String> = metrics_contract
            .get("allowed_cancellation_reasons")
            .and_then(|value| value.as_array())
            .expect("allowed_cancellation_reasons")
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .expect("allowed cancellation reason string")
                    .to_string()
            })
            .collect();

        let reasons_set: BTreeSet<String> = reasons.iter().cloned().collect();
        let cancellation_reasons_set: BTreeSet<String> =
            cancellation_reasons.iter().cloned().collect();
        assert!(
            !cancellation_reasons_set.is_empty(),
            "contract must define cancellation reasons"
        );
        for reason in &cancellation_reasons_set {
            assert!(
                reasons_set.contains(reason),
                "cancellation reason {reason} must be present in allowed_reasons"
            );
        }

        for origin in &origins {
            assert_eq!(
                normalize_observability_origin_label(origin),
                origin,
                "contract origin must remain in bounded normalization set"
            );
        }
        for trigger in &triggers {
            assert_eq!(
                normalize_diagnostics_trigger_label(trigger),
                trigger,
                "contract trigger must remain in bounded normalization set"
            );
        }
        for profile in &profiles {
            assert_eq!(
                normalize_diagnostics_profile_label(profile),
                profile,
                "contract profile must remain in bounded normalization set"
            );
        }
        for reason in &reasons {
            assert_eq!(
                normalize_diagnostics_reason_label(reason),
                reason,
                "contract reason must remain in bounded normalization set"
            );
            assert_eq!(
                diagnostics_reason_is_cancellation(reason),
                cancellation_reasons_set.contains(reason),
                "contract reason cancellation classification drifted for {reason}"
            );
        }

        let observability = BasicObservability::default();
        let origin = origins
            .iter()
            .find(|origin| origin.as_str() == "lsp")
            .map(String::as_str)
            .unwrap_or(origins[0].as_str());
        let trigger = triggers
            .iter()
            .find(|trigger| trigger.as_str() == "did_change")
            .map(String::as_str)
            .unwrap_or(triggers[0].as_str());
        let profile = profiles
            .iter()
            .find(|profile| profile.as_str() == "debounced_full")
            .map(String::as_str)
            .unwrap_or(profiles[0].as_str());
        for reason in &reasons {
            observability.record_intellisense_v2_diagnostics_pipeline_event(
                origin, trigger, profile, reason,
            );
        }

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let histograms = histograms(&exported);
        for reason in &reasons {
            let counter_key = format!(
                "{counter_prefix}{origin}_trigger_{trigger}_profile_{profile}_reason_{reason}"
            );
            assert!(
                counter_value(counters, &counter_key) > 0,
                "diagnostics pipeline counter must be exported for reason {reason}"
            );

            let histogram_key = format!(
                "{histogram_prefix}{origin}_trigger_{trigger}_profile_{profile}_reason_{reason}"
            );
            if cancellation_reasons_set.contains(reason) {
                assert!(
                    histogram_count(histograms, &histogram_key) > 0,
                    "diagnostics pipeline cancellation histogram must be exported for reason {reason}"
                );
            } else {
                assert_eq!(
                    histogram_count(histograms, &histogram_key),
                    0,
                    "non-cancellation reason {reason} must not emit cancellation histogram sample"
                );
            }
        }
    }

    #[test]
    fn payload_shape_metrics_export_bucket_and_histograms() {
        let observability = BasicObservability::default();
        observability.record_intellisense_v2_payload_shape_with_origin(
            "lsp",
            "completion",
            "runtime_snapshot_with_deps",
            12_345,
            321,
        );

        let exported = observability.get_metrics().export_metrics();
        let counters = counters(&exported);
        let histograms = histograms(&exported);
        let counter_key = "intellisense_v2_payload_shape_total_origin_lsp_operation_completion_stage_runtime_snapshot_with_deps_size_bucket_lt_16k_line_bucket_lt_500";
        let bytes_histogram_key =
            "intellisense_v2_payload_shape_bytes_origin_lsp_operation_completion_stage_runtime_snapshot_with_deps";
        let lines_histogram_key =
            "intellisense_v2_payload_shape_lines_origin_lsp_operation_completion_stage_runtime_snapshot_with_deps";

        assert_eq!(
            counter_value(counters, counter_key),
            1,
            "payload-shape bucket counter should include normalized dimensions"
        );
        assert!(
            histogram_count(histograms, bytes_histogram_key) > 0,
            "payload-shape bytes histogram should be exported"
        );
        assert!(
            histogram_count(histograms, lines_histogram_key) > 0,
            "payload-shape lines histogram should be exported"
        );
    }
}

// === COMPARISON WITH COMPLEX OBSERVABILITY ===

#[cfg(test)]
mod comparison_notes {
    //! Сравнение: Simple vs Complex observability
    //!
    //! Complex (Full Enterprise Stack):
    //! - LoggingManager + MetricsCollector + HealthChecker
    //! - CircuitBreaker + EventBus + AlertingManager  
    //! - Distributed tracing + APM integration
    //! - Advanced dashboards + SLA monitoring
    //! - ~500+ LOC
    //!
    //! Simple (BasicObservability):
    //! - StructuredLogger + SimpleMetrics + HealthEndpoint
    //! - Basic health check
    //! - ~150 LOC
    //!
    //! Экономия: ~70% сложности, покрывает основные потребности monitoring
}
