# Residual Risk Review: update-gradual-core-production-readiness

Дата: 2026-03-08  
Change: `update-gradual-core-production-readiness`  
Task lineage: `bsl-gradual-types-6mx.5` -> `bsl-gradual-types-b6q.3` / `bsl-gradual-types-b6q.4`

## Scope

Этот артефакт фиксирует closure для residual semantic / edge-case risk matrix из production-readiness review.
Финальный governance / backlog verdict дополнительно закреплён в
`validation/final-closure-checklist.md` и `governance/readiness_status.json`.

## Residual Risk Matrix

| Risk | Previous review concern | Current status | Stronger guarantee / code | Automated evidence |
| --- | --- | --- | --- | --- |
| Stable structural member identity | Structural members сравнивались слишком lossy и не имели consumer-independent key | `retired` | Shared contract теперь несёт explicit `StructuralMemberId`, сохраняет identity через materialization / replace / serialization, а completion payload не теряет `member_identity` | `bsl-types/src/types/tests/structural_members_tests.rs`; `analysis-v2/src/type_inference_v2/tests.rs`; `backend/src/bin/lsp_server/handlers/completion/tests.rs`; `backend/src/bin/lsp_server/server/core/tests.rs` |
| Hidden completion-local reconstruction | Completion мог оставаться “умнее” других consumers за счёт local owner/member reconstruction | `retired` | Completion owner resolution теперь strict shared-hint-driven: direct handler path fail-closed без hint, а default LSP path по `FormModule.Объект.` продолжает работать через shared owner-hint producer без runtime fallback | `backend/tests/universal_collection_cross_consumer_consistency_test.rs`; `backend/tests/form_module_object_unified_contract_test.rs`; `backend/tests/legacy_form_object_alias_outputs_test.rs`; `backend/src/bin/lsp_server/server/core/tests.rs`; `bsl-runtime/src/application/type_system/services/completion_service/tests.rs` |
| Exact acceptance did not prove identity / hidden-hint failure | Happy-path parity не доказывал same member identity и fail-closed behaviour | `retired` | Cross-consumer acceptance проверяет runtime structural identity против LSP completion / MCP members, same type/policy verdict и fail-closed negative path без shared hint | `backend/src/bin/lsp_server/server/core/tests.rs`; `backend/tests/universal_collection_cross_consumer_consistency_test.rs`; `backend/src/bin/lsp_server/handlers/completion/tests.rs` |
| Lifecycle / revision leakage and edge cases | Case-insensitive replace/update, branch merges и snapshot switch могли ломать identity или протекать между interfaces | `retired` | Shared structural carrier сохраняет snapshot-local continuity, а revision-switch acceptance подтверждает fail-closed behaviour across runtime / LSP / MCP / Web diagnostics | `analysis-v2/src/type_inference_v2/tests.rs`; `backend/src/bin/lsp_server/server/core/tests.rs` |

## Review Notes

### 1. Stable identity is now a shared contract, not a review wish

Главный P1 risk из исходного review был в том, что identity structural member мог существовать
только неявно через name/span heuristics. Delivered state это устраняет:
- shared carrier содержит явный `StructuralMemberId`;
- legacy serialization path rehydrates identity instead of silently dropping it;
- ranking / dedup / adapter payloads не схлопывают distinct members по `label + owner` без identity.

### 2. Completion больше не держит hidden local truth

Исходный review был прав: completion path содержал риск local semantic reconstruction.
Теперь это закрыто в двух направлениях:
- при наличии shared owner hint completion идёт по тому же resolved path;
- если shared owner hint убрать из direct handler path, completion fail-closed и не восстанавливает owner/member эвристикой;
- actual `textDocument/completion` path по `FormModule.Объект.` продолжает работать через shared LSP owner-hint producer, а не за счёт runtime fallback.

### 3. Acceptance now checks what the review asked for

Evidence теперь включает не только smoke/parity, но и:
- exact member identity parity между runtime, `LSP` и `MCP`;
- same known/unknown policy for exact vs typo access;
- fail-closed negative path при удалении shared hint;
- revision-switch regressions, ловящие stale structural leakage;
- default-LSP acceptance для implicit `FormModule.Объект.`.

## Verdict

Residual semantic / edge-case risk matrix из production-readiness review закрыт:
- каждый бывший P1/P2 risk retired stronger shared-contract guarantee или direct automated evidence;
- незакрытого semantic gap, который сам по себе ломает honest readiness verdict, больше не осталось.
