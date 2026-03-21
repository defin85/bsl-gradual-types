# Traceability: add-completion-service-future-poll-wake-attribution

## Requirement -> Code -> Test

### Requirement: LSP предоставляет versioned per-request completion timeline контракт `v11`
- Authoritative `v11` contract shape и additive bounded fields:
  - Code: `backend/src/bin/lsp_server/server/mod.rs`
  - Code: `backend/src/bin/lsp_server/types.rs`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_exposes_versioned_contract`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_contains_completion_trace`
- Versioned contract baseline и policy checker синхронизированы с shipped payload:
  - Code: `contracts/lsp-completion-timeline/v8/contract.json`
  - Code: `contracts/lsp-completion-timeline/v8/schema.json`
  - Code: `contracts/lsp-completion-timeline/v8/changelog.md`
  - Code: `scripts/check-versioned-contracts.py`
  - Test: `bsl-runtime/src/system/basic_observability/tests.rs::completion_timeline_v8_contract_matches_current_runtime_payload_shape`
  - Test: `python3 scripts/check-versioned-contracts.py`
- Default runtime path отдаёт server-generated payload без client-side reconstruction:
  - Code: `backend/src/bin/lsp_server/main.rs`
  - Code: `backend/src/bin/lsp_server/server/request_context.rs`
  - Code: `vscode-extension/src/lsp/customRequests.ts`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_records_first_poll_and_first_wake_for_pending_future`
  - Test: `vscode-extension/src/test/suite/customRequests.test.ts::getCompletionTimeline should work via executeCommand`

### Requirement: `v11` service-future poll / wake split сохраняет truthful post-dispatch attribution semantics
- Returned service future фиксирует первый `poll()` и bounded outcome на authoritative path:
  - Code: `backend/src/bin/lsp_server/server/request_context.rs`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_records_first_poll_and_first_wake_for_pending_future`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_does_not_fabricate_first_wake_for_ready_first_poll`
- Derived waits сериализуются только при наличии соответствующих observed timestamps:
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_derive_first_poll_and_first_wake_split_when_present`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_do_not_fabricate_first_wake_split_when_first_poll_is_ready`
- Existing `v10` / `v9` / `v8` trustworthy semantics остаются bounded и fail-closed:
  - Code: `backend/src/bin/lsp_server/server/request_context.rs`
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::overlapping_completion_request_context_can_be_taken_by_request_id_out_of_order`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_does_not_propagate_request_id_to_spawned_handler`

### Requirement: Existing completion surfaces переносят `v11` split без invented data
- Typed client contract, panel и clipboard принимают `v11` payload и явно деградируют на `v10`:
  - Code: `vscode-extension/src/lsp/customRequests.ts`
  - Code: `vscode-extension/src/providers/completionTimelineModel.ts`
  - Code: `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - Code: `vscode-extension/src/providers/completionTimelineWebview.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineModel.test.ts::Mapping LSP timeline payload -> UI model`
  - Test: `vscode-extension/src/test/suite/completionTimelineModel.test.ts::Average trace provenance notice should mark averaged traces as synthetic`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts::formatVisibleCompletionTimelineForClipboard should include header and visible traces`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts::formatVisibleCompletionTimelineForClipboard should mark v10 payload as missing v11 first-poll / first-wake split by design`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts::copyVisible message should write current visible traces to clipboard`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts::inline webview script should mark v10 payload as missing v11 first-poll / first-wake split by design`
- Incident bundle summary/findings/gaps переносят bounded first-poll / first-wake split и явно называют limitation на `v10`:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::happy path bundle should contain request-centric incident report and all raw attachments`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::v10 completion timeline should mark v11 first-poll / first-wake split as unavailable by design`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::v7 completion timeline should stay valid and mark v8 provenance details as unavailable`

## Operational truth
- Shipped smoke path:
  - Code: `scripts/run-intellisense-tests.sh`
  - Test: `scripts/test-intellisense-readiness-assets.py`
  - Smoke selector: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_records_first_poll_and_first_wake_for_pending_future`
  - Smoke selector: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_does_not_fabricate_first_wake_for_ready_first_poll`
  - Smoke selector: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_derive_first_poll_and_first_wake_split_when_present`
  - Smoke selector: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_do_not_fabricate_first_wake_split_when_first_poll_is_ready`
- Manual/runbook expectations:
  - Doc: `scripts/README.md`
  - Doc: `vscode-extension/src/test/README.md`
  - Doc: `vscode-extension/manual-lsp-test.md`
- Canonical OpenSpec truth после фиксации change:
  - Spec delta: `openspec/changes/add-completion-service-future-poll-wake-attribution/specs/bsl-intellisense-v2/spec.md`
  - Command: `openspec archive add-completion-service-future-poll-wake-attribution --yes`
