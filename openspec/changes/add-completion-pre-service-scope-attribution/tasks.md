## 1. Контракт `v9` для bounded pre-service-scope attribution
- [x] 1.1 Зафиксировать additive `v9` semantics для `service_future_created_at_ms`, `transport_to_service_future_wait_ms` и `service_future_to_scope_wait_ms`.
- [x] 1.2 Зафиксировать explicit degradation на `v8` без guessed reconstruction и без молчаливого `No gaps were recorded` для missing split.
- [x] 1.3 Сохранить bounded contract discipline и existing `v8` provenance semantics без free-text/high-cardinality полей.

## 2. Wiring backend producer path
- [x] 2.1 Протянуть `service_future_created_at_ms` через `RequestContextService` и authoritative completion timeline producer path.
- [x] 2.2 Сериализовать derived waits так, чтобы оператор не вычислял их вручную.
- [x] 2.3 Сохранить trustworthy handoff semantics для overlapping completion requests и fallback paths.

## 3. Projection и handoff
- [x] 3.1 Обновить Completion Timeline panel и clipboard под `v9` pre-service-scope split и explicit `v8` degradation.
- [x] 3.2 Обновить request-centric incident bundle summary / findings / gaps так, чтобы новый split был виден без invented data, а на `v8` limitation была названа явно.

## 4. Проверка и фиксация
- [x] 4.1 Добавить backend tests для `v9` contract и derived pre-service-scope waits.
- [x] 4.2 Добавить extension tests для `v9` projection и explicit `v8` degradation в panel / clipboard / incident bundle.
- [x] 4.3 Обновить smoke/runbook expectations для нового bounded split и для `v8` note `split unavailable by design`.
- [x] 4.4 Зафиксировать `Requirement -> Code -> Test` traceability.
- [x] 4.5 Провалидировать change через `openspec validate add-completion-pre-service-scope-attribution --strict --no-interactive`.
