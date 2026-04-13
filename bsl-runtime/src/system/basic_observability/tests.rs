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
fn sidebar_metrics_export_filters_histograms_to_sidebar_subset() {
    let observability = BasicObservability::default();
    observability.record_completion_latency(Duration::from_millis(33));
    observability.record_intellisense_v2_ir_query_latency("completion", Duration::from_millis(21));

    let exported = observability.get_metrics().export_metrics_sidebar();
    let exported_histograms = histograms(&exported);

    assert!(
        exported_histograms.contains_key("intellisense_v2_ir_query_completion_ms"),
        "sidebar export must keep key observability latency histograms"
    );
    assert!(
        !exported_histograms.contains_key("completion_duration_ms"),
        "sidebar export must skip unrelated histogram summaries to stay lightweight"
    );
    let config = exported
        .get("config")
        .and_then(|value| value.as_object())
        .expect("sidebar export must expose config object");
    assert!(
        config.contains_key("BSL_INTELLISENSE_V2_INTERACTIVE_WAIT_BUDGET_MS"),
        "sidebar export must keep effective interactive wait budget"
    );
}

#[test]
fn parse_snapshot_export_uses_only_canonical_fallback_buckets() {
    let observability = BasicObservability::default();
    observability.record_intellisense_v2_parse_snapshot(
        "lsp",
        "full",
        0,
        0,
        Some("incremental_parse_failed"),
        Duration::from_millis(12),
    );

    let exported = observability.get_metrics().export_metrics();
    let exported_counters = counters(&exported);

    assert_eq!(
        counter_value(
            exported_counters,
            "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_incremental_parse_failed"
        ),
        1,
        "canonical incremental-parse failure bucket must be exported"
    );
    assert!(
        !exported_counters.contains_key(
            "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_incremental_failed"
        ),
        "legacy generic incremental_failed bucket must not remain pre-registered in the export"
    );

    observability.record_intellisense_v2_parse_snapshot(
        "lsp",
        "full",
        0,
        0,
        Some("stale_parser_base"),
        Duration::from_millis(7),
    );
    let exported = observability.get_metrics().export_metrics();
    let exported_counters = counters(&exported);
    assert_eq!(
        counter_value(
            exported_counters,
            "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_stale_parser_base"
        ),
        1,
        "stale-parser-base fallback bucket must be exported explicitly instead of collapsing into other"
    );
}

#[test]
fn completion_exact_wait_and_semantic_breakdown_metrics_are_recorded() {
    let observability = BasicObservability::default();
    observability
        .record_intellisense_v2_completion_exact_type_index_wait_outcome("no_matching_task");
    observability.record_intellisense_v2_completion_exact_type_index_wait_promotion();
    observability.record_intellisense_v2_completion_exact_type_index_wait_join();
    observability.record_intellisense_v2_completion_exact_type_index_wait_ready_after_wait();
    observability
        .record_completion_stage_latency("prepare_apply_age_at_start", Duration::from_millis(9));
    observability.record_completion_stage_latency(
        "prepare_apply_age_at_terminal",
        Duration::from_millis(13),
    );
    observability.record_completion_stage_latency(
        "exact_wait_apply_age_at_start",
        Duration::from_millis(17),
    );
    observability.record_completion_stage_latency(
        "exact_wait_apply_age_at_terminal",
        Duration::from_millis(21),
    );
    observability.record_intellisense_v2_semantic_diagnostics_query_breakdown(
        Duration::from_millis(3),
        Duration::from_millis(5),
        Duration::from_millis(7),
        Duration::from_millis(11),
        Some(Duration::from_millis(13)),
    );

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    let histograms = histograms(&exported);

    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task"
        ),
        1,
        "exact wait outcome counter must be exported under bounded reason labels"
    );
    for key in [
        "intellisense_v2_completion_exact_type_index_wait_promotion_total",
        "intellisense_v2_completion_exact_type_index_wait_join_total",
        "intellisense_v2_completion_exact_type_index_wait_ready_after_wait_total",
    ] {
        assert_eq!(
            counter_value(counters, key),
            1,
            "expected counter key {key} in observability export"
        );
    }
    for key in [
        "completion_stage_prepare_apply_age_at_start_ms",
        "completion_stage_prepare_apply_age_at_terminal_ms",
        "completion_stage_exact_wait_apply_age_at_start_ms",
        "completion_stage_exact_wait_apply_age_at_terminal_ms",
        "intellisense_v2_semantic_diagnostics_query_inputs_ms",
        "intellisense_v2_semantic_diagnostics_query_parse_result_ms",
        "intellisense_v2_semantic_diagnostics_query_ir_ms",
        "intellisense_v2_semantic_diagnostics_query_collect_ms",
        "intellisense_v2_semantic_diagnostics_query_flow_sensitive_ms",
    ] {
        assert!(
            histograms.contains_key(key),
            "expected histogram key {key} in observability export"
        );
    }
}

#[test]
fn completion_route_fail_closed_cause_and_upgrade_metrics_are_recorded() {
    let observability = BasicObservability::default();
    observability.record_intellisense_v2_completion_route("head_hit");
    observability.record_intellisense_v2_completion_route("exact_hit");
    observability.record_intellisense_v2_completion_fail_closed_cause("prepare_timeout");
    observability.record_intellisense_v2_completion_fail_closed_cause("exact_deadline");
    observability
        .record_intellisense_v2_completion_head_to_exact_upgrade(Duration::from_millis(17));

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    let histograms = histograms(&exported);

    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_completion_route_total_route_head_hit"
        ),
        1,
        "head-hit completion route must be exported under bounded labels"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_completion_route_total_route_exact_hit"
        ),
        1,
        "exact-hit completion route must be exported under bounded labels"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout"
        ),
        1,
        "prepare-timeout completion fail-closed cause must be exported under bounded labels"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline"
        ),
        1,
        "exact-deadline completion fail-closed cause must be exported under bounded labels"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_completion_head_to_exact_upgrade_total"
        ),
        1,
        "head-to-exact upgrade counter must be exported"
    );
    assert_eq!(
        histogram_count(
            histograms,
            "intellisense_v2_completion_head_to_exact_upgrade_ms"
        ),
        1,
        "head-to-exact upgrade latency histogram must be exported"
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
    observability.record_intellisense_v2_parse_result_query_latency_with_origin_operation_and_mode(
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
fn syntax_diagnostics_stage_metrics_include_parse_mode_dimension_and_keep_projection_parity() {
    let observability = BasicObservability::default();
    observability.record_intellisense_v2_syntax_diagnostics_query_latency_with_origin_and_mode(
        "lsp",
        "incremental",
        Duration::from_millis(14),
    );

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    let histograms = histograms(&exported);

    let drilldown_counter = "intellisense_v2_drilldown_stage_total_origin_lsp_mode_incremental_operation_diagnostics_stage_syntax_diagnostics_query";
    let drilldown_histogram = "intellisense_v2_drilldown_stage_latency_ms_origin_lsp_mode_incremental_operation_diagnostics_stage_syntax_diagnostics_query";
    assert_eq!(counter_value(counters, drilldown_counter), 1);
    assert_eq!(histogram_count(histograms, drilldown_histogram), 1);
    assert_eq!(
        counter_value(counters, "intellisense_v2_syntax_diagnostics_query_total"),
        counter_value(counters, drilldown_counter),
        "syntax_diagnostics legacy total must stay in deterministic projection parity",
    );
    assert_eq!(
        histogram_count(histograms, "intellisense_v2_syntax_diagnostics_query_ms"),
        histogram_count(histograms, drilldown_histogram),
        "syntax_diagnostics legacy latency must stay in deterministic projection parity",
    );
}

#[test]
fn syntax_diagnostics_parse_mode_dimension_normalizes_unknown_values_to_other() {
    let observability = BasicObservability::default();
    observability.record_intellisense_v2_syntax_diagnostics_query_latency_with_origin_and_mode(
        "web",
        "unknown-mode",
        Duration::from_millis(9),
    );

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);

    let normalized_key = "intellisense_v2_drilldown_stage_total_origin_web_mode_other_operation_diagnostics_stage_syntax_diagnostics_query";
    assert_eq!(
        counter_value(counters, normalized_key),
        1,
        "unknown parse mode must collapse into bounded syntax mode label set",
    );
    assert!(
        !counters
            .keys()
            .any(|key| key.contains("_mode_unknown-mode")),
        "unexpected syntax mode labels must not leak into drilldown metrics",
    );
}

#[test]
fn stage_aware_mode_schema_rejects_mixed_parse_and_completion_modes() {
    let observability = BasicObservability::default();

    observability.emit_canonical_event(
        CanonicalEvent {
            family: CanonicalFamily::StageTotal,
            origin: "lsp",
            mode: Some("legacy"),
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
        None,
    );
    observability.emit_canonical_event(
        CanonicalEvent {
            family: CanonicalFamily::StageTotal,
            origin: "lsp",
            mode: Some("incremental"),
            operation: Some("completion"),
            stage: Some("ir_query"),
            outcome: None,
            reason: None,
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

    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_observability_contract_violation_total"
        ),
        2,
        "invalid stage/mode combinations must be rejected by schema validation",
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_drilldown_stage_total_origin_lsp_mode_legacy_operation_diagnostics_stage_syntax_diagnostics_query"
        ),
        0,
        "completion mode labels must not leak into syntax diagnostics stage",
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_drilldown_stage_total_origin_lsp_mode_incremental_operation_completion_stage_ir_query"
        ),
        0,
        "parse mode labels must not leak into completion stage metrics",
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
    observability.record_intellisense_v2_runtime_queue_wait_latency_with_origin(
        "lsp",
        "apply_changes_batch",
        Duration::from_millis(6),
    );
    observability.record_intellisense_v2_runtime_queue_wait_latency_with_origin(
        "lsp",
        "type_index_precompute",
        Duration::from_millis(5),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "snapshot_with_deps",
        Duration::from_millis(9),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "apply_changes_batch",
        Duration::from_millis(10),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "apply_change_set_file",
        Duration::from_millis(3),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "type_index_precompute",
        Duration::from_millis(8),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "type_index_precompute_build",
        Duration::from_millis(4),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "type_index_precompute_ir",
        Duration::from_millis(7),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "type_index_precompute_ast_to_ir",
        Duration::from_millis(5),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "type_index_precompute_semantic_facts",
        Duration::from_millis(3),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "type_index_precompute_semantic_facts_seed_module_context",
        Duration::from_millis(1),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "type_index_precompute_semantic_facts_local_function_summaries",
        Duration::from_millis(2),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "type_index_precompute_semantic_facts_visit_statements",
        Duration::from_millis(9),
    );
    observability.record_intellisense_v2_runtime_apply_changes_batch_size(4);
    observability.record_intellisense_v2_runtime_apply_changes_changed_files_count(2);

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
        counter_value(
            counters,
            "intellisense_v2_runtime_apply_changes_queue_wait_total"
        ) > 0,
        "legacy apply-changes queue wait counter should be projected"
    );
    assert!(
        counter_value(counters, "intellisense_v2_runtime_apply_changes_exec_total") > 0,
        "legacy apply-changes batch exec counter should be projected"
    );
    assert!(
        counter_value(
            counters,
            "intellisense_v2_runtime_type_index_precompute_queue_wait_total"
        ) > 0,
        "type_index precompute queue wait must not be projected into runtime_other_*"
    );
    assert!(
        counter_value(
            counters,
            "intellisense_v2_runtime_type_index_precompute_exec_total"
        ) > 0,
        "type_index precompute exec must not be projected into runtime_other_*"
    );
    assert!(
        counter_value(
            counters,
            "intellisense_v2_runtime_type_index_precompute_build_exec_total"
        ) > 0,
        "type_index precompute build exec must not be projected into runtime_other_*"
    );
    assert!(
        counter_value(
            counters,
            "intellisense_v2_runtime_type_index_precompute_ir_exec_total"
        ) > 0,
        "type_index precompute IR exec must not be projected into runtime_other_*"
    );
    assert!(
        counter_value(
            counters,
            "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_total"
        ) > 0,
        "type_index precompute AST->IR exec must not be projected into runtime_other_*"
    );
    assert!(
        counter_value(
            counters,
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_total"
        ) > 0,
        "type_index precompute semantic-facts exec must not be projected into runtime_other_*"
    );
    assert!(
        counter_value(
            counters,
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_total"
        ) > 0,
        "type_index precompute semantic-facts seed-module-context exec must not be projected into runtime_other_*"
    );
    assert!(
        counter_value(
            counters,
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_total"
        ) > 0,
        "type_index precompute semantic-facts local-function-summaries exec must not be projected into runtime_other_*"
    );
    assert!(
        counter_value(
            counters,
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_total"
        ) > 0,
        "type_index precompute semantic-facts visit-statements exec must not be projected into runtime_other_*"
    );
    assert!(
        counter_value(
            counters,
            "intellisense_v2_runtime_apply_change_set_file_exec_total"
        ) > 0,
        "legacy apply-change set_file exec counter should be projected"
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
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_apply_changes_queue_wait_ms"
        ) > 0,
        "legacy apply-changes queue wait histogram should be projected"
    );
    assert!(
        histogram_count(histograms, "intellisense_v2_runtime_apply_changes_exec_ms") > 0,
        "legacy apply-changes batch exec histogram should be projected"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_type_index_precompute_queue_wait_ms"
        ) > 0,
        "type_index precompute queue wait histogram must be projected to dedicated metric"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_type_index_precompute_exec_ms"
        ) > 0,
        "type_index precompute exec histogram must be projected to dedicated metric"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_type_index_precompute_build_exec_ms"
        ) > 0,
        "type_index precompute build exec histogram must be projected to dedicated metric"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_type_index_precompute_ir_exec_ms"
        ) > 0,
        "type_index precompute IR exec histogram must be projected to dedicated metric"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms"
        ) > 0,
        "type_index precompute AST->IR exec histogram must be projected to dedicated metric"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms"
        ) > 0,
        "type_index precompute semantic-facts exec histogram must be projected to dedicated metric"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms"
        ) > 0,
        "type_index precompute semantic-facts seed-module-context exec histogram must be projected to dedicated metric"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms"
        ) > 0,
        "type_index precompute semantic-facts local-function-summaries exec histogram must be projected to dedicated metric"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms"
        ) > 0,
        "type_index precompute semantic-facts visit-statements exec histogram must be projected to dedicated metric"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_apply_change_set_file_exec_ms"
        ) > 0,
        "legacy apply-change set_file exec histogram should be projected"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_apply_changes_batch_size"
        ) > 0,
        "apply-changes batch-size histogram should be projected"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_apply_changes_changed_files_count"
        ) > 0,
        "apply-changes changed-files histogram should be projected"
    );
}

#[test]
fn runtime_stage_registry_and_projection_contract_require_explicit_updates() {
    let registry_stage_kinds: BTreeSet<&str> = RUNTIME_STAGE_KIND_REGISTRY
        .iter()
        .map(|(raw, _normalized)| *raw)
        .collect();
    let expected_registry_stage_kinds: BTreeSet<&str> = [
        "wait_for_file_version",
        "snapshot_with_deps",
        "apply_changes_batch",
        "apply_change_set_file",
        "apply_change_set_file_with_snapshot",
        "apply_change_remove_file",
        "apply_change_set_settings_snapshot",
        "type_index_precompute",
        "type_index_precompute_build",
        "type_index_precompute_ir",
        "type_index_precompute_ast_to_ir",
        "type_index_precompute_semantic_facts",
        "type_index_precompute_semantic_facts_seed_module_context",
        "type_index_precompute_semantic_facts_local_function_summaries",
        "type_index_precompute_semantic_facts_visit_statements",
    ]
    .into_iter()
    .collect();
    assert_eq!(
            registry_stage_kinds, expected_registry_stage_kinds,
            "runtime stage taxonomy changed; update registry/projection tests and contract mappings explicitly"
        );

    let queue_projection_stage_kinds: BTreeSet<&str> = LEGACY_RUNTIME_QUEUE_WAIT_METRICS_REGISTRY
        .iter()
        .map(|(raw, _metrics)| *raw)
        .collect();
    let expected_queue_projection_stage_kinds: BTreeSet<&str> = [
        "snapshot_with_deps",
        "wait_for_file_version",
        "apply_changes_batch",
        "type_index_precompute",
    ]
    .into_iter()
    .collect();
    assert_eq!(
        queue_projection_stage_kinds, expected_queue_projection_stage_kinds,
        "queue-stage projection mapping changed; update dedicated legacy keys explicitly"
    );

    let exec_projection_stage_kinds: BTreeSet<&str> = LEGACY_RUNTIME_EXEC_METRICS_REGISTRY
        .iter()
        .map(|(raw, _metrics)| *raw)
        .collect();
    let expected_exec_projection_stage_kinds: BTreeSet<&str> = [
        "snapshot_with_deps",
        "wait_for_file_version",
        "apply_changes_batch",
        "apply_change_set_file",
        "apply_change_set_file_with_snapshot",
        "apply_change_remove_file",
        "apply_change_set_settings_snapshot",
        "type_index_precompute",
        "type_index_precompute_build",
        "type_index_precompute_ir",
        "type_index_precompute_ast_to_ir",
        "type_index_precompute_semantic_facts",
        "type_index_precompute_semantic_facts_seed_module_context",
        "type_index_precompute_semantic_facts_local_function_summaries",
        "type_index_precompute_semantic_facts_visit_statements",
    ]
    .into_iter()
    .collect();
    assert_eq!(
        exec_projection_stage_kinds, expected_exec_projection_stage_kinds,
        "exec-stage projection mapping changed; update dedicated legacy keys explicitly"
    );

    for stage in &registry_stage_kinds {
        assert!(
            ALLOWED_OPERATIONS
                .iter()
                .any(|operation| operation == stage),
            "runtime stage '{stage}' must be present in allowed operation taxonomy"
        );
    }

    for stage in &queue_projection_stage_kinds {
        let (counter_key, histogram_key) = legacy_runtime_queue_wait_metrics(stage);
        assert!(
            !counter_key.contains("runtime_other") && !histogram_key.contains("runtime_other"),
            "queue stage '{stage}' must map to dedicated metrics, not runtime_other_*"
        );
    }

    for stage in &exec_projection_stage_kinds {
        let (counter_key, histogram_key) = legacy_runtime_exec_metrics(stage);
        assert!(
            !counter_key.contains("runtime_other") && !histogram_key.contains("runtime_other"),
            "exec stage '{stage}' must map to dedicated metrics, not runtime_other_*"
        );
    }
}

#[test]
fn interactive_fail_closed_reason_metrics_are_exported_with_bounded_reasons() {
    let contract = contract_json("observability-completion-v2/v3/contract.json");
    let metrics_contract = contract
        .get("metrics")
        .and_then(|value| value.as_object())
        .expect("metrics contract section");
    let reason_prefix = metrics_contract
        .get("interactive_fail_closed_reason_counter_prefix")
        .and_then(|value| value.as_str())
        .expect("interactive fail-closed reason counter prefix");
    assert_eq!(
        reason_prefix,
        "intellisense_v2_fail_closed_reason_total_origin_"
    );
    let contract_reasons: Vec<String> = metrics_contract
        .get("allowed_fail_closed_reasons")
        .and_then(|value| value.as_array())
        .expect("allowed_fail_closed_reasons")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("allowed fail-closed reason string")
                .to_string()
        })
        .collect();

    let observability = BasicObservability::default();
    for reason in &contract_reasons {
        assert_eq!(
            normalize_shared_fail_closed_reason_label(reason),
            reason,
            "contract fail-closed reason must stay in bounded normalization set"
        );
        observability.record_intellisense_v2_interactive_fail_closed_reason(
            "lsp",
            "completion",
            reason,
        );
    }
    observability.record_intellisense_v2_interactive_fail_closed_reason(
        "lsp",
        "completion",
        "unexpected_reason",
    );

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    for reason in &contract_reasons {
        let key = format!("{reason_prefix}lsp_operation_completion_reason_{reason}");
        assert!(
            counter_value(counters, &key) > 0,
            "fail-closed reason counter must be exported for {reason}"
        );
    }
    let other_key = format!("{reason_prefix}lsp_operation_completion_reason_other");
    assert!(
        counter_value(counters, &other_key) > 0,
        "fail-closed reason counter must collapse unknown reasons into other"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_observability_contract_violation_total",
        ),
        1,
        "unknown type-index reason must emit observability contract violation"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_observability_contract_violation_reason_unknown_fail_closed_reason",
        ),
        1,
        "unknown fail-closed reason must emit dedicated contract-violation reason"
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
fn diagnostics_pipeline_disabled_by_config_stays_non_cancellation() {
    let observability = BasicObservability::default();
    observability.record_intellisense_v2_diagnostics_pipeline_event(
        "lsp",
        "did_save",
        "idle_heavy",
        "disabled_by_config",
    );

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    let histograms = histograms(&exported);
    let key = "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_did_save_profile_idle_heavy_reason_disabled_by_config";
    let cancel_histogram_key = "intellisense_v2_diagnostics_pipeline_cancel_sample_origin_lsp_trigger_did_save_profile_idle_heavy_reason_disabled_by_config";
    assert_eq!(
        counter_value(counters, key),
        1,
        "disabled_by_config must be exported as a first-class diagnostics pipeline reason"
    );
    assert_eq!(
        histogram_count(histograms, cancel_histogram_key),
        0,
        "disabled_by_config must not be classified as a cancellation outcome"
    );
}

#[test]
fn runtime_lane_metrics_export_dedicated_lane_family() {
    let observability = BasicObservability::default();
    observability.record_intellisense_v2_runtime_lane_queue_wait_latency_with_origin(
        "lsp",
        "did_save_followup",
        Duration::from_millis(7),
    );
    observability.record_intellisense_v2_runtime_lane_exec_latency_with_origin(
        "lsp",
        "did_save_followup",
        Duration::from_millis(11),
    );
    observability.record_intellisense_v2_runtime_lane_saturation_gauge_with_origin(
        "lsp",
        "did_save_followup",
        "quota",
        1.0,
    );
    observability.record_intellisense_v2_runtime_lane_saturation_gauge_with_origin(
        "lsp",
        "did_save_followup",
        "active_slots",
        0.0,
    );
    observability.record_intellisense_v2_runtime_lane_saturation_gauge_with_origin(
        "lsp",
        "did_save_followup",
        "queue_depth",
        2.0,
    );

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    let histograms = histograms(&exported);
    let gauges = gauges(&exported);

    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_runtime_lane_queue_wait_total_origin_lsp_lane_did_save_followup"
        ),
        1,
        "dedicated lane queue-wait counter must be exported under canonical lane identity"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_runtime_lane_exec_total_origin_lsp_lane_did_save_followup"
        ),
        1,
        "dedicated lane exec counter must be exported under canonical lane identity"
    );
    assert_eq!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_lane_queue_wait_ms_origin_lsp_lane_did_save_followup"
        ),
        1,
        "dedicated lane queue-wait histogram must be exported"
    );
    assert_eq!(
        histogram_count(
            histograms,
            "intellisense_v2_runtime_lane_exec_ms_origin_lsp_lane_did_save_followup"
        ),
        1,
        "dedicated lane exec histogram must be exported"
    );
    assert_eq!(
        gauges
            .get("intellisense_v2_runtime_lane_saturation_gauge_origin_lsp_lane_did_save_followup_metric_quota")
            .and_then(|value| value.as_f64()),
        Some(1.0),
        "quota gauge must be exported for dedicated runtime lane"
    );
    assert_eq!(
        gauges
            .get("intellisense_v2_runtime_lane_saturation_gauge_origin_lsp_lane_did_save_followup_metric_active_slots")
            .and_then(|value| value.as_f64()),
        Some(0.0),
        "active_slots gauge must be exported for dedicated runtime lane"
    );
    assert_eq!(
        gauges
            .get("intellisense_v2_runtime_lane_saturation_gauge_origin_lsp_lane_did_save_followup_metric_queue_depth")
            .and_then(|value| value.as_f64()),
        Some(2.0),
        "queue_depth gauge must be exported for dedicated runtime lane"
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
fn diagnostics_pipeline_publish_latency_uses_bounded_dimensions() {
    let observability = BasicObservability::default();
    observability.record_intellisense_v2_diagnostics_pipeline_publish_latency(
        "lsp",
        "did_save",
        "save_fastlane",
        Duration::from_millis(42),
    );

    let exported = observability.get_metrics().export_metrics();
    let histograms = histograms(&exported);
    let key =
        "intellisense_v2_diagnostics_pipeline_publish_ms_origin_lsp_trigger_did_save_profile_save_fastlane";
    assert!(
        histogram_count(histograms, key) > 0,
        "publish latency histogram must be exported with canonical trigger/profile dimensions"
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
    observability.record_intellisense_v2_singleflight_leader_with_origin("agent", "parse_result");

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
fn completion_outcome_exports_fail_closed_for_legacy_fail_closed_label() {
    let observability = BasicObservability::default();
    observability.record_intellisense_v2_completion_outcome("fallback_unavailable");

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_completion_result_total_fail_closed"
        ),
        1,
        "legacy fail-closed outcome must collapse into public fail_closed class"
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
fn completion_v2_contract_matches_runtime_transport_and_trigger_modes() {
    let contract = contract_json("lsp-completion-v2/v2/contract.json");
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

    let transport_outcomes: BTreeSet<String> = completion
        .get("transport_outcomes")
        .and_then(|value| value.as_array())
        .expect("transport_outcomes array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("transport outcome string")
                .to_string()
        })
        .collect();
    let expected_transport_outcomes: BTreeSet<String> = ["ok_non_empty", "ok_empty"]
        .iter()
        .map(|value| value.to_string())
        .collect();
    assert_eq!(
        transport_outcomes, expected_transport_outcomes,
        "contract transport outcomes must match canonical/fail-closed completion transport baseline"
    );

    for outcome in &transport_outcomes {
        assert!(
            expected_transport_outcomes.contains(outcome),
            "contract must include transport outcome {outcome}"
        );
    }
}

#[test]
fn completion_timeline_v8_contract_matches_current_runtime_payload_shape() {
    let contract = contract_json("lsp-completion-timeline/v8/contract.json");
    let response = contract
        .get("response")
        .and_then(|value| value.as_object())
        .expect("response contract section");

    assert_eq!(
        response
            .get("version")
            .and_then(|value| value.as_u64())
            .expect("response.version"),
        11,
        "timeline contract must match current runtime response.version"
    );

    let trace_fields: BTreeSet<String> = response
        .get("trace_fields")
        .and_then(|value| value.as_array())
        .expect("trace_fields array")
        .iter()
        .map(|value| value.as_str().expect("trace field string").to_string())
        .collect();
    let expected_trace_fields: BTreeSet<String> = [
        "trace_id",
        "request_id",
        "uri",
        "trigger_mode",
        "outcome",
        "started_at_ms",
        "total_duration_ms",
        "dominant_stage",
        "prepare_details",
        "server_edge_details",
        "turn_attribution",
        "stages",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect();
    assert_eq!(trace_fields, expected_trace_fields);

    let prepare_details_fields: BTreeSet<String> = response
        .get("prepare_details_fields")
        .and_then(|value| value.as_array())
        .expect("prepare_details_fields array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("prepare_details field string")
                .to_string()
        })
        .collect();
    let expected_prepare_details_fields: BTreeSet<String> = [
        "wait_budget_ms",
        "guard_outcome",
        "outcome",
        "route",
        "fail_closed_cause",
        "min_file_version",
        "shadow_version_at_start",
        "observed_file_version",
        "wait_elapsed_ms",
        "snapshot_elapsed_ms",
        "apply_age_at_start_ms",
        "apply_age_at_terminal_ms",
        "progress",
        "wait_for_file_version_runtime",
        "snapshot_with_deps_runtime",
        "snapshot_with_deps_timeout_runtime",
        "timeout_attribution",
        "exact_wait",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect();
    assert_eq!(prepare_details_fields, expected_prepare_details_fields);

    let prepare_progress_fields: BTreeSet<String> = response
        .get("prepare_progress_fields")
        .and_then(|value| value.as_array())
        .expect("prepare_progress_fields array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("prepare_progress field string")
                .to_string()
        })
        .collect();
    let expected_prepare_progress_fields: BTreeSet<String> = [
        "phase",
        "phase_started_offset_ms",
        "wait_completed_offset_ms",
        "snapshot_completed_offset_ms",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect();
    assert_eq!(prepare_progress_fields, expected_prepare_progress_fields);

    let prepare_runtime_fields: BTreeSet<String> = response
        .get("prepare_runtime_fields")
        .and_then(|value| value.as_array())
        .expect("prepare_runtime_fields array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("prepare_runtime field string")
                .to_string()
        })
        .collect();
    let expected_prepare_runtime_fields: BTreeSet<String> =
        ["queue_wait_ms", "exec_ms", "wake_wait_ms", "resolution"]
            .iter()
            .map(|value| value.to_string())
            .collect();
    assert_eq!(prepare_runtime_fields, expected_prepare_runtime_fields);

    let prepare_timeout_attribution_fields: BTreeSet<String> = response
        .get("prepare_timeout_attribution_fields")
        .and_then(|value| value.as_array())
        .expect("prepare_timeout_attribution_fields array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("prepare_timeout_attribution field string")
                .to_string()
        })
        .collect();
    let expected_prepare_timeout_attribution_fields: BTreeSet<String> =
        ["source", "phase", "budget_ms", "elapsed_ms", "overshoot_ms"]
            .iter()
            .map(|value| value.to_string())
            .collect();
    assert_eq!(
        prepare_timeout_attribution_fields,
        expected_prepare_timeout_attribution_fields
    );

    let exact_wait_fields: BTreeSet<String> = response
        .get("exact_wait_fields")
        .and_then(|value| value.as_array())
        .expect("exact_wait_fields array")
        .iter()
        .map(|value| value.as_str().expect("exact_wait field string").to_string())
        .collect();
    let expected_exact_wait_fields: BTreeSet<String> = [
        "head_ready_before_wait",
        "exact_ready_before_wait",
        "current_revision_head_owner_hints_ready",
        "artifact_wait_outcome",
        "type_index_wait_outcome",
        "type_index_waiter_action",
        "matching_task_state",
        "task_phase",
        "artifact_poll",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect();
    assert_eq!(exact_wait_fields, expected_exact_wait_fields);

    let exact_artifact_poll_fields: BTreeSet<String> = response
        .get("exact_artifact_poll_fields")
        .and_then(|value| value.as_array())
        .expect("exact_artifact_poll_fields array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("exact_artifact_poll field string")
                .to_string()
        })
        .collect();
    let expected_exact_artifact_poll_fields: BTreeSet<String> = [
        "poll_count",
        "poll_elapsed_ms",
        "observed_file_version",
        "head_ready",
        "exact_ready",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect();
    assert_eq!(
        exact_artifact_poll_fields,
        expected_exact_artifact_poll_fields
    );

    let turn_attribution_fields: BTreeSet<String> = response
        .get("turn_attribution_fields")
        .and_then(|value| value.as_array())
        .expect("turn_attribution_fields array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("turn_attribution field string")
                .to_string()
        })
        .collect();
    let expected_turn_attribution_fields: BTreeSet<String> = [
        "request_file_seq",
        "request_epoch",
        "queue_outcome",
        "turn_wait_outcome",
        "dispatcher_resolution_latency_ms",
        "queue_capacity",
        "queue_depth_before_enqueue",
        "queue_depth_after_enqueue",
        "queued_completion_ahead_count",
        "did_change_ahead_count",
        "active_completion_count",
        "dropped_completion_file_seq",
        "active_holder",
        "queued_completion_ahead",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect();
    assert_eq!(turn_attribution_fields, expected_turn_attribution_fields);

    let server_edge_details_fields: BTreeSet<String> = response
        .get("server_edge_details_fields")
        .and_then(|value| value.as_array())
        .expect("server_edge_details_fields array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("server_edge_details field string")
                .to_string()
        })
        .collect();
    let expected_server_edge_details_fields: BTreeSet<String> = [
        "transport_received_at_ms",
        "transport_received_at_ms_provenance",
        "pre_method_attribution_provenance",
        "service_future_created_at_ms",
        "service_future_first_poll_entered_at_ms",
        "service_future_first_poll_outcome",
        "service_future_first_wake_scheduled_at_ms",
        "service_scope_entered_at_ms",
        "method_entered_at_ms",
        "handler_entered_at_ms",
        "response_sent_at_ms",
        "cancel_observed_at_ms",
        "jsonrpc_dispatch_received_at_ms",
        "dispatch_to_request_context_wait_ms",
        "transport_to_service_future_wait_ms",
        "service_future_to_scope_wait_ms",
        "service_future_to_first_poll_wait_ms",
        "first_poll_to_first_wake_wait_ms",
        "transport_to_service_scope_wait_ms",
        "service_scope_to_method_wait_ms",
        "transport_to_method_wait_ms",
        "method_prelude_exec_ms",
        "transport_to_handler_wait_ms",
        "server_handler_exec_ms",
        "cancel_observed_after_handler_enter_ms",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect();
    assert_eq!(
        server_edge_details_fields,
        expected_server_edge_details_fields
    );
}

#[test]
fn observability_completion_v4_contract_matches_runtime_metric_labels() {
    let contract = contract_json("observability-completion-v2/v4/contract.json");
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
            .get("completion_result_counter_prefix")
            .and_then(|value| value.as_str())
            .expect("completion result counter prefix"),
        "intellisense_v2_completion_result_total_"
    );
    assert_eq!(
        metrics_contract
            .get("completion_route_counter_prefix")
            .and_then(|value| value.as_str())
            .expect("completion route counter prefix"),
        "intellisense_v2_completion_route_total_route_"
    );
    assert_eq!(
        metrics_contract
            .get("completion_fail_closed_cause_counter_prefix")
            .and_then(|value| value.as_str())
            .expect("completion fail-closed cause counter prefix"),
        "intellisense_v2_completion_fail_closed_cause_total_cause_"
    );
    assert_eq!(
        metrics_contract
            .get("completion_head_to_exact_upgrade_counter")
            .and_then(|value| value.as_str())
            .expect("completion head-to-exact upgrade counter"),
        "intellisense_v2_completion_head_to_exact_upgrade_total"
    );
    assert_eq!(
        metrics_contract
            .get("completion_head_to_exact_upgrade_histogram")
            .and_then(|value| value.as_str())
            .expect("completion head-to-exact upgrade histogram"),
        "intellisense_v2_completion_head_to_exact_upgrade_ms"
    );
    assert_eq!(
        metrics_contract
            .get("interactive_fail_closed_reason_counter_prefix")
            .and_then(|value| value.as_str())
            .expect("interactive fail-closed reason counter prefix"),
        "intellisense_v2_fail_closed_reason_total_origin_"
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
    let fail_closed_reasons: Vec<String> = metrics_contract
        .get("allowed_fail_closed_reasons")
        .and_then(|value| value.as_array())
        .expect("allowed_fail_closed_reasons")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("allowed fail-closed reason string")
                .to_string()
        })
        .collect();
    let completion_outcomes: Vec<String> = metrics_contract
        .get("allowed_completion_outcomes")
        .and_then(|value| value.as_array())
        .expect("allowed_completion_outcomes")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("allowed completion outcome string")
                .to_string()
        })
        .collect();
    let completion_routes: Vec<String> = metrics_contract
        .get("allowed_completion_routes")
        .and_then(|value| value.as_array())
        .expect("allowed_completion_routes")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("allowed completion route string")
                .to_string()
        })
        .collect();
    let completion_fail_closed_causes: Vec<String> = metrics_contract
        .get("allowed_completion_fail_closed_causes")
        .and_then(|value| value.as_array())
        .expect("allowed_completion_fail_closed_causes")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("allowed completion fail-closed cause string")
                .to_string()
        })
        .collect();
    let fail_closed_reason_prefix = metrics_contract
        .get("interactive_fail_closed_reason_counter_prefix")
        .and_then(|value| value.as_str())
        .expect("interactive fail-closed reason counter prefix");
    let completion_route_prefix = metrics_contract
        .get("completion_route_counter_prefix")
        .and_then(|value| value.as_str())
        .expect("completion route counter prefix");
    let completion_fail_closed_cause_prefix = metrics_contract
        .get("completion_fail_closed_cause_counter_prefix")
        .and_then(|value| value.as_str())
        .expect("completion fail-closed cause counter prefix");
    let completion_head_to_exact_upgrade_counter = metrics_contract
        .get("completion_head_to_exact_upgrade_counter")
        .and_then(|value| value.as_str())
        .expect("completion head-to-exact upgrade counter");
    let completion_head_to_exact_upgrade_histogram = metrics_contract
        .get("completion_head_to_exact_upgrade_histogram")
        .and_then(|value| value.as_str())
        .expect("completion head-to-exact upgrade histogram");
    let runtime_queue_wait_interactive_counter = metrics_contract
        .get("runtime_queue_wait_interactive_counter")
        .and_then(|value| value.as_str())
        .expect("runtime_queue_wait_interactive_counter");
    let runtime_exec_interactive_counter = metrics_contract
        .get("runtime_exec_interactive_counter")
        .and_then(|value| value.as_str())
        .expect("runtime_exec_interactive_counter");
    let runtime_queue_wait_interactive_histogram = metrics_contract
        .get("runtime_queue_wait_interactive_histogram")
        .and_then(|value| value.as_str())
        .expect("runtime_queue_wait_interactive_histogram");
    let runtime_exec_interactive_histogram = metrics_contract
        .get("runtime_exec_interactive_histogram")
        .and_then(|value| value.as_str())
        .expect("runtime_exec_interactive_histogram");

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
    for reason in &fail_closed_reasons {
        observability.record_intellisense_v2_interactive_fail_closed_reason(
            "lsp",
            "completion",
            reason,
        );
        assert_eq!(
            normalize_shared_fail_closed_reason_label(reason),
            reason,
            "contract fail-closed reason must remain in bounded normalization set"
        );
    }
    observability.record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
        "lsp",
        "interactive",
        Duration::from_millis(3),
    );
    observability.record_intellisense_v2_runtime_exec_class_latency_with_origin(
        "lsp",
        "interactive",
        Duration::from_millis(5),
    );
    for outcome in &completion_outcomes {
        let raw_outcome = match outcome.as_str() {
            "fail_closed" => "fallback_unavailable",
            value => value,
        };
        observability.record_intellisense_v2_completion_outcome(raw_outcome);
        assert_eq!(
            normalize_public_completion_outcome_label(raw_outcome),
            outcome,
            "contract completion outcome must remain in bounded normalization set"
        );
    }
    for route in &completion_routes {
        observability.record_intellisense_v2_completion_route(route);
        assert_eq!(
            normalize_completion_route_label(route),
            route,
            "contract completion route must remain in bounded normalization set"
        );
    }
    for cause in &completion_fail_closed_causes {
        observability.record_intellisense_v2_completion_fail_closed_cause(cause);
        assert_eq!(
            normalize_completion_fail_closed_cause_label(cause),
            cause,
            "contract completion fail-closed cause must remain in bounded normalization set"
        );
    }
    observability.record_intellisense_v2_completion_head_to_exact_upgrade(Duration::from_millis(7));

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    let histograms = histograms(&exported);
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
    for reason in &fail_closed_reasons {
        let reason_key =
            format!("{fail_closed_reason_prefix}lsp_operation_completion_reason_{reason}");
        assert!(
            counter_value(counters, &reason_key) > 0,
            "interactive fail-closed reason counter must be exported for reason {reason}"
        );
    }
    for route in &completion_routes {
        let route_key = format!("{completion_route_prefix}{route}");
        assert!(
            counter_value(counters, &route_key) > 0,
            "completion route counter must be exported for route {route}"
        );
    }
    for cause in &completion_fail_closed_causes {
        let cause_key = format!("{completion_fail_closed_cause_prefix}{cause}");
        assert!(
            counter_value(counters, &cause_key) > 0,
            "completion fail-closed cause counter must be exported for cause {cause}"
        );
    }
    assert!(
        counter_value(counters, completion_head_to_exact_upgrade_counter) > 0,
        "head-to-exact upgrade counter must be exported"
    );
    assert!(
        histogram_count(histograms, completion_head_to_exact_upgrade_histogram) > 0,
        "head-to-exact upgrade histogram must be exported"
    );
    assert!(
        counter_value(counters, runtime_queue_wait_interactive_counter) > 0,
        "runtime queue wait counter must be projected via contract key"
    );
    assert!(
        counter_value(counters, runtime_exec_interactive_counter) > 0,
        "runtime exec counter must be projected via contract key"
    );
    assert!(
        histogram_count(histograms, runtime_queue_wait_interactive_histogram) > 0,
        "runtime queue wait histogram must be projected via contract key"
    );
    assert!(
        histogram_count(histograms, runtime_exec_interactive_histogram) > 0,
        "runtime exec histogram must be projected via contract key"
    );
    assert!(
        counter_value(
            counters,
            "intellisense_v2_completion_result_total_fail_closed"
        ) > 0,
        "fail_closed completion outcome counter must be exported"
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
    let publish_histogram_prefix = metrics_contract
        .get("publish_latency_histogram_prefix")
        .and_then(|value| value.as_str())
        .expect("publish latency histogram prefix");
    assert_eq!(
        counter_prefix,
        "intellisense_v2_diagnostics_pipeline_total_origin_"
    );
    assert_eq!(
        histogram_prefix,
        "intellisense_v2_diagnostics_pipeline_cancel_sample_origin_"
    );
    assert_eq!(
        publish_histogram_prefix,
        "intellisense_v2_diagnostics_pipeline_publish_ms_origin_"
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
    let cancellation_reasons_set: BTreeSet<String> = cancellation_reasons.iter().cloned().collect();
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
        observability
            .record_intellisense_v2_diagnostics_pipeline_event(origin, trigger, profile, reason);
    }

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    let exported_histograms = histograms(&exported);
    for reason in &reasons {
        let counter_key =
            format!("{counter_prefix}{origin}_trigger_{trigger}_profile_{profile}_reason_{reason}");
        assert!(
            counter_value(counters, &counter_key) > 0,
            "diagnostics pipeline counter must be exported for reason {reason}"
        );

        let histogram_key = format!(
            "{histogram_prefix}{origin}_trigger_{trigger}_profile_{profile}_reason_{reason}"
        );
        if cancellation_reasons_set.contains(reason) {
            assert!(
                histogram_count(exported_histograms, &histogram_key) > 0,
                "diagnostics pipeline cancellation histogram must be exported for reason {reason}"
            );
        } else {
            assert_eq!(
                histogram_count(exported_histograms, &histogram_key),
                0,
                "non-cancellation reason {reason} must not emit cancellation histogram sample"
            );
        }
    }

    let publish_profile = profiles
        .iter()
        .find(|profile| profile.as_str() == "save_fastlane")
        .map(String::as_str)
        .unwrap_or(profiles[0].as_str());
    observability.record_intellisense_v2_diagnostics_pipeline_publish_latency(
        origin,
        "did_save",
        publish_profile,
        Duration::from_millis(17),
    );
    let publish_exported = observability.get_metrics().export_metrics();
    let publish_histograms = histograms(&publish_exported);
    let publish_key =
        format!("{publish_histogram_prefix}{origin}_trigger_did_save_profile_{publish_profile}");
    assert!(
        histogram_count(publish_histograms, &publish_key) > 0,
        "diagnostics publish latency histogram must be exported for bounded profile {publish_profile}"
    );
}

#[test]
fn completion_owner_hint_metrics_are_exported_with_bounded_reasons() {
    let observability = BasicObservability::default();
    let reasons = [
        "not_member_access",
        "no_file_content",
        "no_line",
        "no_dot",
        "no_receiver",
        "offset_unresolved",
        "flow_type_hit",
        "flow_type_miss",
        "type_hit",
        "type_miss",
        "cancelled",
        "type_index_exact_hit",
        "type_index_fallback_unavailable",
        "unexpected_reason",
    ];
    for reason in reasons {
        observability.record_intellisense_v2_completion_owner_hint_result(reason);
    }
    for path in [
        "direct",
        "flow_only",
        "flow_plus_fallback",
        "unexpected_path",
    ] {
        observability.record_intellisense_v2_completion_owner_hint_lookup_path(path);
    }
    for result in ["hit", "miss", "cancelled", "error", "unexpected_result"] {
        observability.record_intellisense_v2_completion_owner_hint_lookup_result(result);
    }
    for (reason, millis) in [
        ("allocator_pressure", 11_u64),
        ("lock_wait", 13_u64),
        ("queue_backpressure", 17_u64),
        ("unexpected_reason", 19_u64),
    ] {
        observability
            .record_completion_resource_pressure(reason, std::time::Duration::from_millis(millis));
    }
    observability.record_intellisense_v2_completion_owner_hint_context(240, 18);
    observability.record_intellisense_v2_completion_owner_hint_index_fetch_salsa_counters(
        CompletionOwnerHintIndexFetchSalsaCounters {
            block_on_total: 7,
            block_on_type_index_total: 4,
            block_on_parse_result_total: 2,
            block_on_other_total: 1,
            will_execute_total: 11,
            will_execute_type_index_total: 5,
            will_execute_parse_result_total: 3,
            will_execute_other_total: 3,
            did_validate_memoized_total: 13,
            did_validate_memoized_type_index_total: 6,
            did_validate_memoized_parse_result_total: 4,
            did_validate_memoized_other_total: 3,
            will_check_cancellation_total: 9,
        },
    );
    observability.record_intellisense_v2_completion_owner_hint_index_fetch_active_gauge(3);
    observability
        .record_intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch(
            9,
        );
    observability
        .record_intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch(3);
    observability
        .record_intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch(5);
    observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch(1);
    observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch(2);
    observability.record_intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch(2);
    observability
        .record_intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch(
            4,
        );
    observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch(5);
    observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch(4);
    observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch(1);
    observability
            .record_intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch(1);
    observability.record_intellisense_v2_completion_owner_hint_index_fetch_revision_start(17);
    observability.record_intellisense_v2_completion_owner_hint_index_fetch_revision_end(19);
    observability.record_intellisense_v2_completion_owner_hint_index_fetch_revision_delta(2);
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_flow_lookup",
        std::time::Duration::from_millis(3),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_direct",
        std::time::Duration::from_millis(5),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_fallback",
        std::time::Duration::from_millis(7),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_wait",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_unattributed",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_type_index",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_other",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_cancellation",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
            "query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index",
            std::time::Duration::from_millis(2),
        );
    observability.record_completion_stage_latency(
            "query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_to_first_will_execute_type_index",
            std::time::Duration::from_millis(2),
        );
    observability.record_completion_stage_latency(
            "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index",
            std::time::Duration::from_millis(2),
        );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_idle_before_first_will_execute_type_index",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_apply_age_at_query_start",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_apply_to_first_will_execute_type_index",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_fetch_apply_to_fetch_end",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_query_total",
        std::time::Duration::from_millis(4),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_query_inputs",
        std::time::Duration::from_millis(1),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_query_parse_result_query",
        std::time::Duration::from_millis(1),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_query_build",
        std::time::Duration::from_millis(1),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_parse_result",
        std::time::Duration::from_millis(1),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_build_total",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_build_seed_context",
        std::time::Duration::from_millis(1),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_build_local_function_summaries",
        std::time::Duration::from_millis(1),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_build_visit_statements",
        std::time::Duration::from_millis(1),
    );
    observability.record_completion_stage_latency(
        "query_bundle_owner_hint_type_lookup_index_scan",
        std::time::Duration::from_millis(1),
    );
    observability.record_completion_stage_latency(
        "transport_to_handler_wait",
        std::time::Duration::from_millis(6),
    );
    observability.record_completion_stage_latency(
        "server_handler_exec",
        std::time::Duration::from_millis(12),
    );
    observability.record_completion_stage_latency(
        "cancel_observed_after_handler_enter",
        std::time::Duration::from_millis(9),
    );
    observability.record_completion_stage_latency(
        "response_ready_to_output_handoff_wait",
        std::time::Duration::from_millis(1),
    );
    observability.record_completion_stage_latency(
        "response_output_handoff_send_wait",
        std::time::Duration::from_millis(2),
    );
    observability.record_completion_stage_latency(
        "response_output_handoff_to_writer_wait",
        std::time::Duration::from_millis(3),
    );
    observability.record_completion_stage_latency(
        "response_ready_to_output_enqueue_wait",
        std::time::Duration::from_millis(4),
    );
    observability.record_completion_stage_latency(
        "response_output_queue_wait",
        std::time::Duration::from_millis(5),
    );
    observability.record_completion_stage_latency(
        "response_output_encode_exec",
        std::time::Duration::from_millis(6),
    );
    observability.record_completion_stage_latency(
        "response_output_write_and_flush_exec",
        std::time::Duration::from_millis(7),
    );
    observability.record_completion_stage_latency(
        "response_ready_to_flush_wait",
        std::time::Duration::from_millis(8),
    );
    observability.record_intellisense_v2_completion_cancel_observed();

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    let gauges = gauges(&exported);
    let histograms = histograms(&exported);

    for reason in [
        "not_member_access",
        "no_file_content",
        "no_line",
        "no_dot",
        "no_receiver",
        "offset_unresolved",
        "flow_type_hit",
        "flow_type_miss",
        "type_hit",
        "type_miss",
        "cancelled",
        "type_index_exact_hit",
        "type_index_fallback_unavailable",
        "other",
    ] {
        let key = format!("intellisense_v2_completion_owner_hint_result_total_reason_{reason}");
        assert!(
            counter_value(counters, &key) > 0,
            "owner-hint reason counter must be exported for {reason}"
        );
    }
    assert!(
        counter_value(counters, "intellisense_v2_completion_cancel_observed_total") > 0,
        "completion cancel-observed counter must be exported"
    );
    for (label, key) in [
        (
            "direct",
            "intellisense_v2_completion_owner_hint_lookup_path_total_direct",
        ),
        (
            "flow_only",
            "intellisense_v2_completion_owner_hint_lookup_path_total_flow_only",
        ),
        (
            "flow_plus_fallback",
            "intellisense_v2_completion_owner_hint_lookup_path_total_flow_plus_fallback",
        ),
        (
            "other",
            "intellisense_v2_completion_owner_hint_lookup_path_total_other",
        ),
    ] {
        assert!(
            counter_value(counters, key) > 0,
            "owner-hint lookup-path counter must be exported for {label}"
        );
    }
    for (label, key) in [
        (
            "hit",
            "intellisense_v2_completion_owner_hint_lookup_result_total_hit",
        ),
        (
            "miss",
            "intellisense_v2_completion_owner_hint_lookup_result_total_miss",
        ),
        (
            "cancelled",
            "intellisense_v2_completion_owner_hint_lookup_result_total_cancelled",
        ),
        (
            "error",
            "intellisense_v2_completion_owner_hint_lookup_result_total_error",
        ),
        (
            "other",
            "intellisense_v2_completion_owner_hint_lookup_result_total_other",
        ),
    ] {
        assert!(
            counter_value(counters, key) > 0,
            "owner-hint lookup-result counter must be exported for {label}"
        );
    }
    for (label, counter_key, histogram_key) in [
        (
            "allocator_pressure",
            "intellisense_v2_completion_resource_pressure_total_reason_allocator_pressure",
            "intellisense_v2_completion_resource_pressure_ms_reason_allocator_pressure",
        ),
        (
            "lock_wait",
            "intellisense_v2_completion_resource_pressure_total_reason_lock_wait",
            "intellisense_v2_completion_resource_pressure_ms_reason_lock_wait",
        ),
        (
            "queue_backpressure",
            "intellisense_v2_completion_resource_pressure_total_reason_queue_backpressure",
            "intellisense_v2_completion_resource_pressure_ms_reason_queue_backpressure",
        ),
        (
            "other",
            "intellisense_v2_completion_resource_pressure_total_reason_other",
            "intellisense_v2_completion_resource_pressure_ms_reason_other",
        ),
    ] {
        assert!(
            counter_value(counters, counter_key) > 0,
            "resource-pressure counter must be exported for {label}"
        );
        assert!(
            histogram_count(histograms, histogram_key) > 0,
            "resource-pressure histogram must be exported for {label}"
        );
    }
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_completion_owner_hint_line_len_chars"
        ) > 0,
        "owner-hint line length histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_completion_owner_hint_receiver_len_chars"
        ) > 0,
        "owner-hint receiver length histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_flow_lookup_ms"
        ) > 0,
        "owner-hint flow lookup histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_direct_ms"
        ) > 0,
        "owner-hint direct lookup histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_fallback_ms"
        ) > 0,
        "owner-hint fallback lookup histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_ms"
        ) > 0,
        "owner-hint index fetch histogram must be exported"
    );
    assert!(
        histogram_count(histograms, "completion_stage_transport_to_handler_wait_ms") > 0,
        "server-edge transport wait histogram must be exported"
    );
    assert!(
        histogram_count(histograms, "completion_stage_server_handler_exec_ms") > 0,
        "server-edge handler execution histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_cancel_observed_after_handler_enter_ms"
        ) > 0,
        "server-edge cancel observation histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_response_ready_to_output_handoff_wait_ms"
        ) > 0,
        "server-edge handoff start wait histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_response_output_handoff_send_wait_ms"
        ) > 0,
        "server-edge handoff send wait histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_response_output_handoff_to_writer_wait_ms"
        ) > 0,
        "server-edge handoff-to-writer wait histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_response_ready_to_output_enqueue_wait_ms"
        ) > 0,
        "server-edge enqueue readiness histogram must be exported"
    );
    assert!(
        histogram_count(histograms, "completion_stage_response_output_queue_wait_ms") > 0,
        "server-edge output queue wait histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_response_output_encode_exec_ms"
        ) > 0,
        "server-edge output encode histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_response_output_write_and_flush_exec_ms"
        ) > 0,
        "server-edge output write-and-flush histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_response_ready_to_flush_wait_ms"
        ) > 0,
        "server-edge ready-to-flush histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms"
        ) > 0,
        "owner-hint index fetch wait histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_unattributed_ms"
        ) > 0,
        "owner-hint index fetch unattributed histogram must be exported"
    );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait_ms"
            ) > 0,
            "owner-hint index fetch pre-first-salsa-event histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail_ms"
            ) > 0,
            "owner-hint index fetch post-last-salsa-event histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window_ms"
            ) > 0,
            "owner-hint index fetch inside-salsa-window histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms"
            ) > 0,
            "owner-hint first WillExecute(type_index) histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_type_index_ms"
            ) > 0,
            "owner-hint last WillExecute(type_index) histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result_ms"
            ) > 0,
            "owner-hint first WillExecute(parse_result) histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other_ms"
            ) > 0,
            "owner-hint first WillExecute(other) histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_ms"
            ) > 0,
            "owner-hint last WillExecute(parse_result) histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_other_ms"
            ) > 0,
            "owner-hint last WillExecute(other) histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle_ms"
            ) > 0,
            "owner-hint first WillIterateCycle histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle_ms"
            ) > 0,
            "owner-hint last WillIterateCycle histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation_ms"
            ) > 0,
            "owner-hint first WillCheckCancellation histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_cancellation_ms"
            ) > 0,
            "owner-hint last WillCheckCancellation histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index_ms"
            ) > 0,
            "owner-hint first WillCheckCancellation -> first WillExecute(type_index) histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_to_first_will_execute_type_index_ms"
            ) > 0,
            "owner-hint last WillCheckCancellation -> first WillExecute(type_index) histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index_ms"
            ) > 0,
            "owner-hint last WillExecute(parse_result) -> first WillExecute(type_index) histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_idle_before_first_will_execute_type_index_ms"
            ) > 0,
            "owner-hint idle-before-first-WillExecute(type_index) histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_age_at_query_start_ms"
            ) > 0,
            "owner-hint apply-age-at-query-start histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_first_will_execute_type_index_ms"
            ) > 0,
            "owner-hint apply-to-first-WillExecute(type_index) histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_fetch_end_ms"
            ) > 0,
            "owner-hint apply-to-fetch-end histogram must be exported"
        );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_total_ms"
        ) > 0,
        "owner-hint index query total histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_inputs_ms"
        ) > 0,
        "owner-hint index query inputs histogram must be exported"
    );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_parse_result_query_ms"
            ) > 0,
            "owner-hint index query parse-result histogram must be exported"
        );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_build_ms"
        ) > 0,
        "owner-hint index query build histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_index_parse_result_ms"
        ) > 0,
        "owner-hint index parse-result histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_total_ms"
        ) > 0,
        "owner-hint index build total histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_seed_context_ms"
        ) > 0,
        "owner-hint index build seed-context histogram must be exported"
    );
    assert!(
            histogram_count(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_local_function_summaries_ms"
            ) > 0,
            "owner-hint index build local-function-summaries histogram must be exported"
        );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_visit_statements_ms"
        ) > 0,
        "owner-hint index build visit-statements histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "completion_stage_query_bundle_owner_hint_type_lookup_index_scan_ms"
        ) > 0,
        "owner-hint index scan histogram must be exported"
    );
    for (label, key) in [
        (
            "total",
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_total",
        ),
        (
            "type_index",
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_type_index_total",
        ),
        (
            "parse_result",
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_parse_result_total",
        ),
        (
            "other",
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_other_total",
        ),
    ] {
        assert!(
            counter_value(counters, key) > 0,
            "owner-hint block-on counter must be exported for {label}"
        );
    }
    for (label, key) in [
            (
                "will_execute_total",
                "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_total",
            ),
            (
                "will_execute_type_index_total",
                "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_type_index_total",
            ),
            (
                "will_execute_parse_result_total",
                "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_parse_result_total",
            ),
            (
                "will_execute_other_total",
                "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_other_total",
            ),
            (
                "did_validate_memoized_total",
                "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_total",
            ),
            (
                "did_validate_memoized_type_index_total",
                "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_type_index_total",
            ),
            (
                "did_validate_memoized_parse_result_total",
                "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_parse_result_total",
            ),
            (
                "did_validate_memoized_other_total",
                "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_other_total",
            ),
            (
                "will_check_cancellation_total",
                "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_check_cancellation_total",
            ),
        ] {
            assert!(
                counter_value(counters, key) > 0,
                "owner-hint salsa counter must be exported for {label}"
            );
        }
    assert!(
        gauges
            .get("intellisense_v2_completion_owner_hint_index_fetch_active")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0)
            >= 1.0,
        "owner-hint index-fetch active gauge must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch"
        ) > 0,
        "owner-hint WillCheckCancellation-per-fetch histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch"
        ) > 0,
        "owner-hint WillExecute(other)-per-fetch histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch"
        ) > 0,
        "owner-hint WillIterateCycle-per-fetch histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch"
        ) > 0,
        "owner-hint DidSetCancellationFlag-per-fetch histogram must be exported"
    );
    assert!(
            histogram_count(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch"
            ) > 0,
            "owner-hint global DidSetCancellationFlag-per-fetch histogram must be exported"
        );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch"
        ) > 0,
        "owner-hint DidDiscard-per-fetch histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch"
        ) > 0,
        "owner-hint DidDiscardAccumulated-per-fetch histogram must be exported"
    );
    assert!(
            histogram_count(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch"
            ) > 0,
            "owner-hint events-before-first-WillExecute(type_index)-per-fetch histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch"
            ) > 0,
            "owner-hint WillCheck-before-first-WillExecute(type_index)-per-fetch histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch"
            ) > 0,
            "owner-hint WillExecute(parse_result)-before-first-WillExecute(type_index)-per-fetch histogram must be exported"
        );
    assert!(
            histogram_count(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch"
            ) > 0,
            "owner-hint first-WillExecute(type_index)-seen-per-fetch histogram must be exported"
        );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_completion_owner_hint_index_fetch_revision_start"
        ) > 0,
        "owner-hint index-fetch revision-start histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_completion_owner_hint_index_fetch_revision_end"
        ) > 0,
        "owner-hint index-fetch revision-end histogram must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_completion_owner_hint_index_fetch_revision_delta"
        ) > 0,
        "owner-hint index-fetch revision-delta histogram must be exported"
    );
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

#[test]
fn completion_collect_detail_stage_histograms_are_exported() {
    let observability = BasicObservability::default();

    for stage in [
        "collect_member_owner_resolve",
        "collect_member_methods",
        "collect_member_properties",
        "collect_member_metadata",
        "collect_non_member_local_symbols",
        "collect_non_member_contextual_symbols",
        "collect_non_member_module_routines",
        "collect_non_member_global_functions",
        "collect_non_member_metadata_items",
        "collect_non_member_repository_types",
        "collect_non_member_keywords",
    ] {
        observability.record_completion_stage_latency(stage, Duration::from_millis(3));
    }

    let exported = observability.get_metrics().export_metrics();
    let histograms = histograms(&exported);

    for key in [
        "completion_stage_collect_member_owner_resolve_ms",
        "completion_stage_collect_member_methods_ms",
        "completion_stage_collect_member_properties_ms",
        "completion_stage_collect_member_metadata_ms",
        "completion_stage_collect_non_member_local_symbols_ms",
        "completion_stage_collect_non_member_contextual_symbols_ms",
        "completion_stage_collect_non_member_module_routines_ms",
        "completion_stage_collect_non_member_global_functions_ms",
        "completion_stage_collect_non_member_metadata_items_ms",
        "completion_stage_collect_non_member_repository_types_ms",
        "completion_stage_collect_non_member_keywords_ms",
    ] {
        assert!(
            histogram_count(histograms, key) > 0,
            "collect detail histogram must be exported for {key}"
        );
    }
}

#[test]
fn current_context_parse_source_metrics_are_exported() {
    let observability = BasicObservability::default();

    for source in [
        "ready_snapshot",
        "parser_coordinator",
        "syntax_fallback",
        "parse_unavailable",
    ] {
        observability.record_intellisense_v2_current_context_parse_source(source);
        observability
            .record_intellisense_v2_current_context_parse_latency(source, Duration::from_millis(7));
        observability
            .record_intellisense_v2_current_context_wall_latency(source, Duration::from_millis(11));
    }
    for role in ["ready_snapshot", "broker_leader", "broker_follower"] {
        observability.record_intellisense_v2_current_context_role(role);
        observability.record_intellisense_v2_current_context_parse_latency_by_role(
            role,
            Duration::from_millis(13),
        );
        observability.record_intellisense_v2_current_context_wall_latency_by_role(
            role,
            Duration::from_millis(17),
        );
    }
    for outcome in [
        "resolved",
        "parse_unavailable",
        "superseded",
        "budget_exhausted",
    ] {
        observability.record_intellisense_v2_current_context_terminal_outcome(outcome);
    }

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    let histograms = histograms(&exported);

    for source in [
        "ready_snapshot",
        "parser_coordinator",
        "syntax_fallback",
        "parse_unavailable",
    ] {
        let counter_key =
            format!("intellisense_v2_current_context_parse_source_total_source_{source}");
        let parse_histogram_key =
            format!("intellisense_v2_current_context_parse_ms_source_{source}");
        let wall_histogram_key = format!("intellisense_v2_current_context_wall_ms_source_{source}");
        assert!(
            counter_value(counters, &counter_key) > 0,
            "current-context parse source counter must be exported for {source}"
        );
        assert!(
            histogram_count(histograms, &parse_histogram_key) > 0,
            "current-context parse histogram must be exported for {source}"
        );
        assert!(
            histogram_count(histograms, &wall_histogram_key) > 0,
            "current-context wall histogram must be exported for {source}"
        );
    }

    for role in ["ready_snapshot", "broker_leader", "broker_follower"] {
        let counter_key = format!("intellisense_v2_current_context_role_total_role_{role}");
        let parse_histogram_key = format!("intellisense_v2_current_context_parse_ms_role_{role}");
        let wall_histogram_key = format!("intellisense_v2_current_context_wall_ms_role_{role}");
        assert!(
            counter_value(counters, &counter_key) > 0,
            "current-context role counter must be exported for {role}"
        );
        assert!(
            histogram_count(histograms, &parse_histogram_key) > 0,
            "current-context parse-by-role histogram must be exported for {role}"
        );
        assert!(
            histogram_count(histograms, &wall_histogram_key) > 0,
            "current-context wall-by-role histogram must be exported for {role}"
        );
    }

    for outcome in [
        "resolved",
        "parse_unavailable",
        "superseded",
        "budget_exhausted",
    ] {
        let counter_key =
            format!("intellisense_v2_current_context_terminal_total_outcome_{outcome}");
        assert!(
            counter_value(counters, &counter_key) > 0,
            "current-context terminal outcome counter must be exported for {outcome}"
        );
    }
}

#[test]
fn did_save_followup_ready_snapshot_metrics_are_exported() {
    let observability = BasicObservability::default();

    observability.record_intellisense_v2_ready_parse_snapshot_worker_started("lsp", "did_change");
    observability
        .record_intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization(
            "lsp",
            "did_save",
            "aborted",
            Duration::from_millis(1200),
        );
    observability.record_intellisense_v2_ready_parse_snapshot_materialization(
        "lsp",
        "did_change",
        Duration::from_millis(77),
    );
    observability.record_intellisense_v2_diagnostics_save_followup_ready_snapshot_probe(
        "zero_budget",
        "not_ready",
        Duration::from_millis(1),
    );
    observability.record_intellisense_v2_diagnostics_save_followup_ready_snapshot_probe(
        "bounded_wait",
        "timeout",
        Duration::from_millis(3500),
    );
    observability.record_intellisense_v2_diagnostics_save_followup_wait_state("apply_lag");
    observability.record_intellisense_v2_diagnostics_save_followup_semantic_path("shadow_state");

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    let histograms = histograms(&exported);

    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_did_change"
        ),
        1,
        "ready parse snapshot worker-start counter must be exported per source"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_aborted"
        ),
        1,
        "ready parse snapshot worker termination counter must be exported per source/reason"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_aborted"
        ) > 0,
        "ready parse snapshot worker termination latency histogram must be exported per source/reason"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_change"
        ),
        1,
        "ready parse snapshot materialization counter must be exported per source"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_did_change"
        ) > 0,
        "ready parse snapshot materialization latency histogram must be exported per source"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_zero_budget_outcome_not_ready"
        ),
        1,
        "zero-budget probe outcome counter must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_zero_budget_outcome_not_ready"
        ) > 0,
        "zero-budget probe latency histogram must be exported"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_timeout"
        ),
        1,
        "bounded-wait probe timeout counter must be exported"
    );
    assert!(
        histogram_count(
            histograms,
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_bounded_wait_outcome_timeout"
        ) > 0,
        "bounded-wait probe timeout latency histogram must be exported"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_diagnostics_save_followup_wait_state_total_reason_apply_lag"
        ),
        1,
        "didSave follow-up wait-state counter must be exported"
    );
    assert_eq!(
        counter_value(
            counters,
            "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_shadow_state"
        ),
        1,
        "didSave follow-up semantic path counter must be exported"
    );
}
