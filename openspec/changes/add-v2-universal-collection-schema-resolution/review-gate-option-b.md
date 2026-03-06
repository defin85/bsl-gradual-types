# Review Gate: Reject Implementations Outside Option B

Дата: 2026-03-06  
Change: `add-v2-universal-collection-schema-resolution`

## Gate Criteria

1. **Option C запрещён**: нет runtime-пути, который добавляет per-instance synthetic types в global `TypeRepository`.
2. **Option A запрещён**: `completion`/`hover`/`type-at-position`/`diagnostics` читают один resolved path, а не отдельные consumer-local schema модели.
3. **Guardrails есть и проверяются тестами**.

## Evidence

### 1) Guardrails против Option C

- Code:
  - `bsl-repository/src/repository.rs`
    - `ensure_no_forbidden_instance_local_types(...)`
    - вызов в `load_types(...)` и `upsert_types(...)`
- Tests:
  - `bsl-repository/src/repository/tests.rs`
    - `test_load_types_rejects_per_instance_collection_synthetic_types`
    - `test_upsert_types_rejects_per_instance_collection_synthetic_types`
    - `test_load_types_allows_form_synthetic_type_names`
- Verification command:
  - `cargo test -p bsl-repository repository::tests:: -- --nocapture`
  - Result: `11 passed; 0 failed`

### 2) Unified resolved path (против Option A)

- Code:
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
    - completion берёт owner из `member_access_owner_type_hint` как primary path.
  - `bsl-runtime/src/application/type_system/services/hover_service.rs`
    - hover использует `type_at_byte_offset_serve_only` / owner-span probing.
  - `bsl-agent/src/session/helpers_semantic.rs`
    - `type_at_position` и owner-hint выровнены на тот же analysis path.
- Cross-consumer integration tests:
  - `backend/tests/universal_collection_cross_consumer_consistency_test.rs`
    - `map_index_access_cross_consumer_consistency`
    - `structure_field_cross_consumer_consistency`
    - `value_table_row_column_cross_consumer_consistency`
- Verification command:
  - `cargo test -p bsl-backend --test universal_collection_cross_consumer_consistency_test -- --nocapture`
  - Result: `14 passed; 0 failed`

### 3) Нет признаков runtime-мутирования repository из analysis/consumer pipeline

- Проверка по runtime/analysis путям (`analysis-v2/src`, `bsl-runtime/src/application/type_system/services`, `semantic-diagnostics/src`) показывает, что `load_types/upsert_types/remove_types` встречаются только в тестах, не в production-path этих модулей.
- Быстрая проверка:
  - `rg -n "\\b(load_types|upsert_types|remove_types)\\b" analysis-v2/src bsl-runtime/src/application/type_system/services semantic-diagnostics/src --glob '*.rs'`
  - Результат: совпадения только в `*_tests.rs`.

## Gate Decision

- **PASS (для 4.2)**: текущий change не вводит Option A/Option C runtime paths как source of truth.
- Ограничения и незакрытые MUST-gaps отражены отдельно в `traceability.md` (rows со статусом `gap`/`partial`).
