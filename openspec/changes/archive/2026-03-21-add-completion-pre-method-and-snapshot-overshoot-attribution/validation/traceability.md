# Traceability: add-completion-pre-method-and-snapshot-overshoot-attribution

## Requirement -> Code -> Test

### Requirement: Completion timeline v7 сужает `server_before_method_entry` до bounded pre-method segments
- Authoritative `v7` contract shape и additive server-edge fields:
  - Code: `backend/src/bin/lsp_server/types.rs`
  - Code: `backend/src/bin/lsp_server/server/mod.rs`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_exposes_versioned_contract`
- `service_scope_entered_at_ms` и derived waits сериализуются без free-text:
  - Code: `backend/src/bin/lsp_server/server/request_context.rs`
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_records_completion_context_for_position_lookup`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_are_derived_from_transport_handler_and_response_timestamps`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_keep_first_cancel_observation_and_derive_late_cancel_delta`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_contains_completion_trace`

### Requirement: `prepare_timeout` на `snapshot_with_deps` получает timeout-safe bounded runtime attribution
- Runtime/facade накапливает partial timeout split для `snapshot_with_deps`:
  - Code: `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - Code: `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - Code: `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - Code: `bsl-runtime/src/application/intellisense_v2/mod.rs`
  - Code: `bsl-runtime/src/application/mod.rs`
  - Test: `bsl-runtime/src/application/intellisense_v2/facade/tests.rs::snapshot_with_deps_timeout_can_report_queue_wait_runtime_split_via_progress`
- Authoritative completion trace сериализует bounded `snapshot_with_deps_timeout_runtime` и explicit `resolution=unavailable`:
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::snapshot_timeout_runtime_is_serialised_into_trace`

### Requirement: Existing completion surfaces переносят `v7` pre-method и snapshot overshoot facts без invented data
- Typed client contract для additive `v7` fields:
  - Code: `vscode-extension/src/lsp/customRequests.ts`
  - Test: `vscode-extension/src/test/suite/customRequests.test.ts::getCompletionTimeline should work via executeCommand`
- Completion Timeline clipboard/panel показывают pre-method split и timeout runtime:
  - Code: `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - Code: `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - Code: `vscode-extension/src/providers/completionTimelineWebview.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts::formatVisibleCompletionTimelineForClipboard should include header and visible traces`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts::formatVisibleCompletionTimelineForClipboard should mark v6 payload as missing v7 fields by design`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts::copyVisible message should write current visible traces to clipboard`
  - Test: `vscode-extension/src/test/suite/completionTimelineModel.test.ts::Mapping LSP timeline payload -> UI model`
- Request-centric incident bundle summary переносит `v7` facts и явно деградирует на `v6`:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Code: `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::happy path bundle should contain request-centric incident report and all raw attachments`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::v6 completion timeline should stay valid and mark v7 pre-method and snapshot overshoot details as unavailable`
  - Test: `vscode-extension/src/test/suite/observabilityCommands.test.ts::exportObservabilityIncidentBundle should write bundle files via command callback`

## Operational truth
- Shipped smoke path:
  - Code: `scripts/run-intellisense-tests.sh`
  - Test: `scripts/test-intellisense-readiness-assets.py`
- Manual/runbook expectations:
  - Doc: `scripts/README.md`
  - Doc: `vscode-extension/manual-lsp-test.md`
  - Doc: `vscode-extension/src/test/README.md`
