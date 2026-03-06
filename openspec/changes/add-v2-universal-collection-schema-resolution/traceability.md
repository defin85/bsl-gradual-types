# Traceability Matrix: `add-v2-universal-collection-schema-resolution`

Дата: 2026-03-06

Цель матрицы: явно связать каждый MUST из `specs/bsl-intellisense-v2/spec.md` с кодом и автотестами.

## Legend
- `covered`: есть прямая автоматизированная проверка.
- `partial`: есть код/частичные проверки, но нет полного acceptance-покрытия сценария.
- `gap`: кодовая точка есть, но явного тестового подтверждения в текущем change нет.

| Req | MUST (кратко) | Code | Test Evidence | Status |
| --- | --- | --- | --- | --- |
| R1 | v2 хранит schema-effects universal collections в одном snapshot | `analysis-v2/src/type_inference_v2.rs`; `bsl-runtime/src/application/type_system/services/completion_service.rs`; `bsl-runtime/src/application/type_system/services/hover_service.rs` | `backend/tests/universal_collection_cross_consumer_consistency_test.rs` | partial |
| R2 | Consumer channels используют единый resolved path | `bsl-runtime/src/application/type_system/services/completion_service.rs`; `bsl-runtime/src/application/type_system/services/hover_service.rs`; `semantic-diagnostics/src/visitor.rs`; `bsl-agent/src/session/helpers_semantic.rs` | `backend/tests/universal_collection_cross_consumer_consistency_test.rs`; `backend/src/bin/lsp_server/server/core/tests.rs` (`p23`, `p25`) | partial |
| R3 | `Соответствие[key]` резолвит value type | `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs` (`resolve_index_access_element_type`) | `bsl-runtime/src/application/type_system/services/completion_service/tests.rs` (`completion_supports_member_access_after_map_index_access`) | covered |
| R4 | Приоритет `map[key]`: literal -> generic `V` -> `Произвольный` | `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs`; `shared/src/domain/generic_inference.rs` | `bsl-runtime/src/application/type_system/services/completion_service/tests.rs` (map/index-access coverage) | gap |
| R5 | Completion/hover/type-at-position после `map[key]` согласованы | `bsl-runtime/src/application/type_system/services/completion_service.rs`; `bsl-runtime/src/application/type_system/services/hover_service.rs`; `bsl-agent/src/session/helpers_semantic.rs` | `backend/tests/universal_collection_cross_consumer_consistency_test.rs` (`map_index_access_cross_consumer_consistency`) | covered |
| R6 | Dynamic keys не дают hard-fail unknown-key | `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs`; `semantic-diagnostics/src/visitor.rs` | — | gap |
| R7 | Per-instance schema не мутирует global `TypeRepository` | `bsl-repository/src/repository.rs` (`ensure_no_forbidden_instance_local_types`) | `bsl-repository/src/repository/tests.rs` (`test_load_types_rejects_per_instance_collection_synthetic_types`, `test_upsert_types_rejects_per_instance_collection_synthetic_types`, `test_load_types_allows_form_synthetic_type_names`) | covered |
| R8 | Flow-sensitive schema полей `Структура` отслеживается | `analysis-v2/src/type_inference_v2.rs`; `semantic-diagnostics/src/visitor.rs` | `backend/tests/universal_collection_cross_consumer_consistency_test.rs` (`structure_field_cross_consumer_consistency`) | partial |
| R9 | Typed-structure поля резолвятся в user channels | `bsl-runtime/src/application/type_system/services/completion_service.rs`; `bsl-runtime/src/application/type_system/services/hover_service.rs`; `semantic-diagnostics/src/visitor.rs` | `backend/tests/universal_collection_cross_consumer_consistency_test.rs` (`structure_field_cross_consumer_consistency`) | partial |
| R10 | Unknown field у typed-structure -> hard-fail | `semantic-diagnostics/src/visitor.rs`; `shared/src/domain/metadata_lookup/core.rs` | `backend/tests/form_module_object_unified_contract_test.rs` (non-existent property diagnostics contract) | partial |
| R11 | Тип поля: best-effort, fallback в `Произвольный` | `analysis-v2/src/type_inference_v2.rs`; `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs` | — | gap |
| R12 | Schema-effect `ТЗ.Колонки.Добавить` отслеживается | `analysis-v2/src/type_inference_v2.rs`; `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs` | `backend/tests/universal_collection_cross_consumer_consistency_test.rs` (`value_table_row_column_cross_consumer_consistency`) | partial |
| R13 | Typed-row `ТаблицаЗначений` резолвит колонки как свойства | `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs`; `semantic-diagnostics/src/visitor.rs` | `backend/tests/universal_collection_cross_consumer_consistency_test.rs` (`value_table_row_column_cross_consumer_consistency`) | partial |
| R14 | Unknown column у typed-row -> hard-fail | `semantic-diagnostics/src/visitor.rs`; `shared/src/domain/metadata_lookup/core.rs` | — | gap |
| R15 | Тип колонки из `ОписаниеТипов`, иначе fallback `Произвольный` | `analysis-v2/src/type_inference_v2.rs`; `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs` | `backend/tests/universal_collection_cross_consumer_consistency_test.rs` (`value_table_row_column_cross_consumer_consistency`) | partial |
| R16 | Acceptance: cross-consumer consistency в одной позиции | `bsl-runtime/src/application/type_system/services/completion_service.rs`; `bsl-runtime/src/application/type_system/services/hover_service.rs`; `semantic-diagnostics/src/visitor.rs` | `backend/tests/universal_collection_cross_consumer_consistency_test.rs`; `backend/src/bin/lsp_server/server/core/tests.rs` (`p23`, `p25`) | covered |

## Notes
- `R4`, `R6`, `R11`, `R14` помечены как `gap` и требуют адресных acceptance/diagnostics тестов для полного MUST coverage.
- Матрица намеренно фиксирует текущее состояние evidence (без «молчаливого» переобъявления требований выполненными).
