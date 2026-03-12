use super::*;
use serde_json::json;

fn contract_fixture() -> Value {
    serde_json::json!({
        "surface": "intellisense-perf-gate",
        "major_version": 2,
        "schema_version": 1,
        "input": {
            "required_profiles": ["small", "large", "churn"],
            "required_operation_matrix": {
                "steady_member_chain": [
                    "completion",
                    "hover",
                    "definition",
                    "type_at_position",
                    "members"
                ],
                "post_did_change_current_revision": [
                    "completion",
                    "hover",
                    "definition",
                    "type_at_position",
                    "members"
                ],
                "object_module_explicit_context": [
                    "completion",
                    "hover",
                    "definition",
                    "type_at_position",
                    "members"
                ],
                "recordset_module_explicit_context": [
                    "completion",
                    "hover",
                    "definition",
                    "type_at_position",
                    "members"
                ],
                "incomplete_syntax_member_access": [
                    "completion"
                ]
            },
            "required_latency_metric_families": [
                "total_duration_ms",
                "wait_for_file_version_ms",
                "snapshot_preparation_ms",
                "ir_query_ms"
            ],
            "required_resource_metric_families": [
                "allocations_per_request",
                "allocated_bytes_per_request",
                "lock_wait_ms_per_request",
                "lock_contention_events_per_request"
            ]
        },
        "coverage": {
            "operation_coverage_mode": "representative_matrix",
            "reported_operations": [
                "completion",
                "hover",
                "definition",
                "type_at_position",
                "members"
            ],
            "reported_fixture_families": [
                "steady_member_chain",
                "post_did_change_current_revision",
                "object_module_explicit_context",
                "recordset_module_explicit_context",
                "incomplete_syntax_member_access"
            ],
            "authoritative_for_cutover_acceptance": true
        },
        "baseline": {
            "absolute_latency_ceilings_ms": {
                "small": {
                    "default": {
                        "completion": latency_budget(300, 600),
                        "hover": latency_budget(300, 600),
                        "definition": latency_budget(300, 600),
                        "type_at_position": latency_budget(300, 600),
                        "members": latency_budget(300, 600)
                    },
                    "incomplete_syntax_member_access": {
                        "completion": latency_budget(300, 600)
                    }
                },
                "large": {
                    "default": {
                        "completion": latency_budget(1500, 3000),
                        "hover": latency_budget(1500, 3000),
                        "definition": latency_budget(1500, 3000),
                        "type_at_position": latency_budget(1500, 3000),
                        "members": latency_budget(1500, 3000)
                    },
                    "incomplete_syntax_member_access": {
                        "completion": latency_budget(1500, 3000)
                    }
                },
                "churn": {
                    "default": {
                        "completion": latency_budget(1800, 3500),
                        "hover": latency_budget(1800, 3500),
                        "definition": latency_budget(1800, 3500),
                        "type_at_position": latency_budget(1800, 3500),
                        "members": latency_budget(1800, 3500)
                    },
                    "incomplete_syntax_member_access": {
                        "completion": latency_budget(1800, 3500)
                    }
                }
            },
            "resource_budget_ceilings": {
                "small": {
                    "default": {
                        "completion": resource_budget(2000000, 200000000, 5000, 5000),
                        "hover": resource_budget(2000000, 200000000, 5000, 5000),
                        "definition": resource_budget(2000000, 200000000, 5000, 5000),
                        "type_at_position": resource_budget(2000000, 200000000, 5000, 5000),
                        "members": resource_budget(2000000, 200000000, 5000, 5000)
                    },
                    "incomplete_syntax_member_access": {
                        "completion": resource_budget(2000000, 200000000, 5000, 5000)
                    }
                },
                "large": {
                    "default": {
                        "completion": resource_budget(5000000, 500000000, 10000, 10000),
                        "hover": resource_budget(5000000, 500000000, 10000, 10000),
                        "definition": resource_budget(5000000, 500000000, 10000, 10000),
                        "type_at_position": resource_budget(5000000, 500000000, 10000, 10000),
                        "members": resource_budget(5000000, 500000000, 10000, 10000)
                    },
                    "incomplete_syntax_member_access": {
                        "completion": resource_budget(5000000, 500000000, 10000, 10000)
                    }
                },
                "churn": {
                    "default": {
                        "completion": resource_budget(6000000, 600000000, 15000, 15000),
                        "hover": resource_budget(6000000, 600000000, 15000, 15000),
                        "definition": resource_budget(6000000, 600000000, 15000, 15000),
                        "type_at_position": resource_budget(6000000, 600000000, 15000, 15000),
                        "members": resource_budget(6000000, 600000000, 15000, 15000)
                    },
                    "incomplete_syntax_member_access": {
                        "completion": resource_budget(6000000, 600000000, 15000, 15000)
                    }
                }
            },
            "fail_closed_budget_ceilings": {
                "small": {
                    "default": {
                        "completion": fail_closed_budget(0, 0.0),
                        "hover": fail_closed_budget(0, 0.0),
                        "definition": fail_closed_budget(0, 0.0),
                        "type_at_position": fail_closed_budget(0, 0.0),
                        "members": fail_closed_budget(0, 0.0)
                    },
                    "incomplete_syntax_member_access": {
                        "completion": fail_closed_budget(0, 0.0)
                    }
                },
                "large": {
                    "default": {
                        "completion": fail_closed_budget(0, 0.0),
                        "hover": fail_closed_budget(0, 0.0),
                        "definition": fail_closed_budget(0, 0.0),
                        "type_at_position": fail_closed_budget(0, 0.0),
                        "members": fail_closed_budget(0, 0.0)
                    },
                    "incomplete_syntax_member_access": {
                        "completion": fail_closed_budget(0, 0.0)
                    }
                },
                "churn": {
                    "default": {
                        "completion": fail_closed_budget(0, 0.0),
                        "hover": fail_closed_budget(0, 0.0),
                        "definition": fail_closed_budget(0, 0.0),
                        "type_at_position": fail_closed_budget(0, 0.0),
                        "members": fail_closed_budget(0, 0.0)
                    },
                    "incomplete_syntax_member_access": {
                        "completion": fail_closed_budget(0, 0.0)
                    }
                }
            },
            "relative_ratio_baseline_floors": {
                "total_duration_ms": 6,
                "wait_for_file_version_ms": 3,
                "snapshot_preparation_ms": 5,
                "ir_query_ms": 3,
                "allocations_per_request": 100,
                "allocated_bytes_per_request": 8192,
                "lock_wait_ms_per_request": 1,
                "lock_contention_events_per_request": 1
            },
            "anti_rescue_budget_ceilings": {
                "stale_fallback_total": 0,
                "stale_served_total": 0,
                "degraded_substitute_total": 0,
                "search_backed_substitute_total": 0
            },
            "bootstrap_policy": {
                "required_profiles": ["small", "large", "churn"],
                "sample_size_min": 5,
                "aggregation_rule": "median"
            }
        },
        "report": {
            "required_fields": [
                "contract_version",
                "profile",
                "coverage",
                "results",
                "verdict",
                "reason_codes"
            ],
            "verdict_enum": [
                "pass",
                "fail"
            ]
        },
        "evaluator": {
            "reason_codes": [
                "missing_required_metric_field",
                "missing_required_matrix_coverage",
                "unsupported_contract_version",
                "latency_relative_ratio_exceeded",
                "latency_absolute_ceiling_exceeded",
                "allocation_budget_exceeded",
                "lock_wait_budget_exceeded",
                "lock_contention_budget_exceeded",
                "anti_rescue_budget_exceeded",
                "fail_closed_budget_exceeded"
            ]
        }
    })
}

fn latency_budget(p95: u64, p99: u64) -> Value {
    json!({
        "total_duration_ms": { "p95": p95, "p99": p99 },
        "wait_for_file_version_ms": { "p95": p95, "p99": p99 },
        "snapshot_preparation_ms": { "p95": p95, "p99": p99 },
        "ir_query_ms": { "p95": p95, "p99": p99 }
    })
}

fn resource_budget(allocations: u64, bytes: u64, lock_wait: u64, lock_contention: u64) -> Value {
    json!({
        "allocations_per_request": allocations,
        "allocated_bytes_per_request": bytes,
        "lock_wait_ms_per_request": lock_wait,
        "lock_contention_events_per_request": lock_contention
    })
}

fn fail_closed_budget(total: u64, rate: f64) -> Value {
    json!({
        "fail_closed_total": total,
        "fail_closed_rate": rate
    })
}

fn sample(fixture_family: &str, operation: &str, p95_ms: f64, p99_ms: f64) -> PerfGateSample {
    PerfGateSample {
        fixture_family: fixture_family.to_string(),
        operation: operation.to_string(),
        total_duration_p95_ms: p95_ms,
        total_duration_p99_ms: p99_ms,
        wait_for_file_version_p95_ms: 1.0,
        wait_for_file_version_p99_ms: 1.0,
        snapshot_preparation_p95_ms: 1.0,
        snapshot_preparation_p99_ms: 1.0,
        ir_query_p95_ms: 1.0,
        ir_query_p99_ms: 1.0,
        fail_closed_total: 0,
        fail_closed_rate: 0.0,
        error_rate: 0.0,
        incomplete_rate: 0.0,
        allocations_per_request: 1000.0,
        allocated_bytes_per_request: 100000.0,
        lock_wait_ms_per_request: 1.0,
        lock_contention_events_per_request: 1.0,
        stale_fallback_total: 0,
        stale_served_total: 0,
        degraded_substitute_total: 0,
        search_backed_substitute_total: 0,
    }
}

fn required_matrix_samples(p95_ms: f64, p99_ms: f64) -> Vec<PerfGateSample> {
    vec![
        sample("steady_member_chain", "completion", p95_ms, p99_ms),
        sample("steady_member_chain", "hover", p95_ms, p99_ms),
        sample("steady_member_chain", "definition", p95_ms, p99_ms),
        sample("steady_member_chain", "type_at_position", p95_ms, p99_ms),
        sample("steady_member_chain", "members", p95_ms, p99_ms),
        sample(
            "post_did_change_current_revision",
            "completion",
            p95_ms,
            p99_ms,
        ),
        sample("post_did_change_current_revision", "hover", p95_ms, p99_ms),
        sample(
            "post_did_change_current_revision",
            "definition",
            p95_ms,
            p99_ms,
        ),
        sample(
            "post_did_change_current_revision",
            "type_at_position",
            p95_ms,
            p99_ms,
        ),
        sample(
            "post_did_change_current_revision",
            "members",
            p95_ms,
            p99_ms,
        ),
        sample(
            "object_module_explicit_context",
            "completion",
            p95_ms,
            p99_ms,
        ),
        sample("object_module_explicit_context", "hover", p95_ms, p99_ms),
        sample(
            "object_module_explicit_context",
            "definition",
            p95_ms,
            p99_ms,
        ),
        sample(
            "object_module_explicit_context",
            "type_at_position",
            p95_ms,
            p99_ms,
        ),
        sample("object_module_explicit_context", "members", p95_ms, p99_ms),
        sample(
            "recordset_module_explicit_context",
            "completion",
            p95_ms,
            p99_ms,
        ),
        sample("recordset_module_explicit_context", "hover", p95_ms, p99_ms),
        sample(
            "recordset_module_explicit_context",
            "definition",
            p95_ms,
            p99_ms,
        ),
        sample(
            "recordset_module_explicit_context",
            "type_at_position",
            p95_ms,
            p99_ms,
        ),
        sample(
            "recordset_module_explicit_context",
            "members",
            p95_ms,
            p99_ms,
        ),
        sample(
            "incomplete_syntax_member_access",
            "completion",
            p95_ms,
            p99_ms,
        ),
    ]
}

fn sample_mut<'a>(
    samples: &'a mut [PerfGateSample],
    fixture_family: &str,
    operation: &str,
) -> &'a mut PerfGateSample {
    samples
        .iter_mut()
        .find(|sample| sample.fixture_family == fixture_family && sample.operation == operation)
        .expect("sample must exist")
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
        &[
            sample("steady_member_chain", "completion", 100.0, 120.0),
            sample("steady_member_chain", "hover", 100.0, 120.0),
            sample("steady_member_chain", "definition", 100.0, 120.0),
            sample("steady_member_chain", "type_at_position", 100.0, 120.0),
            sample("steady_member_chain", "members", 100.0, 120.0),
            sample(
                "post_did_change_current_revision",
                "completion",
                100.0,
                120.0,
            ),
            sample("post_did_change_current_revision", "hover", 100.0, 120.0),
            sample(
                "post_did_change_current_revision",
                "definition",
                100.0,
                120.0,
            ),
            sample(
                "post_did_change_current_revision",
                "type_at_position",
                100.0,
                120.0,
            ),
            sample("post_did_change_current_revision", "members", 100.0, 120.0),
            sample("object_module_explicit_context", "completion", 100.0, 120.0),
            sample("object_module_explicit_context", "hover", 100.0, 120.0),
            sample("object_module_explicit_context", "definition", 100.0, 120.0),
            sample(
                "object_module_explicit_context",
                "type_at_position",
                100.0,
                120.0,
            ),
            sample("object_module_explicit_context", "members", 100.0, 120.0),
            sample(
                "recordset_module_explicit_context",
                "completion",
                100.0,
                120.0,
            ),
            sample("recordset_module_explicit_context", "hover", 100.0, 120.0),
            sample(
                "recordset_module_explicit_context",
                "definition",
                100.0,
                120.0,
            ),
            sample(
                "recordset_module_explicit_context",
                "type_at_position",
                100.0,
                120.0,
            ),
            sample("recordset_module_explicit_context", "members", 100.0, 120.0),
            sample(
                "incomplete_syntax_member_access",
                "completion",
                100.0,
                120.0,
            ),
        ],
        Some(&[
            sample("steady_member_chain", "completion", 100.0, 120.0),
            sample("steady_member_chain", "hover", 100.0, 120.0),
            sample("steady_member_chain", "definition", 100.0, 120.0),
            sample("steady_member_chain", "type_at_position", 100.0, 120.0),
            sample("steady_member_chain", "members", 100.0, 120.0),
            sample(
                "post_did_change_current_revision",
                "completion",
                100.0,
                120.0,
            ),
            sample("post_did_change_current_revision", "hover", 100.0, 120.0),
            sample(
                "post_did_change_current_revision",
                "definition",
                100.0,
                120.0,
            ),
            sample(
                "post_did_change_current_revision",
                "type_at_position",
                100.0,
                120.0,
            ),
            sample("post_did_change_current_revision", "members", 100.0, 120.0),
            sample("object_module_explicit_context", "completion", 100.0, 120.0),
            sample("object_module_explicit_context", "hover", 100.0, 120.0),
            sample("object_module_explicit_context", "definition", 100.0, 120.0),
            sample(
                "object_module_explicit_context",
                "type_at_position",
                100.0,
                120.0,
            ),
            sample("object_module_explicit_context", "members", 100.0, 120.0),
            sample(
                "recordset_module_explicit_context",
                "completion",
                100.0,
                120.0,
            ),
            sample("recordset_module_explicit_context", "hover", 100.0, 120.0),
            sample(
                "recordset_module_explicit_context",
                "definition",
                100.0,
                120.0,
            ),
            sample(
                "recordset_module_explicit_context",
                "type_at_position",
                100.0,
                120.0,
            ),
            sample("recordset_module_explicit_context", "members", 100.0, 120.0),
            sample(
                "incomplete_syntax_member_access",
                "completion",
                100.0,
                120.0,
            ),
        ]),
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
        &[sample("steady_member_chain", "completion", 301.0, 601.0)],
        Some(&[sample("steady_member_chain", "completion", 100.0, 120.0)]),
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
        &[sample("steady_member_chain", "completion", 100.0, 120.0)],
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
fn perf_gate_fails_when_required_matrix_entry_is_missing() {
    let evaluation = evaluate_intellisense_perf_profile(
        &contract_fixture(),
        "small",
        &[sample("steady_member_chain", "completion", 100.0, 120.0)],
        Some(&[sample("steady_member_chain", "completion", 100.0, 120.0)]),
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
        .any(|code| code == "missing_required_matrix_coverage"));
}

#[test]
fn perf_gate_fails_when_anti_rescue_counts_are_non_zero() {
    let mut anti_rescue_sample = sample("steady_member_chain", "completion", 100.0, 120.0);
    anti_rescue_sample.stale_served_total = 1;
    let evaluation = evaluate_intellisense_perf_profile(
        &contract_fixture(),
        "small",
        &[anti_rescue_sample],
        Some(&[sample("steady_member_chain", "completion", 100.0, 120.0)]),
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
        .any(|code| code == "anti_rescue_budget_exceeded"));
}

#[test]
fn perf_gate_fails_when_required_matrix_entry_is_hidden_fail_closed() {
    let mut current = required_matrix_samples(100.0, 120.0);
    let baseline = required_matrix_samples(100.0, 120.0);
    let hover_entry = sample_mut(&mut current, "steady_member_chain", "hover");
    hover_entry.fail_closed_total = 200;
    hover_entry.fail_closed_rate = 1.0;

    let evaluation = evaluate_intellisense_perf_profile(
        &contract_fixture(),
        "small",
        &current,
        Some(&baseline),
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
        .any(|code| code == "fail_closed_budget_exceeded"));
}

#[test]
fn perf_gate_uses_ratio_baseline_floors_for_sub_floor_jitter() {
    let mut current = required_matrix_samples(100.0, 120.0);
    let mut baseline = required_matrix_samples(100.0, 120.0);

    let baseline_entry = sample_mut(&mut baseline, "steady_member_chain", "completion");
    baseline_entry.total_duration_p95_ms = 0.5;
    baseline_entry.total_duration_p99_ms = 0.6;
    baseline_entry.wait_for_file_version_p95_ms = 0.2;
    baseline_entry.wait_for_file_version_p99_ms = 0.2;
    baseline_entry.snapshot_preparation_p95_ms = 0.2;
    baseline_entry.snapshot_preparation_p99_ms = 0.2;
    baseline_entry.ir_query_p95_ms = 0.05;
    baseline_entry.ir_query_p99_ms = 0.05;
    baseline_entry.lock_wait_ms_per_request = 0.05;

    let current_entry = sample_mut(&mut current, "steady_member_chain", "completion");
    current_entry.total_duration_p95_ms = 0.8;
    current_entry.total_duration_p99_ms = 0.9;
    current_entry.wait_for_file_version_p95_ms = 0.3;
    current_entry.wait_for_file_version_p99_ms = 0.3;
    current_entry.snapshot_preparation_p95_ms = 0.3;
    current_entry.snapshot_preparation_p99_ms = 0.3;
    current_entry.ir_query_p95_ms = 0.1;
    current_entry.ir_query_p99_ms = 0.1;
    current_entry.lock_wait_ms_per_request = 0.2;

    let evaluation = evaluate_intellisense_perf_profile(
        &contract_fixture(),
        "small",
        &current,
        Some(&baseline),
        thresholds(true),
    );

    assert_eq!(
        evaluation.get("verdict").and_then(Value::as_str),
        Some("pass")
    );
    let reason_codes = evaluation
        .get("reason_codes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!reason_codes
        .iter()
        .filter_map(Value::as_str)
        .any(|code| code == "latency_relative_ratio_exceeded"));
    assert!(!reason_codes
        .iter()
        .filter_map(Value::as_str)
        .any(|code| code == "lock_wait_budget_exceeded"));
}

#[test]
fn perf_gate_still_fails_when_floor_adjusted_ratio_is_exceeded() {
    let mut current = required_matrix_samples(100.0, 120.0);
    let mut baseline = required_matrix_samples(100.0, 120.0);

    let baseline_entry = sample_mut(&mut baseline, "steady_member_chain", "completion");
    baseline_entry.total_duration_p95_ms = 0.5;
    baseline_entry.total_duration_p99_ms = 0.6;
    baseline_entry.lock_wait_ms_per_request = 0.05;

    let current_entry = sample_mut(&mut current, "steady_member_chain", "completion");
    current_entry.total_duration_p95_ms = 7.0;
    current_entry.total_duration_p99_ms = 7.1;
    current_entry.lock_wait_ms_per_request = 1.3;

    let evaluation = evaluate_intellisense_perf_profile(
        &contract_fixture(),
        "small",
        &current,
        Some(&baseline),
        thresholds(true),
    );

    assert_eq!(
        evaluation.get("verdict").and_then(Value::as_str),
        Some("fail")
    );
    let reason_codes = evaluation
        .get("reason_codes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(reason_codes
        .iter()
        .filter_map(Value::as_str)
        .any(|code| code == "latency_relative_ratio_exceeded"));
    assert!(reason_codes
        .iter()
        .filter_map(Value::as_str)
        .any(|code| code == "lock_wait_budget_exceeded"));
}

#[test]
fn perf_gate_reports_but_does_not_block_on_p99_only_tail_jitter() {
    let mut current = required_matrix_samples(100.0, 120.0);
    let mut baseline = required_matrix_samples(100.0, 120.0);

    let baseline_entry = sample_mut(&mut baseline, "post_did_change_current_revision", "hover");
    baseline_entry.total_duration_p95_ms = 5.0;
    baseline_entry.total_duration_p99_ms = 5.5;
    baseline_entry.snapshot_preparation_p95_ms = 4.6;
    baseline_entry.snapshot_preparation_p99_ms = 4.8;

    let current_entry = sample_mut(&mut current, "post_did_change_current_revision", "hover");
    current_entry.total_duration_p95_ms = 5.1;
    current_entry.total_duration_p99_ms = 7.2;
    current_entry.snapshot_preparation_p95_ms = 4.9;
    current_entry.snapshot_preparation_p99_ms = 6.3;

    let evaluation = evaluate_intellisense_perf_profile(
        &contract_fixture(),
        "churn",
        &current,
        Some(&baseline),
        thresholds(true),
    );

    assert_eq!(
        evaluation.get("verdict").and_then(Value::as_str),
        Some("pass")
    );
    let entry = evaluation
        .get("entries")
        .and_then(Value::as_array)
        .and_then(|entries| {
            entries.iter().find(|entry| {
                entry.get("fixture_family").and_then(Value::as_str)
                    == Some("post_did_change_current_revision")
                    && entry.get("operation").and_then(Value::as_str) == Some("hover")
            })
        })
        .expect("hover entry");
    assert!(
        entry.get("latency")
            .and_then(|value| value.get("total_duration_ms"))
            .and_then(|value| value.get("ratio_p99"))
            .and_then(Value::as_f64)
            .is_some_and(|ratio| ratio > 1.15)
    );
    let reason_codes = evaluation
        .get("reason_codes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!reason_codes
        .iter()
        .filter_map(Value::as_str)
        .any(|code| code == "latency_relative_ratio_exceeded"));
}

#[test]
fn provenance_allows_legacy_local_without_expected_change_id() {
    let report = json!({
        "contract_version": "v2",
        "metrics": {}
    });

    validate_perf_report_provenance(&report, None).expect("legacy-local should pass");
}

#[test]
fn provenance_requires_change_id_when_expected_is_provided() {
    let report = json!({
        "contract_version": "v2",
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
fn provenance_accepts_optional_v2_provenance_payload() {
    let report = json!({
        "change_id": "refactor-v2-contract-first-hardening",
        "provenance": {
            "change_id": "refactor-v2-contract-first-hardening",
            "generated_at": "2026-03-03T19:00:00Z",
            "profile": "small",
            "schema_version": 1,
            "contract_version": "v2"
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

fn scale_aware_phase_with_counters(
    completion_p95: f64,
    wait_p95: f64,
    snapshot_p95: f64,
    ir_p95: f64,
    count: u64,
    wait_budget_exhausted_total: u64,
    stale_served_total: u64,
    stale_fallback_total: u64,
    fallback_unavailable_total: u64,
) -> Value {
    json!({
        "metrics": {
            "completion_duration_ms": { "p95": completion_p95, "count": count },
            "intellisense_v2_wait_for_file_version_completion_ms": { "p95": wait_p95, "count": count },
            "intellisense_v2_snapshot_completion_ms": { "p95": snapshot_p95, "count": count },
            "intellisense_v2_ir_query_completion_ms": { "p95": ir_p95, "count": count },
            "intellisense_v2_interactive_wait_budget_exhausted_total": wait_budget_exhausted_total,
            "intellisense_v2_interactive_stale_served_total": stale_served_total,
            "intellisense_v2_completion_stale_fallback_total": stale_fallback_total,
            "intellisense_v2_completion_fallback_unavailable_total": fallback_unavailable_total
        }
    })
}

fn scale_aware_phase(
    completion_p95: f64,
    wait_p95: f64,
    snapshot_p95: f64,
    ir_p95: f64,
    count: u64,
) -> Value {
    scale_aware_phase_with_counters(
        completion_p95,
        wait_p95,
        snapshot_p95,
        ir_p95,
        count,
        0,
        0,
        0,
        0,
    )
}

fn scale_aware_profile(phases: [Value; 3], warm_total: u64, warm_cancelled: u64) -> Value {
    let mut warm = phases[2].clone();
    warm["completion_total"] = json!(warm_total);
    warm["completion_cancelled_total"] = json!(warm_cancelled);

    json!({
        "start": phases[0],
        "cold": phases[1],
        "warm": warm
    })
}

fn scale_aware_report(
    large: [Value; 3],
    small: [Value; 3],
    large_total: u64,
    large_cancelled: u64,
    small_total: u64,
    small_cancelled: u64,
) -> Value {
    json!({
        "profiles": {
            "large": scale_aware_profile(large, large_total, large_cancelled),
            "small": scale_aware_profile(small, small_total, small_cancelled),
        }
    })
}

#[test]
fn scale_aware_gate_fails_closed_when_authoritative_report_contains_semantic_substitute_metrics() {
    let baseline = scale_aware_report(
        [
            scale_aware_phase(4200.0, 3200.0, 700.0, 320.0, 60),
            scale_aware_phase(4000.0, 3000.0, 680.0, 300.0, 80),
            scale_aware_phase(4000.0, 3000.0, 650.0, 280.0, 120),
        ],
        [
            scale_aware_phase(300.0, 8.0, 4.0, 180.0, 60),
            scale_aware_phase(280.0, 6.0, 3.0, 170.0, 80),
            scale_aware_phase(250.0, 5.0, 2.0, 160.0, 120),
        ],
        120,
        6,
        120,
        3,
    );
    let current = scale_aware_report(
        [
            scale_aware_phase(3100.0, 1800.0, 600.0, 260.0, 60),
            scale_aware_phase(2950.0, 1700.0, 560.0, 240.0, 80),
            scale_aware_phase_with_counters(2900.0, 1700.0, 540.0, 220.0, 120, 10, 1, 1, 4),
        ],
        [
            scale_aware_phase(300.0, 7.0, 3.0, 180.0, 60),
            scale_aware_phase(290.0, 6.0, 3.0, 170.0, 80),
            scale_aware_phase_with_counters(300.0, 5.0, 2.0, 165.0, 120, 10, 0, 0, 1),
        ],
        120,
        8,
        120,
        5,
    );

    let gate = evaluate_scale_aware_gate(&current, &baseline).expect("gate evaluation");
    assert_eq!(gate.get("pass").and_then(Value::as_bool), Some(false));
    assert_eq!(
        gate.get("anti_rescue_guard")
            .and_then(|value| value.get("pass"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        gate.get("anti_rescue_guard")
            .and_then(|value| value.get("semantic_substitute_detected"))
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn scale_aware_gate_keeps_fail_closed_miss_rate_as_diagnostic_without_marking_semantic_substitute()
{
    let baseline = scale_aware_report(
        [
            scale_aware_phase(4200.0, 3200.0, 700.0, 320.0, 60),
            scale_aware_phase(4000.0, 3000.0, 680.0, 300.0, 80),
            scale_aware_phase(4000.0, 3000.0, 650.0, 280.0, 120),
        ],
        [
            scale_aware_phase(300.0, 8.0, 4.0, 180.0, 60),
            scale_aware_phase(280.0, 6.0, 3.0, 170.0, 80),
            scale_aware_phase(250.0, 5.0, 2.0, 160.0, 120),
        ],
        120,
        6,
        120,
        3,
    );
    let current = scale_aware_report(
        [
            scale_aware_phase(3100.0, 1800.0, 600.0, 260.0, 60),
            scale_aware_phase(2950.0, 1700.0, 560.0, 240.0, 80),
            scale_aware_phase_with_counters(2900.0, 1700.0, 540.0, 220.0, 120, 10, 0, 0, 4),
        ],
        [
            scale_aware_phase(300.0, 7.0, 3.0, 180.0, 60),
            scale_aware_phase(290.0, 6.0, 3.0, 170.0, 80),
            scale_aware_phase_with_counters(300.0, 5.0, 2.0, 165.0, 120, 10, 0, 0, 1),
        ],
        120,
        8,
        120,
        5,
    );

    let gate = evaluate_scale_aware_gate(&current, &baseline).expect("gate evaluation");
    assert_eq!(gate.get("pass").and_then(Value::as_bool), Some(true));
    assert_eq!(
        gate.get("anti_rescue_guard")
            .and_then(|value| value.get("pass"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        gate.get("anti_rescue_guard")
            .and_then(|value| value.get("rates"))
            .and_then(|value| value.get("large_warm_fallback_unavailable_per_budget_exhausted"))
            .and_then(Value::as_f64),
        Some(0.4)
    );
}
