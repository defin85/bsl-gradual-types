# Traceability: update-gradual-core-production-readiness

Этот артефакт фиксирует прямую трассировку `Requirement -> Code -> Test`
для delivered production-readiness contract.

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
- `backend/src/bin/lsp_server/handlers/completion.rs`
- `backend/src/bin/lsp_server/handlers/hover.rs`
- `backend/src/presentation/web/handlers.rs`
- `backend/src/presentation/web/handlers/semantic.rs`
- `bsl-agent/src/session/manager_semantic_core.rs`

Tests:
- `backend/tests/universal_collection_cross_consumer_consistency_test.rs`
  - `typed_structure_completion_without_shared_owner_hint_fails_closed_in_direct_handler_path`
  - `typed_value_table_row_completion_without_shared_owner_hint_fails_closed_in_direct_handler_path`
- `backend/src/bin/lsp_server/server/core/tests.rs`
  - `p7_typed_structure_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
  - `p7_typed_value_table_row_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
  - `p7_typed_structure_revision_switch_does_not_leak_stale_structural_members_across_interfaces`
  - `p7_typed_value_table_row_revision_switch_does_not_leak_stale_structural_members_across_interfaces`

Notes:
- bounded bootstrap-only module-context path remains explicitly catalogued in `design.md` and does not act as second structural truth for reviewed scenarios.

## Requirement: Cross-consumer acceptance доказывает semantic equivalence, а не только smoke consistency

Status: `covered`

Code:
- `backend/src/bin/lsp_server/server/core/tests.rs`
- `backend/tests/universal_collection_cross_consumer_consistency_test.rs`
- `backend/tests/universal_collection_strict_policy_test.rs`
- `backend/src/bin/lsp_server/handlers/completion.rs`
- `backend/src/bin/lsp_server/handlers/completion/tests.rs`
- `bsl-agent/src/types/mod.rs`
- `bsl-agent/src/session/manager_semantic_core.rs`

Tests:
- `backend/src/bin/lsp_server/server/core/tests.rs`
  - `p7_typed_structure_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
  - `p7_typed_value_table_row_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
  - `p7_typed_structure_revision_switch_does_not_leak_stale_structural_members_across_interfaces`
  - `p7_typed_value_table_row_revision_switch_does_not_leak_stale_structural_members_across_interfaces`
- `backend/tests/universal_collection_cross_consumer_consistency_test.rs`
  - `typed_structure_completion_without_shared_owner_hint_fails_closed_in_direct_handler_path`
  - `typed_value_table_row_completion_without_shared_owner_hint_fails_closed_in_direct_handler_path`
- `backend/src/bin/lsp_server/handlers/completion/tests.rs`
  - `structural_property_completion_items_include_member_identity_in_data`

## Requirement: Change completion MUST NOT завышать readiness относительно MUST backlog

Status: `covered`

Code / artefacts:
- `openspec/changes/update-gradual-core-production-readiness/tasks.md`
- `openspec/changes/update-gradual-core-production-readiness/validation/acceptance_matrix.md`
- `openspec/changes/update-gradual-core-production-readiness/validation/final-closure-checklist.md`
- `openspec/changes/update-gradual-core-production-readiness/governance/change_criticality.json`
- `openspec/changes/update-gradual-core-production-readiness/governance/dependency_checks.json`
- `openspec/changes/update-gradual-core-production-readiness/governance/readiness_status.json`
- `scripts/check-openspec-change-governance.py`

Evidence:
- `openspec validate update-gradual-core-production-readiness --strict --no-interactive`
- `python3 scripts/check-openspec-change-governance.py --change-id update-gradual-core-production-readiness`

## Requirement: Traceability и review artifacts MUST отражать реальные gaps без optimistic overclaim

Status: `covered`

Code / artefacts:
- `openspec/changes/update-gradual-core-production-readiness/proposal.md`
- `openspec/changes/update-gradual-core-production-readiness/design.md`
- `openspec/changes/update-gradual-core-production-readiness/traceability.md`
- `openspec/changes/update-gradual-core-production-readiness/residual-risk-review.md`
- `openspec/changes/update-gradual-core-production-readiness/validation/readiness-review-status.md`
- `openspec/changes/update-gradual-core-production-readiness/validation/final-closure-checklist.md`
- `openspec/changes/update-gradual-core-production-readiness/governance/readiness_status.json`

Evidence:
- `residual-risk-review.md` отделяет semantic risk closure от final closure status;
- `readiness_status.json` объявляет `complete` только после закрытия critical backlog;
- `final-closure-checklist.md` фиксирует final validation/gate evidence без optimistic wording.
