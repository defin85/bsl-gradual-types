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

    let err = validate_perf_report_provenance(&report, None).expect_err("invalid format must fail");
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
fn cutover_authority_requires_expected_change_id() {
    let err = validate_cutover_evidence_authority(None)
        .expect_err("cutover context without expected change id must fail-closed");
    assert_eq!(err, NON_AUTHORITATIVE_CUTOVER_EVIDENCE_REASON);
}

#[test]
fn cutover_authority_accepts_valid_expected_change_id() {
    validate_cutover_evidence_authority(Some("refactor-v2-contract-first-hardening"))
        .expect("valid expected change id should mark evidence as authoritative");
}

#[test]
fn cutover_authority_rejects_invalid_expected_change_id() {
    let err = validate_cutover_evidence_authority(Some("Refactor_V2"))
        .expect_err("invalid expected change id must fail");
    assert_eq!(err, "provenance_invalid");
}

#[test]
fn parity_cutover_rejects_insufficient_pairs_total() {
    let report = json!({
        "results": {
            "parity_pairs_total": 99,
            "parity_mismatch_rate": 0.0
        }
    });

    let err = validate_parity_cutover_evidence(&report).expect_err("insufficient pairs must fail");
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
fn parity_cutover_canary_rollback_guard_blocks_drift_regression() {
    let canary_regression = json!({
        "mode": "canary",
        "canary_percent": 100,
        "results": {
            "parity_pairs_total": 120,
            "parity_drift_rate": PARITY_DRIFT_RATE_MAX_FOR_CUTOVER + 0.0001
        }
    });

    let err = validate_parity_cutover_evidence(&canary_regression)
        .expect_err("canary drift regression must fail-closed and block cutover");
    assert_eq!(err, "parity_drift_threshold_exceeded");

    let canary_at_threshold = json!({
        "mode": "canary",
        "canary_percent": 100,
        "results": {
            "parity_pairs_total": 120,
            "parity_drift_rate": PARITY_DRIFT_RATE_MAX_FOR_CUTOVER
        }
    });
    validate_parity_cutover_evidence(&canary_at_threshold)
        .expect("exact threshold value must remain acceptable");
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
