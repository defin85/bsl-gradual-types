## 1. Контракт v7 для root-cause narrowing
- [x] 1.1 Зафиксировать bounded `v7` pre-method ingress split (`transport_received -> service_scope_entered -> method_entered`) без нового лог-канала.
- [x] 1.2 Зафиксировать bounded timeout-safe `snapshot_with_deps_timeout_runtime` для timeout path с fixed `resolution` vocabulary.
- [x] 1.3 Зафиксировать additive serialization и explicit degradation semantics для старого `v6` payload.

## 2. Wiring backend/runtime
- [x] 2.1 Протянуть `service_scope_entered` timestamp и derived waits в authoritative completion timeline.
- [x] 2.2 Протянуть timeout-safe snapshot overshoot attribution из runtime/facade в `prepare_timeout` path.
- [x] 2.3 Сохранить существующие `v6` поля и не смешивать новый drilldown с free-text/high-cardinality данными.

## 3. Projection и handoff
- [x] 3.1 Обновить Completion Timeline panel и clipboard под `v7` pre-method/snapshot overshoot facts.
- [x] 3.2 Обновить request-centric incident bundle summary под `v7` facts и явную деградацию на `v6`.

## 4. Проверка и фиксация
- [x] 4.1 Добавить backend/runtime tests для `v7` contract и timeout-safe snapshot attribution.
- [x] 4.2 Добавить extension tests для `v7` projection и `v6` degradation.
- [x] 4.3 Обновить smoke/runbook expectations для `v7` root-cause attribution.
- [x] 4.4 Зафиксировать `Requirement -> Code -> Test` traceability для нового drilldown.
- [x] 4.5 Провалидировать change через `openspec validate add-completion-pre-method-and-snapshot-overshoot-attribution --strict --no-interactive`.
