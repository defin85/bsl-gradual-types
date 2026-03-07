# Review Gate: Reject Implementations Outside Option B

Дата: 2026-03-07  
Change: `add-v2-universal-collection-schema-resolution`

## Gate Criteria

1. **Option C запрещён**: per-instance schema/effects не попадают в global `TypeRepository`.
2. **Option A запрещён**: `completion` / `hover` / `type-at-position` / `semantic diagnostics` читают один resolved path, а не отдельные consumer-local schema модели.
3. **Rollout/rollback guardrails fail-closed**: change использует существующий `intellisense_v2_*` observability + perf gate contract, а не несуществующий feature flag.

## Evidence

### 1) Guardrails против Option C

- Code:
  - `bsl-repository/src/repository.rs`
    - `ensure_no_forbidden_instance_local_types(...)`
    - запрет в `load_types(...)` и `upsert_types(...)`
  - `analysis-v2/src/type_inference_v2/instance_effects.rs`
    - snapshot-local effect store для `Соответствие` / `Структура` / `ТаблицаЗначений`
- Tests:
  - `bsl-repository/src/repository/tests.rs`
    - `test_load_types_rejects_per_instance_collection_synthetic_types`
    - `test_upsert_types_rejects_per_instance_collection_synthetic_types`
    - `test_load_types_allows_form_synthetic_type_names`
  - `analysis-v2/src/type_inference_v2/tests.rs`
    - `universal_collection_effects_do_not_mutate_type_repository`

### 2) Unified resolved path (против Option A)

- Code:
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `bsl-runtime/src/application/type_system/services/hover_service.rs`
  - `bsl-agent/src/session/helpers_semantic.rs`
  - `semantic-diagnostics/src/visitor.rs`
  - `syntax/src/tree_sitter_adapter/expression_converter.rs`
- Direct acceptance / regression evidence:
  - `backend/src/bin/lsp_server/server/core/tests.rs`
    - `p7_map_index_access_exact_cross_consumer_acceptance_uses_snapshot_owner_without_manual_hint`
    - `p7_dynamic_map_key_exact_cross_consumer_acceptance_uses_safe_policy_without_unknown_key`
    - `p7_typed_structure_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
    - `p7_typed_value_table_row_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics`
    - `p7_hover_cache_miss_on_map_index_access_does_not_use_legacy_word_fallback`
  - `analysis-v2/src/lib/tests.rs`
    - `serve_only_matches_legacy_for_universal_collections_in_complete_snapshot`
    - `serve_only_matches_legacy_for_universal_collections_with_incomplete_member_access`
  - `syntax/src/lib/tests.rs`
    - `parse_incomplete_member_access_preserves_receiver_expression`

### 3) Rollout / rollback guardrails опираются на существующий observability contract

- Code:
  - `bsl-runtime/src/system/basic_observability.rs`
    - canonical `intellisense_v2_*` counter/histogram/gauge keys
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
    - stage-level telemetry hooks для runtime/query pipeline
  - `backend/src/perf_gate_evaluator.rs`
    - rollback thresholds по latency/cancelled/stale-fallback ratios
- Tests:
  - `backend/tests/intellisense_v2_scale_aware_gate_contract_test.rs`
    - schema + threshold contract для gate reports
  - `backend/src/perf_gate_evaluator/tests.rs`
    - `parity_cutover_canary_rollback_guard_blocks_drift_regression`
  - `bsl-runtime/src/application/intellisense_v2/facade/tests.rs`
    - `observability_contract_values_are_stable`
    - `interactive_prepare_timeout_serves_stale_when_gap_within_default`
    - `completion_mode_propagates_into_stage_drilldown_metrics`
    - `interactive_prepare_timeout_rejects_stale_when_gap_exceeds_default`
    - `interactive_prepare_timeout_rejects_stale_when_age_exceeds_default`
    - `singleflight_records_leader_shared_and_wait_metrics`
  - `bsl-runtime/src/system/basic_observability/tests.rs`
    - `singleflight_projection_is_deterministic_for_query_kind`
    - `export_includes_parse_result_singleflight_and_cancel_rates`
    - `completion_outcome_exports_degraded_and_fallback_unavailable`

## Gate Decision

- **PASS (для 4.2)**: текущий change остаётся внутри Option B и не вводит Option A / Option C runtime paths.
- **PASS (для rollout/rollback guardrails)**: contract перепривязан к реально существующему `intellisense_v2_*` telemetry + perf gate surface; устаревших feature-flag claims не осталось.
