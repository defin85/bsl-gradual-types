# Execution Matrix: refactor-ir-canonical-semantic-pipeline

## Scope

Матрица покрывает обязательные требования из:

- `openspec/changes/refactor-ir-canonical-semantic-pipeline/specs/bsl-intellisense-v2/spec.md`
- `openspec/changes/refactor-ir-canonical-semantic-pipeline/specs/mcp-bsl-agent/spec.md`

## Machine-readable validation artifacts

Authoritative checked-in machine-readable verdicts лежат рядом с change:

- `openspec/changes/refactor-ir-canonical-semantic-pipeline/validation/acceptance-report.json`
- `openspec/changes/refactor-ir-canonical-semantic-pipeline/validation/quality-gates.json`
- `openspec/changes/refactor-ir-canonical-semantic-pipeline/validation/contracts-compatibility-report.json`

Этот markdown остаётся human-readable companion matrix.

## Requirement -> Code Area -> Test Class

| Requirement | Primary code areas | Automated evidence |
| --- | --- | --- |
| Completion MUST читать semantic candidates только из current-revision canonical path и fail-closed при miss | `analysis-v2/src/type_inference_v2.rs`; `bsl-runtime/src/application/type_system/services/completion_service.rs`; `bsl-runtime/src/application/type_system/services/completion_service/context.rs`; `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs`; `backend/src/bin/lsp_server/handlers/completion.rs` | `backend/tests/metadata_completion_fixture_test.rs::metadata_completion_supports_documents_facets_and_tabular_sections`; `backend/tests/m8_completion_matrix_golden_v2_test.rs::m8_completion_matrix_golden_v2`; `backend/tests/lsp_completion_test.rs::completion_member_access_without_semantic_owner_stays_empty`; `bsl-runtime/src/application/type_system/services/completion_service/tests.rs::completion_non_member_semantic_path_ignores_polluted_index_snapshot` |
| Interactive answers after `didChange` MUST be exact for current revision or fail-closed | `analysis-v2/src/lib/analysis_api.rs`; `analysis-v2/src/lib/snapshots.rs`; `backend/src/bin/lsp_server/server/language_server/helpers.rs` | `analysis-v2/src/lib/tests.rs::current_type_index_serve_only_ready_rejects_fallback_snapshot_artifact`; `analysis-v2/src/lib/tests.rs::precompute_returns_superseded_when_expected_version_is_stale`; `backend/tests/lsp_incremental_completion_test.rs` |
| Canonical IR MUST be the single semantic source of truth for interactive IDE functions | `shared/src/ir/semantic_facts.rs`; `analysis-v2/src/type_inference_v2.rs`; `bsl-runtime/src/application/type_system/services/hover_service.rs`; `bsl-runtime/src/application/type_system/services/definition_service.rs`; `bsl-runtime/src/application/type_system/services/signature_help_service.rs` | `backend/tests/hover_property_access_test.rs::test_hover_on_property_name_works_with_empty_request_time_repository`; `backend/tests/goto_definition_common_module_test.rs::goto_definition_resolves_configuration_symbol_metadata_xml_from_semantic_facts_with_empty_consumer_repo`; `backend/tests/lsp_intellisense_tests.rs::lsp_signature_help_keeps_method_semantic_facts_with_empty_request_time_repository` |
| `derived semantic index` MUST be the only fast query artifact and MUST be built from one current IR snapshot | `analysis-v2/src/lib/snapshots.rs`; `analysis-v2/src/lib/analysis_api.rs`; `shared/src/ir/semantic_facts.rs`; `analysis-v2/src/type_inference_v2.rs` | `analysis-v2/src/type_inference_v2/tests.rs`; `analysis-v2/src/lib/tests.rs`; `backend/src/bin/lsp_server/server/core/tests.rs::p7_completion_owner_hint_type_lookup_is_serve_only_even_when_flow_sensitive_enabled` |
| Facet-aware identity MUST survive canonical pipeline and materialization | `analysis-v2/src/implicit_bindings.rs`; `analysis-v2/src/type_inference_v2.rs`; `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs`; `bsl-agent/src/session/manager_semantic_core.rs`; `cli/src/main.rs` | `analysis-v2/src/type_inference_v2/tests.rs`; `backend/tests/contextual_implicit_object_matrix_test.rs::implicit_object_bindings_are_contextual_across_module_types`; `backend/tests/form_module_object_unified_contract_test.rs::recordset_module_resolves_system_members_and_manager_path_call`; `bsl-agent/src/session/tests.rs::collect_type_at_position_preserves_available_facets_for_object_module_binding`; `bsl-agent/src/session/tests.rs::collect_type_at_position_preserves_available_facets_for_recordset_module_binding`; `cli/src/main.rs::cli_type_info_preserves_object_module_binding_facets` |
| Discovery/search `IndexSnapshot` MUST NOT become semantic source for interactive queries | `bsl-runtime/src/application/type_system/services/completion_service.rs`; `bsl-runtime/src/application/type_system/services/completion_service/scope_candidates.rs`; `backend/src/presentation/web/handlers.rs`; `bsl-agent/src/session/manager_semantic_core.rs`; `bsl-agent/src/session/manager_semantic_navigation.rs`; `cli/src/main.rs`; `shared/src/domain/metadata_lookup/search.rs` | `bsl-runtime/src/application/type_system/services/completion_service/tests.rs::completion_non_member_semantic_path_ignores_polluted_index_snapshot`; `backend/src/bin/lsp_server/server/core/tests.rs::p7_member_access_completion_does_not_backfill_from_runtime_index_snapshot`; `backend/tests/flow_sensitive_web_api_test.rs::hover_endpoints_do_not_backfill_from_polluted_search_index`; `bsl-agent/src/session/tests.rs::semantic_mcp_tools_do_not_backfill_from_polluted_search_index_on_default_path`; `cli/src/main.rs::cli_inline_completion_does_not_backfill_from_polluted_search_index` |
| Adapter surfaces MUST NOT reconstruct owner/member/type truth locally | `backend/src/bin/lsp_server/server/language_server/helpers.rs`; `backend/src/presentation/web/handlers.rs`; `backend/src/presentation/web/handlers/semantic.rs`; `backend/src/presentation/web/handlers/debug.rs`; `bsl-agent/src/session/helpers_semantic.rs`; `bsl-agent/src/session/manager_semantic_navigation.rs`; `cli/src/main.rs`; `cli/src/runtime.rs` | `backend/tests/lsp_completion_test.rs::completion_member_access_without_semantic_owner_stays_empty`; `backend/tests/goto_definition_fail_closed_test.rs::goto_definition_fails_closed_without_exact_type_index_artifact`; `backend/tests/flow_sensitive_web_api_test.rs::hover_endpoints_fail_closed_on_missing_canonical_artifacts`; `backend/tests/flow_sensitive_web_api_test.rs::hover_endpoints_do_not_backfill_from_polluted_search_index`; `backend/tests/flow_sensitive_web_api_test.rs::hover_endpoints_use_file_path_for_module_context_bindings`; `backend/tests/flow_sensitive_web_api_test.rs::diagnostics_and_validate_use_file_path_for_module_context_bindings`; `cli/src/main.rs::cli_inline_completion_uses_shared_runtime_snapshot`; `cli/src/main.rs::cli_inline_completion_does_not_backfill_from_polluted_search_index`; `cli/src/main.rs::cli_inline_type_info_uses_shared_runtime_snapshot`; `cli/src/main.rs::cli_file_diagnostics_use_shared_runtime_snapshot`; `bsl-agent/src/session/tests.rs::semantic_mcp_tools_do_not_backfill_from_polluted_search_index_on_default_path`; `bsl-agent/tests/stdio_integration.rs::stdio_members_fail_closed_on_current_revision_missing_owner_hint`; `bsl-agent/tests/stdio_integration.rs::stdio_type_at_position_returns_empty_on_current_revision_without_semantic_surface` |
| Interactive hover/signatureHelp/definition/type-at-position/members MUST fail-closed when canonical artifacts are unavailable | `bsl-runtime/src/application/intellisense_v2/facade.rs`; `analysis-v2/src/lib/analysis_api.rs`; `backend/src/presentation/web/handlers.rs`; `bsl-agent/src/session/manager_semantic_core.rs`; `bsl-agent/src/session/manager_semantic_navigation.rs` | `backend/tests/flow_sensitive_web_api_test.rs::hover_endpoints_fail_closed_on_missing_canonical_artifacts`; `backend/tests/goto_definition_fail_closed_test.rs::goto_definition_fails_closed_without_exact_type_index_artifact`; `backend/tests/intellisense_golden_completion_test.rs::golden_completion_member_access_without_semantic_owner_is_empty`; `bsl-agent/src/session/tests.rs::semantic_helpers_fail_closed_without_precomputed_type_index`; `bsl-agent/tests/stdio_integration.rs::stdio_members_fail_closed_on_current_revision_missing_owner_hint`; `bsl-agent/tests/stdio_integration.rs::stdio_type_at_position_returns_empty_on_current_revision_without_semantic_surface`; `bsl-agent/tests/stdio_integration.rs::stdio_definition_fail_closed_on_current_revision_unresolved_target` |
| Fail-closed observability MUST use bounded shared reason codes | `bsl-runtime/src/system/basic_observability/labels.rs`; `bsl-runtime/src/system/basic_observability/completion_metrics.rs`; `bsl-runtime/src/system/system_coordinator/coordinator/observability.rs`; `backend/src/presentation/web/handlers.rs`; `bsl-agent/src/session/manager_semantic_core.rs` | `backend/src/bin/lsp_server/server/core/tests.rs::p7_hover_cache_miss_emits_bounded_fail_closed_reason`; `backend/tests/flow_sensitive_web_api_test.rs::hover_endpoints_fail_closed_on_missing_canonical_artifacts`; `bsl-agent/src/session/tests.rs::type_at_position_members_and_definition_emit_shared_type_index_reason_metrics`; `bsl-runtime/src/system/basic_observability/tests.rs` |
| Latency regressions MUST be solved by canonical fast path optimization, not stale/degraded/search-backed rescue | `bsl-runtime/src/application/intellisense_v2/policy.rs`; `backend/src/perf_gate_evaluator.rs`; `backend/src/bin/intellisense_perf/run_helpers.rs`; `backend/src/bin/intellisense_perf/reporting.rs` | `contracts/intellisense-perf-gate/v2/contract.json`; `backend/tests/perf/baselines/intellisense_small.json`; `backend/tests/perf/baselines/intellisense_churn.json`; `backend/src/perf_gate_evaluator/tests.rs` |
| Applied-owner bare identifier fallback MUST stay removed; explicit `ЭтотОбъект` / `Объект` bindings MUST remain canonical | `analysis-v2/src/implicit_bindings.rs`; `analysis-v2/src/type_inference_v2.rs`; `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs` | `analysis-v2/src/type_inference_v2/tests.rs`; `backend/tests/form_module_object_unified_contract_test.rs::bare_owner_members_without_canonical_binding_stay_undeclared`; `backend/tests/contextual_implicit_object_matrix_test.rs::implicit_object_bindings_are_contextual_across_module_types`; `backend/tests/form_module_object_unified_contract_test.rs::diagnostics_hover_and_type_at_position_follow_unified_form_contract` |
| MCP semantic tools MUST use the same shared runtime rooted in canonical IR + derived index | `bsl-agent/src/session/helpers_semantic.rs`; `bsl-agent/src/session/manager_semantic_core.rs`; `bsl-agent/src/session/manager_semantic_navigation.rs`; `bsl-runtime/src/application/intellisense_v2/facade.rs` | `bsl-agent/src/session/tests.rs::collect_members_uses_exact_owner_hint_on_default_path`; `bsl-agent/src/session/tests.rs::bsl_members_does_not_execute_parse_result_query_on_semantic_path`; `bsl-agent/src/session/tests.rs::type_at_position_members_and_definition_emit_shared_type_index_reason_metrics`; `bsl-agent/tests/stdio_integration.rs::stdio_semantic_tools_happy_path_uses_current_revision_overlay`; `bsl-agent/tests/stdio_integration.rs::stdio_members_fail_closed_on_current_revision_missing_owner_hint`; `bsl-agent/tests/stdio_integration.rs::stdio_type_at_position_returns_empty_on_current_revision_without_semantic_surface`; `bsl-agent/tests/stdio_integration.rs::stdio_definition_fail_closed_on_current_revision_unresolved_target` |

## Closed implementation hotspots

1. `shared/src/ir/semantic_facts.rs` и `analysis-v2/src/type_inference_v2.rs` теперь materialize-ят canonical type/receiver/member/definition facts внутри одного IR snapshot; configuration symbols дополнительно несут canonical `metadata_path`.
2. `analysis-v2/src/lib/snapshots.rs` и `analysis-v2/src/lib/analysis_api.rs` публикуют exact current-revision semantic artifact из того же snapshot и fail-closed на stale/miss вместо stale/degraded substitute.
3. `bsl-runtime` shared services для `completion`, `hover`, `definition` и `signatureHelp` читают shared canonical facts на analyzed path и не восстанавливают semantic truth локально из текста или consumer-local repository state.
4. `backend`, `web`, `bsl-agent` и `cli` остались transport wrappers над shared runtime contract и общей bounded observability taxonomy; Web inline surfaces теперь используют checked-in `filePath`/`file_path` contract вместо synthetic path там, где нужен module context.
5. docs, execution matrix и machine-readable validation assets приведены к реально shipped contract и checked-in evidence после закрытия runtime gap `xgg.3`.

## Validation status

Состояние change считается синхронизированным, если одновременно верны все пункты:

- human-readable matrix указывает на реальные checked-in validation paths;
- machine-readable acceptance/gate assets живут под `openspec/changes/<change-id>/validation/`;
- shipped smoke entry point прогоняет `./scripts/run-intellisense-tests.sh smoke`, а perf/OpenSpec readiness gates живут в `./scripts/validate-v2-completion-gates.sh`;
- docs не описывают legacy AST-based hover/completion fallback как production path;
- web module-context evidence опирается только на path-aware surfaces: inline `hover`, `hover/enhanced`, `diagnostics` и `validate` принимают реальный `filePath`, а regression coverage фиксирует, что synthetic inline path больше не подменяет module owner;
- acceptance evidence включает `signatureHelp`, canonical `definition` anchors и metadata-backed member completion;
- `openspec validate refactor-ir-canonical-semantic-pipeline --strict --no-interactive` проходит без замечаний.

## Implemented acceptance slice

Representative checked-in evidence для этого change:

- `backend/src/bin/lsp_server/server/core/tests.rs::p7_typed_structure_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
- `backend/src/bin/lsp_server/server/core/tests.rs::p7_typed_value_table_row_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
- `backend/tests/metadata_completion_fixture_test.rs::metadata_completion_supports_documents_facets_and_tabular_sections`
- `backend/tests/m8_completion_matrix_golden_v2_test.rs::m8_completion_matrix_golden_v2`
- `backend/tests/flow_sensitive_web_api_test.rs::hover_endpoints_emit_type_index_reason_metrics`
- `backend/tests/flow_sensitive_web_api_test.rs::hover_endpoints_fail_closed_on_missing_canonical_artifacts`
- `backend/tests/flow_sensitive_web_api_test.rs::hover_endpoints_do_not_backfill_from_polluted_search_index`
- `backend/tests/flow_sensitive_web_api_test.rs::hover_endpoints_use_file_path_for_module_context_bindings`
- `backend/tests/flow_sensitive_web_api_test.rs::diagnostics_and_validate_use_file_path_for_module_context_bindings`
- `backend/tests/form_module_object_unified_contract_test.rs::diagnostics_hover_and_type_at_position_follow_unified_form_contract`
- `backend/tests/form_module_object_unified_contract_test.rs::completion_and_resolve_follow_unified_form_contract`
- `backend/tests/form_module_object_unified_contract_test.rs::recordset_module_resolves_system_members_and_manager_path_call`
- `backend/tests/form_module_object_unified_contract_test.rs::bare_owner_members_without_canonical_binding_stay_undeclared`
- `backend/tests/hover_property_access_test.rs::test_hover_on_property_name_works_with_empty_request_time_repository`
- `backend/tests/lsp_intellisense_tests.rs::lsp_signature_help_keeps_method_semantic_facts_with_empty_request_time_repository`
- `backend/tests/lsp_intellisense_tests.rs::lsp_signature_help_uses_semantic_facts_for_local_function_with_empty_request_time_repository`
- `backend/tests/lsp_intellisense_tests.rs::lsp_signature_help_uses_semantic_facts_for_global_function_with_empty_request_time_repository`
- `backend/tests/goto_definition_common_module_test.rs::goto_definition_resolves_common_module_receiver_from_semantic_facts_with_empty_consumer_repo`
- `backend/tests/goto_definition_common_module_test.rs::goto_definition_resolves_configuration_symbol_metadata_xml_from_semantic_facts_with_empty_consumer_repo`
- `cli/src/main.rs::cli_inline_completion_uses_shared_runtime_snapshot`
- `cli/src/main.rs::cli_inline_completion_does_not_backfill_from_polluted_search_index`
- `cli/src/main.rs::cli_inline_type_info_uses_shared_runtime_snapshot`
- `cli/src/main.rs::cli_type_info_preserves_object_module_binding_facets`
- `cli/src/main.rs::cli_file_diagnostics_use_shared_runtime_snapshot`
- `bsl-agent/src/session/tests.rs::type_at_position_members_and_definition_emit_shared_type_index_reason_metrics`
- `bsl-agent/src/session/tests.rs::semantic_mcp_tools_do_not_backfill_from_polluted_search_index_on_default_path`
- `bsl-agent/src/session/tests.rs::collect_type_at_position_preserves_available_facets_for_recordset_module_binding`
- `bsl-agent/tests/stdio_integration.rs::stdio_semantic_tools_happy_path_uses_current_revision_overlay`
- `bsl-agent/tests/stdio_integration.rs::stdio_members_fail_closed_on_current_revision_missing_owner_hint`
- `bsl-agent/tests/stdio_integration.rs::stdio_type_at_position_returns_empty_on_current_revision_without_semantic_surface`
- `bsl-agent/tests/stdio_integration.rs::stdio_definition_fail_closed_on_current_revision_unresolved_target`
- `analysis-v2/src/lib/tests.rs::precompute_returns_superseded_when_expected_version_is_stale`
- `openspec validate refactor-ir-canonical-semantic-pipeline --strict --no-interactive`
