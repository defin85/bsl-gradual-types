## 1. Specification
- [ ] 1.1 Обновить `bsl-intellisense-v2` requirement для latency-priority policy: зафиксировать dual revision model (`received` vs `applied`) и интерактивный wait по `applied_version`.
- [ ] 1.2 Обновить `bsl-intellisense-v2` requirement для completion fallback: stale-first частичный ответ (`isIncomplete=true`) при timeout/cancel и безопасном stale snapshot.
- [ ] 1.3 Обновить `bsl-intellisense-v2` requirement для CPU scheduling: приоритет control-path и защита interactive guarantees от background contention.
- [ ] 1.4 Обновить `bsl-intellisense-v2` requirement для observability и quality gate: добавить lag/fallback метрики и cancel-rate критерий.

## 2. Runtime Architecture (implementation follow-up)
- [ ] 2.1 Ввести applied-revision tracking в runtime/LSP coordination path и синхронизировать его с `SetFile` apply lifecycle.
- [ ] 2.2 Реализовать priority lanes (control/query) в runtime writer orchestration без нарушения monotonic diagnostics publish.
- [ ] 2.3 Реализовать completion stale-first fallback с явным `isIncomplete=true` и совместимостью по `deps_id/settings_id`.

## 3. Validation
- [ ] 3.1 Добавить regression-тест: первый completion после didChange не деградирует в "пусто" при допустимом stale snapshot.
- [ ] 3.2 Добавить perf smoke на warm-path: проверить `p95 wait_for_version`, `p95 completion_duration`, `cancel-rate`.
- [ ] 3.3 `openspec validate update-v2-interactive-latency-applied-revision --strict --no-interactive`.
