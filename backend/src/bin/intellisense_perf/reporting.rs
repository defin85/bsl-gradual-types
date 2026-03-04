use super::*;

pub(super) fn build_metrics(total_requests: usize, measured: &MeasuredResults) -> PerfMetrics {
    let count = measured.durations_ms.len();
    let (p50, p95, p99) = if count == 0 {
        (0.0, 0.0, 0.0)
    } else {
        let mut sorted = measured.durations_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        (
            percentile_sorted(&sorted, 0.50),
            percentile_sorted(&sorted, 0.95),
            percentile_sorted(&sorted, 0.99),
        )
    };

    let total_requests_f = total_requests as f64;
    let error_rate = if total_requests == 0 {
        0.0
    } else {
        measured.errors as f64 / total_requests_f
    };
    let incomplete_rate = if total_requests == 0 {
        0.0
    } else {
        measured.incomplete as f64 / total_requests_f
    };
    let allocations_per_completion = if total_requests == 0 {
        0.0
    } else {
        measured.allocation_count_total as f64 / total_requests_f
    };
    let allocated_bytes_per_completion = if total_requests == 0 {
        0.0
    } else {
        measured.allocated_bytes_total as f64 / total_requests_f
    };
    let lock_wait_ms_per_completion = if total_requests == 0 {
        0.0
    } else {
        measured.lock_wait_ms_total / total_requests_f
    };
    let lock_contention_events_per_completion = if total_requests == 0 {
        0.0
    } else {
        measured.lock_contention_events_total as f64 / total_requests_f
    };

    PerfMetrics {
        total_requests,
        count,
        p50_ms: p50,
        p95_ms: p95,
        p99_ms: p99,
        error_rate,
        incomplete_rate,
        allocations_per_completion,
        allocated_bytes_per_completion,
        lock_wait_ms_per_completion,
        lock_contention_events_per_completion,
    }
}

fn percentile_sorted(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let clamped = percentile.clamp(0.0, 1.0);
    let rank = ((values.len() - 1) as f64 * clamped).round() as usize;
    values[rank]
}

const REQUIRED_RESOURCE_METRIC_KEYS: [&str; 4] = [
    "allocations_per_completion",
    "allocated_bytes_per_completion",
    "lock_wait_ms_per_completion",
    "lock_contention_events_per_completion",
];

fn is_numeric_json_value(value: &serde_json::Value) -> bool {
    value.as_f64().is_some() || value.as_i64().is_some() || value.as_u64().is_some()
}

pub(super) fn missing_resource_metric_keys(report: &serde_json::Value) -> Vec<&'static str> {
    let Some(metrics) = report.get("metrics").and_then(|value| value.as_object()) else {
        return REQUIRED_RESOURCE_METRIC_KEYS.into();
    };

    REQUIRED_RESOURCE_METRIC_KEYS
        .iter()
        .copied()
        .filter(|key| {
            metrics
                .get(*key)
                .is_none_or(|value| !is_numeric_json_value(value))
        })
        .collect()
}

pub(super) fn contract_version_from_contract(contract: &serde_json::Value) -> String {
    match (
        contract.get("surface").and_then(|value| value.as_str()),
        contract
            .get("major_version")
            .and_then(|value| value.as_u64()),
    ) {
        (Some("intellisense-perf-gate"), Some(1)) => "v1".to_string(),
        _ => "unknown".to_string(),
    }
}

fn normalize_report_contract_version(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "unknown"
    } else {
        trimmed
    }
}

pub(super) fn is_report_contract_version_compatible(
    expected_contract_version: &str,
    current_contract_version: &str,
    baseline_contract_version: &str,
) -> bool {
    let current = normalize_report_contract_version(current_contract_version);
    let baseline = normalize_report_contract_version(baseline_contract_version);
    current == expected_contract_version
        && (baseline == expected_contract_version || baseline == "unknown")
}

pub(super) fn build_missing_required_metric_comparison(
    contract: &serde_json::Value,
    current: &PerfReport,
    threshold_p95: f64,
    threshold_p99: f64,
    threshold_resource: f64,
    max_error_rate: f64,
    max_incomplete_rate: f64,
) -> PerfComparison {
    PerfComparison {
        baseline_p95_ms: current.metrics.p95_ms,
        baseline_p99_ms: current.metrics.p99_ms,
        ratio_p95: 1.0,
        ratio_p99: 1.0,
        threshold_p95,
        threshold_p99,
        threshold_resource,
        max_error_rate,
        max_incomplete_rate,
        error_rate: current.metrics.error_rate,
        incomplete_rate: current.metrics.incomplete_rate,
        contract_version: contract_version_from_contract(contract),
        verdict: "fail".to_string(),
        reason_codes: vec!["missing_required_metric_field".to_string()],
        pass: false,
    }
}

pub(super) fn build_provenance_failure_comparison(
    current: &PerfReport,
    reason_code: &str,
    thresholds: PerfGateThresholds,
) -> PerfComparison {
    PerfComparison {
        baseline_p95_ms: current.metrics.p95_ms,
        baseline_p99_ms: current.metrics.p99_ms,
        ratio_p95: 1.0,
        ratio_p99: 1.0,
        threshold_p95: thresholds.latency_ratio_p95_max,
        threshold_p99: thresholds.latency_ratio_p99_max,
        threshold_resource: thresholds.resource_ratio_max,
        max_error_rate: thresholds.max_error_rate,
        max_incomplete_rate: thresholds.max_incomplete_rate,
        error_rate: current.metrics.error_rate,
        incomplete_rate: current.metrics.incomplete_rate,
        contract_version: current.contract_version.clone(),
        verdict: "fail".to_string(),
        reason_codes: vec![reason_code.to_string()],
        pass: false,
    }
}

fn build_unsupported_contract_version_comparison(
    expected_contract_version: &str,
    current: &PerfReport,
    baseline: &PerfReport,
    thresholds: PerfGateThresholds,
) -> PerfComparison {
    let baseline_p95 = baseline.metrics.p95_ms.max(0.000_001);
    let baseline_p99 = baseline.metrics.p99_ms.max(0.000_001);
    PerfComparison {
        baseline_p95_ms: baseline.metrics.p95_ms,
        baseline_p99_ms: baseline.metrics.p99_ms,
        ratio_p95: current.metrics.p95_ms / baseline_p95,
        ratio_p99: current.metrics.p99_ms / baseline_p99,
        threshold_p95: thresholds.latency_ratio_p95_max,
        threshold_p99: thresholds.latency_ratio_p99_max,
        threshold_resource: thresholds.resource_ratio_max,
        max_error_rate: thresholds.max_error_rate,
        max_incomplete_rate: thresholds.max_incomplete_rate,
        error_rate: current.metrics.error_rate,
        incomplete_rate: current.metrics.incomplete_rate,
        contract_version: expected_contract_version.to_string(),
        verdict: "fail".to_string(),
        reason_codes: vec!["unsupported_contract_version".to_string()],
        pass: false,
    }
}

pub(super) fn read_json_value(path: &Path) -> Result<serde_json::Value> {
    let data =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&data).context("Invalid JSON")?;
    Ok(value)
}

pub(super) fn write_report(path: &Path, report: &PerfReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create report dir: {}", parent.to_string_lossy())
        })?;
    }
    let json = serde_json::to_string_pretty(report)?;
    fs::write(path, json).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub(super) fn compare_reports(
    contract: &serde_json::Value,
    profile: &str,
    current: &PerfReport,
    baseline: &PerfReport,
    thresholds: PerfGateThresholds,
) -> PerfComparison {
    let expected_contract_version = contract_version_from_contract(contract);
    if !is_report_contract_version_compatible(
        &expected_contract_version,
        &current.contract_version,
        &baseline.contract_version,
    ) {
        return build_unsupported_contract_version_comparison(
            &expected_contract_version,
            current,
            baseline,
            thresholds,
        );
    }

    let baseline_p95 = baseline.metrics.p95_ms.max(0.000_001);
    let baseline_p99 = baseline.metrics.p99_ms.max(0.000_001);
    let ratio_p95 = current.metrics.p95_ms / baseline_p95;
    let ratio_p99 = current.metrics.p99_ms / baseline_p99;
    let evaluation = evaluate_intellisense_perf_profile(
        contract,
        profile,
        PerfGateSample {
            p95_ms: current.metrics.p95_ms,
            p99_ms: current.metrics.p99_ms,
            error_rate: current.metrics.error_rate,
            incomplete_rate: current.metrics.incomplete_rate,
            allocations_per_completion: current.metrics.allocations_per_completion,
            allocated_bytes_per_completion: current.metrics.allocated_bytes_per_completion,
            lock_wait_ms_per_completion: current.metrics.lock_wait_ms_per_completion,
            lock_contention_events_per_completion: current
                .metrics
                .lock_contention_events_per_completion,
        },
        Some(PerfGateSample {
            p95_ms: baseline.metrics.p95_ms,
            p99_ms: baseline.metrics.p99_ms,
            error_rate: baseline.metrics.error_rate,
            incomplete_rate: baseline.metrics.incomplete_rate,
            allocations_per_completion: baseline.metrics.allocations_per_completion,
            allocated_bytes_per_completion: baseline.metrics.allocated_bytes_per_completion,
            lock_wait_ms_per_completion: baseline.metrics.lock_wait_ms_per_completion,
            lock_contention_events_per_completion: baseline
                .metrics
                .lock_contention_events_per_completion,
        }),
        thresholds,
    );
    let verdict = evaluation
        .get("verdict")
        .and_then(|value| value.as_str())
        .unwrap_or("fail")
        .to_string();
    let reason_codes = evaluation
        .get("reason_codes")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["missing_required_metric_field".to_string()]);
    let pass = verdict == "pass";

    PerfComparison {
        baseline_p95_ms: baseline.metrics.p95_ms,
        baseline_p99_ms: baseline.metrics.p99_ms,
        ratio_p95,
        ratio_p99,
        threshold_p95: thresholds.latency_ratio_p95_max,
        threshold_p99: thresholds.latency_ratio_p99_max,
        threshold_resource: thresholds.resource_ratio_max,
        max_error_rate: thresholds.max_error_rate,
        max_incomplete_rate: thresholds.max_incomplete_rate,
        error_rate: current.metrics.error_rate,
        incomplete_rate: current.metrics.incomplete_rate,
        contract_version: evaluation
            .get("contract_version")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string(),
        verdict,
        reason_codes,
        pass,
    }
}

pub(super) fn write_summary(path: &Path, report: &PerfReport) -> Result<()> {
    let mut lines = Vec::new();
    lines.push(format!("# IntelliSense perf summary ({})", report.scenario));
    lines.push(String::new());
    lines.push(format!("- cases: {}", report.cases));
    lines.push(format!("- iterations: {}", report.iterations));
    lines.push(format!("- warmup: {}", report.warmup));
    lines.push(format!(
        "- p50/p95/p99 (ms): {:.3} / {:.3} / {:.3}",
        report.metrics.p50_ms, report.metrics.p95_ms, report.metrics.p99_ms
    ));
    lines.push(format!(
        "- resource per completion (allocs/bytes/lock_wait_ms/lock_contention): {:.3} / {:.3} / {:.3} / {:.3}",
        report.metrics.allocations_per_completion,
        report.metrics.allocated_bytes_per_completion,
        report.metrics.lock_wait_ms_per_completion,
        report.metrics.lock_contention_events_per_completion
    ));
    lines.push(format!("- error_rate: {:.3}", report.metrics.error_rate));
    lines.push(format!(
        "- incomplete_rate: {:.3}",
        report.metrics.incomplete_rate
    ));
    if let Some(thresholds) = &report.thresholds {
        lines.push(format!(
            "- max_error_rate: {:.3}",
            thresholds.max_error_rate
        ));
        lines.push(format!(
            "- max_incomplete_rate: {:.3}",
            thresholds.max_incomplete_rate
        ));
    }
    if let Some(comparison) = &report.comparison {
        lines.push(format!(
            "- ratio_p95/ratio_p99: {:.3} / {:.3}",
            comparison.ratio_p95, comparison.ratio_p99
        ));
        lines.push(format!(
            "- thresholds p95/p99: {:.2} / {:.2}",
            comparison.threshold_p95, comparison.threshold_p99
        ));
        lines.push(format!(
            "- resource_ratio_threshold: {:.2}",
            comparison.threshold_resource
        ));
        lines.push(format!(
            "- contract_version: {}",
            comparison.contract_version
        ));
        lines.push(format!("- verdict: {}", comparison.verdict));
        lines.push(format!(
            "- reason_codes: {}",
            comparison.reason_codes.join(", ")
        ));
        lines.push(format!(
            "- pass: {}",
            if comparison.pass { "yes" } else { "no" }
        ));
    }
    lines.push(String::new());

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create summary dir: {}", parent.to_string_lossy())
        })?;
    }
    fs::write(path, lines.join("\n"))
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}
