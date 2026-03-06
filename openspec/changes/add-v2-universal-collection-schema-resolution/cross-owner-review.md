# Cross-Owner Architecture Review

Дата: 2026-03-06  
Change: `add-v2-universal-collection-schema-resolution`

## Scope

Проверка согласованности реализации и guardrails по доменам:
- `analysis-v2`
- `completion`
- `diagnostics`
- `metadata_lookup`

## Owner Review Notes

### analysis-v2
- Проверено, что в production-path `analysis-v2/src/**` нет мутаций глобального `TypeRepository` через `load_types/upsert_types/remove_types` (совпадения только в тестах).
- Runtime contract использует snapshot-based APIs (`type_at_*`, `flow_type_at_*`) как источник type hints.
- Вердикт: **accepted** для Option B boundary (без Option C path).

### completion
- Проверено, что `completion` использует owner-type hint path и не опирается на consumer-local schema inference как source of truth:
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs`
- Добавлены guardrails тесты cross-consumer consistency:
  - `backend/tests/universal_collection_cross_consumer_consistency_test.rs`
- Вердикт: **accepted** с учётом зафиксированных MUST gaps в traceability.

### diagnostics
- Проверен единый diagnostics path через semantic visitor:
  - `semantic-diagnostics/src/visitor.rs`
- Проверена совместимость с unified consumer contract на интеграционных кейсах (`completion` candidate не даёт ложный unknown-member diagnostics).
- Вердикт: **accepted** как часть unified path.

### metadata_lookup
- Проверена роль metadata lookup как общего слоя lookup/validation:
  - `shared/src/domain/metadata_lookup/core.rs`
- В review-gate и traceability зафиксированы места, где нужны дополнительные адресные acceptance tests (rows `gap`/`partial`).
- Вердикт: **accepted** с явными follow-up пунктами.

## Verification Artifacts

- `openspec/changes/add-v2-universal-collection-schema-resolution/review-gate-option-b.md`
- `openspec/changes/add-v2-universal-collection-schema-resolution/traceability.md`
- `cargo test -p bsl-repository repository::tests:: -- --nocapture` (pass)
- `cargo test -p bsl-backend --test universal_collection_cross_consumer_consistency_test -- --nocapture` (pass)
- `openspec validate add-v2-universal-collection-schema-resolution --strict --no-interactive` (pass)

## Decision

- Архитектурный review по владельческим зонам завершён.
- Change совместим с зафиксированным Option B contract.
- Незакрытые MUST evidence-gaps явно перечислены в `traceability.md` и не скрыты.
