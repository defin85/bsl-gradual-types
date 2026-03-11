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
pub const NON_AUTHORITATIVE_CUTOVER_EVIDENCE_REASON: &str =
    "provenance_non_authoritative_cutover_evidence";

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

pub fn validate_cutover_evidence_authority(expected_change_id: Option<&str>) -> Result<(), String> {
    let Some(change_id) = expected_change_id else {
        return Err(NON_AUTHORITATIVE_CUTOVER_EVIDENCE_REASON.to_string());
    };
    if !is_valid_change_id(change_id) {
        return Err("provenance_invalid".to_string());
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
    let large_start_stale_served_total = phase_counter(
        "large",
        "start",
        "intellisense_v2_interactive_stale_served_total",
    )?;
    let large_cold_stale_served_total = phase_counter(
        "large",
        "cold",
        "intellisense_v2_interactive_stale_served_total",
    )?;
    let large_warm_stale_served_total = phase_counter(
        "large",
        "warm",
        "intellisense_v2_interactive_stale_served_total",
    )?;
    let small_start_stale_served_total = phase_counter(
        "small",
        "start",
        "intellisense_v2_interactive_stale_served_total",
    )?;
    let small_cold_stale_served_total = phase_counter(
        "small",
        "cold",
        "intellisense_v2_interactive_stale_served_total",
    )?;
    let small_warm_stale_served_total = phase_counter(
        "small",
        "warm",
        "intellisense_v2_interactive_stale_served_total",
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

    let large_warm_fallback_unavailable_rate = large_warm_fallback_unavailable_total as f64
        / large_warm_budget_exhausted_total.max(1) as f64;
    let small_warm_fallback_unavailable_rate = small_warm_fallback_unavailable_total as f64
        / small_warm_budget_exhausted_total.max(1) as f64;
    let semantic_substitute_detected = large_start_stale_fallback_total > 0
        || large_cold_stale_fallback_total > 0
        || large_warm_stale_fallback_total > 0
        || small_start_stale_fallback_total > 0
        || small_cold_stale_fallback_total > 0
        || small_warm_stale_fallback_total > 0
        || large_start_stale_served_total > 0
        || large_cold_stale_served_total > 0
        || large_warm_stale_served_total > 0
        || small_start_stale_served_total > 0
        || small_cold_stale_served_total > 0
        || small_warm_stale_served_total > 0;
    let anti_rescue_guard_pass = !semantic_substitute_detected;
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
        && anti_rescue_guard_pass;

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
        "anti_rescue_guard": {
            "semantic_substitute_detected": semantic_substitute_detected,
            "pass": anti_rescue_guard_pass,
            "counts": {
                "large": {
                    "start_stale_served_total": large_start_stale_served_total,
                    "start_stale_fallback_total": large_start_stale_fallback_total,
                    "cold_stale_served_total": large_cold_stale_served_total,
                    "cold_stale_fallback_total": large_cold_stale_fallback_total,
                    "warm_stale_served_total": large_warm_stale_served_total,
                    "warm_stale_fallback_total": large_warm_stale_fallback_total,
                    "warm_budget_exhausted_total": large_warm_budget_exhausted_total,
                    "warm_fallback_unavailable_total": large_warm_fallback_unavailable_total
                },
                "small": {
                    "start_stale_served_total": small_start_stale_served_total,
                    "start_stale_fallback_total": small_start_stale_fallback_total,
                    "cold_stale_served_total": small_cold_stale_served_total,
                    "cold_stale_fallback_total": small_cold_stale_fallback_total,
                    "warm_stale_served_total": small_warm_stale_served_total,
                    "warm_stale_fallback_total": small_warm_stale_fallback_total,
                    "warm_budget_exhausted_total": small_warm_budget_exhausted_total,
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
            "min_completion_total": MIN_COMPLETION_TOTAL
        }
    }))
}

#[derive(Debug, Clone)]
pub struct PerfGateSample {
    pub fixture_family: String,
    pub operation: String,
    pub total_duration_p95_ms: f64,
    pub total_duration_p99_ms: f64,
    pub wait_for_file_version_p95_ms: f64,
    pub wait_for_file_version_p99_ms: f64,
    pub snapshot_preparation_p95_ms: f64,
    pub snapshot_preparation_p99_ms: f64,
    pub ir_query_p95_ms: f64,
    pub ir_query_p99_ms: f64,
    pub error_rate: f64,
    pub incomplete_rate: f64,
    pub allocations_per_request: f64,
    pub allocated_bytes_per_request: f64,
    pub lock_wait_ms_per_request: f64,
    pub lock_contention_events_per_request: f64,
    pub stale_fallback_total: u64,
    pub stale_served_total: u64,
    pub degraded_substitute_total: u64,
    pub search_backed_substitute_total: u64,
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

fn read_contract_f64(contract: &Value, path: &[&str]) -> Option<f64> {
    let mut cursor = contract;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_f64().or_else(|| cursor.as_u64().map(|value| value as f64))
}

fn read_required_operation_matrix(contract: &Value) -> BTreeSet<(String, String)> {
    let mut pairs = BTreeSet::new();
    let Some(matrix) = contract
        .get("input")
        .and_then(|value| value.get("required_operation_matrix"))
        .and_then(Value::as_object)
    else {
        return pairs;
    };

    for (fixture_family, operations) in matrix {
        let Some(operations) = operations.as_array() else {
            continue;
        };
        for operation in operations.iter().filter_map(Value::as_str) {
            pairs.insert((fixture_family.clone(), operation.to_string()));
        }
    }

    pairs
}

fn read_latency_ceiling(
    contract: &Value,
    profile: &str,
    fixture_family: &str,
    operation: &str,
    metric_family: &str,
    percentile: &str,
) -> Option<f64> {
    read_contract_f64(
        contract,
        &[
            "baseline",
            "absolute_latency_ceilings_ms",
            profile,
            fixture_family,
            operation,
            metric_family,
            percentile,
        ],
    )
    .or_else(|| {
        read_contract_f64(
            contract,
            &[
                "baseline",
                "absolute_latency_ceilings_ms",
                profile,
                "default",
                operation,
                metric_family,
                percentile,
            ],
        )
    })
}

fn read_resource_budget(
    contract: &Value,
    profile: &str,
    fixture_family: &str,
    operation: &str,
    metric_name: &str,
) -> Option<f64> {
    read_contract_f64(
        contract,
        &[
            "baseline",
            "resource_budget_ceilings",
            profile,
            fixture_family,
            operation,
            metric_name,
        ],
    )
    .or_else(|| {
        read_contract_f64(
            contract,
            &[
                "baseline",
                "resource_budget_ceilings",
                profile,
                "default",
                operation,
                metric_name,
            ],
        )
    })
}

fn read_relative_ratio_baseline_floor(contract: &Value, metric_name: &str) -> Option<f64> {
    read_contract_f64(
        contract,
        &["baseline", "relative_ratio_baseline_floors", metric_name],
    )
}

fn sample_by_key<'a>(
    samples: &'a [PerfGateSample],
    fixture_family: &str,
    operation: &str,
) -> Option<&'a PerfGateSample> {
    samples.iter().find(|sample| {
        sample.fixture_family == fixture_family && sample.operation == operation
    })
}

pub fn evaluate_intellisense_perf_profile(
    contract: &Value,
    profile: &str,
    current: &[PerfGateSample],
    baseline: Option<&[PerfGateSample]>,
    thresholds: PerfGateThresholds,
) -> Value {
    let mut top_level_reason_codes = BTreeSet::new();

    let contract_version = match (
        contract.get("surface").and_then(|v| v.as_str()),
        contract.get("major_version").and_then(|v| v.as_u64()),
    ) {
        (Some("intellisense-perf-gate"), Some(2)) => "v2".to_string(),
        _ => {
            top_level_reason_codes.insert("unsupported_contract_version".to_string());
            "unknown".to_string()
        }
    };

    let required_profiles =
        read_contract_string_vec(contract, &["input", "required_profiles"]).unwrap_or_default();
    if !required_profiles.iter().any(|item| item == profile) {
        top_level_reason_codes.insert("missing_required_metric_field".to_string());
    }

    let required_matrix = read_required_operation_matrix(contract);
    let current_matrix = current
        .iter()
        .map(|sample| (sample.fixture_family.clone(), sample.operation.clone()))
        .collect::<BTreeSet<_>>();
    if required_matrix
        .iter()
        .any(|pair| !current_matrix.contains(pair))
    {
        top_level_reason_codes.insert("missing_required_matrix_coverage".to_string());
    }
    if thresholds.blocking_mode {
        if let Some(baseline_samples) = baseline {
            let baseline_matrix = baseline_samples
                .iter()
                .map(|sample| (sample.fixture_family.clone(), sample.operation.clone()))
                .collect::<BTreeSet<_>>();
            if required_matrix
                .iter()
                .any(|pair| !baseline_matrix.contains(pair))
            {
                top_level_reason_codes.insert("missing_required_matrix_coverage".to_string());
            }
        } else {
            top_level_reason_codes.insert("initial_budget_not_fixed".to_string());
        }
    }

    let anti_rescue_budget_stale_fallback = read_contract_u64(
        contract,
        &["baseline", "anti_rescue_budget_ceilings", "stale_fallback_total"],
    )
    .unwrap_or(0);
    let anti_rescue_budget_stale_served = read_contract_u64(
        contract,
        &["baseline", "anti_rescue_budget_ceilings", "stale_served_total"],
    )
    .unwrap_or(0);
    let anti_rescue_budget_degraded = read_contract_u64(
        contract,
        &["baseline", "anti_rescue_budget_ceilings", "degraded_substitute_total"],
    )
    .unwrap_or(0);
    let anti_rescue_budget_search = read_contract_u64(
        contract,
        &[
            "baseline",
            "anti_rescue_budget_ceilings",
            "search_backed_substitute_total",
        ],
    )
    .unwrap_or(0);

    let mut entries = Vec::new();
    for (fixture_family, operation) in &required_matrix {
        let Some(current_entry) = sample_by_key(current, fixture_family, operation) else {
            continue;
        };
        let baseline_entry = baseline.and_then(|samples| sample_by_key(samples, fixture_family, operation));
        let mut entry_reason_codes = BTreeSet::new();
        let mut latency = serde_json::Map::new();
        let mut resource = serde_json::Map::new();

        if !current_entry.allocations_per_request.is_finite()
            || !current_entry.allocated_bytes_per_request.is_finite()
            || !current_entry.lock_wait_ms_per_request.is_finite()
            || !current_entry.lock_contention_events_per_request.is_finite()
        {
            entry_reason_codes.insert("missing_required_metric_field".to_string());
        }
        if current_entry.error_rate > thresholds.max_error_rate
            || current_entry.incomplete_rate > thresholds.max_incomplete_rate
        {
            entry_reason_codes.insert("missing_required_metric_field".to_string());
        }

        let latency_fields = [
            (
                "total_duration_ms",
                current_entry.total_duration_p95_ms,
                current_entry.total_duration_p99_ms,
                baseline_entry.map(|entry| entry.total_duration_p95_ms),
                baseline_entry.map(|entry| entry.total_duration_p99_ms),
            ),
            (
                "wait_for_file_version_ms",
                current_entry.wait_for_file_version_p95_ms,
                current_entry.wait_for_file_version_p99_ms,
                baseline_entry.map(|entry| entry.wait_for_file_version_p95_ms),
                baseline_entry.map(|entry| entry.wait_for_file_version_p99_ms),
            ),
            (
                "snapshot_preparation_ms",
                current_entry.snapshot_preparation_p95_ms,
                current_entry.snapshot_preparation_p99_ms,
                baseline_entry.map(|entry| entry.snapshot_preparation_p95_ms),
                baseline_entry.map(|entry| entry.snapshot_preparation_p99_ms),
            ),
            (
                "ir_query_ms",
                current_entry.ir_query_p95_ms,
                current_entry.ir_query_p99_ms,
                baseline_entry.map(|entry| entry.ir_query_p95_ms),
                baseline_entry.map(|entry| entry.ir_query_p99_ms),
            ),
        ];

        for (metric_family, current_p95, current_p99, baseline_p95, baseline_p99) in latency_fields {
            let ceiling_p95 =
                read_latency_ceiling(contract, profile, fixture_family, operation, metric_family, "p95");
            let ceiling_p99 =
                read_latency_ceiling(contract, profile, fixture_family, operation, metric_family, "p99");
            let ratio_baseline_floor =
                read_relative_ratio_baseline_floor(contract, metric_family).unwrap_or(0.0);
            if let Some(max_p95) = ceiling_p95 {
                if current_p95 > max_p95 {
                    entry_reason_codes.insert("latency_absolute_ceiling_exceeded".to_string());
                }
            } else if thresholds.blocking_mode {
                entry_reason_codes.insert("initial_budget_not_fixed".to_string());
            }
            if let Some(max_p99) = ceiling_p99 {
                if current_p99 > max_p99 {
                    entry_reason_codes.insert("latency_absolute_ceiling_exceeded".to_string());
                }
            } else if thresholds.blocking_mode {
                entry_reason_codes.insert("initial_budget_not_fixed".to_string());
            }

            let ratio_p95 = baseline_p95
                .map(|baseline| current_p95 / baseline.max(ratio_baseline_floor).max(0.000_001));
            let ratio_p99 = baseline_p99
                .map(|baseline| current_p99 / baseline.max(ratio_baseline_floor).max(0.000_001));
            if let (Some(ratio_p95), Some(ratio_p99)) = (ratio_p95, ratio_p99) {
                if ratio_p95 > thresholds.latency_ratio_p95_max
                    || ratio_p99 > thresholds.latency_ratio_p99_max
                {
                    entry_reason_codes.insert("latency_relative_ratio_exceeded".to_string());
                }
            } else if thresholds.blocking_mode {
                entry_reason_codes.insert("initial_budget_not_fixed".to_string());
            }

            latency.insert(
                metric_family.to_string(),
                serde_json::json!({
                    "current_p95_ms": current_p95,
                    "current_p99_ms": current_p99,
                    "baseline_p95_ms": baseline_p95,
                    "baseline_p99_ms": baseline_p99,
                    "ratio_baseline_floor_ms": ratio_baseline_floor,
                    "ratio_p95": ratio_p95,
                    "ratio_p99": ratio_p99,
                    "ceiling_p95_ms": ceiling_p95,
                    "ceiling_p99_ms": ceiling_p99,
                }),
            );
        }

        let resource_fields = [
            (
                "allocations_per_request",
                current_entry.allocations_per_request,
                baseline_entry.map(|entry| entry.allocations_per_request),
                "allocation_budget_exceeded",
            ),
            (
                "allocated_bytes_per_request",
                current_entry.allocated_bytes_per_request,
                baseline_entry.map(|entry| entry.allocated_bytes_per_request),
                "allocation_budget_exceeded",
            ),
            (
                "lock_wait_ms_per_request",
                current_entry.lock_wait_ms_per_request,
                baseline_entry.map(|entry| entry.lock_wait_ms_per_request),
                "lock_wait_budget_exceeded",
            ),
            (
                "lock_contention_events_per_request",
                current_entry.lock_contention_events_per_request,
                baseline_entry.map(|entry| entry.lock_contention_events_per_request),
                "lock_contention_budget_exceeded",
            ),
        ];

        for (metric_name, current_value, baseline_value, reason_code) in resource_fields {
            let ceiling = read_resource_budget(contract, profile, fixture_family, operation, metric_name);
            let ratio_baseline_floor =
                read_relative_ratio_baseline_floor(contract, metric_name).unwrap_or(0.0);
            if let Some(max_value) = ceiling {
                if current_value > max_value {
                    entry_reason_codes.insert(reason_code.to_string());
                }
            } else if thresholds.blocking_mode {
                entry_reason_codes.insert("initial_budget_not_fixed".to_string());
            }

            let ratio = baseline_value
                .map(|baseline| current_value / baseline.max(ratio_baseline_floor).max(0.000_001));
            if let Some(ratio) = ratio {
                if ratio > thresholds.resource_ratio_max {
                    entry_reason_codes.insert(reason_code.to_string());
                }
            } else if thresholds.blocking_mode {
                entry_reason_codes.insert("initial_budget_not_fixed".to_string());
            }

            resource.insert(
                metric_name.to_string(),
                serde_json::json!({
                    "current": current_value,
                    "baseline": baseline_value,
                    "ratio_baseline_floor": ratio_baseline_floor,
                    "ratio": ratio,
                    "ceiling": ceiling,
                }),
            );
        }

        if current_entry.stale_fallback_total > anti_rescue_budget_stale_fallback
            || current_entry.stale_served_total > anti_rescue_budget_stale_served
            || current_entry.degraded_substitute_total > anti_rescue_budget_degraded
            || current_entry.search_backed_substitute_total > anti_rescue_budget_search
        {
            entry_reason_codes.insert("anti_rescue_budget_exceeded".to_string());
        }

        top_level_reason_codes.extend(entry_reason_codes.iter().cloned());
        let entry_reason_codes = entry_reason_codes.into_iter().collect::<Vec<_>>();
        let pass = entry_reason_codes.is_empty();
        entries.push(serde_json::json!({
            "fixture_family": fixture_family,
            "operation": operation,
            "verdict": if pass { "pass" } else { "fail" },
            "reason_codes": entry_reason_codes,
            "pass": pass,
            "latency": latency,
            "resource": resource,
            "rates": {
                "error_rate": current_entry.error_rate,
                "incomplete_rate": current_entry.incomplete_rate,
            },
            "anti_rescue": {
                "stale_fallback_total": current_entry.stale_fallback_total,
                "stale_served_total": current_entry.stale_served_total,
                "degraded_substitute_total": current_entry.degraded_substitute_total,
                "search_backed_substitute_total": current_entry.search_backed_substitute_total,
            }
        }));
    }

    let reason_codes_vec: Vec<String> = top_level_reason_codes.into_iter().collect();
    let verdict = if reason_codes_vec.is_empty() {
        "pass"
    } else {
        "fail"
    };

    serde_json::json!({
        "contract_version": contract_version,
        "profile": profile,
        "verdict": verdict,
        "reason_codes": reason_codes_vec,
        "entries": entries,
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
mod tests;
