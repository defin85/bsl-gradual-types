pub(super) fn contains_allowed(allowed: &[&str], value: &str) -> bool {
    allowed.contains(&value)
}

pub(super) fn sanitize_identifier(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

pub(super) fn registry_label(
    value: &str,
    registry: &[(&'static str, &'static str)],
    fallback: &'static str,
) -> &'static str {
    registry
        .iter()
        .find_map(|(raw, normalized)| (*raw == value).then_some(*normalized))
        .unwrap_or(fallback)
}

pub(super) fn registry_metric_pair(
    value: &str,
    registry: &[(&'static str, (&'static str, &'static str))],
    fallback: (&'static str, &'static str),
) -> (&'static str, &'static str) {
    registry
        .iter()
        .find_map(|(raw, metrics)| (*raw == value).then_some(*metrics))
        .unwrap_or(fallback)
}

pub(super) fn registry_metric_single(
    value: &str,
    registry: &[(&'static str, &'static str)],
    fallback: &'static str,
) -> &'static str {
    registry
        .iter()
        .find_map(|(raw, metric)| (*raw == value).then_some(*metric))
        .unwrap_or(fallback)
}

pub(super) const OPERATION_LABEL_REGISTRY: &[(&str, &str)] = &[
    ("completion", "completion"),
    ("hover", "hover"),
    ("signature_help", "signature_help"),
    ("definition", "definition"),
    ("document_symbol", "document_symbol"),
    ("rename", "rename"),
    ("diagnostics", "diagnostics"),
    ("members", "members"),
    ("type_at_position", "type_at_position"),
    ("symbol_search", "symbol_search"),
    ("references", "references"),
];

pub(super) const RUNTIME_STAGE_KIND_REGISTRY: &[(&str, &str)] = &[
    ("wait_for_file_version", "wait_for_file_version"),
    ("snapshot_with_deps", "snapshot_with_deps"),
    ("apply_changes_batch", "apply_changes_batch"),
    ("apply_change_set_file", "apply_change_set_file"),
    (
        "apply_change_set_file_with_snapshot",
        "apply_change_set_file_with_snapshot",
    ),
    ("apply_change_remove_file", "apply_change_remove_file"),
    (
        "apply_change_set_settings_snapshot",
        "apply_change_set_settings_snapshot",
    ),
    ("type_index_precompute", "type_index_precompute"),
    ("type_index_precompute_build", "type_index_precompute_build"),
    ("type_index_precompute_ir", "type_index_precompute_ir"),
    (
        "type_index_precompute_ast_to_ir",
        "type_index_precompute_ast_to_ir",
    ),
    (
        "type_index_precompute_semantic_facts",
        "type_index_precompute_semantic_facts",
    ),
    (
        "type_index_precompute_semantic_facts_seed_module_context",
        "type_index_precompute_semantic_facts_seed_module_context",
    ),
    (
        "type_index_precompute_semantic_facts_local_function_summaries",
        "type_index_precompute_semantic_facts_local_function_summaries",
    ),
    (
        "type_index_precompute_semantic_facts_visit_statements",
        "type_index_precompute_semantic_facts_visit_statements",
    ),
];

pub(super) const QUERY_KIND_REGISTRY: &[(&str, &str)] = &[
    ("parse_result", "parse_result"),
    ("syntax_diagnostics", "syntax_diagnostics"),
    ("ir", "ir"),
];

pub(super) const REASON_LABEL_REGISTRY: &[(&str, &str)] =
    &[("syntax", "syntax"), ("semantic", "semantic")];

pub(super) const WORK_CLASS_REGISTRY: &[(&str, &str)] = &[("background", "background")];

pub(super) const OBSERVABILITY_ORIGIN_REGISTRY: &[(&str, &str)] = &[
    ("lsp", "lsp"),
    ("web", "web"),
    ("agent", "agent"),
    ("runtime", "runtime"),
];

pub(super) const TYPE_INDEX_REASON_REGISTRY: &[(&str, &str)] = &[
    ("type_index_exact_hit", "type_index_exact_hit"),
    (
        "type_index_fallback_unavailable",
        "type_index_fallback_unavailable",
    ),
    (
        "type_index_precompute_exact_stored",
        "type_index_precompute_exact_stored",
    ),
    (
        "type_index_precompute_superseded",
        "type_index_precompute_superseded",
    ),
    (
        "type_index_precompute_cancelled",
        "type_index_precompute_cancelled",
    ),
    (
        "type_index_precompute_missing_file",
        "type_index_precompute_missing_file",
    ),
    (
        "type_index_precompute_queue_saturated",
        "type_index_precompute_queue_saturated",
    ),
    (
        "type_index_artifact_invalidated_deps",
        "type_index_artifact_invalidated_deps",
    ),
    (
        "type_index_artifact_invalidated_settings",
        "type_index_artifact_invalidated_settings",
    ),
    (
        "type_index_artifact_evicted_global_guard",
        "type_index_artifact_evicted_global_guard",
    ),
    (
        "type_index_artifact_evicted_per_file_window",
        "type_index_artifact_evicted_per_file_window",
    ),
];

pub(super) const COMPLETION_OWNER_HINT_REASON_REGISTRY: &[(&str, &str)] = &[
    ("not_member_access", "not_member_access"),
    ("no_file_content", "no_file_content"),
    ("no_line", "no_line"),
    ("no_dot", "no_dot"),
    ("no_receiver", "no_receiver"),
    ("offset_unresolved", "offset_unresolved"),
    ("flow_type_hit", "flow_type_hit"),
    ("flow_type_miss", "flow_type_miss"),
    ("type_hit", "type_hit"),
    ("type_miss", "type_miss"),
    ("cancelled", "cancelled"),
    ("type_index_exact_hit", "type_index_exact_hit"),
    (
        "type_index_fallback_unavailable",
        "type_index_fallback_unavailable",
    ),
];

pub(super) const COMPLETION_ROUTE_REGISTRY: &[(&str, &str)] =
    &[("head_hit", "head_hit"), ("exact_hit", "exact_hit")];

pub(super) const COMPLETION_FAIL_CLOSED_CAUSE_REGISTRY: &[(&str, &str)] = &[
    ("prepare_timeout", "prepare_timeout"),
    ("exact_deadline", "exact_deadline"),
];

pub(super) const SHARED_FAIL_CLOSED_REASON_REGISTRY: &[(&str, &str)] = &[
    ("missing_canonical_ir", "missing_canonical_ir"),
    ("missing_semantic_index", "missing_semantic_index"),
    ("superseded_revision", "superseded_revision"),
    ("cancelled", "cancelled"),
    ("unavailable_by_contract", "unavailable_by_contract"),
    ("missing_ir", "missing_canonical_ir"),
    ("fallback_unavailable", "missing_semantic_index"),
    ("type_index_fallback_unavailable", "missing_semantic_index"),
    ("wait_not_ready", "missing_semantic_index"),
    ("stale_version", "superseded_revision"),
    ("superseded", "superseded_revision"),
    ("queue_rejected", "unavailable_by_contract"),
    ("missing_deps", "unavailable_by_contract"),
    ("missing_file_content", "unavailable_by_contract"),
    ("missing_file_path", "unavailable_by_contract"),
];

pub(super) const COMPLETION_EXACT_TYPE_INDEX_WAIT_REASON_REGISTRY: &[(&str, &str)] = &[
    ("ready", "ready"),
    ("deadline", "deadline"),
    ("no_matching_task", "no_matching_task"),
    ("task_present_wrong_version", "task_present_wrong_version"),
    ("observed_version_mismatch", "observed_version_mismatch"),
];

pub(super) const LEGACY_WAIT_FOR_FILE_VERSION_METRICS_REGISTRY: &[(&str, (&str, &str))] = &[
    (
        "completion",
        (
            "intellisense_v2_wait_for_file_version_completion_total",
            "intellisense_v2_wait_for_file_version_completion_ms",
        ),
    ),
    (
        "hover",
        (
            "intellisense_v2_wait_for_file_version_hover_total",
            "intellisense_v2_wait_for_file_version_hover_ms",
        ),
    ),
    (
        "signature_help",
        (
            "intellisense_v2_wait_for_file_version_signature_help_total",
            "intellisense_v2_wait_for_file_version_signature_help_ms",
        ),
    ),
    (
        "diagnostics",
        (
            "intellisense_v2_wait_for_file_version_diagnostics_total",
            "intellisense_v2_wait_for_file_version_diagnostics_ms",
        ),
    ),
];

pub(super) const LEGACY_SNAPSHOT_METRICS_REGISTRY: &[(&str, (&str, &str))] = &[
    (
        "completion",
        (
            "intellisense_v2_snapshot_completion_total",
            "intellisense_v2_snapshot_completion_ms",
        ),
    ),
    (
        "hover",
        (
            "intellisense_v2_snapshot_hover_total",
            "intellisense_v2_snapshot_hover_ms",
        ),
    ),
    (
        "signature_help",
        (
            "intellisense_v2_snapshot_signature_help_total",
            "intellisense_v2_snapshot_signature_help_ms",
        ),
    ),
    (
        "diagnostics",
        (
            "intellisense_v2_snapshot_diagnostics_total",
            "intellisense_v2_snapshot_diagnostics_ms",
        ),
    ),
];

pub(super) const LEGACY_RUNTIME_QUEUE_WAIT_METRICS_REGISTRY: &[(&str, (&str, &str))] = &[
    (
        "snapshot_with_deps",
        (
            "intellisense_v2_runtime_snapshot_with_deps_queue_wait_total",
            "intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms",
        ),
    ),
    (
        "wait_for_file_version",
        (
            "intellisense_v2_runtime_wait_for_file_version_queue_wait_total",
            "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms",
        ),
    ),
    (
        "apply_changes_batch",
        (
            "intellisense_v2_runtime_apply_changes_queue_wait_total",
            "intellisense_v2_runtime_apply_changes_queue_wait_ms",
        ),
    ),
    (
        "type_index_precompute",
        (
            "intellisense_v2_runtime_type_index_precompute_queue_wait_total",
            "intellisense_v2_runtime_type_index_precompute_queue_wait_ms",
        ),
    ),
];

pub(super) const LEGACY_RUNTIME_EXEC_METRICS_REGISTRY: &[(&str, (&str, &str))] = &[
    (
        "snapshot_with_deps",
        (
            "intellisense_v2_runtime_snapshot_with_deps_exec_total",
            "intellisense_v2_runtime_snapshot_with_deps_exec_ms",
        ),
    ),
    (
        "wait_for_file_version",
        (
            "intellisense_v2_runtime_wait_for_file_version_exec_total",
            "intellisense_v2_runtime_wait_for_file_version_exec_ms",
        ),
    ),
    (
        "apply_changes_batch",
        (
            "intellisense_v2_runtime_apply_changes_exec_total",
            "intellisense_v2_runtime_apply_changes_exec_ms",
        ),
    ),
    (
        "apply_change_set_file",
        (
            "intellisense_v2_runtime_apply_change_set_file_exec_total",
            "intellisense_v2_runtime_apply_change_set_file_exec_ms",
        ),
    ),
    (
        "apply_change_set_file_with_snapshot",
        (
            "intellisense_v2_runtime_apply_change_set_file_with_snapshot_exec_total",
            "intellisense_v2_runtime_apply_change_set_file_with_snapshot_exec_ms",
        ),
    ),
    (
        "apply_change_remove_file",
        (
            "intellisense_v2_runtime_apply_change_remove_file_exec_total",
            "intellisense_v2_runtime_apply_change_remove_file_exec_ms",
        ),
    ),
    (
        "apply_change_set_settings_snapshot",
        (
            "intellisense_v2_runtime_apply_change_set_settings_snapshot_exec_total",
            "intellisense_v2_runtime_apply_change_set_settings_snapshot_exec_ms",
        ),
    ),
    (
        "type_index_precompute",
        (
            "intellisense_v2_runtime_type_index_precompute_exec_total",
            "intellisense_v2_runtime_type_index_precompute_exec_ms",
        ),
    ),
    (
        "type_index_precompute_build",
        (
            "intellisense_v2_runtime_type_index_precompute_build_exec_total",
            "intellisense_v2_runtime_type_index_precompute_build_exec_ms",
        ),
    ),
    (
        "type_index_precompute_ir",
        (
            "intellisense_v2_runtime_type_index_precompute_ir_exec_total",
            "intellisense_v2_runtime_type_index_precompute_ir_exec_ms",
        ),
    ),
    (
        "type_index_precompute_ast_to_ir",
        (
            "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_total",
            "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms",
        ),
    ),
    (
        "type_index_precompute_semantic_facts",
        (
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_total",
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms",
        ),
    ),
    (
        "type_index_precompute_semantic_facts_seed_module_context",
        (
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_total",
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms",
        ),
    ),
    (
        "type_index_precompute_semantic_facts_local_function_summaries",
        (
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_total",
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms",
        ),
    ),
    (
        "type_index_precompute_semantic_facts_visit_statements",
        (
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_total",
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms",
        ),
    ),
];

pub(super) const LEGACY_IR_QUERY_METRICS_REGISTRY: &[(&str, (&str, &str))] = &[
    (
        "completion",
        (
            "intellisense_v2_ir_query_completion_total",
            "intellisense_v2_ir_query_completion_ms",
        ),
    ),
    (
        "hover",
        (
            "intellisense_v2_ir_query_hover_total",
            "intellisense_v2_ir_query_hover_ms",
        ),
    ),
];

pub(super) const LEGACY_IR_QUERY_CANCELLED_METRIC_REGISTRY: &[(&str, &str)] = &[
    (
        "completion",
        "intellisense_v2_ir_query_cancelled_total_completion",
    ),
    ("hover", "intellisense_v2_ir_query_cancelled_total_hover"),
];

pub(super) fn normalize_operation_label(kind: &str) -> &'static str {
    registry_label(kind, OPERATION_LABEL_REGISTRY, "other")
}

pub(super) fn normalize_runtime_stage_kind(kind: &str) -> &'static str {
    registry_label(kind, RUNTIME_STAGE_KIND_REGISTRY, "other")
}

pub(super) fn normalize_query_kind_label(kind: &str) -> &'static str {
    registry_label(kind, QUERY_KIND_REGISTRY, "other")
}

pub(super) fn normalize_reason_label(kind: &str) -> &'static str {
    registry_label(kind, REASON_LABEL_REGISTRY, "other")
}

pub(super) fn normalize_work_class_label(class: &str) -> &'static str {
    registry_label(class, WORK_CLASS_REGISTRY, "interactive")
}

pub(super) fn normalize_observability_origin_label(origin: &str) -> &'static str {
    registry_label(origin, OBSERVABILITY_ORIGIN_REGISTRY, "runtime")
}

pub(super) fn normalize_diagnostics_trigger_label(trigger: &str) -> &'static str {
    match trigger {
        "did_change" => "did_change",
        "did_open" => "did_open",
        "did_save" => "did_save",
        "idle" => "idle",
        "documents_set" => "documents_set",
        "job_start" => "job_start",
        _ => "idle",
    }
}

pub(super) fn normalize_diagnostics_profile_label(profile: &str) -> &'static str {
    match profile {
        "fast" => "fast",
        "debounced_full" => "debounced_full",
        "idle_heavy" => "idle_heavy",
        _ => "debounced_full",
    }
}

pub(super) fn normalize_diagnostics_reason_label(reason: &str) -> &'static str {
    match reason {
        "published" => "published",
        "superseded_version" => "superseded_version",
        "superseded_generation" => "superseded_generation",
        "client_cancel" => "client_cancel",
        "other_cancel" | "cancelled" => "other_cancel",
        _ => "other_cancel",
    }
}

pub(super) fn diagnostics_reason_is_cancellation(reason: &str) -> bool {
    matches!(
        reason,
        "superseded_version" | "superseded_generation" | "client_cancel" | "other_cancel"
    )
}

pub(super) fn normalize_large_churn_state_label(state: &str) -> &'static str {
    match state {
        "enter" => "enter",
        "exit" => "exit",
        _ => "enter",
    }
}

pub(super) fn normalize_heavy_deferred_reason_label(reason: &str) -> &'static str {
    match reason {
        "large_and_churn" => "large_and_churn",
        _ => "other",
    }
}

pub(super) fn normalize_parse_snapshot_mode_label(mode: &str) -> &'static str {
    match mode {
        "incremental" => "incremental",
        "reused" => "reused",
        "full" => "full",
        _ => "other",
    }
}

pub(super) fn normalize_parse_snapshot_fallback_reason_label(reason: &str) -> &'static str {
    if reason.starts_with("incremental_failed:") {
        return "incremental_failed";
    }
    match reason {
        "no_previous_tree" => "no_previous_tree",
        "no_edits_provided" => "no_edits_provided",
        _ => "other",
    }
}

pub(super) fn normalize_completion_trigger_mode_label(mode: &str) -> &'static str {
    match mode {
        "trigger_character" => "trigger_character",
        "invoked" => "invoked",
        "trigger_for_incomplete" => "trigger_for_incomplete",
        "none" => "none",
        _ => "other",
    }
}

pub(super) fn normalize_completion_parity_overlap_bucket_label(bucket: &str) -> &'static str {
    match bucket {
        "none" => "none",
        "low" => "low",
        "high" => "high",
        _ => "other",
    }
}

pub(super) fn normalize_completion_terminal_reason_label(reason: &str) -> &'static str {
    if reason == "ok_empty" {
        return "ok_empty";
    }
    normalize_shared_fail_closed_reason_label(reason)
}

pub(super) fn normalize_completion_owner_hint_reason_label(reason: &str) -> &'static str {
    registry_label(reason, COMPLETION_OWNER_HINT_REASON_REGISTRY, "other")
}

pub(super) fn normalize_completion_route_label(route: &str) -> &'static str {
    registry_label(route, COMPLETION_ROUTE_REGISTRY, "other")
}

pub(super) fn normalize_completion_fail_closed_cause_label(cause: &str) -> &'static str {
    registry_label(cause, COMPLETION_FAIL_CLOSED_CAUSE_REGISTRY, "other")
}

pub(super) fn normalize_type_index_reason_label(reason: &str) -> &'static str {
    registry_label(reason, TYPE_INDEX_REASON_REGISTRY, "other")
}

pub(super) fn normalize_completion_exact_type_index_wait_reason_label(
    reason: &str,
) -> &'static str {
    registry_label(
        reason,
        COMPLETION_EXACT_TYPE_INDEX_WAIT_REASON_REGISTRY,
        "other",
    )
}

pub(super) fn normalize_shared_fail_closed_reason_label(reason: &str) -> &'static str {
    registry_label(reason, SHARED_FAIL_CLOSED_REASON_REGISTRY, "other")
}

pub(super) fn normalize_public_completion_outcome_label(outcome: &str) -> &'static str {
    match outcome {
        "ok_non_empty" => "ok_non_empty",
        "ok_empty" => "ok_empty",
        "cancelled" | "superseded" => "cancelled",
        "handler_error" => "handler_error",
        "wait_not_ready"
        | "missing_file_content"
        | "missing_file_path"
        | "missing_deps"
        | "missing_ir"
        | "fallback_unavailable"
        | "queue_rejected" => "fail_closed",
        _ => "other",
    }
}

pub(super) fn normalize_completion_resource_reason_label(reason: &str) -> &'static str {
    match reason {
        "allocator_pressure" => "allocator_pressure",
        "lock_wait" => "lock_wait",
        "queue_backpressure" => "queue_backpressure",
        _ => "other",
    }
}

pub(super) fn normalize_completion_observability_mode_label(mode: &str) -> &'static str {
    match mode {
        "legacy" => "legacy",
        "event_driven" => "event_driven",
        "shadow" => "shadow",
        _ => "legacy",
    }
}

pub(super) fn normalize_payload_shape_stage_label(stage: &str) -> &'static str {
    match stage {
        "runtime_snapshot_with_deps" => "runtime_snapshot_with_deps",
        "runtime_wait_for_file_version" => "runtime_wait_for_file_version",
        "syntax_diagnostics_query" => "syntax_diagnostics_query",
        "semantic_diagnostics_query" => "semantic_diagnostics_query",
        "parse_result_query" => "parse_result_query",
        _ => "other",
    }
}

pub(super) fn payload_size_bucket(file_bytes: usize) -> &'static str {
    match file_bytes {
        0 => "zero",
        1..=4095 => "lt_4k",
        4096..=16383 => "lt_16k",
        16384..=65535 => "lt_64k",
        _ => "ge_64k",
    }
}

pub(super) fn payload_line_bucket(line_count: usize) -> &'static str {
    match line_count {
        0 => "zero",
        1..=99 => "lt_100",
        100..=499 => "lt_500",
        500..=1999 => "lt_2k",
        _ => "ge_2k",
    }
}

pub(super) fn legacy_wait_for_file_version_metrics(kind: &str) -> (&'static str, &'static str) {
    registry_metric_pair(
        kind,
        LEGACY_WAIT_FOR_FILE_VERSION_METRICS_REGISTRY,
        (
            "intellisense_v2_wait_for_file_version_other_total",
            "intellisense_v2_wait_for_file_version_other_ms",
        ),
    )
}

pub(super) fn legacy_snapshot_metrics(kind: &str) -> (&'static str, &'static str) {
    registry_metric_pair(
        kind,
        LEGACY_SNAPSHOT_METRICS_REGISTRY,
        (
            "intellisense_v2_snapshot_other_total",
            "intellisense_v2_snapshot_other_ms",
        ),
    )
}

pub(super) fn legacy_runtime_queue_wait_metrics(kind: &str) -> (&'static str, &'static str) {
    registry_metric_pair(
        kind,
        LEGACY_RUNTIME_QUEUE_WAIT_METRICS_REGISTRY,
        (
            "intellisense_v2_runtime_other_queue_wait_total",
            "intellisense_v2_runtime_other_queue_wait_ms",
        ),
    )
}

pub(super) fn legacy_runtime_exec_metrics(kind: &str) -> (&'static str, &'static str) {
    registry_metric_pair(
        kind,
        LEGACY_RUNTIME_EXEC_METRICS_REGISTRY,
        (
            "intellisense_v2_runtime_other_exec_total",
            "intellisense_v2_runtime_other_exec_ms",
        ),
    )
}

pub(super) fn legacy_ir_query_metrics(kind: &str) -> (&'static str, &'static str) {
    registry_metric_pair(
        kind,
        LEGACY_IR_QUERY_METRICS_REGISTRY,
        (
            "intellisense_v2_ir_query_other_total",
            "intellisense_v2_ir_query_other_ms",
        ),
    )
}

pub(super) fn legacy_ir_query_cancelled_metric(kind: &str) -> &'static str {
    registry_metric_single(
        kind,
        LEGACY_IR_QUERY_CANCELLED_METRIC_REGISTRY,
        "intellisense_v2_ir_query_cancelled_total_other",
    )
}
