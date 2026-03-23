#!/usr/bin/env python3
"""Regression tests for the shipped IntelliSense smoke gate contract."""

from __future__ import annotations

import json
import unittest
from pathlib import Path


class IntellisenseSmokeGateContractTest(unittest.TestCase):
    REPO_ROOT = Path(__file__).resolve().parents[1]
    SMOKE_SCRIPT = REPO_ROOT / "scripts" / "run-intellisense-tests.sh"
    QUALITY_GATES = (
        REPO_ROOT
        / "openspec"
        / "changes"
        / "archive"
        / "2026-03-13-refactor-ir-canonical-semantic-pipeline"
        / "validation"
        / "quality-gates.json"
    )
    REQUIRED_SMOKE_SELECTORS = {
        "backend/src/bin/lsp_server/server/core/tests.rs::p7_hover_and_type_at_position_revision_switch_do_not_report_stale_typed_structure_member": "p7_hover_and_type_at_position_revision_switch_do_not_report_stale_typed_structure_member",
        "backend/src/bin/lsp_server/server/core/tests.rs::p7_definition_revision_switch_does_not_return_stale_previous_revision_location_across_lsp_and_mcp": "p7_definition_revision_switch_does_not_return_stale_previous_revision_location_across_lsp_and_mcp",
        "bsl-agent/tests/stdio_integration.rs::stdio_type_at_position_revision_switch_does_not_return_stale_previous_revision_type": "stdio_type_at_position_revision_switch_does_not_return_stale_previous_revision_type",
        "bsl-agent/tests/stdio_integration.rs::stdio_definition_revision_switch_does_not_return_stale_previous_revision_location": "stdio_definition_revision_switch_does_not_return_stale_previous_revision_location",
        "backend/tests/form_module_object_unified_contract_test.rs::bare_owner_members_without_canonical_binding_stay_undeclared": "bare_owner_members_without_canonical_binding_stay_undeclared",
        "cli/src/main.rs::cli_inline_completion_preserves_object_module_binding_facets": "cli_inline_completion_preserves_object_module_binding_facets",
    }

    def test_smoke_script_runs_mandatory_current_revision_and_bare_fallback_regressions(
        self,
    ) -> None:
        content = self.SMOKE_SCRIPT.read_text(encoding="utf-8")

        missing = [
            selector
            for selector in self.REQUIRED_SMOKE_SELECTORS.values()
            if selector not in content
        ]

        self.assertFalse(
            missing,
            (
                "default shipped smoke must execute all mandatory current-revision and "
                f"bare-fallback regression selectors, missing={missing}"
            ),
        )

    def test_quality_gate_artifacts_match_required_shipped_smoke_evidence(self) -> None:
        payload = json.loads(self.QUALITY_GATES.read_text(encoding="utf-8"))
        shipped_gate = next(
            gate for gate in payload["gates"] if gate.get("id") == "shipped_smoke_gate"
        )

        self.assertEqual(
            shipped_gate.get("commands"),
            ["./scripts/run-intellisense-tests.sh smoke"],
        )

        artifacts = set(shipped_gate.get("artifacts", []))
        missing = sorted(
            artifact
            for artifact in self.REQUIRED_SMOKE_SELECTORS
            if artifact not in artifacts
        )

        self.assertFalse(
            missing,
            (
                "quality-gates shipped_smoke_gate must enumerate the same mandatory shipped "
                f"smoke evidence, missing={missing}"
            ),
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
