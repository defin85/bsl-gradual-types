use super::*;

fn percentile_sorted(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let clamped = percentile.clamp(0.0, 1.0);
    let rank = ((values.len() - 1) as f64 * clamped).round() as usize;
    values[rank]
}

fn build_metric_distribution(values: &[f64]) -> PerfMetrics {
    let count = values.len();
    if count == 0 {
        return PerfMetrics {
            count: 0,
            p50_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
        };
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    PerfMetrics {
        count,
        p50_ms: percentile_sorted(&sorted, 0.50),
        p95_ms: percentile_sorted(&sorted, 0.95),
        p99_ms: percentile_sorted(&sorted, 0.99),
    }
}

pub(super) fn build_results(
    _contract: &serde_json::Value,
    iterations: usize,
    case_counts: &HashMap<ResultGroupKey, usize>,
    measured: &HashMap<ResultGroupKey, MeasuredResults>,
) -> Vec<PerfResultEntry> {
    let mut keys = case_counts.keys().copied().collect::<Vec<_>>();
    keys.sort_by_key(|key| (key.fixture_family, key.operation));

    keys.into_iter()
        .map(|key| {
            let cases = *case_counts.get(&key).unwrap_or(&0);
            let total_requests = cases.saturating_mul(iterations);
            let measured = measured.get(&key).cloned().unwrap_or_default();
            let total_requests_f = total_requests.max(1) as f64;
            let fail_closed_rate = measured.fail_closed as f64 / total_requests_f;
            let error_rate = measured.errors as f64 / total_requests_f;
            let incomplete_rate = measured.incomplete as f64 / total_requests_f;

            PerfResultEntry {
                fixture_family: key.fixture_family,
                operation: key.operation,
                cases,
                total_requests,
                fail_closed_total: measured.fail_closed,
                fail_closed_rate,
                error_rate,
                incomplete_rate,
                metrics: PerfResultMetrics {
                    total_duration_ms: build_metric_distribution(&measured.total_duration_ms),
                    wait_for_file_version_ms: build_metric_distribution(
                        &measured.wait_for_file_version_ms,
                    ),
                    snapshot_preparation_ms: build_metric_distribution(
                        &measured.snapshot_preparation_ms,
                    ),
                    ir_query_ms: build_metric_distribution(&measured.ir_query_ms),
                    allocations_per_request: measured.allocation_count_total as f64
                        / total_requests_f,
                    allocated_bytes_per_request: measured.allocated_bytes_total as f64
                        / total_requests_f,
                    lock_wait_ms_per_request: measured.lock_wait_ms_total / total_requests_f,
                    lock_contention_events_per_request: measured.lock_contention_events_total as f64
                        / total_requests_f,
                },
                anti_rescue: measured.anti_rescue,
            }
        })
        .collect()
}

fn required_matrix(contract: &serde_json::Value) -> BTreeSet<(String, String)> {
    let mut pairs = BTreeSet::new();
    let Some(matrix) = contract
        .get("input")
        .and_then(|value| value.get("required_operation_matrix"))
        .and_then(serde_json::Value::as_object)
    else {
        return pairs;
    };

    for (fixture_family, operations) in matrix {
        let Some(operations) = operations.as_array() else {
            continue;
        };
        for operation in operations.iter().filter_map(serde_json::Value::as_str) {
            pairs.insert((fixture_family.clone(), operation.to_string()));
        }
    }

    pairs
}

pub(super) fn build_coverage(
    contract: &serde_json::Value,
    results: &[PerfResultEntry],
) -> PerfCoverage {
    let reported_operations = results
        .iter()
        .map(|entry| entry.operation.as_str().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let reported_fixture_families = results
        .iter()
        .map(|entry| entry.fixture_family.as_str().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let matrix = results
        .iter()
        .map(|entry| {
            (
                entry.fixture_family.as_str().to_string(),
                entry.operation.as_str().to_string(),
            )
        })
        .collect::<BTreeSet<_>>();
    let contract_authoritative = contract
        .get("coverage")
        .and_then(|value| value.get("authoritative_for_cutover_acceptance"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let authoritative_for_cutover_acceptance =
        contract_authoritative && required_matrix(contract).is_subset(&matrix);

    PerfCoverage {
        operation_coverage_mode: contract
            .get("coverage")
            .and_then(|value| value.get("operation_coverage_mode"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        reported_operations,
        reported_fixture_families,
        reported_matrix_entries: matrix.len(),
        authoritative_for_cutover_acceptance,
    }
}

pub(super) fn contract_version_from_contract(contract: &serde_json::Value) -> String {
    match (
        contract.get("surface").and_then(|value| value.as_str()),
        contract
            .get("major_version")
            .and_then(|value| value.as_u64()),
    ) {
        (Some("intellisense-perf-gate"), Some(2)) => "v2".to_string(),
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

pub(super) fn build_provenance_failure_comparison(
    current: &PerfReport,
    reason_code: &str,
    _thresholds: PerfGateThresholds,
) -> PerfComparison {
    PerfComparison {
        contract_version: current.contract_version.clone(),
        verdict: "fail".to_string(),
        reason_codes: vec![reason_code.to_string()],
        pass: false,
        entries: Vec::new(),
    }
}

fn build_unsupported_contract_version_comparison(
    expected_contract_version: &str,
) -> PerfComparison {
    PerfComparison {
        contract_version: expected_contract_version.to_string(),
        verdict: "fail".to_string(),
        reason_codes: vec!["unsupported_contract_version".to_string()],
        pass: false,
        entries: Vec::new(),
    }
}

fn report_samples(results: &[PerfResultEntry]) -> Vec<PerfGateSample> {
    results
        .iter()
        .map(|entry| PerfGateSample {
            fixture_family: entry.fixture_family.as_str().to_string(),
            operation: entry.operation.as_str().to_string(),
            total_duration_p95_ms: entry.metrics.total_duration_ms.p95_ms,
            total_duration_p99_ms: entry.metrics.total_duration_ms.p99_ms,
            wait_for_file_version_p95_ms: entry.metrics.wait_for_file_version_ms.p95_ms,
            wait_for_file_version_p99_ms: entry.metrics.wait_for_file_version_ms.p99_ms,
            snapshot_preparation_p95_ms: entry.metrics.snapshot_preparation_ms.p95_ms,
            snapshot_preparation_p99_ms: entry.metrics.snapshot_preparation_ms.p99_ms,
            ir_query_p95_ms: entry.metrics.ir_query_ms.p95_ms,
            ir_query_p99_ms: entry.metrics.ir_query_ms.p99_ms,
            error_rate: entry.error_rate,
            incomplete_rate: entry.incomplete_rate,
            allocations_per_request: entry.metrics.allocations_per_request,
            allocated_bytes_per_request: entry.metrics.allocated_bytes_per_request,
            lock_wait_ms_per_request: entry.metrics.lock_wait_ms_per_request,
            lock_contention_events_per_request: entry.metrics.lock_contention_events_per_request,
            stale_fallback_total: entry.anti_rescue.stale_fallback_total,
            stale_served_total: entry.anti_rescue.stale_served_total,
            degraded_substitute_total: entry.anti_rescue.degraded_substitute_total,
            search_backed_substitute_total: entry.anti_rescue.search_backed_substitute_total,
        })
        .collect()
}

pub(super) fn read_json_value(path: &Path) -> Result<serde_json::Value> {
    let data =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&data).context("Invalid JSON")?;
    Ok(value)
}

pub(super) fn write_report(path: &Path, report: &PerfReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create report dir: {}", parent.display()))?;
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
        return build_unsupported_contract_version_comparison(&expected_contract_version);
    }

    let evaluation = super::evaluate_intellisense_perf_profile_for_harness(
        contract,
        profile,
        &report_samples(&current.results),
        Some(&report_samples(&baseline.results)),
        thresholds,
    );
    PerfComparison {
        contract_version: evaluation
            .get("contract_version")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string(),
        verdict: evaluation
            .get("verdict")
            .and_then(|value| value.as_str())
            .unwrap_or("fail")
            .to_string(),
        reason_codes: evaluation
            .get("reason_codes")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec!["missing_required_metric_field".to_string()]),
        pass: evaluation
            .get("verdict")
            .and_then(|value| value.as_str())
            .unwrap_or("fail")
            == "pass",
        entries: evaluation
            .get("entries")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default(),
    }
}

pub(super) fn write_summary(path: &Path, report: &PerfReport) -> Result<()> {
    let mut lines = Vec::new();
    lines.push(format!("# IntelliSense perf summary ({})", report.profile));
    lines.push(String::new());
    lines.push(format!("- cases: {}", report.cases));
    lines.push(format!("- iterations: {}", report.iterations));
    lines.push(format!("- warmup: {}", report.warmup));
    lines.push(format!(
        "- coverage: {} matrix entries across {} fixture families / {} operations",
        report.coverage.reported_matrix_entries,
        report.coverage.reported_fixture_families.len(),
        report.coverage.reported_operations.len()
    ));
    lines.push(format!("- verdict: {}", report.verdict));
    lines.push(format!(
        "- reason_codes: {}",
        if report.reason_codes.is_empty() {
            "none".to_string()
        } else {
            report.reason_codes.join(", ")
        }
    ));
    lines.push(String::new());
    lines.push("| Fixture | Operation | total p95/p99 ms | ir_query p95/p99 ms | error | incomplete | fail_closed |".to_string());
    lines.push("| --- | --- | ---: | ---: | ---: | ---: | ---: |".to_string());
    for result in &report.results {
        lines.push(format!(
            "| {} | {} | {:.3} / {:.3} | {:.3} / {:.3} | {:.3} | {:.3} | {:.3} |",
            result.fixture_family.as_str(),
            result.operation.as_str(),
            result.metrics.total_duration_ms.p95_ms,
            result.metrics.total_duration_ms.p99_ms,
            result.metrics.ir_query_ms.p95_ms,
            result.metrics.ir_query_ms.p99_ms,
            result.error_rate,
            result.incomplete_rate,
            result.fail_closed_rate
        ));
    }
    if let Some(comparison) = &report.comparison {
        lines.push(String::new());
        lines.push(format!("- comparison_contract_version: {}", comparison.contract_version));
        lines.push(format!("- comparison_verdict: {}", comparison.verdict));
        lines.push(format!(
            "- comparison_reason_codes: {}",
            if comparison.reason_codes.is_empty() {
                "none".to_string()
            } else {
                comparison.reason_codes.join(", ")
            }
        ));
    }
    lines.push(String::new());

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create summary dir: {}", parent.display()))?;
    }
    fs::write(path, lines.join("\n"))
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}
