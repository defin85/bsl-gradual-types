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
        "backend/src/bin/lsp_server/server/core/tests.rs::p33_document_symbol_returns_unavailable_before_ready_outline_from_did_open_gap": "p33_document_symbol_returns_unavailable_before_ready_outline_from_did_open_gap",
        "backend/src/bin/lsp_server/server/core/tests.rs::p33_document_symbol_returns_latest_ready_from_cache_during_parse_gap": "p33_document_symbol_returns_latest_ready_from_cache_during_parse_gap",
        "backend/src/bin/lsp_server/server/core/tests.rs::p33_document_symbol_supersedes_older_outstanding_refresh": "p33_document_symbol_supersedes_older_outstanding_refresh",
        "backend/src/bin/lsp_server/server/core/tests.rs::p33_document_symbol_burst_does_not_delay_completion_first_poll_under_parse_gap": "p33_document_symbol_burst_does_not_delay_completion_first_poll_under_parse_gap",
        "backend/src/bin/lsp_server/server/core/tests.rs::p33_document_symbol_burst_does_not_delay_hover_signature_help_or_definition_under_parse_gap": "p33_document_symbol_burst_does_not_delay_hover_signature_help_or_definition_under_parse_gap",
        "backend/src/bin/lsp_server/server/core/tests.rs::p33_did_save_rearms_same_version_outline_refresh_on_default_path": "p33_did_save_rearms_same_version_outline_refresh_on_default_path",
        "backend/src/bin/lsp_server/server/core/tests.rs::p33_same_file_completion_supersession_releases_pre_active_turn_wait_before_active_registration": "p33_same_file_completion_supersession_releases_pre_active_turn_wait_before_active_registration",
        "backend/src/bin/lsp_server/server/core/tests.rs::p28_cancel_request_releases_pre_active_turn_wait_before_active_registration": "p28_cancel_request_releases_pre_active_turn_wait_before_active_registration",
        "backend/src/bin/lsp_server/server/core/tests.rs::p33_same_file_completion_supersession_releases_active_turn_at_format_checkpoint": "p33_same_file_completion_supersession_releases_active_turn_at_format_checkpoint",
        "backend/src/bin/lsp_server/server/core/tests.rs::p33_same_version_exact_wait_keeps_completed_task_observable_until_cleanup": "p33_same_version_exact_wait_keeps_completed_task_observable_until_cleanup",
        "backend/src/bin/lsp_server/server/core/tests.rs::p33_shutdown_cleans_retained_same_version_exact_task_entry": "p33_shutdown_cleans_retained_same_version_exact_task_entry",
        "backend/src/bin/lsp_server/server/core/tests.rs::p33_same_version_invoked_completion_keeps_completed_task_visible_on_default_path": "p33_same_version_invoked_completion_keeps_completed_task_visible_on_default_path",
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

    def test_smoke_script_builds_embedded_ui_assets_when_missing(self) -> None:
        content = self.SMOKE_SCRIPT.read_text(encoding="utf-8")
        self.assertIn(
            "target/site/index.html",
            content,
            "default smoke path must check for embedded bsl-agent UI assets",
        )
        self.assertIn(
            "trunk build --release",
            content,
            "default smoke path must rebuild embedded UI assets when target/site is absent",
        )

    def test_smoke_script_executes_each_exact_selector_individually(self) -> None:
        content = self.SMOKE_SCRIPT.read_text(encoding="utf-8")

        self.assertIn(
            'for selector in "${selectors[@]}"; do',
            content,
            (
                "smoke gate exact-selector helper must iterate selectors one by one so "
                "cargo exact filters do not degrade into zero-test no-op bundles"
            ),
        )
        self.assertIn(
            'mapfile -t available_tests < <(cargo test "${cargo_args[@]}" -- --list | sed -n \'s/: test$//p\')',
            content,
            (
                "smoke gate exact-selector helper must resolve short selectors against "
                "cargo --list output before running exact tests"
            ),
        )
        self.assertNotIn(
            'cargo test "${cargo_args[@]}" -- --exact "${selectors[@]}" --nocapture',
            content,
            (
                "smoke gate exact-selector helper must not batch multiple selectors into "
                "a single cargo exact invocation"
            ),
        )
        self.assertIn(
            'resolved_selector="${matching_tests[0]}"',
            content,
            "smoke gate exact-selector helper must fail-closed to one resolved full test name",
        )
        self.assertIn(
            'cargo test "${cargo_args[@]}" "${resolved_selector}" -- --exact --nocapture',
            content,
            (
                "smoke gate exact-selector helper must pass the resolved full test name "
                "to cargo before `-- --exact`"
            ),
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
