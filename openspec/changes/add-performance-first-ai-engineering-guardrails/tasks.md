## 1. Specification
- [x] 1.1 Добавить в `dev-workflow` требование ADR gate для архитектурно-значимых/perf-critical изменений.
- [x] 1.2 Добавить в `dev-workflow` требование doc-first non-MVP контракта с acceptance matrix до имплементации.
- [x] 1.3 Добавить в `dev-workflow` требование test-first цикла для backend/runtime behavioral changes.
- [x] 1.4 Добавить в `dev-workflow` требование protected acceptance assets с fail-closed policy.
- [x] 1.5 Добавить в `dev-workflow` требование perf evidence merge-gate (`latency + allocations + lock contention`).
- [x] 1.6 Добавить в `bsl-intellisense-v2` resource budget requirement для интерактивного completion.
- [x] 1.7 Добавить в `bsl-intellisense-v2` low-cardinality resource observability requirement.
- [x] 1.8 Добавить в `dev-workflow` и `bsl-intellisense-v2` dual latency gate requirement (relative ratio + absolute ceiling).
- [x] 1.9 Зафиксировать `Option B` как единственный путь: dedicated perf-gate module + versioned schema contract.
- [x] 1.10 Добавить в `dev-workflow` требование детерминированной `change_criticality` классификации с fail-closed policy.
- [x] 1.11 Добавить в `dev-workflow` machine-readable test-first evidence contract requirement.
- [x] 1.12 Добавить в `dev-workflow` bootstrap policy requirement для initial perf budgets.
- [x] 1.13 Ужесточить `bsl-intellisense-v2` до canonical metric keys без "или эквивалент".

## 2. Design And Tooling
- [x] 2.1 Утвердить ADR template и критерии "architecturally significant/perf-critical change".
- [x] 2.2 Определить protected-assets manifest (какие тесты/контракты/baselines считаются immutable в implementation change).
- [x] 2.3 Спроектировать schema contract v1 (`input/baseline/report`) и format/version policy для resource baseline artifacts.
- [x] 2.4 Зафиксировать ownership: кто утверждает ADR, кто владелец perf budgets, кто владелец protected-assets policy.
- [x] 2.5 Зафиксировать абсолютные latency ceilings (`p95/p99`) по профилям `small/large/churn` и policy их изменения через ADR.
- [x] 2.6 Спроектировать dedicated perf-gate module boundary (API, integration points, reason-code taxonomy).
- [x] 2.7 Определить schema для `change_criticality` классификации (enum + reason/rule-id + storage path).
- [x] 2.8 Определить schema для machine-readable test-first evidence (`failing_ref`, `passing_ref`, `scope`, reason-codes).
- [x] 2.9 Зафиксировать bootstrap методику initial budgets (sample size, aggregation rule, профили, approval path).

## 3. Implementation Rollout
- [x] 3.1 Реализовать process-gates (ADR/doc-first/protected-assets) как автоматические проверки в CI/локальном workflow.
- [x] 3.2 Добавить instrumentation completion hot path для allocation/lock metrics.
- [x] 3.3 Выделить и внедрить dedicated perf-gate module как единственный evaluator для CI/harness/runtime checks.
- [x] 3.4 Зафиксировать и подключить `contracts/intellisense-perf-gate/v1/**` в pipeline с compatibility-diff проверкой.
- [x] 3.5 Реализовать extended perf gate в модуле: dual latency gate (relative ratio + absolute ceiling) + resource budgets + fail-closed deterministic report с reason-codes.
- [x] 3.6 Включить blocking-mode для unified Option B gate после фиксации baseline.
- [x] 3.7 Реализовать gate-проверки на обязательные canonical metric keys в contract input/report.
- [x] 3.8 Реализовать fail-closed проверку отсутствия `change_criticality` и отсутствия test-first evidence для соответствующих change-классов.

## 4. Validation
- [x] 4.1 `openspec validate add-performance-first-ai-engineering-guardrails --strict --no-interactive`.
- [x] 4.2 Dry-run на репрезентативных профилях (`small`, `large`, `churn`) и подтверждение воспроизводимости отчётов.
- [x] 4.3 Review с владельцами `analysis-v2`, `runtime`, `LSP`, и process ownership.
- [x] 4.4 Подтвердить, что в `lsp_server`/скриптах нет альтернативной inline логики perf-verdict вне dedicated module.
- [x] 4.5 Подтвердить, что bootstrap budgets зафиксированы в versioned contract до включения blocking mode.

## Dependencies / Parallelism
- [x] D1 Пункты 2.1 и 2.2 блокируют 3.1.
- [x] D2 Пункты 2.3 и 2.6 блокируют 3.3 и 3.5.
- [x] D3 Пункт 3.2 может выполняться параллельно с 3.1 после завершения 2.4.
- [x] D4 Пункт 3.4 блокирует 3.6.
