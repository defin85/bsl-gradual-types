#!/usr/bin/env python3
"""Validate versioned contracts under contracts/**."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any


RE_VERSION_DIR = re.compile(r"^v([1-9]\d*)$")

REQUIRED_SURFACES = {
    "lsp-completion-v2",
    "lsp-completion-timeline",
    "intellisense-perf-gate",
    "observability-completion-v2",
}

REQUIRED_LATEST_MAJORS = {
    "lsp-completion-timeline": 19,
    "intellisense-perf-gate": 2,
    "observability-completion-v2": 4,
}

REQUIRED_V1_COMPLETION_TRIGGER_MODES = {
    "trigger_character",
    "invoked",
    "trigger_for_incomplete",
    "none",
}

REQUIRED_V1_COMPLETION_OUTCOMES = {
    "ok_non_empty",
    "ok_empty",
    "degraded_incomplete",
    "fallback_unavailable",
}

REQUIRED_V2_COMPLETION_TRANSPORT_OUTCOMES = {
    "ok_non_empty",
    "ok_empty",
}

REQUIRED_V2_COMPLETION_SEMANTIC_CONTRACT_CLASSES = {
    "exact_current_revision",
    "fail_closed_current_revision",
}

REQUIRED_V1_TERMINAL_EMPTY_REASONS = {
    "ok_empty",
    "fallback_unavailable",
    "missing_ir",
    "wait_not_ready",
}

REQUIRED_V2_ANTI_RESCUE_GUARD_COUNTERS = {
    "intellisense_v2_interactive_stale_served_total",
    "intellisense_v2_completion_stale_fallback_total",
}

REQUIRED_V2_OBSERVABILITY_COMPLETION_OUTCOMES = {
    "ok_non_empty",
    "ok_empty",
    "cancelled",
    "handler_error",
    "missing_deps",
    "missing_file_content",
    "missing_file_path",
    "missing_ir",
    "wait_not_ready",
    "fallback_unavailable",
}

REQUIRED_V3_TIMELINE_OUTCOMES = {
    "ok_non_empty",
    "ok_empty",
    "cancelled",
    "superseded",
    "handler_error",
    "fail_closed",
}

REQUIRED_V4_TIMELINE_TRACE_FIELDS = {
    "trace_id",
    "request_id",
    "uri",
    "trigger_mode",
    "outcome",
    "started_at_ms",
    "total_duration_ms",
    "dominant_stage",
    "prepare_details",
    "turn_attribution",
    "stages",
}

REQUIRED_V5_TIMELINE_TRACE_FIELDS = REQUIRED_V4_TIMELINE_TRACE_FIELDS | {
    "server_edge_details",
}

REQUIRED_V6_TIMELINE_TRACE_FIELDS = REQUIRED_V5_TIMELINE_TRACE_FIELDS

REQUIRED_V15_TIMELINE_TRACE_FIELDS = REQUIRED_V6_TIMELINE_TRACE_FIELDS | {
    "client_probe_id",
}

REQUIRED_V4_TIMELINE_PREPARE_DETAILS_FIELDS = {
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
}

REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS = REQUIRED_V4_TIMELINE_PREPARE_DETAILS_FIELDS | {
    "progress",
    "wait_for_file_version_runtime",
    "snapshot_with_deps_runtime",
    "snapshot_with_deps_timeout_runtime",
    "timeout_attribution",
    "exact_wait",
}

REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS = {
    "phase",
    "phase_started_offset_ms",
    "wait_completed_offset_ms",
    "snapshot_completed_offset_ms",
}

REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS = {
    "queue_wait_ms",
    "exec_ms",
    "wake_wait_ms",
    "resolution",
}

REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS = {
    "source",
    "phase",
    "budget_ms",
    "elapsed_ms",
    "overshoot_ms",
}

REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS = {
    "head_ready_before_wait",
    "exact_ready_before_wait",
    "current_revision_head_owner_hints_ready",
    "artifact_wait_outcome",
    "type_index_wait_outcome",
    "type_index_waiter_action",
    "matching_task_state",
    "task_phase",
    "artifact_poll",
}

REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS = {
    "poll_count",
    "poll_elapsed_ms",
    "observed_file_version",
    "head_ready",
    "exact_ready",
}

REQUIRED_V4_TIMELINE_TURN_ATTRIBUTION_FIELDS = {
    "request_file_seq",
    "request_epoch",
    "queue_outcome",
    "turn_wait_outcome",
    "queue_capacity",
    "queue_depth_before_enqueue",
    "queue_depth_after_enqueue",
    "queued_completion_ahead_count",
    "did_change_ahead_count",
    "active_completion_count",
    "dropped_completion_file_seq",
    "active_holder",
    "queued_completion_ahead",
}

REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS = REQUIRED_V4_TIMELINE_TURN_ATTRIBUTION_FIELDS | {
    "dispatcher_resolution_latency_ms",
}

REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS = {
    "request_id",
    "file_seq",
    "request_epoch",
    "trigger_mode",
    "version_hint",
    "age_ms",
}

REQUIRED_V5_TIMELINE_SERVER_EDGE_DETAILS_FIELDS = {
    "transport_received_at_ms",
    "handler_entered_at_ms",
    "response_sent_at_ms",
    "cancel_observed_at_ms",
    "transport_to_handler_wait_ms",
    "server_handler_exec_ms",
    "cancel_observed_after_handler_enter_ms",
}

REQUIRED_V6_TIMELINE_SERVER_EDGE_DETAILS_FIELDS = REQUIRED_V5_TIMELINE_SERVER_EDGE_DETAILS_FIELDS | {
    "pre_method_attribution_provenance",
    "service_future_created_at_ms",
    "service_scope_entered_at_ms",
    "method_entered_at_ms",
    "transport_to_service_future_wait_ms",
    "service_future_to_scope_wait_ms",
    "transport_to_service_scope_wait_ms",
    "service_scope_to_method_wait_ms",
    "transport_to_method_wait_ms",
    "method_prelude_exec_ms",
}

REQUIRED_V7_TIMELINE_SERVER_EDGE_DETAILS_FIELDS = REQUIRED_V6_TIMELINE_SERVER_EDGE_DETAILS_FIELDS | {
    "transport_received_at_ms_provenance",
    "jsonrpc_dispatch_received_at_ms",
    "dispatch_to_request_context_wait_ms",
}

REQUIRED_V8_TIMELINE_SERVER_EDGE_DETAILS_FIELDS = REQUIRED_V7_TIMELINE_SERVER_EDGE_DETAILS_FIELDS | {
    "service_future_first_poll_entered_at_ms",
    "service_future_first_poll_outcome",
    "service_future_first_wake_scheduled_at_ms",
    "service_future_to_first_poll_wait_ms",
    "first_poll_to_first_wake_wait_ms",
}

REQUIRED_V9_TIMELINE_SERVER_EDGE_DETAILS_FIELDS = REQUIRED_V8_TIMELINE_SERVER_EDGE_DETAILS_FIELDS | {
    "first_poll_contention_attribution",
}

REQUIRED_V10_TIMELINE_SERVER_EDGE_DETAILS_FIELDS = REQUIRED_V9_TIMELINE_SERVER_EDGE_DETAILS_FIELDS | {
    "first_poll_contention_contenders",
}

REQUIRED_V14_TIMELINE_SERVER_EDGE_DETAILS_FIELDS = (
    REQUIRED_V10_TIMELINE_SERVER_EDGE_DETAILS_FIELDS
    | {
        "transport_slot_released_at_ms",
        "transport_to_slot_release_wait_ms",
        "slot_release_to_handler_wait_ms",
        "slot_release_to_response_wait_ms",
    }
)

REQUIRED_V16_TIMELINE_SERVER_EDGE_DETAILS_FIELDS = (
    REQUIRED_V14_TIMELINE_SERVER_EDGE_DETAILS_FIELDS
    | {
        "adapter_read_at_ms",
        "adapter_to_dispatch_wait_ms",
    }
)

REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS = {
    "contender_class",
    "uri_scope",
    "inflight_count",
    "oldest_inflight_age_ms",
    "concurrency_level",
}

REQUIRED_V10_TIMELINE_FIRST_POLL_CONTENDER_FIELDS = {
    "request_class",
    "method",
    "uri",
    "age_ms",
}

REQUIRED_V11_TIMELINE_FIRST_POLL_CONTENDER_FIELDS = (
    REQUIRED_V10_TIMELINE_FIRST_POLL_CONTENDER_FIELDS | {"command"}
)

REQUIRED_V12_TIMELINE_FIRST_POLL_CONTENDER_FIELDS = (
    REQUIRED_V11_TIMELINE_FIRST_POLL_CONTENDER_FIELDS | {"phase"}
)

REQUIRED_V13_TIMELINE_TURN_ATTRIBUTION_FIELDS = REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS | {
    "turn_wait_entered_at_ms",
    "turn_wait_resolved_at_ms",
    "wake_after_turn_resolution_at_ms",
}

REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES = {
    "document_sync",
    "completion",
    "other_request",
    "other_notification",
    "mixed",
    "none_visible",
    "unavailable",
}

REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES = {
    "document_sync",
    "completion",
    "other_request",
    "other_notification",
}

REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES = {
    "same_uri",
    "other_uri",
    "mixed",
    "unavailable",
}

REQUIRED_V17_TIMELINE_QUERY_BUNDLE_STAGE_NAMES = {
    "query_bundle_pool_wait",
    "query_bundle_deps_and_file_snapshot",
    "query_bundle_owner_hint",
    "query_bundle_ir_query",
    "query_bundle_ir_retry",
    "query_bundle_other",
}

REQUIRED_V18_TIMELINE_SERVER_EDGE_DETAILS_FIELDS = (
    REQUIRED_V16_TIMELINE_SERVER_EDGE_DETAILS_FIELDS
    | {
        "response_flush_completed_at_ms",
        "response_ready_to_flush_wait_ms",
    }
)

REQUIRED_V19_TIMELINE_SERVER_EDGE_DETAILS_FIELDS = (
    REQUIRED_V18_TIMELINE_SERVER_EDGE_DETAILS_FIELDS
    | {
        "response_output_enqueue_completed_at_ms",
        "response_output_write_started_at_ms",
        "response_output_encode_completed_at_ms",
        "response_ready_to_output_enqueue_wait_ms",
        "response_output_queue_wait_ms",
        "response_output_encode_exec_ms",
        "response_output_write_and_flush_exec_ms",
    }
)

REQUIRED_V4_COMPLETION_ROUTES = {
    "head_hit",
    "exact_hit",
}

REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES = {
    "prepare_timeout",
    "exact_deadline",
}

REQUIRED_V3_TERMINAL_EMPTY_REASONS = {
    "ok_empty",
    "missing_canonical_ir",
    "missing_semantic_index",
    "superseded_revision",
    "cancelled",
    "unavailable_by_contract",
}

REQUIRED_V3_OBSERVABILITY_COMPLETION_OUTCOMES = {
    "ok_non_empty",
    "ok_empty",
    "fail_closed",
    "cancelled",
    "handler_error",
}

REQUIRED_V4_OBSERVABILITY_COMPLETION_OUTCOMES = REQUIRED_V3_OBSERVABILITY_COMPLETION_OUTCOMES

REQUIRED_V3_FAIL_CLOSED_REASONS = {
    "missing_canonical_ir",
    "missing_semantic_index",
    "superseded_revision",
    "cancelled",
    "unavailable_by_contract",
}

REQUIRED_V3_FAIL_CLOSED_ORIGINS = {
    "lsp",
    "web",
    "agent",
    "runtime",
}

REQUIRED_V3_FAIL_CLOSED_OPERATIONS = {
    "completion",
    "hover",
    "signature_help",
    "definition",
    "members",
    "type_at_position",
}

REQUIRED_V1_PERF_GATE_PROFILES = {
    "small",
    "large",
    "churn",
}

REQUIRED_V1_PERF_GATE_LATENCY_METRICS = {
    "completion_duration_ms",
    "intellisense_v2_wait_for_file_version_completion_ms",
    "intellisense_v2_snapshot_completion_ms",
    "intellisense_v2_ir_query_completion_ms",
}

REQUIRED_V1_PERF_GATE_RESOURCE_METRICS = {
    "allocations_per_completion",
    "allocated_bytes_per_completion",
    "lock_wait_ms_per_completion",
    "lock_contention_events_per_completion",
}

REQUIRED_V1_PERF_GATE_REASON_CODES = {
    "missing_required_metric_field",
    "unsupported_contract_version",
    "latency_relative_ratio_exceeded",
    "latency_absolute_ceiling_exceeded",
    "allocation_budget_exceeded",
    "lock_wait_budget_exceeded",
    "lock_contention_budget_exceeded",
    "protected_acceptance_asset_modified",
    "change_criticality_missing_or_unknown",
    "test_first_evidence_missing_or_invalid",
    "initial_budget_not_fixed",
    "perf_gate_architecture_violation",
}

REQUIRED_V2_PERF_GATE_OPERATIONS = {
    "completion",
    "hover",
    "definition",
    "type_at_position",
    "members",
}

REQUIRED_V2_PERF_GATE_FIXTURE_FAMILIES = {
    "steady_member_chain",
    "post_did_change_current_revision",
    "object_module_explicit_context",
    "recordset_module_explicit_context",
    "incomplete_syntax_member_access",
}

REQUIRED_V2_PERF_GATE_OPERATION_MATRIX = {
    "steady_member_chain": REQUIRED_V2_PERF_GATE_OPERATIONS,
    "post_did_change_current_revision": REQUIRED_V2_PERF_GATE_OPERATIONS,
    "object_module_explicit_context": REQUIRED_V2_PERF_GATE_OPERATIONS,
    "recordset_module_explicit_context": REQUIRED_V2_PERF_GATE_OPERATIONS,
    "incomplete_syntax_member_access": {"completion"},
}

REQUIRED_V2_PERF_GATE_LATENCY_METRIC_FAMILIES = {
    "total_duration_ms",
    "wait_for_file_version_ms",
    "snapshot_preparation_ms",
    "ir_query_ms",
}

REQUIRED_V2_PERF_GATE_RESOURCE_METRIC_FAMILIES = {
    "allocations_per_request",
    "allocated_bytes_per_request",
    "lock_wait_ms_per_request",
    "lock_contention_events_per_request",
}

REQUIRED_V2_PERF_GATE_FAIL_CLOSED_BUDGET = {
    "fail_closed_total": 0,
    "fail_closed_rate": 0.0,
}

REQUIRED_V2_PERF_GATE_RATIO_BASELINE_FLOORS = {
    "total_duration_ms": 6,
    "wait_for_file_version_ms": 3,
    "snapshot_preparation_ms": 5,
    "ir_query_ms": 3,
    "allocations_per_request": 100,
    "allocated_bytes_per_request": 8192,
    "lock_wait_ms_per_request": 1,
    "lock_contention_events_per_request": 1,
}

REQUIRED_V2_PERF_GATE_REPORT_FIELDS = {
    "contract_version",
    "profile",
    "coverage",
    "results",
    "verdict",
    "reason_codes",
}

REQUIRED_V2_PERF_GATE_REASON_CODES = REQUIRED_V1_PERF_GATE_REASON_CODES | {
    "missing_required_matrix_coverage",
    "anti_rescue_budget_exceeded",
    "fail_closed_budget_exceeded",
    "provenance_missing_for_authoritative_run",
    "provenance_mismatch_expected_change_id",
    "provenance_invalid",
    "provenance_non_authoritative_cutover_evidence",
    "parity_evidence_insufficient",
    "parity_drift_threshold_exceeded",
}


class ValidationError(Exception):
    pass


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def ensure(cond: bool, message: str) -> None:
    if not cond:
        raise ValidationError(message)


def parse_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ValidationError(f"{path}: invalid JSON: {exc}") from exc


def type_matches(value: Any, expected: str) -> bool:
    if expected == "object":
        return isinstance(value, dict)
    if expected == "array":
        return isinstance(value, list)
    if expected == "string":
        return isinstance(value, str)
    if expected == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected == "boolean":
        return isinstance(value, bool)
    return True


def validate_schema_like(value: Any, schema: dict[str, Any], where: str) -> None:
    expected_type = schema.get("type")
    if expected_type is not None:
        ensure(
            type_matches(value, expected_type),
            f"{where}: expected type={expected_type}, got {type(value).__name__}",
        )

    if "const" in schema:
        ensure(value == schema["const"], f"{where}: expected const={schema['const']!r}")

    if "enum" in schema:
        ensure(value in schema["enum"], f"{where}: value {value!r} is not in enum")

    if expected_type == "object":
        required = schema.get("required", [])
        for key in required:
            ensure(key in value, f"{where}: missing required key {key!r}")

        properties = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            unknown = sorted(set(value.keys()) - set(properties.keys()))
            ensure(not unknown, f"{where}: unexpected keys: {unknown}")

        for key, child_schema in properties.items():
            if key in value:
                validate_schema_like(value[key], child_schema, f"{where}.{key}")

    if expected_type == "array":
        min_items = schema.get("minItems")
        if min_items is not None:
            ensure(len(value) >= min_items, f"{where}: expected minItems={min_items}")

        if schema.get("uniqueItems") is True:
            normalized = [json.dumps(item, sort_keys=True, ensure_ascii=False) for item in value]
            ensure(
                len(normalized) == len(set(normalized)),
                f"{where}: array items must be unique",
            )

        item_schema = schema.get("items")
        if item_schema is not None:
            for idx, item in enumerate(value):
                validate_schema_like(item, item_schema, f"{where}[{idx}]")


def validate_lsp_completion_timeline_response_fields(
    contract_path: Path,
    response: dict[str, Any],
    expected_version: int,
    expected_server_edge_details_fields: set[str],
    expected_query_bundle_stage_names: set[str] | None = None,
) -> None:
    ensure(
        response.get("version") == expected_version,
        f"{contract_path}: response.version must equal {expected_version}",
    )
    outcomes = set(response.get("outcomes", []))
    trace_fields = set(response.get("trace_fields", []))
    prepare_details_fields = set(response.get("prepare_details_fields", []))
    prepare_progress_fields = set(response.get("prepare_progress_fields", []))
    prepare_runtime_fields = set(response.get("prepare_runtime_fields", []))
    prepare_timeout_attribution_fields = set(
        response.get("prepare_timeout_attribution_fields", [])
    )
    exact_wait_fields = set(response.get("exact_wait_fields", []))
    exact_artifact_poll_fields = set(response.get("exact_artifact_poll_fields", []))
    server_edge_details_fields = set(response.get("server_edge_details_fields", []))
    first_poll_contention_fields = set(
        response.get("first_poll_contention_attribution_fields", [])
    )
    first_poll_contention_contender_fields = set(
        response.get("first_poll_contention_contender_fields", [])
    )
    first_poll_contention_classes = set(
        response.get("first_poll_contention_contender_classes", [])
    )
    first_poll_contention_request_classes = set(
        response.get("first_poll_contention_request_classes", [])
    )
    first_poll_contention_uri_scopes = set(
        response.get("first_poll_contention_uri_scopes", [])
    )
    turn_attribution_fields = set(response.get("turn_attribution_fields", []))
    turn_holder_fields = set(response.get("turn_holder_fields", []))
    query_bundle_stage_names = set(response.get("query_bundle_stage_names", []))
    prepare_routes = set(response.get("prepare_routes", []))
    prepare_fail_closed_causes = set(response.get("prepare_fail_closed_causes", []))
    ensure(
        outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
        f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
    )
    ensure(
        trace_fields == REQUIRED_V15_TIMELINE_TRACE_FIELDS,
        f"{contract_path}: response.trace_fields must equal {sorted(REQUIRED_V15_TIMELINE_TRACE_FIELDS)}",
    )
    ensure(
        prepare_details_fields == REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS,
        f"{contract_path}: response.prepare_details_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS)}",
    )
    ensure(
        prepare_progress_fields == REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS,
        f"{contract_path}: response.prepare_progress_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS)}",
    )
    ensure(
        prepare_runtime_fields == REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS,
        f"{contract_path}: response.prepare_runtime_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS)}",
    )
    ensure(
        prepare_timeout_attribution_fields
        == REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS,
        f"{contract_path}: response.prepare_timeout_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS)}",
    )
    ensure(
        exact_wait_fields == REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS,
        f"{contract_path}: response.exact_wait_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS)}",
    )
    ensure(
        exact_artifact_poll_fields == REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS,
        f"{contract_path}: response.exact_artifact_poll_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS)}",
    )
    ensure(
        server_edge_details_fields == expected_server_edge_details_fields,
        f"{contract_path}: response.server_edge_details_fields must equal {sorted(expected_server_edge_details_fields)}",
    )
    ensure(
        first_poll_contention_fields == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS,
        f"{contract_path}: response.first_poll_contention_attribution_fields must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS)}",
    )
    ensure(
        first_poll_contention_contender_fields
        == REQUIRED_V12_TIMELINE_FIRST_POLL_CONTENDER_FIELDS,
        f"{contract_path}: response.first_poll_contention_contender_fields must equal {sorted(REQUIRED_V12_TIMELINE_FIRST_POLL_CONTENDER_FIELDS)}",
    )
    ensure(
        first_poll_contention_classes == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES,
        f"{contract_path}: response.first_poll_contention_contender_classes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES)}",
    )
    ensure(
        first_poll_contention_request_classes
        == REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES,
        f"{contract_path}: response.first_poll_contention_request_classes must equal {sorted(REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES)}",
    )
    ensure(
        first_poll_contention_uri_scopes == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES,
        f"{contract_path}: response.first_poll_contention_uri_scopes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES)}",
    )
    ensure(
        turn_attribution_fields == REQUIRED_V13_TIMELINE_TURN_ATTRIBUTION_FIELDS,
        f"{contract_path}: response.turn_attribution_fields must equal {sorted(REQUIRED_V13_TIMELINE_TURN_ATTRIBUTION_FIELDS)}",
    )
    ensure(
        turn_holder_fields == REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS,
        f"{contract_path}: response.turn_holder_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS)}",
    )
    if expected_query_bundle_stage_names is not None:
        ensure(
            query_bundle_stage_names == expected_query_bundle_stage_names,
            f"{contract_path}: response.query_bundle_stage_names must equal {sorted(expected_query_bundle_stage_names)}",
        )
    ensure(
        prepare_routes == REQUIRED_V4_COMPLETION_ROUTES,
        f"{contract_path}: response.prepare_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
    )
    ensure(
        prepare_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
        f"{contract_path}: response.prepare_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
    )


def parse_major(version_dir_name: str, where: Path) -> int:
    match = RE_VERSION_DIR.match(version_dir_name)
    ensure(match is not None, f"{where}: invalid version directory {version_dir_name!r}")
    return int(match.group(1))


def validate_surface_contract(surface_dir: Path) -> None:
    ensure(surface_dir.is_dir(), f"{surface_dir}: surface directory is missing")
    versions = sorted(
        (
            (parse_major(child.name, child), child)
            for child in surface_dir.iterdir()
            if child.is_dir() and RE_VERSION_DIR.match(child.name)
        ),
        key=lambda item: item[0],
    )

    ensure(versions, f"{surface_dir}: no version directories found")
    majors = [major for major, _ in versions]
    expected = list(range(1, max(majors) + 1))
    ensure(
        majors == expected,
        f"{surface_dir}: expected contiguous versions {expected}, got {majors}",
    )
    expected_latest_major = REQUIRED_LATEST_MAJORS.get(surface_dir.name)
    if expected_latest_major is not None:
        ensure(
            majors[-1] == expected_latest_major,
            f"{surface_dir}: expected latest major v{expected_latest_major}, got v{majors[-1]}",
        )

    for major, version_dir in versions:
        contract_path = version_dir / "contract.json"
        schema_path = version_dir / "schema.json"
        changelog_path = version_dir / "changelog.md"

        ensure(contract_path.exists(), f"{contract_path}: missing")
        ensure(schema_path.exists(), f"{schema_path}: missing")
        ensure(changelog_path.exists(), f"{changelog_path}: missing")

        contract = parse_json(contract_path)
        schema = parse_json(schema_path)
        ensure(isinstance(schema, dict), f"{schema_path}: schema root must be object")
        validate_schema_like(contract, schema, str(contract_path))

        ensure(contract.get("surface") == surface_dir.name, f"{contract_path}: surface mismatch")
        ensure(contract.get("major_version") == major, f"{contract_path}: major_version mismatch")
        compatibility = contract.get("compatibility")
        ensure(isinstance(compatibility, dict), f"{contract_path}: compatibility must be object")
        ensure(
            compatibility.get("breaking_change_requires_major_bump") is True,
            f"{contract_path}: compatibility.breaking_change_requires_major_bump must be true",
        )
        ensure(
            compatibility.get("breaking_change_requires_migration_note") is True,
            f"{contract_path}: compatibility.breaking_change_requires_migration_note must be true",
        )

        changelog_text = changelog_path.read_text(encoding="utf-8")
        if major > 1:
            ensure(
                "migration note:" in changelog_text.lower(),
                f"{changelog_path}: Migration note is required for major>1",
            )

        if surface_dir.name == "lsp-completion-v2" and major == 1:
            completion = contract.get("completion")
            ensure(isinstance(completion, dict), f"{contract_path}: completion must be object")
            trigger_modes = set(completion.get("trigger_modes", []))
            outcomes = set(completion.get("outcomes", []))
            ensure(
                REQUIRED_V1_COMPLETION_TRIGGER_MODES.issubset(trigger_modes),
                f"{contract_path}: trigger_modes must include {sorted(REQUIRED_V1_COMPLETION_TRIGGER_MODES)}",
            )
            ensure(
                REQUIRED_V1_COMPLETION_OUTCOMES.issubset(outcomes),
                f"{contract_path}: outcomes must include {sorted(REQUIRED_V1_COMPLETION_OUTCOMES)}",
            )

        if surface_dir.name == "lsp-completion-v2" and major == 2:
            completion = contract.get("completion")
            ensure(isinstance(completion, dict), f"{contract_path}: completion must be object")
            trigger_modes = set(completion.get("trigger_modes", []))
            transport_outcomes = set(completion.get("transport_outcomes", []))
            semantic_contract_classes = set(completion.get("semantic_contract_classes", []))
            ensure(
                REQUIRED_V1_COMPLETION_TRIGGER_MODES.issubset(trigger_modes),
                f"{contract_path}: trigger_modes must include {sorted(REQUIRED_V1_COMPLETION_TRIGGER_MODES)}",
            )
            ensure(
                transport_outcomes == REQUIRED_V2_COMPLETION_TRANSPORT_OUTCOMES,
                f"{contract_path}: transport_outcomes must equal {sorted(REQUIRED_V2_COMPLETION_TRANSPORT_OUTCOMES)}",
            )
            ensure(
                semantic_contract_classes == REQUIRED_V2_COMPLETION_SEMANTIC_CONTRACT_CLASSES,
                f"{contract_path}: semantic_contract_classes must equal {sorted(REQUIRED_V2_COMPLETION_SEMANTIC_CONTRACT_CLASSES)}",
            )

        if surface_dir.name == "observability-completion-v2" and major == 1:
            metrics = contract.get("metrics")
            ensure(isinstance(metrics, dict), f"{contract_path}: metrics must be object")
            trigger_modes = set(metrics.get("allowed_trigger_modes", []))
            terminal_reasons = set(metrics.get("allowed_terminal_empty_reasons", []))
            ensure(
                REQUIRED_V1_COMPLETION_TRIGGER_MODES.issubset(trigger_modes),
                f"{contract_path}: allowed_trigger_modes must include {sorted(REQUIRED_V1_COMPLETION_TRIGGER_MODES)}",
            )
            ensure(
                REQUIRED_V1_TERMINAL_EMPTY_REASONS.issubset(terminal_reasons),
                f"{contract_path}: allowed_terminal_empty_reasons must include {sorted(REQUIRED_V1_TERMINAL_EMPTY_REASONS)}",
            )
            ensure(
                metrics.get("fallback_unavailable_counter")
                == "intellisense_v2_completion_result_total_fallback_unavailable",
                f"{contract_path}: fallback_unavailable_counter mismatch",
            )

        if surface_dir.name == "observability-completion-v2" and major == 2:
            metrics = contract.get("metrics")
            ensure(isinstance(metrics, dict), f"{contract_path}: metrics must be object")
            trigger_modes = set(metrics.get("allowed_trigger_modes", []))
            terminal_reasons = set(metrics.get("allowed_terminal_empty_reasons", []))
            anti_rescue_guard_counters = set(
                metrics.get("anti_rescue_guard_zero_expected_counters", [])
            )
            completion_outcomes = set(metrics.get("allowed_completion_outcomes", []))
            ensure(
                REQUIRED_V1_COMPLETION_TRIGGER_MODES.issubset(trigger_modes),
                f"{contract_path}: allowed_trigger_modes must include {sorted(REQUIRED_V1_COMPLETION_TRIGGER_MODES)}",
            )
            ensure(
                REQUIRED_V1_TERMINAL_EMPTY_REASONS.issubset(terminal_reasons),
                f"{contract_path}: allowed_terminal_empty_reasons must include {sorted(REQUIRED_V1_TERMINAL_EMPTY_REASONS)}",
            )
            ensure(
                anti_rescue_guard_counters == REQUIRED_V2_ANTI_RESCUE_GUARD_COUNTERS,
                f"{contract_path}: anti_rescue_guard_zero_expected_counters must equal {sorted(REQUIRED_V2_ANTI_RESCUE_GUARD_COUNTERS)}",
            )
            ensure(
                completion_outcomes == REQUIRED_V2_OBSERVABILITY_COMPLETION_OUTCOMES,
                f"{contract_path}: allowed_completion_outcomes must equal {sorted(REQUIRED_V2_OBSERVABILITY_COMPLETION_OUTCOMES)}",
            )
            ensure(
                metrics.get("completion_result_counter_prefix")
                == "intellisense_v2_completion_result_total_",
                f"{contract_path}: completion_result_counter_prefix mismatch",
            )

        if surface_dir.name == "observability-completion-v2" and major == 3:
            metrics = contract.get("metrics")
            ensure(isinstance(metrics, dict), f"{contract_path}: metrics must be object")
            trigger_modes = set(metrics.get("allowed_trigger_modes", []))
            terminal_reasons = set(metrics.get("allowed_terminal_empty_reasons", []))
            anti_rescue_guard_counters = set(
                metrics.get("anti_rescue_guard_zero_expected_counters", [])
            )
            completion_outcomes = set(metrics.get("allowed_completion_outcomes", []))
            fail_closed_reasons = set(metrics.get("allowed_fail_closed_reasons", []))
            origins = set(metrics.get("allowed_fail_closed_origins", []))
            operations = set(metrics.get("allowed_fail_closed_operations", []))
            ensure(
                REQUIRED_V1_COMPLETION_TRIGGER_MODES.issubset(trigger_modes),
                f"{contract_path}: allowed_trigger_modes must include {sorted(REQUIRED_V1_COMPLETION_TRIGGER_MODES)}",
            )
            ensure(
                terminal_reasons == REQUIRED_V3_TERMINAL_EMPTY_REASONS,
                f"{contract_path}: allowed_terminal_empty_reasons must equal {sorted(REQUIRED_V3_TERMINAL_EMPTY_REASONS)}",
            )
            ensure(
                anti_rescue_guard_counters == REQUIRED_V2_ANTI_RESCUE_GUARD_COUNTERS,
                f"{contract_path}: anti_rescue_guard_zero_expected_counters must equal {sorted(REQUIRED_V2_ANTI_RESCUE_GUARD_COUNTERS)}",
            )
            ensure(
                completion_outcomes == REQUIRED_V3_OBSERVABILITY_COMPLETION_OUTCOMES,
                f"{contract_path}: allowed_completion_outcomes must equal {sorted(REQUIRED_V3_OBSERVABILITY_COMPLETION_OUTCOMES)}",
            )
            ensure(
                fail_closed_reasons == REQUIRED_V3_FAIL_CLOSED_REASONS,
                f"{contract_path}: allowed_fail_closed_reasons must equal {sorted(REQUIRED_V3_FAIL_CLOSED_REASONS)}",
            )
            ensure(
                origins == REQUIRED_V3_FAIL_CLOSED_ORIGINS,
                f"{contract_path}: allowed_fail_closed_origins must equal {sorted(REQUIRED_V3_FAIL_CLOSED_ORIGINS)}",
            )
            ensure(
                operations == REQUIRED_V3_FAIL_CLOSED_OPERATIONS,
                f"{contract_path}: allowed_fail_closed_operations must equal {sorted(REQUIRED_V3_FAIL_CLOSED_OPERATIONS)}",
            )
            ensure(
                metrics.get("completion_result_counter_prefix")
                == "intellisense_v2_completion_result_total_",
                f"{contract_path}: completion_result_counter_prefix mismatch",
            )
            ensure(
                metrics.get("interactive_fail_closed_reason_counter_prefix")
                == "intellisense_v2_fail_closed_reason_total_origin_",
                f"{contract_path}: interactive_fail_closed_reason_counter_prefix mismatch",
            )

        if surface_dir.name == "observability-completion-v2" and major == 4:
            metrics = contract.get("metrics")
            ensure(isinstance(metrics, dict), f"{contract_path}: metrics must be object")
            trigger_modes = set(metrics.get("allowed_trigger_modes", []))
            terminal_reasons = set(metrics.get("allowed_terminal_empty_reasons", []))
            anti_rescue_guard_counters = set(
                metrics.get("anti_rescue_guard_zero_expected_counters", [])
            )
            completion_outcomes = set(metrics.get("allowed_completion_outcomes", []))
            fail_closed_reasons = set(metrics.get("allowed_fail_closed_reasons", []))
            origins = set(metrics.get("allowed_fail_closed_origins", []))
            operations = set(metrics.get("allowed_fail_closed_operations", []))
            completion_routes = set(metrics.get("allowed_completion_routes", []))
            completion_fail_closed_causes = set(
                metrics.get("allowed_completion_fail_closed_causes", [])
            )
            ensure(
                trigger_modes == REQUIRED_V1_COMPLETION_TRIGGER_MODES,
                f"{contract_path}: allowed_trigger_modes must equal {sorted(REQUIRED_V1_COMPLETION_TRIGGER_MODES)}",
            )
            ensure(
                terminal_reasons == REQUIRED_V3_TERMINAL_EMPTY_REASONS,
                f"{contract_path}: allowed_terminal_empty_reasons must equal {sorted(REQUIRED_V3_TERMINAL_EMPTY_REASONS)}",
            )
            ensure(
                anti_rescue_guard_counters == REQUIRED_V2_ANTI_RESCUE_GUARD_COUNTERS,
                f"{contract_path}: anti_rescue_guard_zero_expected_counters must equal {sorted(REQUIRED_V2_ANTI_RESCUE_GUARD_COUNTERS)}",
            )
            ensure(
                completion_outcomes == REQUIRED_V4_OBSERVABILITY_COMPLETION_OUTCOMES,
                f"{contract_path}: allowed_completion_outcomes must equal {sorted(REQUIRED_V4_OBSERVABILITY_COMPLETION_OUTCOMES)}",
            )
            ensure(
                fail_closed_reasons == REQUIRED_V3_FAIL_CLOSED_REASONS,
                f"{contract_path}: allowed_fail_closed_reasons must equal {sorted(REQUIRED_V3_FAIL_CLOSED_REASONS)}",
            )
            ensure(
                origins == REQUIRED_V3_FAIL_CLOSED_ORIGINS,
                f"{contract_path}: allowed_fail_closed_origins must equal {sorted(REQUIRED_V3_FAIL_CLOSED_ORIGINS)}",
            )
            ensure(
                operations == REQUIRED_V3_FAIL_CLOSED_OPERATIONS,
                f"{contract_path}: allowed_fail_closed_operations must equal {sorted(REQUIRED_V3_FAIL_CLOSED_OPERATIONS)}",
            )
            ensure(
                completion_routes == REQUIRED_V4_COMPLETION_ROUTES,
                f"{contract_path}: allowed_completion_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
            )
            ensure(
                completion_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
                f"{contract_path}: allowed_completion_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
            )
            ensure(
                metrics.get("completion_result_counter_prefix")
                == "intellisense_v2_completion_result_total_",
                f"{contract_path}: completion_result_counter_prefix mismatch",
            )
            ensure(
                metrics.get("completion_route_counter_prefix")
                == "intellisense_v2_completion_route_total_route_",
                f"{contract_path}: completion_route_counter_prefix mismatch",
            )
            ensure(
                metrics.get("completion_fail_closed_cause_counter_prefix")
                == "intellisense_v2_completion_fail_closed_cause_total_cause_",
                f"{contract_path}: completion_fail_closed_cause_counter_prefix mismatch",
            )
            ensure(
                metrics.get("completion_head_to_exact_upgrade_counter")
                == "intellisense_v2_completion_head_to_exact_upgrade_total",
                f"{contract_path}: completion_head_to_exact_upgrade_counter mismatch",
            )
            ensure(
                metrics.get("completion_head_to_exact_upgrade_histogram")
                == "intellisense_v2_completion_head_to_exact_upgrade_ms",
                f"{contract_path}: completion_head_to_exact_upgrade_histogram mismatch",
            )
            ensure(
                metrics.get("interactive_fail_closed_reason_counter_prefix")
                == "intellisense_v2_fail_closed_reason_total_origin_",
                f"{contract_path}: interactive_fail_closed_reason_counter_prefix mismatch",
            )

        if surface_dir.name == "intellisense-perf-gate" and major in {1, 2}:
            input_obj = contract.get("input")
            ensure(isinstance(input_obj, dict), f"{contract_path}: input must be object")
            required_profiles = set(input_obj.get("required_profiles", []))

            ensure(
                REQUIRED_V1_PERF_GATE_PROFILES.issubset(required_profiles),
                f"{contract_path}: required_profiles must include {sorted(REQUIRED_V1_PERF_GATE_PROFILES)}",
            )

            baseline = contract.get("baseline")
            ensure(isinstance(baseline, dict), f"{contract_path}: baseline must be object")

            bootstrap_policy = baseline.get("bootstrap_policy")
            ensure(
                isinstance(bootstrap_policy, dict),
                f"{contract_path}: baseline.bootstrap_policy must be object",
            )
            bootstrap_profiles = set(bootstrap_policy.get("required_profiles", []))
            ensure(
                REQUIRED_V1_PERF_GATE_PROFILES.issubset(bootstrap_profiles),
                f"{contract_path}: baseline.bootstrap_policy.required_profiles must include {sorted(REQUIRED_V1_PERF_GATE_PROFILES)}",
            )
            sample_size_min = bootstrap_policy.get("sample_size_min")
            ensure(
                isinstance(sample_size_min, int) and sample_size_min >= 5,
                f"{contract_path}: baseline.bootstrap_policy.sample_size_min must be integer >= 5",
            )
            ensure(
                bootstrap_policy.get("aggregation_rule") == "median",
                f"{contract_path}: baseline.bootstrap_policy.aggregation_rule must be 'median'",
            )

            report = contract.get("report")
            ensure(isinstance(report, dict), f"{contract_path}: report must be object")
            required_report_fields = set(report.get("required_fields", []))
            ensure(
                {"contract_version", "verdict", "reason_codes"}.issubset(required_report_fields),
                f"{contract_path}: report.required_fields must include contract_version/verdict/reason_codes",
            )

            evaluator = contract.get("evaluator")
            ensure(isinstance(evaluator, dict), f"{contract_path}: evaluator must be object")
            reason_codes = set(evaluator.get("reason_codes", []))

            if major == 1:
                required_latency_metrics = set(input_obj.get("required_latency_metrics", []))
                required_resource_metrics = set(input_obj.get("required_resource_metrics", []))
                ensure(
                    REQUIRED_V1_PERF_GATE_LATENCY_METRICS.issubset(required_latency_metrics),
                    f"{contract_path}: required_latency_metrics must include {sorted(REQUIRED_V1_PERF_GATE_LATENCY_METRICS)}",
                )
                ensure(
                    REQUIRED_V1_PERF_GATE_RESOURCE_METRICS.issubset(required_resource_metrics),
                    f"{contract_path}: required_resource_metrics must include {sorted(REQUIRED_V1_PERF_GATE_RESOURCE_METRICS)}",
                )

                ceilings = baseline.get("absolute_latency_ceilings_ms")
                ensure(
                    isinstance(ceilings, dict),
                    f"{contract_path}: baseline.absolute_latency_ceilings_ms must be object",
                )
                for profile in sorted(REQUIRED_V1_PERF_GATE_PROFILES):
                    profile_ceiling = ceilings.get(profile)
                    ensure(
                        isinstance(profile_ceiling, dict),
                        f"{contract_path}: missing ceiling for profile {profile!r}",
                    )
                    p95 = profile_ceiling.get("p95")
                    p99 = profile_ceiling.get("p99")
                    ensure(
                        isinstance(p95, int) and p95 > 0,
                        f"{contract_path}: {profile}.p95 must be positive integer",
                    )
                    ensure(
                        isinstance(p99, int) and p99 > 0,
                        f"{contract_path}: {profile}.p99 must be positive integer",
                    )

                resource_ceilings = baseline.get("resource_budget_ceilings")
                ensure(
                    isinstance(resource_ceilings, dict),
                    f"{contract_path}: baseline.resource_budget_ceilings must be object",
                )
                for profile in sorted(REQUIRED_V1_PERF_GATE_PROFILES):
                    profile_budget = resource_ceilings.get(profile)
                    ensure(
                        isinstance(profile_budget, dict),
                        f"{contract_path}: missing resource budget for profile {profile!r}",
                    )
                    for key in sorted(REQUIRED_V1_PERF_GATE_RESOURCE_METRICS):
                        value = profile_budget.get(key)
                        ensure(
                            isinstance(value, int) and value > 0,
                            f"{contract_path}: {profile}.{key} must be positive integer",
                        )

                ensure(
                    REQUIRED_V1_PERF_GATE_REASON_CODES.issubset(reason_codes),
                    f"{contract_path}: evaluator.reason_codes must include {sorted(REQUIRED_V1_PERF_GATE_REASON_CODES)}",
                )

            if major == 2:
                operation_matrix = input_obj.get("required_operation_matrix")
                ensure(
                    isinstance(operation_matrix, dict),
                    f"{contract_path}: input.required_operation_matrix must be object",
                )
                ensure(
                    set(operation_matrix.keys()) == set(REQUIRED_V2_PERF_GATE_OPERATION_MATRIX.keys()),
                    f"{contract_path}: input.required_operation_matrix keys must equal {sorted(REQUIRED_V2_PERF_GATE_OPERATION_MATRIX.keys())}",
                )
                for fixture_family, expected_operations in REQUIRED_V2_PERF_GATE_OPERATION_MATRIX.items():
                    actual_operations = set(operation_matrix.get(fixture_family, []))
                    ensure(
                        actual_operations == expected_operations,
                        f"{contract_path}: input.required_operation_matrix[{fixture_family!r}] must equal {sorted(expected_operations)}",
                    )

                latency_families = set(input_obj.get("required_latency_metric_families", []))
                resource_families = set(input_obj.get("required_resource_metric_families", []))
                ensure(
                    latency_families == REQUIRED_V2_PERF_GATE_LATENCY_METRIC_FAMILIES,
                    f"{contract_path}: input.required_latency_metric_families must equal {sorted(REQUIRED_V2_PERF_GATE_LATENCY_METRIC_FAMILIES)}",
                )
                ensure(
                    resource_families == REQUIRED_V2_PERF_GATE_RESOURCE_METRIC_FAMILIES,
                    f"{contract_path}: input.required_resource_metric_families must equal {sorted(REQUIRED_V2_PERF_GATE_RESOURCE_METRIC_FAMILIES)}",
                )

                coverage = contract.get("coverage")
                ensure(isinstance(coverage, dict), f"{contract_path}: coverage must be object")
                reported_operations = set(coverage.get("reported_operations", []))
                reported_fixture_families = set(
                    coverage.get("reported_fixture_families", [])
                )
                ensure(
                    coverage.get("operation_coverage_mode") == "representative_matrix",
                    f"{contract_path}: coverage.operation_coverage_mode must be 'representative_matrix'",
                )
                ensure(
                    reported_operations == REQUIRED_V2_PERF_GATE_OPERATIONS,
                    f"{contract_path}: coverage.reported_operations must equal {sorted(REQUIRED_V2_PERF_GATE_OPERATIONS)}",
                )
                ensure(
                    reported_fixture_families == REQUIRED_V2_PERF_GATE_FIXTURE_FAMILIES,
                    f"{contract_path}: coverage.reported_fixture_families must equal {sorted(REQUIRED_V2_PERF_GATE_FIXTURE_FAMILIES)}",
                )
                ensure(
                    coverage.get("authoritative_for_cutover_acceptance") is True,
                    f"{contract_path}: coverage.authoritative_for_cutover_acceptance must be true",
                )

                ceilings = baseline.get("absolute_latency_ceilings_ms")
                ensure(
                    isinstance(ceilings, dict),
                    f"{contract_path}: baseline.absolute_latency_ceilings_ms must be object",
                )
                resource_ceilings = baseline.get("resource_budget_ceilings")
                ensure(
                    isinstance(resource_ceilings, dict),
                    f"{contract_path}: baseline.resource_budget_ceilings must be object",
                )
                fail_closed_ceilings = baseline.get("fail_closed_budget_ceilings")
                ensure(
                    isinstance(fail_closed_ceilings, dict),
                    f"{contract_path}: baseline.fail_closed_budget_ceilings must be object",
                )
                for profile in sorted(REQUIRED_V1_PERF_GATE_PROFILES):
                    profile_latency = ceilings.get(profile)
                    profile_resource = resource_ceilings.get(profile)
                    profile_fail_closed = fail_closed_ceilings.get(profile)
                    ensure(
                        isinstance(profile_latency, dict),
                        f"{contract_path}: missing latency budget for profile {profile!r}",
                    )
                    ensure(
                        isinstance(profile_resource, dict),
                        f"{contract_path}: missing resource budget for profile {profile!r}",
                    )
                    ensure(
                        isinstance(profile_fail_closed, dict),
                        f"{contract_path}: missing fail-closed budget for profile {profile!r}",
                    )
                    default_latency = profile_latency.get("default")
                    default_resource = profile_resource.get("default")
                    default_fail_closed = profile_fail_closed.get("default")
                    ensure(
                        isinstance(default_latency, dict),
                        f"{contract_path}: {profile}.default latency budget must be object",
                    )
                    ensure(
                        isinstance(default_resource, dict),
                        f"{contract_path}: {profile}.default resource budget must be object",
                    )
                    ensure(
                        isinstance(default_fail_closed, dict),
                        f"{contract_path}: {profile}.default fail-closed budget must be object",
                    )
                    incomplete_latency = profile_latency.get("incomplete_syntax_member_access")
                    incomplete_resource = profile_resource.get("incomplete_syntax_member_access")
                    incomplete_fail_closed = profile_fail_closed.get("incomplete_syntax_member_access")
                    ensure(
                        isinstance(incomplete_latency, dict),
                        f"{contract_path}: {profile}.incomplete_syntax_member_access latency budget must be object",
                    )
                    ensure(
                        isinstance(incomplete_resource, dict),
                        f"{contract_path}: {profile}.incomplete_syntax_member_access resource budget must be object",
                    )
                    ensure(
                        isinstance(incomplete_fail_closed, dict),
                        f"{contract_path}: {profile}.incomplete_syntax_member_access fail-closed budget must be object",
                    )
                    for operation in sorted(REQUIRED_V2_PERF_GATE_OPERATIONS):
                        latency_budget = default_latency.get(operation)
                        resource_budget = default_resource.get(operation)
                        fail_closed_budget = default_fail_closed.get(operation)
                        ensure(
                            isinstance(latency_budget, dict),
                            f"{contract_path}: {profile}.default.{operation} latency budget must be object",
                        )
                        ensure(
                            isinstance(resource_budget, dict),
                            f"{contract_path}: {profile}.default.{operation} resource budget must be object",
                        )
                        ensure(
                            isinstance(fail_closed_budget, dict),
                            f"{contract_path}: {profile}.default.{operation} fail-closed budget must be object",
                        )
                        for metric_family in sorted(REQUIRED_V2_PERF_GATE_LATENCY_METRIC_FAMILIES):
                            metric_budget = latency_budget.get(metric_family)
                            ensure(
                                isinstance(metric_budget, dict),
                                f"{contract_path}: {profile}.default.{operation}.{metric_family} latency budget must be object",
                            )
                            ensure(
                                isinstance(metric_budget.get("p95"), int) and metric_budget["p95"] > 0,
                                f"{contract_path}: {profile}.default.{operation}.{metric_family}.p95 must be positive integer",
                            )
                            ensure(
                                isinstance(metric_budget.get("p99"), int) and metric_budget["p99"] > 0,
                                f"{contract_path}: {profile}.default.{operation}.{metric_family}.p99 must be positive integer",
                            )
                        for metric_name in sorted(REQUIRED_V2_PERF_GATE_RESOURCE_METRIC_FAMILIES):
                            value = resource_budget.get(metric_name)
                            ensure(
                                isinstance(value, int) and value > 0,
                                f"{contract_path}: {profile}.default.{operation}.{metric_name} must be positive integer",
                            )
                        ensure(
                            fail_closed_budget == REQUIRED_V2_PERF_GATE_FAIL_CLOSED_BUDGET,
                            (
                                f"{contract_path}: {profile}.default.{operation} fail-closed budget "
                                f"must equal {REQUIRED_V2_PERF_GATE_FAIL_CLOSED_BUDGET}"
                            ),
                        )
                    incomplete_completion_latency = incomplete_latency.get("completion")
                    incomplete_completion_resource = incomplete_resource.get("completion")
                    incomplete_completion_fail_closed = incomplete_fail_closed.get("completion")
                    ensure(
                        isinstance(incomplete_completion_latency, dict),
                        f"{contract_path}: {profile}.incomplete_syntax_member_access.completion latency budget must be object",
                    )
                    ensure(
                        isinstance(incomplete_completion_resource, dict),
                        f"{contract_path}: {profile}.incomplete_syntax_member_access.completion resource budget must be object",
                    )
                    ensure(
                        isinstance(incomplete_completion_fail_closed, dict),
                        f"{contract_path}: {profile}.incomplete_syntax_member_access.completion fail-closed budget must be object",
                    )
                    for metric_family in sorted(REQUIRED_V2_PERF_GATE_LATENCY_METRIC_FAMILIES):
                        metric_budget = incomplete_completion_latency.get(metric_family)
                        ensure(
                            isinstance(metric_budget, dict),
                            f"{contract_path}: {profile}.incomplete_syntax_member_access.completion.{metric_family} latency budget must be object",
                        )
                        ensure(
                            isinstance(metric_budget.get("p95"), int) and metric_budget["p95"] > 0,
                            f"{contract_path}: {profile}.incomplete_syntax_member_access.completion.{metric_family}.p95 must be positive integer",
                        )
                        ensure(
                            isinstance(metric_budget.get("p99"), int) and metric_budget["p99"] > 0,
                            f"{contract_path}: {profile}.incomplete_syntax_member_access.completion.{metric_family}.p99 must be positive integer",
                        )
                    for metric_name in sorted(REQUIRED_V2_PERF_GATE_RESOURCE_METRIC_FAMILIES):
                        value = incomplete_completion_resource.get(metric_name)
                        ensure(
                            isinstance(value, int) and value > 0,
                            f"{contract_path}: {profile}.incomplete_syntax_member_access.completion.{metric_name} must be positive integer",
                        )
                    ensure(
                        incomplete_completion_fail_closed
                        == REQUIRED_V2_PERF_GATE_FAIL_CLOSED_BUDGET,
                        (
                            f"{contract_path}: {profile}.incomplete_syntax_member_access.completion "
                            f"fail-closed budget must equal {REQUIRED_V2_PERF_GATE_FAIL_CLOSED_BUDGET}"
                        ),
                    )

                ratio_baseline_floors = baseline.get("relative_ratio_baseline_floors")
                ensure(
                    isinstance(ratio_baseline_floors, dict),
                    f"{contract_path}: baseline.relative_ratio_baseline_floors must be object",
                )
                ensure(
                    set(ratio_baseline_floors.keys())
                    == set(REQUIRED_V2_PERF_GATE_RATIO_BASELINE_FLOORS.keys()),
                    f"{contract_path}: baseline.relative_ratio_baseline_floors keys must equal {sorted(REQUIRED_V2_PERF_GATE_RATIO_BASELINE_FLOORS.keys())}",
                )
                for metric_name, expected_floor in (
                    REQUIRED_V2_PERF_GATE_RATIO_BASELINE_FLOORS.items()
                ):
                    actual_floor = ratio_baseline_floors.get(metric_name)
                    ensure(
                        isinstance(actual_floor, (int, float)) and actual_floor > 0,
                        f"{contract_path}: baseline.relative_ratio_baseline_floors.{metric_name} must be positive number",
                    )
                    ensure(
                        float(actual_floor) == float(expected_floor),
                        f"{contract_path}: baseline.relative_ratio_baseline_floors.{metric_name} must equal {expected_floor}",
                    )

                anti_rescue_budget = baseline.get("anti_rescue_budget_ceilings")
                ensure(
                    isinstance(anti_rescue_budget, dict),
                    f"{contract_path}: baseline.anti_rescue_budget_ceilings must be object",
                )
                for key in [
                    "stale_fallback_total",
                    "stale_served_total",
                    "degraded_substitute_total",
                    "search_backed_substitute_total",
                ]:
                    ensure(
                        anti_rescue_budget.get(key) == 0,
                        f"{contract_path}: baseline.anti_rescue_budget_ceilings.{key} must equal 0",
                    )

                ensure(
                    REQUIRED_V2_PERF_GATE_REPORT_FIELDS.issubset(required_report_fields),
                    f"{contract_path}: report.required_fields must include {sorted(REQUIRED_V2_PERF_GATE_REPORT_FIELDS)}",
                )
                ensure(
                    REQUIRED_V2_PERF_GATE_REASON_CODES.issubset(reason_codes),
                    f"{contract_path}: evaluator.reason_codes must include {sorted(REQUIRED_V2_PERF_GATE_REASON_CODES)}",
                )

        if surface_dir.name == "lsp-completion-timeline" and major == 3:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            outcomes = set(response.get("outcomes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 4:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            ensure(
                response.get("version") == 2,
                f"{contract_path}: response.version must equal 2",
            )
            outcomes = set(response.get("outcomes", []))
            trace_fields = set(response.get("trace_fields", []))
            prepare_details_fields = set(response.get("prepare_details_fields", []))
            turn_attribution_fields = set(response.get("turn_attribution_fields", []))
            turn_holder_fields = set(response.get("turn_holder_fields", []))
            prepare_routes = set(response.get("prepare_routes", []))
            prepare_fail_closed_causes = set(response.get("prepare_fail_closed_causes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
            )
            ensure(
                trace_fields == REQUIRED_V4_TIMELINE_TRACE_FIELDS,
                f"{contract_path}: response.trace_fields must equal {sorted(REQUIRED_V4_TIMELINE_TRACE_FIELDS)}",
            )
            ensure(
                prepare_details_fields == REQUIRED_V4_TIMELINE_PREPARE_DETAILS_FIELDS,
                f"{contract_path}: response.prepare_details_fields must equal {sorted(REQUIRED_V4_TIMELINE_PREPARE_DETAILS_FIELDS)}",
            )
            ensure(
                turn_attribution_fields == REQUIRED_V4_TIMELINE_TURN_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.turn_attribution_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                turn_holder_fields == REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS,
                f"{contract_path}: response.turn_holder_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS)}",
            )
            ensure(
                prepare_routes == REQUIRED_V4_COMPLETION_ROUTES,
                f"{contract_path}: response.prepare_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
            )
            ensure(
                prepare_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
                f"{contract_path}: response.prepare_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 5:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            ensure(
                response.get("version") == 3,
                f"{contract_path}: response.version must equal 3",
            )
            outcomes = set(response.get("outcomes", []))
            trace_fields = set(response.get("trace_fields", []))
            server_edge_details_fields = set(response.get("server_edge_details_fields", []))
            prepare_details_fields = set(response.get("prepare_details_fields", []))
            turn_attribution_fields = set(response.get("turn_attribution_fields", []))
            turn_holder_fields = set(response.get("turn_holder_fields", []))
            prepare_routes = set(response.get("prepare_routes", []))
            prepare_fail_closed_causes = set(response.get("prepare_fail_closed_causes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
            )
            ensure(
                trace_fields == REQUIRED_V5_TIMELINE_TRACE_FIELDS,
                f"{contract_path}: response.trace_fields must equal {sorted(REQUIRED_V5_TIMELINE_TRACE_FIELDS)}",
            )
            ensure(
                server_edge_details_fields == REQUIRED_V5_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                f"{contract_path}: response.server_edge_details_fields must equal {sorted(REQUIRED_V5_TIMELINE_SERVER_EDGE_DETAILS_FIELDS)}",
            )
            ensure(
                prepare_details_fields == REQUIRED_V4_TIMELINE_PREPARE_DETAILS_FIELDS,
                f"{contract_path}: response.prepare_details_fields must equal {sorted(REQUIRED_V4_TIMELINE_PREPARE_DETAILS_FIELDS)}",
            )
            ensure(
                turn_attribution_fields == REQUIRED_V4_TIMELINE_TURN_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.turn_attribution_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                turn_holder_fields == REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS,
                f"{contract_path}: response.turn_holder_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS)}",
            )
            ensure(
                prepare_routes == REQUIRED_V4_COMPLETION_ROUTES,
                f"{contract_path}: response.prepare_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
            )
            ensure(
                prepare_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
                f"{contract_path}: response.prepare_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 6:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            ensure(
                response.get("version") == 9,
                f"{contract_path}: response.version must equal 9",
            )
            outcomes = set(response.get("outcomes", []))
            trace_fields = set(response.get("trace_fields", []))
            prepare_details_fields = set(response.get("prepare_details_fields", []))
            prepare_progress_fields = set(response.get("prepare_progress_fields", []))
            prepare_runtime_fields = set(response.get("prepare_runtime_fields", []))
            prepare_timeout_attribution_fields = set(
                response.get("prepare_timeout_attribution_fields", [])
            )
            exact_wait_fields = set(response.get("exact_wait_fields", []))
            exact_artifact_poll_fields = set(response.get("exact_artifact_poll_fields", []))
            server_edge_details_fields = set(response.get("server_edge_details_fields", []))
            turn_attribution_fields = set(response.get("turn_attribution_fields", []))
            turn_holder_fields = set(response.get("turn_holder_fields", []))
            prepare_routes = set(response.get("prepare_routes", []))
            prepare_fail_closed_causes = set(response.get("prepare_fail_closed_causes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
            )
            ensure(
                trace_fields == REQUIRED_V6_TIMELINE_TRACE_FIELDS,
                f"{contract_path}: response.trace_fields must equal {sorted(REQUIRED_V6_TIMELINE_TRACE_FIELDS)}",
            )
            ensure(
                prepare_details_fields == REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS,
                f"{contract_path}: response.prepare_details_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS)}",
            )
            ensure(
                prepare_progress_fields == REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS,
                f"{contract_path}: response.prepare_progress_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS)}",
            )
            ensure(
                prepare_runtime_fields == REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS,
                f"{contract_path}: response.prepare_runtime_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS)}",
            )
            ensure(
                prepare_timeout_attribution_fields
                == REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.prepare_timeout_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                exact_wait_fields == REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS,
                f"{contract_path}: response.exact_wait_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS)}",
            )
            ensure(
                exact_artifact_poll_fields == REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS,
                f"{contract_path}: response.exact_artifact_poll_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS)}",
            )
            ensure(
                server_edge_details_fields == REQUIRED_V6_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                f"{contract_path}: response.server_edge_details_fields must equal {sorted(REQUIRED_V6_TIMELINE_SERVER_EDGE_DETAILS_FIELDS)}",
            )
            ensure(
                turn_attribution_fields == REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.turn_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                turn_holder_fields == REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS,
                f"{contract_path}: response.turn_holder_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS)}",
            )
            ensure(
                prepare_routes == REQUIRED_V4_COMPLETION_ROUTES,
                f"{contract_path}: response.prepare_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
            )
            ensure(
                prepare_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
                f"{contract_path}: response.prepare_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 7:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            ensure(
                response.get("version") == 10,
                f"{contract_path}: response.version must equal 10",
            )
            outcomes = set(response.get("outcomes", []))
            trace_fields = set(response.get("trace_fields", []))
            prepare_details_fields = set(response.get("prepare_details_fields", []))
            prepare_progress_fields = set(response.get("prepare_progress_fields", []))
            prepare_runtime_fields = set(response.get("prepare_runtime_fields", []))
            prepare_timeout_attribution_fields = set(
                response.get("prepare_timeout_attribution_fields", [])
            )
            exact_wait_fields = set(response.get("exact_wait_fields", []))
            exact_artifact_poll_fields = set(response.get("exact_artifact_poll_fields", []))
            server_edge_details_fields = set(response.get("server_edge_details_fields", []))
            turn_attribution_fields = set(response.get("turn_attribution_fields", []))
            turn_holder_fields = set(response.get("turn_holder_fields", []))
            prepare_routes = set(response.get("prepare_routes", []))
            prepare_fail_closed_causes = set(response.get("prepare_fail_closed_causes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
            )
            ensure(
                trace_fields == REQUIRED_V6_TIMELINE_TRACE_FIELDS,
                f"{contract_path}: response.trace_fields must equal {sorted(REQUIRED_V6_TIMELINE_TRACE_FIELDS)}",
            )
            ensure(
                prepare_details_fields == REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS,
                f"{contract_path}: response.prepare_details_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS)}",
            )
            ensure(
                prepare_progress_fields == REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS,
                f"{contract_path}: response.prepare_progress_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS)}",
            )
            ensure(
                prepare_runtime_fields == REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS,
                f"{contract_path}: response.prepare_runtime_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS)}",
            )
            ensure(
                prepare_timeout_attribution_fields
                == REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.prepare_timeout_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                exact_wait_fields == REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS,
                f"{contract_path}: response.exact_wait_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS)}",
            )
            ensure(
                exact_artifact_poll_fields == REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS,
                f"{contract_path}: response.exact_artifact_poll_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS)}",
            )
            ensure(
                server_edge_details_fields == REQUIRED_V7_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                f"{contract_path}: response.server_edge_details_fields must equal {sorted(REQUIRED_V7_TIMELINE_SERVER_EDGE_DETAILS_FIELDS)}",
            )
            ensure(
                turn_attribution_fields == REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.turn_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                turn_holder_fields == REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS,
                f"{contract_path}: response.turn_holder_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS)}",
            )
            ensure(
                prepare_routes == REQUIRED_V4_COMPLETION_ROUTES,
                f"{contract_path}: response.prepare_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
            )
            ensure(
                prepare_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
                f"{contract_path}: response.prepare_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 8:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            ensure(
                response.get("version") == 11,
                f"{contract_path}: response.version must equal 11",
            )
            outcomes = set(response.get("outcomes", []))
            trace_fields = set(response.get("trace_fields", []))
            prepare_details_fields = set(response.get("prepare_details_fields", []))
            prepare_progress_fields = set(response.get("prepare_progress_fields", []))
            prepare_runtime_fields = set(response.get("prepare_runtime_fields", []))
            prepare_timeout_attribution_fields = set(
                response.get("prepare_timeout_attribution_fields", [])
            )
            exact_wait_fields = set(response.get("exact_wait_fields", []))
            exact_artifact_poll_fields = set(response.get("exact_artifact_poll_fields", []))
            server_edge_details_fields = set(response.get("server_edge_details_fields", []))
            turn_attribution_fields = set(response.get("turn_attribution_fields", []))
            turn_holder_fields = set(response.get("turn_holder_fields", []))
            prepare_routes = set(response.get("prepare_routes", []))
            prepare_fail_closed_causes = set(response.get("prepare_fail_closed_causes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
            )
            ensure(
                trace_fields == REQUIRED_V6_TIMELINE_TRACE_FIELDS,
                f"{contract_path}: response.trace_fields must equal {sorted(REQUIRED_V6_TIMELINE_TRACE_FIELDS)}",
            )
            ensure(
                prepare_details_fields == REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS,
                f"{contract_path}: response.prepare_details_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS)}",
            )
            ensure(
                prepare_progress_fields == REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS,
                f"{contract_path}: response.prepare_progress_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS)}",
            )
            ensure(
                prepare_runtime_fields == REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS,
                f"{contract_path}: response.prepare_runtime_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS)}",
            )
            ensure(
                prepare_timeout_attribution_fields
                == REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.prepare_timeout_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                exact_wait_fields == REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS,
                f"{contract_path}: response.exact_wait_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS)}",
            )
            ensure(
                exact_artifact_poll_fields == REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS,
                f"{contract_path}: response.exact_artifact_poll_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS)}",
            )
            ensure(
                server_edge_details_fields == REQUIRED_V8_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                f"{contract_path}: response.server_edge_details_fields must equal {sorted(REQUIRED_V8_TIMELINE_SERVER_EDGE_DETAILS_FIELDS)}",
            )
            ensure(
                turn_attribution_fields == REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.turn_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                turn_holder_fields == REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS,
                f"{contract_path}: response.turn_holder_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS)}",
            )
            ensure(
                prepare_routes == REQUIRED_V4_COMPLETION_ROUTES,
                f"{contract_path}: response.prepare_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
            )
            ensure(
                prepare_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
                f"{contract_path}: response.prepare_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 9:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            ensure(
                response.get("version") == 12,
                f"{contract_path}: response.version must equal 12",
            )
            outcomes = set(response.get("outcomes", []))
            trace_fields = set(response.get("trace_fields", []))
            prepare_details_fields = set(response.get("prepare_details_fields", []))
            prepare_progress_fields = set(response.get("prepare_progress_fields", []))
            prepare_runtime_fields = set(response.get("prepare_runtime_fields", []))
            prepare_timeout_attribution_fields = set(
                response.get("prepare_timeout_attribution_fields", [])
            )
            exact_wait_fields = set(response.get("exact_wait_fields", []))
            exact_artifact_poll_fields = set(response.get("exact_artifact_poll_fields", []))
            server_edge_details_fields = set(response.get("server_edge_details_fields", []))
            first_poll_contention_fields = set(
                response.get("first_poll_contention_attribution_fields", [])
            )
            first_poll_contention_classes = set(
                response.get("first_poll_contention_contender_classes", [])
            )
            first_poll_contention_uri_scopes = set(
                response.get("first_poll_contention_uri_scopes", [])
            )
            turn_attribution_fields = set(response.get("turn_attribution_fields", []))
            turn_holder_fields = set(response.get("turn_holder_fields", []))
            prepare_routes = set(response.get("prepare_routes", []))
            prepare_fail_closed_causes = set(response.get("prepare_fail_closed_causes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
            )
            ensure(
                trace_fields == REQUIRED_V6_TIMELINE_TRACE_FIELDS,
                f"{contract_path}: response.trace_fields must equal {sorted(REQUIRED_V6_TIMELINE_TRACE_FIELDS)}",
            )
            ensure(
                prepare_details_fields == REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS,
                f"{contract_path}: response.prepare_details_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS)}",
            )
            ensure(
                prepare_progress_fields == REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS,
                f"{contract_path}: response.prepare_progress_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS)}",
            )
            ensure(
                prepare_runtime_fields == REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS,
                f"{contract_path}: response.prepare_runtime_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS)}",
            )
            ensure(
                prepare_timeout_attribution_fields
                == REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.prepare_timeout_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                exact_wait_fields == REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS,
                f"{contract_path}: response.exact_wait_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS)}",
            )
            ensure(
                exact_artifact_poll_fields == REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS,
                f"{contract_path}: response.exact_artifact_poll_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS)}",
            )
            ensure(
                server_edge_details_fields == REQUIRED_V9_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                f"{contract_path}: response.server_edge_details_fields must equal {sorted(REQUIRED_V9_TIMELINE_SERVER_EDGE_DETAILS_FIELDS)}",
            )
            ensure(
                first_poll_contention_fields == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS,
                f"{contract_path}: response.first_poll_contention_attribution_fields must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS)}",
            )
            ensure(
                first_poll_contention_classes
                == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES,
                f"{contract_path}: response.first_poll_contention_contender_classes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES)}",
            )
            ensure(
                first_poll_contention_uri_scopes
                == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES,
                f"{contract_path}: response.first_poll_contention_uri_scopes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES)}",
            )
            ensure(
                turn_attribution_fields == REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.turn_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                turn_holder_fields == REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS,
                f"{contract_path}: response.turn_holder_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS)}",
            )
            ensure(
                prepare_routes == REQUIRED_V4_COMPLETION_ROUTES,
                f"{contract_path}: response.prepare_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
            )
            ensure(
                prepare_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
                f"{contract_path}: response.prepare_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 10:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            ensure(
                response.get("version") == 13,
                f"{contract_path}: response.version must equal 13",
            )
            outcomes = set(response.get("outcomes", []))
            trace_fields = set(response.get("trace_fields", []))
            prepare_details_fields = set(response.get("prepare_details_fields", []))
            prepare_progress_fields = set(response.get("prepare_progress_fields", []))
            prepare_runtime_fields = set(response.get("prepare_runtime_fields", []))
            prepare_timeout_attribution_fields = set(
                response.get("prepare_timeout_attribution_fields", [])
            )
            exact_wait_fields = set(response.get("exact_wait_fields", []))
            exact_artifact_poll_fields = set(response.get("exact_artifact_poll_fields", []))
            server_edge_details_fields = set(response.get("server_edge_details_fields", []))
            first_poll_contention_fields = set(
                response.get("first_poll_contention_attribution_fields", [])
            )
            first_poll_contention_contender_fields = set(
                response.get("first_poll_contention_contender_fields", [])
            )
            first_poll_contention_classes = set(
                response.get("first_poll_contention_contender_classes", [])
            )
            first_poll_contention_request_classes = set(
                response.get("first_poll_contention_request_classes", [])
            )
            first_poll_contention_uri_scopes = set(
                response.get("first_poll_contention_uri_scopes", [])
            )
            turn_attribution_fields = set(response.get("turn_attribution_fields", []))
            turn_holder_fields = set(response.get("turn_holder_fields", []))
            prepare_routes = set(response.get("prepare_routes", []))
            prepare_fail_closed_causes = set(response.get("prepare_fail_closed_causes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
            )
            ensure(
                trace_fields == REQUIRED_V6_TIMELINE_TRACE_FIELDS,
                f"{contract_path}: response.trace_fields must equal {sorted(REQUIRED_V6_TIMELINE_TRACE_FIELDS)}",
            )
            ensure(
                prepare_details_fields == REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS,
                f"{contract_path}: response.prepare_details_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS)}",
            )
            ensure(
                prepare_progress_fields == REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS,
                f"{contract_path}: response.prepare_progress_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS)}",
            )
            ensure(
                prepare_runtime_fields == REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS,
                f"{contract_path}: response.prepare_runtime_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS)}",
            )
            ensure(
                prepare_timeout_attribution_fields
                == REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.prepare_timeout_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                exact_wait_fields == REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS,
                f"{contract_path}: response.exact_wait_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS)}",
            )
            ensure(
                exact_artifact_poll_fields == REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS,
                f"{contract_path}: response.exact_artifact_poll_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS)}",
            )
            ensure(
                server_edge_details_fields == REQUIRED_V10_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                f"{contract_path}: response.server_edge_details_fields must equal {sorted(REQUIRED_V10_TIMELINE_SERVER_EDGE_DETAILS_FIELDS)}",
            )
            ensure(
                first_poll_contention_fields == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS,
                f"{contract_path}: response.first_poll_contention_attribution_fields must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS)}",
            )
            ensure(
                first_poll_contention_contender_fields
                == REQUIRED_V10_TIMELINE_FIRST_POLL_CONTENDER_FIELDS,
                f"{contract_path}: response.first_poll_contention_contender_fields must equal {sorted(REQUIRED_V10_TIMELINE_FIRST_POLL_CONTENDER_FIELDS)}",
            )
            ensure(
                first_poll_contention_classes
                == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES,
                f"{contract_path}: response.first_poll_contention_contender_classes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES)}",
            )
            ensure(
                first_poll_contention_request_classes
                == REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES,
                f"{contract_path}: response.first_poll_contention_request_classes must equal {sorted(REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES)}",
            )
            ensure(
                first_poll_contention_uri_scopes
                == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES,
                f"{contract_path}: response.first_poll_contention_uri_scopes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES)}",
            )
            ensure(
                turn_attribution_fields == REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.turn_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                turn_holder_fields == REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS,
                f"{contract_path}: response.turn_holder_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS)}",
            )
            ensure(
                prepare_routes == REQUIRED_V4_COMPLETION_ROUTES,
                f"{contract_path}: response.prepare_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
            )
            ensure(
                prepare_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
                f"{contract_path}: response.prepare_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 11:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            ensure(
                response.get("version") == 14,
                f"{contract_path}: response.version must equal 14",
            )
            outcomes = set(response.get("outcomes", []))
            trace_fields = set(response.get("trace_fields", []))
            prepare_details_fields = set(response.get("prepare_details_fields", []))
            prepare_progress_fields = set(response.get("prepare_progress_fields", []))
            prepare_runtime_fields = set(response.get("prepare_runtime_fields", []))
            prepare_timeout_attribution_fields = set(
                response.get("prepare_timeout_attribution_fields", [])
            )
            exact_wait_fields = set(response.get("exact_wait_fields", []))
            exact_artifact_poll_fields = set(response.get("exact_artifact_poll_fields", []))
            server_edge_details_fields = set(response.get("server_edge_details_fields", []))
            first_poll_contention_fields = set(
                response.get("first_poll_contention_attribution_fields", [])
            )
            first_poll_contention_contender_fields = set(
                response.get("first_poll_contention_contender_fields", [])
            )
            first_poll_contention_classes = set(
                response.get("first_poll_contention_contender_classes", [])
            )
            first_poll_contention_request_classes = set(
                response.get("first_poll_contention_request_classes", [])
            )
            first_poll_contention_uri_scopes = set(
                response.get("first_poll_contention_uri_scopes", [])
            )
            turn_attribution_fields = set(response.get("turn_attribution_fields", []))
            turn_holder_fields = set(response.get("turn_holder_fields", []))
            prepare_routes = set(response.get("prepare_routes", []))
            prepare_fail_closed_causes = set(response.get("prepare_fail_closed_causes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
            )
            ensure(
                trace_fields == REQUIRED_V6_TIMELINE_TRACE_FIELDS,
                f"{contract_path}: response.trace_fields must equal {sorted(REQUIRED_V6_TIMELINE_TRACE_FIELDS)}",
            )
            ensure(
                prepare_details_fields == REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS,
                f"{contract_path}: response.prepare_details_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS)}",
            )
            ensure(
                prepare_progress_fields == REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS,
                f"{contract_path}: response.prepare_progress_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS)}",
            )
            ensure(
                prepare_runtime_fields == REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS,
                f"{contract_path}: response.prepare_runtime_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS)}",
            )
            ensure(
                prepare_timeout_attribution_fields
                == REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.prepare_timeout_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                exact_wait_fields == REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS,
                f"{contract_path}: response.exact_wait_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS)}",
            )
            ensure(
                exact_artifact_poll_fields == REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS,
                f"{contract_path}: response.exact_artifact_poll_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS)}",
            )
            ensure(
                server_edge_details_fields == REQUIRED_V10_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                f"{contract_path}: response.server_edge_details_fields must equal {sorted(REQUIRED_V10_TIMELINE_SERVER_EDGE_DETAILS_FIELDS)}",
            )
            ensure(
                first_poll_contention_fields == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS,
                f"{contract_path}: response.first_poll_contention_attribution_fields must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS)}",
            )
            ensure(
                first_poll_contention_contender_fields
                == REQUIRED_V11_TIMELINE_FIRST_POLL_CONTENDER_FIELDS,
                f"{contract_path}: response.first_poll_contention_contender_fields must equal {sorted(REQUIRED_V11_TIMELINE_FIRST_POLL_CONTENDER_FIELDS)}",
            )
            ensure(
                first_poll_contention_classes
                == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES,
                f"{contract_path}: response.first_poll_contention_contender_classes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES)}",
            )
            ensure(
                first_poll_contention_request_classes
                == REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES,
                f"{contract_path}: response.first_poll_contention_request_classes must equal {sorted(REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES)}",
            )
            ensure(
                first_poll_contention_uri_scopes
                == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES,
                f"{contract_path}: response.first_poll_contention_uri_scopes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES)}",
            )
            ensure(
                turn_attribution_fields == REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.turn_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                turn_holder_fields == REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS,
                f"{contract_path}: response.turn_holder_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS)}",
            )
            ensure(
                prepare_routes == REQUIRED_V4_COMPLETION_ROUTES,
                f"{contract_path}: response.prepare_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
            )
            ensure(
                prepare_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
                f"{contract_path}: response.prepare_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 12:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            ensure(
                response.get("version") == 15,
                f"{contract_path}: response.version must equal 15",
            )
            outcomes = set(response.get("outcomes", []))
            trace_fields = set(response.get("trace_fields", []))
            prepare_details_fields = set(response.get("prepare_details_fields", []))
            prepare_progress_fields = set(response.get("prepare_progress_fields", []))
            prepare_runtime_fields = set(response.get("prepare_runtime_fields", []))
            prepare_timeout_attribution_fields = set(
                response.get("prepare_timeout_attribution_fields", [])
            )
            exact_wait_fields = set(response.get("exact_wait_fields", []))
            exact_artifact_poll_fields = set(response.get("exact_artifact_poll_fields", []))
            server_edge_details_fields = set(response.get("server_edge_details_fields", []))
            first_poll_contention_fields = set(
                response.get("first_poll_contention_attribution_fields", [])
            )
            first_poll_contention_contender_fields = set(
                response.get("first_poll_contention_contender_fields", [])
            )
            first_poll_contention_classes = set(
                response.get("first_poll_contention_contender_classes", [])
            )
            first_poll_contention_request_classes = set(
                response.get("first_poll_contention_request_classes", [])
            )
            first_poll_contention_uri_scopes = set(
                response.get("first_poll_contention_uri_scopes", [])
            )
            turn_attribution_fields = set(response.get("turn_attribution_fields", []))
            turn_holder_fields = set(response.get("turn_holder_fields", []))
            prepare_routes = set(response.get("prepare_routes", []))
            prepare_fail_closed_causes = set(response.get("prepare_fail_closed_causes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
            )
            ensure(
                trace_fields == REQUIRED_V6_TIMELINE_TRACE_FIELDS,
                f"{contract_path}: response.trace_fields must equal {sorted(REQUIRED_V6_TIMELINE_TRACE_FIELDS)}",
            )
            ensure(
                prepare_details_fields == REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS,
                f"{contract_path}: response.prepare_details_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS)}",
            )
            ensure(
                prepare_progress_fields == REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS,
                f"{contract_path}: response.prepare_progress_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS)}",
            )
            ensure(
                prepare_runtime_fields == REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS,
                f"{contract_path}: response.prepare_runtime_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS)}",
            )
            ensure(
                prepare_timeout_attribution_fields
                == REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.prepare_timeout_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                exact_wait_fields == REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS,
                f"{contract_path}: response.exact_wait_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS)}",
            )
            ensure(
                exact_artifact_poll_fields == REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS,
                f"{contract_path}: response.exact_artifact_poll_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS)}",
            )
            ensure(
                server_edge_details_fields == REQUIRED_V10_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                f"{contract_path}: response.server_edge_details_fields must equal {sorted(REQUIRED_V10_TIMELINE_SERVER_EDGE_DETAILS_FIELDS)}",
            )
            ensure(
                first_poll_contention_fields == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS,
                f"{contract_path}: response.first_poll_contention_attribution_fields must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS)}",
            )
            ensure(
                first_poll_contention_contender_fields
                == REQUIRED_V12_TIMELINE_FIRST_POLL_CONTENDER_FIELDS,
                f"{contract_path}: response.first_poll_contention_contender_fields must equal {sorted(REQUIRED_V12_TIMELINE_FIRST_POLL_CONTENDER_FIELDS)}",
            )
            ensure(
                first_poll_contention_classes
                == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES,
                f"{contract_path}: response.first_poll_contention_contender_classes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES)}",
            )
            ensure(
                first_poll_contention_request_classes
                == REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES,
                f"{contract_path}: response.first_poll_contention_request_classes must equal {sorted(REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES)}",
            )
            ensure(
                first_poll_contention_uri_scopes
                == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES,
                f"{contract_path}: response.first_poll_contention_uri_scopes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES)}",
            )
            ensure(
                turn_attribution_fields == REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.turn_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_TURN_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                turn_holder_fields == REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS,
                f"{contract_path}: response.turn_holder_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS)}",
            )
            ensure(
                prepare_routes == REQUIRED_V4_COMPLETION_ROUTES,
                f"{contract_path}: response.prepare_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
            )
            ensure(
                prepare_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
                f"{contract_path}: response.prepare_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 13:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            ensure(
                response.get("version") == 16,
                f"{contract_path}: response.version must equal 16",
            )
            outcomes = set(response.get("outcomes", []))
            trace_fields = set(response.get("trace_fields", []))
            prepare_details_fields = set(response.get("prepare_details_fields", []))
            prepare_progress_fields = set(response.get("prepare_progress_fields", []))
            prepare_runtime_fields = set(response.get("prepare_runtime_fields", []))
            prepare_timeout_attribution_fields = set(
                response.get("prepare_timeout_attribution_fields", [])
            )
            exact_wait_fields = set(response.get("exact_wait_fields", []))
            exact_artifact_poll_fields = set(response.get("exact_artifact_poll_fields", []))
            server_edge_details_fields = set(response.get("server_edge_details_fields", []))
            first_poll_contention_fields = set(
                response.get("first_poll_contention_attribution_fields", [])
            )
            first_poll_contention_contender_fields = set(
                response.get("first_poll_contention_contender_fields", [])
            )
            first_poll_contention_classes = set(
                response.get("first_poll_contention_contender_classes", [])
            )
            first_poll_contention_request_classes = set(
                response.get("first_poll_contention_request_classes", [])
            )
            first_poll_contention_uri_scopes = set(
                response.get("first_poll_contention_uri_scopes", [])
            )
            turn_attribution_fields = set(response.get("turn_attribution_fields", []))
            turn_holder_fields = set(response.get("turn_holder_fields", []))
            prepare_routes = set(response.get("prepare_routes", []))
            prepare_fail_closed_causes = set(response.get("prepare_fail_closed_causes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
            )
            ensure(
                trace_fields == REQUIRED_V6_TIMELINE_TRACE_FIELDS,
                f"{contract_path}: response.trace_fields must equal {sorted(REQUIRED_V6_TIMELINE_TRACE_FIELDS)}",
            )
            ensure(
                prepare_details_fields == REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS,
                f"{contract_path}: response.prepare_details_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS)}",
            )
            ensure(
                prepare_progress_fields == REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS,
                f"{contract_path}: response.prepare_progress_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS)}",
            )
            ensure(
                prepare_runtime_fields == REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS,
                f"{contract_path}: response.prepare_runtime_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS)}",
            )
            ensure(
                prepare_timeout_attribution_fields
                == REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.prepare_timeout_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                exact_wait_fields == REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS,
                f"{contract_path}: response.exact_wait_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS)}",
            )
            ensure(
                exact_artifact_poll_fields == REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS,
                f"{contract_path}: response.exact_artifact_poll_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS)}",
            )
            ensure(
                server_edge_details_fields == REQUIRED_V10_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                f"{contract_path}: response.server_edge_details_fields must equal {sorted(REQUIRED_V10_TIMELINE_SERVER_EDGE_DETAILS_FIELDS)}",
            )
            ensure(
                first_poll_contention_fields == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS,
                f"{contract_path}: response.first_poll_contention_attribution_fields must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS)}",
            )
            ensure(
                first_poll_contention_contender_fields
                == REQUIRED_V12_TIMELINE_FIRST_POLL_CONTENDER_FIELDS,
                f"{contract_path}: response.first_poll_contention_contender_fields must equal {sorted(REQUIRED_V12_TIMELINE_FIRST_POLL_CONTENDER_FIELDS)}",
            )
            ensure(
                first_poll_contention_classes
                == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES,
                f"{contract_path}: response.first_poll_contention_contender_classes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES)}",
            )
            ensure(
                first_poll_contention_request_classes
                == REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES,
                f"{contract_path}: response.first_poll_contention_request_classes must equal {sorted(REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES)}",
            )
            ensure(
                first_poll_contention_uri_scopes
                == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES,
                f"{contract_path}: response.first_poll_contention_uri_scopes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES)}",
            )
            ensure(
                turn_attribution_fields == REQUIRED_V13_TIMELINE_TURN_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.turn_attribution_fields must equal {sorted(REQUIRED_V13_TIMELINE_TURN_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                turn_holder_fields == REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS,
                f"{contract_path}: response.turn_holder_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS)}",
            )
            ensure(
                prepare_routes == REQUIRED_V4_COMPLETION_ROUTES,
                f"{contract_path}: response.prepare_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
            )
            ensure(
                prepare_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
                f"{contract_path}: response.prepare_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 14:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            ensure(
                response.get("version") == 17,
                f"{contract_path}: response.version must equal 17",
            )
            outcomes = set(response.get("outcomes", []))
            trace_fields = set(response.get("trace_fields", []))
            prepare_details_fields = set(response.get("prepare_details_fields", []))
            prepare_progress_fields = set(response.get("prepare_progress_fields", []))
            prepare_runtime_fields = set(response.get("prepare_runtime_fields", []))
            prepare_timeout_attribution_fields = set(
                response.get("prepare_timeout_attribution_fields", [])
            )
            exact_wait_fields = set(response.get("exact_wait_fields", []))
            exact_artifact_poll_fields = set(response.get("exact_artifact_poll_fields", []))
            server_edge_details_fields = set(response.get("server_edge_details_fields", []))
            first_poll_contention_fields = set(
                response.get("first_poll_contention_attribution_fields", [])
            )
            first_poll_contention_contender_fields = set(
                response.get("first_poll_contention_contender_fields", [])
            )
            first_poll_contention_classes = set(
                response.get("first_poll_contention_contender_classes", [])
            )
            first_poll_contention_request_classes = set(
                response.get("first_poll_contention_request_classes", [])
            )
            first_poll_contention_uri_scopes = set(
                response.get("first_poll_contention_uri_scopes", [])
            )
            turn_attribution_fields = set(response.get("turn_attribution_fields", []))
            turn_holder_fields = set(response.get("turn_holder_fields", []))
            prepare_routes = set(response.get("prepare_routes", []))
            prepare_fail_closed_causes = set(response.get("prepare_fail_closed_causes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
            )
            ensure(
                trace_fields == REQUIRED_V6_TIMELINE_TRACE_FIELDS,
                f"{contract_path}: response.trace_fields must equal {sorted(REQUIRED_V6_TIMELINE_TRACE_FIELDS)}",
            )
            ensure(
                prepare_details_fields == REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS,
                f"{contract_path}: response.prepare_details_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_DETAILS_FIELDS)}",
            )
            ensure(
                prepare_progress_fields == REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS,
                f"{contract_path}: response.prepare_progress_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_PROGRESS_FIELDS)}",
            )
            ensure(
                prepare_runtime_fields == REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS,
                f"{contract_path}: response.prepare_runtime_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_RUNTIME_FIELDS)}",
            )
            ensure(
                prepare_timeout_attribution_fields
                == REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.prepare_timeout_attribution_fields must equal {sorted(REQUIRED_V6_TIMELINE_PREPARE_TIMEOUT_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                exact_wait_fields == REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS,
                f"{contract_path}: response.exact_wait_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_WAIT_FIELDS)}",
            )
            ensure(
                exact_artifact_poll_fields == REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS,
                f"{contract_path}: response.exact_artifact_poll_fields must equal {sorted(REQUIRED_V6_TIMELINE_EXACT_ARTIFACT_POLL_FIELDS)}",
            )
            ensure(
                server_edge_details_fields
                == REQUIRED_V14_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                f"{contract_path}: response.server_edge_details_fields must equal {sorted(REQUIRED_V14_TIMELINE_SERVER_EDGE_DETAILS_FIELDS)}",
            )
            ensure(
                first_poll_contention_fields == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS,
                f"{contract_path}: response.first_poll_contention_attribution_fields must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_FIELDS)}",
            )
            ensure(
                first_poll_contention_contender_fields
                == REQUIRED_V12_TIMELINE_FIRST_POLL_CONTENDER_FIELDS,
                f"{contract_path}: response.first_poll_contention_contender_fields must equal {sorted(REQUIRED_V12_TIMELINE_FIRST_POLL_CONTENDER_FIELDS)}",
            )
            ensure(
                first_poll_contention_classes
                == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES,
                f"{contract_path}: response.first_poll_contention_contender_classes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_CLASSES)}",
            )
            ensure(
                first_poll_contention_request_classes
                == REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES,
                f"{contract_path}: response.first_poll_contention_request_classes must equal {sorted(REQUIRED_V10_TIMELINE_CONTENDER_REQUEST_CLASSES)}",
            )
            ensure(
                first_poll_contention_uri_scopes
                == REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES,
                f"{contract_path}: response.first_poll_contention_uri_scopes must equal {sorted(REQUIRED_V9_TIMELINE_FIRST_POLL_CONTENTION_URI_SCOPES)}",
            )
            ensure(
                turn_attribution_fields == REQUIRED_V13_TIMELINE_TURN_ATTRIBUTION_FIELDS,
                f"{contract_path}: response.turn_attribution_fields must equal {sorted(REQUIRED_V13_TIMELINE_TURN_ATTRIBUTION_FIELDS)}",
            )
            ensure(
                turn_holder_fields == REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS,
                f"{contract_path}: response.turn_holder_fields must equal {sorted(REQUIRED_V4_TIMELINE_TURN_HOLDER_FIELDS)}",
            )
            ensure(
                prepare_routes == REQUIRED_V4_COMPLETION_ROUTES,
                f"{contract_path}: response.prepare_routes must equal {sorted(REQUIRED_V4_COMPLETION_ROUTES)}",
            )
            ensure(
                prepare_fail_closed_causes == REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES,
                f"{contract_path}: response.prepare_fail_closed_causes must equal {sorted(REQUIRED_V4_COMPLETION_FAIL_CLOSED_CAUSES)}",
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 15:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            validate_lsp_completion_timeline_response_fields(
                contract_path,
                response,
                expected_version=18,
                expected_server_edge_details_fields=REQUIRED_V14_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 16:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            validate_lsp_completion_timeline_response_fields(
                contract_path,
                response,
                expected_version=19,
                expected_server_edge_details_fields=REQUIRED_V16_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 17:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            validate_lsp_completion_timeline_response_fields(
                contract_path,
                response,
                expected_version=20,
                expected_server_edge_details_fields=REQUIRED_V16_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                expected_query_bundle_stage_names=REQUIRED_V17_TIMELINE_QUERY_BUNDLE_STAGE_NAMES,
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 18:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            validate_lsp_completion_timeline_response_fields(
                contract_path,
                response,
                expected_version=21,
                expected_server_edge_details_fields=REQUIRED_V18_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                expected_query_bundle_stage_names=REQUIRED_V17_TIMELINE_QUERY_BUNDLE_STAGE_NAMES,
            )

        if surface_dir.name == "lsp-completion-timeline" and major == 19:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            validate_lsp_completion_timeline_response_fields(
                contract_path,
                response,
                expected_version=22,
                expected_server_edge_details_fields=REQUIRED_V19_TIMELINE_SERVER_EDGE_DETAILS_FIELDS,
                expected_query_bundle_stage_names=REQUIRED_V17_TIMELINE_QUERY_BUNDLE_STAGE_NAMES,
            )


def main() -> int:
    root = repo_root()
    contracts_dir = root / "contracts"
    if not contracts_dir.exists():
        print("ERROR: contracts directory is missing", file=sys.stderr)
        return 1

    surface_dirs = sorted(
        p for p in contracts_dir.iterdir() if p.is_dir() and p.name != "__pycache__"
    )
    found_surfaces = {p.name for p in surface_dirs}
    missing_surfaces = sorted(REQUIRED_SURFACES - found_surfaces)
    if missing_surfaces:
        print(
            f"ERROR: missing required contract surfaces: {', '.join(missing_surfaces)}",
            file=sys.stderr,
        )
        return 1

    try:
        for surface_dir in surface_dirs:
            validate_surface_contract(surface_dir)
    except ValidationError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    print("Versioned contracts policy check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
