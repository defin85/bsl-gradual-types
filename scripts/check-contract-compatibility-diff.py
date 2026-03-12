#!/usr/bin/env python3
"""Compatibility-diff gate for versioned contracts."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


RE_VERSION_DIR = re.compile(r"^v([1-9]\d*)$")


class CompatibilityError(Exception):
    pass


@dataclass(frozen=True)
class ContractVersion:
    major: int
    contract_path: str
    changelog_path: str
    contract: dict[str, Any]
    changelog: str


@dataclass(frozen=True)
class DiffIssue:
    classification: str  # breaking | non_breaking
    reason: str
    path: str
    baseline: Any
    candidate: Any


def ensure(condition: bool, message: str) -> None:
    if not condition:
        raise CompatibilityError(message)


def parse_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise CompatibilityError(f"{path}: invalid json: {exc}") from exc
    ensure(isinstance(data, dict), f"{path}: json root must be object")
    return data


def parse_major(name: str, where: str) -> int:
    match = RE_VERSION_DIR.match(name)
    if not match:
        raise CompatibilityError(f"{where}: invalid version directory name {name!r}")
    return int(match.group(1))


def git_stdout(*args: str) -> str:
    completed = subprocess.run(
        ["git", *args],
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise CompatibilityError(
            f"git {' '.join(args)} failed: {completed.stderr.strip() or completed.stdout.strip()}"
        )
    return completed.stdout


def materialize_contracts_from_ref(git_ref: str) -> Path:
    with tempfile.TemporaryDirectory(prefix="contracts-baseline-") as tmp_dir:
        root = Path(tmp_dir)
        listed = git_stdout("ls-tree", "-r", "--name-only", git_ref, "contracts").splitlines()
        files = [line.strip() for line in listed if line.strip()]
        ensure(files, f"git ref {git_ref!r} does not contain contracts/**")
        for rel_path in files:
            target = root / rel_path
            target.parent.mkdir(parents=True, exist_ok=True)
            blob = subprocess.run(
                ["git", "show", f"{git_ref}:{rel_path}"],
                check=False,
                capture_output=True,
            )
            if blob.returncode != 0:
                raise CompatibilityError(
                    f"git show {git_ref}:{rel_path} failed: {blob.stderr.decode('utf-8', errors='ignore').strip()}"
                )
            target.write_bytes(blob.stdout)

        # Copy the materialized tree into a persistent temp dir to survive context exit.
        persistent = Path(tempfile.mkdtemp(prefix="contracts-baseline-persist-"))
        shutil.copytree(root / "contracts", persistent / "contracts")
        return persistent / "contracts"


def load_contract_tree(contracts_root: Path) -> dict[str, dict[int, ContractVersion]]:
    ensure(contracts_root.exists(), f"{contracts_root}: contracts root is missing")
    ensure(contracts_root.is_dir(), f"{contracts_root}: contracts root must be a directory")
    surfaces: dict[str, dict[int, ContractVersion]] = {}
    for surface_dir in sorted(path for path in contracts_root.iterdir() if path.is_dir()):
        versions: dict[int, ContractVersion] = {}
        for version_dir in sorted(path for path in surface_dir.iterdir() if path.is_dir()):
            if not RE_VERSION_DIR.match(version_dir.name):
                continue
            major = parse_major(version_dir.name, str(version_dir))
            contract_path = version_dir / "contract.json"
            changelog_path = version_dir / "changelog.md"
            ensure(contract_path.exists(), f"{contract_path}: missing")
            ensure(changelog_path.exists(), f"{changelog_path}: missing")
            versions[major] = ContractVersion(
                major=major,
                contract_path=str(contract_path),
                changelog_path=str(changelog_path),
                contract=parse_json(contract_path),
                changelog=changelog_path.read_text(encoding="utf-8"),
            )
        if versions:
            surfaces[surface_dir.name] = versions
    ensure(surfaces, f"{contracts_root}: no contract surfaces found")
    return surfaces


def is_scalar(value: Any) -> bool:
    return value is None or isinstance(value, (str, int, float, bool))


def json_fingerprint(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True)


def classify_contract_diff(
    baseline: Any,
    candidate: Any,
    path: str = "$",
) -> list[DiffIssue]:
    issues: list[DiffIssue] = []

    if type(baseline) is not type(candidate):  # noqa: E721
        issues.append(
            DiffIssue(
                classification="breaking",
                reason="type_changed",
                path=path,
                baseline=baseline,
                candidate=candidate,
            )
        )
        return issues

    if isinstance(baseline, dict):
        for key in sorted(baseline.keys()):
            child_path = f"{path}.{key}"
            if key not in candidate:
                issues.append(
                    DiffIssue(
                        classification="breaking",
                        reason="key_removed",
                        path=child_path,
                        baseline=baseline[key],
                        candidate=None,
                    )
                )
                continue
            issues.extend(classify_contract_diff(baseline[key], candidate[key], child_path))

        for key in sorted(candidate.keys()):
            if key in baseline:
                continue
            child_path = f"{path}.{key}"
            issues.append(
                DiffIssue(
                    classification="non_breaking",
                    reason="key_added",
                    path=child_path,
                    baseline=None,
                    candidate=candidate[key],
                )
            )
        return issues

    if isinstance(baseline, list):
        if all(is_scalar(item) for item in baseline) and all(
            is_scalar(item) for item in candidate
        ):
            baseline_set = {json_fingerprint(item) for item in baseline}
            candidate_set = {json_fingerprint(item) for item in candidate}
            removed = sorted(baseline_set - candidate_set)
            added = sorted(candidate_set - baseline_set)
            if removed:
                issues.append(
                    DiffIssue(
                        classification="breaking",
                        reason="list_values_removed",
                        path=path,
                        baseline=removed,
                        candidate=sorted(candidate_set),
                    )
                )
            if added:
                issues.append(
                    DiffIssue(
                        classification="non_breaking",
                        reason="list_values_added",
                        path=path,
                        baseline=sorted(baseline_set),
                        candidate=added,
                    )
                )
            return issues

        if baseline != candidate:
            issues.append(
                DiffIssue(
                    classification="breaking",
                    reason="list_changed",
                    path=path,
                    baseline=baseline,
                    candidate=candidate,
                )
            )
        return issues

    if baseline != candidate:
        issues.append(
            DiffIssue(
                classification="breaking",
                reason="value_changed",
                path=path,
                baseline=baseline,
                candidate=candidate,
            )
        )
    return issues


def has_migration_note(changelog: str) -> bool:
    normalized = changelog.lower()
    return "migration note:" in normalized or "migration:" in normalized


def summarize_surface_diff(
    surface: str,
    baseline_versions: dict[int, ContractVersion],
    candidate_versions: dict[int, ContractVersion],
) -> dict[str, Any]:
    violations: list[str] = []
    issues: list[DiffIssue] = []

    baseline_major = max(baseline_versions.keys())
    candidate_major = max(candidate_versions.keys())
    major_bump = candidate_major > baseline_major

    if candidate_major < baseline_major:
        violations.append("candidate_major_regressed")
        compared_candidate_major = candidate_major
    else:
        compared_candidate_major = candidate_major if major_bump else baseline_major

    if baseline_major not in candidate_versions:
        violations.append("missing_previous_major_in_candidate")

    if major_bump and candidate_major != baseline_major + 1:
        violations.append("non_contiguous_major_bump")

    baseline_contract = baseline_versions[baseline_major].contract
    if compared_candidate_major in candidate_versions:
        candidate_contract = candidate_versions[compared_candidate_major].contract
        issues = classify_contract_diff(baseline_contract, candidate_contract)
    else:
        violations.append("missing_compared_candidate_version")
        candidate_contract = {}

    breaking_issues = [issue for issue in issues if issue.classification == "breaking"]
    non_breaking_issues = [issue for issue in issues if issue.classification == "non_breaking"]

    if breaking_issues and not major_bump:
        violations.append("breaking_without_major_bump")

    if major_bump:
        new_changelog = candidate_versions[candidate_major].changelog
        if not has_migration_note(new_changelog):
            violations.append("missing_migration_note")

    issue_payload = [
        {
            "classification": issue.classification,
            "reason": issue.reason,
            "path": issue.path,
            "baseline": issue.baseline,
            "candidate": issue.candidate,
        }
        for issue in sorted(
            issues,
            key=lambda item: (
                item.classification,
                item.reason,
                item.path,
                json_fingerprint(item.baseline),
                json_fingerprint(item.candidate),
            ),
        )
    ]

    return {
        "surface": surface,
        "compared_versions": {
            "baseline_major": baseline_major,
            "candidate_major": compared_candidate_major,
            "candidate_latest_major": candidate_major,
            "major_bump": major_bump,
        },
        "diff_classification": "breaking" if breaking_issues else "non_breaking",
        "issues": issue_payload,
        "violations": sorted(set(violations)),
        "pass": len(violations) == 0,
    }


def build_report(
    baseline_tree: dict[str, dict[int, ContractVersion]],
    candidate_tree: dict[str, dict[int, ContractVersion]],
    baseline_source: str,
    candidate_source: str,
) -> dict[str, Any]:
    surfaces = sorted(set(baseline_tree.keys()) | set(candidate_tree.keys()))
    surface_reports: list[dict[str, Any]] = []
    global_violations: list[str] = []

    for surface in surfaces:
        if surface not in baseline_tree:
            surface_reports.append(
                {
                    "surface": surface,
                    "compared_versions": None,
                    "diff_classification": "non_breaking",
                    "issues": [
                        {
                            "classification": "non_breaking",
                            "reason": "surface_added",
                            "path": "$",
                            "baseline": None,
                            "candidate": {
                                "candidate_latest_major": max(candidate_tree[surface].keys())
                            },
                        }
                    ],
                    "violations": [],
                    "pass": True,
                }
            )
            continue
        if surface not in candidate_tree:
            surface_reports.append(
                {
                    "surface": surface,
                    "compared_versions": None,
                    "diff_classification": "breaking",
                    "issues": [],
                    "violations": ["surface_missing_in_candidate"],
                    "pass": False,
                }
            )
            global_violations.append(f"{surface}:surface_missing_in_candidate")
            continue

        summary = summarize_surface_diff(surface, baseline_tree[surface], candidate_tree[surface])
        for violation in summary["violations"]:
            global_violations.append(f"{surface}:{violation}")
        surface_reports.append(summary)

    overall_pass = len(global_violations) == 0
    return {
        "schema_version": 1,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "baseline": {"source": baseline_source},
        "candidate": {"source": candidate_source},
        "overall": {
            "pass": overall_pass,
            "surfaces_total": len(surface_reports),
            "surfaces_failed": sum(0 if item["pass"] else 1 for item in surface_reports),
            "violations_total": len(global_violations),
        },
        "surfaces": surface_reports,
        "violations": sorted(global_violations),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    baseline_group = parser.add_mutually_exclusive_group(required=True)
    baseline_group.add_argument(
        "--baseline-root",
        type=Path,
        help="Path to baseline contracts root (expects contracts/<surface>/vN/*).",
    )
    baseline_group.add_argument(
        "--baseline-ref",
        type=str,
        help="Git ref for baseline contracts tree (reads contracts/** from that ref).",
    )
    parser.add_argument(
        "--candidate-root",
        type=Path,
        default=Path("contracts"),
        help="Path to candidate contracts root (default: contracts).",
    )
    parser.add_argument(
        "--report",
        type=Path,
        default=None,
        help="Optional JSON report path.",
    )
    parser.add_argument(
        "--no-enforce",
        action="store_true",
        help="Always exit with code 0 even if gate fails.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    temp_baseline_root: Path | None = None
    try:
        if args.baseline_ref:
            temp_baseline_root = materialize_contracts_from_ref(args.baseline_ref)
            baseline_root = temp_baseline_root
            baseline_source = f"git:{args.baseline_ref}"
        else:
            baseline_root = args.baseline_root
            baseline_source = str(baseline_root)

        candidate_root = args.candidate_root
        candidate_source = str(candidate_root)

        baseline_tree = load_contract_tree(baseline_root)
        candidate_tree = load_contract_tree(candidate_root)
        report = build_report(
            baseline_tree=baseline_tree,
            candidate_tree=candidate_tree,
            baseline_source=baseline_source,
            candidate_source=candidate_source,
        )

        report_json = json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True)
        if args.report:
            args.report.parent.mkdir(parents=True, exist_ok=True)
            args.report.write_text(report_json + "\n", encoding="utf-8")
            print(f"contracts_compatibility_diff_report={args.report}")
        else:
            print(report_json)

        if report["overall"]["pass"] or args.no_enforce:
            return 0
        return 1
    except CompatibilityError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    finally:
        if temp_baseline_root is not None:
            shutil.rmtree(temp_baseline_root.parent, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
