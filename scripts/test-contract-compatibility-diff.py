#!/usr/bin/env python3
"""Regression tests for scripts/check-contract-compatibility-diff.py."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class ContractCompatibilityDiffScriptTest(unittest.TestCase):
    REPO_ROOT = Path(__file__).resolve().parents[1]
    SCRIPT = REPO_ROOT / "scripts" / "check-contract-compatibility-diff.py"
    FIXTURES = REPO_ROOT / "scripts" / "fixtures" / "contracts-compatibility-diff"

    def run_case(
        self,
        name: str,
        expected_exit: int,
        expected_pass: bool,
        required_violations: list[str],
    ) -> None:
        case_root = self.FIXTURES / name
        baseline_root = case_root / "baseline" / "contracts"
        candidate_root = case_root / "candidate" / "contracts"
        self.assertTrue(baseline_root.exists(), f"missing fixture baseline: {baseline_root}")
        self.assertTrue(candidate_root.exists(), f"missing fixture candidate: {candidate_root}")

        with tempfile.TemporaryDirectory(prefix=f"compat-diff-{name}-") as tmp_dir:
            report_path = Path(tmp_dir) / "report.json"
            command = [
                sys.executable,
                str(self.SCRIPT),
                "--baseline-root",
                str(baseline_root),
                "--candidate-root",
                str(candidate_root),
                "--report",
                str(report_path),
            ]
            completed = subprocess.run(
                command,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(
                completed.returncode,
                expected_exit,
                msg=(
                    f"unexpected exit code for case {name}: stdout={completed.stdout}\n"
                    f"stderr={completed.stderr}"
                ),
            )
            self.assertTrue(report_path.exists(), f"missing report for case {name}")
            report = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertEqual(
                report["overall"]["pass"],
                expected_pass,
                f"unexpected pass flag for case {name}",
            )
            violations_flat = "\n".join(report.get("violations", []))
            for required in required_violations:
                self.assertIn(
                    required,
                    violations_flat,
                    f"missing violation {required!r} for case {name}",
                )

    def test_non_breaking_additive_same_major(self) -> None:
        self.run_case(
            name="non_breaking_additive_same_major",
            expected_exit=0,
            expected_pass=True,
            required_violations=[],
        )

    def test_non_breaking_major_bump_with_migration(self) -> None:
        self.run_case(
            name="non_breaking_major_bump_with_migration",
            expected_exit=0,
            expected_pass=True,
            required_violations=[],
        )

    def test_new_surface_without_baseline_is_non_breaking(self) -> None:
        self.run_case(
            name="non_breaking_new_surface_added",
            expected_exit=0,
            expected_pass=True,
            required_violations=[],
        )

    def test_breaking_same_major_without_bump(self) -> None:
        self.run_case(
            name="breaking_same_major_without_bump",
            expected_exit=1,
            expected_pass=False,
            required_violations=["breaking_without_major_bump"],
        )

    def test_breaking_major_bump_without_migration(self) -> None:
        self.run_case(
            name="breaking_major_bump_without_migration",
            expected_exit=1,
            expected_pass=False,
            required_violations=["missing_migration_note"],
        )

    def test_no_enforce_for_failing_case(self) -> None:
        case_root = self.FIXTURES / "breaking_same_major_without_bump"
        baseline_root = case_root / "baseline" / "contracts"
        candidate_root = case_root / "candidate" / "contracts"
        with tempfile.TemporaryDirectory(prefix="compat-diff-no-enforce-") as tmp_dir:
            report_path = Path(tmp_dir) / "report.json"
            command = [
                sys.executable,
                str(self.SCRIPT),
                "--baseline-root",
                str(baseline_root),
                "--candidate-root",
                str(candidate_root),
                "--report",
                str(report_path),
                "--no-enforce",
            ]
            completed = subprocess.run(
                command,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(
                completed.returncode,
                0,
                msg=f"--no-enforce must return zero: stderr={completed.stderr}",
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
