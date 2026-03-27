#!/usr/bin/env python3
"""Regression tests for shipped IntelliSense readiness assets."""

from __future__ import annotations

import json
import re
import subprocess
import unittest
from pathlib import Path


class IntellisenseReadinessAssetsTest(unittest.TestCase):
    REPO_ROOT = Path(__file__).resolve().parents[1]
    SMOKE_SCRIPT = REPO_ROOT / "scripts" / "run-intellisense-tests.sh"
    CI_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"
    DEVELOPMENT_WORKFLOW_GUIDE = (
        REPO_ROOT / "docs" / "guides" / "development-workflow.md"
    )
    DOCUMENT_SYMBOL_CHANGE_ID = "refactor-document-symbol-interactive-isolation"
    OVERLAP_CHANGE_ID = "refactor-completion-superseded-active-turn-release"
    TURN_WAIT_CHANGE_ID = "refactor-completion-turn-wait-lifecycle"
    SLOT_RELEASE_CHANGE_ID = "refactor-completion-turn-wait-slot-release"
    QUALITY_GATES = (
        REPO_ROOT
        / "openspec"
        / "changes"
        / "archive"
        / "2026-03-13-refactor-ir-canonical-semantic-pipeline"
        / "validation"
        / "quality-gates.json"
    )
    DOCUMENT_SYMBOL_MIXED_LOAD_DOC = (
        REPO_ROOT
        / "openspec"
        / "changes"
        / DOCUMENT_SYMBOL_CHANGE_ID
        / "validation"
        / "mixed-load-gate.md"
    )
    DOCUMENT_SYMBOL_MIXED_LOAD_REPORT = (
        REPO_ROOT
        / "backend"
        / "tests"
        / "perf"
        / "reports"
        / f"{DOCUMENT_SYMBOL_CHANGE_ID}-real-conf-big-document-symbol-mixed-load-live.json"
    )
    OVERLAP_GATE_DOC = (
        REPO_ROOT
        / "openspec"
        / "changes"
        / OVERLAP_CHANGE_ID
        / "validation"
        / "overlap-gate.md"
    )
    OVERLAP_GATE_REPORT = (
        REPO_ROOT
        / "backend"
        / "tests"
        / "perf"
        / "reports"
        / f"{OVERLAP_CHANGE_ID}-real-conf-big-overlap-completion-perf-live.json"
    )
    TURN_WAIT_GATE_DOC = (
        REPO_ROOT
        / "openspec"
        / "changes"
        / TURN_WAIT_CHANGE_ID
        / "validation"
        / "pre-active-overlap-gate.md"
    )
    TURN_WAIT_GATE_REPORT = (
        REPO_ROOT
        / "backend"
        / "tests"
        / "perf"
        / "reports"
        / f"{TURN_WAIT_CHANGE_ID}-real-conf-big-pre-active-overlap-completion-perf-live.json"
    )
    SLOT_RELEASE_ARCHITECTURAL_REVIEW = (
        REPO_ROOT
        / "openspec"
        / "changes"
        / SLOT_RELEASE_CHANGE_ID
        / "validation"
        / "architectural-review.md"
    )
    SLOT_RELEASE_TRACEABILITY_DOC = (
        REPO_ROOT
        / "openspec"
        / "changes"
        / SLOT_RELEASE_CHANGE_ID
        / "validation"
        / "traceability.md"
    )
    SLOT_RELEASE_GATE_DOC = (
        REPO_ROOT
        / "openspec"
        / "changes"
        / SLOT_RELEASE_CHANGE_ID
        / "validation"
        / "pre-active-overlap-gate.md"
    )
    SLOT_RELEASE_CHURN_REPORT = (
        REPO_ROOT
        / "backend"
        / "tests"
        / "perf"
        / "reports"
        / f"{SLOT_RELEASE_CHANGE_ID}-real-conf-big-revision-churn-completion-perf-live.json"
    )
    SLOT_RELEASE_CHURN_SUMMARY = (
        REPO_ROOT
        / "backend"
        / "tests"
        / "perf"
        / "reports"
        / f"{SLOT_RELEASE_CHANGE_ID}-real-conf-big-revision-churn-completion-perf-live.md"
    )
    SLOT_RELEASE_GATE_REPORT = (
        REPO_ROOT
        / "backend"
        / "tests"
        / "perf"
        / "reports"
        / f"{SLOT_RELEASE_CHANGE_ID}-real-conf-big-pre-active-overlap-completion-perf-live.json"
    )
    SLOT_RELEASE_GATE_SUMMARY = (
        REPO_ROOT
        / "backend"
        / "tests"
        / "perf"
        / "reports"
        / f"{SLOT_RELEASE_CHANGE_ID}-real-conf-big-pre-active-overlap-completion-perf-live.md"
    )
    SLOT_RELEASE_READINESS_GATE_JSON = (
        REPO_ROOT
        / "backend"
        / "tests"
        / "perf"
        / "reports"
        / f"{SLOT_RELEASE_CHANGE_ID}-readiness-gate.json"
    )
    SLOT_RELEASE_READINESS_GATE_MD = (
        REPO_ROOT
        / "backend"
        / "tests"
        / "perf"
        / "reports"
        / f"{SLOT_RELEASE_CHANGE_ID}-readiness-gate.md"
    )
    SLOT_RELEASE_OPENSPEC_LOG = (
        REPO_ROOT
        / "backend"
        / "tests"
        / "perf"
        / "reports"
        / f"{SLOT_RELEASE_CHANGE_ID}-openspec-validate.log"
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
            "backend/src/bin/lsp_server/server/core/tests.rs::"
            "p33_document_symbol_returns_unavailable_before_ready_outline_from_did_open_gap"
        ),
        (
            "backend/src/bin/lsp_server/server/core/tests.rs::"
            "p33_document_symbol_returns_latest_ready_from_cache_during_parse_gap"
        ),
        (
            "backend/src/bin/lsp_server/server/core/tests.rs::"
            "p33_document_symbol_supersedes_older_outstanding_refresh"
        ),
        (
            "backend/src/bin/lsp_server/server/core/tests.rs::"
            "p33_document_symbol_burst_does_not_delay_completion_first_poll_under_parse_gap"
        ),
        (
            "backend/src/bin/lsp_server/server/core/tests.rs::"
            "p33_document_symbol_burst_does_not_delay_hover_signature_help_or_definition_under_parse_gap"
        ),
        (
            "backend/src/bin/lsp_server/server/core/tests.rs::"
            "p33_did_save_rearms_same_version_outline_refresh_on_default_path"
        ),
        (
            "backend/src/bin/lsp_server/server/core/tests.rs::"
            "p33_same_file_completion_supersession_releases_pre_active_turn_wait_before_active_registration"
        ),
        (
            "backend/src/bin/lsp_server/server/core/tests.rs::"
            "p28_cancel_request_releases_pre_active_turn_wait_before_active_registration"
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
    REQUIRED_EXTENSION_SMOKE_SNIPPETS = [
        'npm --prefix "${ROOT_DIR}/vscode-extension" run compile:fast',
        "BSL_TEST_GREP=",
        "Completion Probe (Schema|Recorder|Runtime|Store) Test Suite",
        "Completion Timeline (Clipboard|Drilldown|Model|Webview Provider) Test Suite",
        "Client Options Test Suite",
        "Observability Incident Bundle Test Suite",
        "Observability Commands Test Suite",
        "getCompletionTimeline should work via executeCommand",
        "getCompletionTimeline should fail-closed on Method not found",
        "getObservabilityMetricsFetchResult should preserve unsupported capability until reset",
        "getObservabilityMetricsFetchResult should return unavailable error on timeout",
    ]
    REQUIRED_COMPLETION_TIMELINE_DRILLDOWN_SELECTORS = [
        "wait_for_file_version_runtime_trace_distinguishes_immediate_and_waiter_paths",
        "snapshot_with_deps_runtime_trace_exposes_queue_and_exec_latency",
        "interactive_wait_budget_timeout_can_still_report_timeout_attribution_on_success",
        "snapshot_with_deps_timeout_can_report_queue_wait_runtime_split_via_progress",
        "turn_waiter_preserves_non_zero_absolute_lifecycle_after_observed_wait",
        "turn_attribution_trace_preserves_turn_wait_resolution_timestamps",
        "p22_get_completion_timeline_exposes_versioned_contract",
        "p22_get_completion_timeline_contains_completion_trace",
        "dispatch_context_service_records_completion_context_for_position_lookup",
        "server_edge_details_are_derived_from_transport_handler_and_response_timestamps",
        "server_edge_details_include_pre_method_attribution_provenance",
        "server_edge_details_use_outer_dispatch_timestamp_as_transport_anchor_when_available",
        "request_context_service_records_first_poll_and_first_wake_for_pending_future",
        "request_context_service_does_not_fabricate_first_wake_for_ready_first_poll",
        "server_edge_details_derive_first_poll_and_first_wake_split_when_present",
        "server_edge_details_do_not_fabricate_first_wake_split_when_first_poll_is_ready",
        "prepare_runtime_drilldown_is_serialised_into_trace",
        "prepare_timeout_attribution_is_serialised_into_trace",
        "snapshot_timeout_runtime_is_serialised_into_trace",
        "exact_wait_task_state_drilldown_is_serialised_into_trace",
        "exact_wait_artifact_poll_is_serialised_into_trace",
        "overlapping_completion_request_context_can_be_taken_by_request_id_out_of_order",
        "pre_method_attribution_provenance_stays_fail_closed_for_overlapping_completion",
        "p33_same_file_completion_supersession_releases_pre_active_turn_wait_before_active_registration",
        "p28_cancel_request_releases_pre_active_turn_wait_before_active_registration",
        "p33_same_file_completion_supersession_releases_active_turn_during_response_build",
        "p33_same_file_completion_supersession_releases_active_turn_at_format_checkpoint",
    ]

    def smoke_script_text(self) -> str:
        return self.SMOKE_SCRIPT.read_text(encoding="utf-8")

    def shipped_smoke_gate(self) -> dict:
        data = json.loads(self.QUALITY_GATES.read_text(encoding="utf-8"))
        for gate in data["gates"]:
            if gate["id"] == "shipped_smoke_gate":
                return gate
        self.fail("shipped_smoke_gate missing from quality-gates.json")

    def document_symbol_mixed_load_summary(self) -> dict:
        return json.loads(
            self.DOCUMENT_SYMBOL_MIXED_LOAD_REPORT.read_text(encoding="utf-8")
        )["summary"]

    def overlap_gate_summary(self) -> dict:
        return json.loads(self.OVERLAP_GATE_REPORT.read_text(encoding="utf-8"))[
            "summary"
        ]

    def turn_wait_gate_summary(self) -> dict:
        return json.loads(self.TURN_WAIT_GATE_REPORT.read_text(encoding="utf-8"))[
            "summary"
        ]

    def slot_release_gate_summary(self) -> dict:
        return json.loads(self.SLOT_RELEASE_GATE_REPORT.read_text(encoding="utf-8"))[
            "summary"
        ]

    def slot_release_readiness_gate(self) -> dict:
        return json.loads(
            self.SLOT_RELEASE_READINESS_GATE_JSON.read_text(encoding="utf-8")
        )

    @staticmethod
    def format_metric(value: float | int) -> str:
        if isinstance(value, int) or float(value).is_integer():
            return str(int(value))
        return f"{value:g}"

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

    def test_shipped_smoke_script_covers_extension_completion_observability_slice(self) -> None:
        smoke_text = self.smoke_script_text()
        missing = [
            snippet
            for snippet in self.REQUIRED_EXTENSION_SMOKE_SNIPPETS
            if snippet not in smoke_text
        ]
        self.assertFalse(
            missing,
            (
                "run-intellisense-tests.sh smoke is missing the focused extension "
                f"completion-observability slice: {missing}"
            ),
        )

    def test_shipped_smoke_script_covers_completion_timeline_drilldown_contract_slice(self) -> None:
        smoke_text = self.smoke_script_text()
        missing = [
            selector
            for selector in self.REQUIRED_COMPLETION_TIMELINE_DRILLDOWN_SELECTORS
            if selector not in smoke_text
        ]
        self.assertFalse(
            missing,
            (
                "run-intellisense-tests.sh smoke is missing the completion-timeline "
                f"drilldown contract slice: {missing}"
            ),
        )

    def test_document_symbol_mixed_load_validation_doc_matches_checked_in_report(self) -> None:
        content = self.DOCUMENT_SYMBOL_MIXED_LOAD_DOC.read_text(encoding="utf-8")
        summary = self.document_symbol_mixed_load_summary()
        expected_snippets = [
            (
                f"CHANGE_ID={self.DOCUMENT_SYMBOL_CHANGE_ID} "
                "./scripts/validate-v2-completion-gates.sh"
            ),
            f"`{summary['measured_completion_total_delta']}` completion samples",
            f"`{summary['measured_head_hit_traces']}` `head_hit`",
            f"`{summary['measured_exact_hit_traces']}` `exact_hit`",
            f"`{summary['measured_prepare_timeout_total_delta']}` `prepare_timeout`",
            f"`{summary['measured_exact_deadline_total_delta']}` `exact_deadline`",
            f"`{summary['measured_ingress_regression_samples']}` ingress-regression samples",
            f"`{summary['measured_document_symbol_latest_ready_total_delta']}` `latest_ready`",
            f"`{summary['measured_document_symbol_current_ready_total_delta']}` `current_ready`",
            f"`{summary['measured_document_symbol_unavailable_total_delta']}` `unavailable`",
            f"`{summary['measured_document_symbol_superseded_total_delta']}` `superseded`",
            f"`{summary['measured_document_symbol_present_responses_total']}` non-null responses",
            f"`{summary['measured_document_symbol_null_responses_total']}` null responses",
            (
                "`p95(service_future_to_first_poll_wait_ms)="
                f"{self.format_metric(summary['measured_service_future_to_first_poll_wait_ms']['p95'])}ms`"
            ),
            (
                "`max(service_future_to_first_poll_wait_ms)="
                f"{summary['measured_service_future_to_first_poll_wait_max_ms']}ms`"
            ),
            (
                "`p95(transport_to_handler_wait_ms)="
                f"{self.format_metric(summary['measured_transport_to_handler_wait_ms']['p95'])}ms`"
            ),
            (
                "`max(transport_to_handler_wait_ms)="
                f"{summary['measured_transport_to_handler_wait_max_ms']}ms`"
            ),
        ]
        missing = [snippet for snippet in expected_snippets if snippet not in content]
        self.assertFalse(
            missing,
            (
                "documentSymbol mixed-load validation doc drifted from authoritative "
                f"checked-in report, missing snippets: {missing}"
            ),
        )

    def test_overlap_validation_doc_matches_checked_in_report(self) -> None:
        content = self.OVERLAP_GATE_DOC.read_text(encoding="utf-8")
        summary = self.overlap_gate_summary()
        expected_snippets = [
            "./scripts/validate-completion-superseded-active-turn-release.sh",
            (
                f"CHANGE_ID={self.OVERLAP_CHANGE_ID} "
                "./scripts/validate-v2-completion-gates.sh"
            ),
            f"`{summary['measured_first_cancelled_or_superseded_traces']}`",
            f"`{summary['measured_first_empty_response_samples']}`",
            f"`{summary['measured_first_registry_cleared_samples']}`",
            f"`{summary['measured_second_non_empty_samples']}`",
            f"`{summary['measured_head_hit_traces']}` `head_hit`",
            f"`{summary['measured_exact_hit_traces']}` `exact_hit`",
            f"`prepare_timeout` delta: `{summary['measured_prepare_timeout_total_delta']}`",
            f"`exact_deadline` delta: `{summary['measured_exact_deadline_total_delta']}`",
            f"`cancelled` delta: `{summary['measured_cancelled_total_delta']}`",
            f"`fail_closed` delta: `{summary['measured_fail_closed_total_delta']}`",
            (
                "`p95(service_future_to_first_poll_wait_ms)="
                f"{self.format_metric(summary['measured_service_future_to_first_poll_wait_ms']['p95'])}ms`"
            ),
            (
                "`max(service_future_to_first_poll_wait_ms)="
                f"{summary['measured_service_future_to_first_poll_wait_max_ms']}ms`"
            ),
        ]
        missing = [snippet for snippet in expected_snippets if snippet not in content]
        self.assertFalse(
            missing,
            (
                "overlap validation doc drifted from authoritative checked-in report, "
                f"missing snippets: {missing}"
            ),
        )

    def test_turn_wait_validation_doc_matches_checked_in_report(self) -> None:
        content = self.TURN_WAIT_GATE_DOC.read_text(encoding="utf-8")
        summary = self.turn_wait_gate_summary()
        expected_snippets = [
            "./scripts/validate-completion-turn-wait-lifecycle.sh",
            (
                f"CHANGE_ID={self.TURN_WAIT_CHANGE_ID} "
                "./scripts/validate-v2-completion-gates.sh"
            ),
            f"`{summary['measured_first_cancelled_or_superseded_traces']}`",
            f"`{summary['measured_first_empty_response_samples']}`",
            f"`{summary['measured_first_registry_cleared_samples']}`",
            f"`{summary['measured_first_pre_active_turn_wait_ready_traces']}`",
            f"`{summary['measured_stranded_pre_active_turn_wait_samples']}`",
            f"`{summary['measured_second_non_empty_samples']}`",
            f"`{summary['measured_head_hit_traces']}` `head_hit`",
            f"`{summary['measured_exact_hit_traces']}` `exact_hit`",
            f"`prepare_timeout` delta: `{summary['measured_prepare_timeout_total_delta']}`",
            f"`exact_deadline` delta: `{summary['measured_exact_deadline_total_delta']}`",
            f"`cancelled` delta: `{summary['measured_cancelled_total_delta']}`",
            f"`fail_closed` delta: `{summary['measured_fail_closed_total_delta']}`",
            (
                "`p95(service_future_to_first_poll_wait_ms)="
                f"{self.format_metric(summary['measured_service_future_to_first_poll_wait_ms']['p95'])}ms`"
            ),
            (
                "`max(service_future_to_first_poll_wait_ms)="
                f"{summary['measured_service_future_to_first_poll_wait_max_ms']}ms`"
            ),
        ]
        missing = [snippet for snippet in expected_snippets if snippet not in content]
        self.assertFalse(
            missing,
            (
                "turn-wait validation doc drifted from authoritative checked-in report, "
                f"missing snippets: {missing}"
            ),
        )

    def test_slot_release_validation_assets_are_checked_in(self) -> None:
        required_paths = [
            self.SLOT_RELEASE_ARCHITECTURAL_REVIEW,
            self.SLOT_RELEASE_TRACEABILITY_DOC,
            self.SLOT_RELEASE_GATE_DOC,
            self.SLOT_RELEASE_CHURN_REPORT,
            self.SLOT_RELEASE_CHURN_SUMMARY,
            self.SLOT_RELEASE_GATE_REPORT,
            self.SLOT_RELEASE_GATE_SUMMARY,
            self.SLOT_RELEASE_READINESS_GATE_JSON,
            self.SLOT_RELEASE_READINESS_GATE_MD,
            self.SLOT_RELEASE_OPENSPEC_LOG,
        ]
        missing = [str(path.relative_to(self.REPO_ROOT)) for path in required_paths if not path.exists()]
        self.assertFalse(
            missing,
            (
                "slot-release readiness bundle is incomplete; missing checked-in assets: "
                f"{missing}"
            ),
        )

    def test_slot_release_generated_readiness_assets_are_not_gitignored(self) -> None:
        tracked_artifacts = [
            self.SLOT_RELEASE_CHURN_SUMMARY,
            self.SLOT_RELEASE_GATE_SUMMARY,
            self.SLOT_RELEASE_READINESS_GATE_JSON,
            self.SLOT_RELEASE_READINESS_GATE_MD,
            self.SLOT_RELEASE_OPENSPEC_LOG,
        ]
        ignored = []
        for path in tracked_artifacts:
            relative_path = str(path.relative_to(self.REPO_ROOT))
            result = subprocess.run(
                ["git", "check-ignore", "-q", relative_path],
                cwd=self.REPO_ROOT,
                check=False,
                capture_output=True,
                text=True,
            )
            if result.returncode == 0:
                ignored.append(relative_path)
        self.assertFalse(
            ignored,
            (
                "slot-release readiness assets must be commit-able, but git still ignores: "
                f"{ignored}"
            ),
        )

    def test_slot_release_validation_doc_matches_checked_in_report(self) -> None:
        content = self.SLOT_RELEASE_GATE_DOC.read_text(encoding="utf-8")
        summary = self.slot_release_gate_summary()
        expected_snippets = [
            "./scripts/validate-completion-turn-wait-slot-release.sh",
            (
                f"CHANGE_ID={self.SLOT_RELEASE_CHANGE_ID} "
                "./scripts/validate-v2-completion-gates.sh"
            ),
            f"`{summary['measured_first_cancelled_or_superseded_traces']}`",
            f"`{summary['measured_first_empty_response_samples']}`",
            f"`{summary['measured_first_registry_cleared_samples']}`",
            f"`{summary['measured_first_pre_active_turn_wait_ready_traces']}`",
            f"`{summary['measured_stranded_pre_active_turn_wait_samples']}`",
            f"`{summary['measured_second_non_empty_samples']}`",
            f"`{summary['measured_second_transport_slot_released_samples']}`",
            f"`{summary['measured_second_slot_release_before_turn_wait_samples']}`",
            f"`{summary['measured_head_hit_traces']}` `head_hit`",
            f"`{summary['measured_exact_hit_traces']}` `exact_hit`",
            f"`prepare_timeout` delta: `{summary['measured_prepare_timeout_total_delta']}`",
            f"`exact_deadline` delta: `{summary['measured_exact_deadline_total_delta']}`",
            f"`cancelled` delta: `{summary['measured_cancelled_total_delta']}`",
            f"`fail_closed` delta: `{summary['measured_fail_closed_total_delta']}`",
            (
                "`p95(transport_to_slot_release_wait_ms)="
                f"{self.format_metric(summary['measured_transport_to_slot_release_wait_ms']['p95'])}ms`"
            ),
            (
                "`max(transport_to_slot_release_wait_ms)="
                f"{summary['measured_transport_to_slot_release_wait_max_ms']}ms`"
            ),
            (
                "`p95(service_future_to_first_poll_wait_ms)="
                f"{self.format_metric(summary['measured_service_future_to_first_poll_wait_ms']['p95'])}ms`"
            ),
            (
                "`max(service_future_to_first_poll_wait_ms)="
                f"{summary['measured_service_future_to_first_poll_wait_max_ms']}ms`"
            ),
        ]
        missing = [snippet for snippet in expected_snippets if snippet not in content]
        self.assertFalse(
            missing,
            (
                "slot-release validation doc drifted from authoritative checked-in report, "
                f"missing snippets: {missing}"
            ),
        )

    def test_slot_release_readiness_gate_references_checked_in_artifacts(self) -> None:
        gate = self.slot_release_readiness_gate()
        self.assertEqual(gate["change_id"], self.SLOT_RELEASE_CHANGE_ID)
        self.assertTrue(gate["pass"], "slot-release readiness gate must stay green")
        real_module_gates = gate["real_module_gates"]
        self.assertEqual(
            real_module_gates["churn"]["report"],
            f"tests/perf/reports/{self.SLOT_RELEASE_CHANGE_ID}-real-conf-big-revision-churn-completion-perf-live.json",
        )
        self.assertEqual(
            real_module_gates["churn"]["summary"],
            f"tests/perf/reports/{self.SLOT_RELEASE_CHANGE_ID}-real-conf-big-revision-churn-completion-perf-live.md",
        )
        self.assertEqual(
            real_module_gates["preactive_overlap"]["report"],
            f"tests/perf/reports/{self.SLOT_RELEASE_CHANGE_ID}-real-conf-big-pre-active-overlap-completion-perf-live.json",
        )
        self.assertEqual(
            real_module_gates["preactive_overlap"]["summary"],
            f"tests/perf/reports/{self.SLOT_RELEASE_CHANGE_ID}-real-conf-big-pre-active-overlap-completion-perf-live.md",
        )

    def test_slot_release_raw_gate_defaults_use_current_change_id(self) -> None:
        core_tests = (
            self.REPO_ROOT
            / "backend"
            / "src"
            / "bin"
            / "lsp_server"
            / "server"
            / "core"
            / "tests.rs"
        ).read_text(encoding="utf-8")
        p38_pattern = re.compile(
            r'PROFILE_NAME:\s*&str\s*=\s*"p38_real_conf_big_post_handoff_readiness_completion_perf_report_live";'
            r'.*?let change_id = std::env::var\("CHANGE_ID"\)\s*'
            r'\.unwrap_or_else\(\|_\| "refactor-completion-turn-wait-slot-release"\.to_string\(\)\);',
            re.DOTALL,
        )
        p41_pattern = re.compile(
            r'PROFILE_NAME:\s*&str\s*=\s*"p41_real_conf_big_pre_active_turn_wait_overlap_completion_perf_report_live";'
            r'.*?let change_id = std::env::var\("CHANGE_ID"\)\s*'
            r'\.unwrap_or_else\(\|_\| "refactor-completion-turn-wait-slot-release"\.to_string\(\)\);',
            re.DOTALL,
        )
        self.assertRegex(
            core_tests,
            p38_pattern,
            "p38 raw representative gate must default to slot-release change-id",
        )
        self.assertRegex(
            core_tests,
            p41_pattern,
            "p41 raw representative gate must default to slot-release change-id",
        )

    def test_slot_release_ci_wiring_tracks_wrapper_and_artifacts(self) -> None:
        workflow = self.CI_WORKFLOW.read_text(encoding="utf-8")
        self.assertGreaterEqual(
            workflow.count("scripts/validate-completion-turn-wait-slot-release.sh"),
            2,
            "CI path filters must watch slot-release wrapper on pull_request and push",
        )
        expected_snippets = [
            "CHANGE_ID: refactor-completion-turn-wait-slot-release",
            (
                "BSL_V2_REAL_CONF_BIG_REVISION_CHURN_COMPLETION_PERF_REPORT: "
                "${{ github.workspace }}/backend/tests/perf/reports/"
                "refactor-completion-turn-wait-slot-release-real-conf-big-"
                "revision-churn-completion-perf-live.json"
            ),
            (
                "BSL_V2_REAL_CONF_BIG_PRE_ACTIVE_OVERLAP_COMPLETION_PERF_REPORT: "
                "${{ github.workspace }}/backend/tests/perf/reports/"
                "refactor-completion-turn-wait-slot-release-real-conf-big-"
                "pre-active-overlap-completion-perf-live.json"
            ),
            "backend/tests/perf/reports/refactor-completion-turn-wait-slot-release-*.json",
            "backend/tests/perf/reports/refactor-completion-turn-wait-slot-release-*.md",
        ]
        missing = [snippet for snippet in expected_snippets if snippet not in workflow]
        self.assertFalse(
            missing,
            f"CI workflow is missing slot-release readiness wiring: {missing}",
        )

    def test_slot_release_development_workflow_mentions_wrapper(self) -> None:
        workflow_guide = self.DEVELOPMENT_WORKFLOW_GUIDE.read_text(encoding="utf-8")
        self.assertIn(
            "./scripts/validate-completion-turn-wait-slot-release.sh",
            workflow_guide,
            "development workflow guide must list the canonical slot-release wrapper",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
