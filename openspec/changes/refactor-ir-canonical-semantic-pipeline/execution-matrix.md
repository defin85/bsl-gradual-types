# Execution Matrix: refactor-ir-canonical-semantic-pipeline

## Scope

Матрица покрывает обязательные требования из:
- `openspec/changes/refactor-ir-canonical-semantic-pipeline/specs/bsl-intellisense-v2/spec.md`
- `openspec/changes/refactor-ir-canonical-semantic-pipeline/specs/mcp-bsl-agent/spec.md`

## Requirement -> Code Area -> Test Class

| Requirement | Primary code areas | Automated evidence |
| --- | --- | --- |
| Completion MUST читать semantic candidates только из current-revision canonical path и fail-closed при miss | `backend/src/bin/lsp_server/server/language_server/helpers.rs`, `backend/src/bin/lsp_server/handlers/completion.rs`, `backend/src/bin/lsp_server/server/language_server/impl_completion_helpers.rs`, `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`, `bsl-runtime/src/application/intellisense_v2/policy.rs` | `backend/src/bin/lsp_server/server/core/tests.rs`, `backend/tests/lsp_incremental_completion_test.rs`, `bsl-runtime/src/application/intellisense_v2/facade/tests.rs` |
| Interactive answers after `didChange` MUST be exact for current revision or fail-closed | `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`, `backend/src/bin/lsp_server/server/language_server/helpers.rs`, `analysis-v2/src/lib/analysis_api.rs` | `backend/src/bin/lsp_server/server/core/tests.rs`, `bsl-runtime/src/application/intellisense_v2/facade/tests.rs`, `analysis-v2/src/lib/tests.rs` |
| Canonical IR MUST be the single semantic source of truth for interactive IDE functions | `analysis-v2/src/lib/snapshots.rs`, `analysis-v2/src/lib/analysis_api.rs`, `analysis-v2/src/type_inference_v2.rs`, `bsl-runtime/src/application/type_system/services/hover_service.rs`, `bsl-runtime/src/application/type_system/services/definition_service.rs` | `analysis-v2/src/lib/tests.rs`, `backend/src/bin/lsp_server/server/core/tests.rs`, `backend/tests/universal_collection_cross_consumer_consistency_test.rs` |
| `derived semantic index` MUST be the only fast query artifact and MUST be built from one current IR snapshot | `analysis-v2/src/lib/analysis_api.rs`, `analysis-v2/src/lib/snapshots.rs`, `analysis-v2/src/derived_artifacts.rs`, `analysis-v2/src/type_inference_v2.rs` | `analysis-v2/src/lib/tests.rs`, `analysis-v2/src/derived_artifacts/tests.rs`, `backend/src/bin/lsp_server/server/core/tests.rs` |
| Facet-aware identity MUST survive canonical pipeline and materialization | `analysis-v2/src/implicit_bindings.rs`, `analysis-v2/src/type_inference_v2.rs`, `shared/src/domain/metadata_lookup/facets.rs`, `bsl-agent/src/session/manager_semantic_core.rs` | `analysis-v2/src/type_inference_v2/tests.rs`, `analysis-v2/src/implicit_bindings/tests.rs`, `bsl-agent/src/session/tests.rs`, `backend/tests/conf_big_document_realization_tovarov_uslug_matrix_test.rs` |
| Discovery/search `IndexSnapshot` MUST NOT become semantic source for interactive queries | `backend/src/bin/lsp_server/handlers/completion.rs`, `bsl-runtime/src/application/type_system/services/completion_service.rs`, `bsl-runtime/src/system/intellisense_index.rs`, `bsl-agent/src/session/manager_semantic_core.rs` | `backend/src/bin/lsp_server/server/core/tests.rs`, `bsl-agent/src/session/tests.rs` |
| Adapter surfaces MUST NOT reconstruct owner/member/type truth locally | `backend/src/bin/lsp_server/server/language_server/impl_features_b.rs`, `backend/src/bin/lsp_server/server/language_server/impl_features_c.rs`, `backend/src/presentation/web/handlers.rs`, `backend/src/presentation/web/handlers/semantic.rs`, `bsl-agent/src/session/helpers_semantic.rs`, `bsl-agent/src/session/manager_semantic_navigation.rs` | `backend/src/bin/lsp_server/server/core/tests.rs`, `bsl-agent/src/session/tests.rs` |
| Interactive hover/signatureHelp/definition/type-at-position/members MUST fail-closed when canonical artifacts are unavailable | `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`, `analysis-v2/src/lib/analysis_api.rs`, `backend/src/bin/lsp_server/server/language_server/impl_features_b.rs`, `backend/src/bin/lsp_server/server/language_server/impl_features_c.rs`, `bsl-agent/src/session/manager_semantic_core.rs`, `bsl-agent/src/session/manager_semantic_navigation.rs` | `analysis-v2/src/lib/tests.rs`, `backend/src/bin/lsp_server/server/core/tests.rs`, `bsl-agent/src/session/tests.rs` |
| Fail-closed observability MUST use bounded shared reason codes | `bsl-runtime/src/system/basic_observability/labels.rs`, `bsl-runtime/src/system/basic_observability/completion_metrics.rs`, `bsl-runtime/src/system/system_coordinator/coordinator/observability.rs`, `analysis-v2/src/lib/analysis_api.rs` | `analysis-v2/src/lib/tests.rs`, `backend/src/bin/lsp_server/server/core/tests.rs`, `bsl-runtime/src/system/basic_observability/tests.rs` |
| Latency regressions MUST be solved by canonical fast path optimization, not stale/degraded/search-backed rescue | `bsl-runtime/src/application/intellisense_v2/policy.rs`, `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`, `analysis-v2/src/lib/analysis_api.rs` | `backend/src/bin/lsp_server/server/core/tests.rs`, `backend/tests/intellisense_v2_conf_big_perf_regression_test.rs`, `bsl-runtime/src/application/intellisense_v2/facade/tests.rs` |
| Applied-owner bare identifier fallback MUST stay removed; explicit `ЭтотОбъект` / `Объект` bindings MUST remain canonical | `analysis-v2/src/implicit_bindings.rs`, `analysis-v2/src/ast_to_ir/converter.rs`, `analysis-v2/src/type_inference_v2.rs`, `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs` | `analysis-v2/src/type_inference_v2/tests.rs`, `analysis-v2/src/implicit_bindings/tests.rs`, `backend/tests/contextual_implicit_object_matrix_test.rs`, `backend/tests/undeclared_variable_test.rs`, `bsl-agent/src/session/tests.rs` |
| MCP semantic tools MUST use the same shared runtime rooted in canonical IR + derived index | `bsl-agent/src/session/manager_semantic_core.rs`, `bsl-agent/src/session/manager_semantic_navigation.rs`, `bsl-agent/src/session/helpers_semantic.rs`, `bsl-runtime/src/application/intellisense_v2/facade/operations.rs` | `bsl-agent/src/session/tests.rs`, `backend/src/bin/lsp_server/server/core/tests.rs`, `bsl-agent/tests/stdio_integration.rs` |

## Immediate hotspots for code changes

1. `analysis-v2/src/lib/snapshots.rs:612` строит `type_index` из `parse_result.program`, а не из IR-derived snapshot.
2. `analysis-v2/src/lib/analysis_api.rs:190` и `analysis-v2/src/lib/analysis_api.rs:769` продолжают обслуживать interactive type queries через `type_index`.
3. `backend/src/bin/lsp_server/server/language_server/impl_features_b.rs:117`, `backend/src/bin/lsp_server/server/language_server/impl_features_b.rs:470`, `backend/src/bin/lsp_server/server/language_server/impl_features_c.rs:120`, `backend/src/bin/lsp_server/server/language_server/impl_completion_helpers.rs:353` используют `serve_only` как shared fast path.
4. `backend/src/presentation/web/handlers.rs:102` и `backend/src/presentation/web/handlers/semantic.rs:59` явно прогревают `type_index` перед hover.
5. `bsl-agent/src/session/helpers_semantic.rs:102` и `bsl-agent/src/session/helpers_semantic.rs:135` используют `flow_type_at_byte_offset` / `type_at_byte_offset_serve_only` для MCP `type_at_position` и `members`.

## Current validation gap

Матрица фиксирует покрытие, но один ключевой architectural gap ещё остаётся в runtime:
- `type_index` пока строится от `parse_result.program`, а не как projection от canonical IR.

## Implemented slice

Зафиксированный в этой сессии runtime slice:
- `analysis-v2::flow_type_at_byte_offset` больше не выполняет синхронный `type_index` / `parse_result` rescue и берёт base type только через exact `serve_only` artifact.
- `semantic-diagnostics` теперь эмитит `UndeclaredVariable` для RHS присваивания, когда canonical semantic hints сообщают undeclared value.
- backend contract tests закрепляют explicit-binding-only semantics для bare owner members вне `FormModule`.
