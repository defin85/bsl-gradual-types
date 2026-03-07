# Cross-Owner Architecture Review

Дата: 2026-03-07  
Change: `add-v2-universal-collection-schema-resolution`

## Scope

Проверка согласованности реализации и guardrails по доменам:
- `analysis-v2`
- `completion`
- `diagnostics`
- `metadata_lookup`

## Owner Review Notes

### analysis-v2
- Snapshot-local effect store остаётся source of truth для `Соответствие` / `Структура` / `ТаблицаЗначений`:
  - `analysis-v2/src/type_inference_v2.rs`
  - `analysis-v2/src/type_inference_v2/instance_effects.rs`
- Нет production-path мутаций global `TypeRepository`; это дополнительно защищено repository guardrails.
- Exact evidence:
  - `analysis-v2/src/type_inference_v2/tests.rs`
    - `universal_collection_effects_do_not_mutate_type_repository`
    - `typed_structure_alias_keeps_structural_members`
    - `value_table_add_row_materializes_typed_row_members`
  - `analysis-v2/src/lib/tests.rs`
    - `serve_only_matches_legacy_for_universal_collections_in_complete_snapshot`
    - `serve_only_matches_legacy_for_universal_collections_with_incomplete_member_access`
- Вердикт: **accepted**.

### completion
- Completion использует общий resolved owner/type contract и exact owner derivation для index/member access:
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs`
  - `syntax/src/tree_sitter_adapter/expression_converter.rs`
- Exact evidence:
  - `backend/src/bin/lsp_server/server/core/tests.rs`
    - `p7_map_index_access_exact_cross_consumer_acceptance_uses_snapshot_owner_without_manual_hint`
    - `p7_dynamic_map_key_exact_cross_consumer_acceptance_uses_safe_policy_without_unknown_key`
  - `syntax/src/lib/tests.rs`
    - `parse_incomplete_member_access_preserves_receiver_expression`
- Вердикт: **accepted**.

### diagnostics
- Semantic diagnostics используют тот же resolved owner/type path и strict policy для typed-structure / typed-row:
  - `semantic-diagnostics/src/visitor.rs`
  - `shared/src/domain/metadata_lookup/core.rs`
- Exact evidence:
  - `backend/tests/universal_collection_strict_policy_test.rs`
    - `dynamic_map_key_uses_safe_policy_without_unknown_key_diagnostic`
    - `typed_structure_unknown_field_emits_non_existent_property_diagnostic`
    - `typed_value_table_row_unknown_column_emits_non_existent_property_diagnostic`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
    - `p7_typed_structure_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
    - `p7_typed_value_table_row_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
- Вердикт: **accepted**.

### metadata_lookup
- Общий слой metadata lookup / validation поддерживает structural members без repository mutation:
  - `shared/src/domain/metadata_lookup/core.rs`
  - `bsl-types/src/types/structural_members.rs`
- Exact evidence:
  - `shared/src/domain/metadata_lookup/tests.rs`
    - `test_get_properties_includes_structural_members_before_repository_fallback`
    - `test_has_member_finds_structural_property_case_insensitively`
    - `test_get_property_origin_tag_marks_structural_members`
  - `shared/src/domain/validators/tests.rs`
    - `test_validate_property_exists_accepts_structural_member`
    - `test_validate_property_exists_reports_unknown_structural_member`
  - `bsl-types/src/types/tests/structural_members_tests.rs`
    - `test_resolution_preserves_structural_member_contract`
    - `test_find_structural_member_is_case_insensitive`
- Вердикт: **accepted**.

## Verification Artifacts

- `openspec/changes/add-v2-universal-collection-schema-resolution/review-gate-option-b.md`
- `openspec/changes/add-v2-universal-collection-schema-resolution/traceability.md`
- `backend/tests/intellisense_v2_scale_aware_gate_contract_test.rs`
- `backend/src/perf_gate_evaluator/tests.rs`
- `openspec validate add-v2-universal-collection-schema-resolution --strict --no-interactive`

## Decision

- Архитектурный review по владельческим зонам завершён.
- Change совместим с зафиксированным Option B contract и его always-on observability/perf-gate rollout model.
- На дату review незакрытых MUST evidence gaps для change не осталось.
