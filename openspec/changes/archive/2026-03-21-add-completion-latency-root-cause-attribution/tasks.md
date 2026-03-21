## 1. Контракт и модель данных
- [x] 1.1 Зафиксировать `v6` contract shape для method-entry split, prepare timeout attribution и exact artifact polling.
- [x] 1.2 Спроектировать bounded server-edge split между `transport_received -> method_entered` и `method_entered -> handler_entered`.
- [x] 1.3 Спроектировать bounded `prepare_details.timeout_attribution` с `source`, `phase`, `budget_ms`, `elapsed_ms` и `overshoot_ms`.
- [x] 1.4 Спроектировать bounded `exact_wait.artifact_poll` для polling deadline до waiter/task-state path.

## 2. Runtime wiring и consumer compatibility
- [x] 2.1 Протянуть method-entry attribution из request path в authoritative completion timeline без нового API.
- [x] 2.2 Протянуть timeout attribution из outer prepare guard и interactive wait budget path в `v6` payload.
- [x] 2.3 Протянуть artifact polling attribution из exact artifact-readiness loop в `v6` payload.
- [x] 2.4 Обновить extension consumer types и existing completion surfaces для `v6`, с явной деградацией на `v5`.

## 3. Проверка и фиксация
- [x] 3.1 Добавить backend contract/regression tests для `v6` fields, bounded vocabulary и timeout overshoot semantics.
- [x] 3.2 Добавить extension tests для `v6` parsing, panel/clipboard projection и incident handoff compatibility.
- [x] 3.3 Обновить smoke/runbook expectations для `contract=v6` и новых root-cause fact lines.
- [x] 3.4 Зафиксировать `Requirement -> Code -> Test` traceability для всех обязательных сценариев.
- [x] 3.5 Провалидировать change через `openspec validate add-completion-latency-root-cause-attribution --strict --no-interactive`.
