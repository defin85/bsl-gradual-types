## 1. Specification
- [ ] 1.1 Обновить `bsl-intellisense-v2` требования для contract-first hardening observability mapping (typed registry + deterministic projection).
- [ ] 1.2 Добавить requirement для deterministic count-based retention policy `TypeIndexArtifact`.
- [ ] 1.3 Добавить requirement для unified serve-outcome emission на всех interactive операциях.
- [ ] 1.4 Добавить requirement для perf-gate traceability по `change_id` с fail-fast на mismatch.

## 2. Architecture And Contracts
- [ ] 2.0 Явно зафиксировать decision statement: `Contract-first hardening with registry-driven materialization and fail-closed provenance` (recommended) и зафиксировать, что full rewrite observability/perf pipeline вне scope данного change.
- [ ] 2.1 Утвердить design решения по единому источнику truth для `stage/reason` taxonomy и projection completeness.
- [ ] 2.2 Зафиксировать точную retention semantics (`max versions` как count-based контракт, без неоднозначности version-gap).
- [ ] 2.3 Определить migration policy для observability dual-write без ломки legacy keys.
- [ ] 2.4 Зафиксировать perf artifacts provenance contract (`change_id`, profile, generated_at, validator expectations).
- [ ] 2.5 Зафиксировать ownership typed registry и policy "no adapter-local mapping bypass".
- [ ] 2.6 Зафиксировать детерминированный eviction order и latest exact protection для global guard.
- [ ] 2.7 Зафиксировать unified serve-outcome contract для `completion/hover/signatureHelp/definition`.
- [ ] 2.8 Синхронизировать operation policy для `Definition` с interactive path.
- [ ] 2.9 Зафиксировать optional provenance migration в `v1` и fail-правила для provided provenance.

## 3. Implementation
- [ ] 3.1 Ввести typed registry/mapping слой для runtime stage/reason и переподключить normalize + legacy projection к нему.
- [ ] 3.2 Гарантировать dedicated mapping для `type_index_precompute` queue/exec/build (без деградации в `runtime_other_*`).
- [ ] 3.3 Привести retention реализацию `TypeIndexArtifact` к зафиксированному count-based контракту.
- [ ] 3.4 Централизовать emission `type_index` serve reasons для `completion/hover/signatureHelp/definition`.
- [ ] 3.5 Убрать hardcoded foreign `change_id` из perf-gate пути и сделать привязку к runtime invocation context.
- [ ] 3.6 Обновить versioned contracts/changelog при изменениях observability surface.
- [ ] 3.7 Удалить/ограничить разрозненные string-based mapping ветки в пользу typed registry materialization.
- [ ] 3.8 Привести `Definition` к interactive freshness/policy consistency (`queue priority`, `freshness knobs`, serve-only semantics).
- [ ] 3.9 Добавить optional provenance поля в perf report `v1` + validator checks для provided provenance mismatch/invalid.

## 4. Validation
- [ ] 4.1 Добавить contract tests на полноту mapping (registry -> canonical -> legacy), включая unknown -> `other`.
- [ ] 4.2 Добавить retention invariants tests (`max versions`, eviction order, latest exact protection under global guard).
- [ ] 4.3 Добавить integration tests, подтверждающие emission serve reasons для всех interactive операций.
- [ ] 4.4 Добавить/обновить tests для perf report traceability (`change_id` consistency).
- [ ] 4.5 Добавить тесты, что `missing optional provenance` в `v1` не валит gate.
- [ ] 4.6 Добавить тесты, что `provided provenance mismatch/invalid` в `v1` валит gate fail-closed.
- [ ] 4.7 Добавить canary rollback test при parity drift выше утверждённого порога.
- [ ] 4.8 `openspec validate refactor-v2-contract-first-hardening --strict --no-interactive`.

## Dependencies / Parallelism
- [ ] D1 Пункты 1.1-1.4 и 2.1-2.9 блокируют 3.1-3.9.
- [ ] D2 Пункты 3.1 и 3.3 можно делать параллельно после 2.2.
- [ ] D3 Пункты 3.4, 3.8 и 3.9 зависят от 3.1 (единый registry/contract path).
- [ ] D4 Пункты 4.1-4.7 зависят от 3.1-3.9.
