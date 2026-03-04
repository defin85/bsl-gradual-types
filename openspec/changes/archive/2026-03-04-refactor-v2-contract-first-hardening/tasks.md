## 1. Specification
- [x] 1.1 Обновить `bsl-intellisense-v2` требования для contract-first hardening observability mapping (typed registry + deterministic projection).
- [x] 1.2 Добавить requirement для deterministic count-based retention policy `TypeIndexArtifact`.
- [x] 1.3 Добавить requirement для unified serve-outcome emission на всех interactive операциях.
- [x] 1.4 Добавить requirement для perf-gate traceability по `change_id` с fail-fast на mismatch.

## 2. Architecture And Contracts
- [x] 2.0 Явно зафиксировать decision statement: `Contract-first hardening with registry-driven materialization and fail-closed provenance` (recommended) и зафиксировать, что full rewrite observability/perf pipeline вне scope данного change.
- [x] 2.1 Утвердить design решения по единому источнику truth для `stage/reason` taxonomy и projection completeness.
- [x] 2.2 Зафиксировать точную retention semantics (`max versions` как count-based контракт, без неоднозначности version-gap).
- [x] 2.3 Определить migration policy для observability dual-write без ломки legacy keys.
- [x] 2.4 Зафиксировать perf artifacts provenance contract (`change_id`, profile, generated_at, validator expectations).
- [x] 2.5 Зафиксировать ownership typed registry и policy "no adapter-local mapping bypass".
- [x] 2.6 Зафиксировать детерминированный eviction order и latest exact protection для global guard.
- [x] 2.7 Зафиксировать unified serve-outcome contract для `completion/hover/signatureHelp/definition`.
- [x] 2.8 Синхронизировать operation policy для `Definition` с interactive path.
- [x] 2.9 Зафиксировать optional provenance migration в `v1` и fail-правила для provided provenance.
- [x] 2.10 Зафиксировать authoritative source policy для active `change_id` (`--change-id` > `OPENSPEC_CHANGE_ID` > legacy-local) и запрет hardcoded foreign `change_id`.
- [x] 2.11 Зафиксировать блокирующий parity drift threshold для cutover (`parity_drift_rate <= 0.01`, `parity_pairs_total >= 100`).

## 3. Implementation
- [x] 3.1 Ввести typed registry/mapping слой для runtime stage/reason и переподключить normalize + legacy projection к нему.
- [x] 3.2 Гарантировать dedicated mapping для `type_index_precompute` queue/exec/build (без деградации в `runtime_other_*`).
- [x] 3.3 Привести retention реализацию `TypeIndexArtifact` к зафиксированному count-based контракту.
- [x] 3.4 Централизовать emission `type_index` serve reasons для `completion/hover/signatureHelp/definition`.
- [x] 3.5 Убрать hardcoded foreign `change_id` из perf-gate пути и сделать привязку к runtime invocation context.
- [x] 3.6 Обновить versioned contracts/changelog при изменениях observability surface.
- [x] 3.7 Удалить/ограничить разрозненные string-based mapping ветки в пользу typed registry materialization.
- [x] 3.8 Привести `Definition` к interactive freshness/policy consistency (`queue priority`, `freshness knobs`, serve-only semantics).
- [x] 3.9 Добавить optional provenance поля в perf report `v1` + validator checks для provided provenance mismatch/invalid.
- [x] 3.10 Добавить plumbing для invocation-context `expected_change_id` (CLI/env) в perf/gate pipeline.
- [x] 3.11 Убрать hardcoded `CHANGE_ID` из production/perf report path; оставить только test-fixture constants в test-контексте.
- [x] 3.12 Внедрить gate checks для parity threshold (`<= 0.01`) и минимального объёма evidence (`parity_pairs_total >= 100`).

## 4. Validation
- [x] 4.1 Добавить contract tests на полноту mapping (registry -> canonical -> legacy), включая unknown -> `other`.
- [x] 4.2 Добавить retention invariants tests (`max versions`, eviction order, latest exact protection under global guard).
- [x] 4.3 Добавить integration tests, подтверждающие emission serve reasons для всех interactive операций.
- [x] 4.4 Добавить/обновить tests для perf report traceability (`change_id` consistency).
- [x] 4.5 Добавить тесты, что `missing optional provenance` в `v1` не валит только legacy-local прогон без `expected_change_id`.
- [x] 4.6 Добавить тесты, что `provided provenance mismatch/invalid` в `v1` валит gate fail-closed.
- [x] 4.7 Добавить canary rollback test при parity drift выше утверждённого порога.
- [x] 4.8 Добавить тесты, что отсутствие `expected_change_id` даёт only-local/non-authoritative evidence (без права на cutover).
- [x] 4.9 Добавить тесты для fail-closed при `parity_pairs_total < 100` (insufficient evidence).
- [x] 4.10 `openspec validate refactor-v2-contract-first-hardening --strict --no-interactive`.

## Dependencies / Parallelism
- [x] D1 Пункты 1.1-1.4 и 2.1-2.11 блокируют 3.1-3.12.
- [x] D2 Пункты 3.1 и 3.3 можно делать параллельно после 2.2.
- [x] D3 Пункты 3.4, 3.8, 3.9 и 3.12 зависят от 3.1 (единый registry/contract path).
- [x] D4 Пункты 4.1-4.10 зависят от 3.1-3.12.
