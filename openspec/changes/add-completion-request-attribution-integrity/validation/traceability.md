# Traceability: add-completion-request-attribution-integrity

## Requirement -> Code -> Test

### Requirement: Completion timeline `v8` публикует trustworthy pre-method attribution provenance
- Authoritative `v8` contract shape и additive provenance field:
  - Code: `backend/src/bin/lsp_server/server/mod.rs`
  - Code: `backend/src/bin/lsp_server/types.rs`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_exposes_versioned_contract`
  - Test: `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_contains_completion_trace`
- Same-request authoritative handoff идёт через request-id keyed registry, а не только через `uri + position`:
  - Code: `backend/src/bin/lsp_server/server/request_context.rs`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::overlapping_completion_request_context_can_be_taken_by_request_id_out_of_order`
- Timeline serializes bounded provenance without free-text:
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_include_pre_method_attribution_provenance`

### Requirement: Pre-method attribution integrity остаётся bounded и side-effect-safe
- Completion response semantics не меняются; producer path только понижает attribution confidence:
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Code: `backend/src/bin/lsp_server/server/request_context.rs`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_records_completion_context_for_position_lookup`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_sets_jsonrpc_numeric_id`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::request_context_service_does_not_propagate_request_id_to_spawned_handler`
- Overlap на одной позиции не маскируется под strong same-request ingress:
  - Code: `backend/src/bin/lsp_server/server/request_context.rs`
  - Code: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - Test: `backend/src/bin/lsp_server/server/request_context/tests.rs::overlapping_completion_request_context_can_be_taken_by_request_id_out_of_order`
  - Test: `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::pre_method_attribution_provenance_stays_fail_closed_for_overlapping_completion`

### Requirement: Existing completion surfaces различают strong и weak pre-method attribution без invented findings
- Typed client contract принимает `v8` provenance и не реконструирует его локально:
  - Code: `vscode-extension/src/lsp/customRequests.ts`
  - Test: `vscode-extension/src/test/suite/customRequests.test.ts::getCompletionTimeline should work via executeCommand`
- Shared verdict layer считает strong ingress только для `same_request_authoritative`:
  - Code: `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineDrilldown.test.ts::buildCompletionTraceBottleneckVerdicts should distinguish server wait before method entry dominance`
  - Test: `vscode-extension/src/test/suite/completionTimelineDrilldown.test.ts::buildCompletionTraceBottleneckVerdicts should fail-closed for weak pre-method provenance`
  - Test: `vscode-extension/src/test/suite/completionTimelineDrilldown.test.ts::buildCompletionTraceBottleneckVerdicts should require strong provenance for client ingress verdict`
- Clipboard/panel показывают provenance рядом с pre-method split и явно деградируют на `v7`:
  - Code: `vscode-extension/src/providers/completionTimelineModel.ts`
  - Code: `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - Code: `vscode-extension/src/providers/completionTimelineWebview.ts`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts::formatVisibleCompletionTimelineForClipboard should include header and visible traces`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts::formatVisibleCompletionTimelineForClipboard should keep synthetic average traces fail-closed for provenance`
  - Test: `vscode-extension/src/test/suite/completionTimelineClipboard.test.ts::formatVisibleCompletionTimelineForClipboard should mark v7 payload as missing v8 provenance by design`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts::copyVisible message should write current visible traces to clipboard`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts::copyVisible message should mark average mode traces as synthetic provenance`
  - Test: `vscode-extension/src/test/suite/completionTimelineWebviewProvider.test.ts::webview content declares separate server and client sections`
  - Test: `vscode-extension/src/test/suite/completionTimelineModel.test.ts::Mapping LSP timeline payload -> UI model`
  - Test: `vscode-extension/src/test/suite/completionTimelineModel.test.ts::Average trace provenance notice should mark averaged traces as synthetic`
- Incident bundle request summary сохраняет bounded provenance, а findings не агрегируют weak attribution как сильный ingress bottleneck:
  - Code: `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - Code: `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::happy path bundle should contain request-centric incident report and all raw attachments`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::best-effort pre-method provenance should stay visible but not aggregate as strong ingress finding`
  - Test: `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts::v7 completion timeline should stay valid and mark v8 provenance details as unavailable`
  - Test: `vscode-extension/src/test/suite/observabilityCommands.test.ts::exportObservabilityIncidentBundle should write bundle files via command callback`

## Operational truth
- Shipped smoke path:
  - Code: `scripts/run-intellisense-tests.sh`
  - Test: `scripts/test-intellisense-readiness-assets.py`
- Manual/runbook expectations:
  - Doc: `scripts/README.md`
  - Doc: `vscode-extension/manual-lsp-test.md`
  - Doc: `vscode-extension/src/test/README.md`
