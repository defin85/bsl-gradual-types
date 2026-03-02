#!/usr/bin/env python3
"""Fail-closed check: perf verdict logic must stay in dedicated evaluator module."""

from __future__ import annotations

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


def main() -> int:
    repo_root = Path(__file__).resolve().parents[1]
    core_rs = repo_root / "backend" / "src" / "bin" / "lsp_server" / "server" / "core.rs"
    harness_rs = repo_root / "backend" / "src" / "bin" / "intellisense_perf.rs"
    evaluator_rs = repo_root / "backend" / "src" / "perf_gate_evaluator.rs"
    scanned_rs = [
        core_rs,
        harness_rs,
    ]

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
    except GateError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    print("Perf gate architecture check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
