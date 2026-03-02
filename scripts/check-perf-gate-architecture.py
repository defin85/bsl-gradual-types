#!/usr/bin/env python3
"""Fail-closed check: perf verdict logic must stay in dedicated evaluator module."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


class GateError(Exception):
    pass


def ensure(cond: bool, message: str) -> None:
    if not cond:
        raise GateError(message)


def read_text(path: Path) -> str:
    if not path.exists():
        raise GateError(f"perf_gate_architecture_violation: missing {path}")
    return path.read_text(encoding="utf-8")


def contains_forbidden_definition(text: str, function_name: str) -> bool:
    signature = f"fn {function_name}("
    return signature in text


def contains_forbidden_reason_code_logic(text: str) -> bool:
    # Dedicated evaluator is the only place allowed to build `reason_codes`.
    return "reason_codes.insert(" in text


def list_backend_rust_files(repo_root: Path) -> list[Path]:
    backend_root = repo_root / "backend" / "src"
    return sorted(path for path in backend_root.rglob("*.rs") if path.is_file())


def list_policy_scripts(repo_root: Path) -> list[Path]:
    scripts_root = repo_root / "scripts"
    if not scripts_root.exists():
        return []
    scripts = sorted(scripts_root.glob("check-*.py"))
    run_script = scripts_root / "run-intellisense-perf.sh"
    if run_script.exists():
        scripts.append(run_script)
    return scripts


def contains_forbidden_reason_code_literals(text: str) -> bool:
    forbidden_literals = (
        "latency_relative_ratio_exceeded",
        "latency_absolute_ceiling_exceeded",
        "allocation_budget_exceeded",
        "lock_wait_budget_exceeded",
        "lock_contention_budget_exceeded",
    )
    return any(literal in text for literal in forbidden_literals)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate dedicated perf-gate evaluator boundary."
    )
    parser.add_argument(
        "--repo-root",
        default=str(Path(__file__).resolve().parents[1]),
        help="Repository root path",
    )
    args = parser.parse_args()

    repo_root = Path(args.repo_root).resolve()
    core_rs = repo_root / "backend" / "src" / "bin" / "lsp_server" / "server" / "core.rs"
    harness_rs = repo_root / "backend" / "src" / "bin" / "intellisense_perf.rs"
    evaluator_rs = repo_root / "backend" / "src" / "perf_gate_evaluator.rs"
    scanned_rs = [
        path for path in list_backend_rust_files(repo_root) if path != evaluator_rs
    ]
    allowlisted_reason_code_scripts = {
        repo_root / "scripts" / "check-versioned-contracts.py",
        repo_root / "scripts" / "check-perf-gate-architecture.py",
    }

    try:
        core_text = read_text(core_rs)
        harness_text = read_text(harness_rs)
        evaluator_text = read_text(evaluator_rs)

        ensure(
            not contains_forbidden_definition(core_text, "evaluate_scale_aware_gate"),
            "perf_gate_architecture_violation: inline evaluate_scale_aware_gate found in core.rs",
        )
        ensure(
            not contains_forbidden_definition(core_text, "validate_scale_aware_baseline_schema"),
            "perf_gate_architecture_violation: inline validate_scale_aware_baseline_schema found in core.rs",
        )
        ensure(
            not contains_forbidden_definition(core_text, "evaluate_intellisense_perf_profile"),
            "perf_gate_architecture_violation: inline evaluate_intellisense_perf_profile found in core.rs",
        )
        ensure(
            "evaluate_scale_aware_gate(" in core_text,
            (
                "perf_gate_architecture_violation: runtime acceptance path must consume "
                "evaluate_scale_aware_gate from dedicated evaluator"
            ),
        )
        ensure(
            "validate_scale_aware_baseline_schema(" in core_text,
            (
                "perf_gate_architecture_violation: runtime acceptance path must consume "
                "validate_scale_aware_baseline_schema from dedicated evaluator"
            ),
        )
        ensure(
            "evaluate_intellisense_perf_profile(" in harness_text,
            "perf_gate_architecture_violation: harness does not call dedicated evaluator",
        )
        ensure(
            "pub fn evaluate_scale_aware_gate(" in evaluator_text
            and "pub fn evaluate_intellisense_perf_profile(" in evaluator_text,
            "perf_gate_architecture_violation: dedicated evaluator API missing",
        )
        for file_path in scanned_rs:
            file_text = read_text(file_path)
            ensure(
                not contains_forbidden_reason_code_logic(file_text),
                (
                    "perf_gate_architecture_violation: reason_codes verdict logic must stay in "
                    f"dedicated evaluator module, found in {file_path}"
                ),
            )
            ensure(
                not contains_forbidden_definition(file_text, "evaluate_scale_aware_gate"),
                (
                    "perf_gate_architecture_violation: duplicate evaluate_scale_aware_gate "
                    f"definition outside evaluator ({file_path})"
                ),
            )
            ensure(
                not contains_forbidden_definition(
                    file_text, "validate_scale_aware_baseline_schema"
                ),
                (
                    "perf_gate_architecture_violation: duplicate "
                    "validate_scale_aware_baseline_schema definition outside evaluator "
                    f"({file_path})"
                ),
            )
            ensure(
                not contains_forbidden_definition(
                    file_text, "evaluate_intellisense_perf_profile"
                ),
                (
                    "perf_gate_architecture_violation: duplicate "
                    "evaluate_intellisense_perf_profile definition outside evaluator "
                    f"({file_path})"
                ),
            )

        for script_path in list_policy_scripts(repo_root):
            script_text = read_text(script_path)
            if script_path in allowlisted_reason_code_scripts:
                continue
            ensure(
                not contains_forbidden_reason_code_literals(script_text),
                (
                    "perf_gate_architecture_violation: perf verdict reason-code literals must "
                    "not appear in policy/harness scripts outside allowlist, found in "
                    f"{script_path}"
                ),
            )
    except GateError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    print("Perf gate architecture check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
