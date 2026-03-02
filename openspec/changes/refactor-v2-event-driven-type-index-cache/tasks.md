## 1. Specification
- [x] 1.1 Добавить/уточнить требования `bsl-intellisense-v2` для event-driven precompute `type_index` и serve-only request path.
- [x] 1.2 Уточнить churn fastpath requirement: latest-path использует только precomputed artifacts, без sync parse/index.
- [x] 1.3 Зафиксировать observability контракт для precompute queue/exec и cache serving outcomes.

## 2. Architecture And Contracts
- [x] 2.1 Утвердить ADR для `perf_critical` перехода на serve-only model (с rollback стратегией).
- [x] 2.2 Зафиксировать модель ключей/инвалидации `TypeIndexArtifactKey(file_id, file_version, deps_id, settings_id)`.
- [x] 2.3 Определить retention/eviction policy для artifact cache (per-file window + global guard).
- [x] 2.4 Определить reason-code taxonomy для cache miss/degraded serve/superseded precompute.

## 3. Implementation
- [x] 3.1 Выделить границу ответственности для snapshot-derived artifacts в `analysis-v2` (отдельный модуль).
- [ ] 3.2 Реализовать event-driven precompute pipeline на `didOpen/didChange` и latest-wins supersede.
- [ ] 3.3 Перевести интерактивный type lookup на serve-only cache API.
- [ ] 3.4 Запретить sync parse/index compute в интерактивном request path и покрыть это тестом/инвариантом.
- [ ] 3.5 Добавить bounded fallback поведения при cache miss (`stale/degraded_incomplete/fallback_unavailable`).
- [ ] 3.6 Интегрировать mode-based rollout (`shadow`, `canary`, `on`) и rollback путь.

## 4. Validation
- [ ] 4.1 Добавить parity/consistency тесты между legacy и serve-only path для одинаковых version/deps/settings.
- [ ] 4.2 Добавить тесты supersede/cancel precompute jobs под burst `didChange`.
- [ ] 4.3 Добавить perf regression checks для `large/small/churn` с акцентом на completion tail latency.
- [ ] 4.4 Подтвердить через observability, что интерактивный path не выполняет on-demand parse/index.
- [ ] 4.5 `openspec validate refactor-v2-event-driven-type-index-cache --strict --no-interactive`.

## Dependencies / Parallelism
- [ ] D1 Пункты 2.1-2.4 блокируют 3.2-3.5.
- [ ] D2 Пункты 3.2 и 3.3 можно делать параллельно после 2.2.
- [ ] D3 Пункт 3.6 зависит от 4.1-4.3 (для безопасного canary).
