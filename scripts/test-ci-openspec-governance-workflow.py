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

    def test_active_ci_workflow_is_temporarily_manual_only(self) -> None:
        for required_snippet in (
            "workflow_dispatch:",
            "Temporary pause of automatic CI runs for this repository.",
        ):
            self.assertIn(required_snippet, self.content)
        self.assertNotIn("pull_request:", self.content)
        self.assertNotIn("\npush:\n", self.content)

    def test_active_ci_workflow_keeps_intellisense_runtime_and_evidence_jobs(
        self,
    ) -> None:
        for required_snippet in (
            "Validate IntelliSense smoke contract",
            "python3 -m unittest scripts/test-intellisense-smoke-gate.py",
            "Validate IntelliSense readiness asset sync",
            "python3 -m unittest scripts/test-intellisense-readiness-assets.py",
            "Run IntelliSense smoke gate",
            "./scripts/run-intellisense-tests.sh smoke",
            "Run IntelliSense perf gate (small|large|churn)",
            "./scripts/run-intellisense-perf.sh",
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

    def test_active_ci_workflow_fails_closed_when_governance_artifacts_are_missing(self) -> None:
        for required_snippet in (
            'if [[ ! -f "${governance_root}/change_criticality.json" ]]; then',
            'echo "Missing ${change_id}: governance artifacts are required."',
            'exit 1',
            'if [[ ! -f "${manifest}" ]]; then',
            'echo "Missing protected-assets manifest for ${change_id}."',
        ):
            self.assertIn(required_snippet, self.content)
        self.assertNotIn("Skipping ${change_id}: governance artifacts are missing.", self.content)
        self.assertNotIn("Skipping protected-assets gate for ${change_id}: manifest missing.", self.content)

    def test_active_ci_workflow_runs_agent_readiness_docs_gate(self) -> None:
        for required_snippet in (
            "agent_readiness_docs_gate:",
            "Agent readiness docs gate",
            "./scripts/run-agent-readiness-checks.sh",
            "python3 -m unittest scripts/test-agent-readiness.py",
        ):
            self.assertIn(required_snippet, self.content)


if __name__ == "__main__":
    unittest.main(verbosity=2)
