# Traceability: add-completion-bottleneck-drilldown

## Requirement -> Code -> Test

### Requirement: LSP предоставляет versioned per-request completion timeline контракт
- Authoritative `v5` contract shape и bounded DTO:
  - Code: `backend/src/bin/lsp_server/types.rs`
  - Code: `backend/src/bin/lsp_server/server/mod.rs`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_exposes_versioned_contract`
- Bounded ingress/dispatcher attribution и server-generated trace serialization:
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_contains_completion_trace`
- Bounded prepare runtime drilldown для `wait_for_file_version` / `snapshot_with_deps`:
  - Code: `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - Code: `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - Code: `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - Test: `bsl-runtime/src/application/intellisense_v2/facade/tests.rs::wait_for_file_version_runtime_trace_distinguishes_immediate_and_waiter_paths`
  - Test: `bsl-runtime/src/application/intellisense_v2/facade/tests.rs::snapshot_with_deps_runtime_trace_exposes_queue_and_exec_latency`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::prepare_runtime_drilldown_is_serialised_into_trace`
- Bounded `exact_wait` waiter/task-state drilldown:
  - Code: `backend/src/bin/lsp_server/server/core/deps_and_precompute.rs`
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::exact_wait_task_state_drilldown_is_serialised_into_trace`

### Requirement: Человекочитаемые completion timeline projections сохраняют authoritative bottleneck semantics
- Shared drilldown derivation для verdict'ов и bounded fact lines:
  - Code: `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts`
- Completion Timeline panel и clipboard export:
  - Code: `vscode-extension/src/providers/completionTimelineWebview.ts`
  - Code: `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineModel.test.ts`
- Incident handoff summary поверх authoritative timeline:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts`
  - Test: `vscode-extension/src/test/suite/observabilityCommands.test.ts`
- Graceful degradation для `v4` payload:
  - Code: `vscode-extension/src/lsp/customRequests.ts`
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Code: `vscode-extension/src/providers/completionTimelineWebview.ts`
  - Test: `vscode-extension/src/test/suite/customRequests.test.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts`

## Operational truth
- Shipped smoke path:
  - Code: `scripts/run-intellisense-tests.sh`
  - Test: `scripts/test-intellisense-readiness-assets.py`
- Manual/runbook expectations:
  - Doc: `scripts/README.md`
  - Doc: `vscode-extension/manual-lsp-test.md`
  - Doc: `vscode-extension/src/test/README.md`
