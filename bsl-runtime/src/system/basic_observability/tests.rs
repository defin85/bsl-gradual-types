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
fn type_index_reason_metrics_are_exported_with_bounded_reasons() {
    let contract = contract_json("observability-completion-v2/v2/contract.json");
    let metrics_contract = contract
        .get("metrics")
        .and_then(|value| value.as_object())
        .expect("metrics contract section");
    let reason_prefix = metrics_contract
        .get("type_index_reason_counter_prefix")
        .and_then(|value| value.as_str())
        .expect("type_index reason counter prefix");
    assert_eq!(
        reason_prefix,
        "intellisense_v2_type_index_reason_total_reason_"
    );
    let contract_reasons: Vec<String> = metrics_contract
        .get("allowed_type_index_reasons")
        .and_then(|value| value.as_array())
        .expect("allowed_type_index_reasons")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("allowed type_index reason string")
                .to_string()
        })
        .collect();

    let observability = BasicObservability::default();
    for reason in &contract_reasons {
        assert_eq!(
            normalize_type_index_reason_label(reason),
            reason,
            "contract type-index reason must stay in bounded normalization set"
        );
        observability.record_intellisense_v2_type_index_reason(reason);
    }
    observability.record_intellisense_v2_type_index_reason("unexpected_reason");

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
    for reason in &contract_reasons {
        let key = format!("{reason_prefix}{reason}");
        assert!(
            counter_value(counters, &key) > 0,
            "type-index reason counter must be exported for {reason}"
        );
    }
    let other_key = format!("{reason_prefix}other");
    assert!(
        counter_value(counters, &other_key) > 0,
        "type-index reason counter must collapse unknown reasons into other"
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
            "intellisense_v2_observability_contract_violation_reason_unknown_type_index_reason",
        ),
        1,
        "unknown type-index reason must emit dedicated contract-violation reason"
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
fn completion_outcome_exports_fallback_unavailable() {
    let observability = BasicObservability::default();
    observability.record_intellisense_v2_completion_outcome("fallback_unavailable");

    let exported = observability.get_metrics().export_metrics();
    let counters = counters(&exported);
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
fn completion_v2_contract_matches_runtime_outcomes_and_modes() {
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

    let outcomes: BTreeSet<String> = completion
        .get("outcomes")
        .and_then(|value| value.as_array())
        .expect("outcomes array")
        .iter()
        .map(|value| value.as_str().expect("outcome string").to_string())
        .collect();
    let expected_outcomes: BTreeSet<String> = ["ok_non_empty", "ok_empty", "fallback_unavailable"]
        .iter()
        .map(|value| value.to_string())
        .collect();
    assert_eq!(
        outcomes, expected_outcomes,
        "contract outcomes must match current completion baseline"
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
fn observability_completion_v2_contract_matches_runtime_metric_labels() {
    let contract = contract_json("observability-completion-v2/v2/contract.json");
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
    assert_eq!(
        metrics_contract
            .get("type_index_reason_counter_prefix")
            .and_then(|value| value.as_str())
            .expect("type_index reason counter prefix"),
        "intellisense_v2_type_index_reason_total_reason_"
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
    let type_index_reasons: Vec<String> = metrics_contract
        .get("allowed_type_index_reasons")
        .and_then(|value| value.as_array())
        .expect("allowed_type_index_reasons")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("allowed type_index reason string")
                .to_string()
        })
        .collect();
    let type_index_reason_prefix = metrics_contract
        .get("type_index_reason_counter_prefix")
        .and_then(|value| value.as_str())
        .expect("type_index reason counter prefix");
    let precompute_queue_wait_counter = metrics_contract
        .get("type_index_precompute_queue_wait_counter")
        .and_then(|value| value.as_str())
        .expect("type_index precompute queue wait counter");
    let precompute_exec_counter = metrics_contract
        .get("type_index_precompute_exec_counter")
        .and_then(|value| value.as_str())
        .expect("type_index precompute exec counter");
    let precompute_build_exec_counter = metrics_contract
        .get("type_index_precompute_build_exec_counter")
        .and_then(|value| value.as_str())
        .expect("type_index precompute build exec counter");
    let precompute_queue_wait_histogram = metrics_contract
        .get("type_index_precompute_queue_wait_histogram")
        .and_then(|value| value.as_str())
        .expect("type_index precompute queue wait histogram");
    let precompute_exec_histogram = metrics_contract
        .get("type_index_precompute_exec_histogram")
        .and_then(|value| value.as_str())
        .expect("type_index precompute exec histogram");
    let precompute_build_exec_histogram = metrics_contract
        .get("type_index_precompute_build_exec_histogram")
        .and_then(|value| value.as_str())
        .expect("type_index precompute build exec histogram");

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
    for reason in &type_index_reasons {
        observability.record_intellisense_v2_type_index_reason(reason);
        assert_eq!(
            normalize_type_index_reason_label(reason),
            reason,
            "contract type_index reason must remain in bounded normalization set"
        );
    }
    observability.record_intellisense_v2_runtime_queue_wait_latency_with_origin(
        "lsp",
        "type_index_precompute",
        Duration::from_millis(3),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "type_index_precompute",
        Duration::from_millis(5),
    );
    observability.record_intellisense_v2_runtime_exec_latency_with_origin(
        "lsp",
        "type_index_precompute_build",
        Duration::from_millis(2),
    );
    observability.record_intellisense_v2_completion_outcome("fallback_unavailable");

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
    for reason in &type_index_reasons {
        let reason_key = format!("{type_index_reason_prefix}{reason}");
        assert!(
            counter_value(counters, &reason_key) > 0,
            "type-index reason counter must be exported for reason {reason}"
        );
    }
    assert!(
        counter_value(counters, precompute_queue_wait_counter) > 0,
        "precompute queue wait counter must be projected via contract key"
    );
    assert!(
        counter_value(counters, precompute_exec_counter) > 0,
        "precompute exec counter must be projected via contract key"
    );
    assert!(
        counter_value(counters, precompute_build_exec_counter) > 0,
        "precompute build exec counter must be projected via contract key"
    );
    assert!(
        histogram_count(histograms, precompute_queue_wait_histogram) > 0,
        "precompute queue wait histogram must be projected via contract key"
    );
    assert!(
        histogram_count(histograms, precompute_exec_histogram) > 0,
        "precompute exec histogram must be projected via contract key"
    );
    assert!(
        histogram_count(histograms, precompute_build_exec_histogram) > 0,
        "precompute build exec histogram must be projected via contract key"
    );
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
    let histograms = histograms(&exported);
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
