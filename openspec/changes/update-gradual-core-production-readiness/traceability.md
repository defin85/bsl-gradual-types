# Traceability: update-gradual-core-production-readiness

Этот артефакт фиксирует обязательную трассировку `Requirement -> Future Code Area -> Required Test Class`
для future-facing production-readiness contract.

## Requirement: Shared resolved contract first-class выражает snapshot-local structural members

Future code areas:
- `bsl-types/src/types/certainty.rs`
- `bsl-types/src/types/resolution.rs`
- `bsl-types/src/types/structural_members.rs`
- `bsl-types/src/types/resolution_impl/structural_members.rs`
- `analysis-v2/src/type_inference_v2/instance_effects.rs`
- `analysis-v2/src/type_inference_v2.rs`

Required test class:
- `analysis-v2` unit tests for typed `Структура` member materialization
- `analysis-v2` unit tests for typed-row column materialization
- `analysis-v2` snapshot-isolation tests proving structural members do not leak across revisions

Current signal:
- existing `analysis-v2/src/type_inference_v2/tests.rs` already covers typed `Структура`, typed-row and `source_span`;
- future remediation still needs delivery evidence for stable member identity as explicit shared contract data.

## Requirement: Semantic consumers используют один resolved path или thin adapters

Future code areas:
- `bsl-runtime/src/application/intellisense_v2/facade.rs`
- `bsl-runtime/src/application/type_system/services/completion_service.rs`
- `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs`
- `bsl-runtime/src/application/type_system/services/hover_service.rs`
- `backend/src/bin/lsp_server/handlers/completion.rs`
- `backend/src/bin/lsp_server/handlers/hover.rs`
- `backend/src/presentation/web/handlers.rs`
- `backend/src/presentation/web/handlers/semantic.rs`
- `bsl-agent/src/session/manager_semantic_core.rs`

Required test class:
- backend exact acceptance tests for completion + hover + type-at-position + diagnostics
- cross-interface parity tests for `LSP` / `MCP` / `Web`
- regression tests proving semantic correctness does not require consumer-only owner/member hints

Current signal:
- existing backend exact-acceptance tests already compare completion/hover/type-at-position/diagnostics on typed `Структура` and typed-row;
- completion still contains bootstrap/local owner-resolution branches that must shrink to thin-adapter or bootstrap-only status.

## Requirement: Cross-consumer acceptance доказывает semantic equivalence, а не только smoke consistency

Future code areas:
- `backend/src/bin/lsp_server/server/core/tests.rs`
- `backend/tests/universal_collection_cross_consumer_consistency_test.rs`
- `backend/tests/universal_collection_strict_policy_test.rs`
- acceptance/reporting artifacts under `openspec/changes/update-gradual-core-production-readiness/`

Required test class:
- exact acceptance tests with same owner resolution result
- exact acceptance tests with same member identity
- policy tests for known/unknown member parity
- negative tests that intentionally remove hidden consumer-only hints and expect drift detection

Current signal:
- smoke/parity coverage already exists;
- exact matrix must remain explicit and must not be replaced by smoke-only pass criteria.

## Requirement: Change completion MUST NOT завышать readiness относительно MUST backlog

Future code areas:
- `openspec/changes/update-gradual-core-production-readiness/tasks.md`
- `openspec/changes/update-gradual-core-production-readiness/traceability.md`
- `openspec/changes/update-gradual-core-production-readiness/specs/dev-workflow/spec.md`
- Beads task graph in `.beads/`
- future workflow automation hook or CI job that compares OpenSpec evidence against critical backlog

Required test class:
- strict OpenSpec validation
- workflow audit/review artifact that checks checklist vs traceability vs critical backlog
- CI or scripted readiness-gate smoke check once automation is introduced

Current signal:
- current change now has explicit contract text plus traceability artifact;
- automated readiness gate is still a future delivery item and therefore cannot yet be overclaimed as shipped workflow enforcement.
