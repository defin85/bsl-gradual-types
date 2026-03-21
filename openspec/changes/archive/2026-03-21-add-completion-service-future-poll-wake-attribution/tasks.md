## 1. Контракт `v11` для bounded service-future poll / wake attribution
- [x] 1.1 Зафиксировать additive `v11` semantics для `service_future_first_poll_entered_at_ms`, `service_future_to_first_poll_wait_ms`, `service_future_first_poll_outcome`, `service_future_first_wake_scheduled_at_ms` и `first_poll_to_first_wake_wait_ms`.
- [x] 1.2 Зафиксировать relationship между `service_future_created_at_ms` и новым first-poll / first-wake cut без guessed interpretation.
- [x] 1.3 Сохранить bounded contract discipline и existing `v10` / `v9` / `v8` trustworthy semantics без free-text/high-cardinality полей.

## 2. Wiring backend producer path
- [x] 2.1 Добавить instrumentation wrapper вокруг returned service future, чтобы authoritative path мог фиксировать первый `poll()` и его bounded outcome.
- [x] 2.2 Протянуть first-poll / first-wake split через completion timeline producer path и сериализовать derived waits.
- [x] 2.3 Сохранить honest fallback semantics, если first wake не наблюдался или future не требует pending path, и не сломать overlapping/fallback request paths.

## 3. Projection и handoff
- [x] 3.1 Обновить Completion Timeline panel и clipboard под `v11` poll / wake split и explicit `v10` degradation.
- [x] 3.2 Обновить request-centric incident bundle summary / findings / gaps так, чтобы first-poll / first-wake split был виден без invented data, а на `v10` limitation называлась явно.

## 4. Проверка и фиксация
- [x] 4.1 Добавить backend tests для `v11` contract, first-poll outcome и no-fabrication rules для first wake.
- [x] 4.2 Добавить extension tests для `v11` projection и explicit `v10` degradation в panel / clipboard / incident bundle.
- [x] 4.3 Обновить smoke/runbook expectations для нового bounded split и для `v10` note `first poll / wake split unavailable by design`.
- [x] 4.4 Зафиксировать `Requirement -> Code -> Test` traceability.
- [x] 4.5 Провалидировать change через `openspec validate add-completion-service-future-poll-wake-attribution --strict --no-interactive`.
- [x] 4.6 Синхронизировать versioned contract baseline и canonical OpenSpec truth с shipped `v11` payload.
