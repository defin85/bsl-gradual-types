use super::*;
use crate::reporting::is_report_contract_version_compatible;
use serde_json::json;

#[test]
fn build_churned_content_switches_marker_without_growth() {
    let base = "Процедура Тест()\nКонецПроцедуры";
    let first = build_churned_content(base, 1);
    let second = build_churned_content(base, 2);

    assert!(first.contains("__intellisense_perf_churn_marker__ A"));
    assert!(second.contains("__intellisense_perf_churn_marker__ B"));
    assert_eq!(
        first.matches("__intellisense_perf_churn_marker__").count(),
        1
    );
    assert_eq!(
        second.matches("__intellisense_perf_churn_marker__").count(),
        1
    );
}

#[test]
fn build_churn_state_uses_target_case_file_for_all_matching_ids() {
    let file_uri = "/tmp/test.bsl".to_string();
    let cases = vec![
        PreparedCase {
            file_id: V2FileId(1),
            file_uri: file_uri.clone(),
            content: Arc::from("A"),
            line: 0,
            column: 0,
        },
        PreparedCase {
            file_id: V2FileId(2),
            file_uri: file_uri.clone(),
            content: Arc::from("A"),
            line: 0,
            column: 1,
        },
    ];
    let scenario = Scenario {
        name: "churn".to_string(),
        syntax_helper_path: None,
        config_path: None,
        platform_version: None,
        churn: Some(ScenarioChurn {
            every: 2,
            target_case: Some(1),
        }),
        cases: Vec::new(),
    };

    let state = build_churn_state(&scenario, &cases)
        .expect("churn state")
        .expect("churn enabled");
    assert_eq!(state.plan.every, 2);
    assert_eq!(state.plan.target_file_ids, vec![V2FileId(1), V2FileId(2)]);
}

#[test]
fn missing_resource_metric_keys_reports_absent_fields() {
    let report = json!({
        "metrics": {
            "allocations_per_completion": 10.0,
            "allocated_bytes_per_completion": 20.0
        }
    });

    let missing = missing_resource_metric_keys(&report);
    assert_eq!(
        missing,
        vec![
            "lock_wait_ms_per_completion",
            "lock_contention_events_per_completion"
        ]
    );
}

#[test]
fn missing_resource_metric_keys_accepts_complete_numeric_metrics() {
    let report = json!({
        "metrics": {
            "allocations_per_completion": 10.0,
            "allocated_bytes_per_completion": 20,
            "lock_wait_ms_per_completion": 1.5,
            "lock_contention_events_per_completion": 2
        }
    });

    let missing = missing_resource_metric_keys(&report);
    assert!(missing.is_empty());
}

#[test]
fn missing_required_metric_comparison_is_fail_closed() {
    let contract = json!({
        "surface": "intellisense-perf-gate",
        "major_version": 1
    });
    let report = PerfReport {
        scenario: "small".to_string(),
        cases: 1,
        iterations: 1,
        warmup: 0,
        change_id: None,
        provenance: None,
        contract_version: "v1".to_string(),
        metrics: PerfMetrics {
            total_requests: 1,
            count: 1,
            p50_ms: 1.0,
            p95_ms: 2.0,
            p99_ms: 3.0,
            error_rate: 0.0,
            incomplete_rate: 0.0,
            allocations_per_completion: 10.0,
            allocated_bytes_per_completion: 20.0,
            lock_wait_ms_per_completion: 1.0,
            lock_contention_events_per_completion: 1.0,
        },
        thresholds: None,
        comparison: None,
    };

    let comparison =
        build_missing_required_metric_comparison(&contract, &report, 1.10, 1.15, 1.20, 0.0, 0.0);

    assert_eq!(comparison.verdict, "fail");
    assert!(!comparison.pass);
    assert_eq!(
        comparison.reason_codes,
        vec!["missing_required_metric_field".to_string()]
    );
}

#[test]
fn report_contract_version_compatibility_allows_unknown_baseline_only() {
    assert!(is_report_contract_version_compatible("v1", "v1", "unknown"));
    assert!(is_report_contract_version_compatible("v1", "v1", "v1"));
    assert!(!is_report_contract_version_compatible("v1", "v2", "v1"));
    assert!(!is_report_contract_version_compatible("v1", "v1", "v2"));
}

#[test]
fn compare_reports_fails_with_unsupported_contract_version() {
    let contract = json!({
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
    });

    let current = PerfReport {
        scenario: "small".to_string(),
        cases: 1,
        iterations: 1,
        warmup: 0,
        change_id: None,
        provenance: None,
        contract_version: "v1".to_string(),
        metrics: PerfMetrics {
            total_requests: 1,
            count: 1,
            p50_ms: 1.0,
            p95_ms: 2.0,
            p99_ms: 3.0,
            error_rate: 0.0,
            incomplete_rate: 0.0,
            allocations_per_completion: 10.0,
            allocated_bytes_per_completion: 20.0,
            lock_wait_ms_per_completion: 1.0,
            lock_contention_events_per_completion: 1.0,
        },
        thresholds: None,
        comparison: None,
    };
    let baseline = PerfReport {
        contract_version: "v2".to_string(),
        ..current.clone()
    };

    let thresholds = PerfGateThresholds {
        latency_ratio_p95_max: 1.10,
        latency_ratio_p99_max: 1.15,
        resource_ratio_max: 1.20,
        max_error_rate: 0.0,
        max_incomplete_rate: 0.0,
        blocking_mode: true,
    };
    let comparison = compare_reports(&contract, "small", &current, &baseline, thresholds);
    assert_eq!(comparison.verdict, "fail");
    assert!(!comparison.pass);
    assert_eq!(
        comparison.reason_codes,
        vec!["unsupported_contract_version".to_string()]
    );
}

#[test]
fn resolve_expected_change_id_prefers_cli_over_env() {
    let from_cli =
        resolve_expected_change_id_from_sources(Some("cli-change-id"), Some("env-change-id"));
    assert_eq!(from_cli.as_deref(), Some("cli-change-id"));

    let from_env = resolve_expected_change_id_from_sources(None, Some("env-change-id"));
    assert_eq!(from_env.as_deref(), Some("env-change-id"));
}

#[test]
fn cutover_context_requires_authoritative_change_id_unless_updating_baseline() {
    assert!(requires_authoritative_evidence_context(true, false, false));
    assert!(requires_authoritative_evidence_context(false, false, true));
    assert!(!requires_authoritative_evidence_context(true, true, false));
    assert!(!requires_authoritative_evidence_context(
        false, false, false
    ));
}

#[test]
fn provenance_failure_comparison_is_fail_closed() {
    let report = PerfReport {
        scenario: "small".to_string(),
        cases: 1,
        iterations: 1,
        warmup: 0,
        change_id: Some("refactor-v2-contract-first-hardening".to_string()),
        provenance: None,
        contract_version: "v1".to_string(),
        metrics: PerfMetrics {
            total_requests: 1,
            count: 1,
            p50_ms: 1.0,
            p95_ms: 2.0,
            p99_ms: 3.0,
            error_rate: 0.0,
            incomplete_rate: 0.0,
            allocations_per_completion: 10.0,
            allocated_bytes_per_completion: 20.0,
            lock_wait_ms_per_completion: 1.0,
            lock_contention_events_per_completion: 1.0,
        },
        thresholds: None,
        comparison: None,
    };
    let thresholds = PerfGateThresholds {
        latency_ratio_p95_max: 1.10,
        latency_ratio_p99_max: 1.15,
        resource_ratio_max: 1.20,
        max_error_rate: 0.0,
        max_incomplete_rate: 0.0,
        blocking_mode: true,
    };

    let comparison = build_provenance_failure_comparison(
        &report,
        "provenance_missing_for_authoritative_run",
        thresholds,
    );
    assert_eq!(comparison.verdict, "fail");
    assert!(!comparison.pass);
    assert_eq!(
        comparison.reason_codes,
        vec!["provenance_missing_for_authoritative_run".to_string()]
    );
}
