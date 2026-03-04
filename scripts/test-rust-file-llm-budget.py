#!/usr/bin/env python3
"""Regression tests for scripts/check-rust-file-llm-budget.py."""

from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class RustFileLlmBudgetScriptTest(unittest.TestCase):
    REPO_ROOT = Path(__file__).resolve().parents[1]
    SCRIPT = REPO_ROOT / "scripts" / "check-rust-file-llm-budget.py"
    _TARGET_FILES: list[str] | None = None

    @classmethod
    def target_files(cls) -> list[str]:
        if cls._TARGET_FILES is None:
            spec = importlib.util.spec_from_file_location(
                "check_rust_file_llm_budget",
                cls.SCRIPT,
            )
            assert spec and spec.loader  # pragma: no cover - defensive
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)  # type: ignore[union-attr]
            cls._TARGET_FILES = list(module.TARGET_FILES)
        return cls._TARGET_FILES

    def write_tiktoken_stub(self, repo_root: Path) -> Path:
        stub_root = repo_root / ".stub_modules"
        stub_root.mkdir(parents=True, exist_ok=True)
        (stub_root / "tiktoken.py").write_text(
            "\n".join(
                [
                    "class _Encoding:",
                    "    def encode(self, text):",
                    "        return text.split()",
                    "",
                    "def get_encoding(_name):",
                    "    return _Encoding()",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        return stub_root

    def seed_target_files(self, repo_root: Path) -> None:
        for rel in self.target_files():
            path = repo_root / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("pub fn target_fixture() {}\n", encoding="utf-8")

    def run_gate(self, repo_root: Path, expected_exit: int) -> dict[str, object]:
        stub_root = self.write_tiktoken_stub(repo_root)
        env = os.environ.copy()
        env["PYTHONPATH"] = (
            f"{stub_root}:{env['PYTHONPATH']}"
            if "PYTHONPATH" in env and env["PYTHONPATH"]
            else str(stub_root)
        )
        completed = subprocess.run(
            [
                sys.executable,
                str(self.SCRIPT),
                "--repo-root",
                str(repo_root),
                "--json",
            ],
            check=False,
            capture_output=True,
            text=True,
            env=env,
        )
        self.assertEqual(
            completed.returncode,
            expected_exit,
            msg=(
                f"unexpected exit code: stdout={completed.stdout}\n"
                f"stderr={completed.stderr}"
            ),
        )
        return json.loads(completed.stdout)

    def test_fails_when_inline_test_module_detected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="rust-llm-inline-fail-") as tmp_dir:
            repo_root = Path(tmp_dir)
            self.seed_target_files(repo_root)
            violating_file = repo_root / "backend" / "src" / "demo.rs"
            violating_file.parent.mkdir(parents=True, exist_ok=True)
            violating_file.write_text(
                "\n".join(
                    [
                        "pub fn demo() {}",
                        "",
                        "#[cfg(test)]",
                        "mod tests {",
                        "    #[test]",
                        "    fn smoke() {}",
                        "}",
                        "",
                    ]
                ),
                encoding="utf-8",
            )

            payload = self.run_gate(repo_root, expected_exit=1)
            self.assertFalse(payload["pass"])
            self.assertEqual(payload["counts"]["inline_test_module_violations"], 1)
            violation = payload["violations"]["inline_test_modules"][0]
            self.assertEqual(violation["path"], "backend/src/demo.rs")
            self.assertEqual(violation["line"], 4)

    def test_passes_when_inline_test_modules_absent(self) -> None:
        with tempfile.TemporaryDirectory(prefix="rust-llm-inline-pass-") as tmp_dir:
            repo_root = Path(tmp_dir)
            self.seed_target_files(repo_root)
            production_file = repo_root / "backend" / "src" / "demo.rs"
            production_file.parent.mkdir(parents=True, exist_ok=True)
            production_file.write_text("pub fn demo() {}\n", encoding="utf-8")

            payload = self.run_gate(repo_root, expected_exit=0)
            self.assertTrue(payload["pass"])
            self.assertEqual(payload["counts"]["inline_test_module_violations"], 0)
            self.assertEqual(payload["violations"]["inline_test_modules"], [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
