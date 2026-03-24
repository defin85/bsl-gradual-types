#!/usr/bin/env python3
"""Regression tests for canonical agent-readiness validation assets."""

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path


class AgentReadinessValidationTest(unittest.TestCase):
    REPO_ROOT = Path(__file__).resolve().parents[1]
    CHECK_SCRIPT = REPO_ROOT / "scripts" / "check-agent-readiness.py"
    WRAPPER_SCRIPT = REPO_ROOT / "scripts" / "run-agent-readiness-checks.sh"
    DOCUMENT_SYMBOL_WRAPPER = (
        REPO_ROOT / "scripts" / "validate-document-symbol-interactive-isolation.sh"
    )
    TARGETS_FILE = REPO_ROOT / "scripts" / "doc-path-check-targets.txt"
    VERIFICATION_DOC = REPO_ROOT / "docs" / "agent" / "verification.md"

    def test_required_targets_cover_primary_onboarding_and_agent_docs(self) -> None:
        content = self.TARGETS_FILE.read_text(encoding="utf-8")
        for required_entry in (
            "AGENTS.md",
            "README.md",
            "CONTRIBUTING.md",
            ".github/copilot-instructions.md",
            "backend/AGENTS.md",
            "bsl-agent/AGENTS.md",
            "bsl-agent/README.md",
            "vscode-extension/AGENTS.md",
            "docs/README.md",
            "docs/BUILD_GUIDE.md",
            "docs/guides/development-workflow.md",
            "docs/agent/*.md",
        ):
            self.assertIn(required_entry, content)

    def test_verification_doc_exposes_local_agent_readiness_command(self) -> None:
        content = self.VERIFICATION_DOC.read_text(encoding="utf-8")
        self.assertIn("./scripts/run-agent-readiness-checks.sh", content)

    def test_verification_doc_exposes_document_symbol_readiness_wrapper(self) -> None:
        content = self.VERIFICATION_DOC.read_text(encoding="utf-8")
        self.assertIn(
            "./scripts/validate-document-symbol-interactive-isolation.sh",
            content,
        )

    def test_document_symbol_wrapper_pins_change_id(self) -> None:
        content = self.DOCUMENT_SYMBOL_WRAPPER.read_text(encoding="utf-8")
        self.assertIn(
            "CHANGE_ID=refactor-document-symbol-interactive-isolation",
            content,
        )

    def test_wrapper_runs_all_agent_readiness_checks(self) -> None:
        content = self.WRAPPER_SCRIPT.read_text(encoding="utf-8")
        self.assertIn("check-doc-paths.py", content)
        self.assertIn("check-agent-readiness.py", content)

    def test_agent_readiness_checker_passes_for_repository_state(self) -> None:
        result = subprocess.run(
            [sys.executable, str(self.CHECK_SCRIPT)],
            cwd=self.REPO_ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr or result.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
