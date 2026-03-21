# Traceability: add-completion-latency-root-cause-attribution

## Requirement -> Code -> Test

### Requirement: LSP предоставляет versioned per-request completion timeline контракт
- Authoritative `v6` contract shape и bounded DTO:
  - Code: `backend/src/bin/lsp_server/types.rs`
  - Code: `backend/src/bin/lsp_server/server/mod.rs`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_exposes_versioned_contract`
- Bounded method-entry split между `transport_received`, `method_entered` и `handler_entered`:
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_contains_completion_trace`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_are_derived_from_transport_handler_and_response_timestamps`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_keep_first_cancel_observation_and_derive_late_cancel_delta`
- Bounded prepare runtime drilldown и timeout attribution:
  - Code: `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - Code: `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `bsl-runtime/src/application/intellisense_v2/facade/tests.rs::wait_for_file_version_runtime_trace_distinguishes_immediate_and_waiter_paths`
  - Test: `bsl-runtime/src/application/intellisense_v2/facade/tests.rs::snapshot_with_deps_runtime_trace_exposes_queue_and_exec_latency`
  - Test: `bsl-runtime/src/application/intellisense_v2/facade/tests.rs::interactive_wait_budget_timeout_can_still_report_timeout_attribution_on_success`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::prepare_runtime_drilldown_is_serialised_into_trace`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::prepare_timeout_attribution_is_serialised_into_trace`
- Bounded `exact_wait.artifact_poll` до waiter/task-state path:
  - Code: `backend/src/bin/lsp_server/server/core/deps_and_precompute.rs`
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::exact_wait_task_state_drilldown_is_serialised_into_trace`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::exact_wait_artifact_poll_is_serialised_into_trace`

### Requirement: Existing completion surfaces переносят `v6` root-cause attribution без invented data
- Shared drilldown derivation и bounded fact lines:
  - Code: `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineDrilldown.test.ts`
- Completion Timeline panel и clipboard export:
  - Code: `vscode-extension/src/providers/completionTimelineWebview.ts`
  - Code: `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineModel.test.ts`
- Incident handoff summary и явная деградация на `v5`:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Code: `vscode-extension/src/lsp/customRequests.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts`
  - Test: `vscode-extension/src/test/suite/observabilityCommands.test.ts`
  - Test: `vscode-extension/src/test/suite/customRequests.test.ts`

## Operational truth
- Shipped smoke path:
  - Code: `scripts/run-intellisense-tests.sh`
  - Test: `scripts/test-intellisense-readiness-assets.py`
- Manual/runbook expectations:
  - Doc: `scripts/README.md`
  - Doc: `vscode-extension/manual-lsp-test.md`
  - Doc: `vscode-extension/src/test/README.md`
