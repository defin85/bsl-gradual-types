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


def main() -> int:
    repo_root = Path(__file__).resolve().parents[1]
    core_rs = repo_root / "backend" / "src" / "bin" / "lsp_server" / "server" / "core.rs"
    harness_rs = repo_root / "backend" / "src" / "bin" / "intellisense_perf.rs"
    evaluator_rs = repo_root / "backend" / "src" / "perf_gate_evaluator.rs"

    try:
        core_text = read_text(core_rs)
        harness_text = read_text(harness_rs)
        evaluator_text = read_text(evaluator_rs)

        ensure(
            "fn evaluate_scale_aware_gate(" not in core_text,
            "perf_gate_architecture_violation: inline evaluate_scale_aware_gate found in core.rs",
        )
        ensure(
            "fn validate_scale_aware_baseline_schema(" not in core_text,
            "perf_gate_architecture_violation: inline validate_scale_aware_baseline_schema found in core.rs",
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
    except GateError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    print("Perf gate architecture check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
