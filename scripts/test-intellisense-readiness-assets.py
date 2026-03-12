#!/usr/bin/env python3
"""Regression tests for shipped IntelliSense readiness assets."""

from __future__ import annotations

import json
import unittest
from pathlib import Path


class IntellisenseReadinessAssetsTest(unittest.TestCase):
    REPO_ROOT = Path(__file__).resolve().parents[1]
    SMOKE_SCRIPT = REPO_ROOT / "scripts" / "run-intellisense-tests.sh"
    QUALITY_GATES = (
        REPO_ROOT
        / "openspec"
        / "changes"
        / "refactor-ir-canonical-semantic-pipeline"
        / "validation"
        / "quality-gates.json"
    )

    REQUIRED_SHIPPED_SMOKE_ARTIFACTS = [
        (
            "backend/src/bin/lsp_server/server/core/tests.rs::"
            "p7_hover_and_type_at_position_revision_switch_do_not_report_stale_typed_structure_member"
        ),
        (
            "backend/src/bin/lsp_server/server/core/tests.rs::"
            "p7_definition_revision_switch_does_not_return_stale_previous_revision_location_across_lsp_and_mcp"
        ),
        (
            "backend/tests/form_module_object_unified_contract_test.rs::"
            "bare_owner_members_without_canonical_binding_stay_undeclared"
        ),
        (
            "bsl-agent/tests/stdio_integration.rs::"
            "stdio_type_at_position_revision_switch_does_not_return_stale_previous_revision_type"
        ),
        (
            "bsl-agent/tests/stdio_integration.rs::"
            "stdio_definition_revision_switch_does_not_return_stale_previous_revision_location"
        ),
        (
            "cli/src/main.rs::"
            "cli_inline_completion_preserves_object_module_binding_facets"
        ),
    ]

    def smoke_script_text(self) -> str:
        return self.SMOKE_SCRIPT.read_text(encoding="utf-8")

    def shipped_smoke_gate(self) -> dict:
        data = json.loads(self.QUALITY_GATES.read_text(encoding="utf-8"))
        for gate in data["gates"]:
            if gate["id"] == "shipped_smoke_gate":
                return gate
        self.fail("shipped_smoke_gate missing from quality-gates.json")

    def test_shipped_smoke_gate_lists_mandatory_selectors(self) -> None:
        artifacts = self.shipped_smoke_gate()["artifacts"]
        missing = [
            artifact
            for artifact in self.REQUIRED_SHIPPED_SMOKE_ARTIFACTS
            if artifact not in artifacts
        ]
        self.assertFalse(
            missing,
            f"quality-gates shipped_smoke_gate is missing mandatory artifacts: {missing}",
        )

    def test_shipped_smoke_script_executes_all_declared_smoke_selectors(self) -> None:
        smoke_text = self.smoke_script_text()
        declared_selectors = [
            artifact.split("::", 1)[1]
            for artifact in self.shipped_smoke_gate()["artifacts"]
            if "::" in artifact
        ]
        missing = [
            selector for selector in declared_selectors if selector not in smoke_text
        ]
        self.assertFalse(
            missing,
            (
                "run-intellisense-tests.sh smoke is out of sync with shipped_smoke_gate "
                f"artifacts; missing selectors: {missing}"
            ),
        )

    def test_shipped_smoke_script_covers_current_revision_and_cli_module_context_proofs(self) -> None:
        smoke_text = self.smoke_script_text()
        missing = [
            artifact.split("::", 1)[1]
            for artifact in self.REQUIRED_SHIPPED_SMOKE_ARTIFACTS
            if artifact.split("::", 1)[1] not in smoke_text
        ]
        self.assertFalse(
            missing,
            f"mandatory shipped smoke selectors are missing from run-intellisense-tests.sh: {missing}",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
