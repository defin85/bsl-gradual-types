# Traceability: update-gradual-core-production-readiness

Этот артефакт фиксирует прямую трассировку `Requirement -> Code -> Test`
для финального production-readiness состояния после закрытия epic
`bsl-gradual-types-b6q`.

## Requirement: Shared resolved contract first-class выражает snapshot-local structural members

Status: `covered`

Code:
- `bsl-types/src/types/certainty.rs`
- `bsl-types/src/types/structural_members.rs`
- `bsl-types/src/types/resolution_impl/structural_members.rs`
- `analysis-v2/src/type_inference_v2/instance_effects.rs`
- `analysis-v2/src/type_inference_v2.rs`

Tests:
- `bsl-types/src/types/tests/structural_members_tests.rs`
  - `test_resolution_preserves_structural_member_contract`
  - `test_replacing_same_structural_member_preserves_member_id`
  - `test_structural_member_contract_roundtrips_member_id_through_serde`
  - `test_structural_member_contract_rehydrates_member_id_from_legacy_payload`
- `analysis-v2/src/type_inference_v2/tests.rs`
  - `typed_structure_alias_preserves_member_identity`
  - `typed_structure_case_insensitive_update_preserves_identity_and_canonical_name`
  - `typed_value_table_row_alias_preserves_column_identity`
  - `typed_value_table_column_case_insensitive_update_preserves_identity_and_canonical_name`
  - `structure_field_identity_survives_branch_merge`
  - `structure_member_identity_survives_else_branch_merge`
  - `value_table_column_identity_survives_else_branch_merge`

## Requirement: Semantic consumers используют один resolved path или thin adapters

Status: `covered`

Code:
- `bsl-runtime/src/application/intellisense_v2/facade.rs`
- `bsl-runtime/src/application/type_system/services/completion_service.rs`
- `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs`
- `bsl-runtime/src/application/type_system/services/hover_service.rs`
- `bsl-runtime/src/application/type_system/services/signature_help_service.rs`
- `backend/src/bin/lsp_server/server/language_server/impl_completion_helpers.rs`
- `backend/src/bin/lsp_server/server/language_server/impl_features_c.rs`
- `backend/src/bin/lsp_server/handlers/completion.rs`
- `backend/src/bin/lsp_server/handlers/hover.rs`
- `backend/src/bin/lsp_server/handlers/signature_help.rs`
- `backend/src/presentation/web/handlers.rs`
- `backend/src/presentation/web/handlers/semantic.rs`
- `bsl-agent/src/session/helpers_semantic.rs`
- `bsl-agent/src/session/manager_semantic_core.rs`

Tests:
- `backend/tests/universal_collection_cross_consumer_consistency_test.rs`
  - `typed_structure_completion_without_shared_owner_hint_uses_ir_contract_in_direct_handler_path`
  - `typed_value_table_row_completion_without_shared_owner_hint_uses_ir_contract_in_direct_handler_path`
- `bsl-runtime/src/application/type_system/services/completion_service/tests.rs`
  - `completion_implicit_form_object_member_access_resolves_from_ir_without_shared_hint`
  - `completion_resolves_member_owner_from_ir_without_owner_hint`
  - `completion_resolves_implicit_form_object_member_access_with_shared_hint`
  - `implicit_module_context_owner_resolution_uses_ir_for_supported_modules`
  - `implicit_module_context_owner_resolution_fails_closed_outside_supported_modules`
- `backend/tests/form_module_object_unified_contract_test.rs`
  - `completion_and_resolve_follow_unified_form_contract`
  - `completion_form_module_object_uses_ir_contract_without_shared_owner_hint`
- `backend/tests/legacy_form_object_alias_outputs_test.rs`
  - `completion_and_resolve_do_not_expose_legacy_form_alias`
- `backend/src/bin/lsp_server/server/core/tests.rs`
  - `p7_form_module_object_completion_uses_default_lsp_owner_hint_path`
  - `p7_hover_emits_type_index_reasons_while_completion_signature_and_definition_reuse_current_semantic_state`
  - `p7_typed_structure_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
  - `p7_typed_value_table_row_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
  - `p7_typed_structure_revision_switch_does_not_leak_stale_structural_members_across_interfaces`
  - `p7_typed_value_table_row_revision_switch_does_not_leak_stale_structural_members_across_interfaces`

Notes:
- completion owner resolution больше не содержит bootstrap-only implicit module-context fallback;
- supported implicit module-context completion работает через IR-derived owner contract на default path, а unsupported/no-binding path остаётся fail-closed.

## Requirement: Cross-consumer acceptance доказывает semantic equivalence, а не только smoke consistency

Status: `covered`

Code:
- `backend/src/bin/lsp_server/server/core/tests.rs`
- `backend/tests/universal_collection_cross_consumer_consistency_test.rs`
- `backend/tests/universal_collection_strict_policy_test.rs`
- `backend/tests/form_module_object_unified_contract_test.rs`
- `backend/tests/legacy_form_object_alias_outputs_test.rs`
- `backend/src/bin/lsp_server/handlers/completion.rs`
- `backend/src/bin/lsp_server/handlers/completion/tests.rs`
- `bsl-agent/src/types/mod.rs`
- `bsl-agent/src/session/manager_semantic_core.rs`

Tests:
- `backend/src/bin/lsp_server/server/core/tests.rs`
  - `p7_form_module_object_completion_uses_default_lsp_owner_hint_path`
  - `p7_typed_structure_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
  - `p7_typed_value_table_row_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
  - `p7_typed_structure_revision_switch_does_not_leak_stale_structural_members_across_interfaces`
  - `p7_typed_value_table_row_revision_switch_does_not_leak_stale_structural_members_across_interfaces`
- `backend/tests/universal_collection_cross_consumer_consistency_test.rs`
  - `typed_structure_completion_without_shared_owner_hint_uses_ir_contract_in_direct_handler_path`
  - `typed_value_table_row_completion_without_shared_owner_hint_uses_ir_contract_in_direct_handler_path`
- `backend/tests/form_module_object_unified_contract_test.rs`
  - `completion_and_resolve_follow_unified_form_contract`
  - `completion_form_module_object_uses_ir_contract_without_shared_owner_hint`
- `backend/tests/legacy_form_object_alias_outputs_test.rs`
  - `completion_and_resolve_do_not_expose_legacy_form_alias`
- `backend/src/bin/lsp_server/handlers/completion/tests.rs`
  - `structural_property_completion_items_include_member_identity_in_data`

## Requirement: Change completion MUST NOT завышать readiness относительно MUST backlog

Status: `covered`

Code / artefacts:
- `.github/workflows/ci.yml`
- `README.md`
- `CONTRIBUTING.md`
- `scripts/README.md`
- `scripts/check-openspec-change-governance.py`
- `scripts/check-protected-assets-gate.py`
- `scripts/test-ci-openspec-governance-workflow.py`
- `scripts/test-openspec-change-governance.py`
- `openspec/changes/update-gradual-core-production-readiness/validation/acceptance_matrix.md`
- `openspec/changes/update-gradual-core-production-readiness/validation/final-closure-checklist.md`
- `openspec/changes/update-gradual-core-production-readiness/governance/readiness_status.json`

Evidence:
- `python3 scripts/test-ci-openspec-governance-workflow.py`
- `python3 scripts/check-openspec-change-governance.py --change-id update-gradual-core-production-readiness`
- `python3 -m unittest scripts.test-openspec-change-governance -v`

## Requirement: Traceability и review artifacts MUST отражать реальные gaps без optimistic overclaim

Status: `covered`

Code / artefacts:
- `openspec/changes/update-gradual-core-production-readiness/design.md`
- `openspec/changes/update-gradual-core-production-readiness/traceability.md`
- `openspec/changes/update-gradual-core-production-readiness/residual-risk-review.md`
- `openspec/changes/update-gradual-core-production-readiness/validation/acceptance_matrix.md`
- `openspec/changes/update-gradual-core-production-readiness/validation/readiness-review-status.md`
- `openspec/changes/update-gradual-core-production-readiness/validation/final-closure-checklist.md`
- `openspec/changes/update-gradual-core-production-readiness/governance/readiness_status.json`

Evidence:
- `readiness-review-status.md` и `readiness_status.json` ссылаются на final reviewed evidence, а не на stale self-reported tokens;
- `traceability.md` больше не каталогизирует удалённый bootstrap fallback как допустимый end-state;
- `final-closure-checklist.md` фиксирует active workflow wiring, runtime convergence и full acceptance evidence;
- `residual-risk-review.md` сохраняет semantic-risk reasoning как вход в финальный verdict, без stale references на старый follow-up backlog.
