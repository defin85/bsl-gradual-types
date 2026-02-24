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
    "observability-completion-v2",
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

REQUIRED_V1_TERMINAL_EMPTY_REASONS = {
    "ok_empty",
    "fallback_unavailable",
    "missing_ir",
    "wait_not_ready",
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
