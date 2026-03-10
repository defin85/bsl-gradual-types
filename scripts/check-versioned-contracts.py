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
    "lsp-completion-timeline": 3,
    "intellisense-perf-gate": 2,
    "observability-completion-v2": 3,
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

REQUIRED_V2_PERF_GATE_REPORTED_OPERATIONS = {
    "completion",
}

REQUIRED_V2_PERF_GATE_MISSING_OPERATIONS = {
    "hover",
    "definition",
    "type_at_position",
    "members",
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


def parse_major(version_dir_name: str, where: Path) -> int:
    match = RE_VERSION_DIR.match(version_dir_name)
    ensure(match is not None, f"{where}: invalid version directory {version_dir_name!r}")
    return int(match.group(1))


def validate_surface_contract(surface_dir: Path) -> None:
    ensure(surface_dir.is_dir(), f"{surface_dir}: surface directory is missing")
    versions: list[tuple[int, Path]] = []
    for child in sorted(surface_dir.iterdir()):
        if child.is_dir() and RE_VERSION_DIR.match(child.name):
            versions.append((parse_major(child.name, child), child))

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

        if surface_dir.name == "intellisense-perf-gate" and major in {1, 2}:
            input_obj = contract.get("input")
            ensure(isinstance(input_obj, dict), f"{contract_path}: input must be object")
            required_profiles = set(input_obj.get("required_profiles", []))
            required_latency_metrics = set(input_obj.get("required_latency_metrics", []))
            required_resource_metrics = set(input_obj.get("required_resource_metrics", []))

            ensure(
                REQUIRED_V1_PERF_GATE_PROFILES.issubset(required_profiles),
                f"{contract_path}: required_profiles must include {sorted(REQUIRED_V1_PERF_GATE_PROFILES)}",
            )
            ensure(
                REQUIRED_V1_PERF_GATE_LATENCY_METRICS.issubset(required_latency_metrics),
                f"{contract_path}: required_latency_metrics must include {sorted(REQUIRED_V1_PERF_GATE_LATENCY_METRICS)}",
            )
            ensure(
                REQUIRED_V1_PERF_GATE_RESOURCE_METRICS.issubset(required_resource_metrics),
                f"{contract_path}: required_resource_metrics must include {sorted(REQUIRED_V1_PERF_GATE_RESOURCE_METRICS)}",
            )

            baseline = contract.get("baseline")
            ensure(isinstance(baseline, dict), f"{contract_path}: baseline must be object")
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
                {"contract_version", "verdict", "reason_codes", "profiles"}.issubset(
                    required_report_fields
                ),
                f"{contract_path}: report.required_fields must include contract_version/verdict/reason_codes/profiles",
            )

            evaluator = contract.get("evaluator")
            ensure(isinstance(evaluator, dict), f"{contract_path}: evaluator must be object")
            reason_codes = set(evaluator.get("reason_codes", []))
            ensure(
                REQUIRED_V1_PERF_GATE_REASON_CODES.issubset(reason_codes),
                f"{contract_path}: evaluator.reason_codes must include {sorted(REQUIRED_V1_PERF_GATE_REASON_CODES)}",
            )

            if major == 2:
                coverage = contract.get("coverage")
                ensure(isinstance(coverage, dict), f"{contract_path}: coverage must be object")
                reported_operations = set(coverage.get("reported_operations", []))
                missing_operations = set(
                    coverage.get("missing_representative_operations", [])
                )
                ensure(
                    coverage.get("operation_coverage_mode") == "completion_only",
                    f"{contract_path}: coverage.operation_coverage_mode must be 'completion_only'",
                )
                ensure(
                    reported_operations == REQUIRED_V2_PERF_GATE_REPORTED_OPERATIONS,
                    f"{contract_path}: coverage.reported_operations must equal {sorted(REQUIRED_V2_PERF_GATE_REPORTED_OPERATIONS)}",
                )
                ensure(
                    missing_operations == REQUIRED_V2_PERF_GATE_MISSING_OPERATIONS,
                    f"{contract_path}: coverage.missing_representative_operations must equal {sorted(REQUIRED_V2_PERF_GATE_MISSING_OPERATIONS)}",
                )
                ensure(
                    coverage.get("authoritative_for_cutover_acceptance") is False,
                    f"{contract_path}: coverage.authoritative_for_cutover_acceptance must be false",
                )

        if surface_dir.name == "lsp-completion-timeline" and major == 3:
            response = contract.get("response")
            ensure(isinstance(response, dict), f"{contract_path}: response must be object")
            outcomes = set(response.get("outcomes", []))
            ensure(
                outcomes == REQUIRED_V3_TIMELINE_OUTCOMES,
                f"{contract_path}: response.outcomes must equal {sorted(REQUIRED_V3_TIMELINE_OUTCOMES)}",
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
