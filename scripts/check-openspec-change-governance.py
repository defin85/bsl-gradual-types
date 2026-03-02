#!/usr/bin/env python3
"""Fail-closed governance checks for an OpenSpec change."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


ALLOWED_CRITICALITY = {
    "routine",
    "behavioral",
    "architectural",
    "perf_critical",
}

TEST_FIRST_REQUIRED_FOR = {
    "behavioral",
    "architectural",
    "perf_critical",
}

ADR_REQUIRED_FOR = {
    "architectural",
    "perf_critical",
}

PROTECTED_ASSETS_REQUIRED_FOR = {
    "architectural",
    "perf_critical",
}


class GateError(Exception):
    pass


def ensure(condition: bool, message: str) -> None:
    if not condition:
        raise GateError(message)


def parse_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise GateError(f"{path}: invalid JSON: {exc}") from exc
    ensure(isinstance(data, dict), f"{path}: JSON root must be object")
    return data


def check_required_file(path: Path, reason_code: str) -> None:
    ensure(path.exists(), f"{reason_code}: missing {path}")
    ensure(path.is_file(), f"{reason_code}: expected file, got {path}")


def validate_change_criticality(path: Path, expected_change_id: str) -> str:
    payload = parse_json(path)
    ensure(
        payload.get("schema_version") == "v1",
        f"change_criticality_missing_or_unknown: {path} schema_version must be 'v1'",
    )
    ensure(
        payload.get("change_id") == expected_change_id,
        f"change_criticality_missing_or_unknown: {path} change_id mismatch",
    )
    criticality = payload.get("change_criticality")
    ensure(
        criticality in ALLOWED_CRITICALITY,
        (
            "change_criticality_missing_or_unknown: "
            f"{path} change_criticality must be one of {sorted(ALLOWED_CRITICALITY)}"
        ),
    )
    ensure(
        isinstance(payload.get("rule_id"), str) and payload["rule_id"].strip(),
        f"change_criticality_missing_or_unknown: {path} rule_id is required",
    )
    ensure(
        isinstance(payload.get("reason"), str) and payload["reason"].strip(),
        f"change_criticality_missing_or_unknown: {path} reason is required",
    )
    return criticality


def validate_acceptance_matrix(change_root: Path) -> None:
    candidates = [
        change_root / "validation" / "acceptance_matrix.md",
        change_root / "validation" / "acceptance-matrix.md",
        change_root / "acceptance_matrix.md",
        change_root / "acceptance-matrix.md",
    ]
    matrix_path = next(
        (path for path in candidates if path.exists() and path.is_file()),
        None,
    )
    ensure(
        matrix_path is not None,
        (
            "doc_first_contract_missing: acceptance matrix is required for non-MVP "
            "architectural/perf_critical changes "
            f"(expected one of: {', '.join(str(path) for path in candidates)})"
        ),
    )
    content = matrix_path.read_text(encoding="utf-8").lower()
    has_pass = any(token in content for token in ("pass", "критерии успеха", "успех"))
    has_fail = any(token in content for token in ("fail", "критерии провала", "провал"))
    ensure(
        has_pass and has_fail,
        (
            "doc_first_contract_missing: acceptance matrix must include explicit pass/fail "
            f"criteria ({matrix_path})"
        ),
    )


def resolve_evidence_ref(repo_root: Path, raw_ref: str) -> Path | None:
    if "://" in raw_ref:
        return None
    ref_path = Path(raw_ref)
    if not ref_path.is_absolute():
        ref_path = (repo_root / ref_path).resolve()
    return ref_path


def validate_evidence_ref(
    repo_root: Path,
    ref_value: str,
    *,
    field_name: str,
    reason_code: str,
    source_path: Path,
) -> None:
    resolved = resolve_evidence_ref(repo_root, ref_value)
    if resolved is None:
        return
    ensure(
        resolved.exists(),
        f"{reason_code}: {source_path} {field_name} does not exist: {ref_value}",
    )
    ensure(
        resolved.is_file(),
        f"{reason_code}: {source_path} {field_name} must point to a file: {ref_value}",
    )


def validate_test_first_evidence(
    path: Path, expected_change_id: str, repo_root: Path
) -> None:
    payload = parse_json(path)
    ensure(
        payload.get("schema_version") == "v1",
        f"test_first_evidence_missing_or_invalid: {path} schema_version must be 'v1'",
    )
    ensure(
        payload.get("change_id") == expected_change_id,
        f"test_first_evidence_missing_or_invalid: {path} change_id mismatch",
    )
    ensure(
        isinstance(payload.get("scope"), str) and payload["scope"].strip(),
        f"test_first_evidence_missing_or_invalid: {path} scope is required",
    )
    ensure(
        isinstance(payload.get("failing_ref"), str) and payload["failing_ref"].strip(),
        f"test_first_evidence_missing_or_invalid: {path} failing_ref is required",
    )
    ensure(
        isinstance(payload.get("passing_ref"), str) and payload["passing_ref"].strip(),
        f"test_first_evidence_missing_or_invalid: {path} passing_ref is required",
    )
    validate_evidence_ref(
        repo_root,
        payload["failing_ref"],
        field_name="failing_ref",
        reason_code="test_first_evidence_missing_or_invalid",
        source_path=path,
    )
    validate_evidence_ref(
        repo_root,
        payload["passing_ref"],
        field_name="passing_ref",
        reason_code="test_first_evidence_missing_or_invalid",
        source_path=path,
    )
    reason_codes = payload.get("reason_codes")
    ensure(
        isinstance(reason_codes, list) and all(isinstance(item, str) for item in reason_codes),
        f"test_first_evidence_missing_or_invalid: {path} reason_codes must be string array",
    )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate governance artifacts for OpenSpec change."
    )
    parser.add_argument(
        "--change-id",
        required=True,
        help="OpenSpec change id (e.g. add-performance-first-ai-engineering-guardrails)",
    )
    parser.add_argument(
        "--repo-root",
        default=str(Path(__file__).resolve().parents[1]),
        help="Repository root path",
    )
    args = parser.parse_args()

    repo_root = Path(args.repo_root).resolve()
    change_root = repo_root / "openspec" / "changes" / args.change_id
    governance_root = change_root / "governance"

    try:
        ensure(change_root.exists(), f"change_not_found: {change_root}")
        check_required_file(
            change_root / "proposal.md",
            "doc_first_contract_missing",
        )
        check_required_file(
            change_root / "tasks.md",
            "doc_first_contract_missing",
        )
        ensure(
            (change_root / "specs").exists(),
            f"doc_first_contract_missing: missing {change_root / 'specs'}",
        )

        check_required_file(
            governance_root / "change_criticality.json",
            "change_criticality_missing_or_unknown",
        )
        criticality = validate_change_criticality(
            governance_root / "change_criticality.json",
            args.change_id,
        )

        if criticality in ADR_REQUIRED_FOR:
            validate_acceptance_matrix(change_root)

        if criticality in TEST_FIRST_REQUIRED_FOR:
            check_required_file(
                governance_root / "test_first_evidence.json",
                "test_first_evidence_missing_or_invalid",
            )
            validate_test_first_evidence(
                governance_root / "test_first_evidence.json",
                args.change_id,
                repo_root,
            )

        if criticality in ADR_REQUIRED_FOR:
            check_required_file(
                change_root / "design.md",
                "adr_missing_or_not_approved",
            )
            check_required_file(
                governance_root / "adr.md",
                "adr_missing_or_not_approved",
            )

        if criticality in PROTECTED_ASSETS_REQUIRED_FOR:
            check_required_file(
                governance_root / "protected_assets_manifest.txt",
                "protected_acceptance_asset_modified",
            )
    except GateError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    print(f"OpenSpec governance gate passed for {args.change_id}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
