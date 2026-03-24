#!/usr/bin/env python3
"""Regression tests for the active OpenSpec governance workflow wiring."""

from __future__ import annotations

import unittest
from pathlib import Path


class OpenSpecGovernanceWorkflowTest(unittest.TestCase):
    REPO_ROOT = Path(__file__).resolve().parents[1]
    WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"

    @classmethod
    def setUpClass(cls) -> None:
        cls.content = cls.WORKFLOW.read_text(encoding="utf-8")

    def test_active_ci_workflow_exists(self) -> None:
        self.assertTrue(self.WORKFLOW.exists(), "active ci workflow must exist")

    def test_active_ci_workflow_keeps_governance_gate_as_default_job(self) -> None:
        self.assertIn("name: CI", self.content)
        self.assertIn("openspec_governance_gate:", self.content)
        self.assertIn(
            "OpenSpec governance gate (default readiness path)",
            self.content,
        )

    def test_active_ci_workflow_triggers_for_default_operational_paths(self) -> None:
        for required_snippet in (
            "workflow_dispatch:",
            "pull_request:",
            "push:",
            'branches:\n      - master',
            '      - "openspec/changes/**"',
            '      - ".github/workflows/ci.yml"',
        ):
            self.assertIn(required_snippet, self.content)

    def test_active_ci_workflow_triggers_for_intellisense_runtime_and_evidence_surfaces(
        self,
    ) -> None:
        for required_snippet in (
            '      - "backend/src/bin/lsp_server/server/**"',
            '      - "bsl-runtime/src/system/basic_observability/**"',
            '      - "backend/tests/perf/reports/**"',
            '      - "scripts/validate-v2-completion-gates.sh"',
            '      - "scripts/README.md"',
            '      - "scripts/run-intellisense-tests.sh"',
        ):
            self.assertIn(required_snippet, self.content)

    def test_active_ci_workflow_runs_governance_scripts_for_touched_changes(self) -> None:
        for required_snippet in (
            'sed -n \'s#^openspec/changes/\\([^/]*\\)/.*#\\1#p\'',
            "python3 scripts/check-openspec-change-governance.py",
            "python3 scripts/check-protected-assets-gate.py",
            '--change-id "${change_id}"',
            '--base-ref "${{ steps.protected_base_ref.outputs.value }}"',
        ):
            self.assertIn(required_snippet, self.content)

    def test_active_ci_workflow_runs_agent_readiness_docs_gate(self) -> None:
        for required_snippet in (
            "agent_readiness_docs_gate:",
            "Agent readiness docs gate",
            "./scripts/run-agent-readiness-checks.sh",
            "python3 -m unittest scripts/test-agent-readiness.py",
            '      - "docs/agent/**"',
            '      - "scripts/run-agent-readiness-checks.sh"',
        ):
            self.assertIn(required_snippet, self.content)


if __name__ == "__main__":
    unittest.main(verbosity=2)
