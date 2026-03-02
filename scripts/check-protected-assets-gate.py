#!/usr/bin/env python3
"""Fail-closed protected-assets gate for a change."""

from __future__ import annotations

import argparse
import fnmatch
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


class GateError(Exception):
    pass


def ensure(condition: bool, message: str) -> None:
    if not condition:
        raise GateError(message)


def git_changed_files(repo_root: Path, base_ref: str) -> list[str]:
    def run(*args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", *args],
            cwd=repo_root,
            check=False,
            capture_output=True,
            text=True,
        )

    primary = run("diff", "--name-only", f"{base_ref}...HEAD")
    if primary.returncode == 0:
        return [line.strip() for line in primary.stdout.splitlines() if line.strip()]

    fallback = run("diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD")
    if fallback.returncode == 0:
        return [line.strip() for line in fallback.stdout.splitlines() if line.strip()]

    raise GateError(
        "protected_acceptance_asset_modified: unable to get changed files from git"
    )


def load_manifest(path: Path) -> list[str]:
    ensure(path.exists(), f"protected_acceptance_asset_modified: missing {path}")
    patterns: list[str] = []
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        patterns.append(line)
    ensure(patterns, f"protected_acceptance_asset_modified: empty manifest {path}")
    return patterns


def matches_any(path: str, patterns: list[str]) -> bool:
    return any(fnmatch.fnmatch(path, pattern) for pattern in patterns)


def load_override(path: Path, expected_change_id: str) -> None:
    ensure(
        path.exists(),
        "protected_acceptance_asset_modified: protected assets changed without override",
    )
    try:
        payload: Any = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise GateError(
            f"protected_acceptance_asset_modified: invalid override JSON {path}: {exc}"
        ) from exc

    ensure(
        isinstance(payload, dict),
        f"protected_acceptance_asset_modified: override root must be object in {path}",
    )
    ensure(
        payload.get("schema_version") == "v1",
        f"protected_acceptance_asset_modified: {path} schema_version must be 'v1'",
    )
    ensure(
        payload.get("change_id") == expected_change_id,
        f"protected_acceptance_asset_modified: {path} change_id mismatch",
    )
    ensure(
        isinstance(payload.get("reason"), str) and payload["reason"].strip(),
        f"protected_acceptance_asset_modified: {path} reason is required",
    )
    ensure(
        isinstance(payload.get("approved_by"), str) and payload["approved_by"].strip(),
        f"protected_acceptance_asset_modified: {path} approved_by is required",
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Check protected-assets gate.")
    parser.add_argument("--repo-root", default=str(Path(__file__).resolve().parents[1]))
    parser.add_argument("--change-id", required=True)
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--override", required=True)
    parser.add_argument("--base-ref", default="HEAD~1")
    args = parser.parse_args()

    repo_root = Path(args.repo_root).resolve()
    manifest_path = (repo_root / args.manifest).resolve()
    override_path = (repo_root / args.override).resolve()

    try:
        patterns = load_manifest(manifest_path)
        changed_files = git_changed_files(repo_root, args.base_ref)
        protected_touched = [
            path for path in changed_files if matches_any(path, patterns)
        ]

        if protected_touched:
            load_override(override_path, args.change_id)
            print(
                "Protected assets changed with approved override:",
                ", ".join(sorted(protected_touched)),
            )
        else:
            print("Protected-assets gate passed (no protected paths changed).")
    except GateError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

