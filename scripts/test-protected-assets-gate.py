#!/usr/bin/env python3
"""Regression tests for scripts/check-protected-assets-gate.py."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class ProtectedAssetsGateScriptTest(unittest.TestCase):
    REPO_ROOT = Path(__file__).resolve().parents[1]
    SCRIPT = REPO_ROOT / "scripts" / "check-protected-assets-gate.py"
    CHANGE_ID = "test-change"

    def git(self, repo_root: Path, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", *args],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )

    def write_override(
        self,
        override_path: Path,
        *,
        approved_change_id: str | None = "approved-change-123",
        include_migration_note: bool = True,
    ) -> None:
        payload: dict[str, str] = {
            "schema_version": "v1",
            "change_id": self.CHANGE_ID,
            "reason": "approved override",
            "approved_by": "owner",
        }
        if approved_change_id is not None:
            payload["approved_change_id"] = approved_change_id
        if include_migration_note:
            payload["migration_note"] = "migration note reference"
        override_path.write_text(
            json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

    def seed_repo(self, repo_root: Path) -> tuple[Path, Path]:
        self.git(repo_root, "init")
        self.git(repo_root, "config", "user.email", "test@example.com")
        self.git(repo_root, "config", "user.name", "Test User")

        manifest = repo_root / "governance" / "manifest.txt"
        override = repo_root / "governance" / "override.json"
        protected_file = repo_root / "protected" / "perf.txt"
        non_protected = repo_root / "notes.txt"

        manifest.parent.mkdir(parents=True, exist_ok=True)
        protected_file.parent.mkdir(parents=True, exist_ok=True)
        manifest.write_text("protected/**\n", encoding="utf-8")
        protected_file.write_text("v1\n", encoding="utf-8")
        non_protected.write_text("note v1\n", encoding="utf-8")
        self.write_override(override)

        self.git(repo_root, "add", ".")
        self.git(repo_root, "commit", "-m", "initial")
        return manifest, override

    def run_gate(
        self, repo_root: Path, manifest: Path, override: Path, expected_exit: int
    ) -> subprocess.CompletedProcess[str]:
        completed = subprocess.run(
            [
                sys.executable,
                str(self.SCRIPT),
                "--repo-root",
                str(repo_root),
                "--change-id",
                self.CHANGE_ID,
                "--manifest",
                str(manifest.relative_to(repo_root)),
                "--override",
                str(override.relative_to(repo_root)),
                "--base-ref",
                "HEAD~1",
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

    def test_passes_when_no_protected_assets_changed(self) -> None:
        with tempfile.TemporaryDirectory(prefix="protected-no-change-") as tmp_dir:
            repo_root = Path(tmp_dir)
            manifest, override = self.seed_repo(repo_root)
            (repo_root / "notes.txt").write_text("note v2\n", encoding="utf-8")
            self.git(repo_root, "add", "notes.txt")
            self.git(repo_root, "commit", "-m", "non-protected change")
            completed = self.run_gate(repo_root, manifest, override, expected_exit=0)
            self.assertIn("no protected paths changed", completed.stdout)

    def test_fails_when_protected_assets_changed_without_override(self) -> None:
        with tempfile.TemporaryDirectory(prefix="protected-no-override-") as tmp_dir:
            repo_root = Path(tmp_dir)
            manifest, override = self.seed_repo(repo_root)
            override.unlink()
            (repo_root / "protected" / "perf.txt").write_text("v2\n", encoding="utf-8")
            self.git(repo_root, "add", "protected/perf.txt")
            self.git(repo_root, "commit", "-m", "protected change")
            completed = self.run_gate(repo_root, manifest, override, expected_exit=1)
            self.assertIn("protected_acceptance_asset_modified", completed.stderr)

    def test_fails_when_override_missing_approved_change_id(self) -> None:
        with tempfile.TemporaryDirectory(prefix="protected-missing-approved-change-") as tmp_dir:
            repo_root = Path(tmp_dir)
            manifest, override = self.seed_repo(repo_root)
            self.write_override(override, approved_change_id=None)
            (repo_root / "protected" / "perf.txt").write_text("v2\n", encoding="utf-8")
            self.git(repo_root, "add", ".")
            self.git(repo_root, "commit", "-m", "protected change")
            completed = self.run_gate(repo_root, manifest, override, expected_exit=1)
            self.assertIn("approved_change_id is required", completed.stderr)

    def test_fails_when_approved_change_id_matches_current_change(self) -> None:
        with tempfile.TemporaryDirectory(prefix="protected-same-approved-change-") as tmp_dir:
            repo_root = Path(tmp_dir)
            manifest, override = self.seed_repo(repo_root)
            self.write_override(override, approved_change_id=self.CHANGE_ID)
            (repo_root / "protected" / "perf.txt").write_text("v2\n", encoding="utf-8")
            self.git(repo_root, "add", ".")
            self.git(repo_root, "commit", "-m", "protected change")
            completed = self.run_gate(repo_root, manifest, override, expected_exit=1)
            self.assertIn("separate approved change", completed.stderr)

    def test_passes_with_valid_override_for_protected_changes(self) -> None:
        with tempfile.TemporaryDirectory(prefix="protected-valid-override-") as tmp_dir:
            repo_root = Path(tmp_dir)
            manifest, override = self.seed_repo(repo_root)
            self.write_override(override, approved_change_id="approved-separate-change")
            (repo_root / "protected" / "perf.txt").write_text("v2\n", encoding="utf-8")
            self.git(repo_root, "add", ".")
            self.git(repo_root, "commit", "-m", "protected change")
            completed = self.run_gate(repo_root, manifest, override, expected_exit=0)
            self.assertIn("Protected assets changed with approved override", completed.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
