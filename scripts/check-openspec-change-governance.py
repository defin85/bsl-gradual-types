#!/usr/bin/env python3
"""Fail-closed governance checks for an OpenSpec change."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from datetime import date
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

BOOTSTRAP_POLICY_REQUIRED_FOR = {
    "perf_critical",
}

DEPENDENCY_CHECKS_REQUIRED_FOR = {
    "architectural",
    "perf_critical",
}

OWNERSHIP_SIGNOFF_REQUIRED_FOR = {
    "architectural",
    "perf_critical",
}

FAILING_EVIDENCE_HINTS = (
    "fail",
    "failing",
    "regression",
    "reason_codes",
    "before",
)

PASSING_EVIDENCE_HINTS = (
    "pass",
    "passing",
    "resolved",
    "after",
)

REQUIRED_ADR_SECTIONS = (
    "## status",
    "## options considered",
    "## budgets",
    "## rollback",
    "## owners and approvers",
)

REQUIRED_BOOTSTRAP_PROFILES = {
    "small",
    "large",
    "churn",
}

TASK_ITEM_RE = re.compile(r"^\s*-\s*\[(?P<checked>[ xX])\]\s+(?P<item_id>(?:\d+\.\d+|D\d+))\b")

REQUIRED_OWNERSHIP_ROLES = {
    "analysis_v2_owner",
    "runtime_owner",
    "lsp_owner",
    "process_owner",
}

SUCCESS_READINESS_REVIEW = {
    "pass",
    "covered",
    "resolved",
    "ready",
    "complete",
    "closed",
}

FAILURE_READINESS_REVIEW = {
    "partial",
    "gap",
    "not_ready",
    "open",
    "blocked",
    "fail",
    "unresolved",
}

ALLOWED_DECLARED_CHANGE_STATUS = {
    "partial",
    "not_ready",
    "complete",
}

READINESS_TOKEN_RE = re.compile(
    r"\b(pass|covered|resolved|ready|complete|closed|partial|gap|not[_ -]?ready|open|blocked|fail(?:ing)?|unresolved)\b",
    re.IGNORECASE,
)
TRACEABILITY_STATUS_RE = re.compile(r"(?im)^status:\s*`?([^`\n]+)`?\s*$")
VERDICT_HEADING_RE = re.compile(r"(?ims)^##\s*verdict\s*$\s*([^\n]+)")
FINAL_VERDICT_SECTION_RE = re.compile(r"(?ims)^##\s*final verdict\s*$([\s\S]+?)(?:^##\s|\Z)")
ARROW_RESULT_RE = re.compile(r"(?im)->\s*`?([^`\n]+)`?")
DECLARED_STATUS_RE = re.compile(r"(?im)declared status(?:\s+is)?\s*`?([^`\n]+)`?")


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
) -> Path | None:
    resolved = resolve_evidence_ref(repo_root, ref_value)
    if resolved is None:
        return None
    ensure(
        resolved.exists(),
        f"{reason_code}: {source_path} {field_name} does not exist: {ref_value}",
    )
    ensure(
        resolved.is_file(),
        f"{reason_code}: {source_path} {field_name} must point to a file: {ref_value}",
    )
    return resolved


def repo_root_from_artifact_path(path: Path) -> Path:
    for candidate in (path.resolve(), *path.resolve().parents):
        if (candidate / "openspec").exists():
            return candidate
    raise GateError(
        "doc_first_contract_missing: "
        f"unable to resolve repository root for {path}"
    )


def normalize_readiness_token(value: str) -> str:
    normalized = value.strip().strip("`'\"").lower()
    normalized = re.sub(r"[^a-z0-9]+", "_", normalized).strip("_")
    if normalized == "failing":
        return "fail"
    return normalized


def classify_readiness_token(token: str, *, source_path: Path, field_name: str) -> str:
    if token in SUCCESS_READINESS_REVIEW:
        return "success"
    if token in FAILURE_READINESS_REVIEW:
        return "failure"
    raise GateError(
        "readiness_gate_conflict: "
        f"{source_path} {field_name} has unsupported readiness token {token!r}"
    )


def extract_readiness_token(text: str) -> str | None:
    match = READINESS_TOKEN_RE.search(text)
    if match is None:
        return None
    return normalize_readiness_token(match.group(1))


def derive_review_evidence_state(path: Path) -> tuple[str, str]:
    content = path.read_text(encoding="utf-8")

    for regex in (
        VERDICT_HEADING_RE,
        FINAL_VERDICT_SECTION_RE,
        DECLARED_STATUS_RE,
        ARROW_RESULT_RE,
    ):
        for match in regex.finditer(content):
            token = extract_readiness_token(match.group(1))
            if token is not None:
                return token, classify_readiness_token(
                    token, source_path=path, field_name="review_ref"
                )

    for line in content.splitlines():
        lowered = line.lower()
        if "verdict" not in lowered and "status" not in lowered:
            continue
        token = extract_readiness_token(line)
        if token is not None:
            return token, classify_readiness_token(
                token, source_path=path, field_name="review_ref"
            )

    raise GateError(
        "readiness_gate_conflict: "
        f"{path} review_ref does not expose a canonical readiness verdict"
    )


def derive_traceability_evidence_state(path: Path) -> tuple[str, str]:
    content = path.read_text(encoding="utf-8")
    status_tokens = [
        normalize_readiness_token(match.group(1))
        for match in TRACEABILITY_STATUS_RE.finditer(content)
    ]
    if status_tokens:
        categories = [
            classify_readiness_token(
                token, source_path=path, field_name="traceability_ref"
            )
            for token in status_tokens
        ]
        if any(category == "failure" for category in categories):
            failing_token = next(
                token
                for token, category in zip(status_tokens, categories, strict=True)
                if category == "failure"
            )
            return failing_token, "failure"
        return status_tokens[0], "success"

    token, category = derive_review_evidence_state(path)
    return token, category


def ensure_path_within(
    path: Path, parent: Path, *, reason_code: str, message: str
) -> None:
    ensure(
        path.is_relative_to(parent),
        f"{reason_code}: {message}: {path} is outside {parent}",
    )


def validate_adr(path: Path, expected_change_id: str) -> None:
    content = path.read_text(encoding="utf-8")
    normalized = content.lower()
    for section in REQUIRED_ADR_SECTIONS:
        ensure(
            section in normalized,
            f"adr_missing_or_not_approved: {path} missing required section {section!r}",
        )

    status_match = re.search(r"(?ims)^##\s*status\s*$\s*([^\n]+)", content)
    ensure(
        status_match is not None,
        f"adr_missing_or_not_approved: {path} unable to parse status section",
    )
    status_value = status_match.group(1).strip().lower()
    ensure(
        status_value == "accepted",
        f"adr_missing_or_not_approved: {path} status must be 'accepted', got {status_value!r}",
    )
    ensure(
        expected_change_id in content,
        f"adr_missing_or_not_approved: {path} must reference change_id {expected_change_id!r}",
    )


def validate_test_first_evidence(
    path: Path, expected_change_id: str, repo_root: Path, change_root: Path
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
    ensure(
        payload["failing_ref"] != payload["passing_ref"],
        f"test_first_evidence_missing_or_invalid: {path} failing_ref and passing_ref must differ",
    )
    ensure(
        isinstance(payload.get("rule_id"), str) and payload["rule_id"].strip(),
        f"test_first_evidence_missing_or_invalid: {path} rule_id is required",
    )
    scope_token = re.sub(r"[^a-z0-9]+", "_", payload["scope"].strip().lower()).strip("_")
    rule_token = re.sub(r"[^a-z0-9]+", "_", payload["rule_id"].strip().lower()).strip("_")
    ensure(
        scope_token and scope_token in rule_token,
        (
            "test_first_evidence_missing_or_invalid: "
            f"{path} rule_id must encode scope token {scope_token!r}"
        ),
    )

    failing_path = validate_evidence_ref(
        repo_root,
        payload["failing_ref"],
        field_name="failing_ref",
        reason_code="test_first_evidence_missing_or_invalid",
        source_path=path,
    )
    passing_path = validate_evidence_ref(
        repo_root,
        payload["passing_ref"],
        field_name="passing_ref",
        reason_code="test_first_evidence_missing_or_invalid",
        source_path=path,
    )
    ensure(
        failing_path is not None and passing_path is not None,
        (
            "test_first_evidence_missing_or_invalid: "
            f"{path} failing_ref/passing_ref must be repository files"
        ),
    )
    ensure_path_within(
        failing_path,
        change_root,
        reason_code="test_first_evidence_missing_or_invalid",
        message=f"{path} failing_ref must point inside change root",
    )
    ensure_path_within(
        passing_path,
        change_root,
        reason_code="test_first_evidence_missing_or_invalid",
        message=f"{path} passing_ref must point inside change root",
    )

    failing_text = failing_path.read_text(encoding="utf-8").lower()
    passing_text = passing_path.read_text(encoding="utf-8").lower()
    ensure(
        any(token in failing_text for token in FAILING_EVIDENCE_HINTS),
        (
            "test_first_evidence_missing_or_invalid: "
            f"{path} failing_ref must contain failure evidence hints"
        ),
    )
    ensure(
        any(token in passing_text for token in PASSING_EVIDENCE_HINTS),
        (
            "test_first_evidence_missing_or_invalid: "
            f"{path} passing_ref must contain passing evidence hints"
        ),
    )
    reason_codes = payload.get("reason_codes")
    ensure(
        isinstance(reason_codes, list) and all(isinstance(item, str) for item in reason_codes),
        f"test_first_evidence_missing_or_invalid: {path} reason_codes must be string array",
    )


def validate_bootstrap_policy(
    path: Path, expected_change_id: str, repo_root: Path, change_root: Path
) -> None:
    payload = parse_json(path)
    ensure(
        payload.get("schema_version") == "v1",
        f"initial_budget_not_fixed: {path} schema_version must be 'v1'",
    )
    ensure(
        payload.get("change_id") == expected_change_id,
        f"initial_budget_not_fixed: {path} change_id mismatch",
    )
    sample_size_min = payload.get("sample_size_min")
    ensure(
        isinstance(sample_size_min, int) and sample_size_min >= 5,
        f"initial_budget_not_fixed: {path} sample_size_min must be integer >= 5",
    )
    ensure(
        payload.get("aggregation_rule") == "median",
        f"initial_budget_not_fixed: {path} aggregation_rule must be 'median'",
    )
    profiles = payload.get("required_profiles")
    ensure(
        isinstance(profiles, list) and all(isinstance(item, str) for item in profiles),
        f"initial_budget_not_fixed: {path} required_profiles must be string array",
    )
    ensure(
        REQUIRED_BOOTSTRAP_PROFILES.issubset(set(profiles)),
        (
            "initial_budget_not_fixed: "
            f"{path} required_profiles must include {sorted(REQUIRED_BOOTSTRAP_PROFILES)}"
        ),
    )
    approval_ref = payload.get("approval_ref")
    ensure(
        isinstance(approval_ref, str) and approval_ref.strip(),
        f"initial_budget_not_fixed: {path} approval_ref is required",
    )
    approval_path = validate_evidence_ref(
        repo_root,
        approval_ref,
        field_name="approval_ref",
        reason_code="initial_budget_not_fixed",
        source_path=path,
    )
    ensure(
        approval_path is not None,
        f"initial_budget_not_fixed: {path} approval_ref must point to a repository file",
    )
    ensure_path_within(
        approval_path,
        change_root,
        reason_code="initial_budget_not_fixed",
        message=f"{path} approval_ref must point inside change root",
    )


def parse_tasks_status(path: Path) -> tuple[dict[str, bool], dict[str, bool]]:
    task_status: dict[str, bool] = {}
    dependency_status: dict[str, bool] = {}
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        match = TASK_ITEM_RE.match(raw_line)
        if not match:
            continue
        item_id = match.group("item_id")
        checked = match.group("checked").strip().lower() == "x"
        if item_id.startswith("D"):
            dependency_status[item_id] = checked
        else:
            task_status[item_id] = checked
    return task_status, dependency_status


def validate_dependency_checks(path: Path, expected_change_id: str, tasks_path: Path) -> None:
    payload = parse_json(path)
    ensure(
        payload.get("schema_version") == "v1",
        f"doc_first_contract_missing: {path} schema_version must be 'v1'",
    )
    ensure(
        payload.get("change_id") == expected_change_id,
        f"doc_first_contract_missing: {path} change_id mismatch",
    )
    repo_root = repo_root_from_artifact_path(path)
    change_root = path.parent.parent
    for field_name in ("traceability_ref", "protected_assets_manifest_ref"):
        raw_ref = payload.get(field_name)
        ensure(
            isinstance(raw_ref, str) and raw_ref.strip(),
            f"doc_first_contract_missing: {path} {field_name} is required",
        )
        evidence_path = validate_evidence_ref(
            repo_root,
            raw_ref,
            field_name=field_name,
            reason_code="doc_first_contract_missing",
            source_path=path,
        )
        ensure(
            evidence_path is not None,
            f"doc_first_contract_missing: {path} {field_name} must point to a repository file",
        )
        ensure_path_within(
            evidence_path,
            change_root,
            reason_code="doc_first_contract_missing",
            message=f"{path} {field_name} must point inside change root",
        )
    dependencies = payload.get("dependencies")
    ensure(
        isinstance(dependencies, list) and dependencies,
        f"doc_first_contract_missing: {path} dependencies must be a non-empty array",
    )

    task_status, dependency_status = parse_tasks_status(tasks_path)
    ensure(
        task_status,
        f"doc_first_contract_missing: {tasks_path} does not contain machine-readable task ids",
    )
    ensure(
        dependency_status,
        f"doc_first_contract_missing: {tasks_path} does not contain dependency ids (D*)",
    )

    declared_ids: set[str] = set()
    for dep in dependencies:
        ensure(
            isinstance(dep, dict),
            f"doc_first_contract_missing: {path} each dependency must be an object",
        )
        dep_id = dep.get("id")
        ensure(
            isinstance(dep_id, str) and re.fullmatch(r"D[1-9]\d*", dep_id),
            f"doc_first_contract_missing: {path} dependency id must match D<n>",
        )
        ensure(
            dep_id not in declared_ids,
            f"doc_first_contract_missing: {path} duplicate dependency id {dep_id!r}",
        )
        declared_ids.add(dep_id)
        ensure(
            dep_id in dependency_status,
            f"doc_first_contract_missing: {path} dependency {dep_id!r} is missing in {tasks_path}",
        )
        ensure(
            dependency_status[dep_id],
            f"doc_first_contract_missing: {tasks_path} dependency {dep_id!r} must be checked",
        )

        requires = dep.get("requires")
        blocked = dep.get("blocked")
        ensure(
            isinstance(requires, list)
            and requires
            and all(isinstance(item, str) for item in requires),
            f"doc_first_contract_missing: {path} dependency {dep_id!r} requires must be non-empty string array",
        )
        ensure(
            isinstance(blocked, list)
            and blocked
            and all(isinstance(item, str) for item in blocked),
            f"doc_first_contract_missing: {path} dependency {dep_id!r} blocked must be non-empty string array",
        )

        for item_id in requires + blocked:
            ensure(
                item_id in task_status,
                (
                    "doc_first_contract_missing: "
                    f"{path} dependency {dep_id!r} references unknown task {item_id!r}"
                ),
            )
        for blocked_task in blocked:
            if task_status[blocked_task]:
                missing_prerequisites = [
                    required for required in requires if not task_status[required]
                ]
                ensure(
                    not missing_prerequisites,
                    (
                        "doc_first_contract_missing: "
                        f"{path} dependency {dep_id!r} violated in {tasks_path}: "
                        f"{blocked_task} checked before prerequisites {missing_prerequisites}"
                    ),
                )

    missing_from_artifact = sorted(set(dependency_status) - declared_ids)
    ensure(
        not missing_from_artifact,
        (
            "doc_first_contract_missing: "
            f"{path} does not cover dependencies from {tasks_path}: {missing_from_artifact}"
        ),
    )


def validate_ownership_signoff(
    path: Path, expected_change_id: str, repo_root: Path, change_root: Path
) -> None:
    payload = parse_json(path)
    ensure(
        payload.get("schema_version") == "v1",
        f"adr_missing_or_not_approved: {path} schema_version must be 'v1'",
    )
    ensure(
        payload.get("change_id") == expected_change_id,
        f"adr_missing_or_not_approved: {path} change_id mismatch",
    )
    signoffs = payload.get("signoffs")
    ensure(
        isinstance(signoffs, list) and signoffs,
        f"adr_missing_or_not_approved: {path} signoffs must be a non-empty array",
    )
    seen_roles: set[str] = set()
    for signoff in signoffs:
        ensure(
            isinstance(signoff, dict),
            f"adr_missing_or_not_approved: {path} each signoff entry must be object",
        )
        role = signoff.get("role")
        ensure(
            isinstance(role, str) and role.strip(),
            f"adr_missing_or_not_approved: {path} signoff role is required",
        )
        ensure(
            role not in seen_roles,
            f"adr_missing_or_not_approved: {path} duplicate role {role!r}",
        )
        seen_roles.add(role)
        status = signoff.get("status")
        ensure(
            status == "approved",
            f"adr_missing_or_not_approved: {path} role {role!r} status must be 'approved'",
        )
        reviewed_on = signoff.get("reviewed_on")
        ensure(
            isinstance(reviewed_on, str) and reviewed_on.strip(),
            f"adr_missing_or_not_approved: {path} role {role!r} reviewed_on is required",
        )
        try:
            date.fromisoformat(reviewed_on)
        except ValueError as exc:
            raise GateError(
                "adr_missing_or_not_approved: "
                f"{path} role {role!r} reviewed_on must be YYYY-MM-DD"
            ) from exc
        evidence_ref = signoff.get("evidence_ref")
        ensure(
            isinstance(evidence_ref, str) and evidence_ref.strip(),
            f"adr_missing_or_not_approved: {path} role {role!r} evidence_ref is required",
        )
        evidence_path = validate_evidence_ref(
            repo_root,
            evidence_ref,
            field_name="evidence_ref",
            reason_code="adr_missing_or_not_approved",
            source_path=path,
        )
        ensure(
            evidence_path is not None,
            (
                "adr_missing_or_not_approved: "
                f"{path} role {role!r} evidence_ref must point to a repository file"
            ),
        )
        ensure_path_within(
            evidence_path,
            change_root,
            reason_code="adr_missing_or_not_approved",
            message=f"{path} role {role!r} evidence_ref must point inside change root",
        )

    missing_roles = sorted(REQUIRED_OWNERSHIP_ROLES - seen_roles)
    ensure(
        not missing_roles,
        (
            "adr_missing_or_not_approved: "
            f"{path} missing required sign-off roles: {missing_roles}"
        ),
    )


def change_requires_readiness_backlog_gate(change_root: Path) -> bool:
    spec_path = change_root / "specs" / "dev-workflow" / "spec.md"
    if not spec_path.exists() or not spec_path.is_file():
        return False

    content = spec_path.read_text(encoding="utf-8").lower()
    has_gate_requirement = "change completion must not" in content
    mentions_backlog = "beads backlog" in content and (
        "must backlog" in content or "follow-up backlog" in content
    )
    return has_gate_requirement and mentions_backlog


def fetch_beads_issue_statuses(repo_root: Path, issue_ids: list[str]) -> dict[str, str]:
    completed = subprocess.run(
        ["bd", "show", "--json", *issue_ids],
        cwd=repo_root,
        check=False,
        capture_output=True,
        text=True,
    )
    ensure(
        completed.returncode == 0,
        (
            "readiness_gate_conflict: unable to read Beads backlog status "
            f"(exit={completed.returncode}): {completed.stderr.strip() or completed.stdout.strip()}"
        ),
    )
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise GateError(
            "readiness_gate_conflict: invalid JSON from `bd show --json`"
        ) from exc

    ensure(
        isinstance(payload, list),
        "readiness_gate_conflict: `bd show --json` must return a JSON array",
    )

    statuses: dict[str, str] = {}
    for item in payload:
        ensure(
            isinstance(item, dict),
            "readiness_gate_conflict: each Beads item must be a JSON object",
        )
        issue_id = item.get("id")
        status = item.get("status")
        ensure(
            isinstance(issue_id, str) and issue_id.strip(),
            "readiness_gate_conflict: Beads item is missing id",
        )
        ensure(
            isinstance(status, str) and status.strip(),
            f"readiness_gate_conflict: Beads item {issue_id!r} is missing status",
        )
        statuses[issue_id] = status

    missing_ids = [issue_id for issue_id in issue_ids if issue_id not in statuses]
    ensure(
        not missing_ids,
        f"readiness_gate_conflict: Beads did not return statuses for {missing_ids}",
    )
    return statuses


def validate_readiness_status(
    path: Path, expected_change_id: str, repo_root: Path, change_root: Path
) -> None:
    payload = parse_json(path)
    ensure(
        payload.get("schema_version") == "v1",
        f"readiness_gate_conflict: {path} schema_version must be 'v1'",
    )
    ensure(
        payload.get("change_id") == expected_change_id,
        f"readiness_gate_conflict: {path} change_id mismatch",
    )
    declared_status = payload.get("declared_status")
    ensure(
        declared_status in ALLOWED_DECLARED_CHANGE_STATUS,
        (
            "readiness_gate_conflict: "
            f"{path} declared_status must be one of {sorted(ALLOWED_DECLARED_CHANGE_STATUS)}"
        ),
    )

    declared_evidence_states: dict[str, tuple[str, str]] = {}
    for field_name in ("review_verdict", "traceability_status"):
        value = payload.get(field_name)
        ensure(
            isinstance(value, str) and value.strip(),
            f"readiness_gate_conflict: {path} {field_name} is required",
        )
        token = normalize_readiness_token(value)
        declared_evidence_states[field_name] = (
            token,
            classify_readiness_token(token, source_path=path, field_name=field_name),
        )

    evidence_refs: dict[str, Path] = {}
    for field_name in ("review_ref", "traceability_ref"):
        raw_ref = payload.get(field_name)
        ensure(
            isinstance(raw_ref, str) and raw_ref.strip(),
            f"readiness_gate_conflict: {path} {field_name} is required",
        )
        evidence_path = validate_evidence_ref(
            repo_root,
            raw_ref,
            field_name=field_name,
            reason_code="readiness_gate_conflict",
            source_path=path,
        )
        ensure(
            evidence_path is not None,
            f"readiness_gate_conflict: {path} {field_name} must point to a repository file",
        )
        ensure_path_within(
            evidence_path,
            change_root,
            reason_code="readiness_gate_conflict",
            message=f"{path} {field_name} must point inside change root",
        )
        evidence_refs[field_name] = evidence_path

    critical_backlog = payload.get("critical_backlog")
    ensure(
        isinstance(critical_backlog, list)
        and critical_backlog
        and all(isinstance(item, str) and item.strip() for item in critical_backlog),
        f"readiness_gate_conflict: {path} critical_backlog must be a non-empty string array",
    )

    superseding_delivery_path = payload.get("superseding_delivery_path")
    superseding_approved = False
    if superseding_delivery_path is not None:
        ensure(
            isinstance(superseding_delivery_path, str) and superseding_delivery_path.strip(),
            f"readiness_gate_conflict: {path} superseding_delivery_path must be null or non-empty string",
        )
        superseding_path = validate_evidence_ref(
            repo_root,
            superseding_delivery_path,
            field_name="superseding_delivery_path",
            reason_code="readiness_gate_conflict",
            source_path=path,
        )
        ensure(
            superseding_path is not None,
            (
                "readiness_gate_conflict: "
                f"{path} superseding_delivery_path must point to a repository file"
            ),
        )
        ensure_path_within(
            superseding_path,
            change_root,
            reason_code="readiness_gate_conflict",
            message=f"{path} superseding_delivery_path must point inside change root",
        )
        superseding_text = superseding_path.read_text(encoding="utf-8").lower()
        superseding_approved = "approved" in superseding_text
        ensure(
            superseding_approved,
            (
                "readiness_gate_conflict: "
                f"{path} superseding_delivery_path must contain approved handoff evidence"
            ),
        )

    statuses = fetch_beads_issue_statuses(repo_root, critical_backlog)
    open_backlog = [
        issue_id for issue_id, status in statuses.items() if status.strip().lower() != "closed"
    ]

    review_token, review_category = derive_review_evidence_state(evidence_refs["review_ref"])
    traceability_token, traceability_category = derive_traceability_evidence_state(
        evidence_refs["traceability_ref"]
    )
    declared_review_token, declared_review_category = declared_evidence_states["review_verdict"]
    declared_traceability_token, declared_traceability_category = declared_evidence_states[
        "traceability_status"
    ]

    ensure(
        declared_review_category == review_category,
        (
            "readiness_gate_conflict: "
            f"{path} review_ref={evidence_refs['review_ref']} yields {review_token!r}, "
            f"but review_verdict={declared_review_token!r}"
        ),
    )
    ensure(
        declared_traceability_category == traceability_category,
        (
            "readiness_gate_conflict: "
            f"{path} traceability_ref={evidence_refs['traceability_ref']} yields {traceability_token!r}, "
            f"but traceability_status={declared_traceability_token!r}"
        ),
    )
    ensure(
        review_category == traceability_category,
        (
            "readiness_gate_conflict: "
            f"{path} review/traceability evidence disagree: "
            f"review_ref={review_token!r}, traceability_ref={traceability_token!r}"
        ),
    )

    has_conflicting_evidence = (
        review_category != "success" or traceability_category != "success"
    )

    if declared_status == "complete":
        ensure(
            not has_conflicting_evidence,
            (
                "readiness_gate_conflict: "
                f"{path} declared_status='complete' conflicts with "
                f"review_verdict={review_token!r} traceability_status={traceability_token!r}"
            ),
        )
        ensure(
            not open_backlog or superseding_approved,
            (
                "readiness_gate_conflict: "
                f"{path} declared_status='complete' is blocked by open critical backlog "
                f"{open_backlog}"
            ),
        )
        return

    ensure(
        declared_status in {"partial", "not_ready"},
        f"readiness_gate_conflict: unexpected declared_status {declared_status!r}",
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
                change_root,
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
            validate_adr(governance_root / "adr.md", args.change_id)

        if criticality in PROTECTED_ASSETS_REQUIRED_FOR:
            check_required_file(
                governance_root / "protected_assets_manifest.txt",
                "protected_acceptance_asset_modified",
            )

        if criticality in BOOTSTRAP_POLICY_REQUIRED_FOR:
            check_required_file(
                governance_root / "bootstrap_policy.json",
                "initial_budget_not_fixed",
            )
            validate_bootstrap_policy(
                governance_root / "bootstrap_policy.json",
                args.change_id,
                repo_root,
                change_root,
            )

        if criticality in DEPENDENCY_CHECKS_REQUIRED_FOR:
            check_required_file(
                governance_root / "dependency_checks.json",
                "doc_first_contract_missing",
            )
            validate_dependency_checks(
                governance_root / "dependency_checks.json",
                args.change_id,
                change_root / "tasks.md",
            )

        if criticality in OWNERSHIP_SIGNOFF_REQUIRED_FOR:
            check_required_file(
                governance_root / "ownership_signoff.json",
                "adr_missing_or_not_approved",
            )
            validate_ownership_signoff(
                governance_root / "ownership_signoff.json",
                args.change_id,
                repo_root,
                change_root,
            )

        if change_requires_readiness_backlog_gate(change_root):
            check_required_file(
                governance_root / "readiness_status.json",
                "readiness_gate_conflict",
            )
            validate_readiness_status(
                governance_root / "readiness_status.json",
                args.change_id,
                repo_root,
                change_root,
            )
    except GateError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    print(f"OpenSpec governance gate passed for {args.change_id}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
