# Residual Risk Review: update-gradual-core-production-readiness

Дата: 2026-03-08  
Change: `update-gradual-core-production-readiness`  
Task: `bsl-gradual-types-6mx.5`

## Scope

Этот артефакт закрывает только residual semantic / edge-case risk matrix из production-readiness review.

Он **не** является финальным verdict `complete/ready` для всего change:
- readiness gate и machine-readable governance artefacts остаются scope `bsl-gradual-types-6mx.4`;
- refresh traceability / final closure wording остаётся scope `bsl-gradual-types-6mx.6`.

## Residual Risk Matrix

| Risk | Previous review concern | Current status | Stronger guarantee / code | Automated evidence |
| --- | --- | --- | --- | --- |
| Stable structural member identity | Structural members сравнивались слишком lossy и не имели consumer-independent key | `retired` | Shared contract теперь несёт explicit `StructuralMemberId` и rehydrates legacy payload без потери identity: `bsl-types/src/types/structural_members.rs:21`, `bsl-types/src/types/structural_members.rs:102`. Completion ranking/dedup/order тоже учитывает `member_identity`: `bsl-runtime/src/application/type_system/services/completion_ranking.rs:29`, `bsl-runtime/src/application/type_system/services/completion_ranking.rs:176`, `bsl-runtime/src/application/type_system/services/completion_ranking.rs:320`. LSP transport публикует identity в completion payload: `backend/src/bin/lsp_server/handlers/completion.rs:207`. | `bsl-types/src/types/tests/structural_members_tests.rs:47`; `bsl-types/src/types/tests/structural_members_tests.rs:97`; `analysis-v2/src/type_inference_v2/tests.rs:1584`; `analysis-v2/src/type_inference_v2/tests.rs:1709`; `backend/src/bin/lsp_server/handlers/completion/tests.rs:324`; `backend/src/bin/lsp_server/server/core/tests.rs:4300`; `backend/src/bin/lsp_server/server/core/tests.rs:4459` |
| Hidden completion-local reconstruction | Completion мог оставаться “умнее” других consumers за счёт local owner/member reconstruction | `retired for typed structure / typed-row paths under review` | Completion получает shared owner hint и передаёт его в unified completion path: `backend/src/bin/lsp_server/handlers/completion.rs:169`. Member resolution сначала читает shared owner hint и only then допускает bootstrap-only fallback для implicit module-context symbols, не для structural truth: `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs:77`. | `backend/tests/universal_collection_cross_consumer_consistency_test.rs:315`; `backend/tests/universal_collection_cross_consumer_consistency_test.rs:398`; `backend/src/bin/lsp_server/server/core/tests.rs:4300`; `backend/src/bin/lsp_server/server/core/tests.rs:4459` |
| Exact acceptance did not prove identity / hidden-hint failure | Happy-path parity не доказывал same member identity и fail-closed behaviour | `retired` | Cross-consumer acceptance теперь сравнивает runtime structural member identity против LSP completion и MCP members, плюс same type/policy verdict: `backend/src/bin/lsp_server/server/core/tests.rs:4300`, `backend/src/bin/lsp_server/server/core/tests.rs:4459`. LSP candidate payload сохраняет `member_identity`: `backend/src/bin/lsp_server/handlers/completion.rs:397`. | `backend/src/bin/lsp_server/server/core/tests.rs:4300`; `backend/src/bin/lsp_server/server/core/tests.rs:4459`; `backend/tests/universal_collection_cross_consumer_consistency_test.rs:315`; `backend/tests/universal_collection_cross_consumer_consistency_test.rs:398`; `backend/src/bin/lsp_server/handlers/completion/tests.rs:324` |
| Lifecycle / revision leakage and edge cases | Case-insensitive replace/update, branch merges и snapshot switch могли ломать identity или протекать между interfaces | `retired` | Shared structural carrier сохраняет stable identity при replace/update; analysis materialization and merge paths поддерживают snapshot-local continuity: `bsl-types/src/types/structural_members.rs:71`. Revision switch exact checks подтверждают fail-closed behaviour across runtime / LSP / MCP / Web diagnostics: `backend/src/bin/lsp_server/server/core/tests.rs:4622`, `backend/src/bin/lsp_server/server/core/tests.rs:4698`. | `analysis-v2/src/type_inference_v2/tests.rs:1616`; `analysis-v2/src/type_inference_v2/tests.rs:1742`; `analysis-v2/src/type_inference_v2/tests.rs:1828`; `analysis-v2/src/type_inference_v2/tests.rs:1886`; `analysis-v2/src/type_inference_v2/tests.rs:1968`; `backend/src/bin/lsp_server/server/core/tests.rs:4622`; `backend/src/bin/lsp_server/server/core/tests.rs:4698` |

## Review Notes

### 1. Stable identity is now a shared contract, not a review wish

Главный P1 risk из исходного review был в том, что identity structural member мог существовать только неявно через name/span heuristics. После `6mx.1` и связанных follow-ups это больше не так:
- shared carrier содержит явный `StructuralMemberId`;
- legacy serialization path rehydrates identity instead of silently dropping it;
- ranking / dedup / adapter payloads не схлопывают distinct members по `label + owner` без identity.

Это переводит риск из категории `open semantic gap` в `retired by stronger shared-contract guarantee`.

### 2. Structural completion no longer depends on a hidden local truth

Исходный review был прав: completion path содержал риск local semantic reconstruction. Для typed `Структура` и typed-row этот риск теперь закрыт именно как risk matrix item:
- при наличии shared owner hint completion идёт по тому же resolved path;
- если shared owner hint убрать из direct handler path, completion fail-closed и не восстанавливает structural member эвристикой.

Оставшийся bootstrap fallback в `completion_service/member_resolution.rs` относится к implicit module-context symbols и явно помечен как transitional path. Он не снимает closed verdict для structural/typed-row residual risks, потому что не создаёт отдельную structural truth и не проходит через covered review scenarios.

### 3. Exact acceptance now checks what the review asked for

Теперь evidence включает не только smoke/parity, но и:
- exact member identity parity между runtime, `LSP` и `MCP`;
- same known/unknown policy for exact vs typo access;
- fail-closed negative path, когда shared hint intentionally removed;
- revision-switch regressions, которые ловят stale structural leakage across interfaces.

Именно этот набор закрывает P1/P2 review concerns, из-за которых final readiness verdict раньше был только `partial`.

## Verdict For `6mx.5`

Residual semantic / edge-case risk matrix из production-readiness review закрыт:
- каждый бывший P1/P2 risk либо retired stronger shared-contract guarantee, либо mapped to direct automated evidence;
- незакрытого edge-case gap, который сам по себе ломает honest semantic readiness verdict, больше не осталось.

Остающийся незавершённый scope по change:
- process/governance readiness gate (`6mx.4`);
- финальный refresh traceability / review / closure wording (`6mx.6`).
