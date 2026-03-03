//! Dedicated perf-gate evaluator used by runtime/harness checks.
//!
//! The evaluator intentionally keeps verdict logic in one place to avoid
//! drift between LSP runtime checks and external scripts.

use serde_json::Value;
use std::collections::BTreeSet;

fn get_report_metric_f64(report: &Value, path: &[&str]) -> Result<f64, String> {
    let mut cursor = report;
    for segment in path {
        cursor = cursor
            .get(*segment)
            .ok_or_else(|| format!("missing field '{}'", path.join(".")))?;
    }
    cursor
        .as_f64()
        .or_else(|| cursor.as_u64().map(|n| n as f64))
        .ok_or_else(|| format!("field '{}' must be numeric", path.join(".")))
}

pub const PARITY_DRIFT_RATE_MAX_FOR_CUTOVER: f64 = 0.01;
pub const PARITY_PAIRS_TOTAL_MIN_FOR_CUTOVER: u64 = 100;

fn is_valid_change_id(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !trimmed.starts_with('-')
        && !trimmed.ends_with('-')
        && !trimmed.contains("--")
}

pub fn validate_perf_report_provenance(
    report: &Value,
    expected_change_id: Option<&str>,
) -> Result<(), String> {
    let top_level_change_id = match report.get("change_id") {
        Some(value) => value
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "provenance_invalid".to_string())?,
        None => String::new(),
    };

    let provenance_obj = match report.get("provenance") {
        Some(value) => Some(
            value
                .as_object()
                .ok_or_else(|| "provenance_invalid".to_string())?,
        ),
        None => None,
    };

    let provenance_change_id = match provenance_obj.and_then(|obj| obj.get("change_id")) {
        Some(value) => value
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "provenance_invalid".to_string())?,
        None => String::new(),
    };
    let effective_change_id = if !provenance_change_id.is_empty() {
        if !top_level_change_id.is_empty() && top_level_change_id != provenance_change_id {
            return Err("provenance_invalid".to_string());
        }
        provenance_change_id
    } else {
        top_level_change_id
    };

    if !effective_change_id.is_empty() && !is_valid_change_id(&effective_change_id) {
        return Err("provenance_invalid".to_string());
    }

    if let Some(expected_change_id) = expected_change_id {
        if !is_valid_change_id(expected_change_id) {
            return Err("provenance_invalid".to_string());
        }
        if effective_change_id.is_empty() {
            return Err("provenance_missing_for_authoritative_run".to_string());
        }
        if effective_change_id != expected_change_id {
            return Err("provenance_mismatch_expected_change_id".to_string());
        }
    }

    for field in ["generated_at", "profile", "contract_version"] {
        if let Some(value) = provenance_obj.and_then(|obj| obj.get(field)) {
            if value.as_str().is_none_or(|string| string.trim().is_empty()) {
                return Err("provenance_invalid".to_string());
            }
        }
    }
    if let Some(value) = provenance_obj.and_then(|obj| obj.get("schema_version")) {
        if value.as_u64().is_none_or(|number| number == 0) {
            return Err("provenance_invalid".to_string());
        }
    }

    Ok(())
}

pub fn validate_parity_cutover_evidence(report: &Value) -> Result<(), String> {
    let parity_pairs_total = get_report_u64(report, &["results", "parity_pairs_total"])
        .map_err(|_| "parity_evidence_insufficient".to_string())?;
    if parity_pairs_total < PARITY_PAIRS_TOTAL_MIN_FOR_CUTOVER {
        return Err("parity_evidence_insufficient".to_string());
    }

    let parity_drift_rate = get_report_metric_f64(report, &["results", "parity_mismatch_rate"])
        .or_else(|_| get_report_metric_f64(report, &["results", "parity_drift_rate"]))
        .map_err(|_| "parity_evidence_insufficient".to_string())?;
    if !parity_drift_rate.is_finite() {
        return Err("parity_evidence_insufficient".to_string());
    }
    if parity_drift_rate > PARITY_DRIFT_RATE_MAX_FOR_CUTOVER {
        return Err("parity_drift_threshold_exceeded".to_string());
    }

    Ok(())
}

pub fn get_report_u64(report: &Value, path: &[&str]) -> Result<u64, String> {
    let mut cursor = report;
    for segment in path {
        cursor = cursor
            .get(*segment)
            .ok_or_else(|| format!("missing field '{}'", path.join(".")))?;
    }
    cursor
        .as_u64()
        .ok_or_else(|| format!("field '{}' must be u64", path.join(".")))
}

fn get_report_bool(report: &Value, path: &[&str]) -> Result<bool, String> {
    let mut cursor = report;
    for segment in path {
        cursor = cursor
            .get(*segment)
            .ok_or_else(|| format!("missing field '{}'", path.join(".")))?;
    }
    cursor
        .as_bool()
        .ok_or_else(|| format!("field '{}' must be bool", path.join(".")))
}

pub fn validate_scale_aware_baseline_schema(baseline_report: &Value) -> Result<(), String> {
    const PROFILES: &[&str] = &["large", "small"];
    const PHASES: &[&str] = &["start", "cold", "warm"];
    const REQUIRED_METRICS: &[&str] = &[
        "completion_duration_ms",
        "intellisense_v2_wait_for_file_version_completion_ms",
        "intellisense_v2_snapshot_completion_ms",
        "intellisense_v2_ir_query_completion_ms",
    ];

    let schema_version = get_report_u64(baseline_report, &["schema_version"])?;
    if schema_version == 0 {
        return Err("field 'schema_version' must be >= 1".to_string());
    }
    get_report_bool(baseline_report, &["gate", "pass"])?;

    for profile in PROFILES {
        for phase in PHASES {
            for metric in REQUIRED_METRICS {
                get_report_u64(
                    baseline_report,
                    &["profiles", profile, phase, "metrics", metric, "count"],
                )?;
                get_report_metric_f64(
                    baseline_report,
                    &["profiles", profile, phase, "metrics", metric, "p95"],
                )?;
            }
        }
    }

    Ok(())
}

pub fn evaluate_scale_aware_gate(
    current_report: &Value,
    baseline_report: &Value,
) -> Result<Value, String> {
    const LARGE_WAIT_RATIO_MAX: f64 = 0.60;
    const LARGE_COMPLETION_RATIO_MAX: f64 = 0.75;
    const SMALL_COMPLETION_RATIO_MAX: f64 = 1.25;
    const MAX_CANCELLED_RATE: f64 = 0.10;
    const MIN_COMPLETION_TOTAL: u64 = 50;
    const MAX_STALE_FALLBACK_UNAVAILABLE_PER_BUDGET_EXHAUSTED: f64 = 0.20;

    let large_current_wait = get_report_metric_f64(
        current_report,
        &[
            "profiles",
            "large",
            "warm",
            "metrics",
            "intellisense_v2_wait_for_file_version_completion_ms",
            "p95",
        ],
    )?;
    let large_current_completion = get_report_metric_f64(
        current_report,
        &[
            "profiles",
            "large",
            "warm",
            "metrics",
            "completion_duration_ms",
            "p95",
        ],
    )?;
    let small_current_completion = get_report_metric_f64(
        current_report,
        &[
            "profiles",
            "small",
            "warm",
            "metrics",
            "completion_duration_ms",
            "p95",
        ],
    )?;

    let large_baseline_wait = get_report_metric_f64(
        baseline_report,
        &[
            "profiles",
            "large",
            "warm",
            "metrics",
            "intellisense_v2_wait_for_file_version_completion_ms",
            "p95",
        ],
    )?;
    let large_baseline_completion = get_report_metric_f64(
        baseline_report,
        &[
            "profiles",
            "large",
            "warm",
            "metrics",
            "completion_duration_ms",
            "p95",
        ],
    )?;
    let small_baseline_completion = get_report_metric_f64(
        baseline_report,
        &[
            "profiles",
            "small",
            "warm",
            "metrics",
            "completion_duration_ms",
            "p95",
        ],
    )?;

    let large_completion_total = get_report_u64(
        current_report,
        &["profiles", "large", "warm", "completion_total"],
    )?;
    let small_completion_total = get_report_u64(
        current_report,
        &["profiles", "small", "warm", "completion_total"],
    )?;
    let large_cancelled_total = get_report_u64(
        current_report,
        &["profiles", "large", "warm", "completion_cancelled_total"],
    )?;
    let small_cancelled_total = get_report_u64(
        current_report,
        &["profiles", "small", "warm", "completion_cancelled_total"],
    )?;

    let large_wait_ratio = large_current_wait / large_baseline_wait.max(0.000_001);
    let large_completion_ratio =
        large_current_completion / large_baseline_completion.max(0.000_001);
    let small_completion_ratio =
        small_current_completion / small_baseline_completion.max(0.000_001);

    let large_cancelled_rate = large_cancelled_total as f64 / large_completion_total.max(1) as f64;
    let small_cancelled_rate = small_cancelled_total as f64 / small_completion_total.max(1) as f64;

    let phase_counter = |profile: &str, phase: &str, metric: &str| -> Result<u64, String> {
        get_report_u64(
            current_report,
            &["profiles", profile, phase, "metrics", metric],
        )
    };

    let large_start_stale_fallback_total = phase_counter(
        "large",
        "start",
        "intellisense_v2_completion_stale_fallback_total",
    )?;
    let large_cold_stale_fallback_total = phase_counter(
        "large",
        "cold",
        "intellisense_v2_completion_stale_fallback_total",
    )?;
    let large_warm_stale_fallback_total = phase_counter(
        "large",
        "warm",
        "intellisense_v2_completion_stale_fallback_total",
    )?;
    let small_start_stale_fallback_total = phase_counter(
        "small",
        "start",
        "intellisense_v2_completion_stale_fallback_total",
    )?;
    let small_cold_stale_fallback_total = phase_counter(
        "small",
        "cold",
        "intellisense_v2_completion_stale_fallback_total",
    )?;
    let small_warm_stale_fallback_total = phase_counter(
        "small",
        "warm",
        "intellisense_v2_completion_stale_fallback_total",
    )?;

    let large_warm_budget_exhausted_total = phase_counter(
        "large",
        "warm",
        "intellisense_v2_interactive_wait_budget_exhausted_total",
    )?;
    let small_warm_budget_exhausted_total = phase_counter(
        "small",
        "warm",
        "intellisense_v2_interactive_wait_budget_exhausted_total",
    )?;
    let large_warm_fallback_unavailable_total = phase_counter(
        "large",
        "warm",
        "intellisense_v2_completion_fallback_unavailable_total",
    )?;
    let small_warm_fallback_unavailable_total = phase_counter(
        "small",
        "warm",
        "intellisense_v2_completion_fallback_unavailable_total",
    )?;
    let large_warm_stale_served_total = phase_counter(
        "large",
        "warm",
        "intellisense_v2_interactive_stale_served_total",
    )?;
    let small_warm_stale_served_total = phase_counter(
        "small",
        "warm",
        "intellisense_v2_interactive_stale_served_total",
    )?;

    let large_warm_fallback_unavailable_rate = large_warm_fallback_unavailable_total as f64
        / large_warm_budget_exhausted_total.max(1) as f64;
    let small_warm_fallback_unavailable_rate = small_warm_fallback_unavailable_total as f64
        / small_warm_budget_exhausted_total.max(1) as f64;
    let stale_fastpath_exercised =
        large_warm_stale_fallback_total > 0 || small_warm_stale_fallback_total > 0;
    let stale_fastpath_pass = !stale_fastpath_exercised
        || (large_warm_fallback_unavailable_rate
            <= MAX_STALE_FALLBACK_UNAVAILABLE_PER_BUDGET_EXHAUSTED
            && small_warm_fallback_unavailable_rate
                <= MAX_STALE_FALLBACK_UNAVAILABLE_PER_BUDGET_EXHAUSTED);
    let large_warm_dominant_stage = current_report
        .get("profiles")
        .and_then(|value| value.get("large"))
        .and_then(|value| value.get("warm"))
        .and_then(|value| value.get("dominant_stage"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({"stage": "unknown", "p95_ms": 0.0}));
    let small_warm_dominant_stage = current_report
        .get("profiles")
        .and_then(|value| value.get("small"))
        .and_then(|value| value.get("warm"))
        .and_then(|value| value.get("dominant_stage"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({"stage": "unknown", "p95_ms": 0.0}));

    let pass = large_wait_ratio <= LARGE_WAIT_RATIO_MAX
        && large_completion_ratio <= LARGE_COMPLETION_RATIO_MAX
        && small_completion_ratio <= SMALL_COMPLETION_RATIO_MAX
        && large_cancelled_rate <= MAX_CANCELLED_RATE
        && small_cancelled_rate <= MAX_CANCELLED_RATE
        && large_completion_total >= MIN_COMPLETION_TOTAL
        && small_completion_total >= MIN_COMPLETION_TOTAL
        && stale_fastpath_pass;

    Ok(serde_json::json!({
        "pass": pass,
        "ratios": {
            "large_wait_ratio": large_wait_ratio,
            "large_completion_ratio": large_completion_ratio,
            "small_completion_ratio": small_completion_ratio
        },
        "rates": {
            "large_completion_cancelled_rate": large_cancelled_rate,
            "small_completion_cancelled_rate": small_cancelled_rate
        },
        "dominant_stage": {
            "large_warm": large_warm_dominant_stage,
            "small_warm": small_warm_dominant_stage
        },
        "stale_fastpath": {
            "evaluated": stale_fastpath_exercised,
            "pass": stale_fastpath_pass,
            "counts": {
                "large": {
                    "start_stale_fallback_total": large_start_stale_fallback_total,
                    "cold_stale_fallback_total": large_cold_stale_fallback_total,
                    "warm_stale_fallback_total": large_warm_stale_fallback_total,
                    "warm_budget_exhausted_total": large_warm_budget_exhausted_total,
                    "warm_stale_served_total": large_warm_stale_served_total,
                    "warm_fallback_unavailable_total": large_warm_fallback_unavailable_total
                },
                "small": {
                    "start_stale_fallback_total": small_start_stale_fallback_total,
                    "cold_stale_fallback_total": small_cold_stale_fallback_total,
                    "warm_stale_fallback_total": small_warm_stale_fallback_total,
                    "warm_budget_exhausted_total": small_warm_budget_exhausted_total,
                    "warm_stale_served_total": small_warm_stale_served_total,
                    "warm_fallback_unavailable_total": small_warm_fallback_unavailable_total
                }
            },
            "rates": {
                "large_warm_fallback_unavailable_per_budget_exhausted": large_warm_fallback_unavailable_rate,
                "small_warm_fallback_unavailable_per_budget_exhausted": small_warm_fallback_unavailable_rate
            }
        },
        "counts": {
            "large_completion_total": large_completion_total,
            "small_completion_total": small_completion_total
        },
        "thresholds": {
            "large_wait_ratio_max": LARGE_WAIT_RATIO_MAX,
            "large_completion_ratio_max": LARGE_COMPLETION_RATIO_MAX,
            "small_completion_ratio_max": SMALL_COMPLETION_RATIO_MAX,
            "completion_cancelled_rate_max": MAX_CANCELLED_RATE,
            "min_completion_total": MIN_COMPLETION_TOTAL,
            "stale_fallback_unavailable_per_budget_exhausted_max": MAX_STALE_FALLBACK_UNAVAILABLE_PER_BUDGET_EXHAUSTED
        }
    }))
}

#[derive(Debug, Clone, Copy)]
pub struct PerfGateSample {
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub error_rate: f64,
    pub incomplete_rate: f64,
    pub allocations_per_completion: f64,
    pub allocated_bytes_per_completion: f64,
    pub lock_wait_ms_per_completion: f64,
    pub lock_contention_events_per_completion: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct PerfGateThresholds {
    pub latency_ratio_p95_max: f64,
    pub latency_ratio_p99_max: f64,
    pub resource_ratio_max: f64,
    pub max_error_rate: f64,
    pub max_incomplete_rate: f64,
    pub blocking_mode: bool,
}

fn read_contract_u64(contract: &Value, path: &[&str]) -> Option<u64> {
    let mut cursor = contract;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_u64()
}

fn read_contract_string_vec(contract: &Value, path: &[&str]) -> Option<Vec<String>> {
    let mut cursor = contract;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    let items = cursor.as_array()?;
    Some(
        items
            .iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
            .collect(),
    )
}

pub fn evaluate_intellisense_perf_profile(
    contract: &Value,
    profile: &str,
    current: PerfGateSample,
    baseline: Option<PerfGateSample>,
    thresholds: PerfGateThresholds,
) -> Value {
    let mut reason_codes = BTreeSet::new();

    let contract_version = match (
        contract.get("surface").and_then(|v| v.as_str()),
        contract.get("major_version").and_then(|v| v.as_u64()),
    ) {
        (Some("intellisense-perf-gate"), Some(1)) => "v1".to_string(),
        _ => {
            reason_codes.insert("unsupported_contract_version".to_string());
            "unknown".to_string()
        }
    };

    let required_profiles =
        read_contract_string_vec(contract, &["input", "required_profiles"]).unwrap_or_default();
    if !required_profiles.iter().any(|item| item == profile) {
        reason_codes.insert("missing_required_metric_field".to_string());
    }

    if !current.allocations_per_completion.is_finite()
        || !current.allocated_bytes_per_completion.is_finite()
        || !current.lock_wait_ms_per_completion.is_finite()
        || !current.lock_contention_events_per_completion.is_finite()
    {
        reason_codes.insert("missing_required_metric_field".to_string());
    }

    if current.error_rate > thresholds.max_error_rate
        || current.incomplete_rate > thresholds.max_incomplete_rate
    {
        reason_codes.insert("missing_required_metric_field".to_string());
    }

    let ceiling_p95 = read_contract_u64(
        contract,
        &["baseline", "absolute_latency_ceilings_ms", profile, "p95"],
    )
    .map(|v| v as f64);
    let ceiling_p99 = read_contract_u64(
        contract,
        &["baseline", "absolute_latency_ceilings_ms", profile, "p99"],
    )
    .map(|v| v as f64);

    if let Some(p95) = ceiling_p95 {
        if current.p95_ms > p95 {
            reason_codes.insert("latency_absolute_ceiling_exceeded".to_string());
        }
    } else if thresholds.blocking_mode {
        reason_codes.insert("initial_budget_not_fixed".to_string());
    }
    if let Some(p99) = ceiling_p99 {
        if current.p99_ms > p99 {
            reason_codes.insert("latency_absolute_ceiling_exceeded".to_string());
        }
    } else if thresholds.blocking_mode {
        reason_codes.insert("initial_budget_not_fixed".to_string());
    }

    let budget_allocations = read_contract_u64(
        contract,
        &[
            "baseline",
            "resource_budget_ceilings",
            profile,
            "allocations_per_completion",
        ],
    )
    .map(|v| v as f64);
    let budget_allocated_bytes = read_contract_u64(
        contract,
        &[
            "baseline",
            "resource_budget_ceilings",
            profile,
            "allocated_bytes_per_completion",
        ],
    )
    .map(|v| v as f64);
    let budget_lock_wait = read_contract_u64(
        contract,
        &[
            "baseline",
            "resource_budget_ceilings",
            profile,
            "lock_wait_ms_per_completion",
        ],
    )
    .map(|v| v as f64);
    let budget_lock_contention = read_contract_u64(
        contract,
        &[
            "baseline",
            "resource_budget_ceilings",
            profile,
            "lock_contention_events_per_completion",
        ],
    )
    .map(|v| v as f64);

    if let Some(max_allocations) = budget_allocations {
        if current.allocations_per_completion > max_allocations {
            reason_codes.insert("allocation_budget_exceeded".to_string());
        }
    } else if thresholds.blocking_mode {
        reason_codes.insert("initial_budget_not_fixed".to_string());
    }
    if let Some(max_allocated_bytes) = budget_allocated_bytes {
        if current.allocated_bytes_per_completion > max_allocated_bytes {
            reason_codes.insert("allocation_budget_exceeded".to_string());
        }
    } else if thresholds.blocking_mode {
        reason_codes.insert("initial_budget_not_fixed".to_string());
    }
    if let Some(max_lock_wait_ms) = budget_lock_wait {
        if current.lock_wait_ms_per_completion > max_lock_wait_ms {
            reason_codes.insert("lock_wait_budget_exceeded".to_string());
        }
    } else if thresholds.blocking_mode {
        reason_codes.insert("initial_budget_not_fixed".to_string());
    }
    if let Some(max_lock_contention_events) = budget_lock_contention {
        if current.lock_contention_events_per_completion > max_lock_contention_events {
            reason_codes.insert("lock_contention_budget_exceeded".to_string());
        }
    } else if thresholds.blocking_mode {
        reason_codes.insert("initial_budget_not_fixed".to_string());
    }

    let mut ratio_p95 = None;
    let mut ratio_p99 = None;
    if let Some(base) = baseline {
        if base.p95_ms > 0.0 && base.p99_ms > 0.0 {
            let p95_ratio = current.p95_ms / base.p95_ms.max(0.000_001);
            let p99_ratio = current.p99_ms / base.p99_ms.max(0.000_001);
            ratio_p95 = Some(p95_ratio);
            ratio_p99 = Some(p99_ratio);
            if p95_ratio > thresholds.latency_ratio_p95_max
                || p99_ratio > thresholds.latency_ratio_p99_max
            {
                reason_codes.insert("latency_relative_ratio_exceeded".to_string());
            }
        } else if thresholds.blocking_mode {
            reason_codes.insert("initial_budget_not_fixed".to_string());
        }

        let has_resource_baseline = base.allocations_per_completion > 0.0
            && base.allocated_bytes_per_completion > 0.0
            && base.lock_wait_ms_per_completion > 0.0
            && base.lock_contention_events_per_completion > 0.0;
        if has_resource_baseline {
            let allocation_ratio =
                current.allocations_per_completion / base.allocations_per_completion.max(0.000_001);
            let allocated_bytes_ratio = current.allocated_bytes_per_completion
                / base.allocated_bytes_per_completion.max(0.000_001);
            let lock_wait_ratio = current.lock_wait_ms_per_completion
                / base.lock_wait_ms_per_completion.max(0.000_001);
            let lock_contention_ratio = current.lock_contention_events_per_completion
                / base.lock_contention_events_per_completion.max(0.000_001);
            if allocation_ratio > thresholds.resource_ratio_max
                || allocated_bytes_ratio > thresholds.resource_ratio_max
            {
                reason_codes.insert("allocation_budget_exceeded".to_string());
            }
            if lock_wait_ratio > thresholds.resource_ratio_max {
                reason_codes.insert("lock_wait_budget_exceeded".to_string());
            }
            if lock_contention_ratio > thresholds.resource_ratio_max {
                reason_codes.insert("lock_contention_budget_exceeded".to_string());
            }
        } else if thresholds.blocking_mode {
            reason_codes.insert("initial_budget_not_fixed".to_string());
        }
    } else if thresholds.blocking_mode {
        reason_codes.insert("initial_budget_not_fixed".to_string());
    }

    let reason_codes_vec: Vec<String> = reason_codes.into_iter().collect();
    let verdict = if reason_codes_vec.is_empty() {
        "pass"
    } else {
        "fail"
    };

    serde_json::json!({
        "contract_version": contract_version,
        "verdict": verdict,
        "reason_codes": reason_codes_vec,
        "profiles": {
            profile: {
                "metrics": {
                    "latency": {
                        "p95_ms": current.p95_ms,
                        "p99_ms": current.p99_ms,
                        "ratio_p95": ratio_p95,
                        "ratio_p99": ratio_p99,
                        "absolute_ceiling_p95_ms": ceiling_p95,
                        "absolute_ceiling_p99_ms": ceiling_p99
                    },
                    "resource": {
                        "allocations_per_completion": current.allocations_per_completion,
                        "allocated_bytes_per_completion": current.allocated_bytes_per_completion,
                        "lock_wait_ms_per_completion": current.lock_wait_ms_per_completion,
                        "lock_contention_events_per_completion": current.lock_contention_events_per_completion
                    }
                },
                "rates": {
                    "error_rate": current.error_rate,
                    "incomplete_rate": current.incomplete_rate
                }
            }
        },
        "thresholds": {
            "latency_ratio_p95_max": thresholds.latency_ratio_p95_max,
            "latency_ratio_p99_max": thresholds.latency_ratio_p99_max,
            "resource_ratio_max": thresholds.resource_ratio_max,
            "max_error_rate": thresholds.max_error_rate,
            "max_incomplete_rate": thresholds.max_incomplete_rate,
            "blocking_mode": thresholds.blocking_mode
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn contract_fixture() -> Value {
        serde_json::json!({
            "surface": "intellisense-perf-gate",
            "major_version": 1,
            "input": {
                "required_profiles": ["small", "large", "churn"]
            },
            "baseline": {
                "absolute_latency_ceilings_ms": {
                    "small": {"p95": 300, "p99": 600},
                    "large": {"p95": 1500, "p99": 3000},
                    "churn": {"p95": 1800, "p99": 3500}
                },
                "resource_budget_ceilings": {
                    "small": {
                        "allocations_per_completion": 2000000,
                        "allocated_bytes_per_completion": 200000000,
                        "lock_wait_ms_per_completion": 5000,
                        "lock_contention_events_per_completion": 5000
                    },
                    "large": {
                        "allocations_per_completion": 5000000,
                        "allocated_bytes_per_completion": 500000000,
                        "lock_wait_ms_per_completion": 10000,
                        "lock_contention_events_per_completion": 10000
                    },
                    "churn": {
                        "allocations_per_completion": 6000000,
                        "allocated_bytes_per_completion": 600000000,
                        "lock_wait_ms_per_completion": 15000,
                        "lock_contention_events_per_completion": 15000
                    }
                }
            }
        })
    }

    fn sample(p95_ms: f64, p99_ms: f64) -> PerfGateSample {
        PerfGateSample {
            p95_ms,
            p99_ms,
            error_rate: 0.0,
            incomplete_rate: 0.0,
            allocations_per_completion: 1000.0,
            allocated_bytes_per_completion: 100000.0,
            lock_wait_ms_per_completion: 1.0,
            lock_contention_events_per_completion: 1.0,
        }
    }

    fn thresholds(blocking_mode: bool) -> PerfGateThresholds {
        PerfGateThresholds {
            latency_ratio_p95_max: 1.10,
            latency_ratio_p99_max: 1.15,
            resource_ratio_max: 1.20,
            max_error_rate: 0.0,
            max_incomplete_rate: 0.0,
            blocking_mode,
        }
    }

    #[test]
    fn perf_gate_passes_when_metrics_are_within_thresholds() {
        let evaluation = evaluate_intellisense_perf_profile(
            &contract_fixture(),
            "small",
            sample(100.0, 120.0),
            Some(sample(100.0, 120.0)),
            thresholds(false),
        );

        assert_eq!(
            evaluation.get("verdict").and_then(Value::as_str),
            Some("pass")
        );
        assert_eq!(
            evaluation
                .get("reason_codes")
                .and_then(Value::as_array)
                .map(std::vec::Vec::len),
            Some(0)
        );
    }

    #[test]
    fn perf_gate_fails_on_absolute_latency_ceiling() {
        let evaluation = evaluate_intellisense_perf_profile(
            &contract_fixture(),
            "small",
            sample(301.0, 601.0),
            Some(sample(100.0, 120.0)),
            thresholds(false),
        );

        let reason_codes = evaluation
            .get("reason_codes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            evaluation.get("verdict").and_then(Value::as_str),
            Some("fail")
        );
        assert!(reason_codes
            .iter()
            .filter_map(Value::as_str)
            .any(|code| code == "latency_absolute_ceiling_exceeded"));
    }

    #[test]
    fn perf_gate_fails_closed_for_missing_baseline_in_blocking_mode() {
        let evaluation = evaluate_intellisense_perf_profile(
            &contract_fixture(),
            "small",
            sample(100.0, 120.0),
            None,
            thresholds(true),
        );

        let reason_codes = evaluation
            .get("reason_codes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            evaluation.get("verdict").and_then(Value::as_str),
            Some("fail")
        );
        assert!(reason_codes
            .iter()
            .filter_map(Value::as_str)
            .any(|code| code == "initial_budget_not_fixed"));
    }

    #[test]
    fn provenance_allows_legacy_local_without_expected_change_id() {
        let report = json!({
            "contract_version": "v1",
            "metrics": {}
        });

        validate_perf_report_provenance(&report, None).expect("legacy-local should pass");
    }

    #[test]
    fn provenance_requires_change_id_when_expected_is_provided() {
        let report = json!({
            "contract_version": "v1",
            "metrics": {}
        });

        let err =
            validate_perf_report_provenance(&report, Some("refactor-v2-contract-first-hardening"))
                .expect_err("missing provenance must fail");
        assert_eq!(err, "provenance_missing_for_authoritative_run");
    }

    #[test]
    fn provenance_rejects_mismatch_with_expected_change_id() {
        let report = json!({
            "change_id": "refactor-v2-event-driven-type-index-cache"
        });

        let err =
            validate_perf_report_provenance(&report, Some("refactor-v2-contract-first-hardening"))
                .expect_err("mismatch must fail");
        assert_eq!(err, "provenance_mismatch_expected_change_id");
    }

    #[test]
    fn provenance_rejects_invalid_change_id_format() {
        let report = json!({
            "change_id": "Refactor_V2"
        });

        let err =
            validate_perf_report_provenance(&report, None).expect_err("invalid format must fail");
        assert_eq!(err, "provenance_invalid");
    }

    #[test]
    fn provenance_accepts_optional_v1_provenance_payload() {
        let report = json!({
            "change_id": "refactor-v2-contract-first-hardening",
            "provenance": {
                "change_id": "refactor-v2-contract-first-hardening",
                "generated_at": "2026-03-03T19:00:00Z",
                "profile": "small",
                "schema_version": 1,
                "contract_version": "v1"
            }
        });

        validate_perf_report_provenance(&report, Some("refactor-v2-contract-first-hardening"))
            .expect("matching provenance should pass");
    }

    #[test]
    fn parity_cutover_rejects_insufficient_pairs_total() {
        let report = json!({
            "results": {
                "parity_pairs_total": 99,
                "parity_mismatch_rate": 0.0
            }
        });

        let err =
            validate_parity_cutover_evidence(&report).expect_err("insufficient pairs must fail");
        assert_eq!(err, "parity_evidence_insufficient");
    }

    #[test]
    fn parity_cutover_rejects_drift_over_threshold() {
        let report = json!({
            "results": {
                "parity_pairs_total": 120,
                "parity_mismatch_rate": 0.02
            }
        });

        let err =
            validate_parity_cutover_evidence(&report).expect_err("drift over threshold must fail");
        assert_eq!(err, "parity_drift_threshold_exceeded");
    }

    #[test]
    fn parity_cutover_accepts_valid_evidence() {
        let report = json!({
            "results": {
                "parity_pairs_total": 120,
                "parity_mismatch_rate": 0.005
            }
        });

        validate_parity_cutover_evidence(&report).expect("valid parity evidence should pass");
    }
}
