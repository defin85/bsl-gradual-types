# Traceability: add-completion-dispatch-to-request-context-attribution

## Requirement -> Code -> Test

### Requirement: Completion timeline `v10` добавляет bounded dispatch-to-request-context split
- Authoritative `v10` contract shape и additive bounded fields:
  - Code: `backend/src/bin/lsp_server/server/mod.rs`
  - Code: `backend/src/bin/lsp_server/types.rs`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_exposes_versioned_contract`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_contains_completion_trace`
- Versioned contract baseline и policy checker синхронизированы с shipped payload:
  - Code: `contracts/lsp-completion-timeline/v7/contract.json`
  - Code: `contracts/lsp-completion-timeline/v7/schema.json`
  - Code: `contracts/lsp-completion-timeline/v7/changelog.md`
  - Code: `scripts/check-versioned-contracts.py`
  - Test: `bsl-runtime/src/system/basic_observability/tests.rs::completion_timeline_v7_contract_matches_current_runtime_payload_shape`
  - Test: `python3 scripts/check-versioned-contracts.py`
- Outer dispatch hook wired на default runtime path и заполняет authoritative ingress timestamp:
  - Code: `backend/src/bin/lsp_server/main.rs`
  - Code: `backend/src/bin/lsp_server/server/request_context.rs`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::dispatch_context_service_records_completion_context_for_position_lookup`
- Derived dispatch split сериализуется только когда сервер действительно знает outer dispatch anchor:
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_use_outer_dispatch_timestamp_as_transport_anchor_when_available`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_do_not_fabricate_service_future_split_when_timestamp_is_absent`

### Requirement: `v10` сохраняет truthful ingress provenance и honest fallback semantics
- Missing outer dispatch timestamp не подменяется guessed полями:
  - Code: `backend/src/bin/lsp_server/server/request_context.rs`
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::current_request_jsonrpc_dispatch_received_at_ms_is_none_outside_scope`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_are_derived_from_transport_handler_and_response_timestamps`
- Existing `v9` overlap/fallback semantics остаются bounded и fail-closed:
  - Code: `backend/src/bin/lsp_server/server/request_context.rs`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::overlapping_completion_request_context_can_be_taken_by_request_id_out_of_order`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_does_not_propagate_request_id_to_spawned_handler`

### Requirement: Existing completion surfaces переносят `v10` split без invented data
- Typed client contract принимает `v10` payload и не реконструирует missing split локально:
  - Code: `vscode-extension/src/lsp/customRequests.ts`
  - Test: `vscode-extension/src/test/suite/customRequests.test.ts::getCompletionTimeline should work via executeCommand`
- Panel / clipboard / webview показывают bounded dispatch split и явно деградируют на `v9`:
  - Code: `vscode-extension/src/providers/completionTimelineModel.ts`
  - Code: `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - Code: `vscode-extension/src/providers/completionTimelineWebview.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineModel.test.ts::Mapping LSP timeline payload -> UI model`
  - Test: `vscode-extension/src/test/suite/completionTimelineModel.test.ts::Average trace provenance notice should mark averaged traces as synthetic`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts::formatVisibleCompletionTimelineForClipboard should include header and visible traces`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts::formatVisibleCompletionTimelineForClipboard should mark v9 payload as missing v10 dispatch split by design`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts::copyVisible message should write current visible traces to clipboard`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts::inline webview script should mark v9 payload as missing v10 dispatch split by design`
- Incident bundle summary/findings/gaps переносят bounded dispatch split и явно называют `v9` limitation:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::happy path bundle should contain request-centric incident report and all raw attachments`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::v9 completion timeline should mark v10 dispatch split as unavailable by design`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::v7 completion timeline should stay valid and mark v8 provenance details as unavailable`

## Operational truth
- Shipped smoke path:
  - Code: `scripts/run-intellisense-tests.sh`
  - Test: `scripts/test-intellisense-readiness-assets.py`
- Canonical OpenSpec truth после фиксации change:
  - Spec delta: `openspec/changes/add-completion-dispatch-to-request-context-attribution/specs/bsl-intellisense-v2/spec.md`
  - Command: `openspec archive add-completion-dispatch-to-request-context-attribution --yes`
- Manual/runbook expectations:
  - Doc: `scripts/README.md`
  - Doc: `vscode-extension/src/test/README.md`
  - Doc: `vscode-extension/manual-lsp-test.md`
