#!/usr/bin/env python3
"""Regression tests for scripts/check-perf-gate-architecture.py."""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class PerfGateArchitectureScriptTest(unittest.TestCase):
    REPO_ROOT = Path(__file__).resolve().parents[1]
    SCRIPT = REPO_ROOT / "scripts" / "check-perf-gate-architecture.py"

    def seed_valid_repo(self, repo_root: Path) -> None:
        core = repo_root / "backend" / "src" / "bin" / "lsp_server" / "server" / "core.rs"
        harness = repo_root / "backend" / "src" / "bin" / "intellisense_perf.rs"
        evaluator = repo_root / "backend" / "src" / "perf_gate_evaluator.rs"
        scripts_dir = repo_root / "scripts"

        core.parent.mkdir(parents=True, exist_ok=True)
        harness.parent.mkdir(parents=True, exist_ok=True)
        evaluator.parent.mkdir(parents=True, exist_ok=True)
        scripts_dir.mkdir(parents=True, exist_ok=True)

        core.write_text(
            "\n".join(
                [
                    "pub fn core_entry() {",
                    "    validate_scale_aware_baseline_schema();",
                    "    evaluate_scale_aware_gate();",
                    "}",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        harness.write_text(
            "fn run() { evaluate_intellisense_perf_profile(); }\n",
            encoding="utf-8",
        )
        evaluator.write_text(
            "\n".join(
                [
                    "pub fn evaluate_scale_aware_gate() {}",
                    "pub fn evaluate_intellisense_perf_profile() {}",
                    "fn build_reason_codes() {",
                    "    let mut reason_codes = std::collections::BTreeSet::new();",
                    '    reason_codes.insert("latency_relative_ratio_exceeded".to_string());',
                    "}",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        # Allowlisted checker may contain perf reason-code literals.
        (scripts_dir / "check-versioned-contracts.py").write_text(
            'REASONS = ["latency_relative_ratio_exceeded"]\n',
            encoding="utf-8",
        )
        (scripts_dir / "check-safe.py").write_text(
            "def run() -> None:\n    return None\n",
            encoding="utf-8",
        )
        (scripts_dir / "run-intellisense-perf.sh").write_text(
            "#!/usr/bin/env bash\necho run\n",
            encoding="utf-8",
        )

    def run_check(self, repo_root: Path, expected_exit: int) -> subprocess.CompletedProcess[str]:
        completed = subprocess.run(
            [
                sys.executable,
                str(self.SCRIPT),
                "--repo-root",
                str(repo_root),
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(
            completed.returncode,
            expected_exit,
            msg=(
                f"unexpected exit code: stdout={completed.stdout}\n"
                f"stderr={completed.stderr}"
            ),
        )
        return completed

    def test_passes_for_valid_layout(self) -> None:
        with tempfile.TemporaryDirectory(prefix="perf-arch-pass-") as tmp_dir:
            repo_root = Path(tmp_dir)
            self.seed_valid_repo(repo_root)
            completed = self.run_check(repo_root, expected_exit=0)
            self.assertIn("Perf gate architecture check passed.", completed.stdout)

    def test_fails_when_reason_codes_insert_is_outside_evaluator(self) -> None:
        with tempfile.TemporaryDirectory(prefix="perf-arch-reason-codes-") as tmp_dir:
            repo_root = Path(tmp_dir)
            self.seed_valid_repo(repo_root)
            extra = repo_root / "backend" / "src" / "bin" / "lsp_server" / "server" / "extra.rs"
            extra.write_text(
                '\n'.join(
                    [
                        "fn bad() {",
                        "    let mut reason_codes = std::collections::BTreeSet::new();",
                        '    reason_codes.insert("latency_relative_ratio_exceeded".to_string());',
                        "}",
                        "",
                    ]
                ),
                encoding="utf-8",
            )
            completed = self.run_check(repo_root, expected_exit=1)
            self.assertIn("perf_gate_architecture_violation", completed.stderr)

    def test_fails_when_duplicate_gate_function_defined_outside_evaluator(self) -> None:
        with tempfile.TemporaryDirectory(prefix="perf-arch-duplicate-fn-") as tmp_dir:
            repo_root = Path(tmp_dir)
            self.seed_valid_repo(repo_root)
            duplicate = (
                repo_root / "backend" / "src" / "bin" / "lsp_server" / "server" / "dup.rs"
            )
            duplicate.write_text(
                "fn evaluate_scale_aware_gate() {}\n",
                encoding="utf-8",
            )
            completed = self.run_check(repo_root, expected_exit=1)
            self.assertIn("perf_gate_architecture_violation", completed.stderr)

    def test_fails_when_non_allowlisted_script_contains_reason_code_literals(self) -> None:
        with tempfile.TemporaryDirectory(prefix="perf-arch-script-literals-") as tmp_dir:
            repo_root = Path(tmp_dir)
            self.seed_valid_repo(repo_root)
            (repo_root / "scripts" / "check-inline-verdict.py").write_text(
                'REASON = "latency_relative_ratio_exceeded"\n',
                encoding="utf-8",
            )
            completed = self.run_check(repo_root, expected_exit=1)
            self.assertIn("perf_gate_architecture_violation", completed.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
