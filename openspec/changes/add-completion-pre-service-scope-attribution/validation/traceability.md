# Traceability: add-completion-pre-service-scope-attribution

## Requirement -> Code -> Test

### Requirement: Completion timeline `v9` сужает pre-service-scope attribution до bounded segments
- Authoritative `v9` contract shape и additive bounded fields:
  - Code: `backend/src/bin/lsp_server/server/mod.rs`
  - Code: `backend/src/bin/lsp_server/types.rs`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_contains_completion_trace`
- `service_future_created_at_ms` проходит через request-context producer path без guesswork:
  - Code: `backend/src/bin/lsp_server/server/request_context.rs`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_sets_jsonrpc_numeric_id`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_records_completion_context_for_position_lookup`
- Derived waits сериализуются только когда сервер действительно знает split:
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_are_derived_from_transport_handler_and_response_timestamps`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_do_not_fabricate_service_future_split_when_timestamp_is_absent`

### Requirement: `v9` split сохраняет trustworthy `v8` attribution semantics
- Overlap и request-id handoff остаются fail-closed и не подменяются invented `v9` facts:
  - Code: `backend/src/bin/lsp_server/server/request_context.rs`
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::overlapping_completion_request_context_can_be_taken_by_request_id_out_of_order`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_does_not_propagate_request_id_to_spawned_handler`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_keep_first_cancel_observation_and_derive_late_cancel_delta`

### Requirement: Existing completion surfaces переносят `v9` split без invented data
- Typed client contract принимает `v9` payload и не реконструирует missing split локально:
  - Code: `vscode-extension/src/lsp/customRequests.ts`
  - Test: `vscode-extension/src/test/suite/customRequests.test.ts::getCompletionTimeline should work via executeCommand`
- Panel / clipboard / webview показывают bounded `service_future_created` split и явно деградируют на `v8`:
  - Code: `vscode-extension/src/providers/completionTimelineModel.ts`
  - Code: `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - Code: `vscode-extension/src/providers/completionTimelineWebview.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineModel.test.ts::Mapping LSP timeline payload -> UI model`
  - Test: `vscode-extension/src/test/suite/completionTimelineModel.test.ts::Average trace provenance notice should mark averaged traces as synthetic`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts::formatVisibleCompletionTimelineForClipboard should include header and visible traces`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts::formatVisibleCompletionTimelineForClipboard should mark v8 payload as missing v9 split by design`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts::copyVisible message should write current visible traces to clipboard`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts::copyVisible message should mark average mode traces as synthetic provenance`
- Incident bundle summary/findings/gaps переносят bounded split и явно называют `v8` limitation:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::happy path bundle should contain request-centric incident report and all raw attachments`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::v8 completion timeline should mark v9 pre-service-scope split as unavailable by design`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::v7 completion timeline should stay valid and mark v8 provenance details as unavailable`

## Operational truth
- Shipped smoke path:
  - Code: `scripts/run-intellisense-tests.sh`
  - Test: `scripts/test-intellisense-readiness-assets.py`
- Manual/runbook expectations:
  - Doc: `scripts/README.md`
  - Doc: `vscode-extension/src/test/README.md`
  - Doc: `vscode-extension/manual-lsp-test.md`
