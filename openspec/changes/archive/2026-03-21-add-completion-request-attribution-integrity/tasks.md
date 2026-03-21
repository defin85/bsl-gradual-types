## 1. Контракт v8 для trustworthy pre-method attribution
- [x] 1.1 Зафиксировать bounded `v8` provenance semantics для pre-method attribution и explicit degradation на `v7`.
- [x] 1.2 Зафиксировать fail-closed правило: strong ingress verdict возможен только для same-request authoritative attribution.

## 2. Wiring backend/runtime
- [x] 2.1 Протянуть request-aware provenance в completion timeline producer path.
- [x] 2.2 Убрать silent same-request illusion для overlapping completion на одном `uri + position`.
- [x] 2.3 Сохранить bounded contract discipline без free-text/high-cardinality debug fields.

## 3. Projection и handoff
- [x] 3.1 Обновить Completion Timeline panel и clipboard под `v8` provenance.
- [x] 3.2 Обновить incident bundle request summary и findings так, чтобы weak attribution не агрегировался как сильный ingress bottleneck.

## 4. Проверка и фиксация
- [x] 4.1 Добавить backend tests для overlapping completion attribution и `v8` contract.
- [x] 4.2 Добавить extension tests для `v8` provenance и `v7` degradation.
- [x] 4.3 Обновить smoke/runbook expectations для trustworthy ingress attribution.
- [x] 4.4 Зафиксировать `Requirement -> Code -> Test` traceability.
- [x] 4.5 Провалидировать change через `openspec validate add-completion-request-attribution-integrity --strict --no-interactive`.
