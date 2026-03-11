use super::*;
use crate::reporting::is_report_contract_version_compatible;
use serde_json::json;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

fn minimal_coverage() -> PerfCoverage {
    PerfCoverage {
        operation_coverage_mode: "representative_matrix".to_string(),
        reported_operations: vec!["completion".to_string()],
        reported_fixture_families: vec!["steady_member_chain".to_string()],
        reported_matrix_entries: 1,
        authoritative_for_cutover_acceptance: false,
    }
}

fn minimal_results() -> Vec<PerfResultEntry> {
    vec![PerfResultEntry {
        fixture_family: PerfFixtureFamily::SteadyMemberChain,
        operation: PerfOperation::Completion,
        cases: 1,
        total_requests: 1,
        fail_closed_total: 0,
        fail_closed_rate: 0.0,
        error_rate: 0.0,
        incomplete_rate: 0.0,
        metrics: PerfResultMetrics {
            total_duration_ms: PerfMetrics {
                count: 1,
                p50_ms: 1.0,
                p95_ms: 2.0,
                p99_ms: 3.0,
            },
            wait_for_file_version_ms: PerfMetrics {
                count: 1,
                p50_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
            },
            snapshot_preparation_ms: PerfMetrics {
                count: 1,
                p50_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
            },
            ir_query_ms: PerfMetrics {
                count: 1,
                p50_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
            },
            allocations_per_request: 10.0,
            allocated_bytes_per_request: 20.0,
            lock_wait_ms_per_request: 1.0,
            lock_contention_events_per_request: 1.0,
        },
        anti_rescue: PerfAntiRescueCounts::default(),
    }]
}

fn minimal_report(contract_version: &str) -> PerfReport {
    PerfReport {
        scenario: "small".to_string(),
        profile: "small".to_string(),
        cases: 1,
        iterations: 1,
        warmup: 0,
        change_id: None,
        provenance: None,
        contract_version: contract_version.to_string(),
        coverage: minimal_coverage(),
        results: minimal_results(),
        thresholds: None,
        verdict: "pass".to_string(),
        reason_codes: Vec::new(),
        pass: true,
        comparison: None,
    }
}

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
            operation: PerfOperation::Completion,
            fixture_family: PerfFixtureFamily::SteadyMemberChain,
        },
        PreparedCase {
            file_id: V2FileId(2),
            file_uri: file_uri.clone(),
            content: Arc::from("A"),
            line: 0,
            column: 1,
            operation: PerfOperation::Completion,
            fixture_family: PerfFixtureFamily::SteadyMemberChain,
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
    assert_eq!(state.plan.trigger_case_index, 1);
    assert_eq!(state.plan.target_file_ids, vec![V2FileId(1), V2FileId(2)]);
}

#[test]
fn churn_state_only_applies_at_configured_case_boundary() {
    let plan = ChurnPlan {
        every: 2,
        trigger_case_index: 3,
        target_file_uri: "/tmp/test.bsl".to_string(),
        target_file_path: Arc::from("/tmp/test.bsl"),
        target_file_ids: vec![V2FileId(1)],
        base_content: Arc::from("Процедура Тест()\nКонецПроцедуры\n"),
    };
    let state = ChurnRuntimeState::new(plan);

    assert!(!state.should_apply(0, 0));
    assert!(!state.should_apply(0, 2));
    assert!(state.should_apply(0, 3));
    assert!(!state.should_apply(0, 4));

    assert!(!state.should_apply(1, 3));
    assert!(state.should_apply(2, 3));
}

#[test]
fn read_scenario_parses_operation_and_fixture_family() {
    let dir = tempdir().expect("tempdir");
    let scenario_path = dir.path().join("scenario.json");
    fs::write(
        &scenario_path,
        r#"{
  "name": "small",
  "cases": [
    {
      "file": "examples/test_lsp.bsl",
      "marker": "Arr.Add",
      "label": "steady_hover",
      "operation": "hover",
      "fixture_family": "steady_member_chain"
    }
  ]
}"#,
    )
    .expect("write scenario");

    let scenario = read_scenario(&scenario_path).expect("read scenario");
    assert_eq!(scenario.cases.len(), 1);
    assert_eq!(scenario.cases[0].operation, PerfOperation::Hover);
    assert_eq!(
        scenario.cases[0].fixture_family,
        PerfFixtureFamily::SteadyMemberChain
    );
}

#[test]
fn report_contract_version_compatibility_allows_unknown_baseline_only() {
    assert!(is_report_contract_version_compatible("v2", "v2", "unknown"));
    assert!(is_report_contract_version_compatible("v2", "v2", "v2"));
    assert!(!is_report_contract_version_compatible("v2", "v1", "v2"));
    assert!(!is_report_contract_version_compatible("v2", "v2", "v1"));
}

#[test]
fn compare_reports_fails_with_unsupported_contract_version() {
    let contract = json!({
        "surface": "intellisense-perf-gate",
        "major_version": 2
    });
    let current = minimal_report("v2");
    let baseline = minimal_report("v1");
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
fn update_baseline_skips_existing_baseline_comparison() {
    assert!(should_compare_against_existing_baseline(true, false));
    assert!(!should_compare_against_existing_baseline(true, true));
    assert!(!should_compare_against_existing_baseline(false, false));
}

#[test]
fn provenance_failure_comparison_is_fail_closed() {
    let report = minimal_report("v2");
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

#[test]
fn perf_operation_maps_to_shared_runtime_semantic_operation_ids() {
    assert_eq!(
        PerfOperation::Completion.semantic_operation().as_str(),
        "completion"
    );
    assert_eq!(PerfOperation::Hover.semantic_operation().as_str(), "hover");
    assert_eq!(
        PerfOperation::Definition.semantic_operation().as_str(),
        "definition"
    );
    assert_eq!(
        PerfOperation::TypeAtPosition.semantic_operation().as_str(),
        "type_at_position"
    );
    assert_eq!(PerfOperation::Members.semantic_operation().as_str(), "members");
}

#[tokio::test]
async fn prime_runtime_files_unblocks_non_timeout_operations_without_churn() {
    let workspace_root = workspace_root();
    let prepared = prepare_cases(
        &[
            ScenarioCase {
                file: PathBuf::from("backend/tests/perf/fixtures/steady_member_chain.bsl"),
                marker: "ДляType = Массив".to_string(),
                label: Some("type_probe".to_string()),
                operation: PerfOperation::TypeAtPosition,
                fixture_family: PerfFixtureFamily::SteadyMemberChain,
            },
            ScenarioCase {
                file: PathBuf::from("backend/tests/perf/fixtures/steady_member_chain.bsl"),
                marker: "ДляMembers = Массив.".to_string(),
                label: Some("members_probe".to_string()),
                operation: PerfOperation::Members,
                fixture_family: PerfFixtureFamily::SteadyMemberChain,
            },
        ],
        &workspace_root,
    )
    .expect("prepare cases");
    let coordinator = Arc::new(SystemCoordinator::new());
    coordinator
        .start_with_paths_blocking(None, None, None, None)
        .expect("start system coordinator");
    let deps_bundle =
        build_deps_bundle_v2(&coordinator, None, None).expect("build_deps_bundle_v2");
    let settings = bsl_backend::application::ExecutionSettings {
        settings_id: SettingsId::from_hash("intellisense-perf-test"),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: settings.settings_id.clone(),
        diagnostics_detail_level: settings.diagnostics_detail_level,
    });
    let facade = bsl_backend::application::IntellisenseV2Facade::new(
        host,
        deps_bundle.index_snapshot.clone(),
        Some(coordinator.clone()),
    );
    prime_runtime_files(&facade, &prepared).await;

    for case in &prepared {
        let state = tokio::time::timeout(
            Duration::from_secs(2),
            facade.file_revision_state(case.file_id),
        )
        .await
        .expect("file revision state must not hang")
        .expect("file revision state must exist");
        assert_eq!(state.version, 0);
    }

    let mut content_by_file = build_content_by_file_map(&prepared);
    let version_by_file = build_file_version_map(&prepared);
    let resolver = deps_bundle
        .semantic_deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps_bundle.semantic_deps.repository.clone())));
    let metadata_lookup = TypeMetadataLookup::new(deps_bundle.semantic_deps.repository.clone());
    let context = IterationContext {
        facade: &facade,
        deps_id: &deps_bundle.deps_id,
        settings,
        coordinator: coordinator.as_ref(),
        metadata_lookup: &metadata_lookup,
        resolver: resolver.as_ref(),
    };

    for case in &prepared {
        tokio::time::timeout(
            Duration::from_secs(2),
            execute_case_iteration(&context, case, &mut content_by_file, &version_by_file),
        )
        .await
        .expect("operation must not hang")
        .expect("operation measurement");
    }
}
