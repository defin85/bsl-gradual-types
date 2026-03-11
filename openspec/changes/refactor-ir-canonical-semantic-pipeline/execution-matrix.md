# Execution Matrix: refactor-ir-canonical-semantic-pipeline

## Scope

Матрица покрывает обязательные требования из:
- `openspec/changes/refactor-ir-canonical-semantic-pipeline/specs/bsl-intellisense-v2/spec.md`
- `openspec/changes/refactor-ir-canonical-semantic-pipeline/specs/mcp-bsl-agent/spec.md`

## Machine-readable validation artifacts

- `validation/acceptance-report.json` - per fixture / per operation / per consumer acceptance verdicts с привязкой к automated assets.
- `validation/quality-gates.json` - stacked gate summary для acceptance, contracts, observability, perf и docs sync.
- `validation/contracts-compatibility-report.json` - machine-readable output compatibility diff gate against `origin/master`.

Этот markdown остаётся human-readable companion matrix. Authoritative machine-readable verdicts живут в `validation/*.json`.

## Requirement -> Code Area -> Test Class

| Requirement | Primary code areas | Automated evidence |
| --- | --- | --- |
| Completion MUST читать semantic candidates только из current-revision canonical path и fail-closed при miss | `backend/src/bin/lsp_server/server/language_server/helpers.rs`, `backend/src/bin/lsp_server/handlers/completion.rs`, `backend/src/bin/lsp_server/server/language_server/impl_completion_helpers.rs`, `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`, `bsl-runtime/src/application/intellisense_v2/policy.rs` | `backend/src/bin/lsp_server/server/core/tests.rs`, `backend/tests/lsp_incremental_completion_test.rs`, `bsl-runtime/src/application/intellisense_v2/facade/tests.rs` |
| Interactive answers after `didChange` MUST be exact for current revision or fail-closed | `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`, `backend/src/bin/lsp_server/server/language_server/helpers.rs`, `analysis-v2/src/lib/analysis_api.rs` | `backend/src/bin/lsp_server/server/core/tests.rs`, `bsl-runtime/src/application/intellisense_v2/facade/tests.rs`, `analysis-v2/src/lib/tests.rs` |
| Canonical IR MUST be the single semantic source of truth for interactive IDE functions | `analysis-v2/src/lib/snapshots.rs`, `analysis-v2/src/lib/analysis_api.rs`, `analysis-v2/src/type_inference_v2.rs`, `bsl-runtime/src/application/type_system/services/hover_service.rs`, `bsl-runtime/src/application/type_system/services/definition_service.rs` | `analysis-v2/src/lib/tests.rs`, `backend/src/bin/lsp_server/server/core/tests.rs`, `backend/tests/universal_collection_cross_consumer_consistency_test.rs` |
| `derived semantic index` MUST be the only fast query artifact and MUST be built from one current IR snapshot | `analysis-v2/src/lib/analysis_api.rs`, `analysis-v2/src/lib/snapshots.rs`, `analysis-v2/src/derived_artifacts.rs`, `analysis-v2/src/type_inference_v2.rs` | `analysis-v2/src/lib/tests.rs`, `analysis-v2/src/derived_artifacts/tests.rs`, `backend/src/bin/lsp_server/server/core/tests.rs` |
| Facet-aware identity MUST survive canonical pipeline and materialization | `analysis-v2/src/implicit_bindings.rs`, `analysis-v2/src/type_inference_v2.rs`, `shared/src/domain/metadata_lookup/facets.rs`, `bsl-agent/src/session/manager_semantic_core.rs` | `analysis-v2/src/type_inference_v2/tests.rs`, `analysis-v2/src/implicit_bindings/tests.rs`, `bsl-agent/src/session/tests.rs`, `backend/tests/conf_big_document_realization_tovarov_uslug_matrix_test.rs` |
| Discovery/search `IndexSnapshot` MUST NOT become semantic source for interactive queries | `backend/src/bin/lsp_server/handlers/completion.rs`, `bsl-runtime/src/application/type_system/services/completion_service.rs`, `bsl-runtime/src/system/intellisense_index.rs`, `bsl-agent/src/session/manager_semantic_core.rs` | `backend/src/bin/lsp_server/server/core/tests.rs`, `bsl-agent/src/session/tests.rs` |
| Adapter surfaces MUST NOT reconstruct owner/member/type truth locally | `backend/src/bin/lsp_server/server/language_server/impl_features_b.rs`, `backend/src/bin/lsp_server/server/language_server/impl_features_c.rs`, `backend/src/presentation/web/handlers.rs`, `backend/src/presentation/web/handlers/semantic.rs`, `bsl-agent/src/session/helpers_semantic.rs`, `bsl-agent/src/session/manager_semantic_navigation.rs`, `cli/src/main.rs`, `cli/src/runtime.rs` | `backend/src/bin/lsp_server/server/core/tests.rs`, `bsl-agent/src/session/tests.rs`, `cli/src/main.rs`, `cli/src/runtime.rs` |
| Interactive hover/signatureHelp/definition/type-at-position/members MUST fail-closed when canonical artifacts are unavailable | `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`, `analysis-v2/src/lib/analysis_api.rs`, `backend/src/bin/lsp_server/server/language_server/impl_features_b.rs`, `backend/src/bin/lsp_server/server/language_server/impl_features_c.rs`, `bsl-agent/src/session/manager_semantic_core.rs`, `bsl-agent/src/session/manager_semantic_navigation.rs` | `analysis-v2/src/lib/tests.rs`, `backend/src/bin/lsp_server/server/core/tests.rs`, `bsl-agent/src/session/tests.rs` |
| Fail-closed observability MUST use bounded shared reason codes | `bsl-runtime/src/system/basic_observability/labels.rs`, `bsl-runtime/src/system/basic_observability/completion_metrics.rs`, `bsl-runtime/src/system/system_coordinator/coordinator/observability.rs`, `analysis-v2/src/lib/analysis_api.rs` | `analysis-v2/src/lib/tests.rs`, `backend/src/bin/lsp_server/server/core/tests.rs`, `bsl-runtime/src/system/basic_observability/tests.rs` |
| Latency regressions MUST be solved by canonical fast path optimization, not stale/degraded/search-backed rescue | `bsl-runtime/src/application/intellisense_v2/policy.rs`, `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`, `analysis-v2/src/lib/analysis_api.rs` | `backend/src/bin/lsp_server/server/core/tests.rs`, `backend/tests/intellisense_v2_conf_big_perf_regression_test.rs`, `bsl-runtime/src/application/intellisense_v2/facade/tests.rs` |
| Applied-owner bare identifier fallback MUST stay removed; explicit `ЭтотОбъект` / `Объект` bindings MUST remain canonical | `analysis-v2/src/implicit_bindings.rs`, `analysis-v2/src/ast_to_ir/converter.rs`, `analysis-v2/src/type_inference_v2.rs`, `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs` | `analysis-v2/src/type_inference_v2/tests.rs`, `analysis-v2/src/implicit_bindings/tests.rs`, `backend/tests/contextual_implicit_object_matrix_test.rs`, `backend/tests/undeclared_variable_test.rs`, `bsl-agent/src/session/tests.rs` |
| MCP semantic tools MUST use the same shared runtime rooted in canonical IR + derived index | `bsl-agent/src/session/manager_semantic_core.rs`, `bsl-agent/src/session/manager_semantic_navigation.rs`, `bsl-agent/src/session/helpers_semantic.rs`, `bsl-runtime/src/application/intellisense_v2/facade/operations.rs` | `bsl-agent/src/session/tests.rs`, `backend/src/bin/lsp_server/server/core/tests.rs`, `bsl-agent/tests/stdio_integration.rs` |

## Closed implementation hotspots

1. `shared/src/ir/semantic_facts.rs`, `shared/src/ir/program.rs`, `analysis-v2/src/ast_to_ir/converter.rs` и `analysis-v2/src/type_inference_v2.rs` теперь материализуют semantic facts в canonical IR и строят exact semantic index как projection того же snapshot без повторного inference по projected `Program`.
2. `analysis-v2/src/lib/snapshots.rs` и `analysis-v2/src/lib/analysis_api.rs` обслуживают diagnostics и interactive queries из IR-derived facts текущей revision; stale parse snapshot больше не может подменять current text при semantic extraction.
3. `bsl-agent/src/session/helpers_semantic.rs`, `bsl-agent/src/session/manager_semantic_core.rs`, `bsl-agent/src/session/manager_semantic_navigation.rs`, `backend/src/presentation/web/handlers/semantic.rs` и `backend/src/presentation/web/handlers.rs` убрали adapter-local rescue/precompute и используют shared bounded type-index reason taxonomy на default path.
4. `cli/src/main.rs` и `cli/src/runtime.rs` выполняют `check`, `complete`, `info` и `analyze-ir` через shared prepared snapshot/runtime contract вместо adapter-local `AnalysisHostV2` path.
5. `shared/src/domain/metadata_lookup/generic.rs` наследует signature-index methods для generic platform collections даже без raw type payload, чтобы canonical member truth не зависел от transport-specific metadata shape.
6. `contracts/lsp-completion-v2/v2/`, `contracts/lsp-completion-timeline/v2/`, `contracts/observability-completion-v2/v2/`, `docs/guides/lsp-v2-latency-policy.md`, `backend/src/perf_gate_evaluator.rs` и `backend/src/bin/intellisense_perf/reporting.rs` выровнены под fail-closed cutover без stale/degraded contract drift.

## Validation status

Обязательный runtime gap закрыт:
- exact semantic index больше не строится из `parse_result.program`; canonical semantic facts живут в `SemanticProgram` и материализуются в derived index из того же IR snapshot.
- MCP/Web/LSP/CLI используют один shared semantic runtime contract и одну bounded observability taxonomy на default path.
- checked-in contracts, docs и perf/acceptance gates больше не объявляют stale/degraded substitute допустимым semantic поведением.

## Implemented acceptance slice

В этой сессии зафиксированы и прогнаны ключевые acceptance-доказательства:
- `analysis-v2` tests подтверждают IR-derived materialization, current-text correctness и сохранение explicit/facet-aware semantics.
- `bsl-agent` tests подтверждают receiver-hint parity для `definition`, exact owner-hint wiring для `members` и shared type-index reason metrics для `type_at_position` / `members` / `definition` (`collect_members_uses_exact_owner_hint_on_default_path`, `bsl_members_does_not_execute_parse_result_query_on_semantic_path`).
- `backend` web/LSP acceptance tests подтверждают fail-closed observability, scale-aware perf gates и отсутствие adapter-local semantic rescue path (`p7_member_access_completion_does_not_backfill_from_runtime_index_snapshot`, `p7_completion_owner_hint_type_lookup_is_serve_only_even_when_flow_sensitive_enabled`, `hover_endpoints_fail_closed_on_missing_canonical_artifacts`).
- `cli` tests подтверждают, что transport wrapper не строит alternate semantic path для `complete`, `info` и `check` (`cli_inline_completion_uses_shared_runtime_snapshot`, `cli_inline_type_info_uses_shared_runtime_snapshot`, `cli_file_diagnostics_use_shared_runtime_snapshot`, `prepare_cli_file_operation_uses_runtime_contract_for_diagnostics`).
- `bsl-runtime` completion-service tests подтверждают, что polluted discovery/search snapshot не backfill-ит semantic member-access miss (`completion_unknown_bare_receiver_member_access_ignores_polluted_index_snapshot`).
- versioned contracts и perf-gate architecture checks подтверждают, что shipped operational surfaces соответствуют runtime cutover.
- `openspec validate refactor-ir-canonical-semantic-pipeline --strict --no-interactive` проходит без замечаний.
